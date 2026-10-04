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

export const en = {
  previousModels: 'Previous',
  chooseModel: 'Choose model',
  models: 'Models',
  loadingModels: 'Loading models…',
  modelsUnavailable: 'Could not load models.',
  noModels: 'Add a connection to choose a model.',
  refreshModels: 'Refresh models',
  moreModels: 'More models',
  closeModels: 'Close',

  new: 'New conversation',
  conversations: 'Conversations',
  moreSessions: 'More conversations',
  send: 'Send',
  stop: 'Stop',
  placeholder: 'What would you like to work on?',
  draft: 'Message',
  edit: 'Edit message',
  undo: 'Undo',
  redo: 'Redo',
  cut: 'Cut',
  copy: 'Copy',
  paste: 'Paste',
  selectAll: 'Select all',
  welcome: 'A place to think.',
  welcomeCopy: 'Start a conversation and make room for the next idea.',
  loading: 'Loading conversation…',
  more: 'Read next messages',
  partial: 'Excerpt',
  you: 'You',
  assistant: 'Maka',
  working: 'Working…',
  waiting: 'Waiting for your input',
  stopping: 'Stopping…',
  unknown: 'The result is not yet known. Your message has been kept.',
  check: 'Check result',
  unavailable: 'Could not load the conversation.',
  sendFailed: 'The message was not sent. Your draft has been kept.',
  tooLarge: 'The message is too long to send.',
  empty: 'Write a message first.',
  draftKept: 'A newer draft was kept.',
  completed: 'Finished',
  failed: 'The response was interrupted.',
  enterHint: 'Enter for a new line · Tab to Send',
  selectFirst: 'Create or select a conversation',
  connecting: 'Connecting…',
} as const;
export type Copy = { [K in keyof typeof en]: string };
const zhCN: Copy = {
  previousModels: '上一页',
  chooseModel: '选择模型',
  models: '模型',
  loadingModels: '正在加载模型…',
  modelsUnavailable: '无法加载模型。',
  noModels: '添加连接后即可选择模型。',
  refreshModels: '刷新模型',
  moreModels: '更多模型',
  closeModels: '关闭',

  new: '新建对话',
  conversations: '对话',
  moreSessions: '更多对话',
  send: '发送',
  stop: '停止',
  placeholder: '你想一起完成什么？',
  draft: '消息',
  edit: '编辑消息',
  undo: '撤销',
  redo: '重做',
  cut: '剪切',
  copy: '复制',
  paste: '粘贴',
  selectAll: '全选',
  welcome: '留一点空间，想点新东西。',
  welcomeCopy: '开启一段对话，让想法慢慢成形。',
  loading: '正在加载对话…',
  more: '阅读后续消息',
  partial: '节选',
  you: '你',
  assistant: 'Maka',
  working: '正在处理…',
  waiting: '等待你的输入',
  stopping: '正在停止…',
  unknown: '结果暂未确认，消息已保留。',
  check: '查询结果',
  unavailable: '无法加载对话。',
  sendFailed: '消息未发送，草稿已保留。',
  tooLarge: '消息过长，暂时无法发送。',
  empty: '请先输入消息。',
  draftKept: '已保留后来编辑的草稿。',
  completed: '已完成',
  failed: '回复已中断。',
  enterHint: 'Enter 换行 · Tab 前往发送',
  selectFirst: '新建或选择一段对话',
  connecting: '正在连接…',
};
const zhTW: Copy = {
  previousModels: '上一頁',
  chooseModel: '選擇模型',
  models: '模型',
  loadingModels: '正在載入模型…',
  modelsUnavailable: '無法載入模型。',
  noModels: '新增連線後即可選擇模型。',
  refreshModels: '重新整理模型',
  moreModels: '更多模型',
  closeModels: '關閉',

  new: '新增對話',
  conversations: '對話',
  moreSessions: '更多對話',
  send: '傳送',
  stop: '停止',
  placeholder: '你想一起完成什麼？',
  draft: '訊息',
  edit: '編輯訊息',
  undo: '復原',
  redo: '重做',
  cut: '剪下',
  copy: '複製',
  paste: '貼上',
  selectAll: '全選',
  welcome: '留一點空間，想點新東西。',
  welcomeCopy: '開啟一段對話，讓想法慢慢成形。',
  loading: '正在載入對話…',
  more: '閱讀後續訊息',
  partial: '節選',
  you: '你',
  assistant: 'Maka',
  working: '正在處理…',
  waiting: '等待你的輸入',
  stopping: '正在停止…',
  unknown: '結果暫未確認，訊息已保留。',
  check: '查詢結果',
  unavailable: '無法載入對話。',
  sendFailed: '訊息未傳送，草稿已保留。',
  tooLarge: '訊息過長，暫時無法傳送。',
  empty: '請先輸入訊息。',
  draftKept: '已保留後來編輯的草稿。',
  completed: '已完成',
  failed: '回覆已中斷。',
  enterHint: 'Enter 換行 · Tab 前往傳送',
  selectFirst: '新增或選擇一段對話',
  connecting: '正在連線…',
};
export function copy(locale: string): Copy {
  const tag = locale.toLowerCase();
  return tag.startsWith('zh')
    ? tag.includes('hant') ||
      tag.startsWith('zh-tw') ||
      tag.startsWith('zh-hk') ||
      tag.startsWith('zh-mo')
      ? zhTW
      : zhCN
    : en;
}
