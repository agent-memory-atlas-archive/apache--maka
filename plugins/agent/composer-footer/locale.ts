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

import { translate } from '../../shared/tui-locale.js';
import { tui_chat_context, tui_chat_metrics_hint } from '../../shared/tui-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

import type { Catalog } from '../../shared/locale.js';

const terminalMessages = {
  'chat-context': tui_chat_context,
  'chat-compacting': ['Compressing context', '正在压缩上下文', '正在壓縮上下文'],
  'chat-context-last': ['last', '上次', '上次'],
  'chat-context-last-hint': [
    'Last known context usage; newer additions are not yet counted.',
    '上次已知的上下文用量；尚未计入后续新增内容。',
    '上次已知的上下文用量；尚未計入後續新增內容。',
  ],
  'chat-metrics-hint': tui_chat_metrics_hint,
} satisfies Catalog;
export function tuiText(
  locale: string,
  key: string,
  values: Record<string, string | number> = {},
): string {
  return translate(locale, terminalMessages, key, values);
}
