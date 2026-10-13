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
  AgentDirectory,
  AgentImportChange,
  AgentImportEntry,
  AgentImportLocation,
  AgentImportPreview,
  AgentImportSource,
  AgentImportSourceRef,
  AgentSession,
  EditorCommand,
  EditorObservation,
  ReadingCommand,
  RecoveryRef,
} from '../../../packages/plugin-sdk/src/index.js';
import { type Copy } from '../../shared/locale.js';
import { type PageState } from '../../shared/page-shell.js';

export type FieldName = 'name' | 'database' | 'search' | 'title';

export type Format = AgentImportLocation['kind'];

export type Route = 'sources' | 'form' | 'catalog' | 'preview' | 'copies';

export type Request =
  | { kind: 'sources' | 'mutate' | 'receipt' }
  | { kind: 'catalog' | 'copies'; after?: string | null }
  | { kind: 'preview'; after?: number }
  | { kind: 'directory'; purpose: 'source' | 'destination' }
  | { kind: 'field'; field: FieldName; observation: EditorObservation }
  | { kind: 'history'; after: number };

export interface Field {
  observation: EditorObservation | null;
  command: EditorCommand | null;
  reset: string | null;
  ready: boolean;
  sequence: number;
}

export interface State extends PageState {
  revision: number;
  serial: number;
  open: boolean;
  route: Route;
  focus: ReadingCommand | null;
  sources: AgentImportSource[];
  sourcesRevision: number;
  source: AgentImportSourceRef | null;
  entries: AgentImportEntry[];
  catalogNext: string | null;
  query: string;
  archived: boolean;
  preview:
    | (Omit<AgentImportPreview, 'items'> & { items: AgentImportPreview['items'][number][] })
    | null;
  destination: AgentDirectory | null;
  copies: AgentSession[];
  copiesNext: string | null;
  latest: AgentSession | null;
  selected: string | null;
  form: { id: string | null; format: Format; directory: AgentDirectory | null } | null;
  picker: 'source' | 'destination' | null;
  fields: Record<FieldName, Field>;
  capturedName: string | null;
  pending: { key: string; request: Request } | null;
  restoring: boolean;
  historyFailed: boolean;
  original: RecoveryRef | null;
  current: { input: AgentImportChange; reference: RecoveryRef | null } | null;
  confirm: AgentImportChange | null;
  notice: keyof Copy | null;
}

export const field = (): Field => ({
  observation: null,
  command: null,
  reset: null,
  ready: true,
  sequence: 0,
});

export const formatName = (format: Format) =>
  format === 'codex' ? 'Codex' : format === 'claude_code' ? 'Claude Code' : 'OpenCode';
