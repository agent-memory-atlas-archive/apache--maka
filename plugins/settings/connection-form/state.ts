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
  ConnectionVersionBasis,
  EditorCommand,
  EditorObservation,
  EditorTextRef,
  OperationReply,
  ReadingCommand,
  RecoveryRef,
  SettingsConnectionDetail,
  SettingsProvider,
  SettingsProviderSummary,
  Identity,
} from '../../../packages/plugin-sdk/src/index.js';
import { type Search } from '../search.js';
import { type Copy } from './locale.js';
import { type PageState } from '../../shared/page-shell.js';

export type Detail = Extract<SettingsConnectionDetail, { kind: 'ready' }>;

export interface SettingsValidation {
  create(input: unknown): boolean;
  update(input: unknown): boolean;
}

export const names = [
  'name',
  'slug',
  'endpoint',
  'models',
  'configuration',
  'overrides',
  'overlay',
] as const;

export type FieldName = (typeof names)[number];

export const advancedFields: readonly FieldName[] = [
  'slug',
  'models',
  'configuration',
  'overrides',
  'overlay',
];

export interface Field {
  enabled: boolean;
  observation: EditorObservation | null;
  command: EditorCommand | null;
  sequence: number;
  initialized: boolean;
  initialText: EditorTextRef | null;
  failed: boolean;
}

export type Request =
  | { kind: 'providers'; offset: number }
  | { kind: 'provider'; identity: Identity }
  | { kind: 'read'; field: FieldName; captured: EditorObservation; attempt: number }
  | { kind: 'mutate' }
  | { kind: 'receipt'; reference: RecoveryRef }
  | { kind: 'review'; basis: ConnectionVersionBasis; retried: boolean }
  | { kind: 'reviewCatalog' }
  | { kind: 'history'; after: number }
  | { kind: 'earlier'; reference: RecoveryRef; cursor: number };

export interface State extends PageState {
  revision: number;
  serial: number;
  opened: Detail | null;
  baseline: Detail | null;
  catalogRevision: number;
  provider: SettingsProvider | null;
  providers: SettingsProviderSummary[];
  search: Search;
  providerActive: number;
  providerGroup: string | null;
  providerFocus: ReadingCommand | null;
  providersFailed: boolean;
  fields: Record<FieldName, Field>;
  enabled: boolean;
  enabledEdited: boolean;
  defaultSlug: string;
  advanced: boolean;
  pending: Record<string, Request>;
  current: {
    kind: 'save' | 'remove';
    attempt: number;
    values: Partial<Record<FieldName, string>>;
    remaining: FieldName[];
    reference: RecoveryRef | null;
  } | null;
  notice: keyof Copy | null;
  invalidField: FieldName | null;
  conflict: boolean;
  reviewed: Detail | null;
  reviewedCatalog: number | null;
  reviewPage: number;
  confirmation: 'leave' | 'rebase' | 'remove' | null;
  historyOpen: boolean;
  earlier: { cursor: number; reference: RecoveryRef; result: OperationReply | null }[];
  historyNext: number | null;
  historyFailed: boolean;
}
