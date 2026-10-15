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
  msg_check,
  msg_configureInstance,
  msg_confirmDefault,
  msg_connecting,
  msg_connectionDisabled,
  msg_connectionEnabled,
  msg_deploymentChangeHelp,
  msg_deploymentConfirmDisable,
  msg_deploymentConfirmEnable,
  msg_deploymentConflict,
  msg_deploymentDisable,
  msg_deploymentDocumentRejected,
  msg_deploymentEdit,
  msg_deploymentEmpty,
  msg_deploymentEnable,
  msg_deploymentEnvironment,
  msg_deploymentFileFailed,
  msg_deploymentHelp,
  msg_deploymentHistory,
  msg_deploymentHistoryHelp,
  msg_deploymentInstall,
  msg_deploymentInstallRejected,
  msg_deploymentInstalled,
  msg_deploymentPackageUnavailable,
  msg_deploymentParentDisabled,
  msg_deploymentRecorded,
  msg_deploymentRefresh,
  msg_deploymentRejected,
  msg_deploymentSaved,
  msg_deploymentTitle,
  msg_deploymentUnavailable,
  msg_deploymentUnknown,
  msg_deploymentUrlInvalid,
  msg_failed,
  msg_historyUnavailable,
  msg_instanceConfiguration,
  msg_loading,
  msg_pluginSearch,
  msg_refreshDefaultOperations,
  msg_reloadView,
  msg_restoringView,
  msg_searchTooLong,
  msg_settings,
  msg_settingsChanged,
  msg_settingsReadFailed,
  msg_settingsReload,
  msg_viewSaveFailed,
} from '../../shared/locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  back: msg_back,
  cancel: msg_cancel,
  check: msg_check,
  configureInstance: msg_configureInstance,
  confirmDefault: msg_confirmDefault,
  connecting: msg_connecting,
  connectionDisabled: msg_connectionDisabled,
  connectionEnabled: msg_connectionEnabled,
  deploymentChangeHelp: msg_deploymentChangeHelp,
  deploymentConfirmDisable: msg_deploymentConfirmDisable,
  deploymentConfirmEnable: msg_deploymentConfirmEnable,
  deploymentConflict: msg_deploymentConflict,
  deploymentDisable: msg_deploymentDisable,
  deploymentDocumentRejected: msg_deploymentDocumentRejected,
  deploymentEdit: msg_deploymentEdit,
  deploymentEmpty: msg_deploymentEmpty,
  deploymentEnable: msg_deploymentEnable,
  deploymentEnvironment: msg_deploymentEnvironment,
  deploymentFileFailed: msg_deploymentFileFailed,
  deploymentHelp: msg_deploymentHelp,
  deploymentHistory: msg_deploymentHistory,
  deploymentHistoryHelp: msg_deploymentHistoryHelp,
  deploymentInstall: msg_deploymentInstall,
  deploymentInstallRejected: msg_deploymentInstallRejected,
  deploymentInstalled: msg_deploymentInstalled,
  deploymentPackageUnavailable: msg_deploymentPackageUnavailable,
  deploymentParentDisabled: msg_deploymentParentDisabled,
  deploymentRecorded: msg_deploymentRecorded,
  deploymentRefresh: msg_deploymentRefresh,
  deploymentRejected: msg_deploymentRejected,
  deploymentSaved: msg_deploymentSaved,
  deploymentTitle: msg_deploymentTitle,
  deploymentUnavailable: msg_deploymentUnavailable,
  deploymentUnknown: msg_deploymentUnknown,
  deploymentUrlInvalid: msg_deploymentUrlInvalid,
  failed: msg_failed,
  historyUnavailable: msg_historyUnavailable,
  instanceConfiguration: msg_instanceConfiguration,
  loading: msg_loading,
  pluginSearch: msg_pluginSearch,
  refreshDefaultOperations: msg_refreshDefaultOperations,
  reloadView: msg_reloadView,
  restoringView: msg_restoringView,
  searchTooLong: msg_searchTooLong,
  settings: msg_settings,
  settingsChanged: msg_settingsChanged,
  settingsReadFailed: msg_settingsReadFailed,
  settingsReload: msg_settingsReload,
  viewSaveFailed: msg_viewSaveFailed,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
