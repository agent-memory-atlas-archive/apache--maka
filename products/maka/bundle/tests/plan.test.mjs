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

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { contracts } from '../../../../packages/plugin-sdk/src/contracts.ts';
const { activate } = await import('../dist/packages/appearance/tui.js');
const theme = activate({ scheme: 'dark' }).Appearance.value;
async function client(target) {
  const module = await import(`../dist/packages/agent-screen/${target}.js`);
  module.activate({ locale: 'en' });
  const values = { appearance: [theme] };
  let out = module.component.mount({}, values);
  Object.assign(out.frame.state, {
    selected: {
      id: 'session',
      title: 'Plan test',
      labels: [],
      revision: 1,
      updatedAt: 1,
      archived: false,
      activity: 'idle',
      boundaryRevision: 7,
      hasUnread: false,
      dataSource: {
        stateDomain: 'a'.repeat(64),
        deployment: '00000000-0000-0000-0000-000000000001',
        namespace: ['00000000-0000-0000-0000-000000000002'],
      },
      permissions: { access: 'read_only', requestPermissions: true },
      workspace: { path: '/original', directoryIdentity: 'a'.repeat(64), identity: 'original' },
    },
    model: {
      choice: {
        selection: { connectionId: '00000000-0000-0000-0000-000000000003', modelId: 'fixture' },
        name: 'fixture',
      },
      thinkingLevel: null,
    },
    inspector: target === 'tui' ? 'plan' : null,
    dialog: target === 'desktop' ? 'plan' : null,
    modelRefreshing: false,
    pending: {},
    restoring: false,
  });
  out.frame.state.plan.page = {
    enabled: true,
    current: {
      revision: 1,
      proposal: {
        id: 'proposal',
        planId: 'plan',
        revision: 1,
        turnId: 'turn',
        status: 'pending_approval',
        supersedes: null,
        sourceExecutionId: null,
        submittedAt: 1,
        artifact: {
          title: 'Reviewed 中😀',
          overview: 'Exact version',
          steps: [
            {
              id: 'a',
              title: 'Implement',
              description: 'Original step',
              files: [],
              complexity: 'low',
            },
          ],
          risks: ['Original risk'],
        },
      },
      execution: null,
    },
  };
  const c = {
    get state() {
      return out.frame.state;
    },
    get out() {
      return out;
    },
    event(input) {
      out = module.component.event(c.state, input, {}, values);
      return c;
    },
    raw(key) {
      return c.event({ kind: 'raw', key, event: { kind: 'activate' } });
    },
    reply(value, key = out.actions[0].key) {
      return c.event({ kind: 'action', key, event: { status: 'succeeded', value } });
    },
  };
  return c;
}

for (const target of ['tui', 'desktop']) {
  test(`${target} restart settles original operations before restoring a newer draft`, async () => {
    const c = await client(target);
    const reference = {
      source: {
        stateDomain: c.state.selected.dataSource.stateDomain,
        deployment: c.state.selected.dataSource.deployment,
        revision: 1,
      },
      namespace: c.state.selected.dataSource.namespace,
      export: 'agent',
      method: 'send',
      operation: '00000000-0000-0000-0000-000000000004',
      fingerprint: 'b'.repeat(64),
      contract: contracts.agent,
      recovery: 'journal_v1',
    };
    const original = {
      cursor: 3,
      reference,
      recovery: {
        format: 'maka.agent.draft-send@1',
        sessionId: 'session',
        editor: { owner: 'original-editor', revision: 1 },
        attachments: [],
      },
    };
    c.state.restoring = true;
    c.state.pending.restore = { kind: 'restoreReceipt', reference, cursor: 1 };
    c.state.drafts = [
      {
        context: 'session',
        dataSource: c.state.selected.dataSource,
        observation: {
          owner: 'restored-editor',
          state: {
            kind: 'restore',
            version: 1,
            snapshot: { text: { owner: 'original-editor', revision: 2 } },
          },
        },
        work: {
          after: 3,
          done: false,
          queue: [original],
          clearText: false,
          attachments: [],
          valid: true,
          status: 'checking',
          restore: null,
        },
      },
    ];
    c.raw('plan-refresh');
    assert.equal(
      c.out.actions.length,
      0,
      'receipt restoration must not retain another draft receipt in parallel',
    );
    assert.ok(Buffer.byteLength(JSON.stringify(c.out)) < 1024 * 1024);
    c.reply(
      {
        reference,
        receipt: {
          kind: 'recorded',
          outcome: { kind: 'returned', value: {} },
          cleanup: { kind: 'confirmed' },
        },
      },
      'restore',
    );
    assert.equal(c.out.actions[0].method, 'page');
    c.reply([]);
    assert.equal(c.out.actions[0].method, 'read');
    assert.deepEqual(c.out.actions[0].input, reference, 'preserve the complete original receipt');
    assert.ok(Buffer.byteLength(JSON.stringify(c.out)) < 1024 * 1024);
    c.reply({
      reference,
      receipt: {
        kind: 'recorded',
        outcome: {
          kind: 'returned',
          value: { kind: 'started', sessionId: 'session', invocationId: 'original-invocation' },
        },
        cleanup: { kind: 'confirmed' },
      },
    });
    assert.equal(c.out.actions[0].method, 'page');
    c.reply([]);
    assert.equal(
      c.state.drafts[0].work.restore.clearText,
      false,
      'the later independent draft survives',
    );
  });
  test(`${target} Plan approval freezes reviewed version, model and permission boundary`, async () => {
    const c = await client(target);
    c.raw('plan-approve');
    const confirmation = structuredClone(c.state.plan.confirmation);
    assert.equal(c.out.actions.length, 0);
    assert.equal(confirmation.snapshot.proposal.artifact.steps[0].description, 'Original step');
    assert.equal(confirmation.input.action.approval.target.boundaryRevision, 7);
    c.state.selected.workspace.path = '/later';
    c.state.selected.permissions.access = 'workspace_write';
    c.state.model.choice.selection.modelId = 'later';
    c.raw('plan-confirm');
    assert.equal(c.out.actions[0].method, 'changePlan');
    assert.deepEqual(c.out.actions[0].input, confirmation.input);
  });
  test(`${target} newer Plan revision invalidates approval and history stays read-only`, async () => {
    const c = await client(target);
    c.raw('plan-approve');
    c.state.plan.watch = { key: 'original-plan-watch', input: { sessionId: 'session' } };
    const next = structuredClone(c.state.plan.page);
    next.current.revision = 2;
    next.current.proposal.artifact.steps[0].description = 'Revised step';
    c.event({
      kind: 'action',
      key: 'original-plan-watch',
      event: { status: 'updated', value: next },
    });
    assert.equal(c.state.plan.confirmation, null);
    c.raw('plan-confirm');
    assert.equal(c.out.actions.length, 0);
    c.raw('plan-history');
    assert.equal(c.out.actions[0].method, 'planHistory');
    assert.equal(c.out.actions[0].input.throughRevision, 2);
    const old = structuredClone(next.current);
    old.revision = 1;
    c.reply({ throughRevision: 2, snapshots: [old], nextAfter: 1 });
    c.raw('plan-approve');
    assert.equal(c.state.plan.confirmation, null);
    assert.equal(c.out.actions.length, 0);
    c.raw('plan-history-next');
    assert.equal(c.out.actions[0].input.throughRevision, 2);
    assert.equal(c.out.actions[0].input.after, 1);
  });
  test(`${target} Plan mode is an original metadata mutation preserving conversation settings`, async () => {
    const c = await client(target);
    const original = structuredClone(c.state.selected);
    c.raw('plan-mode-plan');
    assert.equal(c.out.actions[0].method, 'metadata');
    assert.deepEqual(c.out.actions[0].input, {
      sessionId: original.id,
      expectedRevision: original.revision,
      title: original.title,
      labels: original.labels,
      collaborationMode: 'plan',
    });
    assert.equal(
      c.state.selected.collaborationMode,
      undefined,
      'mode is not changed optimistically',
    );
  });
  test(`${target} Plan recovers the unknown original approval without a second mutation`, async () => {
    const c = await client(target);
    c.raw('plan-approve');
    c.raw('plan-confirm');
    const original = c.out.actions[0];
    const reference = {
      source: {
        stateDomain: 'a'.repeat(64),
        deployment: '00000000-0000-0000-0000-000000000001',
        revision: 1,
      },
      namespace: ['00000000-0000-0000-0000-000000000002'],
      export: 'agent',
      method: 'changePlan',
      operation: '00000000-0000-0000-0000-000000000004',
      fingerprint: 'sha256-' + 'b'.repeat(64),
      contract: {},
      recovery: 'journal_v1',
    };
    c.event({
      kind: 'action',
      key: original.key,
      event: { status: 'operation', reply: { reference, receipt: { kind: 'unknown' } } },
    });
    assert.equal(c.out.actions[0].method, 'planReceipt');
    assert.deepEqual(c.out.actions[0].input, { reference });
    const recovery = c.out.actions[0];
    c.raw('plan-approve');
    assert.equal(c.state.plan.confirmation, null);
    assert.equal(c.out.actions.length, 0);
    c.reply({ kind: 'committed', revision: 2 }, recovery.key);
    assert.equal(c.state.uncertain, null);
    assert.equal(c.state.plan.watch.input.sessionId, 'session');
    assert.ok(c.out.actions.every((action) => action.method !== 'changePlan'));
  });
}
