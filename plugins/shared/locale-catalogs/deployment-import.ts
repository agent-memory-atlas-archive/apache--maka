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
  msg_back,
  msg_cancel,
  msg_check,
  msg_confirmDefault,
  msg_copy,
  msg_cut,
  msg_deploymentActivateHelp,
  msg_deploymentChooseFile,
  msg_deploymentEdit,
  msg_deploymentFileBusy,
  msg_deploymentFileFailed,
  msg_deploymentFileLoading,
  msg_deploymentFilePath,
  msg_deploymentFilePlaceholder,
  msg_deploymentFileTooLarge,
  msg_deploymentHistory,
  msg_deploymentInstall,
  msg_deploymentInstallHelp,
  msg_deploymentInstallQuestion,
  msg_deploymentLocal,
  msg_deploymentRefresh,
  msg_deploymentUrl,
  msg_directoryParent,
  msg_edit,
  msg_failed,
  msg_fileBrowse,
  msg_fileListEmpty,
  msg_fileListTruncated,
  msg_paste,
  msg_redo,
  msg_selectAll,
  msg_undo,
} from '../locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  back: msg_back,
  cancel: msg_cancel,
  check: msg_check,
  confirmDefault: msg_confirmDefault,
  copy: msg_copy,
  cut: msg_cut,
  deploymentActivateHelp: msg_deploymentActivateHelp,
  deploymentChooseFile: msg_deploymentChooseFile,
  deploymentEdit: msg_deploymentEdit,
  deploymentFileBusy: msg_deploymentFileBusy,
  deploymentFileFailed: msg_deploymentFileFailed,
  deploymentFileLoading: msg_deploymentFileLoading,
  deploymentFilePath: msg_deploymentFilePath,
  deploymentFilePlaceholder: msg_deploymentFilePlaceholder,
  deploymentFileTooLarge: msg_deploymentFileTooLarge,
  deploymentHistory: msg_deploymentHistory,
  deploymentInstall: msg_deploymentInstall,
  deploymentInstallHelp: msg_deploymentInstallHelp,
  deploymentInstallQuestion: msg_deploymentInstallQuestion,
  deploymentLocal: msg_deploymentLocal,
  deploymentRefresh: msg_deploymentRefresh,
  deploymentUrl: msg_deploymentUrl,
  directoryParent: msg_directoryParent,
  edit: msg_edit,
  failed: msg_failed,
  fileBrowse: msg_fileBrowse,
  fileListEmpty: msg_fileListEmpty,
  fileListTruncated: msg_fileListTruncated,
  paste: msg_paste,
  redo: msg_redo,
  selectAll: msg_selectAll,
  undo: msg_undo,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
