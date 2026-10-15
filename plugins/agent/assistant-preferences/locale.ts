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
  tui_assistant_instructions_help,
  tui_assistant_invalid,
  tui_assistant_name,
  tui_assistant_privacy,
  tui_assistant_privacy_help,
  tui_assistant_reload,
  tui_assistant_save,
  tui_assistant_saved,
  tui_assistant_tone,
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
  'assistant-conflict': tui_assistant_conflict,
  'assistant-instructions': tui_assistant_instructions,
  'assistant-instructions-help': tui_assistant_instructions_help,
  'assistant-invalid': tui_assistant_invalid,
  'assistant-name': tui_assistant_name,
  'assistant-privacy': tui_assistant_privacy,
  'assistant-privacy-help': tui_assistant_privacy_help,
  'assistant-reload': tui_assistant_reload,
  'assistant-save': tui_assistant_save,
  'assistant-saved': tui_assistant_saved,
  'assistant-tone': tui_assistant_tone,
  'assistant-unavailable': tui_assistant_unavailable,
  'assistant-unknown': tui_assistant_unknown,
  'settings-assistant': tui_settings_assistant,
};
export function tuiText(
  locale: string,
  key: string,
  values: Record<string, string | number> = {},
): string {
  return translate(locale, terminalMessages, key, values);
}
