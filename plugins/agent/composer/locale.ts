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

import { createCopy } from '../../shared/locale.js';
import {
  msg_attachmentTitle,
  msg_copy,
  msg_cut,
  msg_draft,
  msg_draftCapacity,
  msg_draftKept,
  msg_draftPersistenceFailed,
  msg_draftRestoring,
  msg_draftUncertain,
  msg_edit,
  msg_empty,
  msg_enterHint,
  msg_failed,
  msg_loading,
  msg_paste,
  msg_placeholder,
  msg_redo,
  msg_selectAll,
  msg_send,
  msg_skillInputUnavailable,
  msg_sourceRetry,
  msg_stop,
  msg_undo,
  msg_unknown,
} from '../../shared/locale-messages.js';
import { translate } from '../../shared/tui-locale.js';
import { tui_composer_placeholder } from '../../shared/tui-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  attachmentTitle: msg_attachmentTitle,
  copy: msg_copy,
  cut: msg_cut,
  draft: msg_draft,
  draftCapacity: msg_draftCapacity,
  draftKept: msg_draftKept,
  draftPersistenceFailed: msg_draftPersistenceFailed,
  draftRestoring: msg_draftRestoring,
  draftUncertain: msg_draftUncertain,
  edit: msg_edit,
  empty: msg_empty,
  enterHint: msg_enterHint,
  failed: msg_failed,
  loading: msg_loading,
  paste: msg_paste,
  placeholder: msg_placeholder,
  redo: msg_redo,
  selectAll: msg_selectAll,
  send: msg_send,
  skillInputUnavailable: msg_skillInputUnavailable,
  sourceRetry: msg_sourceRetry,
  stop: msg_stop,
  undo: msg_undo,
  unknown: msg_unknown,
};
const baseCopy = /* @__PURE__ */ createCopy(messages);
export const copy = (locale: string) => ({
  ...baseCopy(locale),
  unknownCommand: locale.startsWith('zh') ? '没有可用的匹配命令' : 'No matching commands available',
  commandHint: locale.startsWith('zh')
    ? '↑↓ 选择 · Enter 选择 · Tab 补全 · Esc 关闭'
    : '↑↓ select · Enter choose · Tab complete · Esc dismiss',
});
export type Copy = ReturnType<typeof copy>;

const terminalMessages = {
  'composer-placeholder': tui_composer_placeholder,
};
export function tuiText(
  locale: string,
  key: string,
  values: Record<string, string | number> = {},
): string {
  return translate(locale, terminalMessages, key, values);
}
