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

import type { ComponentInput, ReadingObservation } from '../../packages/plugin-sdk/src/index.js';

// One physical scroll can produce several viewport observations. Consume it once,
// only at the matching viewport revision, and never during a reveal command.
export function scrollBoundary(
  state: { revision: number },
  input: ComponentInput,
  child: string,
): 'up' | 'down' | null {
  if (input.kind !== 'component' || input.child !== child || input.name !== 'observed') return null;
  const observed = input.event as ReadingObservation;
  if (!observed.scroll && observed.readingRevision === 0) {
    delete (state as { scrollRevisions?: Record<string, number> }).scrollRevisions?.[child];
  }
  if (
    !observed.scroll ||
    observed.viewport?.readingRevision !== observed.readingRevision ||
    observed.pendingCommand != null
  )
    return null;
  const owner = state as { scrollRevisions?: Record<string, number> };
  const revisions = (owner.scrollRevisions ??= {});
  if (observed.scroll.readingRevision <= (revisions[child] ?? -1)) return null;
  revisions[child] = observed.scroll.readingRevision;
  return (observed.scroll.direction === 'up' ? observed.viewport.atStart : observed.viewport.atEnd)
    ? observed.scroll.direction
    : null;
}

// Keep scene size independent of catalogue length, with overlapping windows so
// the native reading area can retain the visible row as its scroll anchor.
const windowSize = 32;
function windows(state: { revision: number }) {
  const owner = state as { scrollWindows?: Record<string, number> };
  return (owner.scrollWindows ??= {});
}
export function resetScroll(state: { revision: number }, child: string) {
  windows(state)[child] = 0;
}
export function visibleRows<T>(state: { revision: number }, child: string, entries: T[]) {
  const start = Math.min(windows(state)[child] ?? 0, Math.max(0, entries.length - 1));
  return entries
    .slice(start, start + windowSize)
    .map((entry, offset) => ({ entry, at: start + offset }));
}
export function scrollRows(
  state: { revision: number },
  input: ComponentInput,
  child: string,
  count: number,
) {
  const direction = scrollBoundary(state, input, child);
  const offsets = windows(state),
    start = offsets[child] ?? 0;
  if (direction === 'up') offsets[child] = Math.max(0, start - windowSize / 2);
  if (direction !== 'down') return false;
  if (start + windowSize < count) {
    offsets[child] = start + windowSize / 2;
    return false;
  }
  return true;
}

export function showRow(state: { revision: number }, child: string, at: number) {
  const offsets = windows(state),
    start = offsets[child] ?? 0;
  if (at < start || at >= start + windowSize)
    offsets[child] = Math.floor(at / (windowSize / 2)) * (windowSize / 2);
}
