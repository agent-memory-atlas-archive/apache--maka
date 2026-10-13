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
  BindingCandidate,
  BindingCandidatesPage,
  BindingCandidatesRead,
  CompositionEdit,
  DeploymentRef,
  EditorCommand,
  EditorObservation,
  FileObservation,
  ManagedImport,
  PackageSettingsPage,
  RecoveryRef,
  SettingChange,
  SettingField,
  Target,
} from '../../../packages/plugin-sdk/src/index.js';
import { type Copy } from '../../shared/locale.js';
import { type PageState } from '../../shared/page-shell.js';

export type Mutation =
  | { kind: 'install'; source: DeploymentRef; file: FileObservation }
  | { kind: 'edit'; input: CompositionEdit; outcome: 'add' | 'remove' };

export type Request =
  | { kind: 'catalog' | 'bindings'; input: BindingCandidatesRead }
  | { kind: 'settings'; path: string[]; after: string | null }
  | { kind: 'field'; name: 'name' | 'setting'; observation: EditorObservation }
  | { kind: 'mutation' | 'receipt' }
  | { kind: 'history'; after: number };

export interface Field {
  observation: EditorObservation | null;
  command: EditorCommand | null;
  reset: string | null;
  sequence: number;
  ready: boolean;
}

export interface Draft {
  candidate: BindingCandidate;
  id: string;
  label: string;
  changes: SettingChange[];
  bindings: Record<string, Target[]>;
  page: PackageSettingsPage | null;
  path: string[];
  after: string | null;
  previous: (string | null)[];
  setting: { descriptor: SettingField; path: string[] } | null;
  choosing: ManagedImport | null;
  choices: BindingCandidatesPage | null;
}

export interface State extends PageState {
  revision: number;
  serial: number;
  open: boolean;
  route: 'list' | 'app' | 'choose' | 'configure' | 'bindings' | 'file';
  catalog: BindingCandidatesPage | null;
  selected: string | null;
  listWindow: number;
  scrollRevision: number;
  focused: string | null;
  opened: string[];
  draft: Draft | null;
  fields: { name: Field; setting: Field };
  file: FileObservation | null;
  pending: { key: string; request: Request } | null;
  original: RecoveryRef | null;
  restoring: boolean;
  historyFailed: boolean;
  current: { input: Mutation; reference: RecoveryRef | null } | null;
  confirm: Mutation | null;
  notice: keyof Copy | null;
  diagnostic: string | null;
}

export const field = (): Field => ({
  observation: null,
  command: null,
  reset: null,
  sequence: 0,
  ready: true,
});
