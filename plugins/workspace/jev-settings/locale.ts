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
  msg_completed,
  msg_executionReceipt,
  msg_failed,
  msg_refreshDefaultOperations,
  msg_settingOff,
  msg_settingOn,
} from '../../shared/locale-messages.js';
import { translate } from '../../shared/tui-locale.js';
import {
  tui_jev_conflict,
  tui_jev_credential_help,
  tui_jev_enabled,
  tui_jev_help,
  tui_jev_invalid,
  tui_jev_key_absent,
  tui_jev_key_present,
  tui_jev_reload,
  tui_jev_remove,
  tui_jev_remove_header,
  tui_jev_remove_key,
  tui_jev_remove_question,
  tui_jev_save,
  tui_jev_save_header,
  tui_jev_save_key,
  tui_jev_saved,
  tui_jev_saved_endpoint,
  tui_jev_test,
  tui_jev_test_failed,
  tui_jev_test_help,
  tui_jev_tested,
  tui_jev_unavailable,
  tui_jev_unknown,
  tui_jev_url,
  tui_jev_model,
  tui_jev_timeoutMs,
  tui_jev_headerName,
  tui_jev_secret,
} from '../../shared/tui-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  back: msg_back,
  cancel: msg_cancel,
  completed: msg_completed,
  executionReceipt: msg_executionReceipt,
  failed: msg_failed,
  refreshDefaultOperations: msg_refreshDefaultOperations,
  settingOff: msg_settingOff,
  settingOn: msg_settingOn,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;

const terminalMessages = {
  'jev-secret': tui_jev_secret,
  'jev-headerName': tui_jev_headerName,
  'jev-timeoutMs': tui_jev_timeoutMs,
  'jev-model': tui_jev_model,
  'jev-url': tui_jev_url,
  'jev-conflict': tui_jev_conflict,
  'jev-credential-help': tui_jev_credential_help,
  'jev-enabled': tui_jev_enabled,
  'jev-help': tui_jev_help,
  'jev-invalid': tui_jev_invalid,
  'jev-key-absent': tui_jev_key_absent,
  'jev-key-present': tui_jev_key_present,
  'jev-reload': tui_jev_reload,
  'jev-remove': tui_jev_remove,
  'jev-remove-header': tui_jev_remove_header,
  'jev-remove-key': tui_jev_remove_key,
  'jev-remove-question': tui_jev_remove_question,
  'jev-save': tui_jev_save,
  'jev-save-header': tui_jev_save_header,
  'jev-save-key': tui_jev_save_key,
  'jev-saved': tui_jev_saved,
  'jev-saved-endpoint': tui_jev_saved_endpoint,
  'jev-test': tui_jev_test,
  'jev-test-failed': tui_jev_test_failed,
  'jev-test-help': tui_jev_test_help,
  'jev-tested': tui_jev_tested,
  'jev-unavailable': tui_jev_unavailable,
  'jev-unknown': tui_jev_unknown,
};
export function tuiText(
  locale: string,
  key: string,
  values: Record<string, string | number> = {},
): string {
  return translate(locale, terminalMessages, key, values);
}
