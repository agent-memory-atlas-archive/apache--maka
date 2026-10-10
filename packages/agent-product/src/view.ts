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
  TerminalColor,
  DesktopStyle,
  DesktopDimension,
  DesktopProps,
  Emission,
  Output,
  TerminalLayout,
  TerminalResponsiveStack,
  Shortcut,
  FocusShortcut,
  TextAppearance,
  ReadingProps,
  TerminalChoiceOption,
  TerminalPaneCommand,
} from '../../plugin-sdk/src/index.js';
import { animation, type ActivityAnimation } from './motion.js';
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
  wide?: TerminalResponsiveStack;
  background?: number;
  viewport?: boolean;
  focusGroup?: boolean;
  autoFocus?: boolean;
  wheel?: { up: string; down: string };
  border?: boolean;
  busy?: boolean;
  shortcuts?: Shortcut[];
  focusShortcuts?: FocusShortcut[];
}
interface Painter<N extends Node> {
  activity(
    key: string,
    kind: ActivityAnimation,
    foreground?: number,
    prefix?: string,
    suffix?: string,
  ): N;
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
    copyText?: string,
    hint?: string,
  ): N;
  contextMenu?(
    key: string,
    label: string,
    background: N,
    actions: N[],
    copySelectionLabel?: string,
  ): N;
  choice?(
    key: string,
    label: string,
    options: TerminalChoiceOption[],
    current: string | null,
    disabled: boolean,
  ): N;
  item?(
    key: string,
    content: N,
    current: boolean,
    disabled: boolean,
    interaction?: {
      followFocus?: boolean;
      submit?: boolean;
      submitSpace?: boolean;
    },
  ): N;
  focusButton?(key: string, label: string, child: string): N;
  adaptivePane?(
    key: string,
    label: string,
    background: N,
    content: N,
    command: TerminalPaneCommand | null,
  ): N;
  dialog(
    key: string,
    label: string,
    background: N,
    content: N,
    open: boolean,
    style?: DesktopStyle,
    terminalSize?: { width?: number; height?: number; padding?: number },
  ): N;
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
    activity: (key, kind, foreground = theme.accent, prefix = '', suffix = '') =>
      ui.desktop.text({
        key,
        text: animation(kind, true, false, prefix, suffix)[0]!.text,
        color: rgba(foreground),
        style: { text: font(theme, 14, 400, foreground) },
      }),
    text: (key, text, size = 15, color = theme.text, weight = 400, style = {}) =>
      ui.desktop.text({
        key,
        text,
        color: rgba(color),
        style: { text: font(theme, size, weight, color), ...style },
      }),
    button: (key, label, disabled = false, primary = false, style = {}, copyText, hint = '') =>
      ui.desktop.button({
        key,
        label,
        disabled,
        copy_text: copyText,
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
    dialog: (key, label, background, content, open, style = {}) =>
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
          ...style,
        },
      }),
    box: (key, children, options = {}) =>
      ui.desktop.stack({
        key,
        direction: options.row ? 'row' : 'column',
        gap: options.gap ?? 12,
        padding: options.padding ?? 0,
        background: rgba(
          options.background ?? (options.surface ? theme.surface : theme.background),
        ),
        children,
        visible: options.visible ?? true,
        shortcuts: options.shortcuts ?? [],
        style: options.style ?? {},
        scroll: options.scroll ?? 'none',
      }),
  };
}
function terminalPaint(theme: Theme): Painter<TerminalNode> {
  const color = (value: number): TerminalColor => {
    const palette = theme.terminal;
    if (!palette) return rgba(value);
    const roles = {
      background: 'background',
      surface: 'surface',
      text: 'foreground',
      muted: 'muted',
      accent: 'accent',
      border: 'border',
      selected: 'selection',
      thinking: 'thinking',
      success: 'success',
      warning: 'warning',
      error: 'error',
      subtle: 'subtle',
      scrollbar: 'scrollbar',
      selectionText: 'selectionText',
      search: 'search',
      searchActive: 'searchActive',
      searchText: 'searchText',
    } as const;
    for (const [role, native] of Object.entries(roles)) {
      if (theme[role as keyof typeof roles] === value) return palette[native];
    }
    return rgba(value);
  };
  return {
    activity: (key, kind, foreground = theme.accent, prefix = '', suffix = '') => {
      const frames = animation(kind, theme.ascii, theme.motion, prefix, suffix);
      return ui.terminal.text({
        key,
        text: frames[0]!.text,
        color: color(foreground),
        animation: theme.motion ? frames : [],
        truncate: true,
      });
    },
    adaptivePane: (key, label, background, content, command) =>
      ui.terminal.adaptivePane({
        key,
        background,
        content,
        command,
        pane_width: 32,
        inline_min_width: 60,
        auto_inline_min_width: 90,
        motion: theme.motion,
        overlay: {
          ascii: theme.ascii,
          label,
          width: 72,
          height: 24,
          padding: 1,
          background_color: color(theme.background),
          border_color: color(theme.border),
          title_color: color(theme.text),
        },
      }),
    item: (key, content, current, disabled, interaction = {}) =>
      ui.terminal.item({
        key,
        content,
        current,
        disabled,
        selected_color: theme.terminal?.terminal ? undefined : color(theme.selected),
        selected_foreground: theme.terminal?.terminal ? undefined : color(theme.selectionText),
        follow_focus: interaction.followFocus ?? false,
        submit: interaction.submit ?? false,
        submit_space: interaction.submitSpace ?? false,
      }),
    contextMenu: (key, label, background, actions, copySelectionLabel) => {
      const buttons = (nodes: TerminalNode[]): TerminalNode[] =>
        nodes.flatMap((node) =>
          node.kind === 'stack'
            ? buttons(node.children)
            : node.kind === 'button'
              ? [
                  {
                    ...node,
                    align: 'start' as const,
                    background: color(theme.surface),
                    hover_background: color(theme.selected),
                  },
                ]
              : [node],
        );
      return ui.terminal.contextMenu({
        key,
        label,
        background,
        copy_selection_label: copySelectionLabel,
        content: ui.terminal.stack({
          key: key + '-actions',
          direction: 'column',
          children: buttons(actions),
          gap: 0,
          padding: 0,
        }),
        width: 32,
        height: 18,
        padding: 0,
        ascii: theme.ascii,
        background_color: color(theme.surface),
        border_color: color(theme.border),
        title_color: color(theme.text),
      });
    },
    choice: (key, label, options, current, disabled) =>
      ui.terminal.choice({
        key,
        label,
        options,
        current,
        disabled,
        ascii: theme.ascii,
        foreground: color(theme.text),
        background: color(theme.surface),
        border: color(theme.border),
        selected: color(theme.selected),
        accent: color(theme.accent),
      }),
    focusButton: (key, label, child) =>
      ui.terminal.button({
        key,
        label,
        focus_child: child,
        color: color(theme.muted),
        background: color(theme.surface),
        hover_background: color(theme.border),
      }),
    text: (key, text, _size = 15, foreground = theme.text, weight = 400, style = {}) =>
      ui.terminal.text({
        key,
        text,
        color: color(foreground),
        truncate: style.truncate ?? false,
        bold: weight >= 600,
        align: style.align === 'center' ? 'center' : style.align === 'end' ? 'end' : 'start',
      }),
    button: (key, label, disabled = false, primary = false, style = {}, copyText, hint = '') =>
      ui.terminal.button({
        key,
        label,
        disabled,
        copy_text: copyText,
        hint,
        color: color(disabled ? theme.muted : primary ? theme.accent : theme.text),
        align:
          (style.align ?? style.justify) === 'start'
            ? 'start'
            : (style.align ?? style.justify) === 'end'
              ? 'end'
              : 'center',
        background: style.background === 0 ? undefined : color(style.background ?? theme.surface),
        hover_background:
          style.background === 0 || theme.terminal?.terminal ? undefined : color(theme.border),
        bold: primary,
      }),
    dialog: (key, label, background, content, open, _style, terminalSize = {}) =>
      ui.terminal.dialog({
        ascii: theme.ascii,
        key,
        label,
        background,
        content,
        open,
        width: terminalSize.width ?? 72,
        height: terminalSize.height ?? 20,
        padding: terminalSize.padding ?? 1,
        background_color: color(theme.background),
        border_color: color(theme.border),
        title_color: color(theme.text),
      }),
    box: (key, children, options = {}) =>
      ui.terminal.stack({
        key,
        direction: options.row ? 'row' : 'column',
        wide: options.wide ?? null,
        gap: options.gap ?? 1,
        padding: options.padding ?? 0,
        background:
          options.background !== undefined
            ? color(options.background)
            : options.surface
              ? color(theme.surface)
              : undefined,
        children,
        visible: options.visible ?? true,
        shortcuts: options.shortcuts ?? [],
        focus_shortcuts: options.focusShortcuts ?? [],
        viewport: options.viewport ?? false,
        focus_group: options.focusGroup ?? false,
        auto_focus: options.autoFocus ?? false,
        wheel: options.wheel ?? null,
        border: options.border
          ? {
              breathing: options.busy === true && theme.motion,
              color: color(options.busy ? theme.accent : theme.border),
              focused_color: color(theme.accent),
              style: theme.ascii ? 'ascii' : 'rounded',
            }
          : null,
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
