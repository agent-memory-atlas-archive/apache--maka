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
  msg_approvalAllow,
  msg_approvalAllowed,
  msg_approvalDenied,
  msg_approvalDeny,
  msg_check,
  msg_closeModels,
  msg_computerApprovalAllow,
  msg_computerApprovalDeny,
  msg_computerApprovalDescription,
  msg_computerApprovalTitle,
  msg_questionClosed,
} from '../../shared/locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  approvalAllow: msg_approvalAllow,
  approvalAllowed: msg_approvalAllowed,
  approvalDenied: msg_approvalDenied,
  approvalDeny: msg_approvalDeny,
  check: msg_check,
  closeModels: msg_closeModels,
  computerApprovalAllow: msg_computerApprovalAllow,
  computerApprovalDeny: msg_computerApprovalDeny,
  computerApprovalDescription: msg_computerApprovalDescription,
  computerApprovalTitle: msg_computerApprovalTitle,
  questionClosed: msg_questionClosed,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
