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
  msg_directoryTitle,
  msg_failed,
  msg_loading,
  msg_recoveryRefreshFailed,
  msg_unavailable,
  msg_workhubAnswer,
  msg_workhubAnswerFirst,
  msg_workhubCancelled,
  msg_workhubCapture,
  msg_workhubChanged,
  msg_workhubChat,
  msg_workhubCleanup,
  msg_workhubCompleted,
  msg_workhubContinue,
  msg_workhubCreating,
  msg_workhubDelivering,
  msg_workhubDependencies,
  msg_workhubEmpty,
  msg_workhubFailed,
  msg_workhubFirst,
  msg_workhubNoAnswer,
  msg_workhubOpen,
  msg_workhubOriginal,
  msg_workhubPaused,
  msg_workhubPending,
  msg_workhubQueued,
  msg_workhubRejected,
  msg_workhubRunning,
  msg_workhubSessions,
  msg_workhubSetup,
  msg_workhubSetupPending,
  msg_workhubShared,
  msg_workhubStart,
  msg_workhubTasks,
  msg_workhubUnavailable,
  msg_workhubUnknown,
  msg_workhubUntitled,
  msg_workhubUploading,
  msg_workhubWaiting,
} from '../../shared/locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  back: msg_back,
  check: msg_check,
  directoryTitle: msg_directoryTitle,
  failed: msg_failed,
  loading: msg_loading,
  recoveryRefreshFailed: msg_recoveryRefreshFailed,
  unavailable: msg_unavailable,
  workhubAnswer: msg_workhubAnswer,
  workhubAnswerFirst: msg_workhubAnswerFirst,
  workhubCancelled: msg_workhubCancelled,
  workhubCapture: msg_workhubCapture,
  workhubChanged: msg_workhubChanged,
  workhubChat: msg_workhubChat,
  workhubCleanup: msg_workhubCleanup,
  workhubCompleted: msg_workhubCompleted,
  workhubContinue: msg_workhubContinue,
  workhubCreating: msg_workhubCreating,
  workhubDelivering: msg_workhubDelivering,
  workhubDependencies: msg_workhubDependencies,
  workhubEmpty: msg_workhubEmpty,
  workhubFailed: msg_workhubFailed,
  workhubFirst: msg_workhubFirst,
  workhubNoAnswer: msg_workhubNoAnswer,
  workhubOpen: msg_workhubOpen,
  workhubOriginal: msg_workhubOriginal,
  workhubPaused: msg_workhubPaused,
  workhubPending: msg_workhubPending,
  workhubQueued: msg_workhubQueued,
  workhubRejected: msg_workhubRejected,
  workhubRunning: msg_workhubRunning,
  workhubSessions: msg_workhubSessions,
  workhubSetup: msg_workhubSetup,
  workhubSetupPending: msg_workhubSetupPending,
  workhubShared: msg_workhubShared,
  workhubStart: msg_workhubStart,
  workhubTasks: msg_workhubTasks,
  workhubUnavailable: msg_workhubUnavailable,
  workhubUnknown: msg_workhubUnknown,
  workhubUntitled: msg_workhubUntitled,
  workhubUploading: msg_workhubUploading,
  workhubWaiting: msg_workhubWaiting,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
