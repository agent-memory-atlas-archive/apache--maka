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
test('busy TUI navigation preserves native buttons and restores their original actions', async () => {
  const { component: navigation } = await import(
    pathToFileURL(resolve(root, 'dist/packages/navigation/tui.js'))
  );
  const props = {
    theme: values.appearance[0],
    locale: 'en',
    sessions: [],
    active: null,
    busy: false,
    more: false,
  };
  let result = navigation.mount(props, {});
  const button = () => result.frame.view.children.find((node) => node.key === 'new');
  const original = button();
  assert.equal(original.kind, 'button');
  assert.equal(original.disabled, false);
  assert.equal(original.color, props.theme.accent >>> 8);
  result = navigation.update(result.frame.state, { ...props, busy: true }, {});
  assert.equal(button().kind, original.kind);
  assert.equal(button().key, original.key);
  assert.equal(button().label, original.label);
  assert.equal(button().disabled, true);
  assert.equal(button().color, props.theme.muted >>> 8);
  result = navigation.event(
    result.frame.state,
    { kind: 'raw', key: 'new', event: null },
    { ...props, busy: true },
    {},
  );
  assert.deepEqual(result.events, []);
  result = navigation.update(result.frame.state, props, {});
  result = navigation.event(
    result.frame.state,
    { kind: 'raw', key: 'new', event: null },
    props,
    {},
  );
  assert.deepEqual(result.events, [{ name: 'create', value: {} }]);
});
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
  constructor(definition = product.component) {
    this.definition = definition;
    this.actions = new Map();
    this.output = this.definition.mount({}, values);
    this.save();
  }

  save() {
    for (const action of this.output.actions) this.actions.set(action.key, action);
  }
  input(input) {
    this.output = this.definition.event(
      structuredClone(this.output.frame.state),
      input,
      {},
      values,
    );
    this.save();
    return this.output;
  }
  find(method) {
    const action = [...this.actions.values()].reverse().find((action) => action.method === method);
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
    if (name === 'tailReady' && !Object.hasOwn(event, 'landing'))
      event = { landing: this.state.landing.sequence, readingRevision: 0 };
    return this.input({
      kind: 'component',
      key: name,
      child: ['send', 'draft', 'stop'].includes(name)
        ? 'composer'
        : [
              'older',
              'newer',
              'latest',
              'source',
              'sourceFirst',
              'sourcePrevious',
              'sourceNext',
              'sourceLast',
              'sourceBack',
              'sourceRetry',
              'toolSource',
              'tailReady',
            ].includes(name)
          ? 'timeline'
          : 'navigation',
      name: name === 'draft' ? 'changed' : name === 'sessions' ? 'more' : name,
      event,
    });
  }
  raw(key) {
    return this.input({ kind: 'raw', key, event: {} });
  }
  get state() {
    return this.output.frame.state;
  }
  get timeline() {
    return modelSlots(this.output.frame.view).find((node) => node.binding === 'timeline').props;
  }
  get messages() {
    return this.timeline.messages.filter((message) => !message.id.startsWith('live-'));
  }
  get liveMessages() {
    return this.timeline.messages.filter((message) => message.id.startsWith('live-'));
  }
  start({ through = 0, parts = [], run = null, chunks = [] } = {}) {
    this.query('page', []);
    this.query('list', { revision: 'catalog', sessions: [session], nextCursor: null });
    this.query('observe', { session, through, run, parts });
    this.query('history', { kind: 'ready', through, chunks, before: null, after: null });
    this.input({
      kind: 'component',
      key: 'model',
      child: 'models',
      name: 'selected',
      event: {
        selection: { connectionId: 'fixture', revision: 2, modelId: 'fixture-model' },
        connectionName: 'Fixture',
        label: 'Fixture model',
      },
    });
    this.component('tailReady');
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
  assert.equal(client.liveMessages[0].text, 'prefix suffix');
  assert.equal(client.liveMessages.length, 1);
  client.page([fact(9, { kind: 'delta', stepId: 'step', partId: 'same', text: 'duplicate' })], 9);
  assert.equal(client.liveMessages[0].text, 'prefix suffix');
  client.page(
    [
      fact(10, { kind: 'part_finished', stepId: 'step', partId: 'same' }),
      fact(11, { kind: 'step_ended', stepId: 'step' }),
      fact(12, { kind: 'ended', failed: false }),
    ],
    12,
  );
  assert.equal(
    [...client.actions.values()].some((action) => action.method === 'history'),
    true,
  );
  client.component('latest');
  const history = client.find('history');
  assert.deepEqual(history.input.position, { kind: 'tail' });
  client.query('history', {
    kind: 'ready',
    through: 12,
    chunks: [
      {
        messageId: 'answer',
        role: 'assistant',
        tool: null,
        sequence: 3,
        offset: 0,
        totalBytes: 13,
        text: 'prefix suffix',
      },
    ],
    before: null,
    after: null,
  });
  assert.equal(client.messages[0].text, 'prefix suffix');
  assert.equal(client.liveMessages.length, 0);
  assert.equal(client.state.parts.length, 0);
});
test('opening a run requests guarded follow while retaining open live text and its watch', () => {
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
  assert.equal(client.messages.length, 0);
  assert.equal(client.timeline.latest, true);
  assert.deepEqual(client.find('history').continuations, [
    { child: 'timeline', method: 'tail', input: null },
  ]);
  client.query('history', {
    kind: 'ready',
    through: 4,
    chunks: [
      {
        messageId: 'user',
        role: 'user',
        tool: null,
        sequence: 1,
        offset: 0,
        totalBytes: 5,
        text: 'Hello',
      },
    ],
    before: null,
    after: null,
  });
  assert.equal(client.messages[0].text, 'Hello');
  assert.equal(client.liveMessages[0].text, 'First response');
  assert.equal(client.state.live.length, 1);
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
  const message = client.liveMessages[0];
  assert.ok(message.text.length <= 16000);
  assert.equal(message.continued, true);
  assert.ok(message.text.isWellFormed());
  assert.ok(JSON.stringify(client.output).length < 200000);
});
test('both product compositions use actual immutable package refs and check in the real CLI', {
  skip: !process.env.MAKA_PRODUCT_CLI_CHECK,
}, async () => {
  const binary = resolve(root, '../..', process.env.CARGO_TARGET_DIR ?? 'target', 'debug/maka');
  const { packageNames } = await import(
    pathToFileURL(resolve(root, 'dist/authoring/agent-product/src/composition.mjs'))
  );
  const keys = Object.fromEntries(packageNames.map((name, index) => [name, uuid(index + 1)]));
  const directory = mkdtempSync(resolve(tmpdir(), 'maka-product-composition-'));
  try {
    const sourcePackage = resolve(root, 'dist/packages/appearance');
    const bundle = resolve(directory, 'appearance.maka-plugin');
    const invoke = (args) =>
      execFileSync(binary, ['application', ...args], {
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
      });
    const originalRef = JSON.parse(invoke(['inspect-package', '--package', sourcePackage]));
    assert.deepEqual(
      JSON.parse(invoke(['export-package', '--package', sourcePackage, '--output', bundle])),
      originalRef,
    );
    assert.deepEqual(JSON.parse(invoke(['inspect-package', '--package', bundle])), originalRef);
    const originalBytes = readFileSync(bundle);
    assert.throws(() => invoke(['export-package', '--package', sourcePackage, '--output', bundle]));
    assert.deepEqual(readFileSync(bundle), originalBytes);
    for (const productName of ['agent', 'studio'])
      for (const presentation of ['desktop', 'tui']) {
        const configuration = {
          presentation,
          locale: 'zh-Hant-HK',
          scheme: 'light',
          keys,
          agent: {
            maxActiveRuns: 1,
            maxSteps: 1,
            tools: [],
          },
          hostRoot: '/private/product-fixture',
          hostDeployment: uuid(80),
          managementDeployment: uuid(81),
        };
        const configurationFile = resolve(directory, 'installation.json');
        const output = resolve(directory, productName + '-' + presentation);
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
            '--product',
            productName,
          ],
          { encoding: 'utf8', maxBuffer: 1024 * 1024 },
        );
        const ui = JSON.parse(readFileSync(resolve(output, 'ui.json'), 'utf8'));
        const host = JSON.parse(readFileSync(resolve(output, 'host.json'), 'utf8'));
        const connections = JSON.parse(readFileSync(resolve(output, 'connections.json'), 'utf8'));
        assert.equal(ui.environment.presentation, presentation);
        if (productName === 'agent') {
          assert.equal(ui.instances.workspace.key, keys.workspace);
          assert.equal(host.instances.agent.key, keys.agent);
        } else {
          assert.deepEqual(ui.launch, { kind: 'instance', path: ['studio'], export: 'Workbench' });
          assert.equal(ui.instances.studio.key, keys.studio);
          for (const name of ['workspace', 'composer', 'timeline', 'navigation', 'agent'])
            assert.equal(name in ui.instances, false);
          assert.deepEqual(Object.keys(host.instances).sort(), ['gateway', 'receipts', 'settings']);
        }
        const business = connections.hosts.find((host) => host.name === 'local');
        const manager = connections.hosts.find((host) => host.name === 'manager');
        assert.deepEqual(manager.location, { kind: 'cohost' });
        assert.equal(manager.deployment, configuration.managementDeployment);
        assert.deepEqual(manager.imports.management.methods, [
          'instances',
          'setEnabled',
          'readDocument',
          'replace',
          'install',
        ]);
        assert.equal(manager.imports.receipts.recovery, true);
        assert.deepEqual(business.location, {
          kind: 'process',
          root: configuration.hostRoot,
        });
        assert.deepEqual(business.consumer, [
          keys[productName === 'agent' ? 'gateway' : 'studio-gateway'],
        ]);
        assert.equal('agent' in business.imports, productName === 'agent');
        assert.equal(business.deployment, configuration.hostDeployment);
        assert.deepEqual(
          business.imports.settings.methods,
          Object.keys(sdk.contracts.settings.interface.methods),
        );
        assert.equal(business.imports.settings.recovery, true);
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

test('reconnect replaces one canonical window without cross-page append', () => {
  const chunk = {
    messageId: 'user',
    role: 'user',
    tool: null,
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
  assert.equal(
    [...client.actions.values()].some((action) => action.method === 'history'),
    false,
  );
  assert.equal(client.messages[0].text, 'Hello');
  assert.equal(client.messages.length, 1);
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
  client.component('latest');
  const currentHistory = client.find('history');
  client.complete(lateObserve, {
    status: 'succeeded',
    value: { session, through: 20, parts: [], run: null },
  });
  assert.equal(client.state.latestKnown, 40);
  assert.equal(client.state.parts[0].startSequence, 30);
  client.complete(currentHistory, {
    status: 'succeeded',
    value: { kind: 'ready', through: 40, chunks: [], before: null, after: null },
  });
  assert.equal(client.state.watch.input.after, 29);
  client.page(
    [
      fact(30, { kind: 'part_started', ...part }),
      fact(41, { kind: 'delta', stepId: 'step', partId: 'part', text: 'Preserved reply' }),
    ],
    41,
  );
  assert.equal(client.liveMessages[0].text, 'Preserved reply');
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
    value: {
      kind: 'ready',
      revision: 5,
      choices: modelChoices,
      defaultModel: { kind: 'ready', choice: modelChoices[0] },
      next: null,
    },
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
      value: {
        kind: 'ready',
        revision: 5,
        choices: modelChoices,
        defaultModel: { kind: 'ready', choice: modelChoices[0] },
        next: null,
      },
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
test('sidebar presentation retains the same children, draft command, and active work', () => {
  const client = new Client().start();
  client.state.sessions[0].title = '长标题 · A🙂é '.repeat(25);
  client.component('draft', draft);
  const originalState = structuredClone(client.state);
  const originalSlots = modelSlots(client.output.frame.view);
  client.raw('navigation-toggle');
  assert.equal(client.state.navigationOpen, false);
  assert.equal(client.output.frame.view.children[0].children[0].visible, false);
  assert.deepEqual(modelSlots(client.output.frame.view), originalSlots);
  assert.deepEqual(client.output.actions, []);
  assert.deepEqual(client.state.watch, originalState.watch);
  assert.deepEqual(client.state.command, originalState.command);
  assert.deepEqual(client.state.model, originalState.model);
  client.raw('settings');
  client.input({ kind: 'component', child: 'connections', name: 'closed', event: {} });
  assert.equal(client.state.navigationOpen, false);
  client.raw('navigation-toggle');
  assert.equal(client.state.navigationOpen, true);
  assert.deepEqual(modelSlots(client.output.frame.view), originalSlots);
});
test('desktop model trigger retains its full label for native measurement and accessibility', async () => {
  const module = await import(pathToFileURL(resolve(root, 'dist/packages/models/desktop.js')));
  const label = '长模型名称 · A🙂é '.repeat(30);
  const output = module.component.mount({
    theme: values.appearance[0],
    locale: 'en',
    selection: {
      selection: { connectionId: 'fixture', revision: 2, modelId: 'fixture-model' },
      connectionName: 'Fixture',
      label,
    },
    busy: false,
    catalogEpoch: 0,
  });
  assert.equal(output.frame.view.trigger.label, label);
  assert.equal(output.frame.view.trigger.style.truncate, true);
  assert.equal(output.frame.view.trigger.style.minWidth, 0);
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
  };
  form.complete(old, {
    status: 'succeeded',
    value: {
      kind: 'ready',
      revision: 2,
      choices: [choice],
      defaultModel: { kind: 'ready', choice },
      next: null,
    },
  });
  assert.equal(form.state.choices.length, 0);
  assert.deepEqual(form.output.events, []);
  form.complete(current, {
    status: 'succeeded',
    value: {
      kind: 'ready',
      revision: 3,
      choices: [{ ...choice, selection: { ...choice.selection, revision: 3 } }],
      defaultModel: {
        kind: 'ready',
        choice: { ...choice, selection: { ...choice.selection, revision: 3 } },
      },
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
  };
  const form = new FormClient(module.component, {
    theme: values.appearance[0],
    locale: 'en',
    selection: chosen,
    busy: false,
    catalogEpoch: 0,
  });
  form.query('models', {
    kind: 'ready',
    revision: 2,
    choices: [chosen],
    defaultModel: { kind: 'none' },
    next: null,
  });
  form.update({ catalogEpoch: 1 });
  const other = {
    selection: { connectionId: 'other', revision: 8, modelId: 'default-model' },
    connectionName: 'Other',
    label: 'Default model',
  };
  form.query('models', {
    kind: 'ready',
    revision: 8,
    choices: [other],
    defaultModel: { kind: 'ready', choice: other },
    next: null,
  });
  assert.deepEqual(form.output.events, []);
  assert.equal(form.state.staleSelection, true);
  form.update({ busy: false });
  assert.deepEqual(form.output.events, []);
  form.raw('refresh');
  const renewed = { ...chosen, selection: { ...chosen.selection, revision: 9 } };
  form.query('models', {
    kind: 'ready',
    revision: 9,
    choices: [other, renewed],
    defaultModel: { kind: 'ready', choice: other },
    next: null,
  });
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

const connectionFormModules = await Promise.all(
  ['desktop', 'tui'].map(async (target) => [
    target,
    await import(pathToFileURL(resolve(root, 'dist/packages/connection-form/' + target + '.js'))),
  ]),
);
const publicDetail = {
  kind: 'ready',
  target: {
    ...connectionTarget,
    configuration: { baseUrl: 'https://example.test/v1', notes: '界'.repeat(12000) },
  },
  name: 'A\nconnection',
  enabled: true,
  enabledModelCount: 29,
};
const publicRef = { ...credentialRef, method: 'update' };
class PublicFormClient extends FormClient {
  constructor(component, connection = publicDetail) {
    super(component, { theme: values.appearance[0], locale: 'en', catalogRevision: 7, connection });
    this.texts = new Map();
    for (const name of [
      'name',
      'slug',
      'endpoint',
      'models',
      'configuration',
      'overrides',
      'overlay',
    ])
      this.observe(name, '');
  }
  observe(name, text, extra = {}) {
    this.texts.set(name, text);
    const field = this.state.fields[name];
    const revision = (field.observation?.revision ?? 0) + 1;
    const command = field.command;
    return this.input({
      kind: 'component',
      child: 'field-' + name,
      name: 'changed',
      key: 'field-changed',
      event: {
        revision,
        snapshot: {
          text: { owner: 'public-' + name, revision, bytes: Buffer.byteLength(text) },
          selection: { start: text.length, end: text.length },
          reversed: false,
          marked: null,
        },
        receipt: command
          ? {
              owner: command.owner,
              sequence: command.sequence,
              revision,
              outcome: { status: 'applied' },
            }
          : null,
        ...extra,
      },
    });
  }
  commands() {
    for (const name of Object.keys(this.state.fields)) {
      const command = this.state.fields[name].command;
      if (command) this.observe(name, command.edit.text);
    }
  }
  edit(name, text) {
    if (!this.state.fields[name].enabled) this.raw('edit-' + name);
    this.commands();
    this.observe(name, text);
  }
  readFields() {
    this.raw('form-save');
    const reads = [...this.actions.values()].filter((a) => a.binding === 'reader');
    for (const action of reads) {
      const name = action.resources.text.child.slice(6);
      const capture = action.resources.text.input;
      this.complete(action, {
        status: 'succeeded',
        value: {
          text: this.texts.get(name),
          metadata: {
            owner: capture.owner,
            revision: capture.revision,
            bytes: action.input.end,
          },
        },
      });
    }
    return reads;
  }
  chooseProvider() {
    this.query('providers', {
      next: null,
      providers: [
        {
          identity: connectionTarget.provider,
          descriptor: {
            label: 'Compatible',
            configurationDefaults: { baseUrl: 'https://api.test/v1', custom: 42 },
            configurationSchema: {},
            authentication: [],
            anonymous: true,
            discovery: false,
          },
        },
      ],
    });
    this.raw('provider-0');
    this.commands();
    this.observe('slug', 'my-provider');
    this.observe('name', 'My provider');
    this.observe('models', 'first-model\nsecond-model');
  }
}
for (const [target, module] of connectionFormModules) {
  test(
    target + ' removal requires confirmation and freezes the saved basis without reading drafts',
    () => {
      const form = new PublicFormClient(module.component);
      form.edit('name', 'unsaved🙂');
      const observation = structuredClone(form.state.fields.name.observation);
      form.raw('form-confirm-remove');
      assert.equal(form.output.actions.length, 0);
      form.raw('form-remove');
      assert.equal(form.state.confirmation, 'remove');
      assert.equal(form.output.actions.length, 0);
      const confirmation = form.output.frame.view.children.find(
        (n) => n.key === 'form-confirmation',
      );
      let identity = '';
      do {
        const panel = form.output.frame.view.children.find((n) => n.key === 'form-confirmation');
        identity += panel.children
          .filter((n) => n.key.startsWith('review-line-'))
          .map((n) => n.text)
          .join('');
        const navigation = panel.children.find((n) => n.key === 'confirm-actions');
        if (!navigation.children.some((n) => n.key === 'review-next')) break;
        form.raw('review-next');
      } while (true);
      assert.ok(identity.includes('Name: ' + publicDetail.name.replace(/\r?\n/gu, '')));
      assert.ok(identity.includes('Connection ID: ' + connectionTarget.slug));
      assert.ok(!identity.includes(connectionTarget.connectionId));
      assert.equal(confirmation.visible, true);
      form.raw('form-cancel');
      assert.deepEqual(form.state.fields.name.observation, observation);
      form.raw('form-remove');
      form.raw('form-confirm-remove');
      assert.deepEqual(form.find('remove').input, {
        expected: {
          connectionId: connectionTarget.connectionId,
          revision: connectionTarget.revision,
        },
      });
      assert.equal(form.output.actions.length, 1);
      assert.ok(![...form.actions.values()].some((a) => a.binding === 'reader'));
      form.complete(
        form.find('remove'),
        formReply({ kind: 'committed', catalogRevision: 8 }, { ...publicRef, method: 'remove' }),
      );
      assert.deepEqual(
        form.output.events.map((e) => e.name),
        ['changed', 'closed'],
      );
    },
  );
  test(
    target + ' unknown removal only reads its original receipt and never resets or resends',
    () => {
      const form = new PublicFormClient(module.component);
      form.edit('name', 'kept draft');
      const before = structuredClone(form.state.fields);
      const reference = { ...publicRef, method: 'remove' };
      form.raw('form-remove');
      form.raw('form-confirm-remove');
      form.complete(form.find('remove'), {
        status: 'operation',
        reply: { reference, receipt: { kind: 'unavailable' } },
      });
      assert.equal(form.state.current.kind, 'remove');
      assert.equal(form.state.notice, 'connectionUnknown');
      form.raw('form-remove');
      form.raw('form-confirm-remove');
      form.raw('form-save');
      assert.equal(form.output.actions.length, 0);
      form.raw('form-check');
      assert.deepEqual(form.find('read').input, reference);
      form.query(
        'read',
        formReply({ kind: 'committed', catalogRevision: 8 }, { ...reference, operation: uuid(99) })
          .reply,
      );
      assert.ok(form.state.current);
      assert.equal(form.output.events.length, 0);
      assert.deepEqual(form.state.fields, before);
      form.raw('form-check');
      form.query('read', formReply({ kind: 'committed', catalogRevision: 8 }, reference).reply);
      assert.deepEqual(
        form.output.events.map((e) => e.name),
        ['changed', 'closed'],
      );
      assert.deepEqual(form.state.fields, before);
    },
  );
  test(target + ' stale removal cannot silently adopt a newer connection version', () => {
    const form = new PublicFormClient(module.component);
    const reference = { ...publicRef, method: 'remove' };
    form.raw('form-remove');
    form.raw('form-confirm-remove');
    form.complete(
      form.find('remove'),
      formReply(
        {
          kind: 'connection_stale',
          expected: {
            connectionId: connectionTarget.connectionId,
            revision: connectionTarget.revision,
          },
          actual: { connectionId: connectionTarget.connectionId, revision: 20 },
        },
        reference,
      ),
    );
    form.raw('form-remove');
    assert.equal(form.output.actions.length, 0);
    assert.equal(form.state.confirmation, null);
    form.raw('form-review');
    form.query('connection', {
      kind: 'stale',
      actual: { connectionId: connectionTarget.connectionId, revision: 20 },
    });
    form.query('connection', { ...publicDetail, target: { ...connectionTarget, revision: 20 } });
    form.raw('form-rebase');
    assert.equal(form.output.actions.length, 0);
    form.raw('form-remove');
    assert.equal(form.output.actions.length, 0);
    form.raw('form-confirm-remove');
    assert.equal(form.find('remove').input.expected.revision, 20);
  });
  test(
    target +
      ' parent replacement blocks a pending removal confirmation and historical receipts preserve drafts',
    () => {
      const form = new PublicFormClient(module.component);
      form.edit('name', 'draft');
      const before = structuredClone(form.state.fields);
      form.raw('form-remove');
      form.update({
        connection: { ...publicDetail, target: { ...connectionTarget, connectionId: 'other' } },
      });
      form.raw('form-confirm-remove');
      assert.equal(form.output.actions.length, 0);
      form.raw('form-cancel');
      form.raw('form-history');
      const reference = { ...publicRef, method: 'remove' };
      form.query('page', [{ cursor: 1, reference }]);
      form.raw('form-earlier-0');
      form.query('read', formReply({ kind: 'committed', catalogRevision: 8 }, reference).reply);
      assert.equal(form.state.earlier.length, 1);
      assert.equal(form.output.events.length, 0);
      assert.deepEqual(form.state.fields, before);
      assert.equal(form.state.current, null);
    },
  );

  test(
    target +
      ' public name update preserves huge provider configuration and unknown model collections',
    () => {
      const form = new PublicFormClient(module.component);
      form.edit('name', 'Updated\nname');
      const reads = form.readFields();
      assert.equal(reads.length, 1);
      assert.deepEqual(reads[0].resources.text.input, {
        kind: 'at_revision',
        owner: form.state.fields.name.observation.snapshot.text.owner,
        revision: form.state.fields.name.observation.revision,
      });
      assert.deepEqual(form.find('update').input, {
        expected: {
          connectionId: connectionTarget.connectionId,
          revision: connectionTarget.revision,
        },
        changes: { name: 'Updated\nname' },
      });
      assert.equal(form.state.fields.configuration.observation.snapshot.text.bytes, 0);
      assert.equal(form.state.fields.configuration.command, null);
      assert.ok(!Object.hasOwn(form.find('update').input.changes, 'enabledModelIds'));
      form.complete(
        form.find('update'),
        formReply(
          {
            kind: 'committed',
            catalogRevision: 8,
            connection: {
              connectionId: connectionTarget.connectionId,
              revision: connectionTarget.revision + 1,
            },
          },
          publicRef,
        ),
      );
      assert.deepEqual(
        form.output.events.map((e) => e.name),
        ['changed', 'closed'],
      );
    },
  );
  test(target + ' native selection changes do not replace unmodified configuration', () => {
    const form = new PublicFormClient(module.component);
    form.raw('edit-configuration');
    const command = form.state.fields.configuration.command;
    assert.ok(command.edit.text.length > 8192);
    assert.deepEqual(JSON.parse(command.edit.text), publicDetail.target.configuration);
    form.commands();
    form.observe('configuration', form.texts.get('configuration'));
    form.readFields();
    assert.ok(![...form.actions.values()].some((a) => a.method === 'update'));
    assert.equal(form.state.notice, 'noConnectionChanges');
  });
  test(target + ' marked, unacknowledged prefill and failed original capture never mutate', () => {
    const form = new PublicFormClient(module.component);
    form.raw('edit-name');
    form.raw('form-save');
    assert.equal(form.output.actions.length, 0);
    form.commands();
    const f = form.state.fields.name.observation;
    form.observe('name', 'composing', {
      snapshot: { ...f.snapshot, marked: { start: 0, end: 2 } },
    });
    form.raw('form-save');
    assert.equal(form.output.actions.length, 0);
    form.observe('name', 'Settled');
    form.raw('form-save');
    const read = form.find('read');
    form.complete(read, {
      status: 'succeeded',
      value: {
        text: 'Settled',
        metadata: { ...form.state.fields.name.observation.snapshot.text, owner: 'another-editor' },
      },
    });
    assert.equal(form.state.current, null);
    assert.equal(form.state.notice, 'connectionDraftKept');
    assert.ok(![...form.actions.values()].some((a) => a.method === 'update'));
    assert.equal(form.state.fields.name.observation.snapshot.text.owner, 'public-name');
    form.readFields();
    assert.equal(form.find('update').input.changes.name, 'Settled');
  });
  test(target + ' full generated input validation keeps invalid typed JSON fixable', () => {
    for (const invalid of [
      '{"m":{"vision":null}}',
      '{"m":{"maxOutputTokens":"oops"}}',
      '{"m":{"maxOutputTokens":1e400}}',
      '{"m":{"unknown":true}}',
      '{"m":{"apiProtocol":"not-a-protocol"}}',
    ]) {
      const form = new PublicFormClient(module.component);
      form.edit('overrides', invalid);
      form.readFields();
      assert.equal(form.state.current, null);
      assert.equal(form.state.notice, 'invalidConnectionFields');
      assert.ok(![...form.actions.values()].some((a) => a.method === 'update'));
      assert.equal(form.texts.get('overrides'), invalid);
      const owner = form.state.fields.overrides.observation.snapshot.text.owner;
      form.observe('overrides', '{"m":{"vision":true}}');
      form.readFields();
      assert.deepEqual(form.find('update').input.changes, {
        modelOverrides: { m: { vision: true } },
      });
      assert.equal(form.state.fields.overrides.observation.snapshot.text.owner, owner);
    }
  });
  test(target + ' creation uses observed provider and separate exact field captures', () => {
    const form = new PublicFormClient(module.component, null);
    form.chooseProvider();
    const reads = form.readFields();
    assert.equal(reads.length, 4);
    const create = form.find('create');
    assert.deepEqual(create.input, {
      expectedCatalogRevision: 7,
      connection: {
        slug: 'my-provider',
        name: 'My provider',
        provider: connectionTarget.provider,
        configuration: { baseUrl: 'https://api.test/v1', custom: 42 },
        enabled: true,
        enabledModelIds: ['first-model', 'second-model'],
      },
    });
    assert.equal(Object.keys(create.resources).length, 0);
    assert.ok(!JSON.stringify(form.output.frame.view).includes('"binding":"secret"'));
  });
  test(target + ' nonfinite public configuration and overlay never silently become null', () => {
    for (const field of ['configuration', 'overlay']) {
      const form = new PublicFormClient(module.component);
      form.edit(field, '{"value":1e400}');
      form.readFields();
      assert.equal(form.state.notice, 'invalidConnectionFields');
      assert.equal(form.state.current, null);
      assert.ok(![...form.actions.values()].some((a) => a.method === 'update'));
      const owner = form.state.fields[field].observation.snapshot.text.owner;
      form.observe(field, '{"value":1}');
      form.readFields();
      assert.deepEqual(form.find('update').input.changes, {
        [field === 'overlay' ? 'requestBodyOverlay' : 'configuration']: { value: 1 },
      });
      assert.equal(form.state.fields[field].observation.snapshot.text.owner, owner);
      form.complete(form.find('update'), { status: 'not_accepted', diagnostic: 'blocked' });
      const nested = '{"value":'.repeat(150) + '1' + '}'.repeat(150);
      form.observe(field, nested);
      form.readFields();
      assert.equal(form.state.current, null);
      assert.equal(form.state.notice, 'invalidConnectionFields');
      assert.ok(![...form.actions.values()].some((a) => a.method === 'update'));
      assert.equal(form.state.fields[field].observation.snapshot.text.owner, owner);
    }
  });
  test(
    target + ' catalog CAS requires explicit review and preserves create owners and drafts',
    () => {
      const form = new PublicFormClient(module.component, null);
      form.chooseProvider();
      form.readFields();
      const snapshot = structuredClone(form.state.fields);
      const ref = { ...publicRef, method: 'create' };
      form.complete(
        form.find('create'),
        formReply(
          {
            kind: 'revision_conflict',
            expectedRevision: 7,
            actualRevision: 8,
          },
          ref,
        ),
      );
      form.raw('form-save');
      assert.equal(form.output.actions.length, 0);
      form.raw('form-review');
      form.query('connections', { kind: 'ready', revision: 9, connections: [], next: null });
      assert.equal(form.state.catalogRevision, 7);
      form.raw('form-rebase');
      assert.equal(form.state.catalogRevision, 9);
      assert.equal(form.output.actions.length, 0);
      assert.deepEqual(form.state.fields, snapshot);
      form.readFields();
      assert.equal(form.find('create').input.expectedCatalogRevision, 9);
    },
  );
  test(
    target + ' connection CAS review shows full remote keys and requires explicit replacement',
    () => {
      const form = new PublicFormClient(module.component);
      form.edit('configuration', '{"baseUrl":"https://local.test","mine":1}');
      form.readFields();
      form.complete(
        form.find('update'),
        formReply(
          {
            kind: 'connection_stale',
            expected: {
              connectionId: connectionTarget.connectionId,
              revision: connectionTarget.revision,
            },
            actual: {
              connectionId: connectionTarget.connectionId,
              revision: connectionTarget.revision + 1,
            },
          },
          publicRef,
        ),
      );
      form.raw('form-review');
      form.query('connection', {
        kind: 'stale',
        actual: {
          connectionId: connectionTarget.connectionId,
          revision: connectionTarget.revision + 1,
        },
      });
      const latest = {
        ...publicDetail,
        target: {
          ...connectionTarget,
          revision: connectionTarget.revision + 1,
          configuration: {
            baseUrl: 'https://remote.test',
            newlyAdded: 'keep-unless-explicitly-replaced',
          },
        },
      };
      form.query('connection', latest);
      assert.deepEqual(form.state.baseline, publicDetail);
      const owner = form.state.fields.configuration.observation.snapshot.text.owner;
      form.raw('form-rebase');
      assert.deepEqual(form.state.baseline, latest);
      assert.equal(form.state.fields.configuration.observation.snapshot.text.owner, owner);
      form.readFields();
      assert.deepEqual(form.find('update').input, {
        expected: {
          connectionId: connectionTarget.connectionId,
          revision: connectionTarget.revision + 1,
        },
        changes: { configuration: { baseUrl: 'https://local.test', mine: 1 } },
      });
    },
  );
  test(
    target +
      ' parent refresh of the same connection needs consent and another target cannot take the draft',
    () => {
      const form = new PublicFormClient(module.component);
      form.edit('name', 'Retained local name');
      const fields = structuredClone(form.state.fields);
      const latest = {
        ...publicDetail,
        name: 'Refreshed remote name',
        target: {
          ...publicDetail.target,
          revision: publicDetail.target.revision + 1,
        },
      };
      form.update({ connection: latest });
      form.raw('form-save');
      assert.equal(form.output.actions.length, 0);
      form.raw('form-review');
      form.query('connection', {
        kind: 'stale',
        actual: {
          connectionId: latest.target.connectionId,
          revision: latest.target.revision,
        },
      });
      form.query('connection', latest);
      form.raw('form-rebase');
      assert.deepEqual(form.state.opened, latest);
      assert.deepEqual(form.state.fields, fields);
      form.readFields();
      assert.equal(form.find('update').input.changes.name, 'Retained local name');
      form.complete(form.find('update'), { status: 'not_accepted', diagnostic: 'blocked' });
      form.update({
        connection: { ...latest, target: { ...latest.target, connectionId: uuid(112) } },
      });
      form.raw('form-review');
      assert.equal(form.output.actions.length, 0);
      form.raw('form-save');
      assert.equal(form.output.actions.length, 0);
      assert.deepEqual(form.state.fields, fields);
    },
  );
  test(
    target + ' unknown mutation cannot be reviewed or replayed and checks only original reference',
    () => {
      const form = new PublicFormClient(module.component);
      form.edit('name', 'Unknown draft');
      form.readFields();
      form.complete(form.find('update'), unknownReply(publicRef));
      form.raw('form-save');
      assert.equal(form.output.actions.length, 0);
      form.raw('form-review');
      assert.equal(form.output.actions.length, 0);
      form.raw('form-check');
      const check = form.find('read');
      assert.equal(check.binding, 'receipts');
      assert.deepEqual(check.input, publicRef);
      form.complete(check, {
        status: 'succeeded',
        value: formReply(
          {
            kind: 'committed',
            connection: { connectionId: 'other', revision: 1 },
            catalogRevision: 9,
          },
          { ...publicRef, operation: uuid(119) },
        ).reply,
      });
      assert.deepEqual(form.state.current.reference, publicRef);
      assert.equal(form.output.events.length, 0);
    },
  );
}

const defaultModules = await Promise.all(
  ['desktop', 'tui'].map(async (target) => [
    target,
    await import(pathToFileURL(resolve(root, 'dist/packages/model-default/' + target + '.js'))),
  ]),
);
const defaultChoice = modelChoices[15];
const defaultReference = { ...credentialRef, method: 'setDefault' };
for (const [target, module] of defaultModules) {
  const make = (extra = {}) =>
    new FormClient(module.component, {
      theme: values.appearance[0],
      locale: 'en',
      selection: defaultChoice,
      catalogRevision: 5,
      defaultModel: { kind: 'none' },
      busy: false,
      ...extra,
    });
  test(
    target +
      ' default confirmation freezes its chosen model and catalog basis without selecting it',
    () => {
      const form = make();
      form.raw('default-confirm');
      assert.equal(form.output.actions.length, 0);
      form.raw('default-set');
      assert.equal(form.output.actions.length, 0);
      form.update({ selection: modelChoices[0] });
      form.raw('default-confirm');
      assert.deepEqual(form.find('setDefault').input, {
        expectedCatalogRevision: 5,
        target: {
          connectionId: defaultChoice.selection.connectionId,
          modelId: defaultChoice.selection.modelId,
        },
      });
      form.complete(
        form.find('setDefault'),
        formReply({ kind: 'committed', catalogRevision: 6 }, defaultReference),
      );
      assert.deepEqual(form.output.events, [{ name: 'changed', value: {} }]);
      assert.deepEqual(form.props.selection, modelChoices[0]);
    },
  );
  test(
    target +
      ' clearing a ready or unavailable default requires confirmation and preserves the current selection',
    () => {
      for (const saved of [
        { kind: 'ready', choice: modelChoices[0] },
        { kind: 'unavailable', target: { connectionId: 'unsupported', modelId: 'other-model' } },
      ]) {
        const form = make({ defaultModel: saved });
        form.raw('default-clear');
        form.raw('default-cancel');
        assert.equal(form.output.actions.length, 0);
        assert.equal(form.state.current, null);
        assert.deepEqual(form.props.defaultModel, saved);
        form.raw('default-clear');
        form.raw('default-confirm');
        assert.deepEqual(form.find('setDefault').input, {
          expectedCatalogRevision: 5,
          target: null,
        });
        form.complete(
          form.find('setDefault'),
          formReply({ kind: 'committed', catalogRevision: 6 }, defaultReference),
        );
        assert.equal(form.state.notice, 'defaultCleared');
        assert.deepEqual(form.output.events, [{ name: 'changed', value: {} }]);
        assert.deepEqual(form.props.selection, defaultChoice);
      }
    },
  );
  test(target + ' default CAS conflict needs a fresh observation and a new confirmation', () => {
    const form = make();
    form.raw('default-set');
    form.raw('default-confirm');
    form.complete(
      form.find('setDefault'),
      formReply(
        { kind: 'revision_conflict', expectedRevision: 5, actualRevision: 8 },
        defaultReference,
      ),
    );
    assert.equal(form.state.current, null);
    form.raw('default-set');
    assert.equal(form.state.confirmation, null);
    form.update({ catalogRevision: 8 });
    assert.equal(form.output.actions.length, 0);
    form.raw('default-set');
    assert.equal(form.output.actions.length, 0);
    form.raw('default-confirm');
    assert.equal(form.find('setDefault').input.expectedCatalogRevision, 8);
  });
  test(target + ' an in-flight confirmation cannot accept a refreshed catalog version', () => {
    const form = make();
    form.raw('default-set');
    form.update({ catalogRevision: 6 });
    form.raw('default-confirm');
    assert.equal(form.output.actions.length, 0);
    assert.equal(form.state.confirmation.revision, 5);
    form.raw('default-cancel');
    form.raw('default-set');
    form.raw('default-confirm');
    assert.equal(form.find('setDefault').input.expectedCatalogRevision, 6);
  });
  test(target + ' unknown defaults only read the original receipt across parent updates', () => {
    const form = make();
    form.raw('default-set');
    form.raw('default-confirm');
    form.complete(form.find('setDefault'), {
      status: 'operation',
      reply: { reference: defaultReference, receipt: { kind: 'unavailable' } },
    });
    form.update({ catalogRevision: 9, selection: modelChoices[0] });
    for (const key of ['default-set', 'default-clear', 'default-confirm']) {
      form.raw(key);
      assert.equal(form.output.actions.length, 0);
    }
    form.raw('default-check');
    assert.deepEqual(form.find('read').input, defaultReference);
    form.query(
      'read',
      formReply(
        { kind: 'committed', catalogRevision: 6 },
        { ...defaultReference, operation: uuid(99) },
      ).reply,
    );
    assert.ok(form.state.current);
    assert.equal(form.output.events.length, 0);
    form.raw('default-check');
    form.query(
      'read',
      formReply({ kind: 'committed', catalogRevision: 6 }, defaultReference).reply,
    );
    assert.equal(form.state.current, null);
    assert.deepEqual(form.output.events, [{ name: 'changed', value: {} }]);
    assert.deepEqual(form.props.selection, modelChoices[0]);
  });
  test(
    target + ' historical default receipts never resolve another current intent or select a model',
    () => {
      const form = make();
      form.raw('default-set');
      form.raw('default-confirm');
      form.complete(form.find('setDefault'), {
        status: 'operation',
        reply: { reference: defaultReference, receipt: { kind: 'unavailable' } },
      });
      const before = structuredClone(form.state.current);
      form.raw('default-history');
      assert.equal(form.find('page').input.limit, 2);
      const earlier = { ...defaultReference, operation: uuid(99) };
      form.query('page', [
        { cursor: 1, reference: credentialRef },
        { cursor: 2, reference: earlier },
      ]);
      assert.equal(form.state.earlier.length, 1);
      assert.equal(form.state.historyNext, 2);
      form.raw('default-earlier-0');
      form.query('read', formReply({ kind: 'committed', catalogRevision: 4 }, earlier).reply);
      assert.deepEqual(form.state.current, before);
      assert.deepEqual(form.output.events, []);
      form.raw('default-history-back');
      assert.deepEqual(form.props.selection, defaultChoice);
    },
  );
  test(
    target + ' every legal long default identity remains available on bounded confirmation pages',
    () => {
      const selected = {
        selection: { connectionId: 'original-connection', revision: 3, modelId: 'm'.repeat(512) },
        connectionName: '界'.repeat(255) + '\n',
        label: '🙂'.repeat(256),
      };
      const form = make({ selection: selected });
      form.raw('default-set');
      let all = '';
      let count = 0;
      while (true) {
        const pane = form.output.frame.view.children.find((n) => n.key === 'default-confirmation');
        const lines = pane.children.filter((n) => n.key.startsWith('default-line-'));
        assert.ok(lines.length <= (target === 'tui' ? 3 : 18));
        if (target === 'tui') assert.ok(lines.every((n) => [...n.text].length <= 36));
        all += lines.map((n) => n.text).join('');
        count++;
        if (
          !pane.children
            .find((n) => n.key === 'default-confirm-actions')
            .children.some((n) => n.key === 'default-next')
        )
          break;
        form.raw('default-next');
        assert.ok(count < 100);
      }
      assert.equal(
        all,
        selected.connectionName.replace(/\r?\n/gu, '') +
          selected.label +
          selected.selection.modelId,
      );
      form.raw('default-confirm');
      assert.equal(form.find('setDefault').input.target.modelId, selected.selection.modelId);
    },
  );
}
function modelSlots(node, result = []) {
  if (node.kind === 'component') result.push(node);
  for (const child of [...(node.children ?? []), node.trigger, node.content].filter(Boolean))
    modelSlots(child, result);
  return result;
}
test('Timeline byte-segment identities leave room for the native key prefix', async () => {
  const { default: Ajv } = await import('ajv/dist/2020.js');
  const { timelineContract } = await import(
    pathToFileURL(resolve(root, 'dist/authoring/agent-product/src/contracts.mjs'))
  );
  const schema = timelineContract.interface.props.$defs.Message;
  const validate = new Ajv({ strict: false }).compile({
    ...schema,
    $defs: timelineContract.interface.props.$defs,
  });
  const message = {
    id: uuid(900),
    contentKey: 'a'.repeat(123),
    image: false,
    role: 'tool_result',
    tool: null,
    text: 'literal output',
    continued: false,
    source: null,
  };
  assert.equal(validate(message), true);
  for (const contentKey of ['', 'a'.repeat(124), '😀'.repeat(31), 'key.with.dot'])
    assert.equal(validate({ ...message, contentKey }), false, contentKey);
});

for (const target of ['desktop', 'tui']) {
  test(
    target + ' canonical tools display names and reported errors even without text',
    async () => {
      const { component: timeline } = await import(
        pathToFileURL(resolve(root, 'dist/packages/timeline/' + target + '.js'))
      );
      const client = new Client().start({
        through: 100,
        chunks: [
          historyChunk('', {
            messageId: uuid(901),
            sequence: 39,
            role: 'tool_call',
            tool: { kind: 'call', toolUseId: uuid(901), toolName: 'Count' },
          }),
          historyChunk('', {
            sequence: 40,
            role: 'tool_result',
            tool: {
              kind: 'result',
              toolUseId: uuid(901),
              toolName: 'Count',
              isError: true,
              image: false,
            },
          }),
        ],
      });
      const out = timeline.mount({ ...client.timeline, locale: 'zh-CN' }, {});
      const rows = out.frame.view.children[0].children[0].props.content.children;
      assert.equal(rows.length, 2);
      assert.equal(rows[0].children[0].children[0].text, '工具活动 · Count');
      assert.equal(rows[1].children[0].children[0].text, '工具输出 · Count');
      assert.equal(
        rows[1].children.find((node) => node.key === 'tool-error-' + uuid(900)).text,
        '工具报告错误',
      );
      for (const [index, row] of rows.entries()) {
        assert.equal(
          row.children.find((node) => node.key.startsWith('summary-')).text,
          '无文本摘要',
        );
        const details = row.children.find((node) => node.key.startsWith('details-'));
        assert.equal(details.visible, false);
        assert.equal(
          row.children[0].children.find((node) => node.key.startsWith('toggle-')).label,
          '展开详情',
        );
        const actions = details.children.find((node) => node.key.startsWith('tool-actions-'));
        assert.equal(actions.children.length, 1);
        assert.equal(actions.children[0].label, index === 0 ? '工具参数' : '工具结果');
      }
      assert.equal(out.actions.length, 0);
      assert.equal(out.events.length, 0);
      assert.deepEqual(
        client.messages.map((message) => message.source),
        [39, 40],
      );
    },
  );
  test(target + ' suggests the exact saved default even beyond the first model page', async () => {
    const module = await import(
      pathToFileURL(resolve(root, 'dist/packages/models/' + target + '.js'))
    );
    const form = new FormClient(module.component, { ...modelProps, busy: false });
    const late = {
      ...modelChoices[15],
      selection: { ...modelChoices[15].selection, modelId: 'model-19' },
    };
    form.query('models', {
      kind: 'ready',
      revision: 5,
      choices: modelChoices,
      defaultModel: { kind: 'ready', choice: late },
      next: { connection: 0, model: 16 },
    });
    assert.deepEqual(form.output.events, [{ name: 'selected', value: late }]);
  });
  test(
    target +
      ' default changes retire an older model query while preserving a later-page selection and child identity',
    async () => {
      const module = await import(
        pathToFileURL(resolve(root, 'dist/packages/models/' + target + '.js'))
      );
      const form = new FormClient(module.component, {
        ...modelProps,
        busy: false,
        selection: defaultChoice,
      });
      const old = form.find('models');
      const child = modelSlots(form.output.frame.view).find((n) => n.binding === 'defaults');
      assert.equal(child.key, 'default-settings');
      form.input({
        kind: 'component',
        child: 'default-settings',
        name: 'changed',
        key: 'default-changed',
        event: {},
      });
      const fresh = [...form.actions.values()].filter((a) => a.method === 'models').at(-1);
      assert.notEqual(old.key, fresh.key);
      form.complete(old, {
        status: 'succeeded',
        value: {
          kind: 'ready',
          revision: 5,
          choices: [modelChoices[0]],
          defaultModel: { kind: 'ready', choice: modelChoices[0] },
          next: null,
        },
      });
      assert.equal(form.state.catalogRevision, null);
      form.complete(fresh, {
        status: 'succeeded',
        value: {
          kind: 'ready',
          revision: 6,
          choices: modelChoices.slice(0, 3),
          defaultModel: { kind: 'none' },
          next: null,
        },
      });
      assert.deepEqual(form.output.events, []);
      assert.equal(form.state.staleSelection, false);
      assert.deepEqual(form.props.selection, defaultChoice);
      form.raw('toggle');
      form.raw('defaults');
      form.raw('default-close');
      assert.equal(
        modelSlots(form.output.frame.view).find((n) => n.binding === 'defaults').key,
        child.key,
      );
    },
  );
}

test('desktop model popup dismissal preserves the retained default workflow and only Escape consumes its native input', async () => {
  const module = await import(pathToFileURL(resolve(root, 'dist/packages/models/desktop.js')));
  const form = new FormClient(module.component, {
    ...modelProps,
    busy: false,
    selection: defaultChoice,
  });
  form.raw('toggle');
  form.raw('defaults');
  const popup = form.output.frame.view;
  assert.equal(popup.kind, 'popup');
  assert.equal(popup.open, true);
  const child = modelSlots(popup).find((node) => node.binding === 'defaults');
  form.input({ kind: 'raw', key: 'model-popup', event: { kind: 'dismiss', reason: 'escape' } });
  assert.equal(form.state.open, false);
  assert.equal(form.state.defaultsOpen, true);
  assert.deepEqual(form.output.decision, { preventDefault: true, stopPropagation: true });
  assert.deepEqual(
    modelSlots(form.output.frame.view).find((node) => node.binding === 'defaults'),
    child,
  );
  form.raw('toggle');
  form.input({ kind: 'raw', key: 'model-popup', event: { kind: 'dismiss', reason: 'outside' } });
  assert.equal(form.state.open, false);
  assert.equal(form.output.decision, undefined);
  assert.equal(form.state.defaultsOpen, true);
  assert.equal(form.output.actions.length, 0);
});

test('prepending history preserves the streamed message presentation identity', async () => {
  const { component: timeline } = await import(
    pathToFileURL(resolve(root, 'dist/packages/timeline/desktop.js'))
  );
  const message = {
    id: uuid(400),
    contentKey: 'live-' + uuid(400),
    role: 'assistant',
    tool: null,
    text: '**流式** café 😀',
    continued: false,
    source: null,
  };
  const props = {
    theme: values.appearance[0],
    locale: 'en',
    messages: [message],
    source: null,
    loading: false,
    older: false,
    newer: false,
    latest: false,
    landing: { sequence: 0, edge: 'end' },
    live: true,
  };
  const initial = timeline.mount(props, {});
  const oldRow = initial.frame.view.children[0].children[0].props.content.children[0];
  const oldBody = oldRow.children.find((node) => node.kind === 'component');
  assert.ok(oldBody);
  const next = timeline.update(
    initial.frame.state,
    {
      ...props,
      messages: [
        {
          id: uuid(399),
          contentKey: 'canonical-399-0',
          role: 'user',
          tool: null,
          text: 'earlier',
          continued: false,
          source: null,
        },
        { ...message, text: message.text + ' continued' },
      ],
    },
    {},
  );
  const row = next.frame.view.children[0].children[0].props.content.children[1];
  const body = row.children.find((node) => node.kind === 'component');
  assert.equal(row.key, oldRow.key);
  assert.equal(body.key, oldBody.key);
  assert.equal(body.binding, oldBody.binding);
  assert.equal(body.props.source, message.text + ' continued');
  assert.notEqual(next.frame.view.children[0].children[0].props.content.children[0].key, row.key);
});

test('desktop scroll-to-end uses native painted facts and preserves its accepted command', async () => {
  const { component: timeline } = await import(
    pathToFileURL(resolve(root, 'dist/packages/timeline/desktop.js'))
  );
  const props = {
    theme: values.appearance[0],
    locale: 'en',
    messages: [],
    source: null,
    loading: false,
    older: false,
    newer: false,
    latest: false,
    landing: { sequence: 0, edge: 'end' },
    live: true,
  };
  let result = timeline.mount(props, {});
  const reading = () => result.frame.view.children[0].children[0];
  assert.equal(reading().binding, 'reading');
  assert.equal(result.frame.view.children.length, 1);
  const observation = {
    readingRevision: 2,
    viewport: { readingRevision: 2, atEnd: false, visibleChildKeys: ['message-original'] },
    pendingCommand: null,
    receipt: null,
  };
  result = timeline.event(
    result.frame.state,
    {
      kind: 'component',
      child: 'reading',
      name: 'observed',
      key: 'reading-observed',
      event: observation,
    },
    props,
    {},
  );
  assert.equal(result.frame.view.children[1].children[0].key, 'end');
  result = timeline.event(result.frame.state, { kind: 'raw', key: 'end', event: null }, props, {});
  assert.deepEqual(reading().props.command, { sequence: 1, target: { kind: 'end' } });
  result = timeline.update(result.frame.state, { ...props, live: false }, {});
  assert.equal(reading().props.command.sequence, 1);
  assert.equal(reading().props.content.scroll, 'anchor');
  result = timeline.event(
    result.frame.state,
    {
      kind: 'component',
      child: 'reading',
      name: 'observed',
      key: 'reading-observed',
      event: { ...observation, pendingCommand: 1 },
    },
    props,
    {},
  );
  assert.equal(result.frame.view.children[1].children[0].disabled, true);
  result = timeline.event(result.frame.state, { kind: 'raw', key: 'end', event: null }, props, {});
  assert.equal(reading().props.command.sequence, 1);
  result = timeline.event(
    result.frame.state,
    {
      kind: 'component',
      child: 'reading',
      name: 'input',
      key: 'reading-input',
      event: { kind: 'raw', key: 'newer', event: null },
    },
    props,
    {},
  );
  assert.deepEqual(result.events, []);
  const navigation = timeline.event(
    result.frame.state,
    { kind: 'raw', key: 'newer', event: null },
    { ...props, newer: true },
    {},
  );
  assert.deepEqual(navigation.events, [{ name: 'newer', value: {} }]);
  result = timeline.event(
    result.frame.state,
    {
      kind: 'component',
      child: 'reading',
      name: 'observed',
      key: 'reading-observed',
      event: {
        ...observation,
        viewport: { ...observation.viewport, atEnd: true },
        receipt: { sequence: 1, status: 'applied' },
      },
    },
    props,
    {},
  );
  assert.equal(result.frame.view.children.length, 1);
});

const historyChunk = (text, overrides = {}) => ({
  messageId: uuid(900),
  turnId: uuid(901),
  timestamp: 1,
  role: 'assistant',
  tool: null,
  sequence: 25600,
  offset: 0,
  totalBytes: Buffer.byteLength(text),
  text,
  ...overrides,
});
test('canonical navigation replaces one exact fixed-fence window and keeps it during Preparing/error', () => {
  const current = historyChunk('**完整** café 😀');
  const client = new Client().start({ through: 100, chunks: [current] });
  const page = {
    ...client.state.window.page,
    before: { sequence: 20000, offset: 3 },
    after: { sequence: 25600, offset: Buffer.byteLength(current.text) },
  };
  client.state.window.page = page;
  // Detached state passed through a real callback; no source concatenation or trimming.
  client.component('older');
  let action = client.find('history');
  const expected = {
    sessionId: session.id,
    through: 100,
    position: { kind: 'before', cursor: page.before },
  };
  assert.deepEqual(action.input, expected);
  assert.deepEqual(client.state.window.page, page);
  client.complete(action, { status: 'succeeded', value: { kind: 'preparing', through: 100 } });
  action = client.find('history');
  assert.deepEqual(action.input, expected);
  assert.deepEqual(client.state.window.page, page);
  client.complete(action, { status: 'failed', reason: 'not available' });
  assert.deepEqual(client.state.window.page, page);
  assert.equal(client.state.loading, false);
  client.component('older');
  const older = {
    kind: 'ready',
    through: 100,
    chunks: [historyChunk('older', { messageId: uuid(902), sequence: 19000 })],
    before: null,
    after: page.before,
  };
  client.query('history', older);
  assert.deepEqual(client.state.window.page, older);
  assert.deepEqual(
    client.messages.map((message) => message.text),
    ['older'],
  );
  assert.equal(client.state.landing.edge, 'end');
  assert.equal(client.timeline.live, false);
  const demand = structuredClone(client.output.subscriptions);
  client.page([fact(101, { kind: 'opened' })], 101);
  assert.deepEqual(client.state.window.page, older);
  assert.equal(client.timeline.latest, true);
  assert.equal(client.output.actions.length, 0);
  client.component('newer');
  assert.deepEqual(client.find('history').input, {
    sessionId: session.id,
    through: 100,
    position: { kind: 'after', cursor: older.after },
  });
  client.query('history', page);
  assert.equal(client.state.landing.edge, 'start');
  client.component('latest');
  assert.deepEqual(client.find('history').input, {
    sessionId: session.id,
    through: 101,
    position: { kind: 'tail' },
  });
  client.query('history', { ...page, through: 101, before: page.before, after: null });
  assert.equal(client.state.landing.edge, 'end');
  assert.equal(client.timeline.latest, false);
  assert.deepEqual(client.output.subscriptions, demand);
  assert.equal(client.messages[0].text, current.text);
});
test('live fragments use event fences even when a canonical prefix has the same message identity', () => {
  const part = {
    startSequence: 3,
    messageId: uuid(900),
    stepId: 'step',
    partId: 'part',
    textKind: 'text',
  };
  const client = new Client().start({
    through: 8,
    chunks: [historyChunk('canonical prefix')],
    parts: [part],
    run: { invocationId: 'run', state: 'running' },
  });
  client.page(
    [
      fact(3, { kind: 'part_started', ...part }),
      fact(4, { kind: 'delta', stepId: 'step', partId: 'part', text: 'already canonical' }),
      fact(9, { kind: 'delta', stepId: 'step', partId: 'part', text: ' new suffix' }),
    ],
    9,
  );
  assert.equal(client.messages[0].text, 'canonical prefix');
  assert.equal(client.liveMessages[0].text, ' new suffix');
  assert.equal(client.liveMessages[0].continued, true);
  assert.equal(client.timeline.latest, true);
  client.component('latest');
  client.query('history', {
    kind: 'ready',
    through: 9,
    chunks: [historyChunk('canonical prefix new suffix')],
    before: null,
    after: null,
  });
  assert.equal(client.state.live.length, 0);
  assert.equal(client.liveMessages.length, 0);
  assert.equal(client.messages[0].text, 'canonical prefix new suffix');
});
test('ended part identities are reclaimed without navigating, while active part demand stays bounded', () => {
  const client = new Client().start();
  let sequence = 0;
  for (let index = 0; index < 150; index++) {
    const stepId = 'step-' + index;
    client.page(
      [
        fact(++sequence, {
          kind: 'part_started',
          messageId: 'message-' + index,
          stepId,
          partId: 'part',
          textKind: 'text',
        }),
        fact(++sequence, { kind: 'delta', stepId, partId: 'part', text: 'reply' }),
        fact(++sequence, { kind: 'step_ended', stepId }),
      ],
      sequence,
    );
    assert.equal(client.state.parts.length, 0);
    assert.ok(client.state.watch);
  }
  for (let index = 0; index < 128; index++)
    client.page(
      [
        fact(++sequence, {
          kind: 'part_started',
          messageId: 'active-' + index,
          stepId: 'active-' + index,
          partId: 'part',
          textKind: 'text',
        }),
      ],
      sequence,
    );
  assert.equal(client.state.parts.length, 128);
  client.page(
    [
      fact(++sequence, {
        kind: 'part_started',
        messageId: 'too-many',
        stepId: 'too-many',
        partId: 'part',
        textKind: 'text',
      }),
    ],
    sequence,
  );
  assert.equal(client.state.watch, null);
  assert.equal(client.state.notice, 'unavailable');
});
test('only a complete canonical UTF-8 message becomes Markdown; tail and live excerpts stay selectable literal source', async () => {
  const { component: timeline } = await import(
    pathToFileURL(resolve(root, 'dist/packages/timeline/desktop.js'))
  );
  const text = '**尾部** 中文😀';
  const client = new Client().start({
    through: 100,
    chunks: [historyChunk(text, { offset: 17, totalBytes: 17 + Buffer.byteLength(text) })],
  });
  assert.equal(client.messages[0].continued, true);
  const excerpt = timeline.mount(client.timeline, {});
  const slots = modelSlots(excerpt.frame.view.children[0].children[0].props.content);
  assert.equal(
    slots.find((node) => node.key === 'text-' + client.messages[0].contentKey).binding,
    'source',
  );
  assert.equal(
    slots.some((node) => node.binding === 'markdown'),
    false,
  );
  const complete = timeline.update(
    excerpt.frame.state,
    { ...client.timeline, messages: [{ ...client.messages[0], continued: false }] },
    {},
  );
  assert.equal(
    modelSlots(complete.frame.view.children[0].children[0].props.content).find(
      (node) => node.key === 'text-' + client.messages[0].contentKey,
    ).binding,
    'markdown',
  );
  const landing = timeline.update(
    complete.frame.state,
    {
      ...client.timeline,
      landing: { sequence: client.timeline.landing.sequence + 1, edge: 'start' },
    },
    {},
  );
  assert.equal(landing.frame.view.children[0].children[0].props.command.target.kind, 'start');
  assert.throws(
    () => timeline.update(landing.frame.state, client.timeline, {}),
    /landing sequence/,
  );
});

test('streaming budget never evicts the displayed prefix before explicit canonical navigation', () => {
  const client = new Client().start();
  const part = { messageId: 'answer', stepId: 'step', partId: 'part', textKind: 'text' };
  client.page(
    [
      fact(1, { kind: 'part_started', ...part }),
      fact(2, { kind: 'delta', stepId: 'step', partId: 'part', text: '中文😀'.repeat(5000) }),
    ],
    2,
  );
  const excerpt = client.liveMessages[0].text;
  assert.ok(excerpt.length <= 16000);
  assert.ok(excerpt.isWellFormed());
  client.page(
    [
      fact(3, {
        kind: 'delta',
        stepId: 'step',
        partId: 'part',
        text: 'new bytes beyond this displayed prefix',
      }),
    ],
    3,
  );
  assert.equal(client.liveMessages[0].text, excerpt);
  assert.equal(client.timeline.latest, true);
  assert.equal(client.state.latestKnown, 3);
  assert.equal(client.output.actions.length, 0);
});

test('surrogate truncation closes the streaming prefix before later deltas', () => {
  const client = new Client().start();
  client.page(
    [
      fact(1, {
        kind: 'part_started',
        messageId: 'answer',
        stepId: 'step',
        partId: 'part',
        textKind: 'text',
      }),
      fact(2, {
        kind: 'delta',
        stepId: 'step',
        partId: 'part',
        text: 'a'.repeat(15999) + '😀omitted',
      }),
    ],
    2,
  );
  client.page([fact(3, { kind: 'delta', stepId: 'step', partId: 'part', text: 'Z' })], 3);
  assert.equal(client.liveMessages[0].text, 'a'.repeat(15999));
  assert.ok(client.liveMessages[0].text.isWellFormed());
  assert.equal(client.state.latestKnown, 3);
  assert.equal(client.timeline.latest, true);
});

for (const readyFirst of [false, true]) {
  test(
    'open replay remains bounded while an unrelated canonical row is Ready ' +
      (readyFirst ? 'before' : 'after') +
      ' watch',
    () => {
      const client = new Client();
      const part = {
        startSequence: 1,
        messageId: 'answer',
        stepId: 'step',
        partId: 'part',
        textKind: 'text',
      };
      client.query('page', []);
      client.query('list', { revision: 'catalog', sessions: [session], nextCursor: null });
      client.query('observe', {
        session,
        through: 100,
        run: { invocationId: 'run', state: 'running' },
        parts: [part],
      });
      const ready = {
        kind: 'ready',
        through: 100,
        chunks: [historyChunk('canonical prefix')],
        before: null,
        after: null,
      };
      if (readyFirst) client.query('history', ready);
      client.page(
        [
          fact(1, { kind: 'part_started', ...part }),
          ...Array.from({ length: 80 }, (_, index) =>
            fact(index + 2, {
              kind: 'delta',
              stepId: 'step',
              partId: 'part',
              text: 'old'.repeat(100),
            }),
          ),
          fact(101, { kind: 'delta', stepId: 'step', partId: 'part', text: 'new suffix' }),
        ],
        101,
      );
      assert.ok(
        client.state.live.reduce((sum, fragment) => sum + fragment.text.length, 0) <= 16000,
      );
      if (!readyFirst) client.query('history', ready);
      assert.ok(client.liveMessages[0].text.startsWith('old'));
      assert.ok(client.liveMessages[0].text.length <= 16000);
      assert.equal(client.messages[0].text, 'canonical prefix');
      assert.equal(client.state.latestKnown, 101);
    },
  );
}

for (const target of ['desktop', 'tui']) {
  test(
    target +
      ' whole-message source navigation reaches every UTF-8 byte at one fence without replacing the conversation',
    async () => {
      const module = await import(
        pathToFileURL(resolve(root, 'dist/packages/workspace/' + target + '.js'))
      );
      module.activate({ locale: 'en' });
      const body = '**原文**\n' + '中文😀\n'.repeat(3000);
      const bytes = Buffer.from(body);
      const sequence = 400;
      const chunkAt = (start, end) =>
        historyChunk(bytes.subarray(start, end).toString('utf8'), {
          sequence,
          offset: start,
          totalBytes: bytes.length,
        });
      let tail = bytes.length - 8192;
      while ((bytes[tail] & 0xc0) === 0x80) tail++;
      const client = new Client(module.component).start({
        through: 1000,
        chunks: [chunkAt(tail, bytes.length)],
      });
      const original = structuredClone(client.state.window);
      const landing = structuredClone(client.state.landing);
      const watch = structuredClone(client.output.subscriptions);
      client.component('source', { sequence });
      assert.equal(client.timeline.source.atEnd, true);
      assert.equal(client.timeline.source.atStart, false);
      assert.equal(client.output.actions.length, 0);
      client.page([fact(1001, { kind: 'opened' })], 1001);
      const completePage = () => {
        const action = client.find('history');
        assert.equal(action.input.through, 1000);
        assert.equal(action.input.position.cursor.sequence, sequence);
        const { kind, cursor } = action.input.position;
        let start = kind === 'after' ? cursor.offset : Math.max(0, cursor.offset - 8192);
        let end = kind === 'before' ? cursor.offset : Math.min(bytes.length, start + 8192);
        while ((bytes[start] & 0xc0) === 0x80) start++;
        while (end < bytes.length && (bytes[end] & 0xc0) === 0x80) end--;
        client.complete(action, {
          status: 'succeeded',
          value: {
            kind: 'ready',
            through: 1000,
            chunks: [chunkAt(start, end)],
            before: null,
            after: null,
          },
        });
        assert.ok(client.state.source.chunk.text.isWellFormed());
        assert.ok(Buffer.byteLength(client.state.source.chunk.text) <= 8192);
      };
      client.component('sourceFirst');
      const initial = client.find('history');
      const expected = structuredClone(initial.input);
      const oldPage = structuredClone(client.timeline.source);
      client.complete(initial, {
        status: 'succeeded',
        value: { kind: 'preparing', through: 1000 },
      });
      assert.deepEqual(client.find('history').input, expected);
      assert.equal(client.timeline.source.text, oldPage.text);
      assert.equal(client.timeline.source.navigation, oldPage.navigation);
      completePage();
      const forward = [client.timeline.source.text];
      while (!client.timeline.source.atEnd) {
        client.component('sourceNext');
        completePage();
        forward.push(client.timeline.source.text);
      }
      assert.equal(forward.join(''), body);
      client.component('sourceFirst');
      completePage();
      client.component('sourceLast');
      completePage();
      const reverse = [client.timeline.source.text];
      while (!client.timeline.source.atStart) {
        client.component('sourcePrevious');
        completePage();
        reverse.unshift(client.timeline.source.text);
      }
      assert.equal(reverse.join(''), body);
      assert.deepEqual(client.state.window, original);
      assert.deepEqual(client.state.landing, landing);
      assert.deepEqual(client.output.subscriptions, watch);
      assert.equal(client.state.latestKnown, 1001);
      client.component('sourceBack');
      assert.equal(client.state.source, null);
      assert.deepEqual(client.state.window, original);
      assert.deepEqual(client.state.landing, landing);
      assert.equal(client.timeline.latest, true);
    },
  );
}

test('source errors retain exact original text and retry; closed or reopened readers reject stale replies', () => {
  const sequence = 400;
  const chunk = historyChunk('tail', { sequence, offset: 100, totalBytes: 104 });
  const client = new Client().start({ through: 1000, chunks: [chunk] });
  client.component('source', { sequence });
  client.component('sourceFirst');
  const action = client.find('history');
  client.complete(action, { status: 'failed', reason: 'not available' });
  assert.equal(client.timeline.source.text, 'tail');
  assert.equal(client.timeline.source.failed, true);
  client.component('sourceRetry');
  assert.deepEqual(client.find('history').input, action.input);
  const stale = client.find('history');
  client.component('sourceBack');
  client.component('source', { sequence });
  const reopened = structuredClone(client.state.source);
  client.complete(stale, {
    status: 'succeeded',
    value: {
      kind: 'ready',
      through: 1000,
      chunks: [historyChunk('wrong replacement', { sequence, totalBytes: 104 })],
      before: null,
      after: null,
    },
  });
  assert.deepEqual(client.state.source, reopened);
  client.component('sourceFirst');
  client.query('history', {
    kind: 'ready',
    through: 1000,
    chunks: [historyChunk('neighbor', { sequence: 401, totalBytes: 104 })],
    before: null,
    after: null,
  });
  assert.equal(client.timeline.source.text, 'tail');
  assert.equal(client.timeline.source.failed, true);
  client.component('sourceRetry');
  client.query('history', {
    kind: 'ready',
    through: 1000,
    chunks: [historyChunk('wrong size', { sequence, totalBytes: 105 })],
    before: null,
    after: null,
  });
  assert.equal(client.timeline.source.text, 'tail');
  assert.equal(client.timeline.source.failed, true);
});

test('source entry waits for main navigation and invalidates an older main reply without moving the saved landing', () => {
  const chunk = historyChunk('tail', { sequence: 400, offset: 100, totalBytes: 104 });
  const client = new Client().start({ through: 1000, chunks: [chunk] });
  client.state.window.page.before = { sequence: 400, offset: 100 };
  client.component('older');
  const old = client.find('history');
  client.component('source', { sequence: 400 });
  assert.equal(client.state.source, null);
  // A separate observation failure can settle loading while that history intent remains pending.
  client.state.loading = false;
  const original = structuredClone(client.state.window);
  const landing = structuredClone(client.state.landing);
  client.component('source', { sequence: 400 });
  client.complete(old, {
    status: 'succeeded',
    value: {
      kind: 'ready',
      through: 1000,
      chunks: [historyChunk('older', { sequence: 399 })],
      before: null,
      after: null,
    },
  });
  assert.deepEqual(client.state.window, original);
  assert.deepEqual(client.state.landing, landing);
  client.component('sourceBack');
  assert.deepEqual(client.state.window, original);
  assert.deepEqual(client.state.landing, landing);
});

for (const target of ['desktop', 'tui']) {
  test(
    target +
      ' source presentation retains the conversation scene and keeps navigation outside the reader',
    async () => {
      const { component: timeline } = await import(
        pathToFileURL(resolve(root, 'dist/packages/timeline/' + target + '.js'))
      );
      const client = new Client().start({
        through: 1000,
        chunks: [historyChunk('literal **tail**', { sequence: 400, offset: 100, totalBytes: 116 })],
      });
      const originalProps = structuredClone(client.timeline);
      const original = timeline.mount(originalProps, {});
      client.component('source', { sequence: 400 });
      const opened = timeline.update(original.frame.state, client.timeline, {});
      assert.equal(opened.frame.view.children[0].visible, false);
      assert.deepEqual(
        opened.frame.view.children[0].children,
        original.frame.view.children[0].children,
      );
      const out = timeline.event(
        opened.frame.state,
        { kind: 'raw', key: 'source-first', event: {} },
        client.timeline,
        {},
      );
      assert.deepEqual(out.events, [{ name: 'sourceFirst', value: {} }]);
      assert.equal(out.frame.view.children.at(-1).key, 'reading-controls');
      {
        const reader = modelSlots(out.frame.view).find((node) => node.key === 'source-reading');
        assert.equal(reader.key, 'source-reading');
        const body = modelSlots(reader.props.content).find((node) => node.binding === 'source');
        assert.equal(body.props.source, 'literal **tail**');
        assert.equal(
          modelSlots(reader.props.content).some((node) => node.binding === 'markdown'),
          false,
        );
        const next = timeline.update(
          out.frame.state,
          {
            ...client.timeline,
            source: { ...client.timeline.source, key: 'other-byte-range', text: body.props.source },
          },
          {},
        );
        assert.notEqual(
          modelSlots(
            modelSlots(next.frame.view).find((node) => node.key === 'source-reading').props.content,
          ).find((node) => node.binding === 'source').key,
          body.key,
        );
      }
      client.component('sourceBack');
      const closed = timeline.update(out.frame.state, client.timeline, {});
      assert.equal(closed.frame.view.children[0].visible, true);
      assert.deepEqual(
        closed.frame.view.children[0].children,
        original.frame.view.children[0].children,
      );
    },
  );
}

test('TUI reader declares a flexible body and wrapped controls while keeping draft and navigation routes', async () => {
  const module = await import(pathToFileURL(resolve(root, 'dist/packages/workspace/tui.js')));
  module.activate({ locale: 'en' });
  const { component: timeline } = await import(
    pathToFileURL(resolve(root, 'dist/packages/timeline/tui.js'))
  );
  const client = new Client(module.component).start({
    through: 1000,
    chunks: [
      historyChunk('中文😀\n'.repeat(700), { sequence: 400, offset: 100, totalBytes: 7800 }),
    ],
  });
  const routes = () => client.output.frame.view.children[0].children;
  assert.equal(client.state.navigationOpen, false);
  assert.equal(routes().find((node) => node.key === 'main-route').visible, true);
  const originalTimeline = timeline.mount(client.timeline, {}).frame.view;
  const retained = originalTimeline.children.find((node) => node.key === 'retained-conversation');
  assert.equal(retained.layout.grow, 1);
  assert.equal(retained.children[0].props.content.layout.grow, 1);
  assert.ok(['anchor', 'follow_end'].includes(retained.children[0].props.content.scroll));
  const originalDraft = modelSlots(client.output.frame.view).find(
    (node) => node.binding === 'composer',
  );
  client.component('source', { sequence: 400 });
  const frame = timeline.mount(client.timeline, {}).frame.view;
  assert.equal(frame.layout.grow, 1);
  const reading = modelSlots(frame).find((node) => node.key === 'source-reading');
  assert.equal(reading.binding, 'reading');
  assert.equal(reading.props.content.layout.grow, 1);
  assert.equal(reading.props.content.scroll, 'anchor');
  assert.equal(frame.children.at(-1).layout.wrap, true);
  assert.equal(frame.children.at(-1).gap, 1);
  const main = routes().find((node) => node.key === 'main-route').children[0];
  const composer = main.children.find((node) => node.key === 'retained-composer');
  assert.equal(composer.visible, false);
  assert.deepEqual(modelSlots(composer)[0], originalDraft);
  const originalSource = structuredClone(client.state.source);
  client.raw('navigation-toggle');
  assert.equal(routes().find((node) => node.key === 'main-route').visible, false);
  assert.equal(routes().find((node) => node.key === 'navigation-route').visible, true);
  client.raw('navigation-toggle');
  assert.equal(routes().find((node) => node.key === 'main-route').visible, true);
  assert.deepEqual(client.state.source, originalSource);
  client.component('sourceBack');
  assert.equal(
    routes()
      .find((node) => node.key === 'main-route')
      .children[0].children.find((node) => node.key === 'retained-composer').visible,
    true,
  );
  assert.deepEqual(
    modelSlots(client.output.frame.view).find((node) => node.binding === 'composer'),
    originalDraft,
  );
});

test('all ordinary TUI node identities remain unique including hidden routes', async () => {
  const module = await import(pathToFileURL(resolve(root, 'dist/packages/workspace/tui.js')));
  module.activate({ locale: 'en' });
  const client = new Client(module.component).start();
  const check = () => {
    const keys = new Set();
    const visit = (node) => {
      assert.equal(keys.has(node.key), false, node.key);
      keys.add(node.key);
      for (const child of node.children ?? []) visit(child);
    };
    visit(client.output.frame.view);
  };
  check();
  client.raw('navigation-toggle');
  check();
  client.raw('navigation-back');
  check();
});

for (const target of ['desktop', 'tui']) {
  test(target + ' empty ordinary canonical messages keep their visible source action', async () => {
    const { component: timeline } = await import(
      pathToFileURL(resolve(root, 'dist/packages/timeline/' + target + '.js'))
    );
    for (const role of ['user', 'assistant']) {
      const client = new Client().start({
        through: 100,
        chunks: [historyChunk('', { role, sequence: 40 })],
      });
      const out = timeline.mount(client.timeline, {});
      const row = out.frame.view.children[0].children[0].props.content.children[0];
      assert.equal(
        row.children.some((n) => n.children?.some((c) => c.key === 'source-40')),
        true,
      );
      const opened = timeline.event(
        out.frame.state,
        { kind: 'raw', key: 'source-40', event: {} },
        client.timeline,
        {},
      );
      assert.deepEqual(opened.events, [{ name: 'source', value: { sequence: 40 } }]);
    }
  });
  test(target + ' tool inspection has one disclosure and keeps hidden actions inert', async () => {
    const { component: timeline } = await import(
      pathToFileURL(resolve(root, 'dist/packages/timeline/' + target + '.js'))
    );
    const visibleButtons = (node) =>
      node.visible === false
        ? []
        : [
            ...(node.kind === 'button' ? [node] : []),
            ...(node.children ?? []).flatMap(visibleButtons),
          ];
    for (const role of ['tool_call', 'tool_result']) {
      for (const bodyText of ['', 'Literal tool output **中🙂**']) {
        const client = new Client().start({
          through: 100,
          chunks: [
            historyChunk(bodyText, {
              sequence: 40,
              role,
              tool: {
                kind: role === 'tool_call' ? 'call' : 'result',
                toolUseId: uuid(901),
                toolName: 'Draw',
                ...(role === 'tool_result' ? { isError: true, image: true } : {}),
              },
            }),
          ],
        });
        const props = client.timeline;
        let out = timeline.mount(props, {});
        const row = () => out.frame.view.children[0].children[0].props.content.children[0];
        const originalBody = structuredClone(
          row().children.find((node) => node.key.startsWith('details-')).children[0],
        );
        const input = (key) => {
          out = timeline.event(
            out.frame.state,
            {
              kind: 'component',
              child: 'reading',
              name: 'input',
              key: 'reading-input',
              event: { kind: 'raw', key, event: { kind: 'click' } },
            },
            props,
            {},
          );
          return out.events;
        };
        const toggle = 'toggle-' + client.messages[0].id;
        assert.deepEqual(
          visibleButtons(row()).map((n) => n.key),
          [toggle],
        );
        for (const hidden of ['tool-source-40', 'image-40', 'source-40'])
          assert.deepEqual(input(hidden), [], 'a collapsed inspector has no active actions');
        input(toggle);
        const expanded = visibleButtons(row());
        assert.equal(
          expanded.find((n) => n.key === 'tool-source-40').label,
          role === 'tool_call' ? 'Tool arguments' : 'Tool result',
        );
        assert.deepEqual(input('tool-source-40'), [
          { name: 'toolSource', value: { sequence: 40 } },
        ]);
        assert.deepEqual(
          input('image-40'),
          role === 'tool_result' ? [{ name: 'viewImage', value: { sequence: 40 } }] : [],
        );
        assert.deepEqual(
          input('source-40'),
          bodyText ? [{ name: 'source', value: { sequence: 40 } }] : [],
        );
        const loading = timeline.update(out.frame.state, { ...props, loading: true }, {});
        const loadingRow = loading.frame.view.children[0].children[0].props.content.children[0];
        for (const action of visibleButtons(loadingRow).filter((n) => n.key !== toggle))
          assert.equal(action.disabled, true);
        input(toggle);
        assert.deepEqual(
          visibleButtons(row()).map((n) => n.key),
          [toggle],
        );
        assert.deepEqual(
          row().children.find((node) => node.key.startsWith('details-')).children[0],
          originalBody,
          'disclosure preserves the original SourceText declaration',
        );
      }
    }
  });
  test(
    target +
      ' tool text stays mounted while collapsed and expands through its checked reading input',
    async () => {
      const { component: timeline } = await import(
        pathToFileURL(resolve(root, 'dist/packages/timeline/' + target + '.js'))
      );
      const client = new Client().start({
        through: 100,
        chunks: [
          historyChunk('line 1\n  **literal** 中😀\n' + 'long output '.repeat(100), {
            role: 'tool_result',
            tool: null,
            sequence: 40,
          }),
        ],
      });
      const props = { ...client.timeline, locale: 'zh-CN' };
      let out = timeline.mount(props, {});
      const row = () => out.frame.view.children[0].children[0].props.content.children[0];
      const details = () => row().children.find((node) => node.key === 'details-' + uuid(900));
      const source = structuredClone(details().children[0]);
      assert.equal(row().children[0].children[0].text, '工具输出');
      assert.equal(details().visible, false);
      assert.ok(
        row().children.find((node) => node.key === 'summary-' + uuid(900)).text.length < 170,
      );
      assert.equal(source.binding, 'source');
      assert.equal(source.props.source, client.messages[0].text);
      const toggle = () => {
        out = timeline.event(
          out.frame.state,
          {
            kind: 'component',
            child: 'reading',
            name: 'input',
            key: 'reading-input',
            event: { kind: 'raw', key: 'toggle-' + uuid(900), event: { kind: 'click' } },
          },
          props,
          {},
        );
      };
      toggle();
      assert.equal(details().visible, true);
      assert.deepEqual(details().children[0], source);
      assert.equal(out.events.length, 0);
      assert.equal(out.actions.length, 0);
      toggle();
      assert.equal(details().visible, false);
      assert.deepEqual(details().children[0], source);
      out = timeline.update(out.frame.state, { ...props, messages: [] }, {});
      assert.deepEqual(out.frame.state.expanded, []);
    },
  );
  test(
    target +
      ' identical text at another canonical byte offset creates a different native body identity',
    async () => {
      const { component: timeline } = await import(
        pathToFileURL(resolve(root, 'dist/packages/timeline/' + target + '.js'))
      );
      const first = new Client().start({
        through: 100,
        chunks: [
          historyChunk('same bytes', {
            role: 'tool_call',
            tool: null,
            sequence: 40,
            offset: 0,
            totalBytes: 200,
          }),
        ],
      });
      const next = new Client().start({
        through: 100,
        chunks: [
          historyChunk('same bytes', {
            role: 'tool_call',
            tool: null,
            sequence: 40,
            offset: 80,
            totalBytes: 200,
          }),
        ],
      });
      const before = timeline.mount(first.timeline, {});
      const expanded = timeline.event(
        before.frame.state,
        { kind: 'raw', key: 'toggle-' + uuid(900), event: {} },
        first.timeline,
        {},
      );
      const after = timeline.update(expanded.frame.state, next.timeline, {});
      const row = (out) => out.frame.view.children[0].children[0].props.content.children[0];
      const body = (out) =>
        row(out).children.find((node) => node.key === 'details-' + uuid(900)).children[0];
      assert.equal(row(expanded).key, row(after).key);
      assert.equal(body(expanded).props.source, body(after).props.source);
      assert.notEqual(body(expanded).key, body(after).key);
      assert.equal(
        row(after).children.find((node) => node.key === 'details-' + uuid(900)).visible,
        true,
      );
      const sourceRoute = {
        ...next.timeline,
        source: {
          key: 'source-page',
          text: 'source',
          atStart: true,
          atEnd: true,
          loading: false,
          failed: false,
          navigation: 1,
          landing: 'start',
        },
      };
      const hidden = timeline.update(after.frame.state, sourceRoute, {});
      const staleToggle = timeline.event(
        hidden.frame.state,
        { kind: 'raw', key: 'toggle-' + uuid(900), event: {} },
        sourceRoute,
        {},
      );
      assert.deepEqual(staleToggle.frame.state.expanded, hidden.frame.state.expanded);
    },
  );
}

for (const target of ['desktop', 'tui']) {
  test(
    target +
      ' automatic history follow waits for native landing, coalesces fences and keeps stale reads optional',
    async () => {
      const module = await import(
        pathToFileURL(resolve(root, 'dist/packages/workspace/' + target + '.js'))
      );
      module.activate({ locale: 'en' });
      const client = new Client(module.component).start({
        through: 10,
        chunks: [historyChunk('Original')],
      });
      const original = structuredClone(client.state.window);
      const landing = structuredClone(client.state.landing);
      client.page([fact(11, { kind: 'opened' })], 11);
      const pending = client.find('history');
      assert.deepEqual(pending.continuations, [{ child: 'timeline', method: 'tail', input: null }]);
      assert.equal(client.state.loading, false);
      assert.deepEqual(client.state.window, original);
      assert.deepEqual(client.state.landing, landing);
      client.complete(pending, { status: 'succeeded', value: { kind: 'preparing', through: 11 } });
      const retry = client.find('history');
      assert.deepEqual(retry.input, pending.input);
      assert.deepEqual(retry.continuations, pending.continuations);
      client.page([fact(12, { kind: 'transcript_changed' })], 12);
      assert.equal(client.output.actions.length, 0);
      client.complete(retry, {
        status: 'succeeded',
        value: {
          kind: 'ready',
          through: 11,
          chunks: [historyChunk('First')],
          before: null,
          after: null,
        },
      });
      assert.equal(
        client.output.actions.length,
        0,
        'new End must paint before capturing another witness',
      );
      client.page([fact(13, { kind: 'transcript_changed' })], 13);
      assert.equal(client.output.actions.length, 0, 'watch before End paint must also wait');
      client.component('tailReady');
      const catchup = client.find('history');
      assert.equal(catchup.input.through, 13);
      const current = structuredClone(client.state.window);
      const currentLanding = structuredClone(client.state.landing);
      client.complete(catchup, { status: 'stale' });
      assert.deepEqual(client.state.window, current);
      assert.deepEqual(client.state.landing, currentLanding);
      assert.equal(client.state.loading, false);
      assert.equal(client.state.notice, null);
      assert.equal(client.timeline.latest, true);
      assert.equal(client.output.actions.length, 0, 'stale does not retry itself');
      client.page([], 13);
      assert.equal(client.output.actions.length, 0);
      client.component('tailReady');
      assert.equal(client.output.actions.length, 0, 'same request/paint basis does not retry');
      client.component('tailReady', { landing: client.state.landing.sequence, readingRevision: 1 });
      assert.equal(client.find('history').input.through, 13);
      client.complete(client.find('history'), { status: 'not_accepted', reason: 'fixture' });
      client.component('latest');
      assert.deepEqual(client.find('history').continuations, []);
      client.query('history', {
        kind: 'ready',
        through: 13,
        chunks: [historyChunk('Latest')],
        before: null,
        after: null,
      });
      assert.equal(client.messages[0].text, 'Latest');
    },
  );

  test(
    target +
      ' atomic text survives part finish and unrelated canonical history until its real step closure',
    async () => {
      const module = await import(
        pathToFileURL(resolve(root, 'dist/packages/workspace/' + target + '.js'))
      );
      module.activate({ locale: 'en' });
      const client = new Client(module.component).start();
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
          fact(3, { kind: 'delta', stepId: 'step', partId: 'part', text: 'prefix ' }),
        ],
        3,
      );
      client.query('history', {
        kind: 'ready',
        through: 3,
        chunks: [historyChunk('User', { role: 'user' })],
        before: null,
        after: null,
      });
      assert.equal(client.liveMessages[0].text, 'prefix ');
      client.component('tailReady');
      client.page(
        [
          fact(4, { kind: 'delta', stepId: 'step', partId: 'part', text: 'suffix' }),
          fact(5, { kind: 'part_finished', stepId: 'step', partId: 'part' }),
        ],
        5,
      );
      client.query('history', { kind: 'ready', through: 5, chunks: [], before: null, after: null });
      assert.equal(
        client.liveMessages[0].text,
        'prefix suffix',
        'PartFinished is not atomic provider publication',
      );
      assert.equal(client.state.parts.length, 0);
      client.component('tailReady');
      client.page([fact(6, { kind: 'step_ended', stepId: 'step' })], 6);
      client.query('history', {
        kind: 'ready',
        through: 6,
        chunks: [historyChunk('prefix suffix', { messageId: 'answer' })],
        before: null,
        after: null,
      });
      assert.equal(client.liveMessages.length, 0);
      assert.equal(client.state.live.length, 0);
      assert.equal(client.messages[0].text, 'prefix suffix');
    },
  );

  test(
    target +
      ' timeline forwards tail capture and schedules catchup only from settled original conversation paint',
    async () => {
      const { component: timeline } = await import(
        pathToFileURL(resolve(root, 'dist/packages/timeline/' + target + '.js'))
      );
      const props = new Client().start().timeline;
      let out = timeline.mount(props, {});
      assert.deepEqual(timeline.capture(out.frame.state, props, 'tail', null, {}), {
        kind: 'forward',
        source: { child: 'reading', method: 'tail', input: null },
      });
      const observed = (child, pendingCommand, status = 'applied') =>
        timeline.event(
          structuredClone(out.frame.state),
          {
            kind: 'component',
            child,
            key: 'observed',
            name: 'observed',
            event: {
              readingRevision: 0,
              viewport: { readingRevision: 0, atEnd: true, visibleChildKeys: [] },
              pendingCommand,
              receipt: out.frame.state.command
                ? { sequence: out.frame.state.command.sequence, status }
                : null,
            },
          },
          props,
          {},
        );
      assert.deepEqual(observed('reading', 1).events, []);
      assert.deepEqual(observed('source-reading', null).events, []);
      out = observed('reading', null);
      assert.deepEqual(out.events, [
        { name: 'tailReady', value: { landing: props.landing.sequence, readingRevision: 0 } },
      ]);
      assert.deepEqual(observed('reading', null, 'cancelled').events, out.events);
      assert.deepEqual(observed('reading', null, 'rejected').events, out.events);
    },
  );
}

for (const target of ['desktop', 'tui']) {
  test(
    target + ' late step closure preserves an already displayed source until guarded publication',
    async () => {
      const module = await import(
        pathToFileURL(resolve(root, 'dist/packages/workspace/' + target + '.js'))
      );
      module.activate({ locale: 'en' });
      const client = new Client(module.component).start();
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
          fact(3, { kind: 'delta', stepId: 'step', partId: 'part', text: 'Selectable original' }),
        ],
        3,
      );
      // Independent observe can be ahead of queued watch delivery.
      client.component('stop');
      client.complete(client.find('stop'), returned({ requested: true }));
      client.query('observe', { session, through: 100, run: null, parts: [] });
      client.component('latest');
      client.query('history', {
        kind: 'ready',
        through: 100,
        chunks: [],
        before: null,
        after: null,
      });
      const original = structuredClone(client.liveMessages);
      client.component('tailReady');
      client.page([fact(90, { kind: 'step_ended', stepId: 'step' })], 100);
      assert.deepEqual(
        client.liveMessages,
        original,
        'watch closure cannot retire selected SourceText',
      );
      assert.equal(
        client.timeline.latest,
        true,
        'a covered closure can still need guarded refresh',
      );
      const pending = client.find('history');
      assert.deepEqual(pending.continuations, [{ child: 'timeline', method: 'tail', input: null }]);
      client.complete(pending, { status: 'stale' });
      assert.deepEqual(client.liveMessages, original);
      client.component('latest');
      client.query('history', {
        kind: 'ready',
        through: 100,
        chunks: [],
        before: null,
        after: null,
      });
      assert.equal(client.liveMessages.length, 0, 'explicit navigation may retire the old prefix');
    },
  );
}

for (const target of ['desktop', 'tui']) {
  test(
    target + ' tool fields read literal JSON at one fence with retry and bidirectional paging',
    async () => {
      const module = await import(
        pathToFileURL(resolve(root, 'dist/packages/workspace/' + target + '.js'))
      );
      module.activate({ locale: 'en' });
      const body = JSON.stringify({
        count: 42,
        ready: false,
        empty: null,
        text: '中文😀\u0000\r\n'.repeat(1000),
      });
      const bytes = Buffer.from(body);
      const tool = { kind: 'call', toolUseId: uuid(902), toolName: 'Count' };
      const client = new Client(module.component).start({
        through: 1000,
        chunks: [
          historyChunk('', { sequence: 400, role: 'tool_call', tool }),
          historyChunk('', {
            sequence: 401,
            role: 'tool_result',
            tool: { ...tool, kind: 'result', isError: false, image: false },
          }),
        ],
      });
      const window = structuredClone(client.state.window);
      const landing = structuredClone(client.state.landing);
      client.component('toolSource', { sequence: 400 });
      assert.equal(client.timeline.source.kind, 'arguments');
      assert.equal(client.timeline.source.ready, false);
      assert.equal(client.timeline.source.text, '');
      const first = client.find('toolSource');
      assert.deepEqual(first.input, {
        sessionId: session.id,
        through: 1000,
        sequence: 400,
        field: 'arguments',
        position: { kind: 'start' },
      });
      client.query('toolSource', { kind: 'preparing', through: 1000 });
      assert.deepEqual(client.find('toolSource').input, first.input);
      client.complete(client.find('toolSource'), { status: 'failed', reason: 'unavailable' });
      assert.equal(client.timeline.source.failed, true);
      assert.equal(client.timeline.source.ready, false);
      client.component('sourceRetry');
      assert.deepEqual(client.find('toolSource').input, first.input);
      const complete = () => {
        const action = client.find('toolSource');
        assert.equal(action.input.through, 1000);
        const position = action.input.position;
        const reverse = ['before', 'tail'].includes(position.kind);
        let end = reverse
          ? position.kind === 'tail'
            ? bytes.length
            : position.offset
          : Math.min(bytes.length, (position.offset ?? 0) + 8192);
        let start = reverse ? Math.max(0, end - 8192) : (position.offset ?? 0);
        while ((bytes[start] & 0xc0) === 0x80) start++;
        while (end < bytes.length && (bytes[end] & 0xc0) === 0x80) end--;
        client.complete(action, {
          status: 'succeeded',
          value: {
            kind: 'ready',
            through: 1000,
            source: {
              sequence: 400,
              field: 'arguments',
              format: 'json',
              offset: start,
              totalBytes: bytes.length,
              text: bytes.subarray(start, end).toString('utf8'),
              tool,
            },
          },
        });
        assert.equal(client.timeline.source.ready, true);
        assert.equal(client.timeline.source.format, 'json');
        assert.ok(client.timeline.source.text.isWellFormed());
      };
      complete();
      client.page([fact(1001, { kind: 'opened' })], 1001);
      const forward = [client.timeline.source.text];
      while (!client.timeline.source.atEnd) {
        client.component('sourceNext');
        const old = client.timeline.source.text;
        client.complete(client.find('toolSource'), { status: 'failed', reason: 'interrupted' });
        assert.equal(client.timeline.source.text, old);
        client.component('sourceRetry');
        complete();
        forward.push(client.timeline.source.text);
      }
      assert.equal(forward.join(''), body);
      client.component('sourceLast');
      complete();
      const reverse = [client.timeline.source.text];
      while (!client.timeline.source.atStart) {
        client.component('sourcePrevious');
        complete();
        reverse.unshift(client.timeline.source.text);
      }
      assert.equal(reverse.join(''), body);
      const argumentKey = client.timeline.source.key;
      client.component('sourceNext');
      const stale = client.find('toolSource');
      client.component('sourceBack');
      client.component('toolSource', { sequence: 401 });
      const reopened = structuredClone(client.state.source);
      client.complete(stale, {
        status: 'succeeded',
        value: {
          kind: 'ready',
          through: 1000,
          source: {
            sequence: 400,
            field: 'arguments',
            format: 'json',
            offset: 0,
            totalBytes: 2,
            text: '{}',
            tool,
          },
        },
      });
      assert.deepEqual(client.state.source, reopened);
      client.query('toolSource', {
        kind: 'ready',
        through: 1000,
        source: {
          sequence: 401,
          field: 'result',
          format: 'json',
          offset: 0,
          totalBytes: 2,
          text: '{}',
          tool: { ...tool, kind: 'result', isError: false, image: false },
        },
      });
      assert.equal(client.timeline.source.kind, 'result');
      assert.notEqual(client.timeline.source.key, argumentKey);
      client.component('sourceBack');
      assert.deepEqual(client.state.window, window);
      assert.deepEqual(client.state.landing, landing);
      assert.equal(client.state.latestKnown, 1001);
    },
  );
}

async function imageClient(target = 'desktop', overrides = {}) {
  const { component } = await import(
    pathToFileURL(resolve(root, 'dist/packages/image-viewer/' + target + '.js'))
  );
  let props = {
    theme: values.appearance[0],
    locale: 'en',
    active: true,
    image: { sessionId: 'session', through: 100, sequence: 40 },
    ...overrides,
  };
  let out = component.mount(props, {});
  return {
    get out() {
      return out;
    },
    get state() {
      return out.frame.state;
    },
    update(change) {
      props = { ...props, ...change };
      out = component.update(structuredClone(out.frame.state), props, {});
      return out;
    },
    event(input) {
      out = component.event(structuredClone(out.frame.state), input, props, {});
      return out;
    },
    answer(action, page) {
      return this.event({
        kind: 'action',
        key: action.key,
        event: { status: 'succeeded', value: page },
      });
    },
    observe(change = {}) {
      return this.event({
        kind: 'component',
        key: 'image-observed',
        child: 'pixels',
        name: 'observed',
        event: {
          owner: 'native-original',
          revision: (this.state.observation?.revision ?? 0) + 1,
          source: this.state.descriptor?.key ?? null,
          received: this.state.received,
          phase: 'receiving',
          width: null,
          height: null,
          error: null,
          receipt: null,
          ...change,
        },
      });
    },
  };
}
function imagePage(offset, total = 5 * 32768 + 3, overrides = {}) {
  return {
    kind: 'ready',
    through: 100,
    sequence: 40,
    offset,
    totalBytes: total,
    mimeType: 'image/png',
    digest: 'sha256:' + 'a'.repeat(64),
    data: Buffer.alloc(Math.min(32768, total - offset), 42).toString('base64'),
    ...overrides,
  };
}
function imageOccupancy(state) {
  return (
    Object.keys(state.pending).length +
    state.waiting.length +
    state.queue.length +
    (state.packet ? 1 : 0)
  );
}
test('image viewer keeps four bounded chunks, ordered delivery and original-owner ACK', async () => {
  const client = await imageClient();
  const first = client.out.actions[0];
  assert.deepEqual(first.input, { sessionId: 'session', through: 100, sequence: 40, offset: 0 });
  client.answer(first, imagePage(0));
  const prefetch = client.out.actions;
  assert.equal(prefetch.length, 3);
  assert.equal(imageOccupancy(client.state), 4);
  client.observe();
  const original = client.state.packet;
  assert.equal(original.offset, 0);
  for (const action of [...prefetch].reverse())
    client.answer(action, imagePage(action.input.offset));
  assert.equal(imageOccupancy(client.state), 4);
  assert.deepEqual(client.state.packet, original);
  client.observe({
    receipt: {
      owner: 'different',
      source: original.source,
      sequence: original.sequence,
      accepted: true,
      error: null,
    },
  });
  assert.deepEqual(client.state.packet, original);
  const ack = () => {
    const packet = client.state.packet;
    return client.observe({
      receipt: {
        owner: packet.owner,
        source: packet.source,
        sequence: packet.sequence,
        accepted: true,
        error: null,
      },
    });
  };
  ack();
  assert.equal(client.state.packet.offset, 32768);
  assert.equal(client.out.actions.length, 1);
  assert.equal(imageOccupancy(client.state), 4);
  const next = client.out.actions[0];
  client.update({ active: false });
  ack();
  assert.equal(client.state.packet, null);
  assert.equal(client.out.actions.length, 0);
  client.answer(next, imagePage(next.input.offset));
  assert.equal(client.out.actions.length, 0);
  assert.equal(imageOccupancy(client.state), 3);
  client.update({ active: true });
  assert.equal(client.state.packet.offset, 65536);
  assert.equal(imageOccupancy(client.state), 4);
});
test('image viewer pauses preparing requests while hidden and ignores old source replies', async () => {
  const client = await imageClient();
  const original = client.out.actions[0];
  client.update({ active: false });
  client.answer(original, { kind: 'preparing', through: 100 });
  assert.equal(client.out.actions.length, 0);
  assert.equal(client.state.waiting.length, 1);
  client.update({ active: true });
  const retry = client.out.actions[0];
  assert.deepEqual(retry.input, original.input);
  client.answer(retry, imagePage(0));
  const old = client.out.actions;
  client.update({ image: null });
  assert.equal(client.state.descriptor, null);
  client.update({ image: { sessionId: 'different', through: 200, sequence: 140 } });
  assert.equal(client.out.actions.length, 0);
  for (const action of old) client.answer(action, imagePage(action.input.offset));
  assert.equal(client.state.descriptor, null);
  const fresh = client.out.actions[0];
  assert.deepEqual(fresh.input, { sessionId: 'different', through: 200, sequence: 140, offset: 0 });
});
test('image errors stop prefetch, preserve original fence on retry and reject mixed metadata', async () => {
  const client = await imageClient();
  client.answer(client.out.actions[0], imagePage(0));
  const old = client.out.actions;
  client.observe({ phase: 'failed', error: 'unsupported_format' });
  assert.equal(client.state.failed, true);
  assert.equal(client.out.actions.length, 0);
  client.event({ kind: 'raw', key: 'image-retry', event: {} });
  assert.equal(client.state.failed, false);
  assert.equal(client.state.descriptor, null);
  for (const action of old) client.answer(action, imagePage(action.input.offset));
  assert.deepEqual(client.out.actions[0].input, {
    sessionId: 'session',
    through: 100,
    sequence: 40,
    offset: 0,
  });
  client.answer(client.out.actions[0], imagePage(0));
  const next = client.out.actions[0];
  client.answer(
    next,
    imagePage(next.input.offset, 5 * 32768 + 3, { digest: 'sha256:' + 'b'.repeat(64) }),
  );
  assert.equal(client.state.failed, true);
  assert.equal(client.out.actions.length, 0);
});
test('image semantic TUI has no pixel loader and Desktop back retains conversation mount', async () => {
  const terminal = await imageClient('tui');
  assert.equal(terminal.out.actions.length, 0);
  assert.equal(modelSlots(terminal.out.frame.view).length, 0);
  assert.match(JSON.stringify(terminal.out.frame.view), /desktop app/);
  const client = new Client().start({
    through: 100,
    chunks: [
      historyChunk('', {
        sequence: 40,
        role: 'tool_result',
        tool: {
          kind: 'result',
          toolUseId: uuid(901),
          toolName: 'Draw',
          isError: false,
          image: true,
        },
      }),
    ],
  });
  const before = client.timeline;
  client.input({
    kind: 'component',
    key: 'view-image',
    child: 'timeline',
    name: 'viewImage',
    event: { sequence: 40 },
  });
  const slots = modelSlots(client.output.frame.view);
  assert.deepEqual(slots.find((s) => s.key === 'image-viewer').props.image, {
    sessionId: 'session',
    through: 100,
    sequence: 40,
  });
  assert.deepEqual(client.timeline, before);
  client.input({
    kind: 'component',
    key: 'image-closed',
    child: 'image-viewer',
    name: 'closed',
    event: {},
  });
  assert.equal(client.output.frame.state.image, null);
  assert.deepEqual(client.timeline, before);
});

test('returning from an image follows new canonical facts, including an in-flight hidden rejection', () => {
  for (const inFlight of [false, true]) {
    const client = new Client().start({
      through: 100,
      chunks: [
        historyChunk('', {
          sequence: 40,
          role: 'tool_result',
          tool: {
            kind: 'result',
            toolUseId: uuid(901),
            toolName: 'Draw',
            isError: false,
            image: true,
          },
        }),
      ],
    });
    const oldWindow = structuredClone(client.state.window);
    if (inFlight) client.page([fact(101, { kind: 'transcript_changed' })], 101);
    const admitted = inFlight ? client.find('history') : null;
    client.input({
      kind: 'component',
      key: 'view-image',
      child: 'timeline',
      name: 'viewImage',
      event: { sequence: 40 },
    });
    if (admitted) client.complete(admitted, { status: 'stale' });
    else client.page([fact(101, { kind: 'transcript_changed' })], 101);
    assert.equal(client.output.actions.length, 0);
    assert.deepEqual(client.state.window, oldWindow);
    client.input({
      kind: 'component',
      key: 'image-closed',
      child: 'image-viewer',
      name: 'closed',
      event: {},
    });
    const follow = client.output.actions;
    assert.equal(follow.length, 1);
    assert.equal(follow[0].input.through, 101);
    assert.deepEqual(follow[0].continuations, [{ child: 'timeline', method: 'tail', input: null }]);
    assert.deepEqual(client.state.window, oldWindow);
    client.complete(follow[0], {
      status: 'succeeded',
      value: {
        kind: 'ready',
        through: 101,
        chunks: [historyChunk('Completed')],
        before: null,
        after: null,
      },
    });
    assert.equal(client.state.window.page.through, 101);
  }
});

test('editor-local submission uses latest observation and the original send gates', async () => {
  for (const target of ['desktop', 'tui']) {
    const { component: composer } = await import(
      pathToFileURL(resolve(root, `dist/packages/composer/${target}.js`))
    );
    const props = {
      theme: values.appearance[0],
      locale: 'en',
      enabled: true,
      busy: false,
      running: false,
      command: null,
    };
    let result = composer.mount(props, {});
    assert.equal(
      modelSlots(result.frame.view).find((n) => n.key === 'draft').props.submit,
      target === 'desktop' ? 'primary_enter' : 'control_enter',
      'the first native edit can submit before a changed observation is published',
    );
    const submit = (observation, configuration = props) =>
      composer.event(
        result.frame.state,
        { kind: 'component', name: 'submitted', key: 'submitted', event: observation },
        configuration,
        {},
      );
    result = submit(draft);
    assert.deepEqual(result.events, [{ name: 'send', value: draft }]);
    const slot = modelSlots(result.frame.view).find((n) => n.key === 'draft');
    assert.equal(slot.props.submit, target === 'desktop' ? 'primary_enter' : 'control_enter');
    const latest = {
      ...draft,
      revision: 5,
      snapshot: { ...draft.snapshot, text: { ...draft.snapshot.text, revision: 5, bytes: 8 } },
    };
    assert.deepEqual(submit(latest).events, [{ name: 'send', value: latest }]);
    for (const override of [
      { busy: true },
      { running: true },
      { enabled: false },
      {
        command: {
          owner: 'original-editor',
          sequence: 9,
          basedOn: 4,
          edit: { kind: 'reset', text: '' },
        },
      },
    ]) {
      assert.deepEqual(submit(latest, { ...props, ...override }).events, []);
    }
    assert.deepEqual(
      submit({ ...latest, snapshot: { ...latest.snapshot, marked: { start: 0, end: 2 } } }).events,
      [],
    );
    assert.deepEqual(
      submit({
        ...latest,
        snapshot: { ...latest.snapshot, text: { ...latest.snapshot.text, bytes: 0 } },
      }).events,
      [],
    );
  }
});

for (const target of ['desktop', 'tui']) {
  test(`${target} deployment paging keeps the exact source and changes navigation only after success`, async () => {
    const { component } = await import(
      pathToFileURL(resolve(root, `dist/packages/deployment/${target}.js`))
    );
    const props = { theme: values.appearance[0], locale: 'en' };
    const source = { stateDomain: 'a'.repeat(64), deployment: uuid(80), revision: 7 };
    const item = (n) => ({
      instance: [uuid(n)],
      path: ['component-' + n],
      package: { id: 'component.' + n, revision: 'sha256-' + 'b'.repeat(64) },
      structural: false,
      disabled: false,
      parentDisabled: false,
    });
    let result = component.mount(props, {});
    const complete = (value, status = 'succeeded') => {
      const key = result.actions[0].key;
      result = component.event(
        result.frame.state,
        {
          kind: 'action',
          key,
          event:
            status === 'succeeded' ? { status, value } : { status: 'failed', message: 'transport' },
        },
        props,
        {},
      );
    };
    const press = (key) =>
      (result = component.event(result.frame.state, { kind: 'raw', key, event: null }, props, {}));
    complete({ source, instances: [item(1)], next: [uuid(1)] });
    press('deployment-next');
    assert.deepEqual(result.actions[0].input, { kind: 'continue', source, after: [uuid(1)] });
    const original = result.frame.state.page;
    complete(null, 'failed');
    assert.deepEqual(result.frame.state.page, original);
    assert.deepEqual(result.frame.state.previous, []);
    assert.equal(result.frame.state.after, null);
    press('deployment-next');
    complete({ source, instances: [item(2)], next: null });
    assert.deepEqual(result.frame.state.previous, [null]);
    press('deployment-previous');
    assert.deepEqual(result.actions[0].input, { kind: 'continue', source, after: null });
    complete({ source: { ...source, revision: 8 }, instances: [item(1)], next: [uuid(1)] });
    assert.deepEqual(result.frame.state.page.instances, [item(2)]);
    assert.deepEqual(result.frame.state.previous, [null]);
    press('deployment-previous');
    complete({ source, instances: [item(1)], next: [uuid(1)] });
    assert.deepEqual(result.frame.state.previous, []);
    assert.equal(result.frame.state.page.source.revision, 7);
  });
  test(`${target} deployment confirmation uses one CAS intent and unknown checks only the original operation`, async () => {
    const { component } = await import(
      pathToFileURL(resolve(root, `dist/packages/deployment/${target}.js`))
    );
    const props = { theme: values.appearance[0], locale: 'en' };
    const source = { stateDomain: 'a'.repeat(64), deployment: uuid(80), revision: 7 };
    const instance = {
      instance: [uuid(1)],
      path: ['optional'],
      package: { id: 'optional', revision: 'sha256-' + 'b'.repeat(64) },
      structural: false,
      disabled: false,
      parentDisabled: false,
    };
    let result = component.mount(props, {});
    const complete = (outcome) => {
      const key = result.actions[0].key;
      result = component.event(
        result.frame.state,
        { kind: 'action', key, event: outcome },
        props,
        {},
      );
    };
    const press = (key) =>
      (result = component.event(result.frame.state, { kind: 'raw', key, event: null }, props, {}));
    complete({ status: 'succeeded', value: { source, instances: [instance], next: null } });
    press('deployment-toggle-0');
    assert.equal(result.actions.length, 0);
    press('deployment-cancel');
    assert.equal(result.frame.state.confirmation, null);
    press('deployment-toggle-0');
    press('deployment-confirm');
    assert.equal(result.actions[0].method, 'setEnabled');
    assert.deepEqual(result.actions[0].input, {
      expectedRevision: 7,
      instance: instance.instance,
      enabled: false,
    });
    const original = {
      ...reference,
      source: { ...source, deployment: uuid(81) },
      namespace: [uuid(90)],
      export: 'Management',
      method: 'setEnabled',
      contract: sdk.contracts.deploymentManager,
    };
    complete({ status: 'operation', reply: { reference: original, receipt: { kind: 'unknown' } } });
    assert.ok(result.frame.state.current);
    press('deployment-toggle-0');
    assert.equal(result.actions.length, 0);
    press('deployment-refresh');
    assert.equal(result.actions.length, 0);
    press('deployment-check');
    assert.equal(result.actions[0].method, 'read');
    assert.deepEqual(result.actions[0].input, original);
    complete({
      status: 'succeeded',
      value: {
        reference: original,
        receipt: {
          kind: 'recorded',
          outcome: { kind: 'returned', value: { status: 'conflict', currentRevision: 8 } },
          cleanup: { kind: 'confirmed' },
        },
      },
    });
    assert.equal(result.frame.state.current, null);
    assert.equal(result.frame.state.conflict, true);
    press('deployment-toggle-0');
    assert.equal(result.frame.state.confirmation, null);
    press('deployment-refresh');
    assert.deepEqual(result.actions[0].input, { kind: 'start' });
    complete({
      status: 'succeeded',
      value: {
        source: { ...source, revision: 8 },
        instances: [{ ...instance, disabled: true }],
        next: null,
      },
    });
    press('deployment-toggle-0');
    press('deployment-confirm');
    assert.deepEqual(result.actions[0].input, {
      expectedRevision: 8,
      instance: instance.instance,
      enabled: true,
    });
  });
}

for (const target of ['desktop', 'tui']) {
  test(`${target} composition editor loads all chunks through original native receipts and captures one confirmed revision`, async () => {
    const { component } = await import(
      pathToFileURL(resolve(root, `dist/packages/deployment/${target}.js`))
    );
    const props = { theme: values.appearance[0], locale: 'en' };
    const source = { stateDomain: 'a'.repeat(64), deployment: uuid(80), revision: 7 };
    let result = component.mount(props, {});
    const complete = (event) => {
      result = component.event(
        result.frame.state,
        { kind: 'action', key: result.actions[0].key, event },
        props,
        {},
      );
    };
    const press = (key) => {
      result = component.event(result.frame.state, { kind: 'raw', key, event: null }, props, {});
    };
    const observed = (revision, bytes, receipt = null) => {
      result = component.event(
        result.frame.state,
        {
          kind: 'component',
          child: 'deployment-document-editor',
          name: 'changed',
          key: 'deployment-document-changed',
          event: {
            revision,
            snapshot: {
              text: { owner: 'original-document', revision, bytes },
              selection: { start: 0, end: 0 },
              reversed: false,
              marked: null,
            },
            receipt,
          },
        },
        props,
        {},
      );
    };
    complete({ status: 'succeeded', value: { source, instances: [], next: null } });
    press('deployment-advanced');
    assert.deepEqual(result.actions[0].input, { source, offset: 0 });
    const chunks = ['{\n"input":"' + '中🌟'.repeat(8500), 'second chunk', '","packages":[]}'];
    const total = chunks.reduce((n, s) => n + Buffer.byteLength(s), 0);
    let offset = 0;
    for (let at = 0; at < chunks.length; at++) {
      const text = chunks[at],
        end = offset + Buffer.byteLength(text),
        next = at === chunks.length - 1 ? null : end;
      complete({ status: 'succeeded', value: { source, offset, totalBytes: total, text, next } });
      if (at === 0) {
        assert.equal(result.frame.state.document.command, null);
        observed(0, 0);
      }
      const command = result.frame.state.document.command;
      assert.equal(command.edit.kind, at === 0 ? 'reset' : 'append');
      assert.equal(command.basedOn, at);
      assert.equal(result.actions.length, 0, 'next query waits for actual native applied receipt');
      if (at === 0) {
        press('deployment-document-back');
        press('deployment-history');
        assert.equal(
          result.frame.state.pending,
          null,
          'initialization keeps the request lane between chunks',
        );
        assert.equal(result.actions.length, 0);
      }
      observed(at + 1, end, {
        owner: command.owner,
        sequence: command.sequence,
        revision: at + 1,
        outcome: { status: 'applied' },
      });
      if (next !== null) assert.deepEqual(result.actions[0].input, { source, offset: next });
      offset = end;
    }
    assert.equal(result.frame.state.document.phase, 'ready');
    assert.equal(result.frame.state.document.chunk, null);
    assert.equal(result.frame.state.document.command, null);
    assert.ok(
      JSON.stringify(result.frame.state).length < 5000,
      'full editable source belongs only to the native owner',
    );
    press('deployment-advanced');
    assert.equal(
      result.actions.length,
      0,
      'returning to the retained editor does not reload its draft',
    );
    press('deployment-document-save');
    assert.equal(result.actions.length, 0);
    press('deployment-document-cancel');
    assert.equal(result.frame.state.document.confirmation, null);
    press('deployment-document-save');
    press('deployment-document-confirm');
    assert.deepEqual(result.actions[0].input, { expectedRevision: 7 });
    assert.deepEqual(result.actions[0].resources, {
      document: {
        child: 'deployment-document-editor',
        method: 'snapshot',
        input: { kind: 'at_revision', owner: 'original-document', revision: 3 },
      },
    });
    const original = {
      ...reference,
      source: { ...source, deployment: uuid(81) },
      namespace: [uuid(90)],
      export: 'Management',
      method: 'replace',
      contract: sdk.contracts.deploymentManager,
    };
    complete({ status: 'operation', reply: { reference: original, receipt: { kind: 'unknown' } } });
    press('deployment-document-save');
    assert.equal(result.actions.length, 0);
    press('deployment-document-check');
    assert.deepEqual(result.actions[0].input, original);
    complete({
      status: 'succeeded',
      value: {
        reference: original,
        receipt: {
          kind: 'recorded',
          outcome: { kind: 'returned', value: { status: 'conflict', currentRevision: 8 } },
          cleanup: { kind: 'confirmed' },
        },
      },
    });
    assert.equal(result.frame.state.document.source.revision, 7);
    assert.equal(result.frame.state.document.observation.snapshot.text.bytes, total);
    assert.equal(result.frame.state.document.conflict, true);
    press('deployment-document-save');
    assert.equal(result.frame.state.document.confirmation, null);
    press('deployment-document-history');
    assert.equal(result.actions[0].method, 'page');
    complete({ status: 'succeeded', value: [{ cursor: 31, reference: original }] });
    assert.deepEqual(result.frame.state.earlier[0].reference, original);
    press('deployment-earlier-check-0');
    assert.deepEqual(result.actions[0].input, original);
    complete({
      status: 'succeeded',
      value: { reference: original, receipt: { kind: 'unknown' } },
    });
    assert.equal(result.frame.state.document.observation.snapshot.text.bytes, total);
    assert.equal(result.frame.state.document.conflict, true);
    press('deployment-history-back');
    press('deployment-advanced');
    assert.equal(result.actions.length, 0);
    press('deployment-document-reload');
    assert.equal(result.actions.length, 0, 'discarding native draft requires confirmation');
    press('deployment-document-cancel');
    assert.equal(result.frame.state.document.source.revision, 7);
    press('deployment-document-reload');
    press('deployment-document-reload-confirm');
    complete({
      status: 'succeeded',
      value: { source: { ...source, revision: 8 }, instances: [], next: null },
    });
    assert.deepEqual(result.actions[0].input, { source: { ...source, revision: 8 }, offset: 0 });
    assert.equal(
      result.frame.state.document.observation.snapshot.text.bytes,
      total,
      'the first accepted new chunk performs reset, not the reload click',
    );
  });
  test(`${target} composition editor rejects mixed-source chunks and failed native initialization without enabling save`, async () => {
    const { component } = await import(
      pathToFileURL(resolve(root, `dist/packages/deployment/${target}.js`))
    );
    const props = { theme: values.appearance[0], locale: 'en' },
      source = { stateDomain: 'a'.repeat(64), deployment: uuid(80), revision: 7 };
    for (const failure of ['source', 'receipt']) {
      let result = component.mount(props, {});
      const complete = (value) => {
        result = component.event(
          result.frame.state,
          { kind: 'action', key: result.actions[0].key, event: { status: 'succeeded', value } },
          props,
          {},
        );
      };
      const press = (key) => {
        result = component.event(result.frame.state, { kind: 'raw', key, event: null }, props, {});
      };
      complete({ source, instances: [], next: null });
      press('deployment-advanced');
      complete({
        source: failure === 'source' ? { ...source, revision: 8 } : source,
        offset: 0,
        totalBytes: 2,
        text: '{}',
        next: null,
      });
      if (failure === 'receipt') {
        const observe = (event) => {
          result = component.event(
            result.frame.state,
            {
              kind: 'component',
              child: 'deployment-document-editor',
              name: 'changed',
              key: 'deployment-document-changed',
              event,
            },
            props,
            {},
          );
        };
        const observation = {
          revision: 0,
          snapshot: {
            text: { owner: 'original', revision: 0, bytes: 0 },
            selection: { start: 0, end: 0 },
            reversed: false,
            marked: null,
          },
          receipt: null,
        };
        observe(observation);
        const command = result.frame.state.document.command;
        observe({
          ...observation,
          receipt: {
            owner: 'original',
            sequence: command.sequence,
            revision: 0,
            outcome: { status: 'conflict', reason: 'revision' },
          },
        });
      }
      assert.equal(result.frame.state.document.phase, 'failed');
      press('deployment-document-save');
      assert.equal(result.frame.state.document.confirmation, null);
      assert.equal(result.actions.length, 0);
    }
  });
}

for (const target of ['desktop', 'tui']) {
  test(`${target} file installation freezes the real resource slot, preserves drafts and recovers only its original operation`, async () => {
    const { component } = await import(
      pathToFileURL(resolve(root, `dist/packages/deployment/${target}.js`))
    );
    const props = { theme: values.appearance[0], locale: 'en' };
    const source = { stateDomain: 'a'.repeat(64), deployment: uuid(80), revision: 7 };
    const file = {
      owner: uuid(99),
      revision: 4,
      phase: 'ready',
      file: { name: '插件.maka-plugin', bytes: 123, digest: 'sha256:' + 'b'.repeat(64) },
      error: null,
    };
    let result = component.mount(props, {});
    const complete = (event) => {
      result = component.event(
        result.frame.state,
        { kind: 'action', key: result.actions[0].key, event },
        props,
        {},
      );
    };
    const press = (key) => {
      result = component.event(result.frame.state, { kind: 'raw', key, event: null }, props, {});
    };
    const observe = (event) => {
      result = component.event(
        result.frame.state,
        {
          kind: 'component',
          child: 'deployment-import-file',
          name: 'changed',
          key: 'deployment-import-changed',
          event,
        },
        props,
        {},
      );
    };
    complete({ status: 'succeeded', value: { source, instances: [], next: null } });
    // A retained, modified composition is never reset by installation.
    const draft = {
      source,
      phase: 'ready',
      observation: null,
      sequence: 0,
      command: null,
      confirmation: null,
      reloadConfirmation: false,
      conflict: false,
    };
    result.frame.state.document = structuredClone(draft);
    press('deployment-import');
    press('deployment-import-install');
    assert.equal(result.frame.state.importer.confirmation, null);
    observe(file);
    press('deployment-import-install');
    assert.equal(result.actions.length, 0);
    assert.deepEqual(result.frame.state.importer.confirmation.file, file);
    observe({ ...file, revision: 5 });
    assert.equal(result.frame.state.importer.confirmation, null);
    press('deployment-import-confirm');
    assert.equal(result.actions.length, 0);
    press('deployment-import-install');
    press('deployment-import-confirm');
    const install = result.actions[0];
    assert.equal(install.method, 'install');
    assert.deepEqual(install.input, { expectedRevision: 7 });
    assert.deepEqual(install.resources.package, {
      child: 'deployment-import-file',
      method: 'snapshot',
      input: { owner: file.owner, revision: 5 },
    });
    // Validate the actual generated contract, including its mandatory resource name.
    assert.deepEqual(
      Object.keys(install.resources),
      Object.keys(sdk.contracts.deploymentManager.interface.methods.install.resources),
    );
    assert.equal(
      sdk.contracts.deploymentManager.interface.methods.install.resources.package.name,
      sdk.contracts.filePicker.interface.captures.snapshot.output.resource.name,
    );
    const original = {
      ...reference,
      source: { ...source, deployment: uuid(81) },
      namespace: [uuid(90)],
      export: 'Management',
      method: 'install',
      contract: sdk.contracts.deploymentManager,
    };
    complete({ status: 'operation', reply: { reference: original, receipt: { kind: 'unknown' } } });
    for (const key of [
      'deployment-import-install',
      'deployment-import-confirm',
      'deployment-refresh',
      'deployment-document-save',
    ]) {
      press(key);
      assert.equal(result.actions.length, 0);
    }
    press('deployment-check');
    assert.deepEqual(result.actions[0].input, original);
    const installed = {
      source: { ...source, revision: 8 },
      package: { id: 'plugin.custom', revision: 'sha256-' + 'c'.repeat(64) },
    };
    complete({
      status: 'succeeded',
      value: {
        reference: original,
        receipt: {
          kind: 'recorded',
          outcome: { kind: 'returned', value: { status: 'committed', value: installed } },
          cleanup: { kind: 'confirmed' },
        },
      },
    });
    assert.equal(result.frame.state.current, null);
    assert.equal(result.frame.state.notice, 'deploymentInstalled');
    assert.deepEqual(result.frame.state.importer.installed, installed);
    assert.equal(result.frame.state.page.source.revision, 8);
    assert.deepEqual(result.frame.state.document, draft);
    assert.equal(result.actions[0].method, 'instances');
    complete({ status: 'failed', message: 'refresh unavailable' });
    assert.equal(result.frame.state.page.source.revision, 8);
    assert.deepEqual(result.frame.state.document, draft);
    observe({ ...file, revision: 6, phase: 'choosing', file: null });
    assert.equal(result.frame.state.importer.installed, null);
    assert.notEqual(result.frame.state.notice, 'deploymentInstalled');
    observe({ ...file, revision: 7, phase: 'failed', file: null, error: 'read_failed' });
    assert.equal(result.frame.state.importer.installed, null);
    press('deployment-import-back');
    press('deployment-history');
    complete({ status: 'succeeded', value: [{ cursor: 1, reference: original }] });
    assert.deepEqual(result.frame.state.earlier[0].reference, original);
  });
  test(`${target} installation conflict requires refreshed source and a new confirmation, while refusal retains selection`, async () => {
    const { component } = await import(
      pathToFileURL(resolve(root, `dist/packages/deployment/${target}.js`))
    );
    const props = { theme: values.appearance[0], locale: 'zh-CN' };
    const source = { stateDomain: 'a'.repeat(64), deployment: uuid(80), revision: 7 };
    const file = {
      owner: uuid(99),
      revision: 4,
      phase: 'ready',
      file: { name: 'sample.maka-plugin', bytes: 123, digest: 'sha256:' + 'b'.repeat(64) },
    };
    let result = component.mount(props, {});
    const complete = (event) => {
      result = component.event(
        result.frame.state,
        { kind: 'action', key: result.actions[0].key, event },
        props,
        {},
      );
    };
    const press = (key) => {
      result = component.event(result.frame.state, { kind: 'raw', key, event: null }, props, {});
    };
    complete({ status: 'succeeded', value: { source, instances: [], next: null } });
    press('deployment-import');
    result = component.event(
      result.frame.state,
      {
        kind: 'component',
        child: 'deployment-import-file',
        name: 'changed',
        key: 'deployment-import-changed',
        event: file,
      },
      props,
      {},
    );
    press('deployment-import-install');
    press('deployment-import-cancel');
    assert.deepEqual(result.frame.state.importer.observation, file);
    press('deployment-import-install');
    press('deployment-import-confirm');
    const original = { ...reference, method: 'install', contract: sdk.contracts.deploymentManager };
    const reply = (value) => ({
      status: 'operation',
      reply: {
        reference: original,
        receipt: {
          kind: 'recorded',
          outcome: { kind: 'returned', value },
          cleanup: { kind: 'confirmed' },
        },
      },
    });
    complete(reply({ status: 'conflict', currentRevision: 8 }));
    press('deployment-import-install');
    assert.equal(result.frame.state.importer.confirmation, null);
    press('deployment-import-confirm');
    assert.equal(result.actions.length, 0);
    assert.deepEqual(result.frame.state.importer.observation, file);
    press('deployment-refresh');
    complete({
      status: 'succeeded',
      value: { source: { ...source, revision: 8 }, instances: [], next: null },
    });
    press('deployment-import-confirm');
    assert.equal(result.actions.length, 0);
    press('deployment-import-install');
    press('deployment-import-confirm');
    assert.deepEqual(result.actions[0].input, { expectedRevision: 8 });
    complete(reply({ status: 'rejected', reason: 'invalid_bundle' }));
    assert.equal(result.frame.state.notice, 'deploymentInstallRejected');
    assert.deepEqual(result.frame.state.importer.observation, file);
    assert.equal(result.frame.state.importer.installed, null);
  });
}

test('independent workbench variants retain page slots and react to real child event names', async () => {
  const choice = { label: 'Fixture model', selection: { connectionId: 'fixture', modelId: 'one' } };
  for (const target of ['desktop', 'tui']) {
    const module = await import(pathToFileURL(resolve(root, `dist/packages/studio/${target}.js`)));
    const nodes = (view) => [view, ...(view.children ?? []).flatMap(nodes)];
    for (const locale of ['en', 'zh-CN', 'zh-TW'])
      for (const scheme of ['light', 'dark']) {
        module.activate({ locale });
        const component = module.component;
        const values = { appearance: [appearance.activate({ scheme }).Appearance.value] };
        let result = component.mount({}, values);
        const slots = () => nodes(result.frame.view).filter((node) => node.kind === 'component');
        const press = (key) => {
          result = component.event(
            result.frame.state,
            { kind: 'raw', key, event: null },
            {},
            values,
          );
        };
        const child = (child, name, event) => {
          result = component.event(
            result.frame.state,
            { kind: 'component', key: 'arbitrary-routing-key', child, name, event },
            {},
            values,
          );
        };
        assert.deepEqual(
          slots().map((node) => [node.key, node.binding]),
          [
            ['connections', 'connections'],
            ['models', 'models'],
            ['deployment', 'deployment'],
          ],
        );
        const initialKeys = nodes(result.frame.view).map((node) => node.key);
        assert.equal(new Set(initialKeys).size, initialKeys.length);
        press('open-connections');
        assert.equal(result.frame.state.route, 'connections');
        child('connections', 'changed', {});
        assert.equal(result.frame.state.catalogEpoch, 1);
        press('route-models');
        child('models', 'selected', choice);
        assert.deepEqual(result.frame.state.selection, choice);
        assert.equal(slots()[1].props.catalogEpoch, 1);
        assert.deepEqual(slots()[1].props.selection, choice);
        press('route-deployment');
        press('route-overview');
        assert.deepEqual(
          nodes(result.frame.view).map((node) => node.key),
          initialKeys,
        );
        assert.equal(result.actions.length, 0);
        assert.equal(result.subscriptions.length, 0);
        assert.equal(result.frame.surface.binding, 'surface');
        assert.equal(result.frame.surface.kind, target === 'desktop' ? 'window' : 'terminal');
        child('deployment', 'closed', {});
        assert.equal(result.frame.state.route, 'overview');
        assert.deepEqual(result.frame.state.selection, choice);
      }
  }
});
