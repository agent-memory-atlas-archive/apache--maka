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

import type { TextareaMenu } from '../../packages/plugin-sdk/src/index.js';
import type { Theme } from './contracts.js';
import { createCopy } from './locale.js';
import {
  msg_undo,
  msg_redo,
  msg_cut,
  msg_copy,
  msg_paste,
  msg_selectAll,
} from './locale-messages.js';

const copy = createCopy({
  title: ['Edit', '编辑', '編輯'],
  undo: msg_undo,
  redo: msg_redo,
  cut: msg_cut,
  copy: msg_copy,
  paste: msg_paste,
  selectAll: msg_selectAll,
});

export function editorMenu(theme: Theme, locale: string): TextareaMenu {
  return { appearance: theme.menu, labels: copy(locale) };
}
