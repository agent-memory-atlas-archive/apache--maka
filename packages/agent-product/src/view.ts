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

import type { ComponentSlot, Emission, Output } from '../../plugin-sdk/src/index.js';
import type { Theme } from './contracts.js';
export type Target = 'desktop' | 'tui';
export type Node = Record<string, unknown>;
export function paint(target: Target, theme: Theme) {
  const desktop = target === 'desktop';
  const rgba = (n: number) => n >>> 8;
  const font = (size: number, weight = 400, color = theme.text) => ({
    font: { family: '.SystemUIFont', fallbacks: [], size, lineHeight: 1.55, weight },
    foreground: color,
  });
  return {
    text: (key: string, text: string, size = 15, color = theme.text, weight = 400): Node => ({
      kind: 'text',
      key,
      text,
      color: rgba(color),
      ...(desktop ? { style: { text: font(size, weight, color) } } : {}),
    }),
    button: (key: string, label: string, disabled = false, primary = false): Node =>
      !desktop && disabled
        ? { kind: 'text', key, text: label, color: rgba(theme.muted) }
        : {
            kind: 'button',
            key,
            label,
            ...(desktop
              ? {
                  disabled,
                  style: {
                    text: font(
                      14,
                      500,
                      disabled ? theme.muted : primary ? theme.background : theme.accent,
                    ),
                    background: disabled ? theme.surface : primary ? theme.accent : 0,
                    padding: { top: 8, right: 12, bottom: 8, left: 12 },
                    radius: 8,
                  },
                  hover: disabled ? {} : { background: primary ? theme.accent : theme.selected },
                  focus: { border: theme.accent, borderWidth: 2 },
                  active: disabled ? {} : { background: theme.selected },
                }
              : {}),
          },
    box: (
      key: string,
      children: Node[],
      options: {
        row?: boolean;
        gap?: number;
        padding?: number;
        surface?: boolean;
        style?: Node;
        scroll?: boolean;
        visible?: boolean;
      } = {},
    ): Node => ({
      kind: 'stack',
      key,
      direction: options.row ? 'row' : 'column',
      gap: options.gap ?? (desktop ? 12 : 1),
      padding: options.padding ?? 0,
      background: rgba(options.surface ? theme.surface : theme.background),
      children,
      visible: options.visible ?? true,
      ...(desktop ? { style: options.style ?? {}, scroll: options.scroll ?? false } : {}),
    }),
  };
}
export function slot(
  key: string,
  binding: string,
  props: unknown,
  on: Record<string, string>,
): Node {
  return { kind: 'component', ...({ key, binding, member: 0, props, on } satisfies ComponentSlot) };
}
export function output<S extends { revision: number }>(
  state: S,
  view: Node,
  events: Emission[] = [],
): Output<S> {
  return {
    frame: { state, revision: state.revision, surface: null, view },
    events,
    actions: [],
    subscriptions: [],
  };
}
export const fill = { unit: 'fraction', value: 1 };
export const pixels = (value: number) => ({ unit: 'pixels', value });
