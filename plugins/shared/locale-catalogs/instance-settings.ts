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
  msg_deploymentUnavailable,
  msg_cancel,
  msg_check,
  msg_connecting,
  msg_instanceConfiguration,
  msg_settingApply,
  msg_settingNone,
  msg_settingOff,
  msg_settingOn,
  msg_settingRemove,
  msg_settingUnset,
  msg_settingUnsupported,
  msg_settingsBack,
  msg_settingsDiscard,
  msg_settingsDiscardQuestion,
  msg_settingsEmpty,
  msg_settingsInvalidNumber,
  msg_settingsLimit,
  msg_settingsPreparing,
  msg_settingsReadFailed,
  msg_settingsReload,
  msg_settingsSave,
} from '../locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  deploymentUnavailable: msg_deploymentUnavailable,
  back: msg_back,
  cancel: msg_cancel,
  check: msg_check,
  connecting: msg_connecting,
  instanceConfiguration: msg_instanceConfiguration,
  settingApply: msg_settingApply,
  settingNone: msg_settingNone,
  settingOff: msg_settingOff,
  settingOn: msg_settingOn,
  settingRemove: msg_settingRemove,
  settingUnset: msg_settingUnset,
  settingUnsupported: msg_settingUnsupported,
  settingsBack: msg_settingsBack,
  settingsDiscard: msg_settingsDiscard,
  settingsDiscardQuestion: msg_settingsDiscardQuestion,
  settingsEmpty: msg_settingsEmpty,
  settingsInvalidNumber: msg_settingsInvalidNumber,
  settingsLimit: msg_settingsLimit,
  settingsPreparing: msg_settingsPreparing,
  settingsReadFailed: msg_settingsReadFailed,
  settingsReload: msg_settingsReload,
  settingsSave: msg_settingsSave,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
