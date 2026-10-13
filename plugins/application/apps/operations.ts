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
  BindingCandidate,
  BindingCandidatesRead,
  DeploymentRef,
  ManagementResultForDeploymentRef,
  ManagementResultForPackageInstalled,
  OperationReply,
  SettingChange,
  SettingField,
  SettingValue,
  Target,
} from '../../../packages/plugin-sdk/src/index.js';
import { type AppEntry, type AppsConfiguration, type AppsProps } from '../../shared/contracts.js';
import { sameValue } from '../../shared/value.js';
import { type Draft, type Field, type Request, type State } from './state.js';

export function initialize(field: Field) {
  if (field.reset !== null && field.observation && !field.command)
    field.command = {
      owner: field.observation.snapshot.text.owner,
      basedOn: field.observation.revision,
      sequence: ++field.sequence,
      edit: { kind: 'reset', text: field.reset },
    };
}

export function reset(state: State, name: 'name' | 'setting', text: string) {
  const field = state.fields[name];
  field.reset = text;
  field.ready = false;
  initialize(field);
}

export function request(
  state: State,
  actions: Action[],
  r: Request,
  binding: string,
  method: string,
  input: unknown,
  resources: Action['resources'] = {},
) {
  if (state.pending) return;
  const key = 'apps-' + ++state.serial;
  state.pending = { key, request: r };
  actions.push({ key, binding, member: 0, method, input, resources });
}

export function catalog(
  state: State,
  actions: Action[],
  configuration: AppsConfiguration,
  source: DeploymentRef | null = null,
  after: string | null = null,
) {
  if (!after && !state.pending) state.listWindow = 0;
  const input: BindingCandidatesRead = {
    source,
    consumer: { kind: 'root', name: configuration.root },
    import: 'pages',
    includePackages: true,
    after,
  };
  request(state, actions, { kind: 'catalog', input }, 'management', 'bindingCandidates', input);
}

export function settings(state: State, actions: Action[]) {
  const d = state.draft,
    source = state.catalog?.source;
  if (!source || d?.candidate.kind !== 'package') return;
  request(
    state,
    actions,
    { kind: 'settings', path: d.path, after: d.after },
    'management',
    'packageSettings',
    { source, package: d.candidate.package, path: d.path, after: d.after, changes: d.changes },
  );
}

export function bindings(state: State, actions: Action[], after: string | null = null) {
  const d = state.draft,
    source = state.catalog?.source;
  if (!source || d?.candidate.kind !== 'package' || !d.choosing) return;
  if (!after && !state.pending) state.listWindow = 0;
  const input: BindingCandidatesRead = {
    source,
    consumer: { kind: 'package', package: d.candidate.package },
    import: d.choosing.name,
    includePackages: false,
    after,
  };
  request(state, actions, { kind: 'bindings', input }, 'management', 'bindingCandidates', input);
}

export function history(state: State, actions: Action[], after = 0) {
  state.restoring = true;
  state.historyFailed = false;
  if (!after) state.original = null;
  request(state, actions, { kind: 'history', after }, 'history', 'page', { after, limit: 16 });
}

export function locked(state: State, props: AppsProps) {
  return (
    !props.open ||
    props.busy ||
    !!state.pending ||
    !!state.current ||
    state.restoring ||
    state.historyFailed
  );
}

export function capture(state: State, actions: Action[], name: 'name' | 'setting') {
  const field = state.fields[name],
    observation = field.observation;
  if (!observation || observation.snapshot.marked || field.command || !field.ready) return;
  request(
    state,
    actions,
    { kind: 'field', name, observation },
    'reader',
    'read',
    { start: 0, end: observation.snapshot.text.bytes },
    {
      text: {
        child: 'apps-' + name,
        method: 'snapshot',
        input: {
          kind: 'at_revision',
          owner: observation.snapshot.text.owner,
          revision: observation.snapshot.text.revision,
        },
      },
    },
  );
}

export function value(draft: Draft, f: SettingField): SettingValue | null {
  const change = draft.changes.find((c) => sameValue(c.path, [...draft.path, f.key]));
  return (change ? change.value : f.value) ?? null;
}

export function change(draft: Draft, path: string[], v: SettingValue | null) {
  const at = draft.changes.findIndex((c) => sameValue(c.path, path));
  const item: SettingChange = { path: path as SettingChange['path'], value: v };
  if (at < 0) draft.changes.push(item);
  else draft.changes[at] = item;
}

export function complete(d: Draft) {
  return (
    d.candidate.kind === 'bound' ||
    (!!d.page &&
      d.page.imports.every((i) => {
        const count = (d.bindings[i.name] ?? []).length;
        return i.cardinality.kind === 'one'
          ? count === 1
          : i.cardinality.kind === 'optional'
            ? count <= 1
            : count >= i.cardinality.min && count <= i.cardinality.max;
      }))
  );
}

export function entries(state: State): AppEntry[] {
  return (state.catalog?.configuration.entries ?? []) as AppEntry[];
}

export function edit(state: State, configuration: AppsConfiguration, label: string) {
  const page = state.catalog,
    draft = state.draft;
  if (!page || !draft || !complete(draft)) {
    state.notice = 'appsBindingsMissing';
    return;
  }
  const candidate = draft.candidate;
  const target: Target =
    candidate.kind === 'bound'
      ? candidate.target
      : { kind: 'instance', path: [draft.id], export: candidate.export };
  state.confirm = {
    kind: 'edit',
    outcome: 'add',
    input: {
      expectedRevision: page.source.revision,
      edits: [
        ...(candidate.kind === 'package'
          ? [
              {
                kind: 'insert' as const,
                name: draft.id,
                package: candidate.package,
                settings: draft.changes,
                bindings: draft.bindings,
              },
            ]
          : []),
        {
          kind: 'update',
          name: configuration.root,
          configuration: { entries: [...entries(state), { id: draft.id, label }] },
          bindings: { pages: [...page.current, target] },
        },
      ],
    },
  };
}

export function mutate(state: State, actions: Action[]) {
  const input = state.confirm;
  if (!input) return;
  state.current = { input, reference: null };
  state.notice = null;
  state.diagnostic = null;
  request(
    state,
    actions,
    { kind: 'mutation' },
    'management',
    input.kind === 'install' ? 'install' : 'editComposition',
    input.kind === 'install' ? { expectedRevision: input.source.revision } : input.input,
    input.kind === 'install'
      ? {
          package: {
            child: 'apps-file',
            method: 'snapshot',
            input: { owner: input.file.owner, revision: input.file.revision },
          },
        }
      : {},
  );
  actions.at(-1)!.recovery = { kind: 'application-apps/v1', input };
}

export function settle(
  state: State,
  reply: OperationReply,
  actions: Action[],
  configuration: AppsConfiguration,
) {
  if (
    !state.current ||
    (state.current.reference && !sameValue(reply.reference, state.current.reference))
  )
    return;
  state.current.reference = reply.reference;
  if (reply.receipt.kind !== 'recorded' || reply.receipt.outcome.kind !== 'returned') {
    state.notice = 'appsUnknown';
    return;
  }
  const result = reply.receipt.outcome.value as
    | ManagementResultForDeploymentRef
    | ManagementResultForPackageInstalled;
  const input = state.current.input;
  state.current = null;
  state.confirm = null;
  if (result.status === 'committed') {
    state.notice =
      input.kind === 'install'
        ? 'appsPackageAdded'
        : input.outcome === 'add'
          ? 'appsAdded'
          : 'appsRemoved';
    state.draft = null;
    state.route = input.kind === 'install' ? 'choose' : 'list';
    catalog(state, actions, configuration);
  } else {
    state.notice = result.status === 'conflict' ? 'appsChanged' : 'appsInvalid';
    state.diagnostic = result.status === 'rejected' ? (result.diagnostic ?? null) : null;
  }
}

export function choose(state: State, candidate: BindingCandidate, actions: Action[]) {
  state.notice = null;
  state.diagnostic = null;
  const label = candidate.label;
  state.draft = {
    candidate,
    id: 'app-' + state.catalog!.source.revision + '-' + ++state.serial,
    label,
    changes: [],
    bindings: {},
    page: null,
    path: [],
    after: null,
    previous: [],
    setting: null,
    choosing: null,
    choices: null,
  };
  state.route = 'configure';
  reset(state, 'name', label);
  if (candidate.kind === 'package') settings(state, actions);
}
