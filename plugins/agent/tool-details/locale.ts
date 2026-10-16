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
  msg_completed,
  msg_copy,
  msg_edit,
  msg_failed,
  msg_loading,
  msg_partial,
  msg_sourceFailed,
  msg_sourceFirst,
  msg_sourceLast,
  msg_sourceRetry,
  msg_stop,
  msg_terminalCommandLabel,
  msg_terminalTruncated,
  msg_toolArguments,
  msg_toolResult,
  msg_toolBusinessFailure,
  msg_toolByteRange,
  msg_toolCopyPath,
  msg_toolExitCode,
  msg_toolFilePath,
  msg_toolMatchingFiles,
  msg_toolMatchingLines,
  msg_toolOriginalUnavailable,
  msg_toolOutputRedacted,
  msg_toolPatch,
  msg_toolPatchOperation,
  msg_toolProcessCancelled,
  msg_toolProcessControl,
  msg_toolProcessCwd,
  msg_toolProcessFailed,
  msg_toolProcessFailure,
  msg_toolProcessOrphaned,
  msg_toolProcessRecorded,
  msg_toolProcessRef,
  msg_toolProcessRunning,
  msg_toolProcessStarting,
  msg_toolProcessTimedOut,
  msg_toolReadContent,
  msg_toolReadLines,
  msg_toolRecordedBefore,
  msg_toolRequestedAfter,
  msg_toolRequestedBefore,
  msg_toolScreen,
  msg_toolScrollback,
  msg_toolSearchIncomplete,
  msg_toolSearchPattern,
  msg_toolStderr,
  msg_toolStdout,
} from '../../shared/locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  completed: msg_completed,
  copy: msg_copy,
  edit: msg_edit,
  failed: msg_failed,
  loading: msg_loading,
  partial: msg_partial,
  sourceFailed: msg_sourceFailed,
  sourceFirst: msg_sourceFirst,
  sourceLast: msg_sourceLast,
  sourceRetry: msg_sourceRetry,
  stop: msg_stop,
  terminalCommandLabel: msg_terminalCommandLabel,
  terminalTruncated: msg_terminalTruncated,
  toolArguments: msg_toolArguments,
  toolResult: msg_toolResult,
  toolBusinessFailure: msg_toolBusinessFailure,
  toolByteRange: msg_toolByteRange,
  toolCopyPath: msg_toolCopyPath,
  toolExitCode: msg_toolExitCode,
  toolFilePath: msg_toolFilePath,
  toolMatchingFiles: msg_toolMatchingFiles,
  toolMatchingLines: msg_toolMatchingLines,
  toolOriginalUnavailable: msg_toolOriginalUnavailable,
  toolOutputRedacted: msg_toolOutputRedacted,
  toolPatch: msg_toolPatch,
  toolPatchOperation: msg_toolPatchOperation,
  toolProcessCancelled: msg_toolProcessCancelled,
  toolProcessControl: msg_toolProcessControl,
  toolProcessCwd: msg_toolProcessCwd,
  toolProcessFailed: msg_toolProcessFailed,
  toolProcessFailure: msg_toolProcessFailure,
  toolProcessOrphaned: msg_toolProcessOrphaned,
  toolProcessRecorded: msg_toolProcessRecorded,
  toolProcessRef: msg_toolProcessRef,
  toolProcessRunning: msg_toolProcessRunning,
  toolProcessStarting: msg_toolProcessStarting,
  toolProcessTimedOut: msg_toolProcessTimedOut,
  toolReadContent: msg_toolReadContent,
  toolReadLines: msg_toolReadLines,
  toolRecordedBefore: msg_toolRecordedBefore,
  toolRequestedAfter: msg_toolRequestedAfter,
  toolRequestedBefore: msg_toolRequestedBefore,
  toolScreen: msg_toolScreen,
  toolScrollback: msg_toolScrollback,
  toolSearchIncomplete: msg_toolSearchIncomplete,
  toolSearchPattern: msg_toolSearchPattern,
  toolStderr: msg_toolStderr,
  toolStdout: msg_toolStdout,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
