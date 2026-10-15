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
import { readFileSync } from 'node:fs';
import Ajv from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';
const { activate } = await import('../dist/packages/appearance/tui.js');
const theme = activate({ scheme: 'dark' }).Appearance.value;
const ajv = new Ajv({
  strict: false,
  formats: { uint64: true, uint32: true, uint16: true, uint: true },
});
addFormats(ajv);
const manifest = JSON.parse(
  readFileSync(new URL('../dist/packages/composer/maka.extension.json', import.meta.url)),
);
const protocol = manifest.application.entries.tui.exports.Composer.contract.interface;
const validProps = ajv.compile(protocol.props);
const validEvents = Object.fromEntries(
  Object.entries(protocol.events).map(([name, schema]) => [name, ajv.compile(schema)]),
);
const dataSource = {
  stateDomain: 'a'.repeat(64),
  deployment: '00000000-0000-0000-0000-000000000001',
  namespace: ['00000000-0000-0000-0000-000000000002'],
};
const backend = {
  kind: 'model',
  model: { connectionId: 'original', revision: 1, modelId: 'first' },
  thinkingLevel: null,
};
const reference = {
  source: { stateDomain: dataSource.stateDomain, deployment: dataSource.deployment, revision: 1 },
  namespace: dataSource.namespace,
  export: 'agent',
  method: 'send',
  operation: '00000000-0000-0000-0000-000000000003',
  fingerprint: `sha256-${'b'.repeat(64)}`,
  contract: {
    name: 'maka.agent.Agent',
    version: 1,
    transport: 'remote',
    digest: `sha256-${'c'.repeat(64)}`,
  },
  recovery: 'journal_v1',
};
const caller = {
  cursor: 42,
  caller: { ...reference.source, deployment: '00000000-0000-0000-0000-000000000004' },
  consumer: ['00000000-0000-0000-0000-000000000005'],
  reference,
};
const scope = JSON.stringify([
  caller.caller.stateDomain,
  caller.caller.deployment,
  caller.consumer,
]);
const returned = (
  value = {
    kind: 'started',
    sessionId: 'session',
    invocationId: 'run',
    messageId: 'canonical-message',
  },
) => ({
  reference,
  receipt: {
    kind: 'recorded',
    outcome: { kind: 'returned', value },
    cleanup: { kind: 'confirmed' },
  },
});
const observed = (text = 'hello', revision = 1, owner = 'editor') => ({
  revision,
  snapshot: {
    text: { owner, revision, bytes: Buffer.byteLength(text) },
    selection: { start: text.length, end: text.length },
    reversed: false,
    marked: null,
  },
  receipt: null,
});
function slot(node, key = 'draft') {
  if (!node || typeof node !== 'object') return;
  if (node.key === key && node.binding === 'editor') return node;
  return Object.values(node)
    .flatMap((value) => (Array.isArray(value) ? value : [value]))
    .map((child) => slot(child, key))
    .find(Boolean);
}
async function client(target) {
  const { component } = await import(`../dist/packages/composer/${target}.js`);
  const props = {
    backend: structuredClone(backend),
    context: 'session',
    dataSource,
    attachments: [],
    filePaste: null,
    fileCommand: null,
    autoFocus: true,
    theme,
    locale: 'en',
    busy: false,
    running: false,
    enabled: true,
  };
  assert.equal(validProps(props), true, JSON.stringify(validProps.errors));
  let out;
  const accept = (result) => {
    out = structuredClone(result);
    for (const event of out.events ?? [])
      assert.equal(
        validEvents[event.name](event.value),
        true,
        JSON.stringify(validEvents[event.name].errors),
      );
    return out;
  };
  accept(component.mount(props));
  return {
    props,
    get out() {
      return out;
    },
    get state() {
      return out.frame.state;
    },
    editor(key) {
      return slot(out.frame.view, key).props;
    },
    update() {
      return accept(component.update(out.frame.state, props));
    },
    event(input) {
      return accept(component.event(out.frame.state, input, props));
    },
    emit(name, event, child = 'draft') {
      return this.event({ kind: 'component', child, name, key: name, event });
    },
    raw(key) {
      return this.event({ kind: 'raw', key, event: {} });
    },
    reply(event, key = out.actions[0].key) {
      return this.event({ kind: 'action', key, event });
    },
    ready(text = 'hello') {
      this.emit('checkpoint', {
        owner: 'editor',
        state: { kind: 'ready', initialized: true, saved: true },
      });
      this.emit('changed', observed(text));
    },
    send(text = 'hello') {
      this.emit('submitted', observed(text));
      return this.reply({
        status: 'succeeded',
        value: { metadata: observed(text).snapshot.text, text },
      });
    },
    clear(status = 'applied') {
      const command = this.editor().command;
      const value = observed(status === 'applied' ? '' : 'new text', 2);
      value.receipt = {
        owner: command.owner,
        sequence: command.sequence,
        revision: 2,
        outcome: status === 'applied' ? { status } : { status, reason: 'revision' },
      };
      return this.emit('changed', value);
    },
  };
}
for (const target of ['tui', 'desktop']) {
  test(`${target} sends from its native snapshot without a parent callback and freezes the original destination`, async () => {
    const c = await client(target);
    c.ready();
    for (const saved of [false, true, false, true]) {
      c.emit('checkpoint', { owner: 'editor', state: { kind: 'ready', initialized: true, saved } });
      assert.deepEqual(c.out.events, []);
    }
    c.emit('changed', observed());
    assert.deepEqual(c.out.events, []);
    c.emit('submitted', observed());
    const read = c.out.actions[0];
    assert.equal(read.binding, 'reader');
    assert.equal(read.resources.text.child, 'draft');
    assert.deepEqual(c.out.events, []);
    c.emit('submitted', observed());
    assert.deepEqual(c.out.actions, [], 'duplicate Enter cannot start another capture');
    c.props.context = 'another-session';
    c.props.backend = { kind: 'executor', selection: { executor: 'new', settings: {} } };
    c.update();
    c.reply(
      { status: 'succeeded', value: { metadata: observed().snapshot.text, text: 'hello' } },
      read.key,
    );
    const send = c.out.actions[0];
    assert.equal(send.binding, 'agent');
    assert.equal(send.method, 'send');
    assert.deepEqual(send.input, {
      sessionId: 'session',
      message: 'hello',
      attachments: [],
      model: backend.model,
      thinkingLevel: null,
    });
    assert.equal(c.out.events[0].name, 'submission');
    assert.deepEqual(c.out.events[0].value, {
      context: 'session',
      dataSource,
      attachments: [],
      key: send.key,
      text: 'hello',
      status: 'pending',
      messageId: null,
    });
    assert.deepEqual(send.recovery, {
      format: 'maka.agent.draft-send@2',
      sessionId: 'session',
      editor: { owner: 'editor', revision: 1 },
      attachments: [],
    });
    c.reply({ status: 'operation', reply: returned(), caller });
    assert.equal(c.out.events[0].value.status, 'accepted');
    assert.equal(c.out.events[0].value.messageId, 'canonical-message');
    assert.deepEqual(
      c.out.events.filter((event) => event.name === 'sent'),
      [{ name: 'sent', value: { context: 'session', dataSource, attachments: [] } }],
    );
    assert.equal(c.editor().command.owner, 'editor');
    assert.equal(
      c.editor().persistence.metadata.historyAfter,
      0,
      'admission alone cannot advance a draft past an unapplied clear',
    );
    c.clear();
    assert.deepEqual(c.out.events, []);
    assert.equal(c.editor().persistence.metadata.historyAfter, 42);
    assert.equal(c.editor().persistence.metadata.historyScope, scope);
  });
  test(`${target} retains text on native conflicts, rejected capture and uncertain original receipts`, async () => {
    for (const value of [
      { metadata: { ...observed().snapshot.text, owner: 'foreign' }, text: 'hello' },
      { metadata: observed().snapshot.text, text: 'longer' },
    ]) {
      const c = await client(target);
      c.ready();
      c.emit('submitted', observed());
      c.reply({ status: 'succeeded', value });
      assert.deepEqual(c.out.actions, []);
      assert.equal(c.state.entries[0].notice, 'draftKept');
      assert.equal(c.editor().command, null);
    }
    const c = await client(target);
    c.ready();
    c.send();
    c.reply({
      status: 'operation',
      caller,
      reply: { reference, receipt: { kind: 'unavailable', diagnostic: 'unknown' } },
    });
    assert.equal(c.editor().command, null);
    c.emit('submitted', observed());
    assert.deepEqual(c.out.actions, []);
    c.raw('draft-retry');
    assert.equal(c.out.actions[0].binding, 'receipts');
    assert.deepEqual(c.out.actions[0].input, reference);
    c.reply({ status: 'succeeded', value: returned() });
    assert.equal(c.out.events.filter((e) => e.name === 'sent').length, 1);
    c.clear('conflict');
    assert.equal(c.editor().persistence.metadata.historyAfter, 0);
    assert.equal(c.state.entries[0].notice, 'draftKept');
    assert.deepEqual(c.out.actions, [], 'recovery and clearing never replay a mutation');
  });
  test(`${target} sends a 64 KiB Unicode snapshot intact and keeps rejected text editable`, async () => {
    const text = '🙂'.repeat(16384),
      c = await client(target);
    c.ready(text);
    c.emit('submitted', observed(text));
    assert.deepEqual(c.out.actions[0].input, { start: 0, end: 65536 });
    c.reply({ status: 'succeeded', value: { metadata: observed(text).snapshot.text, text } });
    assert.equal(c.out.actions[0].input.message, text);
    c.reply({
      status: 'operation',
      caller,
      reply: returned({ kind: 'rejected', sessionId: 'session', reason: 'skills_unavailable' }),
    });
    assert.equal(c.editor().command, null);
    assert.equal(c.editor().persistence.metadata.historyAfter, 0);
    assert.equal(c.state.entries[0].notice, 'skillInputUnavailable');
    assert.equal(c.out.events[0].value.status, 'removed');
    c.emit('submitted', observed(text));
    assert.equal(c.out.actions[0].binding, 'reader');
    c.reply({ status: 'failed', reason: 'capture retired' });
    assert.equal(c.state.entries[0].notice, 'draftKept');
    const ime = observed(text);
    ime.snapshot.marked = { start: 0, end: 2 };
    c.emit('submitted', ime);
    assert.deepEqual(c.out.actions, []);
  });
  test(`${target} restores its own checked prefix without consulting the parent journal`, async () => {
    const c = await client(target);
    c.emit('checkpoint', {
      owner: 'new-editor',
      state: {
        kind: 'restore',
        version: 3,
        snapshot: observed().snapshot,
        metadata: { attachments: [], historyAfter: 17, historyScope: scope },
      },
    });
    assert.deepEqual(c.out.actions[0].input, { after: 0, limit: 1 });
    c.reply({ status: 'succeeded', value: [{ ...caller, cursor: 1, recovery: null }] });
    assert.deepEqual(c.out.actions[0].input, { after: 17, limit: 1 });
    c.reply({ status: 'succeeded', value: [] });
    assert.deepEqual(c.editor().persistence.restore, {
      owner: 'new-editor',
      version: 3,
      clearText: false,
    });
    c.emit('checkpoint', {
      owner: 'new-editor',
      state: { kind: 'ready', initialized: false, saved: false },
    });
    assert.equal(c.editor().persistence.metadata.historyAfter, 17);
    assert.equal(c.editor().persistence.metadata.historyScope, scope);
    assert.deepEqual(c.out.events, [], 'native application precedes its first durable save');
    c.emit('checkpoint', {
      owner: 'new-editor',
      state: { kind: 'ready', initialized: true, saved: true },
    });
    assert.equal(c.editor().persistence.metadata.historyAfter, 17);
    assert.equal(c.editor().persistence.metadata.historyScope, scope);
    assert.equal(c.out.events[0].name, 'readiness');
  });
  test(`${target} resolves an interrupted original send before clearing only its consumed draft and attachments`, async () => {
    const c = await client(target);
    const file = {
      sessionId: 'session',
      id: 'attachment',
      name: 'one.txt',
      mimeType: 'text/plain',
      bytes: 3,
    };
    c.emit('checkpoint', {
      owner: 'new-editor',
      state: {
        kind: 'restore',
        version: 3,
        snapshot: observed().snapshot,
        metadata: { attachments: [file], historyAfter: 17, historyScope: 'another-caller' },
      },
    });
    const original = {
      ...caller,
      cursor: 18,
      recovery: {
        format: 'maka.agent.draft-send@2',
        sessionId: 'session',
        editor: { owner: 'editor', revision: 1 },
        attachments: ['attachment'],
      },
    };
    c.reply({ status: 'succeeded', value: [original] });
    assert.equal(c.out.actions[0].binding, 'receipts');
    c.reply({
      status: 'succeeded',
      value: { reference, receipt: { kind: 'unavailable', diagnostic: 'unknown' } },
    });
    assert.equal(c.editor().persistence.restore, null);
    assert.deepEqual(c.out.actions, []);
    c.raw('draft-retry');
    assert.equal(c.out.actions[0].binding, 'receipts');
    c.reply({ status: 'succeeded', value: returned() });
    assert.deepEqual(c.out.actions[0].input, { after: 18, limit: 1 });
    c.reply({ status: 'succeeded', value: [] });
    assert.deepEqual(c.editor().persistence.restore, {
      owner: 'new-editor',
      version: 3,
      clearText: true,
    });
    assert.deepEqual(c.out.events, [], 'consumed attachments are not restored');
    c.emit('checkpoint', {
      owner: 'new-editor',
      state: { kind: 'ready', initialized: true, saved: true },
    });
    assert.equal(c.editor().persistence.metadata.historyAfter, 18);
  });
  test(`${target} preserves a newer unsent draft, restores unused attachments, and rejects missing recovery metadata`, async () => {
    const c = await client(target);
    const file = {
      sessionId: 'session',
      id: 'attachment',
      name: 'one.txt',
      mimeType: 'text/plain',
      bytes: 3,
    };
    c.emit('checkpoint', {
      owner: 'new-editor',
      state: {
        kind: 'restore',
        version: 3,
        snapshot: observed('new text', 2).snapshot,
        metadata: { attachments: [file], historyAfter: 0, historyScope: null },
      },
    });
    c.reply({
      status: 'succeeded',
      value: [
        {
          ...caller,
          cursor: 18,
          recovery: {
            format: 'maka.agent.draft-send@2',
            sessionId: 'session',
            editor: { owner: 'editor', revision: 1 },
            attachments: [],
          },
        },
      ],
    });
    assert.equal(
      c.out.actions[0].binding,
      'history',
      'older accepted text does not clear a newer draft',
    );
    c.reply({ status: 'succeeded', value: [] });
    assert.equal(c.editor().persistence.restore.clearText, false);
    assert.deepEqual(c.out.events, [
      { name: 'restored', value: { files: [{ ...file, dataSource }] } },
    ]);
    const invalid = await client(target);
    invalid.emit('checkpoint', {
      owner: 'new-editor',
      state: {
        kind: 'restore',
        version: 3,
        snapshot: observed().snapshot,
        metadata: { attachments: [] },
      },
    });
    assert.deepEqual(invalid.out.actions, []);
    assert.equal(invalid.editor().persistence.restore, null);
    assert.equal(invalid.state.records[0].work.status, 'failed');
  });
}

for (const target of ['tui', 'desktop']) {
  test(`${target} pending submissions settle only on original admission`, async () => {
    const c = await client(target);
    c.ready();
    c.send();
    const pending = c.out.events.find((event) => event.name === 'submission').value;
    assert.equal(pending.status, 'pending');
    c.reply({ status: 'failed', reason: 'offline' });
    assert.equal(c.out.events[0].value.status, 'removed');
    assert.equal(c.editor().command, null);
    c.send();
    c.reply({
      status: 'operation',
      caller,
      reply: { reference, receipt: { kind: 'unavailable', diagnostic: 'unknown' } },
    });
    assert.equal(c.out.events[0].value.status, 'unknown');
    assert.equal(
      c.out.events.some((event) => event.name === 'sent'),
      false,
    );
    assert.equal(c.editor().command, null);
    c.raw('draft-retry');
    c.reply({
      status: 'succeeded',
      value: returned({
        kind: 'queued',
        sessionId: 'session',
        messageId: 'queued-message',
        revision: 1,
      }),
    });
    assert.equal(c.out.events[0].value.status, 'removed');
    assert.equal(
      c.out.events.some((event) => event.name === 'sent'),
      true,
    );
  });
}
