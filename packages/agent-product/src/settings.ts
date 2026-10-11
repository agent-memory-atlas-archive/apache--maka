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

import type { ComponentInput } from '../../plugin-sdk/src/index.js';
import type { Theme } from './contracts.js';
import { saveAppearanceSetting, type AppearanceEditor } from './appearance-editor.js';
import { tuiText } from './tui-locale.js';
import type { ThemeName } from './terminal-theme.js';
import type { Node, Presentation } from './view.js';

export const settingsCategories = ['appearance', 'interface', 'models', 'plugins'] as const;
export type SettingsCategory = (typeof settingsCategories)[number];
interface SettingsState {
  preferences: AppearanceEditor;
  settingsCategory: SettingsCategory;
  pages: ('settings' | 'connections' | 'deployment' | 'projects' | 'imports')[];
}
export function settingsEvent(state: SettingsState, input: ComponentInput, base: Theme): boolean {
  if (input.kind !== 'raw') return false;
  const category = input.key.slice('settings-category-'.length) as SettingsCategory;
  if (input.key.startsWith('settings-category-') && settingsCategories.includes(category)) {
    state.settingsCategory = category;
    return true;
  }
  if (input.key === 'settings-connections' || input.key === 'settings-plugins') {
    state.pages.push(input.key === 'settings-connections' ? 'connections' : 'deployment');
    return true;
  }
  if (input.key === 'settings-retry') {
    state.preferences.command = { kind: 'read', sequence: ++state.preferences.sequence };
    state.preferences.error = null;
    return true;
  }
  const key = (input.event as { key?: string }).key;
  if (!key) return false;
  if (
    input.key === 'settings-palette' &&
    ['maka', 'dusk', 'paper', 'terminal', 'custom'].includes(key)
  )
    saveAppearanceSetting(state.preferences, base, (next) => {
      next.appearance.theme = key as ThemeName;
    });
  else if (input.key === 'settings-language' && ['auto', 'en', 'zh-CN', 'zh-TW'].includes(key))
    saveAppearanceSetting(state.preferences, base, (next) => {
      next.locale = key;
    });
  else if (input.key === 'settings-symbols' && ['ascii', 'unicode'].includes(key))
    saveAppearanceSetting(state.preferences, base, (next) => {
      next.appearance.ascii = key === 'ascii';
    });
  else if (input.key === 'settings-motion' && ['on', 'off'].includes(key))
    saveAppearanceSetting(state.preferences, base, (next) => {
      next.appearance.motion = key === 'on';
    });
  else return false;
  return true;
}
export function settingsView<N extends Node>(
  presentation: Presentation<N>,
  state: AppearanceEditor,
  theme: Theme,
  locale: string,
  category: SettingsCategory,
): N {
  const ui = presentation.paint(theme),
    t = (key: string) => tuiText(locale, key);
  const appearance = state.saved?.appearance ?? theme.configuration;
  const busy = state.command !== null || state.checkpoint?.state.kind !== 'ready';
  const choice = (key: string, options: [string, string][], current: string) =>
    ui.choice!(
      key,
      t(key),
      options.map(([key, label]) => ({ key, label })),
      current,
      busy,
    );
  const title = (key: SettingsCategory) =>
    t(key === 'plugins' ? 'route-plugins' : 'settings-' + key);
  const sections: Record<SettingsCategory, N[]> = {
    appearance: [
      choice(
        'settings-palette',
        [
          ['maka', t('settings-palette-dark')],
          ['dusk', t('settings-palette-dusk')],
          ['paper', t('settings-palette-paper')],
          ['terminal', t('settings-palette-default')],
          ...(appearance.customTheme
            ? [['custom', appearance.customTheme.name] as [string, string]]
            : []),
        ],
        appearance.theme ?? (appearance.scheme === 'light' ? 'paper' : 'maka'),
      ),
      ui.button('appearance-open', t('theme-customize'), busy, false, {
        background: 0,
        align: 'start',
      }),
    ],
    interface: [
      choice(
        'settings-language',
        [
          ['auto', tuiText(locale, 'language-auto', { language: locale })],
          ['en', 'English'],
          ['zh-CN', '简体中文'],
          ['zh-TW', '繁體中文'],
        ],
        state.saved?.locale ?? 'auto',
      ),
      choice(
        'settings-symbols',
        [
          ['unicode', t('settings-symbols-unicode')],
          ['ascii', t('settings-symbols-ascii')],
        ],
        appearance.ascii ? 'ascii' : 'unicode',
      ),
      choice(
        'settings-motion',
        [
          ['on', t('settings-motion-on')],
          ['off', t('settings-motion-off')],
        ],
        appearance.motion === false ? 'off' : 'on',
      ),
    ],
    models: [
      ui.button('settings-connections', t('route-connections'), false, false, {
        background: 0,
        align: 'start',
      }),
    ],
    plugins: [
      ui.button('settings-plugins', t('route-plugins'), false, false, {
        background: 0,
        align: 'start',
      }),
    ],
  };
  return ui.box(
    'settings-page',
    [
      ...(state.error ? [ui.text('settings-error', t(state.error), 13, theme.warning)] : []),
      ...(state.checkpoint?.state.kind === 'failed'
        ? [
            ui.text('settings-save-failed', t('state-save-failed'), 13, theme.warning),
            ui.button('settings-retry', t('home-retry')),
          ]
        : []),
      ui.box(
        'settings-sections',
        [
          ui.box(
            'settings-categories',
            settingsCategories.map((key) =>
              ui.item!(
                'settings-category-' + key,
                ui.text('settings-category-label-' + key, title(key), 14, theme.text),
                category === key,
                false,
              ),
            ),
            { gap: 0, terminal: { basis: 0 } },
          ),
          ...settingsCategories.map((key) =>
            ui.box(
              'settings-section-' + key,
              [
                ui.text('settings-heading-' + key, title(key), 14, theme.muted, 550),
                ...sections[key],
              ],
              { gap: 1, padding: 1 },
            ),
          ),
        ],
        {
          gap: 0,
          terminal: { grow: 1 },
          wide: {
            min_width: 56,
            direction: 'row',
            children: {
              'settings-categories': {
                basis_fraction: { numerator: 1, denominator: 3, min: 0, max: 22 },
              },
              ...Object.fromEntries(
                settingsCategories.map((key) => [
                  'settings-section-' + key,
                  key === category ? { grow: 1, basis: 0 } : { basis: 0 },
                ]),
              ),
            },
          },
        },
      ),
    ],
    {
      gap: 0,
      padding: 1,
      scroll: 'manual',
      terminal: { grow: 1 },
    },
  );
}
