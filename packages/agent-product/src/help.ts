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

import type { Component, Emission } from '../../plugin-sdk/src/index.js';
import { helpContract, type HelpProps } from './contracts.js';
import { copy } from './locale.js';
import { fill, pixels, output, type Node, type Presentation } from './view.js';

type State = { revision: number };

export function help<N extends Node>(
  presentation: Presentation<N>,
): Component<State, HelpProps, N> {
  function frame(state: State, props: HelpProps, events: Emission[] = []) {
    const ui = presentation.paint(props.theme);
    const text = copy(props.locale);
    const terminal = presentation.target === 'tui';
    const rows = [
      [terminal ? 'Enter' : 'Ctrl/⌘ Enter', text.helpSend],
      [terminal ? 'Shift Enter / Ctrl J' : 'Enter / Shift Enter', text.helpNewline],
      ['Tab / Shift Tab', text.helpFocus],
      ['Enter / Space', text.helpActivate],
      ['Ctrl/⌘ K', text.helpCommands],
      ['Ctrl/⌘ B', text.helpNavigation],
      ['Ctrl/⌘ N', text.helpNew],
      ['Esc', text.helpBack],
      ['F1', text.helpTitle],
    ];
    return output(
      state,
      ui.box(
        'help-content',
        [
          presentation.reading(
            'help-reading',
            {
              content: ui.box(
                'help-rows',
                rows.map(([key, label], index) =>
                  ui.text(`shortcut-${index}`, `${key}  ·  ${label}`, 14),
                ),
                {
                  scroll: 'manual',
                  gap: terminal ? 0 : 12,
                  terminal: { grow: 1, basis: 0 },
                  style: { width: fill, height: pixels(280), minWidth: 0, minHeight: 0 },
                },
              ),
            },
            {},
          ),
          ui.button('close', text.helpClose),
        ],
        {
          visible: props.open,
          terminal: { grow: 1 },
          style: { width: fill, minWidth: 0 },
        },
      ),
      events,
    );
  }
  return {
    mount: (props) => frame({ revision: 0 }, props),
    update: (state, props) => frame({ revision: state.revision + 1 }, props),
    event(state, input, props) {
      return frame(
        { revision: state.revision + 1 },
        props,
        props.open && input.kind === 'raw' && input.key === 'close'
          ? [{ name: 'closed', value: {} }]
          : [],
      );
    },
  };
}
export function activate() {
  return { Help: { kind: 'component', contract: helpContract, component: ['component'] } };
}
