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

import type { Message, TimelineProps } from '../../shared/contracts.js';

import { type DisplayPart, type State } from './state.js';

export function utf8Bytes(text: string): number {
  let bytes = 0;
  for (const character of text) {
    const point = character.codePointAt(0)!;
    bytes += point <= 0x7f ? 1 : point <= 0x7ff ? 2 : point <= 0xffff ? 3 : 4;
  }
  return bytes;
}
// Raw fences can advance while model text is still open. Only an actual step
// closure covers its live prefix for both atomic and item-accepting providers.

export function appendLive(
  state: State,
  sequence: number,
  part: DisplayPart,
  invocationId: string,
  text: string,
) {
  if (state.visits.find((visit) => visit.id === state.selected?.id)?.following === false) return;
  const messageId = part.messageId;
  const field = part.textKind === 'thinking' ? 'thinking' : 'body';
  if (
    state.window &&
    sequence <= state.window.page.through &&
    state.window.page.chunks.some(
      (chunk) => chunk.messageId === messageId && (chunk.field ?? 'body') === field,
    )
  )
    return;
  const ids = new Set(state.live.map((fragment) => fragment.messageId));
  let remaining = 16000 - state.live.reduce((total, fragment) => total + fragment.text.length, 0);
  // Thinking shares the existing prefix budget but leaves room for an answer
  // before atomic providers publish either field. Never evict a displayed prefix.
  if (field === 'thinking') {
    const thinking = state.live.filter((fragment) => fragment.field === 'thinking');
    const thinkingIds = new Set(thinking.map((fragment) => fragment.messageId));
    if (thinking.length >= 32 || (!thinkingIds.has(messageId) && thinkingIds.size >= 8)) return;
    remaining = Math.min(
      remaining,
      8000 - thinking.reduce((total, fragment) => total + fragment.text.length, 0),
    );
  }
  // A truncated surrogate can leave one unused code unit; keep the prefix closed.
  if (remaining <= 1 || state.live.length >= 64 || (!ids.has(messageId) && ids.size >= 16)) return;
  text = text.slice(0, remaining);
  const last = text.charCodeAt(text.length - 1);
  if (last >= 0xd800 && last <= 0xdbff) text = text.slice(0, -1);
  if (text)
    state.live.push({
      sequence,
      messageId,
      invocationId,
      stepId: part.stepId,
      text,
      closedAt: null,
      field,
    });
}

export function closeLive(state: State, sequence: number, invocationId: string, stepId?: string) {
  for (const fragment of state.live)
    if (
      fragment.closedAt === null &&
      fragment.invocationId === invocationId &&
      (stepId === undefined || fragment.stepId === stepId)
    )
      fragment.closedAt = sequence;
}

export function messages(state: State): Message[] {
  const canonical: Message[] = (state.window?.page.chunks ?? []).map((chunk) => ({
    id: chunk.field === 'thinking' ? 'thinking-' + chunk.messageId : chunk.messageId,
    contentKey: `canonical-${chunk.sequence}-${chunk.offset}${chunk.field === 'thinking' ? '-thinking' : ''}`,
    role: chunk.field === 'thinking' ? 'thinking' : chunk.role,
    timestamp: chunk.timestamp,
    text: chunk.text,
    tool: chunk.tool,
    continued: chunk.offset !== 0 || utf8Bytes(chunk.text) !== chunk.totalBytes,
    source: chunk.sequence,
    image: chunk.tool?.result?.image ?? false,
  }));
  if (!state.window?.latest) return canonical;
  const live = new Map<string, Message>();
  for (const fragment of state.live) {
    if (
      fragment.sequence <= state.window.page.through &&
      state.window.page.chunks.some(
        (chunk) =>
          chunk.messageId === fragment.messageId && (chunk.field ?? 'body') === fragment.field,
      )
    )
      continue;
    const key = fragment.messageId + (fragment.field === 'thinking' ? '-thinking' : '');
    let message = live.get(key);
    if (!message) {
      message = {
        id: 'live-' + key,
        contentKey: 'live-' + key,
        role: fragment.field === 'thinking' ? 'thinking' : 'assistant',
        timestamp: null,
        text: '',
        tool: null,
        continued: true,
        source: null,
        image: false,
      };
      live.set(key, message);
    }
    message.text += fragment.text;
  }
  return [...canonical, ...live.values()];
}

export function turnTimings(state: State, displayed: Message[]): TimelineProps['timings'] {
  const turns = new Map<string, TimelineProps['timings'][number]>();
  for (const chunk of state.window?.page.chunks ?? []) {
    if (!chunk.timing) continue;
    if (!chunk.timing.finished && (!state.window?.latest || state.run?.turnId !== chunk.turnId))
      continue;
    turns.set(chunk.turnId, {
      ...chunk.timing,
      turnId: chunk.turnId,
      afterMessageId: chunk.field === 'thinking' ? 'thinking-' + chunk.messageId : chunk.messageId,
    });
  }
  if (state.window?.latest && state.run) {
    const timing = turns.get(state.run.turnId);
    if (timing && !timing.finished) {
      for (const fragment of state.live) {
        const id =
          'live-' + fragment.messageId + (fragment.field === 'thinking' ? '-thinking' : '');
        if (
          fragment.invocationId === state.run.invocationId &&
          displayed.some((message) => message.id === id)
        )
          timing.afterMessageId = id;
      }
    }
  }
  return [...turns.values()];
}
