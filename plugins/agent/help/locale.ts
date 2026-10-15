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
  msg_findConversation,
  msg_findHistory,
  msg_findNavigate,
  msg_helpActivate,
  msg_helpBack,
  msg_helpClipboard,
  msg_helpClose,
  msg_helpCommands,
  msg_helpCopySelection,
  msg_helpEditorSelection,
  msg_helpFocus,
  msg_helpLineSelection,
  msg_helpListNavigation,
  msg_helpMouseSelection,
  msg_helpNavigation,
  msg_helpNew,
  msg_helpNewline,
  msg_helpPageSelection,
  msg_helpSend,
  msg_helpShiftSelection,
  msg_helpTextSelection,
  msg_helpTitle,
  msg_helpTranscriptScroll,
  msg_reviewRequest,
} from '../../shared/locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  findConversation: msg_findConversation,
  findHistory: msg_findHistory,
  findNavigate: msg_findNavigate,
  helpActivate: msg_helpActivate,
  helpBack: msg_helpBack,
  helpClipboard: msg_helpClipboard,
  helpClose: msg_helpClose,
  helpCommands: msg_helpCommands,
  helpCopySelection: msg_helpCopySelection,
  helpEditorSelection: msg_helpEditorSelection,
  helpFocus: msg_helpFocus,
  helpLineSelection: msg_helpLineSelection,
  helpListNavigation: msg_helpListNavigation,
  helpMouseSelection: msg_helpMouseSelection,
  helpNavigation: msg_helpNavigation,
  helpNew: msg_helpNew,
  helpNewline: msg_helpNewline,
  helpPageSelection: msg_helpPageSelection,
  helpSend: msg_helpSend,
  helpShiftSelection: msg_helpShiftSelection,
  helpTextSelection: msg_helpTextSelection,
  helpTitle: msg_helpTitle,
  helpTranscriptScroll: msg_helpTranscriptScroll,
  reviewRequest: msg_reviewRequest,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
