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
  msg_directoryChoose,
  msg_directoryEmpty,
  msg_directoryFailed,
  msg_directoryFilter,
  msg_directoryHelp,
  msg_directoryLoading,
  msg_directoryParent,
  msg_directoryRefresh,
  msg_directoryTitle,
  msg_importsChooseSource,
  msg_searchTooLong,
  msg_unavailable,
} from '../../shared/locale-messages.js';
import { translate } from '../../shared/tui-locale.js';
import {
  tui_directory_browse,
  tui_directory_empty,
  tui_directory_failed,
  tui_directory_no_roots,
  tui_directory_open,
  tui_directory_parent,
  tui_directory_path,
  tui_directory_register,
  tui_directory_roots,
  tui_directory_title,
} from '../../shared/tui-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.
const messages = {
  cancel: msg_cancel,
  directoryChoose: msg_directoryChoose,
  directoryEmpty: msg_directoryEmpty,
  directoryFailed: msg_directoryFailed,
  directoryFilter: msg_directoryFilter,
  directoryHelp: msg_directoryHelp,
  directoryLoading: msg_directoryLoading,
  directoryParent: msg_directoryParent,
  directoryRefresh: msg_directoryRefresh,
  directoryTitle: msg_directoryTitle,
  importsChooseSource: msg_importsChooseSource,
  searchTooLong: msg_searchTooLong,
  unavailable: msg_unavailable,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;

const terminalMessages = {
  'directory-browse': tui_directory_browse,
  'directory-empty': tui_directory_empty,
  'directory-failed': tui_directory_failed,
  'directory-no-roots': tui_directory_no_roots,
  'directory-open': tui_directory_open,
  'directory-parent': tui_directory_parent,
  'directory-path': tui_directory_path,
  'directory-register': tui_directory_register,
  'directory-roots': tui_directory_roots,
  'directory-title': tui_directory_title,
};
export function tuiText(
  locale: string,
  key: string,
  values: Record<string, string | number> = {},
): string {
  return translate(locale, terminalMessages, key, values);
}
