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
  msg_archivedConversation,
  msg_check,
  msg_detailsConflict,
  msg_detailsDraftKept,
  msg_detailsHistory,
  msg_detailsHistoryHelp,
  msg_detailsInvalid,
  msg_detailsLabels,
  msg_detailsLabelsHelp,
  msg_detailsName,
  msg_detailsRebaseHelp,
  msg_detailsRejected,
  msg_detailsReload,
  msg_detailsReloadHelp,
  msg_detailsRetry,
  msg_detailsReview,
  msg_detailsReviewFailed,
  msg_detailsSave,
  msg_detailsSaved,
  msg_detailsTargetChanged,
  msg_detailsTitle,
  msg_detailsUnknown,
  msg_detailsUseVersion,
  msg_failed,
  msg_historyUnavailable,
  msg_historyUnknown,
  msg_permissionAsk,
  msg_permissionFullAccess,
  msg_permissionNoAsk,
  msg_permissionPolicyHelp,
  msg_permissionReadOnly,
  msg_permissionRevoke,
  msg_permissionWorkspaceWrite,
  msg_permissionsTitle,
  msg_refreshDefaultOperations,
} from '../../shared/locale-messages.js';
import { translate } from '../../shared/tui-locale.js';
import { tui_review_permissions } from '../../shared/tui-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  archivedConversation: msg_archivedConversation,
  check: msg_check,
  detailsConflict: msg_detailsConflict,
  detailsDraftKept: msg_detailsDraftKept,
  detailsHistory: msg_detailsHistory,
  detailsHistoryHelp: msg_detailsHistoryHelp,
  detailsInvalid: msg_detailsInvalid,
  detailsLabels: msg_detailsLabels,
  detailsLabelsHelp: msg_detailsLabelsHelp,
  detailsName: msg_detailsName,
  detailsRebaseHelp: msg_detailsRebaseHelp,
  detailsRejected: msg_detailsRejected,
  detailsReload: msg_detailsReload,
  detailsReloadHelp: msg_detailsReloadHelp,
  detailsRetry: msg_detailsRetry,
  detailsReview: msg_detailsReview,
  detailsReviewFailed: msg_detailsReviewFailed,
  detailsSave: msg_detailsSave,
  detailsSaved: msg_detailsSaved,
  detailsTargetChanged: msg_detailsTargetChanged,
  detailsTitle: msg_detailsTitle,
  detailsUnknown: msg_detailsUnknown,
  detailsUseVersion: msg_detailsUseVersion,
  failed: msg_failed,
  historyUnavailable: msg_historyUnavailable,
  historyUnknown: msg_historyUnknown,
  permissionAsk: msg_permissionAsk,
  permissionFullAccess: msg_permissionFullAccess,
  permissionNoAsk: msg_permissionNoAsk,
  permissionPolicyHelp: msg_permissionPolicyHelp,
  permissionReadOnly: msg_permissionReadOnly,
  permissionRevoke: msg_permissionRevoke,
  permissionWorkspaceWrite: msg_permissionWorkspaceWrite,
  permissionsTitle: msg_permissionsTitle,
  refreshDefaultOperations: msg_refreshDefaultOperations,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;

const terminalMessages = {
  'review-permissions': tui_review_permissions,
};
export function tuiText(
  locale: string,
  key: string,
  values: Record<string, string | number> = {},
): string {
  return translate(locale, terminalMessages, key, values);
}
