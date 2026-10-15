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
  msg_closeModels,
  msg_directoryParent,
  msg_fileBrowse,
  msg_fileListEmpty,
  msg_fileListTruncated,
  msg_skillsCheckResult,
  msg_skillsChooseFile,
  msg_skillsCleanupUnknown,
  msg_skillsConfirm,
  msg_skillsFilePath,
  msg_skillsImport,
  msg_skillsImportConfirm,
  msg_skillsImportHelp,
  msg_skillsImportInvalid,
  msg_skillsSourceId,
  msg_skillsSourceIdHelp,
} from '../locale-messages.js';

const messages = {
  back: msg_back,
  cancel: msg_cancel,
  closeModels: msg_closeModels,
  directoryParent: msg_directoryParent,
  fileBrowse: msg_fileBrowse,
  fileListEmpty: msg_fileListEmpty,
  fileListTruncated: msg_fileListTruncated,
  skillsCheckResult: msg_skillsCheckResult,
  skillsChooseFile: msg_skillsChooseFile,
  skillsCleanupUnknown: msg_skillsCleanupUnknown,
  skillsConfirm: msg_skillsConfirm,
  skillsFilePath: msg_skillsFilePath,
  skillsImport: msg_skillsImport,
  skillsImportConfirm: msg_skillsImportConfirm,
  skillsImportHelp: msg_skillsImportHelp,
  skillsImportInvalid: msg_skillsImportInvalid,
  skillsSourceId: msg_skillsSourceId,
  skillsSourceIdHelp: msg_skillsSourceIdHelp,
};
export const copy = /* @__PURE__ */ createCopy(messages);
