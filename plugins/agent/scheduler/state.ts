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
  AgentScheduleChange,
  AgentScheduledDetail,
  AgentScheduledEffect,
  AgentScheduledTarget,
  AgentScheduleRead,
  AgentSchedules,
  AgentSessionPage,
  EditorCommand,
  EditorObservation,
  RecoveryRef,
} from '../../../packages/plugin-sdk/src/index.js';
import { type SelectedModel } from '../../shared/contracts.js';
import { type Copy } from '../../shared/locale.js';
import { type PageState } from '../../shared/page-shell.js';

export type FieldName =
  | 'title'
  | 'body'
  | 'at'
  | 'seconds'
  | 'cron'
  | 'timezone'
  | 'max'
  | 'expiry';

export const names: FieldName[] = [
  'title',
  'body',
  'at',
  'seconds',
  'cron',
  'timezone',
  'max',
  'expiry',
];

export interface Field {
  observation: EditorObservation | null;
  command: EditorCommand | null;
  reset: string | null;
  ready: boolean;
  sequence: number;
}

export type Timing = 'once' | 'interval' | 'daily' | 'weekly' | 'monthly' | 'cron';

export interface Form {
  id: string | null;
  revision: number;
  timing: Timing;
  effect: AgentScheduledEffect['kind'];
  target: AgentScheduledTarget | null;
  sourceTitle: string | null;
  model: SelectedModel | null;
  misfire: 'skip' | 'latest';
}

export type Request =
  | { kind: 'history'; after: number }
  | { kind: 'field'; field: FieldName; observation: EditorObservation }
  | { kind: 'mutate' | 'receipt' | 'business' | 'sessions' | 'session' };

export interface State extends PageState {
  revision: number;
  serial: number;
  open: boolean;
  requested: number;
  route: 'list' | 'detail' | 'form';
  listCursor: string | null;
  listPrevious: (string | null)[];
  listLoading: boolean;
  scrollRevision: number;
  focused: string | null;
  page: Extract<AgentSchedules, { kind: 'page' }> | null;
  detail: AgentScheduledDetail | null;
  watch: { key: string; input: AgentScheduleRead } | null;
  watchFailed: boolean;
  form: Form | null;
  fields: Record<FieldName, Field>;
  capture: {
    values: Partial<Record<FieldName, string>>;
    observations: Record<FieldName, EditorObservation>;
  } | null;
  picker: boolean;
  sessions: AgentSessionPage | null;
  pending: { key: string; request: Request } | null;
  restoring: boolean;
  historyFailed: boolean;
  current: { input: AgentScheduleChange; reference: RecoveryRef | null } | null;
  confirm: AgentScheduleChange | null;
  confirmTask: AgentScheduledDetail | null;
  nextAction: {
    kind: 'edit' | 'approve' | 'resume' | 'trigger_now' | 'delete';
    taskId: string;
    revision: number;
  } | null;
  notice: keyof Copy | null;
}

export function field(): Field {
  return { observation: null, command: null, reset: null, ready: true, sequence: 0 };
}
