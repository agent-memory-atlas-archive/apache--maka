/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

import type {
  Action,
  ActionOutcome,
  AgentHistorySourcePage,
  ComponentInput,
  MarkdownTextResult,
  SourceField,
  SourceFormat,
} from '../../plugin-sdk/src/index.js';
import type { Message, TimelineProps, MessageCopyObservation } from './contracts.js';
import { sameValue } from './value.js';
function utf8Bytes(text: string) {
  let bytes = 0;
  for (const character of text) {
    const point = character.codePointAt(0)!;
    bytes += point <= 0x7f ? 1 : point <= 0x7ff ? 2 : point <= 0xffff ? 3 : 4;
  }
  return bytes;
}

// Native copy accepts 64 KiB of UTF-8. Escaped control characters also need
// a bounded JSON delivery through Timeline, Root and the physical copy widget.
function fits(text: string) {
  return utf8Bytes(text) <= 65536 && utf8Bytes(JSON.stringify(text)) <= 131074;
}
export interface MessageCopy {
  basis: {
    request: number;
    sessionId: string;
    through: number;
    landing: number;
    sequence: number | null;
    field: SourceField;
    markdown: boolean;
  };
  key: string;
  serial: number;
  pending: { key: string; kind: 'source' | 'format' } | null;
  preparing: number;
  total: number | null;
  format: SourceFormat | null;
  text: string;
  raw: string | null;
  rendered: string | null;
  status: MessageCopyObservation['status'];
}
interface State {
  revision: number;
  copy: MessageCopy | null;
}
function action(copy: MessageCopy, actions: Action[], kind: 'source' | 'format') {
  const key = copy.key + '-' + ++copy.serial;
  copy.pending = { key, kind };
  const { basis } = copy;
  actions.push({
    key,
    binding: kind === 'source' ? 'agent' : 'formatter',
    member: 0,
    method: kind === 'source' ? 'source' : 'plain',
    resources: {},
    input:
      kind === 'source'
        ? {
            sessionId: basis.sessionId,
            through: basis.through,
            source: { kind: 'sequence', sequence: basis.sequence! },
            field: basis.field,
            position: copy.text
              ? { kind: 'after', offset: utf8Bytes(copy.text) }
              : { kind: 'start' },
          }
        : { source: copy.raw! },
  });
}
export function syncMessageCopy(
  state: State,
  props: TimelineProps,
  message: Message | null,
  actions: Action[],
) {
  if (
    !props.copyRequest ||
    !props.toolContext ||
    props.source ||
    props.search ||
    props.loading ||
    !message
  ) {
    state.copy = null;
    return;
  }
  const result = message.tool?.result;
  const tool = message.role === 'tool_call' || message.role === 'tool_result';
  const field: SourceField = tool
    ? result || message.role === 'tool_result'
      ? 'result'
      : 'arguments'
    : message.role === 'thinking'
      ? 'thinking'
      : 'body';
  const sequence = tool
    ? (result?.sequence ??
      (message.role === 'tool_result'
        ? message.source
        : (message.tool?.call?.sequence ?? message.source)))
    : message.source;
  const basis = {
    request: props.copyRequest,
    ...props.toolContext,
    landing: props.landing.sequence,
    sequence,
    field,
    markdown: message.role === 'assistant' || message.role === 'thinking',
  };
  if (state.copy && sameValue(state.copy.basis, basis)) return;
  state.copy = {
    basis,
    key: `message-copy-${props.copyRequest}-${state.revision}`,
    serial: 0,
    pending: null,
    preparing: 0,
    total: null,
    format: null,
    text: '',
    raw: null,
    rendered: null,
    status: sequence === null ? 'streaming' : 'loading',
  };
  if (sequence !== null) action(state.copy, actions, 'source');
}
export function messageCopyEvent(state: State, input: ComponentInput, actions: Action[]): boolean {
  if (input.kind !== 'action' || !state.copy?.pending || state.copy.pending.key !== input.key)
    return false;
  const copy = (state.copy = { ...state.copy });
  const pending = copy.pending!;
  copy.pending = null;
  const result = input.event as ActionOutcome;
  if (result.status !== 'succeeded') {
    copy.status = 'unavailable';
    return true;
  }
  if (pending.kind === 'format') {
    const value = result.value as MarkdownTextResult;
    if (value.kind === 'too_large' || (value.kind === 'ready' && !fits(value.text)))
      copy.status = 'too_large';
    else if (value.kind === 'ready') {
      copy.rendered = value.text || null;
      copy.status = value.text ? 'ready' : 'empty';
    } else copy.status = 'unavailable';
    return true;
  }
  const page = result.value as AgentHistorySourcePage;
  if (page.through !== copy.basis.through) {
    copy.status = 'unavailable';
    return true;
  }
  if (page.kind === 'preparing') {
    if (++copy.preparing <= 8) action(copy, actions, 'source');
    else copy.status = 'unavailable';
    return true;
  }
  const source = page.source,
    offset = utf8Bytes(copy.text),
    end = source.offset + utf8Bytes(source.text);
  if (
    source.sequence !== copy.basis.sequence ||
    source.field !== copy.basis.field ||
    source.path != null ||
    !source.fields.includes(source.field) ||
    source.offset !== offset ||
    end > source.totalBytes ||
    (!source.text && source.totalBytes > 0) ||
    (copy.total !== null && (source.totalBytes !== copy.total || source.format !== copy.format))
  ) {
    copy.status = 'unavailable';
    return true;
  }
  if (source.totalBytes > 65536) {
    copy.status = 'too_large';
    copy.text = '';
    return true;
  }
  copy.total = source.totalBytes;
  copy.format = source.format;
  copy.text += source.text;
  if (end < source.totalBytes) {
    action(copy, actions, 'source');
    return true;
  }
  if (!fits(copy.text)) {
    copy.status = 'too_large';
    copy.text = '';
    return true;
  }
  copy.raw = copy.text || null;
  copy.text = '';
  if (copy.raw === null) copy.status = 'empty';
  else if (copy.basis.markdown && source.format === 'text') action(copy, actions, 'format');
  else {
    copy.rendered = copy.raw;
    copy.status = 'ready';
  }
  return true;
}
export function messageCopyObservation(state: State): MessageCopyObservation | null {
  const copy = state.copy;
  return copy
    ? {
        request: copy.basis.request,
        sessionId: copy.basis.sessionId,
        through: copy.basis.through,
        landing: copy.basis.landing,
        field: copy.basis.field,
        status: copy.status,
        source: copy.raw,
        message: copy.rendered,
      }
    : null;
}
