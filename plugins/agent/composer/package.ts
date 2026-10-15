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
  contract: own.composerContract,
  export: 'Composer',
  imports: {
    editor: one(sdk.contracts.textarea),
    reader: one(sdk.contracts.textReader),
    agent: one(sdk.contracts.agent),
    history: one(sdk.contracts.callerHistory),
    receipts: one(sdk.contracts.receiptReader),
  },
});
export default {
  schemaVersion: 1,
  id: 'maka.agent.composer',
  displayName: 'Message editor',
  description: 'Write messages and add attachments before sending.',
  localizations: {
    'zh-CN': { displayName: '消息编辑器', description: '编写消息，添加附件并发送。' },
  },
  application: { entries },
};
