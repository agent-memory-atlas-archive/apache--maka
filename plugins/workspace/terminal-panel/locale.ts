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
  msg_cancel,
  msg_check,
  msg_completed,
  msg_copy,
  msg_draft,
  msg_empty,
  msg_failed,
  msg_historyUnavailable,
  msg_new,
  msg_refreshDefaultOperations,
  msg_refreshSessions,
  msg_stop,
  msg_terminalCleanupUnknown,
  msg_terminalCommand,
  msg_terminalCommandLabel,
  msg_terminalConnect,
  msg_terminalConnecting,
  msg_terminalControlled,
  msg_terminalEmpty,
  msg_terminalFailed,
  msg_terminalForget,
  msg_terminalForgetConfirm,
  msg_terminalHelp,
  msg_terminalHistory,
  msg_terminalInputFailed,
  msg_terminalInputFull,
  msg_terminalLoading,
  msg_terminalNew,
  msg_terminalOriginalSession,
  msg_terminalReconnect,
  msg_terminalRecorded,
  msg_terminalRelease,
  msg_terminalRun,
  msg_terminalSelect,
  msg_terminalSourceOwned,
  msg_terminalStopConfirm,
  msg_terminalTitle,
  msg_terminalTruncated,
  msg_terminalUnavailable,
  msg_terminalUnknown,
} from '../../shared/locale-messages.js';
import { translate } from '../../shared/tui-locale.js';
import {
  tui_task_continue,
  tui_task_current,
  tui_task_empty,
  tui_task_empty_help,
  tui_task_ended,
  tui_task_evidence_hint,
  tui_task_failed,
  tui_task_followup,
  tui_task_idle,
  tui_task_outputs,
  tui_task_outputs_empty,
  tui_task_queue,
  tui_task_queue_help,
  tui_task_ready,
  tui_task_running,
  tui_task_steering,
  tui_task_stopped,
  tui_task_tool_error,
  tui_task_waiting,
  tui_terminal_required,
  tui_terminal_small,
} from '../../shared/tui-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  cancel: msg_cancel,
  check: msg_check,
  completed: msg_completed,
  copy: msg_copy,
  draft: msg_draft,
  empty: msg_empty,
  failed: msg_failed,
  historyUnavailable: msg_historyUnavailable,
  new: msg_new,
  refreshDefaultOperations: msg_refreshDefaultOperations,
  refreshSessions: msg_refreshSessions,
  stop: msg_stop,
  terminalCleanupUnknown: msg_terminalCleanupUnknown,
  terminalCommand: msg_terminalCommand,
  terminalCommandLabel: msg_terminalCommandLabel,
  terminalConnect: msg_terminalConnect,
  terminalConnecting: msg_terminalConnecting,
  terminalControlled: msg_terminalControlled,
  terminalEmpty: msg_terminalEmpty,
  terminalFailed: msg_terminalFailed,
  terminalForget: msg_terminalForget,
  terminalForgetConfirm: msg_terminalForgetConfirm,
  terminalHelp: msg_terminalHelp,
  terminalHistory: msg_terminalHistory,
  terminalInputFailed: msg_terminalInputFailed,
  terminalInputFull: msg_terminalInputFull,
  terminalLoading: msg_terminalLoading,
  terminalNew: msg_terminalNew,
  terminalOriginalSession: msg_terminalOriginalSession,
  terminalReconnect: msg_terminalReconnect,
  terminalRecorded: msg_terminalRecorded,
  terminalRelease: msg_terminalRelease,
  terminalRun: msg_terminalRun,
  terminalSelect: msg_terminalSelect,
  terminalSourceOwned: msg_terminalSourceOwned,
  terminalStopConfirm: msg_terminalStopConfirm,
  terminalTitle: msg_terminalTitle,
  terminalTruncated: msg_terminalTruncated,
  terminalUnavailable: msg_terminalUnavailable,
  terminalUnknown: msg_terminalUnknown,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;

const terminalMessages = {
  'task-continue': tui_task_continue,
  'task-current': tui_task_current,
  'task-empty': tui_task_empty,
  'task-empty-help': tui_task_empty_help,
  'task-ended': tui_task_ended,
  'task-evidence-hint': tui_task_evidence_hint,
  'task-failed': tui_task_failed,
  'task-followup': tui_task_followup,
  'task-idle': tui_task_idle,
  'task-outputs': tui_task_outputs,
  'task-outputs-empty': tui_task_outputs_empty,
  'task-queue': tui_task_queue,
  'task-queue-help': tui_task_queue_help,
  'task-ready': tui_task_ready,
  'task-running': tui_task_running,
  'task-steering': tui_task_steering,
  'task-stopped': tui_task_stopped,
  'task-tool-error': tui_task_tool_error,
  'task-waiting': tui_task_waiting,
  'terminal-required': tui_terminal_required,
  'terminal-small': tui_terminal_small,
};
export function tuiText(
  locale: string,
  key: string,
  values: Record<string, string | number> = {},
): string {
  return translate(locale, terminalMessages, key, values);
}
