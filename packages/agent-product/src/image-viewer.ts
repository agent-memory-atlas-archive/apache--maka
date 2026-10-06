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
  AgentToolImagePage,
  Component,
  ImageDescriptor,
  ImageObservation,
  ImagePacket,
  ImageProps,
} from '../../plugin-sdk/src/index.js';
import { imageViewerContract, type ImageViewerProps } from './contracts.js';
import { copy } from './locale.js';
import { sameValue } from './value.js';
import { output, paint, slot, fill, type Target } from './view.js';

type Page = Extract<AgentToolImagePage, { kind: 'ready' }>;
interface State {
  revision: number;
  serial: number;
  epoch: number;
  identity: ImageViewerProps['image'];
  pending: Record<string, { epoch: number; offset: number }>;
  queue: Page[];
  waiting: number[];
  descriptor: ImageDescriptor | null;
  observation: ImageObservation | null;
  packet: ImagePacket | null;
  packetSequence: number;
  requested: number;
  received: number;
  failed: boolean;
}
const CHUNK = 32768;
function reset(state: State, image: ImageViewerProps['image']) {
  state.epoch++;
  state.identity = image;
  state.queue = [];
  state.waiting = [];
  state.descriptor = null;
  state.packet = null;
  state.observation = null;
  state.requested = 0;
  state.received = 0;
  state.failed = false;
  // Admitted old queries retain their slots until their actual outcomes arrive.
}
function query(state: State, actions: Action[], offset: number) {
  const key = 'image-query-' + ++state.serial;
  state.pending[key] = { epoch: state.epoch, offset };
  actions.push({
    key,
    binding: 'agent',
    member: 0,
    method: 'toolImage',
    input: { ...state.identity!, offset },
    resources: {},
  });
}
function pump(state: State, props: ImageViewerProps, actions: Action[]) {
  if (!props.active || !state.identity || state.failed) return;
  for (const offset of state.waiting.splice(0)) query(state, actions, offset);
  if (!state.descriptor) {
    if (Object.keys(state.pending).length === 0) query(state, actions, 0);
    return;
  }
  if (!state.packet && state.observation?.source === state.descriptor.key) {
    const index = state.queue.findIndex((p) => p.offset === state.received);
    if (index >= 0) {
      const page = state.queue.splice(index, 1)[0]!;
      state.packet = {
        owner: state.observation.owner,
        source: state.descriptor.key,
        sequence: ++state.packetSequence,
        offset: page.offset,
        data: page.data,
      };
    }
  }
  let occupied = Object.keys(state.pending).length + state.queue.length + (state.packet ? 1 : 0);
  while (occupied < 4 && state.requested < state.descriptor.totalBytes) {
    const offset = state.requested;
    state.requested += Math.min(CHUNK, state.descriptor.totalBytes - offset);
    query(state, actions, offset);
    occupied++;
  }
}
function accept(state: State, request: { epoch: number; offset: number }, page: Page) {
  const identity = state.identity!;
  const size = Math.min(CHUNK, page.totalBytes - page.offset);
  const dataBytes =
    (page.data.length / 4) * 3 - (page.data.endsWith('==') ? 2 : page.data.endsWith('=') ? 1 : 0);
  if (
    page.through !== identity.through ||
    page.sequence !== identity.sequence ||
    page.offset !== request.offset ||
    !Number.isSafeInteger(page.totalBytes) ||
    page.totalBytes < 1 ||
    page.totalBytes > 5 * 1024 * 1024 ||
    size <= 0 ||
    page.data.length > 43692 ||
    dataBytes !== size ||
    !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(page.data) ||
    !/^sha256:[a-f0-9]{64}$/.test(page.digest) ||
    !['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'image/avif'].includes(page.mimeType)
  ) {
    state.failed = true;
    return;
  }
  const descriptor: ImageDescriptor = {
    key: 'image-' + state.epoch,
    digest: page.digest,
    mimeType: page.mimeType,
    totalBytes: page.totalBytes,
  };
  if (state.descriptor && !sameValue(state.descriptor, descriptor)) {
    state.failed = true;
    return;
  }
  if (!state.descriptor) {
    state.descriptor = descriptor;
    state.requested = size;
  }
  state.queue.push(page);
}
function frame(
  target: Target,
  state: State,
  props: ImageViewerProps,
  actions: Action[] = [],
  closed = false,
) {
  if (!sameValue(state.identity, props.image)) reset(state, props.image);
  if (target === 'desktop' && !closed) pump(state, props, actions);
  const ui = paint(target, props.theme),
    text = copy(props.locale);
  const current = state.descriptor && state.observation?.source === state.descriptor.key;
  const failed = state.failed || (current && state.observation?.phase === 'failed');
  const ready = current && state.observation?.phase === 'ready';
  const native: ImageProps = {
    source: state.descriptor,
    packet: state.packet,
    alt: text.imageTitle,
    fit: 'contain',
    height: 420,
    background: props.theme.surface,
    border: props.theme.border,
    radius: 12,
  };
  const content = [
    ui.box(
      'image-toolbar',
      [
        ui.button('image-back', text.sourceBack),
        ui.text('image-title', text.imageTitle, 18, props.theme.text, 600),
        ...(failed ? [ui.button('image-retry', text.sourceRetry)] : []),
      ],
      { row: true, style: { align: 'center', width: fill } },
    ),
    ...(target === 'desktop'
      ? [
          ...(!ready
            ? [
                ui.text(
                  'image-status',
                  failed ? text.imageFailed : text.imageLoading,
                  14,
                  props.theme.muted,
                ),
              ]
            : []),
          slot('pixels', 'image', native, { observed: 'image-observed' }),
          ...(ready
            ? [
                ui.text(
                  'image-dimensions',
                  `${state.observation!.width} × ${state.observation!.height}`,
                  12,
                  props.theme.muted,
                ),
              ]
            : []),
        ]
      : [ui.text('image-terminal', text.imageTerminal, 14, props.theme.muted)]),
  ];
  const out = output(
    state,
    ui.box('image-viewer', content, {
      visible: props.image !== null,
      padding: target === 'desktop' ? 24 : 0,
      style: { width: fill, maxWidth: 856, height: fill },
      terminal: { grow: 1 },
      scroll: 'manual',
    }),
    closed ? [{ name: 'closed', value: {} }] : [],
  );
  out.actions = actions;
  return out;
}
export function imageViewer(target: Target): Component<State, ImageViewerProps> {
  return {
    mount(props) {
      return frame(
        target,
        {
          revision: 0,
          serial: 0,
          epoch: 0,
          identity: null,
          pending: {},
          queue: [],
          waiting: [],
          descriptor: null,
          observation: null,
          packet: null,
          packetSequence: 0,
          requested: 0,
          received: 0,
          failed: false,
        },
        props,
      );
    },
    update(state, props) {
      state.revision++;
      return frame(target, state, props);
    },
    event(state, input, props) {
      state.revision++;
      if (!sameValue(state.identity, props.image)) reset(state, props.image);
      const actions: Action[] = [];
      if (input.kind === 'action') {
        const request = state.pending[input.key];
        if (request) {
          delete state.pending[input.key];
          if (request.epoch === state.epoch && state.identity && !state.failed) {
            const result = input.event as ActionOutcome;
            if (result.status !== 'succeeded') state.failed = true;
            else {
              const page = result.value as AgentToolImagePage;
              if (page.kind === 'preparing' && page.through === state.identity.through)
                state.waiting.push(request.offset);
              else if (page.kind === 'ready') accept(state, request, page);
              else state.failed = true;
            }
          }
        }
      } else if (
        input.kind === 'component' &&
        input.child === 'pixels' &&
        input.name === 'observed'
      ) {
        const observation = input.event as ImageObservation;
        if (
          !state.observation ||
          (observation.owner === state.observation.owner &&
            observation.revision > state.observation.revision)
        ) {
          state.observation = observation;
          if (observation.source === state.descriptor?.key && observation.phase === 'failed')
            state.failed = true;
          const packet = state.packet,
            receipt = observation.receipt;
          if (
            packet &&
            receipt &&
            receipt.owner === packet.owner &&
            receipt.source === packet.source &&
            receipt.sequence === packet.sequence
          ) {
            if (receipt.accepted) {
              state.received +=
                (packet.data.length / 4) * 3 -
                (packet.data.endsWith('==') ? 2 : packet.data.endsWith('=') ? 1 : 0);
            } else state.failed = true;
            state.packet = null;
          }
        }
      } else if (input.kind === 'raw') {
        if (input.key === 'image-back') return frame(target, state, props, [], true);
        if (input.key === 'image-retry' && (state.failed || state.observation?.phase === 'failed'))
          reset(state, props.image);
      }
      return frame(target, state, props, actions);
    },
  };
}
export function activate() {
  return {
    ImageViewer: { kind: 'component', contract: imageViewerContract, component: ['component'] },
  };
}
