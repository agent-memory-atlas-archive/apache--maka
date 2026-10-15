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
  tui_extensions_refresh,
  tui_sessions_access,
  tui_sessions_conflict,
  tui_sessions_defaults_help,
  tui_sessions_help,
  tui_sessions_rejected,
  tui_sessions_requests,
  tui_sessions_saved,
  tui_sessions_unavailable,
  tui_sessions_unknown,
  tui_settings_sessions,
  tui_sessions_loading,
  tui_sessions_failed,
  tui_sessions_changed,
  tui_sessions_empty,
  tui_sessions_next,
  tui_sessions_previous,
  tui_sessions_read_only,
  tui_sessions_workspace_write,
  tui_sessions_danger_full_access,
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
  'sessions-danger_full_access': tui_sessions_danger_full_access,
  'sessions-workspace_write': tui_sessions_workspace_write,
  'sessions-read_only': tui_sessions_read_only,
  'sessions-previous': tui_sessions_previous,
  'sessions-next': tui_sessions_next,
  'sessions-empty': tui_sessions_empty,
  'sessions-changed': tui_sessions_changed,
  'sessions-failed': tui_sessions_failed,
  'sessions-loading': tui_sessions_loading,
  'extensions-refresh': tui_extensions_refresh,
  'sessions-access': tui_sessions_access,
  'sessions-conflict': tui_sessions_conflict,
  'sessions-defaults-help': tui_sessions_defaults_help,
  'sessions-help': tui_sessions_help,
  'sessions-rejected': tui_sessions_rejected,
  'sessions-requests': tui_sessions_requests,
  'sessions-saved': tui_sessions_saved,
  'sessions-unavailable': tui_sessions_unavailable,
  'sessions-unknown': tui_sessions_unknown,
  'settings-sessions': tui_settings_sessions,
};
export function tuiText(
  locale: string,
  key: string,
  values: Record<string, string | number> = {},
): string {
  return translate(locale, terminalMessages, key, values);
}
