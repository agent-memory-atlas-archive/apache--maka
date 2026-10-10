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
import '../scripts/build.mjs';
import { contracts } from '../../plugin-sdk/src/contracts.ts';
const { activate } = await import('../dist/packages/appearance/desktop.js');
const theme = activate({ scheme: 'light' }).Appearance.value;
const entry = {
  reference: 'source:local:review',
  source: 'Local',
  name: 'Review',
  description: 'Review a change',
  valid: true,
  installed: false,
  shadowedBy: null,
  metadataTruncated: false,
  issues: [],
};
const digest = 'sha256:' + 'a'.repeat(64);
const reference = {
  source: {
    deployment: '00000000-0000-0000-0000-000000000001',
    revision: 4,
    stateDomain: 'a'.repeat(64),
  },
  namespace: ['00000000-0000-0000-0000-000000000002'],
  export: 'skills',
  method: 'apply',
  operation: '00000000-0000-0000-0000-000000000003',
  fingerprint: 'sha256-' + 'a'.repeat(64),
  contract: contracts.skillLibrary,
  recovery: 'journal_v1',
};
async function client(target, writable = true) {
  const { component } = await import(`../dist/packages/skill-library/${target}.js`);
  let props = { theme, locale: 'en', open: true };
  let out = component.mount(props, {});
  const c = {
    get out() {
      return out;
    },
    get state() {
      return out.frame.state;
    },
    event(input) {
      out = component.event(c.state, input, props, {});
      return c;
    },
    raw(key) {
      return c.event({ kind: 'raw', key, event: { kind: 'activate' } });
    },
    update(next) {
      props = { ...props, ...next };
      out = component.update(c.state, props, {});
      return c;
    },
    reply(action, value) {
      return c.event({ kind: 'action', key: action.key, event: { status: 'succeeded', value } });
    },
    action(method) {
      return out.actions.find((action) =>
        method === 'outcome' ? action.binding === 'outcomes' : action.method === method,
      );
    },
    open() {
      c.reply(c.action('list'), {
        kind: 'page',
        writable,
        revision: digest,
        entries: [entry],
        after: null,
        diagnostics: [],
      });
      c.raw(`${c.state.pageKey}-row-0`);
      c.reply(c.action('read'), {
        kind: 'page',
        writable,
        revision: digest,
        entry,
        text: 'Instructions',
        offset: 0,
        before: null,
        after: null,
        totalBytes: 12,
      });
      return c;
    },
    preview() {
      c.raw('install-skill');
      const action = c.action('preview');
      assert.ok(action);
      c.reply(action, {
        kind: 'ready',
        review: { change: action.input, digest },
        target: 'installed:review',
        entries: 5,
        paths: ['SKILL.md', 'scripts', 'scripts/check'],
        pathsTruncated: false,
        locallyModified: false,
      });
      return c;
    },
    operation(action, receipt, ref = reference) {
      return c.event({
        kind: 'action',
        key: action.key,
        event: { status: 'operation', reply: { reference: ref, receipt } },
      });
    },
  };
  return c;
}
for (const target of ['tui', 'desktop']) {
  test(`${target} skill changes require writable access and current explicit confirmation`, async () => {
    const readonly = (await client(target, false)).open();
    readonly.raw('install-skill');
    assert.equal(readonly.out.actions.length, 0);
    const c = (await client(target)).open().preview();
    c.raw('cancel-change').raw('confirm-change');
    assert.equal(c.out.actions.length, 0);
    c.preview().raw('close').raw('confirm-change');
    assert.equal(c.out.actions.length, 0);
    c.preview().raw('confirm-change');
    assert.equal(c.action('apply').input.change.reference, entry.reference);
    c.raw('confirm-change');
    assert.equal(
      c.out.actions.length,
      0,
      'a second confirmation cannot dispatch another operation',
    );
  });
  test(`${target} closing skills retains the accepted operation and recovers only its original receipt`, async () => {
    const c = (await client(target)).open().preview().raw('confirm-change');
    const mutation = c.action('apply');
    c.raw('close').update({ open: false });
    c.operation(mutation, { kind: 'unavailable', diagnostic: 'reply lost' });
    assert.deepEqual(c.state.current.reference, reference);
    assert.equal(c.out.actions.length, 0);
    c.update({ open: true });
    const recovery = c.action('read');
    assert.equal(recovery.binding, 'receipts');
    assert.deepEqual(recovery.input, reference);
    c.reply(recovery, {
      reference: { ...reference, namespace: ['00000000-0000-0000-0000-000000000004'] },
      receipt: {
        kind: 'recorded',
        outcome: { kind: 'returned', value: { kind: 'applied', reference: 'installed:review' } },
        cleanup: { kind: 'confirmed' },
      },
    });
    assert.deepEqual(c.state.current.reference, reference);
    c.raw('install-skill').raw('confirm-change');
    assert.equal(c.out.actions.length, 0);
    c.raw('check-change');
    const original = c.action('read');
    assert.deepEqual(original.input, reference);
    c.reply(original, {
      reference,
      receipt: {
        kind: 'recorded',
        outcome: { kind: 'returned', value: { kind: 'applied', reference: 'installed:review' } },
        cleanup: { kind: 'confirmed' },
      },
    });
    assert.equal(c.state.current, null);
    assert.ok(c.action('list'));
    assert.ok(!c.action('apply'), 'receipt observation must not replay installation');
  });
  test(`${target} canonical skill outcomes preserve original cleanup and never replay a lost mutation`, async () => {
    const c = (await client(target)).open().preview().raw('confirm-change');
    c.operation(c.action('apply'), { kind: 'unavailable', diagnostic: 'Host disconnected' });
    const receipt = {
      kind: 'recorded',
      outcome: { kind: 'unresolved' },
      cleanup: { kind: 'unconfirmed' },
    };
    c.raw('check-change');
    c.reply(c.action('read'), { reference, receipt });
    let query = c.action('outcome');
    assert.equal(query.binding, 'outcomes');
    assert.deepEqual(query.input, reference);
    c.reply(query, { original: { reference, receipt }, published: null });
    assert.deepEqual(c.state.current.reference, reference);
    assert.equal(c.state.notice, 'skillsUnknown');
    assert.equal(c.out.actions.length, 0, 'missing publication evidence cannot authorize a retry');
    c.raw('check-change');
    c.reply(c.action('read'), { reference, receipt });
    query = c.action('outcome');
    c.reply(query, {
      original: {
        reference: { ...reference, operation: '00000000-0000-0000-0000-000000000009' },
        receipt,
      },
      published: { kind: 'applied', reference: 'installed:review' },
    });
    assert.deepEqual(
      c.state.current.reference,
      reference,
      'a replacement cannot settle the original operation',
    );
    c.raw('check-change');
    c.reply(c.action('read'), { reference, receipt });
    query = c.action('outcome');
    c.raw('close').update({ open: false });
    c.reply(query, {
      original: { reference, receipt },
      published: { kind: 'applied', reference: 'installed:review' },
    });
    assert.equal(c.state.current, null);
    assert.equal(c.state.cleanupUnconfirmed, true);
    assert.equal(
      c.out.actions.length,
      0,
      'closed library consumes the result without replay or background refresh',
    );
  });
  test(`${target} unavailable original receipts and denied domain reads retain the original skill operation`, async () => {
    const c = (await client(target)).open().preview().raw('confirm-change');
    c.operation(c.action('apply'), { kind: 'unavailable', diagnostic: 'Host disconnected' });
    for (const receipt of [
      { kind: 'not_recorded' },
      { kind: 'unavailable', diagnostic: 'unreachable' },
    ]) {
      c.raw('check-change');
      c.reply(c.action('read'), { reference, receipt });
      assert.deepEqual(c.state.current.reference, reference);
      assert.equal(c.out.actions.length, 0);
    }
    c.raw('check-change');
    c.reply(c.action('read'), {
      reference,
      receipt: {
        kind: 'recorded',
        outcome: { kind: 'unknown', diagnostic: 'reply lost' },
        cleanup: { kind: 'confirmed' },
      },
    });
    const query = c.action('outcome');
    c.event({
      kind: 'action',
      key: query.key,
      event: { status: 'failed', error: 'recovery denied' },
    });
    assert.deepEqual(c.state.current.reference, reference);
    assert.equal(c.state.notice, 'skillsUnknown');
    assert.equal(c.out.actions.length, 0);
  });
  test(`${target} rejected skill results retain cleanup uncertainty separately from the rejection`, async () => {
    const c = (await client(target)).open().preview().raw('confirm-change');
    c.operation(c.action('apply'), {
      kind: 'recorded',
      outcome: { kind: 'returned', value: { kind: 'changed' } },
      cleanup: { kind: 'failed', diagnostic: 'resource settlement failed' },
    });
    assert.equal(c.state.current, null);
    assert.equal(c.state.notice, 'skillsChanged');
    assert.equal(c.state.cleanupUnconfirmed, true);
    assert.ok(
      JSON.stringify(c.out).includes(
        'The operation result is known, but resource cleanup is not confirmed.',
      ),
    );
  });
}
