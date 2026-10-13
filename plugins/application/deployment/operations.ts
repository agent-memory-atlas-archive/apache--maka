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

import type {
  Action,
  DeploymentInstances,
  InstanceSettingsRead,
  ManagedInstance,
  ManagementResultForDeploymentRef,
  ManagementResultForPackageInstalled,
  OperationReply,
} from '../../../packages/plugin-sdk/src/index.js';
import { instanceSettings } from '../../shared/instance-settings.js';
import { pluginDescription } from '../../shared/plugin-description.js';
import { sameValue } from '../../shared/value.js';
import { type DocumentRequest, type Request, type State } from './state.js';

export function request(
  state: State,
  actions: Action[],
  r: Request | DocumentRequest,
  binding: string,
  method: string,
  input: unknown,
  resources: Action['resources'] = {},
  recovery?: unknown,
) {
  if (state.pending) return;
  // Initialization owns the one request lane until the final native receipt.
  // A navigation/history query cannot consume it between two document chunks.
  if (state.document?.phase === 'loading' && r.kind !== 'document') return;
  const key = 'deployment-' + ++state.serial;
  state.pending = { key, request: r };
  actions.push({
    key,
    binding,
    member: 0,
    method,
    input,
    resources,
    ...(recovery ? { recovery } : {}),
  });
}

export function page(
  state: State,
  actions: Action[],
  input: DeploymentInstances,
  movement: 'reset' | 'next' = 'reset',
) {
  request(state, actions, { kind: 'page', input, movement }, 'management', 'instances', input);
}

export function readDocument(state: State, actions: Action[], offset: number) {
  const source = state.document!.source;
  request(state, actions, { kind: 'document', source, offset }, 'management', 'readDocument', {
    source,
    offset,
  });
}

export function readSettings(state: State, actions: Action[]) {
  const settings = state.settings!;
  if (!settings.after) {
    settings.page = null;
    settings.window = 0;
  }
  const input: InstanceSettingsRead = {
    source: settings.source,
    instance: settings.instance.instance,
    path: settings.path as InstanceSettingsRead['path'],
    after: settings.after,
  };
  request(state, actions, { kind: 'settings', input }, 'management', 'readSettings', input);
}

export function openSettings(state: State, actions: Action[], instance: ManagedInstance) {
  if (!state.page || instance.structural) return;
  state.documentOpen = false;
  state.importer.open = false;
  state.historyOpen = false;
  if (state.settings && sameValue(state.settings.instance.instance, instance.instance)) {
    state.settings.open = true;
    return;
  }
  if (state.settings?.changes.length) {
    state.settings.open = true;
    state.settings.discard = instance;
    return;
  }
  state.settings = instanceSettings(state.page.source, instance);
  state.notice = null;
  state.diagnostic = null;
  readSettings(state, actions);
}

export function settle(state: State, reply: OperationReply, actions: Action[]) {
  const current = state.current;
  if (!current || (current.reference && !sameValue(current.reference, reply.reference))) return;
  const firstReceipt = !current.reference;
  current.reference = reply.reference;
  if (reply.receipt.kind !== 'recorded' || reply.receipt.outcome.kind !== 'returned') {
    state.notice = 'deploymentUnknown';
    if (firstReceipt)
      request(
        state,
        actions,
        { kind: 'receipt', reference: reply.reference },
        'receipts',
        'read',
        reply.reference,
      );
    if ('configuration' in current.intent && state.settings && !state.settings.page)
      readSettings(state, actions);
    return;
  }
  const isInstall = 'file' in current.intent || 'url' in current.intent;
  const value = reply.receipt.outcome.value as
    | ManagementResultForDeploymentRef
    | ManagementResultForPackageInstalled;
  const isDocument = 'editor' in current.intent;
  const isSettings = 'configuration' in current.intent;
  if (value.status === 'committed') {
    if (isInstall && 'package' in value.value) {
      state.importer.installed = value.value;
      if (state.page) state.page.source = value.value.source;
      state.importer.confirmation = null;
    }
    if (isDocument && state.document && !('package' in value.value)) {
      state.document.source = value.value;
      state.document.confirmation = null;
      state.document.conflict = false;
    }
    if (isSettings && state.settings && !('package' in value.value)) {
      state.settings.source = value.value;
      state.settings.changes = [];
      state.settings.conflict = false;
      state.settings.notice = null;
    }
    state.current = null;
    state.confirmation = null;
    state.notice = isInstall ? 'deploymentInstalled' : 'deploymentSaved';
    state.conflict = false;
    if (isSettings) readSettings(state, actions);
    else page(state, actions, { kind: 'start' });
  } else {
    if (isSettings && state.settings && !state.settings.page) readSettings(state, actions);
    if (isSettings && state.settings) state.settings.conflict = value.status === 'conflict';
    if (isInstall) state.importer.confirmation = null;
    if (isDocument && state.document) {
      state.document.confirmation = null;
      state.document.conflict = value.status === 'conflict';
    }
    state.diagnostic = value.status === 'rejected' ? (value.diagnostic ?? null) : null;
    state.current = null;
    state.conflict = value.status === 'conflict';
    state.notice =
      value.status === 'conflict'
        ? isSettings
          ? 'settingsChanged'
          : 'deploymentConflict'
        : isInstall
          ? 'deploymentInstallRejected'
          : isDocument
            ? value.reason === 'execution_environment'
              ? 'deploymentEnvironment'
              : value.reason === 'package_unavailable'
                ? 'deploymentPackageUnavailable'
                : 'deploymentDocumentRejected'
            : value.reason === 'parent_disabled'
              ? 'deploymentParentDisabled'
              : 'deploymentRejected';
  }
}

export function filteredInstances(state: State, locale: string) {
  const terms = state.query.toLocaleLowerCase().split(/\s+/u).filter(Boolean);
  return (state.page?.instances ?? [])
    .map((entry, at) => ({ entry, at }))
    .filter(({ entry }) => {
      const info = pluginDescription(entry.presentation, locale, entry.package.id);
      const text = [info.name, info.description, entry.package.id, ...entry.path]
        .join(' ')
        .toLocaleLowerCase();
      return terms.every((term) => text.includes(term));
    });
}
