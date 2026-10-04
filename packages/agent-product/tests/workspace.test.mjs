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
import { execFileSync } from 'node:child_process';
import { readFileSync, mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import '../scripts/build.mjs';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const product = await import(pathToFileURL(resolve(root, 'dist/packages/workspace/desktop.js')));
product.activate({ locale: 'en' });
const appearance = await import(
  pathToFileURL(resolve(root, 'dist/packages/appearance/desktop.js'))
);
const values = { appearance: [appearance.activate({ scheme: 'light' }).Appearance.value] };
const sdk = await import(pathToFileURL(resolve(root, 'dist/authoring/plugin-sdk/src/index.mjs')));
const uuid = (n) => `00000000-0000-0000-0000-${n.toString().padStart(12, '0')}`;
const reference = {
  source: { stateDomain: uuid(1), deployment: uuid(2), revision: 1 },
  namespace: [uuid(3)],
  export: 'agent',
  method: 'send',
  operation: uuid(4),
  fingerprint: 'sha256-' + 'a'.repeat(64),
  contract: sdk.contracts.agent,
  recovery: 'journal_v1',
};
const session = {
  id: 'session',
  title: 'A thoughtful conversation',
  revision: 1,
  updatedAt: 1,
  archived: false,
};
const draft = {
  revision: 4,
  snapshot: {
    text: { owner: 'original-editor', revision: 4, bytes: 5 },
    selection: { start: 5, end: 5 },
    reversed: false,
    marked: null,
  },
  receipt: null,
};
const returned = (value) => ({
  status: 'operation',
  reply: {
    reference,
    receipt: {
      kind: 'recorded',
      outcome: { kind: 'returned', value },
      cleanup: { kind: 'confirmed' },
    },
  },
});
class Client {
  constructor() {
    this.actions = new Map();
    this.output = product.component.mount({}, values);
    this.save();
  }

  save() {
    for (const action of this.output.actions) this.actions.set(action.key, action);
  }
  input(input) {
    this.output = product.component.event(
      structuredClone(this.output.frame.state),
      input,
      {},
      values,
    );
    this.save();
    return this.output;
  }
  find(method) {
    const action = [...this.actions.values()].find((action) => action.method === method);
    assert.ok(action, `expected ${method}`);
    return action;
  }
  complete(action, value) {
    this.actions.delete(action.key);
    return this.input({ kind: 'action', key: action.key, event: value });
  }
  query(method, value) {
    return this.complete(this.find(method), { status: 'succeeded', value });
  }
  component(name, event = {}) {
    return this.input({
      kind: 'component',
      key: name,
      child: ['send', 'draft', 'stop'].includes(name)
        ? 'composer'
        : name === 'history'
          ? 'timeline'
          : 'navigation',
      name: name === 'draft' ? 'changed' : ['history', 'sessions'].includes(name) ? 'more' : name,
      event,
    });
  }
  raw(key) {
    return this.input({ kind: 'raw', key, event: {} });
  }
  get state() {
    return this.output.frame.state;
  }
  start({ through = 0, parts = [], run = null, chunks = [] } = {}) {
    this.query('page', []);
    this.query('list', { revision: 'catalog', sessions: [session], nextCursor: null });
    this.query('observe', { session, through, run, parts });
    this.query('history', { kind: 'ready', through, chunks, next: null });
    this.input({
      kind: 'component',
      key: 'model',
      child: 'models',
      name: 'selected',
      event: {
        selection: { connectionId: 'fixture', revision: 2, modelId: 'fixture-model' },
        connectionName: 'Fixture',
        label: 'Fixture model',
        isDefault: false,
      },
    });
    return this;
  }
  page(events, through) {
    return this.input({
      kind: 'action',
      key: this.state.watch.key,
      event: { status: 'updated', value: { events, through, nextAfter: through } },
    });
  }
}
const fact = (sequence, fact) => ({ sequence, invocationId: 'run', fact });
test('a durable admission clears only the captured owner; unknown never replays the send', () => {
  const client = new Client().start();
  const originalWatch = structuredClone(client.output.subscriptions);
  client.component('send', draft);
  const read = client.find('read');
  assert.deepEqual(read.resources.text, {
    child: 'composer',
    method: 'snapshot',
    input: { kind: 'at_revision', owner: 'original-editor', revision: 4 },
  });
  client.query('read', {
    text: 'Hello',
    metadata: { owner: 'replacement-editor', revision: 4, bytes: 5 },
  });
  assert.equal(client.state.notice, 'draftKept');
  assert.equal(client.state.command, null);
  assert.equal(
    [...client.actions.values()].some((action) => action.method === 'send'),
    false,
  );
  client.component('send', draft);
  client.query('read', {
    text: 'Hello',
    metadata: { owner: 'original-editor', revision: 4, bytes: 5 },
  });
  const send = client.find('send');
  assert.equal(client.state.command, null);
  client.complete(send, {
    status: 'operation',
    reply: {
      reference,
      receipt: {
        kind: 'recorded',
        outcome: { kind: 'unknown', diagnostic: 'lost' },
        cleanup: { kind: 'confirmed' },
      },
    },
  });
  assert.equal(client.state.command, null);
  client.component('send', draft);
  assert.equal(
    [...client.actions.values()].some((action) => action.method === 'send'),
    false,
  );
  client.raw('check');
  assert.deepEqual(client.find('read').input, reference);
  client.query('read', returned({ sessionId: 'session', invocationId: 'run' }).reply);
  assert.deepEqual(client.state.command, {
    owner: 'original-editor',
    sequence: 1,
    basedOn: 4,
    edit: { kind: 'reset', text: '' },
  });
  assert.ok(client.find('observe'));
  assert.deepEqual(client.output.subscriptions, originalWatch);
  client.component('draft', {
    ...draft,
    revision: 5,
    receipt: {
      owner: 'original-editor',
      sequence: 1,
      revision: 5,
      outcome: { status: 'conflict', reason: 'revision' },
    },
  });
  assert.equal(client.state.command, null);
  assert.equal(client.state.notice, 'draftKept');
});
test('a new mount restores original references and never clears its new draft', () => {
  const client = new Client();
  client.query('page', [{ cursor: 7, caller: reference.source, consumer: [uuid(9)], reference }]);
  const recovery = client.find('read');
  assert.deepEqual(recovery.input, reference);
  client.complete(recovery, { status: 'failed', reason: 'original source unavailable' });
  assert.deepEqual(client.state.uncertain.reference, reference);
  assert.equal(client.state.command, null);
  client.raw('check');
  client.query('read', returned({ sessionId: 'session', invocationId: 'old-run' }).reply);
  assert.equal(client.state.command, null);
  assert.equal(
    [...client.actions.values()].some((action) => action.method === 'send'),
    false,
  );
  assert.deepEqual(client.find('page').input, { after: 7, limit: 1 });
});
test('bootstrap replays the original active prefix and does not render thinking as an answer', () => {
  const parts = [
    { startSequence: 3, messageId: 'answer', stepId: 'step', partId: 'same', textKind: 'text' },
    {
      startSequence: 5,
      messageId: 'private-thinking',
      stepId: 'step',
      partId: 'reason',
      textKind: 'thinking',
    },
  ];
  const client = new Client().start({
    through: 8,
    parts,
    run: { invocationId: 'run', state: 'running' },
  });
  assert.equal(client.state.watch.input.after, 2);
  client.page(
    [
      fact(3, { kind: 'part_started', ...parts[0] }),
      fact(4, { kind: 'delta', stepId: 'step', partId: 'same', text: 'prefix ' }),
      fact(5, { kind: 'part_started', ...parts[1] }),
      fact(6, { kind: 'delta', stepId: 'step', partId: 'reason', text: 'private reasoning' }),
      fact(9, { kind: 'delta', stepId: 'step', partId: 'same', text: 'suffix' }),
    ],
    9,
  );
  assert.equal(client.state.liveMessages[0].text, 'prefix suffix');
  assert.equal(client.state.liveMessages.length, 1);
  client.page([fact(9, { kind: 'delta', stepId: 'step', partId: 'same', text: 'duplicate' })], 9);
  assert.equal(client.state.liveMessages[0].text, 'prefix suffix');
  client.page(
    [
      fact(10, { kind: 'part_finished', stepId: 'step', partId: 'same' }),
      fact(11, { kind: 'step_ended', stepId: 'step' }),
      fact(12, { kind: 'ended', failed: false }),
    ],
    12,
  );
  const history = client.find('history');
  assert.equal(history.input.position.sequence, 3);
  client.query('history', {
    kind: 'ready',
    through: 12,
    chunks: [
      {
        messageId: 'answer',
        role: 'assistant',
        sequence: 3,
        offset: 0,
        totalBytes: 13,
        text: 'prefix suffix',
      },
    ],
    next: null,
  });
  assert.equal(client.state.messages[0].text, 'prefix suffix');
  assert.equal(client.state.liveMessages.length, 0);
  assert.equal(client.state.parts.length, 0);
});
test('opening a run refreshes the canonical user message without restarting its watch', () => {
  const client = new Client().start();
  const demand = structuredClone(client.output.subscriptions);
  client.page(
    [
      fact(1, { kind: 'opened' }),
      fact(3, {
        kind: 'part_started',
        messageId: 'answer',
        stepId: 'step',
        partId: 'same',
        textKind: 'text',
      }),
      fact(4, { kind: 'delta', stepId: 'step', partId: 'same', text: 'First response' }),
    ],
    4,
  );
  client.query('history', {
    kind: 'ready',
    through: 4,
    chunks: [
      { messageId: 'user', role: 'user', sequence: 1, offset: 0, totalBytes: 5, text: 'Hello' },
    ],
    next: null,
  });
  assert.equal(client.state.messages[0].text, 'Hello');
  assert.equal(client.state.liveMessages[0].text, 'First response');
  assert.deepEqual(client.output.subscriptions, demand);
});
test('live display is bounded and marks its truncated Unicode excerpt', () => {
  const client = new Client().start();
  client.page(
    [
      fact(1, { kind: 'opened' }),
      fact(2, {
        kind: 'part_started',
        messageId: 'answer',
        stepId: 'step',
        partId: 'part',
        textKind: 'text',
      }),
    ],
    2,
  );
  for (let sequence = 3; sequence < 50; sequence++)
    client.page(
      [
        fact(sequence, {
          kind: 'delta',
          stepId: 'step',
          partId: 'part',
          text: '中😀'.repeat(2000),
        }),
      ],
      sequence,
    );
  const message = client.state.liveMessages[0];
  assert.ok(message.text.length <= 16000);
  assert.equal(message.continued, true);
  assert.ok(message.text.isWellFormed());
  assert.ok(JSON.stringify(client.output).length < 200000);
});
test('both product compositions use actual immutable package refs and check in the real CLI', {
  skip: !process.env.MAKA_PRODUCT_CLI_CHECK,
}, async () => {
  const binary = resolve(root, '../../target/debug/maka');
  const { packageNames } = await import(
    pathToFileURL(resolve(root, 'dist/authoring/agent-product/src/composition.mjs'))
  );
  const keys = Object.fromEntries(packageNames.map((name, index) => [name, uuid(index + 1)]));
  const directory = mkdtempSync(resolve(tmpdir(), 'maka-product-composition-'));
  try {
    for (const presentation of ['desktop', 'tui']) {
      const configuration = {
        presentation,
        locale: 'zh-Hant-HK',
        scheme: 'light',
        keys,
        agent: {
          maxActiveRuns: 1,
          maxSteps: 1,
        },
        hostRoot: '/private/product-fixture',
        hostDeployment: uuid(80),
      };
      const configurationFile = resolve(directory, 'installation.json');
      const output = resolve(directory, presentation);
      writeFileSync(configurationFile, JSON.stringify(configuration));
      execFileSync(
        process.execPath,
        [
          resolve(root, 'scripts/compose.mjs'),
          '--configuration',
          configurationFile,
          '--output',
          output,
          '--maka',
          binary,
        ],
        { encoding: 'utf8', maxBuffer: 1024 * 1024 },
      );
      const ui = JSON.parse(readFileSync(resolve(output, 'ui.json'), 'utf8'));
      const host = JSON.parse(readFileSync(resolve(output, 'host.json'), 'utf8'));
      const connections = JSON.parse(readFileSync(resolve(output, 'connections.json'), 'utf8'));
      assert.equal(ui.environment.presentation, presentation);
      assert.equal(ui.instances.workspace.key, keys.workspace);
      assert.equal(host.instances.agent.key, keys.agent);
      assert.deepEqual(connections.hosts[0].consumer, [keys.gateway]);
      assert.equal(connections.hosts[0].deployment, configuration.hostDeployment);
      assert.deepEqual(
        connections.hosts[0].imports.settings.methods,
        Object.keys(sdk.contracts.settings.interface.methods),
      );
      assert.equal(connections.hosts[0].imports.settings.recovery, true);
      const secret = JSON.parse(
        readFileSync(resolve(root, 'dist/packages/credential/maka.extension.json')),
      );
      assert.equal('reader' in secret.application.entries[presentation].imports, false);
      assert.deepEqual(
        secret.application.entries[presentation].exports.Credential.contract.interface.captures,
        {},
      );
      assert.deepEqual(Object.keys(ui.instances.credential.bindings).sort(), [
        'history',
        'receipts',
        'secret',
        'settings',
      ]);
    }
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test('reconnect rebuilds its display dedupe with the same canonical page', () => {
  const chunk = {
    messageId: 'user',
    role: 'user',
    sequence: 256,
    offset: 0,
    totalBytes: 5,
    text: 'Hello',
  };
  const client = new Client().start({ through: 5, chunks: [chunk] });
  client.input({
    kind: 'action',
    key: client.state.watch.key,
    event: { status: 'failed', reason: 'connection lost' },
  });
  client.raw('reload');
  client.query('observe', { session, through: 5, run: null, parts: [] });
  client.query('history', { kind: 'ready', through: 5, chunks: [chunk], next: null });
  assert.equal(client.state.messages[0].text, 'Hello');
  assert.equal(client.state.messages.length, 1);
});
test('a delayed observation cannot resurrect a run ended by a newer delivery', () => {
  const client = new Client().start({ through: 5, run: { invocationId: 'run', state: 'running' } });
  client.component('stop');
  const stop = client.find('stop');
  client.complete(stop, returned({ requested: true }));
  const observe = client.find('observe');
  client.page([fact(10, { kind: 'ended', failed: false })], 10);
  assert.equal(client.state.run, null);
  client.complete(observe, {
    status: 'succeeded',
    value: { session, through: 9, run: { invocationId: 'run', state: 'running' }, parts: [] },
  });
  assert.equal(client.state.run, null);
  assert.equal(client.state.runThrough, 10);
});

test('reconnect isolates late reads before building the new active-prefix watch', () => {
  const client = new Client().start({
    through: 20,
    run: { invocationId: 'run', state: 'running' },
  });
  client.component('stop');
  client.complete(client.find('stop'), returned({ kind: 'requested' }));
  const lateObserve = client.find('observe');
  client.input({
    kind: 'action',
    key: client.state.watch.key,
    event: { status: 'failed', error: 'lost' },
  });
  const epoch = client.state.epoch;
  client.raw('reload');
  assert.ok(client.state.epoch > epoch);
  const currentObserve = [...client.actions.values()]
    .filter((action) => action.method === 'observe')
    .at(-1);
  const part = {
    startSequence: 30,
    messageId: 'answer',
    stepId: 'step',
    partId: 'part',
    textKind: 'text',
  };
  client.complete(currentObserve, {
    status: 'succeeded',
    value: {
      session,
      through: 40,
      parts: [part],
      run: { invocationId: 'run', state: 'running' },
    },
  });
  const currentHistory = client.find('history');
  client.complete(lateObserve, {
    status: 'succeeded',
    value: { session, through: 20, parts: [], run: null },
  });
  assert.equal(client.state.through, 40);
  assert.equal(client.state.parts[0].startSequence, 30);
  client.complete(currentHistory, {
    status: 'succeeded',
    value: { kind: 'ready', through: 40, chunks: [], next: null },
  });
  assert.equal(client.state.watch.input.after, 29);
  client.page(
    [
      fact(30, { kind: 'part_started', ...part }),
      fact(41, { kind: 'delta', stepId: 'step', partId: 'part', text: 'Preserved reply' }),
    ],
    41,
  );
  assert.equal(client.state.liveMessages[0].text, 'Preserved reply');
});

const modelRoot = new URL('../', import.meta.url);
const modelAppearance = await import(new URL('dist/packages/appearance/tui.js', modelRoot));
const modelProps = {
  theme: modelAppearance.activate({ scheme: 'light' }).Appearance.value,
  locale: 'en',
  selection: null,
  busy: true,
  catalogEpoch: 0,
};
const modelGuest = await import(new URL('dist/packages/models/tui.js', modelRoot));
modelGuest.activate();
const modelChoices = Array.from({ length: 16 }, (_, at) => ({
  selection: { connectionId: 'connection', revision: 5, modelId: `model-${at}` },
  connectionName: 'Account',
  label: `Model ${at}`,
  isDefault: at === 0,
}));
const modelAction = (out, key, reply, p = modelProps) =>
  modelGuest.component.event(
    structuredClone(out.frame.state),
    { kind: 'action', key, event: reply },
    p,
    {},
  );
const modelRaw = (out, key, p = modelProps) =>
  modelGuest.component.event(
    structuredClone(out.frame.state),
    { kind: 'raw', key, event: {} },
    p,
    {},
  );
function modelButtons(view, result = []) {
  if (view.kind === 'button') result.push(view.key);
  for (const child of view.children ?? []) modelButtons(child, result);
  return result;
}
test('TUI pages expose every original choice, keep controls visible and retain late default until enabled', () => {
  let out = modelGuest.component.mount(modelProps, {});
  const request = out.actions[0];
  assert.equal(request.method, 'models');
  out = modelAction(out, request.key, {
    status: 'succeeded',
    value: { kind: 'ready', revision: 5, choices: modelChoices, next: null },
  });
  assert.deepEqual(out.events, []);
  out = modelGuest.component.update(out.frame.state, { ...modelProps, busy: false }, {});
  assert.equal(out.events[0].value.selection.modelId, 'model-0');
  const enabled = { ...modelProps, busy: false, selection: modelChoices[0] };
  out = modelRaw(out, 'toggle', enabled);
  assert.deepEqual(
    modelButtons(out.frame.view).filter((key) => key.startsWith('choose-')),
    ['choose-0', 'choose-1', 'choose-2'],
  );
  for (let i = 0; i < 5; i++) out = modelRaw(out, 'more', enabled);
  assert.deepEqual(
    modelButtons(out.frame.view).filter((key) => key.startsWith('choose-')),
    ['choose-15'],
  );
  assert.ok(modelButtons(out.frame.view).includes('close'));
  out = modelRaw(out, 'choose-15', enabled);
  assert.equal(out.events[0].value.selection.modelId, 'model-15');
  assert.equal(out.frame.state.open, false);
  out = modelGuest.component.update(
    out.frame.state,
    { ...enabled, selection: modelChoices[15] },
    {},
  );
  assert.equal(out.events.length, 0);
});
test('late query is ignored and catalog changes restart only the observation', () => {
  const enabled = { ...modelProps, busy: false, selection: modelChoices[1] };
  let out = modelGuest.component.mount(enabled, {});
  const original = out.actions[0];
  out = modelAction(
    out,
    'retired-query',
    {
      status: 'succeeded',
      value: { kind: 'ready', revision: 5, choices: modelChoices, next: null },
    },
    enabled,
  );
  assert.equal(out.frame.state.pending, original.key);
  assert.equal(out.frame.state.choices.length, 0);
  out = modelAction(
    out,
    original.key,
    { status: 'succeeded', value: { kind: 'changed', revision: 6 } },
    enabled,
  );
  assert.equal(out.actions.length, 1);
  assert.deepEqual(out.actions[0].input, { kind: 'start' });
  assert.equal(out.events.length, 0);
  assert.equal(out.actions[0].method, 'models');
});

const connectionTarget = {
  connectionId: 'connection-A',
  revision: 12,
  slug: 'connection-a',
  provider: {
    packageId: 'maka.providers',
    entryId: 'maka.providers',
    scope: 'profile',
    name: 'openai',
  },
  configuration: { baseUrl: 'https://example.invalid/v1' },
};
const credentialStatus = {
  locator: { scope: 'connection', connectionId: connectionTarget.connectionId, kind: 'provider' },
  configured: true,
  credentialId: 'saved-key',
  revision: 8,
  updatedAt: 1,
};
const credentialRef = {
  ...reference,
  export: 'Settings',
  method: 'saveCredential',
  contract: sdk.contracts.settings,
};
class FormClient {
  constructor(component, props) {
    this.component = component;
    this.props = props;
    this.actions = new Map();
    this.save(component.mount(props, {}));
  }
  save(out) {
    this.output = out;
    for (const action of out.actions) this.actions.set(action.key, action);
    return out;
  }
  input(input) {
    return this.save(this.component.event(structuredClone(this.state), input, this.props, {}));
  }
  raw(key) {
    return this.input({ kind: 'raw', key, event: {} });
  }
  native(observation = draft) {
    return this.input({
      kind: 'component',
      child: 'secret',
      name: 'changed',
      key: 'changed',
      event: observation,
    });
  }
  find(method) {
    const action = [...this.actions.values()].find((a) => a.method === method);
    assert.ok(action, 'expected ' + method);
    return action;
  }
  complete(action, result) {
    this.actions.delete(action.key);
    return this.input({ kind: 'action', key: action.key, event: result });
  }
  query(method, value) {
    return this.complete(this.find(method), { status: 'succeeded', value });
  }
  update(changes) {
    this.props = { ...this.props, ...changes };
    return this.save(this.component.update(structuredClone(this.state), this.props, {}));
  }
  get state() {
    return this.output.frame.state;
  }
}
const credentialModules = await Promise.all(
  ['desktop', 'tui'].map(async (target) => [
    target,
    await import(pathToFileURL(resolve(root, 'dist/packages/credential/' + target + '.js'))),
  ]),
);
const formReply = (value, ref = credentialRef) => ({
  status: 'operation',
  reply: {
    reference: ref,
    receipt: {
      kind: 'recorded',
      outcome: { kind: 'returned', value },
      cleanup: { kind: 'confirmed' },
    },
  },
});
const unknownReply = (ref = credentialRef) => ({
  status: 'operation',
  reply: {
    reference: ref,
    receipt: {
      kind: 'recorded',
      outcome: { kind: 'unknown', diagnostic: 'lost' },
      cleanup: { kind: 'confirmed' },
    },
  },
});
for (const [target, module] of credentialModules) {
  const make = (targetProps = connectionTarget) =>
    new FormClient(module.component, {
      theme: values.appearance[0],
      locale: 'en',
      target: targetProps,
    });
  const ready = () => {
    const form = make();
    form.query('credentialStatus', { kind: 'status', status: credentialStatus });
    form.native();
    return form;
  };
  test(
    target + ' credential captures original native version and resets only confirmed saved input',
    () => {
      const form = ready();
      form.raw('save');
      const action = form.find('saveCredential');
      assert.deepEqual(action.input, {
        expected: connectionTarget,
        expectedCredential: {
          locator: credentialStatus.locator,
          credentialId: 'saved-key',
          revision: 8,
        },
      });
      assert.deepEqual(action.resources, {
        secret: {
          child: 'secret',
          method: 'snapshot',
          input: { kind: 'at_revision', owner: draft.snapshot.text.owner, revision: 4 },
        },
      });
      assert.equal(
        'read' in Object.fromEntries([...form.actions.values()].map((a) => [a.method, a])),
        false,
      );
      const newer = structuredClone(draft);
      newer.revision = newer.snapshot.text.revision = 5;
      form.native(newer);
      form.complete(
        action,
        formReply({
          kind: 'saved',
          connection: {
            connectionId: connectionTarget.connectionId,
            slug: connectionTarget.slug,
            provider: connectionTarget.provider,
          },
        }),
      );
      assert.deepEqual(form.state.command, {
        owner: draft.snapshot.text.owner,
        basedOn: 4,
        sequence: 1,
        edit: { kind: 'reset', text: '' },
      });
      assert.equal(form.state.finished, true);
      const conflict = structuredClone(newer);
      conflict.receipt = {
        owner: draft.snapshot.text.owner,
        sequence: 1,
        revision: 5,
        outcome: { status: 'conflict', reason: 'revision' },
      };
      form.native(conflict);
      assert.equal(form.state.command, null);
      assert.equal(form.state.notice, 'draftKept');
      assert.equal(form.state.observation.snapshot.text.revision, 5);
      form.raw('save');
      assert.equal(form.output.actions.length, 0);
      form.raw('back');
      assert.equal(form.state.confirmation, 'leave');
      assert.deepEqual(form.output.events, []);
      form.raw('cancel');
      assert.equal(form.state.confirmation, null);
    },
  );
  test(
    target + ' unknown only checks original reference; mismatched receipts keep draft and intent',
    () => {
      const form = ready();
      form.raw('save');
      form.complete(form.find('saveCredential'), unknownReply());
      form.raw('save');
      assert.equal(form.output.actions.length, 0);
      form.raw('check');
      const check = form.find('read');
      assert.deepEqual(check.input, credentialRef);
      const wrong = { ...credentialRef, operation: uuid(99) };
      form.complete(check, {
        status: 'succeeded',
        value: formReply(
          {
            kind: 'saved',
            connection: {
              connectionId: connectionTarget.connectionId,
              slug: connectionTarget.slug,
              provider: connectionTarget.provider,
            },
          },
          wrong,
        ).reply,
      });
      assert.equal(form.state.command, null);
      assert.deepEqual(form.state.current.reference, credentialRef);
      form.raw('check');
      form.query('read', formReply({ kind: 'rejected', reason: 'invalid_secret' }).reply);
      assert.equal(form.state.current, null);
      assert.equal(form.state.notice, 'keyRejected');
      assert.equal(form.state.command, null);
      form.raw('save');
      assert.equal(form.output.actions[0].method, 'saveCredential');
    },
  );
  test(target + ' known CAS rejection keeps input and frozen target despite parent changes', () => {
    const form = ready();
    form.raw('save');
    form.complete(
      form.find('saveCredential'),
      formReply({ kind: 'rejected', reason: 'credential_changed' }),
    );
    assert.equal(form.state.notice, 'keyConflict');
    assert.equal(form.state.command, null);
    form.raw('save');
    assert.equal(form.output.actions.length, 0);
    const other = {
      ...connectionTarget,
      connectionId: 'connection-B',
      slug: 'connection-b',
      revision: 19,
    };
    form.update({ target: other });
    assert.deepEqual(form.state.target, connectionTarget);
    assert.deepEqual(form.state.observation, draft);
    form.raw('save');
    assert.equal(form.output.actions.length, 0);
  });
  test(
    target +
      ' earlier operations from another mount never change current draft or credential basis',
    () => {
      const other = { ...connectionTarget, connectionId: 'connection-B', slug: 'connection-b' };
      const form = make(other);
      const status = {
        ...credentialStatus,
        locator: { ...credentialStatus.locator, connectionId: other.connectionId },
      };
      form.query('credentialStatus', { kind: 'status', status });
      form.native();
      form.raw('history');
      const entries = [1, 2, 3, 4].map((cursor) => ({
        cursor,
        reference: credentialRef,
        caller: credentialRef.source,
        consumer: credentialRef.namespace,
      }));
      form.query('page', entries);
      form.raw('earlier-0');
      form.query('read', unknownReply().reply);
      assert.equal(form.state.current, null);
      assert.equal(form.state.notice, null);
      assert.deepEqual(form.state.status, status);
      assert.deepEqual(form.state.observation, draft);
      form.raw('earlier-0');
      form.query(
        'read',
        formReply({
          kind: 'saved',
          connection: {
            connectionId: connectionTarget.connectionId,
            slug: connectionTarget.slug,
            provider: connectionTarget.provider,
          },
        }).reply,
      );
      assert.equal(form.state.command, null);
      assert.equal(form.state.finished, false);
      assert.equal(form.state.current, null);
      assert.deepEqual(form.state.status, status);
      form.raw('history-back');
      form.raw('save');
      assert.deepEqual(form.find('saveCredential').input.expected, other);
      // A second mount of the same provider has independent candidate state, same caller history.
      const second = ready();
      second.raw('history');
      second.query('page', entries);
      second.raw('earlier-0');
      second.query(
        'read',
        formReply({
          kind: 'saved',
          connection: {
            connectionId: other.connectionId,
            slug: other.slug,
            provider: other.provider,
          },
        }).reply,
      );
      assert.equal(second.state.command, null);
      assert.deepEqual(second.state.observation, draft);
    },
  );
  test(
    target +
      ' credential removal requires confirmation and both original bases, never resets draft',
    () => {
      const form = ready();
      form.raw('remove');
      assert.equal(form.state.confirmation, 'remove');
      assert.equal(form.output.actions.length, 0);
      form.raw('cancel');
      assert.equal(form.output.actions.length, 0);
      form.raw('remove');
      form.raw('confirm');
      const action = form.find('deleteCredential');
      assert.deepEqual(action.input, {
        expectedConnection: connectionTarget,
        expected: { locator: credentialStatus.locator, credentialId: 'saved-key', revision: 8 },
      });
      assert.deepEqual(action.resources, {});
      const absent = {
        ...credentialStatus,
        configured: false,
        credentialId: null,
        revision: null,
        updatedAt: null,
      };
      form.complete(
        action,
        formReply(
          { kind: 'committed', vaultRevision: 9, status: absent },
          { ...credentialRef, method: 'deleteCredential' },
        ),
      );
      assert.deepEqual(form.state.status, absent);
      assert.equal(form.state.command, null);
      assert.deepEqual(form.state.observation, draft);
      form.raw('save');
      assert.equal(form.find('saveCredential').input.expectedCredential, null);
    },
  );
  test(target + ' credential status failures and marked input cannot dispatch a save', () => {
    const form = make();
    form.native();
    form.raw('save');
    assert.equal(form.output.actions.length, 0);
    form.complete(form.find('credentialStatus'), { status: 'failed', reason: 'offline' });
    assert.equal(form.state.statusFailed, true);
    form.raw('save');
    assert.equal(form.output.actions.length, 0);
    form.raw('refresh-status');
    form.query('credentialStatus', { kind: 'status', status: credentialStatus });
    const marked = structuredClone(draft);
    marked.snapshot.marked = { start: 0, end: 5 };
    form.native(marked);
    form.input({
      kind: 'component',
      key: 'submitted',
      child: 'secret',
      name: 'submitted',
      event: {},
    });
    assert.equal(form.output.actions.length, 0);
    const large = structuredClone(draft);
    large.snapshot.text.bytes = 65537;
    form.native(large);
    form.raw('save');
    assert.equal(form.state.notice, 'keyTooLarge');
    assert.equal(form.output.actions.length, 0);
  });
}
test('settings routing keeps original composer key and commands while invalidating model observations', () => {
  const client = new Client().start();
  const slots = (node) => [
    ...(node.kind === 'component' ? [node] : []),
    ...(node.children ?? []).flatMap(slots),
  ];
  const original = slots(client.output.frame.view).find((n) => n.binding === 'composer');
  client.raw('settings');
  assert.equal(client.state.settingsOpen, true);
  const hidden = client.output.frame.view.children[0];
  assert.equal(hidden.visible, false);
  assert.deepEqual(
    slots(hidden).find((n) => n.binding === 'composer'),
    original,
  );
  assert.ok(slots(client.output.frame.view).some((n) => n.binding === 'connections'));
  client.input({
    kind: 'component',
    key: 'settings-changed',
    child: 'connections',
    name: 'changed',
    event: {},
  });
  assert.equal(client.state.catalogEpoch, 1);
  assert.equal(client.state.model.selection.modelId, 'fixture-model');
  assert.equal(client.state.modelRefreshing, true);
  client.input({
    kind: 'component',
    key: 'settings-closed',
    child: 'connections',
    name: 'closed',
    event: {},
  });
  assert.equal(client.state.settingsOpen, false);
  assert.equal(client.output.frame.view.children[0].visible, true);
  assert.equal(
    slots(client.output.frame.view).find((n) => n.binding === 'composer').key,
    original.key,
  );
});
test('changed settings discard late model query and old default before selecting a fresh basis', async () => {
  const module = await import(pathToFileURL(resolve(root, 'dist/packages/models/desktop.js')));
  const form = new FormClient(module.component, {
    theme: values.appearance[0],
    locale: 'en',
    selection: {
      selection: { connectionId: 'fixture', revision: 2, modelId: 'fixture-model' },
      connectionName: 'Fixture',
      label: 'Fixture model',
      isDefault: false,
    },
    busy: false,
    catalogEpoch: 0,
  });
  const old = form.find('models');
  form.update({ catalogEpoch: 1 });
  const current = [...form.actions.values()].filter((a) => a.method === 'models').at(-1);
  assert.notEqual(old.key, current.key);
  const choice = {
    selection: { connectionId: 'fixture', revision: 2, modelId: 'fixture-model' },
    connectionName: 'Fixture',
    label: 'Fixture model',
    isDefault: true,
  };
  form.complete(old, {
    status: 'succeeded',
    value: { kind: 'ready', revision: 2, choices: [choice], next: null },
  });
  assert.equal(form.state.choices.length, 0);
  assert.deepEqual(form.output.events, []);
  form.complete(current, {
    status: 'succeeded',
    value: {
      kind: 'ready',
      revision: 3,
      choices: [{ ...choice, selection: { ...choice.selection, revision: 3 } }],
      next: null,
    },
  });
  assert.equal(form.output.events[0].value.selection.revision, 3);
});

for (const [target, module] of credentialModules) {
  test(
    target +
      ' preserved newer native draft can be saved after explicit fresh connection and vault review',
    () => {
      const form = new FormClient(module.component, {
        theme: values.appearance[0],
        locale: 'en',
        target: connectionTarget,
      });
      form.query('credentialStatus', { kind: 'status', status: credentialStatus });
      form.native();
      form.raw('save');
      const save = form.find('saveCredential');
      const newer = structuredClone(draft);
      newer.revision = newer.snapshot.text.revision = 5;
      form.native(newer);
      form.complete(
        save,
        formReply({
          kind: 'saved',
          connection: {
            connectionId: connectionTarget.connectionId,
            slug: connectionTarget.slug,
            provider: connectionTarget.provider,
          },
        }),
      );
      newer.receipt = {
        owner: draft.snapshot.text.owner,
        sequence: 1,
        revision: 5,
        outcome: { status: 'conflict', reason: 'revision' },
      };
      form.native(newer);
      form.raw('review');
      const fresh = {
        ...connectionTarget,
        revision: 13,
        configuration: { baseUrl: 'https://changed.example.invalid/v1' },
      };
      form.query('connection', {
        kind: 'stale',
        actual: { connectionId: fresh.connectionId, revision: 13 },
      });
      form.query('connection', {
        kind: 'ready',
        target: fresh,
        name: 'Connection',
        enabled: true,
        enabledModelCount: 1,
      });
      const updated = { ...credentialStatus, credentialId: 'new-saved-key', revision: 9 };
      form.query('credentialStatus', { kind: 'status', status: updated });
      assert.equal(form.state.confirmation, 'rebase');
      assert.deepEqual(form.state.target, connectionTarget);
      assert.deepEqual(form.state.observation, newer);
      form.raw('save');
      assert.equal(form.output.actions.length, 0);
      form.raw('review-confirm');
      assert.deepEqual(form.state.target, fresh);
      assert.deepEqual(form.state.status, updated);
      assert.deepEqual(form.state.observation, newer);
      assert.equal(form.state.command, null);
      form.raw('save');
      assert.deepEqual(form.find('saveCredential').input.expected, fresh);
      assert.deepEqual(form.find('saveCredential').resources.secret.input, {
        kind: 'at_revision',
        owner: draft.snapshot.text.owner,
        revision: 5,
      });
    },
  );
}
test('a refreshed catalog never replaces a chosen nondefault model with an unrelated default', async () => {
  const module = await import(pathToFileURL(resolve(root, 'dist/packages/models/desktop.js')));
  const chosen = {
    selection: { connectionId: 'chosen', revision: 2, modelId: 'chosen-model' },
    connectionName: 'Chosen',
    label: 'Chosen model',
    isDefault: false,
  };
  const form = new FormClient(module.component, {
    theme: values.appearance[0],
    locale: 'en',
    selection: chosen,
    busy: false,
    catalogEpoch: 0,
  });
  form.query('models', { kind: 'ready', revision: 2, choices: [chosen], next: null });
  form.update({ catalogEpoch: 1 });
  const other = {
    selection: { connectionId: 'other', revision: 8, modelId: 'default-model' },
    connectionName: 'Other',
    label: 'Default model',
    isDefault: true,
  };
  form.query('models', { kind: 'ready', revision: 8, choices: [other], next: null });
  assert.deepEqual(form.output.events, []);
  assert.equal(form.state.staleSelection, true);
  form.update({ busy: false });
  assert.deepEqual(form.output.events, []);
  form.raw('refresh');
  const renewed = { ...chosen, selection: { ...chosen.selection, revision: 9 } };
  form.query('models', { kind: 'ready', revision: 9, choices: [other, renewed], next: null });
  assert.equal(form.output.events[0].value.selection.connectionId, 'chosen');
  assert.equal(form.output.events[0].value.selection.revision, 9);
});

for (const [target, module] of credentialModules) {
  test(
    target +
      ' connection review exposes every receiver and configuration character through bounded pages',
    () => {
      const form = new FormClient(module.component, {
        theme: values.appearance[0],
        locale: 'en',
        target: connectionTarget,
      });
      form.query('credentialStatus', { kind: 'status', status: credentialStatus });
      form.native();
      form.raw('save');
      form.complete(
        form.find('saveCredential'),
        formReply({ kind: 'rejected', reason: 'connection_changed' }),
      );
      const endpoint = 'https://' + ('a'.repeat(52) + '.').repeat(3) + 'receiver-b.test/v1';
      const fresh = {
        ...connectionTarget,
        revision: 13,
        configuration: { baseUrl: endpoint, extra: '界'.repeat(300) },
      };
      form.raw('review');
      form.query('connection', {
        kind: 'stale',
        actual: { connectionId: fresh.connectionId, revision: 13 },
      });
      form.query('connection', {
        kind: 'ready',
        target: fresh,
        name: 'Connection',
        enabled: true,
        enabledModelCount: 1,
      });
      form.query('credentialStatus', { kind: 'status', status: credentialStatus });
      const originalEditor = form.output.frame.view.children[0].children.find(
        (n) => n.key === 'key-input',
      ).children[0];
      let complete = '';
      let pages = 0;
      do {
        const review = form.output.frame.view.children.find((n) => n.key === 'review-target');
        assert.equal(review.visible, true);
        const lines = review.children.filter((n) => n.key.startsWith('target-line-'));
        assert.ok(lines.length <= (target === 'tui' ? 8 : 24));
        assert.ok(lines.every((n) => [...n.text].length <= 36));
        complete += lines.map((n) => n.text).join('');
        assert.deepEqual(
          form.output.frame.view.children[0].children.find((n) => n.key === 'key-input')
            .children[0],
          originalEditor,
        );
        pages++;
        if (
          !review.children
            .find((n) => n.key === 'review-actions')
            .children.some((n) => n.key === 'review-next')
        )
          break;
        form.raw('review-next');
      } while (pages < 100);
      assert.ok(pages > 1 && pages < 100);
      assert.equal(
        complete,
        endpoint +
          JSON.stringify(fresh.provider) +
          fresh.slug +
          JSON.stringify(fresh.configuration),
      );
      assert.deepEqual(form.state.observation, draft);
      assert.deepEqual(form.state.target, connectionTarget);
      form.raw('review-confirm');
      form.raw('save');
      assert.equal(form.find('saveCredential').input.expected.configuration.baseUrl, endpoint);
    },
  );
}
