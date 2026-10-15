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
  msg_cancel,
  msg_executionReceipt,
  msg_refreshDefaultOperations,
  msg_saveKey,
  msg_settingOff,
  msg_settingOn,
} from '../../shared/locale-messages.js';
import { translate } from '../../shared/tui-locale.js';
import {
  tui_settings_web,
  tui_web_check_key,
  tui_web_check_unavailable,
  tui_web_checked,
  tui_web_conflict,
  tui_web_enabled,
  tui_web_key,
  tui_web_key_configured,
  tui_web_key_invalid,
  tui_web_key_missing,
  tui_web_model_help,
  tui_web_reload,
  tui_web_remove_key,
  tui_web_remove_question,
  tui_web_save_key,
  tui_web_saved,
  tui_web_source,
  tui_web_tavily_help,
  tui_web_unavailable,
  tui_web_unknown,
  tui_web_check_valid,
  tui_web_check_invalid_credentials,
  tui_web_check_rate_limited,
  tui_web_check_network_error,
  tui_web_check_timeout,
  tui_web_source_model,
  tui_web_source_tavily,
} from '../../shared/tui-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  back: msg_back,
  cancel: msg_cancel,
  executionReceipt: msg_executionReceipt,
  refreshDefaultOperations: msg_refreshDefaultOperations,
  saveKey: msg_saveKey,
  settingOff: msg_settingOff,
  settingOn: msg_settingOn,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;

const terminalMessages = {
  'web-source-tavily': tui_web_source_tavily,
  'web-source-model': tui_web_source_model,
  'web-check-timeout': tui_web_check_timeout,
  'web-check-network_error': tui_web_check_network_error,
  'web-check-rate_limited': tui_web_check_rate_limited,
  'web-check-invalid_credentials': tui_web_check_invalid_credentials,
  'web-check-valid': tui_web_check_valid,
  'settings-web': tui_settings_web,
  'web-check-key': tui_web_check_key,
  'web-check-unavailable': tui_web_check_unavailable,
  'web-checked': tui_web_checked,
  'web-conflict': tui_web_conflict,
  'web-enabled': tui_web_enabled,
  'web-key': tui_web_key,
  'web-key-configured': tui_web_key_configured,
  'web-key-invalid': tui_web_key_invalid,
  'web-key-missing': tui_web_key_missing,
  'web-model-help': tui_web_model_help,
  'web-reload': tui_web_reload,
  'web-remove-key': tui_web_remove_key,
  'web-remove-question': tui_web_remove_question,
  'web-save-key': tui_web_save_key,
  'web-saved': tui_web_saved,
  'web-source': tui_web_source,
  'web-tavily-help': tui_web_tavily_help,
  'web-unavailable': tui_web_unavailable,
  'web-unknown': tui_web_unknown,
};
export function tuiText(
  locale: string,
  key: string,
  values: Record<string, string | number> = {},
): string {
  return translate(locale, terminalMessages, key, values);
}
