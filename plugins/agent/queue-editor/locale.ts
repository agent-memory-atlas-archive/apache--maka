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
  msg_copy,
  msg_cut,
  msg_detailsReload,
  msg_draft,
  msg_edit,
  msg_paste,
  msg_queueEdit,
  msg_queueEditHint,
  msg_queueSave,
  msg_redo,
  msg_selectAll,
  msg_undo,
} from '../../shared/locale-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  cancel: msg_cancel,
  check: msg_check,
  copy: msg_copy,
  cut: msg_cut,
  detailsReload: msg_detailsReload,
  draft: msg_draft,
  edit: msg_edit,
  paste: msg_paste,
  queueEdit: msg_queueEdit,
  queueEditHint: msg_queueEditHint,
  queueSave: msg_queueSave,
  redo: msg_redo,
  selectAll: msg_selectAll,
  undo: msg_undo,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;
