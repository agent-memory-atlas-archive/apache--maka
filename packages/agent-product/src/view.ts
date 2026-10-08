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

import { ui } from '../../plugin-sdk/src/index.js';
import type {
  DesktopNode,
  TerminalNode,
  DesktopStyle,
  DesktopDimension,
  DesktopProps,
  Emission,
  Output,
  TerminalLayout,
  Shortcut,
  FocusShortcut,
  TextAppearance,
  ReadingProps,
} from '../../plugin-sdk/src/index.js';
import type { Theme } from './contracts.js';
export type Target = 'desktop' | 'tui';
export type Node = DesktopNode | TerminalNode;
interface BoxOptions {
  row?: boolean;
  gap?: number;
  padding?: number;
  surface?: boolean;
  style?: DesktopStyle;
  scroll?: 'none' | 'manual' | 'anchor' | 'follow_end';
  visible?: boolean;
  terminal?: TerminalLayout;
  viewport?: boolean;
  shortcuts?: Shortcut[];
  focusShortcuts?: FocusShortcut[];
}
interface Painter<N extends Node> {
  text(
    key: string,
    text: string,
    size?: number,
    color?: number,
    weight?: number,
    style?: DesktopStyle,
  ): N;
  button(
    key: string,
    label: string,
    disabled?: boolean,
    primary?: boolean,
    style?: DesktopStyle,
  ): N;
  focusButton?(key: string, label: string, child: string): N;
  dialog(key: string, label: string, background: N, content: N, open: boolean): N;
  box(key: string, children: N[], options?: BoxOptions): N;
}
export interface Presentation<N extends Node> {
  readonly target: Target;
  paint(theme: Theme): Painter<N>;
  slot<P>(key: string, binding: string, props: P, on: Record<string, string>): N;
  reading(
    key: string,
    props: Omit<ReadingProps, 'content'> & { content: N },
    on: Record<string, string>,
  ): N;
  popup?: (
    value: Omit<DesktopProps<'popup'>, 'trigger' | 'content'> & { trigger: N; content: N },
  ) => N;
}
const rgba = (n: number) => n >>> 8;
function font(theme: Theme, size: number, weight = 400, color = theme.text): TextAppearance {
  return {
    font: { family: '.SystemUIFont', fallbacks: [], size, lineHeight: 1.55, weight },
    foreground: color,
  };
}
function desktopPaint(theme: Theme): Painter<DesktopNode> {
  return {
    text: (key, text, size = 15, color = theme.text, weight = 400, style = {}) =>
      ui.desktop.text({
        key,
        text,
        color: rgba(color),
        style: { text: font(theme, size, weight, color), ...style },
      }),
    button: (key, label, disabled = false, primary = false, style = {}) =>
      ui.desktop.button({
        key,
        label,
        disabled,
        style: {
          text: font(
            theme,
            14,
            500,
            disabled ? theme.muted : primary ? theme.background : theme.accent,
          ),
          background: disabled ? theme.surface : primary ? theme.accent : 0,
          padding: { top: 8, right: 12, bottom: 8, left: 12 },
          radius: 8,
          ...style,
        },
        hover: disabled ? {} : { background: primary ? theme.accent : theme.selected },
        focus: { border: theme.accent, borderWidth: 2 },
        active: disabled ? {} : { background: primary ? theme.accent : theme.selected },
      }),
    dialog: (key, label, background, content, open) =>
      ui.desktop.dialog({
        key,
        label,
        background,
        content,
        open,
        style: {
          width: pixels(560),
          padding: { top: 20, right: 20, bottom: 20, left: 20 },
          background: theme.surface,
          radius: 16,
          border: theme.border,
          borderWidth: 1,
        },
      }),
    box: (key, children, options = {}) =>
      ui.desktop.stack({
        key,
        direction: options.row ? 'row' : 'column',
        gap: options.gap ?? 12,
        padding: options.padding ?? 0,
        background: rgba(options.surface ? theme.surface : theme.background),
        children,
        visible: options.visible ?? true,
        shortcuts: options.shortcuts ?? [],
        style: options.style ?? {},
        scroll: options.scroll ?? 'none',
      }),
  };
}
function terminalPaint(theme: Theme): Painter<TerminalNode> {
  return {
    focusButton: (key, label, child) =>
      ui.terminal.button({ key, label, focus_child: child, color: rgba(theme.muted) }),
    text: (key, text, _size = 15, color = theme.text, _weight = 400, style = {}) =>
      ui.terminal.text({ key, text, color: rgba(color), truncate: style.truncate ?? false }),
    button: (key, label, disabled = false) =>
      ui.terminal.button({
        key,
        label,
        disabled,
        color: rgba(disabled ? theme.muted : theme.accent),
      }),
    dialog: (key, label, background, content, open) =>
      ui.terminal.dialog({
        key,
        label,
        background,
        content,
        open,
        width: 72,
        height: 20,
        padding: 1,
        background_color: rgba(theme.surface),
        border_color: rgba(theme.border),
      }),
    box: (key, children, options = {}) =>
      ui.terminal.stack({
        key,
        direction: options.row ? 'row' : 'column',
        gap: options.gap ?? 1,
        padding: options.padding ?? 0,
        background: rgba(options.surface ? theme.surface : theme.background),
        children,
        visible: options.visible ?? true,
        shortcuts: options.shortcuts ?? [],
        focus_shortcuts: options.focusShortcuts ?? [],
        viewport: options.viewport ?? false,
        layout: options.terminal ?? {},
        scroll: options.scroll ?? 'none',
      }),
  };
}
export const presentations: {
  desktop: Presentation<DesktopNode>;
  tui: Presentation<TerminalNode>;
} = {
  desktop: {
    target: 'desktop',
    paint: desktopPaint,
    slot: (key, binding, props, on) => ui.desktop.component({ key, binding, props, on }),
    reading: (key, props, on) => ui.desktop.reading({ key, binding: 'reading', props, on }),
    popup: ui.desktop.popup,
  },
  tui: {
    target: 'tui',
    paint: terminalPaint,
    slot: (key, binding, props, on) => ui.terminal.component({ key, binding, props, on }),
    reading: (key, props, on) => ui.terminal.reading({ key, binding: 'reading', props, on }),
  },
};
export function output<S extends { revision: number }, N extends Node>(
  state: S,
  view: N,
  events: Emission[] = [],
): Output<S, N> {
  return {
    frame: { state, revision: state.revision, surface: null, view },
    events,
    actions: [],
    subscriptions: [],
  };
}
export const fill: DesktopDimension = { unit: 'fraction', value: 1 };
export const pixels = (value: number): DesktopDimension => ({ unit: 'pixels', value });

// Disposable labels must not turn one catalog entry into an unbounded terminal
// layout. The full immutable identity/configuration remains in the domain DTO.
export function compactLabel(value: string, limit: number): string {
  const characters = [...value.replace(/\s+/gu, ' ').trim()];
  return characters.length > limit
    ? characters.slice(0, limit - 1).join('') + '…'
    : characters.join('');
}
