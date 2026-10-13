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
  contract: own.serviceApprovalContract,
  export: 'ServiceApproval',
  imports: {
    agent: one(sdk.contracts.agent),
    timeline: one(own.timelineContract),
    reading: one(sdk.contracts.readingArea),
  },
});
export default {
  schemaVersion: 2,
  id: 'maka.agent.service-approval',
  displayName: 'Service requests',
  description: 'Review service calls that need your approval.',
  localizations: {
    'zh-CN': { displayName: '服务调用确认', description: '查看需要你确认的服务调用。' },
  },
  application: { entries },
};
