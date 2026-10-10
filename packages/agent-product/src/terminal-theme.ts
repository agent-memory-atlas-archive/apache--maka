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

import type { TerminalPalette } from '../../plugin-sdk/src/index.js';
export type ThemeName = 'maka' | 'dusk' | 'paper' | 'terminal' | 'custom';
export interface CustomTheme {
  version: 1;
  name: string;
  base?: 'maka' | 'dusk' | 'paper';
  colors?: Partial<Record<string, string>>;
  syntax?: Partial<Record<string, string>>;
}
export interface AppearanceConfiguration {
  scheme: 'light' | 'dark';
  theme?: ThemeName;
  customTheme?: CustomTheme;
  motion?: boolean;
  ascii?: boolean;
}
// Values copied from the original TUI theme.rs. ANSI names intentionally remain
// terminal attributes: Terminal follows the user's terminal palette and defaults.
export const palettes: Record<Exclude<ThemeName, 'custom'>, TerminalPalette> = {
  maka: {
    background: 1251104,
    foreground: 14804976,
    surface: 2107447,
    accent: 7448829,
    thinking: 12297182,
    success: 8310692,
    warning: 15187327,
    error: 15504283,
    muted: 10530755,
    subtle: 8820139,
    border: 3424856,
    scrollbar: 6980517,
    selection: 3165557,
    selectionText: 15857407,
    search: 4932143,
    searchActive: 15187327,
    searchText: 1251104,
    syntax: [12492279, 9358503, 8753570, 15382914, 7448829, 7458001, 12634585],
    terminal: false,
  },
  dusk: {
    background: 1513756,
    foreground: 14278115,
    surface: 2237996,
    accent: 10206159,
    thinking: 11840456,
    success: 10470831,
    warning: 13941387,
    error: 14527143,
    muted: 9674412,
    subtle: 8292761,
    border: 3884369,
    scrollbar: 7635863,
    selection: 3491417,
    selectionText: 16054010,
    search: 4800816,
    searchActive: 13941387,
    searchText: 1513756,
    syntax: [12693463, 11257777, 9608620, 13941387, 10731480, 10208968, 13028307],
    terminal: false,
  },
  paper: {
    background: 16251132,
    foreground: 2569031,
    surface: 15330805,
    accent: 2121665,
    thinking: 7362715,
    success: 2191695,
    warning: 9198611,
    error: 11746888,
    muted: 5990524,
    subtle: 6582662,
    border: 12963293,
    scrollbar: 8819882,
    selection: 13820671,
    selectionText: 1457788,
    search: 15984059,
    searchActive: 8476160,
    searchText: 16777215,
    syntax: [8408239, 2387016, 6188162, 9984018, 2121665, 1339512, 5004402],
    terminal: false,
  },
  terminal: {
    background: 'reset',
    foreground: 'reset',
    surface: 'reset',
    accent: 'light_blue',
    thinking: 'reset',
    success: 'green',
    warning: 'yellow',
    error: 'red',
    muted: 'gray',
    subtle: 'dark_gray',
    border: 'dark_gray',
    scrollbar: 'gray',
    selection: 'cyan',
    selectionText: 'black',
    search: 'dark_gray',
    searchActive: 'yellow',
    searchText: 'black',
    syntax: ['magenta', 'green', 'gray', 'yellow', 'blue', 'cyan', 'cyan'],
    terminal: true,
  },
};
export const roles = [
  'background',
  'foreground',
  'surface',
  'accent',
  'thinking',
  'success',
  'warning',
  'error',
  'muted',
  'subtle',
  'border',
  'scrollbar',
  'selection',
  'selection-text',
  'search',
  'search-active',
  'search-text',
  'keyword',
  'string',
  'comment',
  'number',
  'function',
  'type',
  'operator',
];
export function terminalPalette(configuration: AppearanceConfiguration): TerminalPalette {
  if (
    !['light', 'dark'].includes(configuration.scheme) ||
    (configuration.motion !== undefined && typeof configuration.motion !== 'boolean') ||
    (configuration.ascii !== undefined && typeof configuration.ascii !== 'boolean')
  )
    throw new Error('Invalid appearance settings');
  const name = configuration.theme ?? (configuration.scheme === 'light' ? 'paper' : 'maka');
  if (name !== 'custom') {
    if (!Object.hasOwn(palettes, name)) throw new Error('Unknown theme');
    return JSON.parse(JSON.stringify(palettes[name]));
  }
  const custom = configuration.customTheme;
  if (
    !custom ||
    custom.version !== 1 ||
    !custom.name.trim().replace(/[\p{Mark}\p{Format}]/gu, '') ||
    [...custom.name].length > 40 ||
    /[\p{Control}\u202a-\u202e\u2066-\u2069]/u.test(custom.name) ||
    !['maka', 'dusk', 'paper'].includes(custom.base ?? 'maka') ||
    JSON.stringify(custom).length > 16384
  )
    throw new Error('Invalid custom theme');
  if (
    Object.keys(custom).some(
      (key) => !['version', 'name', 'base', 'colors', 'syntax'].includes(key),
    )
  )
    throw new Error('Unknown custom theme field');
  for (const [section, allowed] of [
    [custom.colors, roles.slice(0, 17)],
    [custom.syntax, roles.slice(17)],
  ] as const) {
    if (
      section !== undefined &&
      (!section ||
        typeof section !== 'object' ||
        Array.isArray(section) ||
        Object.keys(section).some((key) => !allowed.includes(key)))
    )
      throw new Error('Unknown custom theme color');
  }
  const palette = JSON.parse(JSON.stringify(palettes[custom.base ?? 'maka']));
  for (const [role, value] of Object.entries({ ...custom.colors, ...custom.syntax })) {
    const index = roles.indexOf(role);
    if (index < 0 || typeof value !== 'string' || !/^#[0-9a-f]{6}$/i.test(value))
      throw new Error(`Invalid custom theme color: ${role}`);
    const color = Number.parseInt(value.slice(1), 16);
    if (index >= 17) palette.syntax[index - 17] = color;
    else
      (palette as unknown as Record<string, unknown>)[
        role.replace(/-([a-z])/g, (_, c: string) => c.toUpperCase())
      ] = color;
  }
  return palette;
}
export const appearanceConfigurationSchema = {
  type: 'object',
  additionalProperties: false,
  required: ['scheme'],
  properties: {
    scheme: { enum: ['light', 'dark'] },
    theme: { enum: ['maka', 'dusk', 'paper', 'terminal', 'custom'] },
    motion: { type: 'boolean' },
    ascii: { type: 'boolean' },
    customTheme: {
      type: 'object',
      additionalProperties: false,
      required: ['version', 'name'],
      properties: {
        version: { const: 1 },
        name: { type: 'string', minLength: 1, maxLength: 40 },
        base: { enum: ['maka', 'dusk', 'paper'] },
        colors: {
          type: 'object',
          additionalProperties: false,
          properties: Object.fromEntries(
            roles
              .slice(0, 17)
              .map((role) => [role, { type: 'string', pattern: '^#[0-9a-fA-F]{6}$' }]),
          ),
        },
        syntax: {
          type: 'object',
          additionalProperties: false,
          properties: Object.fromEntries(
            roles.slice(17).map((role) => [role, { type: 'string', pattern: '^#[0-9a-fA-F]{6}$' }]),
          ),
        },
      },
    },
  },
};
