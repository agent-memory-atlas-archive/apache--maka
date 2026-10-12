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

import { ui } from '../../../packages/plugin-sdk/src/index.js';
import { composerFooterContract } from '../../shared/contracts.js';
const frame = (revision: number) => ({
  frame: {
    revision,
    state: { revision },
    surface: null,
    view: ui.desktop.text({ key: 'empty-footer', text: '', color: 0 }),
  },
  events: [],
  actions: [],
  subscriptions: [],
});
export const component = {
  mount: () => frame(0),
  update: (state: { revision: number }) => frame(state.revision + 1),
  event: (state: { revision: number }) => frame(state.revision + 1),
};
export function activate() {
  return {
    Footer: { kind: 'component', contract: composerFooterContract, component: ['component'] },
  };
}
