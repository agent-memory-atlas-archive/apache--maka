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
  msg_connecting,
  msg_directoryRefresh,
  msg_directoryTitle,
  msg_empty,
  msg_executionReceipt,
  msg_historyUnavailable,
  msg_loading,
  msg_new,
  msg_projectsActions,
  msg_projectsAdd,
  msg_projectsArchive,
  msg_projectsArchiveHelp,
  msg_projectsArchived,
  msg_projectsBusy,
  msg_projectsChanged,
  msg_projectsChecking,
  msg_projectsEmpty,
  msg_projectsFailed,
  msg_projectsInvalid,
  msg_projectsMergeHelp,
  msg_projectsName,
  msg_projectsRelocate,
  msg_projectsRelocateHelp,
  msg_projectsRename,
  msg_projectsRestore,
  msg_projectsSaved,
  msg_projectsTitle,
  msg_projectsUnavailable,
  msg_projectsUnknown,
  msg_projectsUseDirectory,
  msg_settingsSave,
  msg_skillsConfirm,
  msg_sourceRetry,
} from '../../shared/locale-messages.js';
import { translate } from '../../shared/tui-locale.js';
import {
  tui_project_rename,
  tui_projects_empty,
  tui_projects_failed,
  tui_projects_loading,
  tui_projects_page_empty,
} from '../../shared/tui-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  back: msg_back,
  cancel: msg_cancel,
  connecting: msg_connecting,
  directoryRefresh: msg_directoryRefresh,
  directoryTitle: msg_directoryTitle,
  empty: msg_empty,
  executionReceipt: msg_executionReceipt,
  historyUnavailable: msg_historyUnavailable,
  loading: msg_loading,
  new: msg_new,
  projectsActions: msg_projectsActions,
  projectsAdd: msg_projectsAdd,
  projectsArchive: msg_projectsArchive,
  projectsArchiveHelp: msg_projectsArchiveHelp,
  projectsArchived: msg_projectsArchived,
  projectsBusy: msg_projectsBusy,
  projectsChanged: msg_projectsChanged,
  projectsChecking: msg_projectsChecking,
  projectsEmpty: msg_projectsEmpty,
  projectsFailed: msg_projectsFailed,
  projectsInvalid: msg_projectsInvalid,
  projectsMergeHelp: msg_projectsMergeHelp,
  projectsName: msg_projectsName,
  projectsRelocate: msg_projectsRelocate,
  projectsRelocateHelp: msg_projectsRelocateHelp,
  projectsRename: msg_projectsRename,
  projectsRestore: msg_projectsRestore,
  projectsSaved: msg_projectsSaved,
  projectsTitle: msg_projectsTitle,
  projectsUnavailable: msg_projectsUnavailable,
  projectsUnknown: msg_projectsUnknown,
  projectsUseDirectory: msg_projectsUseDirectory,
  settingsSave: msg_settingsSave,
  skillsConfirm: msg_skillsConfirm,
  sourceRetry: msg_sourceRetry,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;

const terminalMessages = {
  'project-rename': tui_project_rename,
  'projects-empty': tui_projects_empty,
  'projects-failed': tui_projects_failed,
  'projects-loading': tui_projects_loading,
  'projects-page-empty': tui_projects_page_empty,
};
export function tuiText(
  locale: string,
  key: string,
  values: Record<string, string | number> = {},
): string {
  return translate(locale, terminalMessages, key, values);
}
