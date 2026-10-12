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
  ComponentInput,
  ValueCheckpointCommand,
  ValueCheckpointObservation,
} from '../../packages/plugin-sdk/src/index.js';
import { sameValue } from './value.js';
import type { Node, Presentation } from './view.js';

// UI values only: no resource owners, operation dispatch or native editor buffers.
export interface Bookmark<T> {
  initial: T;
  saved: T | null;
  observed: ValueCheckpointObservation | null;
  command: ValueCheckpointCommand | null;
  sequence: number;
  restored: boolean;
  touched: boolean;
}
export function bookmark<T>(initial: T): Bookmark<T> {
  return {
    initial: JSON.parse(JSON.stringify(initial)),
    saved: null,
    observed: null,
    command: null,
    sequence: 0,
    restored: false,
    touched: false,
  };
}
export function observeBookmark<T>(
  state: Bookmark<T>,
  input: ComponentInput,
  id: string,
  current: T,
  validate: (value: unknown) => T | null,
): T | undefined {
  if (input.kind !== 'component' || input.child !== id) return;
  const observed = input.event as ValueCheckpointObservation;
  if (state.observed && observed.sequence < state.observed.sequence) return;
  state.observed = observed;
  if (observed.state.kind !== 'ready') return;
  state.saved = validate(
    observed.state.value === undefined ? null : JSON.parse(JSON.stringify(observed.state.value)),
  );
  if (state.command && observed.sequence === state.command.sequence) state.command = null;
  if (!state.restored) {
    state.restored = true;
    // A late read must never undo navigation already made by the user.
    if (state.saved !== null && !state.touched && sameValue(current, state.initial))
      return JSON.parse(JSON.stringify(state.saved)) as T;
  }
}
export function bookmarkReady<T>(state: Bookmark<T>, value: T): boolean {
  return (
    state.restored &&
    state.observed?.state.kind === 'ready' &&
    !state.command &&
    sameValue(state.saved ?? state.initial, value)
  );
}
export function bookmarkView<T, N extends Node>(
  presentation: Presentation<N>,
  state: Bookmark<T>,
  id: string,
  key: string,
  value: T,
): N {
  if (!sameValue(value, state.initial)) state.touched = true;
  if (
    state.restored &&
    state.observed?.state.kind === 'ready' &&
    !state.command &&
    !sameValue(state.saved ?? state.initial, value)
  ) {
    state.command = {
      kind: 'write',
      sequence: ++state.sequence,
      expectedRevision: state.observed.state.revision,
      value: JSON.parse(JSON.stringify(value)),
    };
  }
  return presentation.slot(
    id,
    'checkpoint',
    { key, format: 'maka.agent.bookmark/v1', command: state.command },
    { changed: 'bookmark' },
  );
}
export function retryBookmark<T>(state: Bookmark<T>) {
  state.command = { kind: 'read', sequence: ++state.sequence };
}
