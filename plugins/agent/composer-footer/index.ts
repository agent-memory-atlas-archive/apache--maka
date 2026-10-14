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

import type {
  AgentSessionMetrics,
  Component,
  AgentDataSource,
} from '../../../packages/plugin-sdk/src/index.js';
import type { Theme } from '../../shared/contracts.js';
import { composerFooterContract } from '../../shared/contracts.js';
import { tuiText } from '../../shared/tui-locale.js';
import { output, type Node, type Presentation } from '../../shared/view.js';

interface Props {
  theme: Theme;
  locale: string;
  sessionId: string | null;
  dataSource: AgentDataSource | null;
  running: boolean;
  metrics: AgentSessionMetrics | null;
}
interface State {
  revision: number;
}
const tokens = (value: number) =>
  value >= 1_000_000
    ? `${(value / 1_000_000).toFixed(1)}M`
    : value >= 1000
      ? `${(value / 1000).toFixed(1).replace(/\.0$/, '')}k`
      : String(value);
export function footer<N extends Node>(presentation: Presentation<N>): Component<State, Props, N> {
  const frame = (state: State, props: Props) => {
    const { metrics, theme, locale } = props;
    const parts: string[] = [];
    const used = metrics?.contextTokens;
    const capacity = metrics?.contextWindow;
    if (used !== null && used !== undefined) {
      const amount = `${metrics?.approximate ? '≈' : ''}${tokens(used)}`;
      parts.push(
        `${tuiText(locale, 'chat-context')} ${amount}${capacity ? ` / ${tokens(capacity)} · ${Math.round((100 * used) / capacity)}%` : ''}`,
      );
    }
    const rate = metrics?.outputTokensPerSecond;
    if (rate !== null && rate !== undefined && Number.isFinite(rate))
      parts.push(`${rate.toFixed(1)} tok/s`);
    const ui = presentation.paint(theme);
    const hint = tuiText(locale, 'chat-metrics-hint');
    const line = (key: string, text: string) =>
      ui.text(key, text, 12, theme.muted, 400, { truncate: true }, hint);
    if (presentation.target === 'desktop')
      return output(state, line('session-metrics-text', parts.join(' · ')));
    const layout = (basis: number) => ({
      basis,
      grow: 0,
      shrink: false,
      wrap: false,
      basis_fraction: null,
    });
    // Native layout uses the width left by the controls, including sidebar changes.
    // Zero-height alternatives do not paint or participate in hit testing.
    const compact =
      used != null
        ? capacity
          ? `${metrics?.approximate ? '≈' : ''}${Math.round((100 * used) / capacity)}%`
          : `${metrics?.approximate ? '≈' : ''}${tokens(used)}`
        : '';
    const context = parts[0] && used != null ? parts[0] : '';
    const reduced = ui.box(
      'metrics-reduced',
      [
        ui.box('metrics-percent', [line('metrics-percent-text', compact)], {
          terminal: layout(compact ? 1 : 0),
        }),
        ui.box('metrics-context', [line('metrics-context-text', context)], { terminal: layout(0) }),
      ],
      {
        gap: 0,
        terminal: layout(1),
        wide: {
          min_width: 32,
          direction: 'column',
          children: {
            'metrics-percent': layout(0),
            'metrics-context': layout(context ? 1 : 0),
          },
        },
      },
    );
    return output(
      state,
      ui.box(
        'metrics-responsive',
        [
          reduced,
          ui.box('metrics-full', [line('session-metrics-text', parts.join(' · '))], {
            terminal: layout(0),
          }),
        ],
        {
          gap: 0,
          wide: {
            min_width: 48,
            direction: 'column',
            children: {
              'metrics-reduced': layout(0),
              'metrics-full': layout(parts.length ? 1 : 0),
            },
          },
        },
      ),
    );
  };
  return {
    mount: (props) => frame({ revision: 0 }, props),
    update: (state, props) => frame({ revision: state.revision + 1 }, props),
    event: (state, _event, props) => frame({ revision: state.revision + 1 }, props),
  };
}
export function activate() {
  return {
    Footer: { kind: 'component', contract: composerFooterContract, component: ['component'] },
  };
}
