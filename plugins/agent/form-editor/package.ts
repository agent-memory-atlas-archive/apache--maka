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
  contract: own.formEditorContract,
  export: 'FormEditor',
  imports: { editor: one(sdk.contracts.textarea), reader: one(sdk.contracts.textReader) },
});
export default {
  schemaVersion: 1,
  id: 'maka.agent.form-editor',
  displayName: 'Form editor',
  description: 'Fill out structured information requested by the assistant.',
  localizations: {
    'zh-CN': { displayName: '表单填写', description: '填写助手请求的结构化信息。' },
  },
  application: { entries },
};
