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

import { type Catalog, createCopy } from '../../shared/locale.js';
import {
  msg_back,
  msg_executionReceipt,
  msg_refreshDefaultOperations,
  msg_settingOff,
  msg_settingOn,
  msg_settings,
} from '../../shared/locale-messages.js';
import { translate } from '../../shared/tui-locale.js';
import {
  tui_assistant_conflict,
  tui_assistant_instructions,
  tui_assistant_shell_titles,
  tui_assistant_name,
  tui_assistant_privacy,
  tui_assistant_reload,
  tui_assistant_saved,
  tui_assistant_unavailable,
  tui_assistant_unknown,
  tui_settings_assistant,
} from '../../shared/tui-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  back: msg_back,
  executionReceipt: msg_executionReceipt,
  refreshDefaultOperations: msg_refreshDefaultOperations,
  settingOff: msg_settingOff,
  settingOn: msg_settingOn,
  settings: msg_settings,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;

const terminalMessages = {
  'assistant-compaction': ['Compression prompt', '压缩提示词', '壓縮提示詞'],
  'assistant-compaction-save': ['Save compression prompt', '保存压缩提示词', '儲存壓縮提示詞'],
  'assistant-compaction-reset': ['Restore default prompt', '恢复默认提示词', '還原預設提示詞'],
  'assistant-conflict': tui_assistant_conflict,
  'assistant-instructions': tui_assistant_instructions,
  'assistant-shell-titles': tui_assistant_shell_titles,
  'assistant-invalid': [
    'Unable to save this draft. Check the name (maximum 256 characters) or reload the editor.',
    '无法保存此草稿。请检查称呼（最多 256 字符）或重新加载编辑器。',
    '無法儲存此草稿。請檢查稱呼（最多 256 字元）或重新載入編輯器。',
  ],
  'assistant-name': tui_assistant_name,
  'assistant-privacy': tui_assistant_privacy,
  'assistant-reload': tui_assistant_reload,
  'assistant-save': ['Save name', '保存称呼', '儲存稱呼'],
  'assistant-saved': tui_assistant_saved,
  'assistant-global': ['Global instructions', '全局指令', '全域指令'],
  'assistant-global-save': ['Save global instructions', '保存全局指令', '儲存全域指令'],
  'assistant-global-reload': [
    'Reload (discard edits)',
    '重新加载（放弃编辑）',
    '重新載入（捨棄編輯）',
  ],
  'assistant-global-saved': ['Global instructions saved.', '全局指令已保存。', '全域指令已儲存。'],
  'assistant-global-conflict': [
    'Source changed. Your draft is preserved. Reload the source before saving again.',
    '源文件已更改，草稿已保留。请重新加载源文件后再保存。',
    '來源檔案已變更，草稿已保留。請重新載入來源檔案後再儲存。',
  ],
  'assistant-global-limit': [
    'Global instructions must be at most 64 KiB. Your draft has not been saved.',
    '全局指令最多 64 KiB。草稿尚未保存。',
    '全域指令最多 64 KiB。草稿尚未儲存。',
  ],
  'assistant-global-unavailable': [
    'Source unavailable or larger than 64 KiB. Reload to try again.',
    '源文件不可用或超过 64 KiB。可重新加载以重试。',
    '來源檔案無法使用或超過 64 KiB。可重新載入以重試。',
  ],
  'assistant-unavailable': tui_assistant_unavailable,
  'assistant-unknown': tui_assistant_unknown,
  'settings-assistant': tui_settings_assistant,
} satisfies Catalog;
export function tuiText(
  locale: string,
  key: string,
  values: Record<string, string | number> = {},
): string {
  return translate(locale, terminalMessages, key, values);
}
