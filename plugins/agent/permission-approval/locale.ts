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
  msg_approvalDeny,
  msg_check,
  msg_closeModels,
  msg_permissionDenied,
  msg_permissionDeny,
  msg_permissionExact,
  msg_permissionGranted,
  msg_permissionNetwork,
  msg_permissionOnce,
  msg_permissionRead,
  msg_permissionRequested,
  msg_permissionSession,
  msg_permissionSubtree,
  msg_permissionTurn,
  msg_permissionWrite,
  msg_questionClosed,
} from '../../shared/locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  permissionAllow: ['Allow', '允许', '允許'] as const,
  approvalDeny: msg_approvalDeny,
  check: msg_check,
  closeModels: msg_closeModels,
  permissionDenied: msg_permissionDenied,
  permissionDeny: msg_permissionDeny,
  permissionExact: msg_permissionExact,
  permissionGranted: msg_permissionGranted,
  permissionNetwork: msg_permissionNetwork,
  permissionOnce: msg_permissionOnce,
  permissionRead: msg_permissionRead,
  permissionRequested: msg_permissionRequested,
  permissionSession: msg_permissionSession,
  permissionSubtree: msg_permissionSubtree,
  permissionTurn: msg_permissionTurn,
  permissionWrite: msg_permissionWrite,
  questionClosed: msg_questionClosed,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
