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

import { ui } from '../src/index.js';
import type {
  DesktopScene,
  TerminalScene,
  DesktopNode,
  TerminalNode,
  Component,
} from '../src/index.js';

const desktopText = ui.desktop.text({ key: 'desktop-text', text: 'Hello', color: 0xffffff });
const terminalText = ui.terminal.text({ key: 'terminal-text', text: 'Hello', color: 0xffffff });
const desktop = ui.desktop.stack({
  key: 'desktop',
  direction: 'column',
  gap: 8,
  padding: 0,
  background: 0,
  children: [desktopText],
  scroll: 'anchor',
  style: { width: { unit: 'fraction', value: 1 } },
});
const terminal = ui.terminal.stack({
  key: 'terminal',
  direction: 'column',
  gap: 1,
  padding: 0,
  background: 0,
  children: [terminalText],
  scroll: 'anchor',
  layout: { grow: 1 },
});
const desktopWire: DesktopScene = desktop;
const terminalWire: TerminalScene = terminal;
const desktopNode: DesktopNode = desktop;
const terminalNode: TerminalNode = terminal;
void [desktopWire, terminalWire, desktopNode, terminalNode];

ui.desktop.dialog({
  key: 'dialog',
  label: 'Dialog',
  background: desktop,
  content: desktopText,
  open: true,
});
ui.terminal.dialog({
  key: 'dialog',
  label: 'Dialog',
  background: terminal,
  content: terminalText,
  open: true,
  width: 40,
  height: 10,
  background_color: 0,
  border_color: 0xffffff,
});
ui.desktop.popup({
  key: 'popup',
  label: 'Menu',
  trigger: desktopText,
  content: desktop,
  open: true,
});
ui.desktop.canvas({
  key: 'canvas',
  contents: {
    commands: [
      { kind: 'line', from: { x: 0, y: 0 }, to: { x: 20, y: 20 }, color: 0xffffff, width: 1 },
    ],
    regions: [],
    focusColor: null,
    dragColor: null,
  },
});
ui.desktop.reading({
  key: 'reading',
  binding: 'reading',
  props: { content: desktop },
  on: { observed: 'viewport' },
});
ui.terminal.reading({
  key: 'reading',
  binding: 'reading',
  props: { content: terminal },
  on: { observed: 'viewport' },
});
ui.desktop.component({
  key: 'plugin',
  binding: 'external',
  props: { custom: true },
  on: { changed: 'change' },
});
ui.terminal.component({
  key: 'plugin',
  binding: 'external',
  member: 1,
  props: { custom: true },
  on: { changed: 'change' },
});

ui.desktop.text({ ...desktopText, text: 'Updated' });
ui.terminal.stack({ ...terminal, children: [terminalText] });

const desktopReading = ui.desktop.reading({
  key: 'reading',
  binding: 'reading',
  props: { content: desktop },
  on: {},
});
// @ts-expect-error An opaque component leaf also retains its target when reconstructed.
ui.terminal.component(desktopReading);
// @ts-expect-error Native wrapper reconstruction cannot retarget a branded leaf.
ui.terminal.reading(desktopReading);
// Structurally compatible variables must retain their original target.
// @ts-expect-error Constructing again cannot silently change a branded variable's target.
ui.terminal.text(desktopText);
// @ts-expect-error A copied desktop node still belongs to the desktop authoring target.
ui.terminal.stack({ ...desktop, children: [terminalText] });
// @ts-expect-error Desktop text cannot enter a terminal tree.
ui.terminal.stack({ ...terminal, children: [desktopText] });
// @ts-expect-error A terminal subtree cannot enter a desktop tree.
ui.desktop.stack({ ...desktop, children: [terminal] });
// @ts-expect-error The nested reading scene retains its target.
ui.terminal.reading({ key: 'reading', binding: 'reading', props: { content: desktop }, on: {} });
// @ts-expect-error Raw wire objects have no authoring target; construct them with ui.*.
const inferredTerminal: TerminalNode = desktopWire;
// @ts-expect-error Terminal presentation has no popup primitive.
ui.terminal.popup({
  key: 'popup',
  label: 'Menu',
  trigger: terminalText,
  content: terminal,
  open: true,
});
// @ts-expect-error Built-in label requires its real appearance contract.
ui.desktop.label({ key: 'label', binding: 'label', props: { text: 'Hello' }, on: {} });
// @ts-expect-error Input props cannot use string flags.
ui.terminal.input({ key: 'input', binding: 'input', props: { disabled: 'yes' }, on: {} });
const component: Component<null, null, DesktopNode> = {
  mount: () => ({
    frame: { state: null, revision: 0, surface: null, view: desktop },
    events: [],
    actions: [],
    subscriptions: [],
  }),
  update: () => ({
    frame: { state: null, revision: 0, surface: null, view: desktop },
    events: [],
    actions: [],
    subscriptions: [],
  }),
  event: () => ({
    frame: { state: null, revision: 0, surface: null, view: desktop },
    events: [],
    actions: [],
    subscriptions: [],
  }),
};
void [component, inferredTerminal];
