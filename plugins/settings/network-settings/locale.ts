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
  tui_network_auth,
  tui_network_auto_bypass,
  tui_network_conflict,
  tui_network_delete_help,
  tui_network_enabled,
  tui_network_help,
  tui_network_invalid,
  tui_network_password,
  tui_network_reload,
  tui_network_save,
  tui_network_saved,
  tui_network_secret_absent,
  tui_network_secret_present,
  tui_network_unavailable,
  tui_network_unknown,
  tui_settings_host,
  tui_network_host,
  tui_network_port,
  tui_network_username,
  tui_network_bypass,
  tui_network_keep,
  tui_network_replace,
  tui_network_delete,
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
  'network-delete': tui_network_delete,
  'network-replace': tui_network_replace,
  'network-keep': tui_network_keep,
  'network-bypass': tui_network_bypass,
  'network-username': tui_network_username,
  'network-port': tui_network_port,
  'network-host': tui_network_host,
  'network-auth': tui_network_auth,
  'network-auto-bypass': tui_network_auto_bypass,
  'network-conflict': tui_network_conflict,
  'network-delete-help': tui_network_delete_help,
  'network-enabled': tui_network_enabled,
  'network-help': tui_network_help,
  'network-invalid': tui_network_invalid,
  'network-password': tui_network_password,
  'network-reload': tui_network_reload,
  'network-save': tui_network_save,
  'network-saved': tui_network_saved,
  'network-secret-absent': tui_network_secret_absent,
  'network-secret-present': tui_network_secret_present,
  'network-unavailable': tui_network_unavailable,
  'network-unknown': tui_network_unknown,
  'settings-host': tui_settings_host,
};
export function tuiText(
  locale: string,
  key: string,
  values: Record<string, string | number> = {},
): string {
  return translate(locale, terminalMessages, key, values);
}
