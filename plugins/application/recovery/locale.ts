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
  msg_check,
  msg_deploymentRecorded,
  msg_deploymentTitle,
  msg_historyUnavailable,
  msg_loading,
  msg_recoveryEmpty,
  msg_recoveryHelp,
  msg_recoveryOperations,
  msg_recoveryRefreshFailed,
  msg_recoveryTitle,
  msg_recoveryUnknown,
  msg_refreshDefaultOperations,
  msg_skillsResultConfirmed,
} from '../../shared/locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  back: msg_back,
  check: msg_check,
  deploymentRecorded: msg_deploymentRecorded,
  deploymentTitle: msg_deploymentTitle,
  historyUnavailable: msg_historyUnavailable,
  loading: msg_loading,
  recoveryEmpty: msg_recoveryEmpty,
  recoveryHelp: msg_recoveryHelp,
  recoveryOperations: msg_recoveryOperations,
  recoveryRefreshFailed: msg_recoveryRefreshFailed,
  recoveryTitle: msg_recoveryTitle,
  recoveryUnknown: msg_recoveryUnknown,
  refreshDefaultOperations: msg_refreshDefaultOperations,
  skillsResultConfirmed: msg_skillsResultConfirmed,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
