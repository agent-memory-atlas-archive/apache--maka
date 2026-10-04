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
import type { UiExport, FontAppearance } from '../../plugin-sdk/src/index.js';
export function appearance(scheme: 'light' | 'dark'): Theme {
  const dark = scheme === 'dark';
  const background = dark ? 0x1b1d1bff : 0xfaf9f6ff,
    surface = dark ? 0x232623ff : 0xf0efebff,
    text = dark ? 0xe8eae6ff : 0x242724ff,
    muted = dark ? 0xa3aaa1ff : 0x656a63ff,
    accent = dark ? 0xa7c4b0ff : 0x416653ff,
    border = dark ? 0x3d433bff : 0xd8ddd5ff,
    selected = dark ? 0x3c5142ff : 0xe2eadfff;
  const font: FontAppearance = {
    family: '.SystemUIFont',
    fallbacks: [],
    size: 16,
    lineHeight: 1.55,
    weight: 400,
  };
  const textStyle = { font, foreground: text };
  const thumb = { color: muted, width: 8, inset: 3, radius: 3, minLength: 40 };
  const insets = (n: number) => ({ top: n, right: n, bottom: n, left: n });
  return {
    background,
    surface,
    text,
    muted,
    accent,
    border,
    selected,
    textarea: {
      text: textStyle,
      mutedForeground: muted,
      background,
      selection: dark ? 0xa7c4b040 : 0x41665340,
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
export function activate(configuration: { scheme: 'light' | 'dark' }): Record<string, UiExport> {
  return {
    Appearance: { kind: 'value', contract: themeContract, value: appearance(configuration.scheme) },
  };
}
