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

import { createCopy } from '../locale.js';
import {
  msg_archiveSession,
  msg_archivedSessions,
  msg_assistant,
  msg_browseHint,
  msg_deleteSession,
  msg_detailsTitle,
  msg_findConversation,
  msg_findFailed,
  msg_findFieldReceipt,
  msg_findFieldThinking,
  msg_findHistory,
  msg_findLocal,
  msg_findPlaceholder,
  msg_latest,
  msg_loading,
  msg_markRead,
  msg_needsYou,
  msg_newer,
  msg_older,
  msg_partial,
  msg_readSource,
  msg_renameSession,
  msg_restoreSession,
  msg_sourceBack,
  msg_sourceBeginning,
  msg_sourceComplete,
  msg_sourceEnding,
  msg_sourceFailed,
  msg_sourceFirst,
  msg_sourceLast,
  msg_sourceNext,
  msg_sourcePrevious,
  msg_sourceRetry,
  msg_sourceTitle,
  msg_stop,
  msg_thinking,
  msg_toEnd,
  msg_toolActivity,
  msg_toolArguments,
  msg_toolResult,
  msg_unread,
  msg_viewImage,
  msg_waiting,
  msg_welcome,
  msg_welcomeCopy,
  msg_working,
} from '../locale-messages.js';
import { translate } from '../tui-locale.js';
import {
  tui_chat_prompt_jump,
  tui_insights_this_session,
  tui_session_name_too_large,
} from '../tui-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  archiveSession: msg_archiveSession,
  archivedSessions: msg_archivedSessions,
  assistant: msg_assistant,
  browseHint: msg_browseHint,
  deleteSession: msg_deleteSession,
  detailsTitle: msg_detailsTitle,
  findConversation: msg_findConversation,
  findFailed: msg_findFailed,
  findFieldReceipt: msg_findFieldReceipt,
  findFieldThinking: msg_findFieldThinking,
  findHistory: msg_findHistory,
  findLocal: msg_findLocal,
  findPlaceholder: msg_findPlaceholder,
  latest: msg_latest,
  loading: msg_loading,
  markRead: msg_markRead,
  needsYou: msg_needsYou,
  newer: msg_newer,
  older: msg_older,
  partial: msg_partial,
  readSource: msg_readSource,
  renameSession: msg_renameSession,
  restoreSession: msg_restoreSession,
  sourceBack: msg_sourceBack,
  sourceBeginning: msg_sourceBeginning,
  sourceComplete: msg_sourceComplete,
  sourceEnding: msg_sourceEnding,
  sourceFailed: msg_sourceFailed,
  sourceFirst: msg_sourceFirst,
  sourceLast: msg_sourceLast,
  sourceNext: msg_sourceNext,
  sourcePrevious: msg_sourcePrevious,
  sourceRetry: msg_sourceRetry,
  sourceTitle: msg_sourceTitle,
  stop: msg_stop,
  thinking: msg_thinking,
  toEnd: msg_toEnd,
  toolActivity: msg_toolActivity,
  toolArguments: msg_toolArguments,
  toolResult: msg_toolResult,
  unread: msg_unread,
  viewImage: msg_viewImage,
  waiting: msg_waiting,
  welcome: msg_welcome,
  welcomeCopy: msg_welcomeCopy,
  working: msg_working,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;

const terminalMessages = {
  'chat-prompt-jump': tui_chat_prompt_jump,
  'insights-this-session': tui_insights_this_session,
  'session-name-too-large': tui_session_name_too_large,
};
export function tuiText(
  locale: string,
  key: string,
  values: Record<string, string | number> = {},
): string {
  return translate(locale, terminalMessages, key, values);
}
