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

import * as sdk from '../../../packages/plugin-sdk/src/index.js';
import * as own from '../../shared/contracts.js';
import { one, uiEntries } from '../../shared/package.js';

const entries = uiEntries({
  contract: own.sessionDetailsContract,
  export: 'SessionDetails',
  imports: {
    agent: one(sdk.contracts.agent),
    input: one(sdk.contracts.textInput),
    editor: one(sdk.contracts.textarea),
    reader: one(sdk.contracts.textReader),
    reading: one(sdk.contracts.readingArea),
    receipts: one(sdk.contracts.receiptReader),
    history: one(sdk.contracts.callerHistory),
  },
});
export default {
  schemaVersion: 1,
  id: 'maka.agent.session-details',
  displayName: 'Conversation settings',
  description: 'Edit a conversation’s name, model, and execution settings.',
  localizations: {
    'zh-CN': { displayName: '对话设置', description: '修改对话名称、模型和执行设置。' },
  },
  application: { entries },
};
