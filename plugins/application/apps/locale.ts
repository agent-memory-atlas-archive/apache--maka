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
  msg_appsAdd,
  msg_appsAdded,
  msg_appsBindingClear,
  msg_appsBindingHelp,
  msg_appsBindings,
  msg_appsBindingsMissing,
  msg_appsChanged,
  msg_appsChoose,
  msg_appsClose,
  msg_appsConfigure,
  msg_appsConfirm,
  msg_appsConfirmHelp,
  msg_appsContinue,
  msg_appsEmpty,
  msg_appsExistingHelp,
  msg_appsInstall,
  msg_appsInvalid,
  msg_appsManage,
  msg_appsName,
  msg_appsNoCandidates,
  msg_appsNoSettings,
  msg_appsPackageAdded,
  msg_appsReload,
  msg_appsRemove,
  msg_appsRemoveHelp,
  msg_appsRemoved,
  msg_appsReview,
  msg_appsSetupHelp,
  msg_appsTitle,
  msg_appsUnavailable,
  msg_appsUnknown,
  msg_back,
  msg_cancel,
  msg_confirmDefault,
  msg_connecting,
  msg_deploymentFileFailed,
  msg_deploymentInstallHelp,
  msg_deploymentUrlInvalid,
  msg_directoryParent,
  msg_edit,
  msg_executionReceipt,
  msg_failed,
  msg_historyUnavailable,
  msg_instanceConfiguration,
  msg_settingApply,
  msg_settingNone,
  msg_settingOff,
  msg_settingOn,
  msg_settingUnset,
  msg_settings,
  msg_sourceRetry,
} from '../../shared/locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  appsAdd: msg_appsAdd,
  appsAdded: msg_appsAdded,
  appsBindingClear: msg_appsBindingClear,
  appsBindingHelp: msg_appsBindingHelp,
  appsBindings: msg_appsBindings,
  appsBindingsMissing: msg_appsBindingsMissing,
  appsChanged: msg_appsChanged,
  appsChoose: msg_appsChoose,
  appsClose: msg_appsClose,
  appsConfigure: msg_appsConfigure,
  appsConfirm: msg_appsConfirm,
  appsConfirmHelp: msg_appsConfirmHelp,
  appsContinue: msg_appsContinue,
  appsEmpty: msg_appsEmpty,
  appsExistingHelp: msg_appsExistingHelp,
  appsInstall: msg_appsInstall,
  appsInvalid: msg_appsInvalid,
  appsManage: msg_appsManage,
  appsName: msg_appsName,
  appsNoCandidates: msg_appsNoCandidates,
  appsNoSettings: msg_appsNoSettings,
  appsPackageAdded: msg_appsPackageAdded,
  appsReload: msg_appsReload,
  appsRemove: msg_appsRemove,
  appsRemoveHelp: msg_appsRemoveHelp,
  appsRemoved: msg_appsRemoved,
  appsReview: msg_appsReview,
  appsSetupHelp: msg_appsSetupHelp,
  appsTitle: msg_appsTitle,
  appsUnavailable: msg_appsUnavailable,
  appsUnknown: msg_appsUnknown,
  back: msg_back,
  cancel: msg_cancel,
  confirmDefault: msg_confirmDefault,
  connecting: msg_connecting,
  deploymentFileFailed: msg_deploymentFileFailed,
  deploymentInstallHelp: msg_deploymentInstallHelp,
  deploymentUrlInvalid: msg_deploymentUrlInvalid,
  directoryParent: msg_directoryParent,
  edit: msg_edit,
  executionReceipt: msg_executionReceipt,
  failed: msg_failed,
  historyUnavailable: msg_historyUnavailable,
  instanceConfiguration: msg_instanceConfiguration,
  settingApply: msg_settingApply,
  settingNone: msg_settingNone,
  settingOff: msg_settingOff,
  settingOn: msg_settingOn,
  settingUnset: msg_settingUnset,
  settings: msg_settings,
  sourceRetry: msg_sourceRetry,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
