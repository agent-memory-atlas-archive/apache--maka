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
  msg_directoryRefresh,
  msg_loading,
  msg_recallAll,
  msg_recallChoose,
  msg_recallDateHint,
  msg_recallEmpty,
  msg_recallFind,
  msg_recallIntro,
  msg_recallInvalid,
  msg_recallPartial,
  msg_recallSince,
  msg_recallTitle,
  msg_recallUnavailable,
  msg_recallUntil,
} from '../../shared/locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  back: msg_back,
  directoryRefresh: msg_directoryRefresh,
  loading: msg_loading,
  recallAll: msg_recallAll,
  recallChoose: msg_recallChoose,
  recallDateHint: msg_recallDateHint,
  recallEmpty: msg_recallEmpty,
  recallFind: msg_recallFind,
  recallIntro: msg_recallIntro,
  recallInvalid: msg_recallInvalid,
  recallPartial: msg_recallPartial,
  recallSince: msg_recallSince,
  recallTitle: msg_recallTitle,
  recallUnavailable: msg_recallUnavailable,
  recallUntil: msg_recallUntil,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
