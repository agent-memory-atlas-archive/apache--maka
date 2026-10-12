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
  DeploymentInstances,
  DeploymentRef,
  EditorObservation,
  InstanceConfigure,
  InstanceSettingsRead,
  ManagedInstance,
  ManagedInstancesPage,
  OperationReply,
  ReadingCommand,
  RecoveryRef,
} from '../../../packages/plugin-sdk/src/index.js';
import { type Bookmark } from '../../shared/bookmark.js';
import { type DocumentEditor } from '../../shared/deployment-document.js';
import { type InstallIntent, type PackageImport } from '../../shared/deployment-import.js';
import { type InstanceSettings } from '../../shared/instance-settings.js';
import { type Copy } from '../../shared/locale.js';
import { type PageState } from '../../shared/page-shell.js';

export type Request =
  | { kind: 'page'; input: DeploymentInstances; movement: 'reset' | 'next' | 'previous' }
  | { kind: 'restoreSettings'; after: number }
  | { kind: 'mutate' }
  | { kind: 'receipt'; reference: RecoveryRef }
  | { kind: 'history'; after: number };

export type DocumentRequest =
  | { kind: 'document'; source: DeploymentRef; offset: number }
  | { kind: 'documentReload' }
  | { kind: 'urlText'; captured: EditorObservation; source: DeploymentRef }
  | { kind: 'settings'; input: InstanceSettingsRead }
  | { kind: 'settingsReload' }
  | { kind: 'settingText'; captured: EditorObservation; path: string[] };

export interface Intent {
  source: DeploymentRef;
  instance: ManagedInstance;
  enabled: boolean;
}

export type SettingsBookmark =
  | (Pick<InstanceSettings, 'source' | 'instance' | 'path' | 'changes' | 'open'> & {
      saved: boolean;
    })
  | null;

export function settingsBookmark(state: State): SettingsBookmark {
  const settings = state.settings;
  return settings
    ? {
        source: settings.source,
        instance: settings.instance,
        path: settings.path,
        changes: settings.changes,
        open: settings.open,
        saved: state.notice === 'deploymentSaved',
      }
    : null;
}

export function validSettingsBookmark(value: unknown): SettingsBookmark {
  const item = value as SettingsBookmark;
  const path = (value: unknown): value is string[] =>
    Array.isArray(value) && value.length <= 16 && value.every((part) => typeof part === 'string');
  return item &&
    item.source &&
    typeof item.source.stateDomain === 'string' &&
    typeof item.source.deployment === 'string' &&
    Number.isSafeInteger(item.source.revision) &&
    item.source.revision > 0 &&
    item.instance &&
    path(item.instance.instance) &&
    item.instance.instance.length > 0 &&
    path(item.instance.path) &&
    path(item.path) &&
    Array.isArray(item.changes) &&
    item.changes.length <= 32 &&
    item.changes.every(
      (change) =>
        change &&
        path(change.path) &&
        change.path.length > 0 &&
        (change.value === null ||
          change.value?.kind === 'null' ||
          (change.value?.kind === 'boolean' && typeof change.value.value === 'boolean') ||
          ((change.value?.kind === 'text' || change.value?.kind === 'number') &&
            typeof change.value.text === 'string')),
    ) &&
    typeof item.open === 'boolean' &&
    typeof item.saved === 'boolean'
    ? item
    : null;
}

export interface State extends PageState {
  bookmark: Bookmark<SettingsBookmark>;
  restoreSettings: boolean;
  restoreFailed: boolean;
  restoreOriginal: RecoveryRef | null;
  revision: number;
  serial: number;
  pending: { key: string; request: Request | DocumentRequest } | null;
  page: ManagedInstancesPage | null;
  selected: ManagedInstance['instance'] | null;
  focus: ReadingCommand | null;
  after: ManagedInstancesPage['next'];
  previous: ManagedInstancesPage['next'][];
  confirmation: Intent | null;
  current: {
    intent:
      | Intent
      | { source: DeploymentRef; editor: EditorObservation }
      | { source: DeploymentRef; configuration: InstanceConfigure }
      | InstallIntent;
    reference: RecoveryRef | null;
  } | null;
  settings: InstanceSettings | null;
  importer: PackageImport;
  document: DocumentEditor | null;
  documentOpen: boolean;
  diagnostic: string | null;
  notice: keyof Copy | null;
  conflict: boolean;
  earlier: { cursor: number; reference: RecoveryRef; reply: OperationReply | null }[];
  historyOpen: boolean;
  historyNext: number | null;
}
