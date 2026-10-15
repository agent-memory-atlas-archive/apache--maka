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
  msg_clearDefault,
  msg_confirmClearDefault,
  msg_confirmDefault,
  msg_confirmSetDefault,
  msg_defaultCleared,
  msg_defaultConflict,
  msg_defaultHelp,
  msg_defaultHistorySaved,
  msg_defaultModel,
  msg_defaultRejected,
  msg_defaultSaved,
  msg_defaultUnavailable,
  msg_defaultUnknown,
  msg_earlierDefaultHelp,
  msg_earlierDefaultOperations,
  msg_historyRejected,
  msg_historyUnavailable,
  msg_historyUnknown,
  msg_loadingModels,
  msg_makeDefault,
  msg_modelApplyPatchHelp,
  msg_modelCodeModeHelp,
  msg_modelCompactionThreshold,
  msg_modelContextWindow,
  msg_modelCurrentLimit,
  msg_modelInputLimit,
  msg_modelLimitsHelp,
  msg_modelLimitsInvalid,
  msg_modelMaxOutput,
  msg_modelOutputCapacity,
  msg_modelToolsAuto,
  msg_modelToolsOff,
  msg_modelToolsOn,
  msg_modelToolsRejected,
  msg_modelToolsSave,
  msg_modelToolsSaved,
  msg_modelToolsScope,
  msg_noDefault,
  msg_refreshDefaultOperations,
  msg_settings,
  msg_unavailable,
} from '../../shared/locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  back: msg_back,
  cancel: msg_cancel,
  check: msg_check,
  clearDefault: msg_clearDefault,
  confirmClearDefault: msg_confirmClearDefault,
  confirmDefault: msg_confirmDefault,
  confirmSetDefault: msg_confirmSetDefault,
  defaultCleared: msg_defaultCleared,
  defaultConflict: msg_defaultConflict,
  defaultHelp: msg_defaultHelp,
  defaultHistorySaved: msg_defaultHistorySaved,
  defaultModel: msg_defaultModel,
  defaultRejected: msg_defaultRejected,
  defaultSaved: msg_defaultSaved,
  defaultUnavailable: msg_defaultUnavailable,
  defaultUnknown: msg_defaultUnknown,
  earlierDefaultHelp: msg_earlierDefaultHelp,
  earlierDefaultOperations: msg_earlierDefaultOperations,
  historyRejected: msg_historyRejected,
  historyUnavailable: msg_historyUnavailable,
  historyUnknown: msg_historyUnknown,
  loadingModels: msg_loadingModels,
  makeDefault: msg_makeDefault,
  modelApplyPatchHelp: msg_modelApplyPatchHelp,
  modelCodeModeHelp: msg_modelCodeModeHelp,
  modelCompactionThreshold: msg_modelCompactionThreshold,
  modelContextWindow: msg_modelContextWindow,
  modelCurrentLimit: msg_modelCurrentLimit,
  modelInputLimit: msg_modelInputLimit,
  modelLimitsHelp: msg_modelLimitsHelp,
  modelLimitsInvalid: msg_modelLimitsInvalid,
  modelMaxOutput: msg_modelMaxOutput,
  modelOutputCapacity: msg_modelOutputCapacity,
  modelToolsAuto: msg_modelToolsAuto,
  modelToolsOff: msg_modelToolsOff,
  modelToolsOn: msg_modelToolsOn,
  modelToolsRejected: msg_modelToolsRejected,
  modelToolsSave: msg_modelToolsSave,
  modelToolsSaved: msg_modelToolsSaved,
  modelToolsScope: msg_modelToolsScope,
  noDefault: msg_noDefault,
  refreshDefaultOperations: msg_refreshDefaultOperations,
  settings: msg_settings,
  unavailable: msg_unavailable,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
