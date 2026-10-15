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
  msg_confirmDefault,
  msg_connecting,
  msg_copy,
  msg_directoryRefresh,
  msg_executionReceipt,
  msg_historyUnavailable,
  msg_importsAdd,
  msg_importsArchived,
  msg_importsAssistant,
  msg_importsCatalogEmpty,
  msg_importsChanged,
  msg_importsChooseDestination,
  msg_importsChooseSource,
  msg_importsConfirm,
  msg_importsConversationName,
  msg_importsDatabase,
  msg_importsDestination,
  msg_importsDone,
  msg_importsEdit,
  msg_importsEmpty,
  msg_importsHelp,
  msg_importsHistory,
  msg_importsHistoryEmpty,
  msg_importsIncomplete,
  msg_importsInvalid,
  msg_importsLimit,
  msg_importsName,
  msg_importsNote,
  msg_importsOpen,
  msg_importsPreview,
  msg_importsRecords,
  msg_importsRemove,
  msg_importsRemoveHelp,
  msg_importsRemoved,
  msg_importsReview,
  msg_importsSaved,
  msg_importsSearch,
  msg_importsSourceHelp,
  msg_importsTitle,
  msg_importsToolCall,
  msg_importsToolResult,
  msg_importsTruncated,
  msg_importsUnavailable,
  msg_importsUnknown,
  msg_importsUser,
  msg_projectsActions,
  msg_projectsChecking,
  msg_projectsUseDirectory,
  msg_settingsSave,
  msg_sourceRetry,
} from '../../shared/locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  back: msg_back,
  cancel: msg_cancel,
  confirmDefault: msg_confirmDefault,
  connecting: msg_connecting,
  copy: msg_copy,
  directoryRefresh: msg_directoryRefresh,
  executionReceipt: msg_executionReceipt,
  historyUnavailable: msg_historyUnavailable,
  importsAdd: msg_importsAdd,
  importsArchived: msg_importsArchived,
  importsAssistant: msg_importsAssistant,
  importsCatalogEmpty: msg_importsCatalogEmpty,
  importsChanged: msg_importsChanged,
  importsChooseDestination: msg_importsChooseDestination,
  importsChooseSource: msg_importsChooseSource,
  importsConfirm: msg_importsConfirm,
  importsConversationName: msg_importsConversationName,
  importsDatabase: msg_importsDatabase,
  importsDestination: msg_importsDestination,
  importsDone: msg_importsDone,
  importsEdit: msg_importsEdit,
  importsEmpty: msg_importsEmpty,
  importsHelp: msg_importsHelp,
  importsHistory: msg_importsHistory,
  importsHistoryEmpty: msg_importsHistoryEmpty,
  importsIncomplete: msg_importsIncomplete,
  importsInvalid: msg_importsInvalid,
  importsLimit: msg_importsLimit,
  importsName: msg_importsName,
  importsNote: msg_importsNote,
  importsOpen: msg_importsOpen,
  importsPreview: msg_importsPreview,
  importsRecords: msg_importsRecords,
  importsRemove: msg_importsRemove,
  importsRemoveHelp: msg_importsRemoveHelp,
  importsRemoved: msg_importsRemoved,
  importsReview: msg_importsReview,
  importsSaved: msg_importsSaved,
  importsSearch: msg_importsSearch,
  importsSourceHelp: msg_importsSourceHelp,
  importsTitle: msg_importsTitle,
  importsToolCall: msg_importsToolCall,
  importsToolResult: msg_importsToolResult,
  importsTruncated: msg_importsTruncated,
  importsUnavailable: msg_importsUnavailable,
  importsUnknown: msg_importsUnknown,
  importsUser: msg_importsUser,
  projectsActions: msg_projectsActions,
  projectsChecking: msg_projectsChecking,
  projectsUseDirectory: msg_projectsUseDirectory,
  settingsSave: msg_settingsSave,
  sourceRetry: msg_sourceRetry,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
