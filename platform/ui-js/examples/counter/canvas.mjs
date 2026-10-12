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

// The exporter inserts the package’s declared public contract here.
const contract = null;
export function activate() {
  return { App: { contract, kind: 'component', component: ['app'] } };
}
const rect = (x, y, width, height) => ({ origin: { x, y }, width, height });
const text = (x, y, text, size, color = 0x303b36ff) => ({
  kind: 'text',
  position: { x, y },
  text,
  size,
  color,
});
function frame(state) {
  const card = rect(state.x, state.y, 216, 76);
  return {
    events: [],
    actions: [],
    frame: {
      revision: state.revision,
      state,
      surface: {
        kind: 'window',
        binding: 'surface',
        title: 'Workflow · Maka',
        width: 640,
        height: 420,
      },
      view: {
        kind: 'canvas',
        key: 'board',
        style: {
          width: { unit: 'pixels', value: 640 },
          height: { unit: 'pixels', value: 400 },
          background: 0xf5f5f0ff,
        },
        contents: {
          focusColor: 0x698778ff,
          dragColor: 0x698778ff,
          commands: [
            text(28, 24, 'Workflow', 24),
            text(
              28,
              60,
              'Drag a task to Review · Arrow keys move · Enter moves to review',
              13,
              0x616c64ff,
            ),
            { kind: 'rect', rect: rect(24, 104, 280, 268), radius: 16, color: 0xeaece6ff },
            { kind: 'rect', rect: rect(324, 104, 292, 268), radius: 16, color: 0xe8eee7ff },
            text(40, 120, 'IN PROGRESS', 11, 0x616c64ff),
            text(340, 120, 'REVIEW', 11, 0x416653ff),
            {
              kind: 'line',
              from: { x: 304, y: 240 },
              to: { x: 324, y: 240 },
              color: 0xa9b5aaff,
              width: 2,
            },
            ...(state.removed
              ? []
              : [
                  {
                    kind: 'rect',
                    rect: card,
                    radius: 12,
                    color: state.disabled ? 0xd6dcd5ff : 0xfefefbff,
                  },
                  text(state.x + 16, state.y + 14, 'Refine the experience', 15),
                  text(
                    state.x + 16,
                    state.y + 43,
                    state.placed ? 'Ready for review' : 'Interaction and visual polish',
                    12,
                    0x616c64ff,
                  ),
                ]),
            text(
              28,
              378,
              state.placed
                ? 'Ready for feedback'
                : 'Move to Review when the task is ready for feedback',
              12,
              0x616c64ff,
            ),
          ],
          regions: [
            { key: 'review', rect: rect(324, 104, 292, 268), label: 'Review lane', tabIndex: -1 },
            ...(state.removed
              ? []
              : [
                  {
                    key: 'task',
                    rect: card,
                    radius: 12,
                    label: 'Refine the experience',
                    draggable: true,
                    disabled: state.disabled,
                    listen: ['pointer_down', 'drop', 'key_down', 'activate'],
                  },
                ]),
          ],
        },
      },
    },
  };
}
export const app = {
  mount(props) {
    return frame({
      revision: 0,
      x: 44,
      y: 164,
      placed: false,
      disabled: false,
      removed: false,
      calls: 0,
      mode: props.mode ?? 'ok',
    });
  },
  update(state) {
    return frame({ ...state, revision: state.revision + 1 });
  },
  event(state, input) {
    const event = input.event;
    if (event.kind === 'pointer_down' && state.mode === 'throw')
      throw new Error('Canvas rule failed');
    const next = { ...state, revision: state.revision + 1, calls: state.calls + 1, last: event };
    if (event.kind === 'pointer_down' && state.mode === 'disable') next.disabled = true;
    if (event.kind === 'pointer_down' && state.mode === 'remove') next.removed = true;
    const decision = {};
    if (event.kind === 'drop') {
      if (event.target === 'review') {
        next.x = Math.max(340, Math.min(380, Math.round((state.x + event.delta.x) / 20) * 20));
        next.y = Math.max(160, Math.min(280, Math.round((state.y + event.delta.y) / 20) * 20));
        next.placed = true;
      } else decision.preventDefault = true;
    } else if (event.kind === 'key_down') {
      const moves = { left: [-20, 0], right: [20, 0], up: [0, -20], down: [0, 20] };
      const move = moves[event.key];
      if (move) {
        next.x = Math.max(24, Math.min(400, state.x + move[0]));
        next.y = Math.max(160, Math.min(280, state.y + move[1]));
        decision.stopPropagation = true;
        decision.preventDefault = true;
      }
      if (event.key === 'enter') {
        next.x = 340;
        next.placed = true;
        decision.stopPropagation = true;
        decision.preventDefault = true;
      }
    } else if (event.kind === 'activate') {
      next.x = 340;
      next.placed = true;
    }
    const output = frame(next);
    if (event.kind === 'pointer_down' && state.mode === 'invalid')
      output.frame.view.contents.commands[0].position.x = 16000.00000001;
    output.decision = decision;
    return output;
  },
};
