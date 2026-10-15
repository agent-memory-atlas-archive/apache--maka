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
  msg_check,
  msg_closeModels,
  msg_questionAnswered,
  msg_questionClosed,
  msg_questionSkip,
  msg_questionSubmit,
  msg_questionText,
} from '../../shared/locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  check: msg_check,
  closeModels: msg_closeModels,
  questionAnswered: msg_questionAnswered,
  questionClosed: msg_questionClosed,
  questionSkip: msg_questionSkip,
  questionSubmit: msg_questionSubmit,
  questionText: msg_questionText,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
