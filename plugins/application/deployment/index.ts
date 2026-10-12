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

import type { Action, Component } from '../../../packages/plugin-sdk/src/index.js';
import { bookmark } from '../../shared/bookmark.js';
import { deploymentContract, type DeploymentProps } from '../../shared/contracts.js';
import { packageImport } from '../../shared/deployment-import.js';
import { withPageShell } from '../../shared/page-shell.js';
import { type Node, type Presentation } from '../../shared/view.js';
import { handleEvent } from './events.js';
import { page } from './operations.js';
import { type SettingsBookmark, type State } from './state.js';
import { frame } from './view.js';

export function deployment<N extends Node>(
  presentation: Presentation<N>,
): Component<State, DeploymentProps, N> {
  return withPageShell<State, DeploymentProps, N>(
    {
      mount(props) {
        const state: State = {
          revision: 0,
          bookmark: bookmark<SettingsBookmark>(null),
          restoreSettings: false,
          restoreFailed: false,
          restoreOriginal: null,
          serial: 0,
          pending: null,
          page: null,
          selected: null,
          focus: null,
          after: null,
          previous: [],
          confirmation: null,
          current: null,
          notice: null,
          conflict: false,
          earlier: [],
          historyOpen: false,
          historyNext: null,
          settings: null,
          importer: packageImport(),
          document: null,
          documentOpen: false,
          diagnostic: null,
        };
        const actions: Action[] = [];
        page(state, actions, { kind: 'start' });
        return frame(presentation, state, props, actions);
      },
      update(state, props) {
        state.revision++;
        return frame(presentation, state, props);
      },
      event(state, input, props) {
        return handleEvent(presentation, state, input, props);
      },
    },
    (state) =>
      state.settings?.open
        ? 'instance-settings-back'
        : state.importer.open
          ? 'deployment-import-back'
          : state.documentOpen
            ? 'deployment-document-back'
            : 'deployment-back',
  );
}

export function activate() {
  return {
    Deployment: { kind: 'component', contract: deploymentContract, component: ['component'] },
  };
}
