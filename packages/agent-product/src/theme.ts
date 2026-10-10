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

import { type Theme, themeContract } from './contracts.js';
import { palettes, terminalPalette, type AppearanceConfiguration } from './terminal-theme.js';
import type { UiExport, FontAppearance } from '../../plugin-sdk/src/index.js';
export function appearance(
  scheme: 'light' | 'dark',
  terminal = false,
  configuration: AppearanceConfiguration = { scheme },
): Theme {
  const palette = terminalPalette(configuration);
  const rgbPalette = palette.terminal ? palettes.maka : palette;
  const packed = (color: number | string): number => ((Number(color) << 8) | 255) >>> 0;
  const selectedTheme = configuration.theme ?? (scheme === 'light' ? 'paper' : 'maka');
  const dark = terminal
    ? selectedTheme === 'custom'
      ? configuration.customTheme?.base !== 'paper'
      : selectedTheme !== 'paper'
    : scheme === 'dark';
  // Original TUI Maka/Paper roles. Target-specific presentation is an ordinary
  // product value; the native kernel does not choose an application theme.
  const colors = terminal
    ? [
        rgbPalette.background,
        rgbPalette.surface,
        rgbPalette.foreground,
        rgbPalette.muted,
        rgbPalette.accent,
        rgbPalette.border,
        rgbPalette.selection,
      ].map(packed)
    : dark
      ? [0x1b1d1bff, 0x232623ff, 0xe8eae6ff, 0xa3aaa1ff, 0xa7c4b0ff, 0x3d433bff, 0x3c5142ff]
      : [0xfaf9f6ff, 0xf0efebff, 0x242724ff, 0x656a63ff, 0x416653ff, 0xd8ddd5ff, 0xe2eadfff];
  const [background, surface, text, muted, accent, border, selected] = colors as [
    number,
    number,
    number,
    number,
    number,
    number,
    number,
  ];
  const selection = terminal ? selected : dark ? 0xa7c4b040 : 0x41665340;
  const font: FontAppearance = {
    family: '.SystemUIFont',
    fallbacks: [],
    size: 16,
    lineHeight: 1.55,
    weight: 400,
  };
  const textStyle = { font, foreground: text };
  const heading = (scale: number) => ({
    font: { ...font, size: font.size * scale, weight: 650 },
    foreground: text,
  });
  const thumb = { color: muted, width: 8, inset: 3, radius: 3, minLength: 40 };
  const insets = (n: number) => ({ top: n, right: n, bottom: n, left: n });
  return {
    configuration: JSON.parse(JSON.stringify(configuration)),
    terminal: terminal ? palette : null,
    motion: configuration.motion !== false,
    ascii: configuration.ascii === true,
    thinking: packed(rgbPalette.thinking),
    success: packed(rgbPalette.success),
    warning: packed(rgbPalette.warning),
    error: packed(rgbPalette.error),
    subtle: packed(rgbPalette.subtle),
    scrollbar: packed(rgbPalette.scrollbar),
    selectionText: packed(rgbPalette.selectionText),
    search: packed(rgbPalette.search),
    searchActive: packed(rgbPalette.searchActive),
    searchText: packed(rgbPalette.searchText),
    background,
    surface,
    text,
    muted,
    accent,
    border,
    selected,
    markdown: {
      terminal: terminal ? palette : null,
      body: textStyle,
      headings: [
        heading(1.75),
        heading(1.5),
        heading(1.25),
        heading(1.1),
        heading(1.1),
        heading(1.1),
      ],
      code: {
        font: {
          ...font,
          family: 'monospace',
          fallbacks: ['Menlo', 'Consolas', 'DejaVu Sans Mono'],
          size: 14,
        },
        foreground: text,
      },
      muted,
      link: accent,
      selection,
      codeBackground: surface,
      tableBackground: background,
      border,
      dark,
      paragraphGap: 16,
    },
    input: {
      ascii: configuration.ascii === true,
      terminal: terminal ? palette : null,
      text: { ...textStyle, font: { ...font, size: 14 } },
      mutedForeground: muted,
      background,
      selection,
      caret: text,
      border,
      focusedBorder: accent,
      borderWidth: 1,
      radius: 8,
      padding: { top: 10, right: 12, bottom: 10, left: 12 },
    },
    textarea: {
      ascii: configuration.ascii === true,
      terminal: terminal ? palette : null,
      text: textStyle,
      mutedForeground: muted,
      background,
      selection,
      caret: text,
      border,
      focusedBorder: accent,
      borderWidth: 1,
      radius: 12,
      padding: insets(12),
      scrollbar: {
        mode: 'hover',
        entrance: 'fade',
        idleMillis: 2000,
        enterMillis: 120,
        exitMillis: 150,
        expandMillis: 120,
        trackWidth: 14,
        track: 0,
        trackHover: 0,
        trackActive: 0,
        thumb,
        thumbHover: thumb,
        thumbActive: thumb,
      },
    },
    menu: {
      text: { ...textStyle, font: { ...font, size: 14 } },
      background,
      border,
      activeBackground: accent,
      activeForeground: background,
      disabledForeground: muted,
      borderWidth: 1,
      radius: 8,
      padding: insets(4),
      rowPadding: { top: 7, right: 12, bottom: 7, left: 12 },
      width: 220,
      windowMargin: 8,
    },
  };
}
export function activate(
  configuration: AppearanceConfiguration,
  terminal = false,
): Record<string, UiExport> {
  return {
    Appearance: {
      kind: 'value',
      contract: themeContract,
      value: appearance(configuration.scheme, terminal, configuration),
    },
  };
}
