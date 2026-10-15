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
  msg_cancel,
  msg_check,
  msg_closeModels,
  msg_formCancel,
  msg_formDecline,
  msg_formInclude,
  msg_formInvalid,
  msg_formNo,
  msg_formOmit,
  msg_formReadFailed,
  msg_formRequired,
  msg_formSubmit,
  msg_formYes,
  msg_questionAnswered,
  msg_questionClosed,
} from '../../shared/locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  cancel: msg_cancel,
  check: msg_check,
  closeModels: msg_closeModels,
  formCancel: msg_formCancel,
  formDecline: msg_formDecline,
  formInclude: msg_formInclude,
  formInvalid: msg_formInvalid,
  formNo: msg_formNo,
  formOmit: msg_formOmit,
  formReadFailed: msg_formReadFailed,
  formRequired: msg_formRequired,
  formSubmit: msg_formSubmit,
  formYes: msg_formYes,
  questionAnswered: msg_questionAnswered,
  questionClosed: msg_questionClosed,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
