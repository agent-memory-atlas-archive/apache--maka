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
  msg_assistant,
  msg_collapseTool,
  msg_copy,
  msg_copyMessage,
  msg_copySource,
  msg_expandTool,
  msg_failed,
  msg_findFieldArguments,
  msg_findFieldBody,
  msg_findFieldError,
  msg_findFieldFailure,
  msg_findFieldReceipt,
  msg_findFieldResult,
  msg_findFieldThinking,
  msg_findFieldTool,
  msg_partial,
  msg_stop,
  msg_terminalCommand,
  msg_thinking,
  msg_toolActivity,
  msg_toolDispatched,
  msg_toolDispatchedMissing,
  msg_toolEditAction,
  msg_toolFailed,
  msg_toolMissing,
  msg_toolNoText,
  msg_toolOutput,
  msg_toolPatchAction,
  msg_toolPending,
  msg_toolProcessInput,
  msg_toolProcessRead,
  msg_toolReadAction,
  msg_toolRejected,
  msg_toolReportedError,
  msg_toolReturned,
  msg_toolSearchAction,
  msg_toolUnknown,
  msg_toolWriteAction,
  msg_unknown,
  msg_waiting,
  msg_you,
} from '../locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  assistant: msg_assistant,
  collapseTool: msg_collapseTool,
  copy: msg_copy,
  copyMessage: msg_copyMessage,
  copySource: msg_copySource,
  expandTool: msg_expandTool,
  failed: msg_failed,
  findFieldArguments: msg_findFieldArguments,
  findFieldBody: msg_findFieldBody,
  findFieldError: msg_findFieldError,
  findFieldFailure: msg_findFieldFailure,
  findFieldReceipt: msg_findFieldReceipt,
  findFieldResult: msg_findFieldResult,
  findFieldThinking: msg_findFieldThinking,
  findFieldTool: msg_findFieldTool,
  partial: msg_partial,
  stop: msg_stop,
  terminalCommand: msg_terminalCommand,
  thinking: msg_thinking,
  toolActivity: msg_toolActivity,
  toolDispatched: msg_toolDispatched,
  toolDispatchedMissing: msg_toolDispatchedMissing,
  toolEditAction: msg_toolEditAction,
  toolFailed: msg_toolFailed,
  toolMissing: msg_toolMissing,
  toolNoText: msg_toolNoText,
  toolOutput: msg_toolOutput,
  toolPatchAction: msg_toolPatchAction,
  toolPending: msg_toolPending,
  toolProcessInput: msg_toolProcessInput,
  toolProcessRead: msg_toolProcessRead,
  toolReadAction: msg_toolReadAction,
  toolRejected: msg_toolRejected,
  toolReportedError: msg_toolReportedError,
  toolReturned: msg_toolReturned,
  toolSearchAction: msg_toolSearchAction,
  toolUnknown: msg_toolUnknown,
  toolWriteAction: msg_toolWriteAction,
  unknown: msg_unknown,
  waiting: msg_waiting,
  you: msg_you,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
