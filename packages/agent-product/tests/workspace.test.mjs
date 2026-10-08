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
    previous: false,
    failed: false,
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
test('typed scene constructors serialize directly into the actual Rust scene schemas', async () => {
  const desktopText = sdk.ui.desktop.text({ key: 'desktop-text', text: '界面', color: 0xffffff });
  const terminalText = sdk.ui.terminal.text({
    key: 'terminal-text',
    text: '界面',
    color: 0xffffff,
  });
  const desktop = sdk.ui.desktop.stack({
    key: 'desktop',
    direction: 'column',
    gap: 8,
    padding: 0,
    background: 0,
    children: [desktopText],
    scroll: 'anchor',
  });
  const terminal = sdk.ui.terminal.stack({
    key: 'terminal',
    direction: 'column',
    gap: 1,
    padding: 0,
    background: 0,
    children: [terminalText],
    scroll: 'anchor',
  });
  const desktopScenes = [
    desktop,
    desktopText,
    sdk.ui.desktop.button({ key: 'button', label: 'Save' }),
    sdk.ui.desktop.dialog({
      key: 'dialog',
      label: 'Dialog',
      background: desktop,
      content: desktopText,
      open: true,
    }),
    sdk.ui.desktop.popup({
      key: 'popup',
      label: 'Menu',
      trigger: desktopText,
      content: desktop,
      open: true,
    }),
    sdk.ui.desktop.canvas({
      key: 'canvas',
      contents: {
        commands: [
          { kind: 'line', from: { x: 0, y: 0 }, to: { x: 20, y: 20 }, color: 0xffffff, width: 1 },
        ],
        regions: [],
      },
    }),
    sdk.ui.desktop.reading({
      key: 'reading',
      binding: 'reading',
      props: { content: desktop },
      on: { observed: 'viewport' },
    }),
  ];
  const terminalScenes = [
    terminal,
    terminalText,
    sdk.ui.terminal.button({ key: 'button', label: 'Save' }),
    sdk.ui.terminal.dialog({
      key: 'dialog',
      label: 'Dialog',
      background: terminal,
      content: terminalText,
      open: true,
      width: 40,
      height: 10,
      background_color: 0,
      border_color: 0xffffff,
    }),
    sdk.ui.terminal.reading({
      key: 'reading',
      binding: 'reading',
      props: { content: terminal },
      on: { observed: 'viewport' },
    }),
  ];
  const encoded = JSON.stringify(desktop);
  assert.deepEqual(JSON.parse(encoded), {
    key: 'desktop',
    direction: 'column',
    gap: 8,
    padding: 0,
    background: 0,
    children: [{ key: 'desktop-text', text: '界面', color: 0xffffff, kind: 'text' }],
    scroll: 'anchor',
    kind: 'stack',
  });
  assert.equal(Object.getOwnPropertySymbols(desktop).length, 1);
  assert.equal(encoded.includes('maka.scene.target'), false);
  assert.equal(desktopScenes.at(-1).member, 0);
  if (process.env.MAKA_PRODUCT_CLI_CHECK) {
    const binary = resolve(root, '../..', process.env.CARGO_TARGET_DIR ?? 'target', 'debug/maka');
    const catalog = JSON.parse(
      execFileSync(binary, ['application', 'schema'], {
        encoding: 'utf8',
        maxBuffer: 4 * 1024 * 1024,
      }),
    );
    const { default: Ajv } = await import('ajv/dist/2020.js');
    for (const [name, scenes] of [
      ['DesktopScene', desktopScenes],
      ['TerminalScene', terminalScenes],
    ]) {
      const validate = new Ajv({ strict: false }).compile({
        $ref: '#/$defs/' + name,
        $defs: catalog.schema.$defs,
      });
      for (const scene of scenes)
        assert.equal(
          validate(JSON.parse(JSON.stringify(scene))),
          true,
          JSON.stringify(validate.errors),
        );
    }
  }
});

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
const workspaceFact = {
  path: '/projects/maka',
  directoryIdentity: 'sha256:' + 'a'.repeat(64),
  identity: 'workspace:v1:' + uuid(99),
};
const session = {
  id: 'session',
  title: 'A thoughtful conversation',
  labels: [],
  workspace: workspaceFact,
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
    this.query('list', { revision: 'catalog', sessions: [session], before: null, after: null });
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
      event: { status: 'updated', value: { kind: 'events', events, through, nextAfter: through } },
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
  client.query(
    'read',
    returned({ kind: 'started', sessionId: 'session', invocationId: 'run' }).reply,
  );
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
  client.query(
    'read',
    returned({ kind: 'started', sessionId: 'session', invocationId: 'old-run' }).reply,
  );
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
            workspaceRoots: [{ label: 'Projects', path: '/private/product-workspace' }],
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
  out = modelGuest.component.event(
    out.frame.state,
    {
      kind: 'component',
      child: 'model-query',
      name: 'changed',
      key: 'query',
      event: modelObservation('', 1),
    },
    enabled,
    {},
  );
  out = modelRaw(out, 'choose-15', enabled);
  const capture = out.actions[0];
  assert.equal(capture.method, 'read');
  assert.deepEqual(capture.resources.text.input, {
    kind: 'at_revision',
    owner: 'model-editor',
    revision: 1,
  });
  out = modelAction(
    out,
    capture.key,
    { status: 'succeeded', value: { text: '', metadata: modelObservation('', 1).snapshot.text } },
    enabled,
  );
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
  assert.equal(out.frame.state.pending.key, original.key);
  assert.equal(out.frame.state.choices.length, 0);
  out = modelRaw(out, 'toggle', enabled);
  out = modelAction(
    out,
    original.key,
    { status: 'succeeded', value: { kind: 'changed', revision: 6 } },
    enabled,
  );
  assert.equal(out.actions.length, 1);
  assert.deepEqual(out.actions[0].input, { kind: 'start', query: '' });
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
function modelObservation(text, revision) {
  const bytes = Buffer.byteLength(text);
  return {
    revision,
    snapshot: {
      text: { owner: 'model-editor', revision, bytes },
      selection: { start: bytes, end: bytes },
      reversed: false,
      marked: null,
    },
    receipt: null,
  };
}
function modelNative(form, text, revision) {
  return form.input({
    kind: 'component',
    child: 'model-query',
    name: 'changed',
    key: 'query',
    event: modelObservation(text, revision),
  });
}
function modelRead(form, text, revision) {
  const read = form.find('read');
  assert.deepEqual(read.resources.text.input, {
    kind: 'at_revision',
    owner: 'model-editor',
    revision,
  });
  return form.complete(read, {
    status: 'succeeded',
    value: { text, metadata: modelObservation(text, revision).snapshot.text },
  });
}
const modelPage = (choices = modelChoices, next = null) => ({
  kind: 'ready',
  revision: 5,
  choices,
  next,
  defaultModel: { kind: 'none' },
});
for (const target of ['tui', 'desktop']) {
  const { component } = await import(
    pathToFileURL(resolve(root, `dist/packages/models/${target}.js`))
  );
  const ready = () => {
    const form = new FormClient(component, {
      ...modelProps,
      busy: false,
      selection: modelChoices[0],
    });
    form.query('models', modelPage());
    modelNative(form, '', 1);
    form.raw('toggle');
    return form;
  };
  test(`${target} model search coalesces original native revisions and scans exact query continuations`, () => {
    const form = ready();
    const query = modelSlots(form.output.frame.view).find((n) => n.key === 'model-query');
    assert.equal(query.props.autoFocus, true);
    modelNative(form, 'earlier', 2);
    const old = form.find('read');
    modelNative(form, 'Account model-299', 3);
    assert.equal(form.output.actions.length, 0);
    form.complete(old, {
      status: 'succeeded',
      value: { text: 'earlier', metadata: modelObservation('earlier', 2).snapshot.text },
    });
    assert.equal(form.state.query, '');
    modelRead(form, 'Account model-299', 3);
    assert.deepEqual(form.find('models').input, { kind: 'start', query: 'Account model-299' });
    const cursor = { query: 'Account model-299', connection: 0, model: 256 };
    form.query('models', modelPage([], cursor));
    assert.deepEqual(form.find('models').input, { kind: 'continue', revision: 5, cursor });
    const choice = {
      ...modelChoices[0],
      selection: { ...modelChoices[0].selection, modelId: 'model-299' },
    };
    form.query('models', modelPage([choice]));
    form.input({
      kind: 'component',
      child: 'model-query',
      name: 'submitted',
      key: 'choose-active',
      event: {},
    });
    assert.deepEqual(form.output.events, []);
    modelRead(form, 'Account model-299', 3);
    assert.deepEqual(form.output.events, [{ name: 'selected', value: choice }]);
    assert.equal(form.state.open, false);
  });
  test(`${target} moving the search caret acknowledges its original revision without resetting model navigation`, () => {
    const form = ready();
    modelNative(form, 'model', 2);
    modelRead(form, 'model', 2);
    form.query('models', modelPage());
    for (let at = 0; at < 7; at++) form.raw('next-choice');
    const reveal = structuredClone(form.state.reveal);
    const choices = structuredClone(form.state.choices);
    const observation = modelObservation('model', 3);
    observation.snapshot.selection = { start: 0, end: 0 };
    form.input({
      kind: 'component',
      child: 'model-query',
      name: 'changed',
      key: 'query',
      event: observation,
    });
    modelRead(form, 'model', 3);
    assert.equal(form.state.active, 7);
    assert.deepEqual(form.state.choices, choices);
    assert.deepEqual(form.state.reveal, reveal);
    assert.equal(form.state.queryBasis.revision, 3);
    assert.deepEqual(form.output.actions, []);
  });
  test(`${target} model arrows keep the original query and reveal the active row in both directions`, () => {
    const form = ready();
    for (let at = 0; at < 15; at++) form.raw('next-choice');
    assert.equal(form.state.active, 15);
    assert.deepEqual(form.state.observation, modelObservation('', 1));
    assert.equal(form.output.actions.length, 0);
    if (target === 'desktop') {
      const reading = modelSlots(form.output.frame.view).find((n) => n.key === 'model-rows');
      assert.equal(reading.props.content.scroll, 'manual');
      assert.equal(reading.props.content.children.length, 16);
      assert.deepEqual(reading.props.command.target, { kind: 'reveal_child', key: 'choose-15' });
      const sequence = reading.props.command.sequence;
      form.input({
        kind: 'component',
        child: 'model-rows',
        name: 'observed',
        key: 'row-viewport',
        event: {},
      });
      assert.equal(form.state.reveal.sequence, sequence);
      form.input({
        kind: 'component',
        child: 'model-rows',
        name: 'input',
        key: 'row-input',
        event: { kind: 'raw', key: 'choose-15', event: {} },
      });
    } else {
      assert.equal(form.state.offset, 15);
      assert.deepEqual(
        modelButtons(form.output.frame.view).filter((n) => n.startsWith('choose-')),
        ['choose-15'],
      );
      form.raw('choose-15');
    }
    modelRead(form, '', 1);
    assert.deepEqual(form.output.events, [{ name: 'selected', value: modelChoices[15] }]);
    form.raw('toggle');
    for (let at = 0; at < 15; at++) form.raw('previous-choice');
    assert.equal(form.state.active, 0);
    assert.deepEqual(form.state.reveal.target, { kind: 'reveal_child', key: 'choose-0' });
  });
  test(`${target} hidden, defaults and busy model searches stop scanning and resume from the original cursor`, () => {
    for (const barrier of ['close', 'defaults', 'busy']) {
      const form = ready();
      modelNative(form, 'later', 2);
      modelRead(form, 'later', 2);
      if (barrier === 'busy') form.update({ busy: true });
      else form.raw(barrier);
      const cursor = { query: 'later', connection: 0, model: 256 };
      form.query('models', modelPage([], cursor));
      assert.deepEqual(form.output.actions, []);
      if (barrier === 'busy') form.update({ busy: false });
      else form.raw(barrier === 'close' ? 'toggle' : 'default-back');
      assert.deepEqual(form.find('models').input, { kind: 'continue', revision: 5, cursor });
      assert.equal(form.state.pending.key, form.find('models').key);
    }
  });
  test(`${target} model selection requires the original snapshot and closing disarms pending confirmation`, () => {
    const form = ready();
    form.raw('choose-3');
    const selection = form.find('read');
    form.raw('close');
    form.raw('toggle');
    form.complete(selection, {
      status: 'succeeded',
      value: { text: '', metadata: modelObservation('', 1).snapshot.text },
    });
    assert.deepEqual(form.output.events, []);
    form.raw('choose-3');
    form.complete(form.find('read'), { status: 'failed', reason: 'snapshot_revision_changed' });
    assert.deepEqual(form.output.events, []);
    modelNative(form, 'new', 2);
    modelRead(form, 'new', 2);
    assert.deepEqual(form.find('models').input, { kind: 'start', query: 'new' });
    const old = form.find('models');
    modelNative(form, 'latest', 3);
    form.complete(old, { status: 'succeeded', value: modelPage() });
    modelRead(form, 'latest', 3);
    assert.deepEqual(form.find('models').input, { kind: 'start', query: 'latest' });
    assert.deepEqual(form.output.events, []);
  });
  test(`${target} changed search retries a failed directory while caret changes preserve the failure`, () => {
    const form = ready();
    modelNative(form, 'model', 2);
    modelRead(form, 'model', 2);
    form.complete(form.find('models'), { status: 'failed', reason: 'offline' });
    modelNative(form, 'model', 3);
    modelRead(form, 'model', 3);
    assert.equal(form.state.failed, true);
    assert.deepEqual(form.output.actions, []);
    modelNative(form, 'different', 4);
    modelRead(form, 'different', 4);
    assert.equal(form.state.failed, false);
    assert.deepEqual(form.find('models').input, { kind: 'start', query: 'different' });
  });
  test(`${target} a failed older directory request cannot strand a coalesced or cleared search`, () => {
    for (const latest of ['latest', '']) {
      const form = ready();
      modelNative(form, 'old', 2);
      modelRead(form, 'old', 2);
      const old = form.find('models');
      modelNative(form, latest, 3);
      assert.deepEqual(form.output.actions, []);
      form.complete(old, { status: 'failed', reason: 'offline' });
      if (latest) modelRead(form, latest, 3);
      assert.equal(form.state.failed, false);
      assert.deepEqual(form.find('models').input, { kind: 'start', query: latest });
    }
  });
  test(`${target} default metadata refresh ignores invalid search and keeps each cursor with its page revision`, () => {
    const form = ready();
    modelNative(form, 'model', 2);
    modelRead(form, 'model', 2);
    const oldCursor = { query: 'model', connection: 0, model: 16 };
    form.query('models', modelPage(modelChoices, oldCursor));
    form.raw('defaults');
    modelNative(form, 'model', 3);
    assert.deepEqual(form.output.actions, []);
    form.raw('default-refresh');
    assert.deepEqual(form.find('models').input, { kind: 'start', query: 'model' });
    const cursor = { query: 'model', connection: 0, model: 256 };
    form.query('models', {
      ...modelPage([], cursor),
      revision: 6,
      defaultModel: { kind: 'ready', choice: modelChoices[15] },
    });
    assert.equal(form.state.catalogRevision, 6);
    assert.deepEqual(form.state.defaultModel, { kind: 'ready', choice: modelChoices[15] });
    assert.deepEqual(form.output.actions, []);
    form.raw('default-back');
    modelRead(form, 'model', 3);
    assert.deepEqual(form.find('models').input, { kind: 'continue', revision: 6, cursor });
    form.query('models', { ...modelPage(), revision: 6 });
    modelNative(form, '界'.repeat(257), 4);
    modelRead(form, '界'.repeat(257), 4);
    assert.equal(form.state.queryError, 'searchTooLong');
    form.raw('defaults');
    form.raw('default-refresh');
    assert.equal(form.find('models').method, 'models');
    form.query('models', { ...modelPage(), revision: 7 });
    assert.equal(form.state.catalogRevision, 7);
    form.input({
      kind: 'component',
      child: 'default-settings',
      name: 'changed',
      key: 'default-changed',
      event: {},
    });
    assert.deepEqual(
      form.output.actions.map((a) => a.method),
      ['models'],
    );
    form.query('models', { ...modelPage(), revision: 8 });
    assert.equal(form.state.catalogRevision, 8);
  });
  test(`${target} model search bounds Unicode input and resumes dirty text after disabled props`, () => {
    const form = ready();
    form.update({ busy: true });
    modelNative(form, 'waiting', 2);
    assert.deepEqual(form.output.actions, []);
    form.update({ busy: false });
    assert.equal(form.state.pending.key, form.find('read').key);
    modelRead(form, 'waiting', 2);
    form.query('models', modelPage());
    modelNative(form, '界'.repeat(257), 3);
    modelRead(form, '界'.repeat(257), 3);
    assert.equal(form.state.queryError, 'searchTooLong');
    assert.deepEqual(form.output.actions, []);
    modelNative(form, '🙂'.repeat(257), 4);
    assert.equal(form.state.queryError, 'searchTooLong');
    assert.deepEqual(form.output.actions, []);
    modelNative(form, '', 5);
    assert.equal(form.state.queryError, null);
    assert.equal(form.state.query, '');
    assert.deepEqual(form.find('models').input, { kind: 'start', query: '' });
  });
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
    ...[
      ...(node.children ?? []),
      node.content,
      typeof node.background === 'object' ? node.background : null,
    ]
      .filter(Boolean)
      .flatMap(slots),
  ];
  const original = slots(client.output.frame.view).find((n) => n.binding === 'composer');
  client.raw('settings');
  assert.equal(client.state.settingsOpen, true);
  const hidden = client.output.frame.view.background.children[0];
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
  assert.equal(client.output.frame.view.background.children[0].visible, true);
  assert.equal(
    slots(client.output.frame.view).find((n) => n.binding === 'composer').key,
    original.key,
  );
});
test('sidebar presentation retains the same children, draft command, and active work', () => {
  const client = new Client().start();
  client.state.selected.title = '长标题 · A🙂é '.repeat(25);
  client.component('draft', draft);
  const originalState = structuredClone(client.state);
  const originalSlots = modelSlots(client.output.frame.view.background);
  client.raw('navigation-toggle');
  assert.equal(client.state.navigationOpen, false);
  assert.equal(client.output.frame.view.background.children[0].children[0].visible, false);
  assert.deepEqual(modelSlots(client.output.frame.view.background), originalSlots);
  assert.deepEqual(client.output.actions, []);
  assert.deepEqual(client.state.watch, originalState.watch);
  assert.deepEqual(client.state.command, originalState.command);
  assert.deepEqual(client.state.model, originalState.model);
  client.raw('settings');
  client.input({ kind: 'component', child: 'connections', name: 'closed', event: {} });
  assert.equal(client.state.navigationOpen, false);
  client.raw('navigation-toggle');
  assert.equal(client.state.navigationOpen, true);
  assert.deepEqual(modelSlots(client.output.frame.view.background), originalSlots);
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
  for (const child of [
    ...(node.children ?? []),
    node.trigger,
    node.content,
    typeof node.background === 'object' ? node.background : null,
  ].filter(Boolean))
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
      next: { query: '', connection: 0, model: 16 },
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
      client.query('list', { revision: 'catalog', sessions: [session], before: null, after: null });
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
  const routes = () => client.output.frame.view.background.children[0].children;
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
      for (const child of [
        ...(node.children ?? []),
        node.content,
        typeof node.background === 'object' ? node.background : null,
      ].filter(Boolean))
        visit(child);
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
      target === 'desktop' ? 'primary_enter' : 'enter',
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
    assert.equal(slot.props.submit, target === 'desktop' ? 'primary_enter' : 'enter');
    const latest = {
      ...draft,
      revision: 5,
      snapshot: { ...draft.snapshot, text: { ...draft.snapshot.text, revision: 5, bytes: 8 } },
    };
    assert.deepEqual(submit(latest).events, [{ name: 'send', value: latest }]);
    if (target === 'tui')
      assert.deepEqual(submit(latest, { ...props, running: true }).events, [
        { name: 'send', value: latest },
      ]);
    for (const override of [
      { busy: true },
      ...(target === 'desktop' ? [{ running: true }] : []),
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
      const blocked = submit(latest, { ...props, ...override });
      assert.deepEqual(blocked.events, []);
      assert.equal(
        modelSlots(blocked.frame.view).find((node) => node.key === 'draft').props.submit,
        target === 'tui' ? 'enter' : null,
        'blocked TUI Enter keeps the draft instead of becoming a newline',
      );
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

for (const target of ['desktop', 'tui']) {
  test(`${target} session pages reach all entries without moving the selected conversation`, async () => {
    const { component } = await import(
      pathToFileURL(resolve(root, `dist/packages/workspace/${target}.js`))
    );
    const client = new Client(component).start();
    const active = structuredClone({
      selected: client.state.selected,
      watch: client.state.watch,
      window: client.state.window,
      command: client.state.command,
      epoch: client.state.epoch,
    });
    const all = Array.from({ length: 145 }, (_, index) => ({
      ...session,
      id: `session-${String(index).padStart(3, '0')}`,
      title: `Conversation ${index}`,
    }));
    const page = (offset) => ({
      revision: 'catalog-2',
      sessions: all.slice(offset, offset + 32),
      before: offset ? all[offset].id : null,
      after: offset + 32 < all.length ? all[offset + 31].id : null,
    });
    client.component('refresh');
    assert.deepEqual(client.find('list').input, { kind: 'start' });
    client.query('list', page(0));
    const visited = [...client.state.catalog.sessions];
    for (let offset = 32; offset < all.length; offset += 32) {
      const cursor = client.state.catalog.after;
      client.component('more');
      assert.deepEqual(client.find('list').input, { kind: 'after', revision: 'catalog-2', cursor });
      client.query('list', page(offset));
      assert.ok(client.state.catalog.sessions.length <= 32);
      visited.push(...client.state.catalog.sessions);
    }
    assert.deepEqual(
      visited.map((entry) => entry.id),
      all.map((entry) => entry.id),
    );
    for (let offset = 96; offset >= 0; offset -= 32) {
      const cursor = client.state.catalog.before;
      client.component('previous');
      assert.deepEqual(client.find('list').input, {
        kind: 'before',
        revision: 'catalog-2',
        cursor,
      });
      client.query('list', page(offset));
    }
    assert.deepEqual(
      {
        selected: client.state.selected,
        watch: client.state.watch,
        window: client.state.window,
        command: client.state.command,
        epoch: client.state.epoch,
      },
      active,
    );
    const oldPage = structuredClone(client.state.catalog);
    client.component('more');
    client.complete(client.find('list'), { status: 'failed', error: 'agent_catalog_unavailable' });
    assert.deepEqual(client.state.catalog, oldPage);
    assert.equal(client.state.catalogFailed, true);
    assert.equal(client.state.notice, null);
    assert.equal(
      modelSlots(client.output.frame.view).find((node) => node.binding === 'composer').props
        .enabled,
      true,
    );
    client.component('refresh');
    client.query('list', page(0));
    assert.equal(client.state.catalogFailed, false);
    // Creation leaves the revisioned page intact until the fresh catalog returns.
    client.component('create');
    const created = { ...session, id: 'created', title: 'Newly created' };
    client.complete(client.find('create'), returned({ kind: 'created', session: created }));
    client.query('list', { revision: 'catalog-3', sessions: [created], before: null, after: null });
    assert.deepEqual(client.state.selected, created);
    const replay = new Client(component);
    const stale = replay.find('list');
    replay.query('page', [{ cursor: 7, caller: reference.source, consumer: [uuid(9)], reference }]);
    replay.complete(replay.find('read'), { status: 'failed', reason: 'temporarily unavailable' });
    replay.raw('check');
    replay.query('read', returned(null).reply);
    const fresh = replay.find('list');
    assert.notEqual(stale.key, fresh.key);
    replay.complete(fresh, {
      status: 'succeeded',
      value: { revision: 'catalog-3', sessions: [created], before: null, after: null },
    });
    replay.complete(stale, { status: 'succeeded', value: page(0) });
    assert.equal(replay.state.catalog.revision, 'catalog-3');
    assert.deepEqual(replay.state.selected, created);
  });
}

test('session catalog contracts require explicit boundaries and bound the page size', async () => {
  const { default: Ajv } = await import('ajv/dist/2020.js');
  const method = sdk.contracts.agent.interface.methods.list;
  const ajv = new Ajv({ strict: false, formats: { uint64: true } });
  const validate = ajv.compile(method.output);
  const page = {
    revision: 'catalog',
    sessions: Array.from({ length: 32 }, () => session),
    before: null,
    after: null,
  };
  assert.equal(validate(page), true);
  for (const boundary of ['before', 'after']) {
    const incomplete = { ...page };
    delete incomplete[boundary];
    assert.equal(validate(incomplete), false);
  }
  assert.equal(validate({ ...page, sessions: [...page.sessions, session] }), false);
  const position = ajv.compile(method.input);
  assert.equal(position({ kind: 'start' }), true);
  assert.equal(position({ kind: 'before', revision: 'catalog', cursor: 'session' }), true);
  assert.equal(position({ kind: 'after', cursor: 'session' }), false);
});

test('first creation captures a published directory and guards the complete preparation', () => {
  const client = new Client();
  client.query('page', []);
  client.query('list', { revision: 'empty', sessions: [], before: null, after: null });
  client.raw('create');
  const query = client.find('directories');
  assert.deepEqual(query.input, { kind: 'roots' });
  assert.equal(
    [...client.actions.values()].some((action) => action.method === 'create'),
    false,
  );
  const requests = client.actions.size;
  client.raw('create');
  assert.equal(client.actions.size, requests);
  const selection = {
    rootId: 'root-1',
    segments: [],
    directoryIdentity: workspaceFact.directoryIdentity,
  };
  client.query('directories', {
    kind: 'roots',
    roots: [{ name: 'Projects', path: workspaceFact.path, selection }],
  });
  assert.deepEqual(client.find('create').input, {
    title: 'New conversation',
    labels: [],
    workspace: { kind: 'directory', selection },
  });
  client.complete(client.find('create'), returned({ kind: 'created', session }));
  assert.deepEqual(client.state.selected, session);
});

test('confirmed creation refusal keeps the original session and does not become Unknown', () => {
  const client = new Client().start();
  const original = {
    selected: structuredClone(client.state.selected),
    watch: structuredClone(client.state.watch),
    epoch: client.state.epoch,
    command: structuredClone(client.state.command),
  };
  client.raw('create');
  client.complete(
    client.find('create'),
    returned({ kind: 'rejected', reason: 'workspace_unavailable' }),
  );
  assert.deepEqual(
    {
      selected: client.state.selected,
      watch: client.state.watch,
      epoch: client.state.epoch,
      command: client.state.command,
    },
    original,
  );
  assert.equal(client.state.uncertain, null);
  assert.equal(client.state.notice, 'createRejected');
  assert.equal(
    modelSlots(client.output.frame.view).find((node) => node.binding === 'composer').props.enabled,
    true,
  );
  client.component('send', draft);
  assert.equal(client.find('read').method, 'read');
});

test('creation refusal cannot erase a watch failure that arrived during creation', () => {
  const client = new Client().start();
  client.raw('create');
  client.input({
    kind: 'action',
    key: client.state.watch.key,
    event: { status: 'failed', reason: 'connection lost' },
  });
  client.complete(client.find('create'), returned({ kind: 'rejected', reason: 'labels_invalid' }));
  assert.equal(client.state.notice, 'unavailable');
  assert.equal(
    modelSlots(client.output.frame.view).find((node) => node.binding === 'composer').props.enabled,
    false,
  );
  client.component('send', draft);
  assert.deepEqual(client.output.actions, []);
});

test('ordinary root shortcuts share creation policy and close only presented pages', () => {
  const client = new Client().start();
  const shortcuts = () => client.output.frame.view.background.shortcuts;
  assert.ok(
    shortcuts().some((shortcut) => shortcut.key === 'b' && shortcut.input === 'navigation-toggle'),
  );
  const watch = structuredClone(client.state.watch);
  client.raw('navigation-back');
  assert.equal(client.state.navigationOpen, false);
  assert.equal(
    shortcuts().some((shortcut) => shortcut.key === 'escape'),
    false,
  );
  assert.deepEqual(client.state.watch, watch);
  client.raw('settings');
  assert.equal(
    shortcuts().some((shortcut) => shortcut.key === 'escape'),
    false,
    'child pages own their closing policy',
  );
  client.input({ kind: 'component', child: 'connections', name: 'closed', event: {} });
  assert.equal(client.state.settingsOpen, false);
  client.raw('create');
  assert.deepEqual(client.find('create').input, {
    title: 'New conversation',
    labels: [],
    workspace: { kind: 'session', sessionId: session.id },
  });
  assert.equal(shortcuts().find((shortcut) => shortcut.key === 'n').enabled, false);
  const requests = client.actions.size;
  client.raw('create');
  assert.equal(
    client.actions.size,
    requests,
    'pending creation cannot be duplicated by another input',
  );
});

for (const target of ['tui', 'desktop']) {
  const { component } = await import(
    pathToFileURL(resolve(root, `dist/packages/command-palette/${target}.js`))
  );
  const commands = Array.from({ length: 80 }, (_, at) => ({
    id: `action-${at}`,
    label: `Action ${at}`,
    hint: '',
    disabled: false,
  }));
  const make = (items = commands) => {
    const form = new FormClient(component, {
      theme: modelProps.theme,
      locale: 'en',
      open: true,
      busy: false,
      selectionDisabled: false,
      commands: items,
    });
    native(form, '', 1);
    return form;
  };
  function native(form, text, revision) {
    return form.input({
      kind: 'component',
      child: 'palette-query',
      name: 'changed',
      key: 'query',
      event: modelObservation(text, revision),
    });
  }
  function read(form, text, revision) {
    const capture = form.find('read');
    assert.deepEqual(capture.resources.text, {
      child: 'palette-query',
      method: 'snapshot',
      input: { kind: 'at_revision', owner: 'model-editor', revision },
    });
    return form.complete(capture, {
      status: 'succeeded',
      value: { text, metadata: modelObservation(text, revision).snapshot.text },
    });
  }
  function sessions(at, titles, after = null, revision = 'catalog-A') {
    return {
      revision,
      before: at ? `session-${at - 1}` : null,
      after,
      sessions: titles.map((title, index) => ({
        id: `session-${at + index}`,
        title,
        labels: [],
        workspace: workspaceFact,
        revision: 1,
        updatedAt: 1,
        archived: false,
      })),
    };
  }
  const viewport = (form) =>
    modelSlots(form.output.frame.view).find((node) => node.key === 'palette-rows').props;
  test(`${target} command IDs reserve the native key byte budget and labels retain Unicode`, async () => {
    const { default: Ajv } = await import('ajv/dist/2020.js');
    const { paletteContract } = await import(
      pathToFileURL(resolve(root, 'dist/authoring/agent-product/src/contracts.mjs'))
    );
    const schema = paletteContract.interface.props.properties.commands.items;
    const validate = new Ajv({ strict: false }).compile(schema);
    const command = { id: 'A'.repeat(120), label: '界'.repeat(120), hint: '', disabled: false };
    assert.equal(validate(command), true);
    assert.equal(validate({ ...command, id: 'A'.repeat(121) }), false);
    assert.equal(validate({ ...command, id: '界'.repeat(120) }), false);
    const form = make([command]);
    const row = viewport(form).content.children[0];
    assert.equal(Buffer.byteLength(row.key), 128);
    form.raw(row.key);
    read(form, '', 1);
    assert.deepEqual(form.output.events, [{ name: 'invoked', value: { id: command.id } }]);
  });
  test(`${target} availability and label updates preserve the original manual-reading command`, () => {
    const form = make();
    form.raw('page-down');
    const command = structuredClone(viewport(form).command);
    form.update({
      commands: commands.map((command) => ({
        ...command,
        disabled: true,
        label: command.label + ' updated',
      })),
    });
    assert.deepEqual(viewport(form).command, command);
    assert.equal(form.state.active, 32);
    assert.equal(form.state.offset, 32);
  });
  test(`${target} ordinary palette bounds native rows and keeps list keys, command guards and controlled open ownership`, () => {
    const form = make();
    assert.equal(viewport(form).content.children.length, 32);
    const original = structuredClone(form.state.observation);
    form.raw('last');
    assert.equal(form.state.active, 79);
    form.raw('first');
    assert.equal(form.state.active, 0);
    form.raw('page-down');
    assert.equal(form.state.active, 32);
    assert.equal(viewport(form).command.target.key, 'command-action-32');
    assert.equal(viewport(form).content.children[0].key, 'command-action-32');
    form.raw('up');
    assert.equal(form.state.active, 31);
    assert.deepEqual(form.state.observation, original);
    form.raw('command-action-31');
    assert.deepEqual(form.output.events, []);
    form.update({
      commands: commands.map((command) =>
        command.id === 'action-31' ? { ...command, disabled: true } : command,
      ),
    });
    read(form, '', 1);
    assert.deepEqual(form.output.events, []);
    form.raw('command-action-30');
    read(form, '', 1);
    assert.deepEqual(form.output.events, [{ name: 'invoked', value: { id: 'action-30' } }]);
    assert.equal(
      form.output.frame.view.visible,
      true,
      'only the parent can close its controlled palette',
    );
    form.raw('close');
    assert.deepEqual(form.output.events, [{ name: 'closed', value: {} }]);
    assert.equal(form.output.frame.view.visible, true);
    form.update({ open: false });
    assert.equal(form.output.frame.view.visible, false);
  });
  test(`${target} palette searches canonical labels and workspace without title matches`, () => {
    const form = make([]);
    native(form, 'research orchid', 2);
    read(form, 'research orchid', 2);
    const matching = {
      ...session,
      title: 'A conversation',
      labels: ['Research'],
      workspace: { ...workspaceFact, path: '/projects/orchid' },
    };
    const other = {
      ...matching,
      id: 'other',
      workspace: { ...workspaceFact, path: '/projects/fern' },
    };
    form.query('list', {
      revision: 'metadata',
      sessions: [matching, other],
      before: null,
      after: null,
    });
    assert.deepEqual(form.state.matches, [matching]);
    form.input({
      kind: 'component',
      child: 'palette-query',
      name: 'submitted',
      key: 'choose',
      event: {},
    });
    read(form, 'research orchid', 2);
    assert.deepEqual(form.output.events, [{ name: 'selected', value: matching }]);
  });
  test(`${target} palette reaches a session beyond 1024 catalog rows without borrowing the sidebar page`, () => {
    const form = make();
    native(form, 'needle', 2);
    read(form, 'needle', 2);
    assert.deepEqual(form.find('list').input, { kind: 'start' });
    for (let page = 0; page < 36; page++) {
      const at = page * 32;
      form.query(
        'list',
        sessions(
          at,
          Array.from({ length: 32 }, () => 'Other conversation'),
          `session-${at + 31}`,
        ),
      );
      assert.deepEqual(form.find('list').input, {
        kind: 'after',
        revision: 'catalog-A',
        cursor: `session-${at + 31}`,
      });
      assert.equal(form.state.matches.length, 0);
      assert.equal(form.state.source.sessions.length, 32);
    }
    const found = sessions(1152, ['A Needle Conversation']).sessions[0];
    form.query('list', sessions(1152, [found.title]));
    assert.deepEqual(form.state.matches, [found]);
    form.input({
      kind: 'component',
      child: 'palette-query',
      name: 'submitted',
      key: 'choose',
      event: {},
    });
    read(form, 'needle', 2);
    assert.deepEqual(form.output.events, [{ name: 'selected', value: found }]);
    assert.equal('catalog' in form.state, false);
  });
  test(`${target} palette More retains the original source cursor and one bounded matching window`, () => {
    const form = make([]);
    native(form, 'match', 2);
    read(form, 'match', 2);
    form.query(
      'list',
      sessions(
        0,
        Array.from({ length: 32 }, (_, at) => `Match ${at}`),
        'session-31',
      ),
    );
    assert.equal(form.state.matches.length, 32);
    assert.equal(form.output.actions.length, 0);
    const firstKey = viewport(form).content.children[0].key;
    form.raw('more');
    assert.equal(form.state.matches.length, 0);
    assert.deepEqual(form.find('list').input, {
      kind: 'after',
      revision: 'catalog-A',
      cursor: 'session-31',
    });
    form.query(
      'list',
      sessions(
        32,
        Array.from({ length: 32 }, (_, at) => `Match ${at + 32}`),
      ),
    );
    assert.equal(form.state.matches.length, 32);
    assert.equal(form.state.matches[0].id, 'session-32');
    assert.notEqual(viewport(form).content.children[0].key, firstKey);
    form.raw(firstKey);
    assert.deepEqual(form.output.actions, []);
    assert.deepEqual(form.output.events, []);
  });
  test(`${target} command choice supersedes a pending directory read and cannot adopt its late result`, () => {
    const form = make([{ id: 'settings', label: 'Settings', hint: '', disabled: false }]);
    native(form, 'settings', 2);
    read(form, 'settings', 2);
    const directory = form.find('list');
    form.raw('command-settings');
    const capture = form.find('read');
    form.complete(directory, {
      status: 'succeeded',
      value: sessions(0, ['Old Settings Conversation']),
    });
    assert.equal(form.state.pending.key, capture.key);
    assert.equal(form.state.matches.length, 0);
    read(form, 'settings', 2);
    assert.deepEqual(form.output.events, [{ name: 'invoked', value: { id: 'settings' } }]);
  });
  test(`${target} a disabled command choice resumes its retired directory scan`, () => {
    const command = { id: 'settings', label: 'Settings', hint: '', disabled: false };
    const form = make([command]);
    native(form, 'settings', 2);
    read(form, 'settings', 2);
    const old = form.find('list');
    form.raw('command-settings');
    form.update({ commands: [{ ...command, disabled: true }] });
    read(form, 'settings', 2);
    assert.deepEqual(form.output.events, []);
    const next = form.output.actions.find((action) => action.method === 'list');
    assert.notEqual(next.key, old.key);
    assert.deepEqual(next.input, { kind: 'start' });
    form.complete(old, { status: 'succeeded', value: sessions(0, ['Old Settings']) });
    assert.equal(form.state.pending.key, next.key);
    assert.deepEqual(form.state.matches, []);
    form.complete(next, { status: 'succeeded', value: sessions(1, ['Current Settings']) });
    assert.deepEqual(
      form.state.matches.map((session) => session.id),
      ['session-1'],
    );
  });
  test(`${target} explicit palette Refresh recovers failed query and selection reads without replaying a choice`, () => {
    const form = make([]);
    native(form, 'match', 2);
    form.complete(form.find('read'), { status: 'failed', reason: 'temporarily unavailable' });
    assert.equal(form.state.queryError, 'unavailable');
    assert.ok(
      form.output.frame.view.children.some(
        (node) =>
          node.key === 'palette-controls' &&
          node.children.some((button) => button.key === 'refresh' && !button.disabled),
      ),
    );
    form.raw('refresh');
    read(form, 'match', 2);
    assert.deepEqual(form.find('list').input, { kind: 'start' });
    form.query('list', sessions(0, ['Match A']));
    form.raw(viewport(form).content.children[0].key);
    form.complete(form.find('read'), { status: 'failed', reason: 'temporarily unavailable' });
    assert.deepEqual(form.output.events, []);
    form.raw('refresh');
    assert.equal(form.state.queryError, null);
    assert.deepEqual(form.find('list').input, { kind: 'start' });
    form.query('list', sessions(1, ['Match B']));
    assert.deepEqual(form.output.events, []);
    assert.equal(form.state.matches[0].id, 'session-1');
    native(form, '界'.repeat(257), 3);
    read(form, '界'.repeat(257), 3);
    assert.equal(form.state.queryError, 'long');
    form.raw('refresh');
    assert.deepEqual(form.output.actions, []);
    assert.equal(form.state.queryError, 'long');
  });
  test(`${target} palette coalesces failed old searches, suspends closed or busy scanning and preserves original query`, () => {
    const form = make();
    native(form, 'old', 2);
    read(form, 'old', 2);
    native(form, 'latest', 3);
    assert.equal(form.output.actions.length, 0);
    form.complete(form.find('list'), { status: 'failed', reason: 'offline' });
    read(form, 'latest', 3);
    assert.equal(form.state.query, 'latest');
    form.update({ open: false });
    form.query('list', sessions(0, ['Another'], 'session-0'));
    assert.deepEqual(form.output.actions, []);
    form.update({ open: true, busy: true });
    assert.deepEqual(form.output.actions, []);
    form.update({ busy: false });
    assert.deepEqual(form.find('list').input, {
      kind: 'after',
      revision: 'catalog-A',
      cursor: 'session-0',
    });
    form.query('list', sessions(1, ['Latest Conversation']));
    assert.equal(form.state.matches[0].id, 'session-1');
    native(form, 'latest', 4);
    read(form, 'latest', 4);
    assert.deepEqual(form.output.actions, []);
    assert.equal(form.state.matches[0].id, 'session-1');
  });
  test(`${target} palette never combines catalog revisions and native snapshot failure cannot select`, () => {
    const form = make([]);
    native(form, 'match', 2);
    read(form, 'match', 2);
    form.query('list', sessions(0, ['Match A'], 'session-0'));
    form.query('list', sessions(1, ['Match B'], null, 'catalog-B'));
    assert.equal(form.state.failed, true);
    assert.deepEqual(
      form.state.matches.map((session) => session.title),
      ['Match A'],
    );
    assert.equal(form.output.actions.length, 0);
    form.raw('refresh');
    assert.deepEqual(form.find('list').input, { kind: 'start' });
    form.query('list', sessions(1, ['Match B'], null, 'catalog-B'));
    form.raw(viewport(form).content.children[0].key);
    form.complete(form.find('read'), { status: 'failed', reason: 'snapshot_revision_changed' });
    assert.deepEqual(form.output.events, []);
    assert.equal(form.state.queryError, 'unavailable');
    native(form, '界'.repeat(257), 3);
    read(form, '界'.repeat(257), 3);
    assert.equal(form.state.queryError, 'long');
    assert.deepEqual(form.output.actions, []);
    native(form, '', 4);
    assert.equal(form.state.queryError, null);
    assert.equal(form.state.query, '');
    assert.deepEqual(form.output.actions, []);
  });
}

for (const target of ['desktop', 'tui']) {
  const rootModule = await import(
    pathToFileURL(resolve(root, `dist/packages/workspace/${target}.js`))
  );
  rootModule.activate({ locale: 'en' });
  const { component: paletteComponent } = await import(
    pathToFileURL(resolve(root, `dist/packages/command-palette/${target}.js`))
  );
  const props = (client) =>
    modelSlots(client.output.frame.view.content).find((node) => node.key === 'command-palette')
      .props;
  const choose = (client, name, event) =>
    client.input({ kind: 'component', child: 'command-palette', name, event });
  const open = (client) => {
    client.raw('palette-open');
    return new FormClient(paletteComponent, props(client));
  };
  const native = (form, text = '') =>
    form.input({
      kind: 'component',
      child: 'palette-query',
      name: 'changed',
      event: modelObservation(text, 1),
    });
  const read = (form, text = '') =>
    form.complete(form.find('read'), {
      status: 'succeeded',
      value: { text, metadata: modelObservation(text, 1).snapshot.text },
    });
  test(`${target} Root Dialog preserves original product and routes only conversation commands`, () => {
    const client = new Client(rootModule.component).start();
    const background = structuredClone(client.output.frame.view.background);
    const watch = structuredClone(client.output.subscriptions);
    assert.equal(client.output.frame.view.open, false);
    open(client);
    assert.equal(client.output.frame.view.open, true);
    assert.deepEqual(client.output.frame.view.background, background);
    assert.deepEqual(client.output.subscriptions, watch);
    assert.equal(
      modelSlots(client.output.frame.view.content).find((node) => node.key === 'command-palette')
        .binding,
      'palette',
    );
    choose(client, 'closed', {});
    assert.equal(client.state.dialog, null);
    client.raw('settings');
    client.raw('palette-open');
    assert.equal(client.state.dialog, null);
    assert.ok(
      props(client)
        .commands.filter((command) => command.id !== 'help-open')
        .every((command) => command.disabled),
    );
    assert.equal(
      props(client).commands.find((command) => command.id === 'help-open').disabled,
      false,
    );
    client.raw('deployment-open');
    assert.equal(
      client.state.settingsOpen,
      true,
      'management route cannot bypass its guarded Back',
    );
    assert.equal(client.state.deploymentOpen, false);
    choose(client, 'invoked', { id: 'create' });
    assert.deepEqual(client.output.actions, []);
  });
  test(`${target} Unknown permits original Check while independently disabling session choice`, () => {
    const client = new Client(rootModule.component).start();
    client.component('send', draft);
    client.query('read', { text: 'Hello', metadata: draft.snapshot.text });
    client.complete(client.find('send'), {
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
    const form = open(client);
    assert.equal(form.props.selectionDisabled, true);
    assert.equal(form.props.busy, false);
    native(form, 'Check');
    read(form, 'Check');
    form.query('list', {
      revision: 'directory',
      sessions: [{ ...session, title: 'Check conversation' }],
      before: null,
      after: null,
    });
    const rows = modelSlots(form.output.frame.view).find((node) => node.key === 'palette-rows')
      .props.content.children;
    assert.equal(rows.find((row) => row.key.startsWith('session-')).disabled, true);
    const check = form.props.commands.find((command) => command.id.startsWith('check:'));
    assert.equal(check.disabled, false);
    form.raw(rows.find((row) => row.label.includes('Check')).key);
    read(form, 'Check');
    choose(client, 'invoked', form.output.events[0].value);
    assert.deepEqual(client.find('read').input, reference);
    assert.equal(client.state.command, null);
    assert.equal(client.state.dialog, null);
    const count = client.actions.size;
    choose(client, 'invoked', { id: check.id });
    assert.equal(client.actions.size, count, 'closed palette cannot replay an admitted lookup');
  });
  test(`${target} already-open palette reveals a newly uncertain original operation`, () => {
    const client = new Client(rootModule.component).start();
    client.component('send', draft);
    client.query('read', { text: 'Hello', metadata: draft.snapshot.text });
    const send = client.find('send');
    const form = open(client);
    native(form);
    assert.equal(
      form.props.commands.some((command) => command.id.startsWith('check:')),
      false,
    );
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
    form.update(props(client));
    const row = modelSlots(form.output.frame.view)
      .find((node) => node.key === 'palette-rows')
      .props.content.children.find((row) => row.label.includes('Check'));
    assert.equal(row.disabled, false);
    form.raw(row.key);
    read(form);
    choose(client, 'invoked', form.output.events[0].value);
    assert.deepEqual(client.find('read').input, reference);
    assert.equal(client.state.command, null);
  });
  test(`${target} asynchronous palette stop cannot replace its original invocation with a new run`, () => {
    const client = new Client(rootModule.component).start({
      through: 0,
      run: { invocationId: 'run-A', state: 'running' },
    });
    const form = open(client);
    native(form);
    const command = form.props.commands.find((command) => command.id.startsWith('stop:'));
    const row = modelSlots(form.output.frame.view)
      .find((node) => node.key === 'palette-rows')
      .props.content.children.find((row) => row.label.includes('Stop'));
    form.raw(row.key);
    client.page(
      [
        { sequence: 1, invocationId: 'run-A', fact: { kind: 'ended', failed: false } },
        { sequence: 2, invocationId: 'run-B', fact: { kind: 'opened' } },
      ],
      2,
    );
    assert.equal(client.state.run.invocationId, 'run-B');
    form.update(props(client));
    read(form);
    assert.deepEqual(form.output.events, []);
    choose(client, 'invoked', { id: command.id });
    assert.deepEqual(client.output.actions, [], 'Root also rejects stale invocation identity');
    assert.equal(client.state.dialog, 'palette');
    const newRow = modelSlots(form.output.frame.view)
      .find((node) => node.key === 'palette-rows')
      .props.content.children.find((row) => row.label.includes('Stop'));
    assert.notEqual(newRow.key, row.key);
    assert.equal(newRow.disabled, false);
    form.raw(newRow.key);
    read(form);
    choose(client, 'invoked', form.output.events[0].value);
    assert.deepEqual(client.find('stop').input, { sessionId: session.id, invocationId: 'run-B' });
  });
  test(`${target} palette session beyond sidebar follows original canonical observation without rewriting directory`, () => {
    const client = new Client(rootModule.component).start();
    const catalog = structuredClone(client.state.catalog);
    const command = structuredClone(client.state.command);
    const candidate = { ...session, id: 'outside-sidebar', title: 'Directory candidate' };
    const form = open(client);
    native(form, 'candidate');
    read(form, 'candidate');
    form.query('list', {
      revision: 'independent-directory',
      sessions: [candidate],
      before: null,
      after: null,
    });
    const row = modelSlots(form.output.frame.view)
      .find((node) => node.key === 'palette-rows')
      .props.content.children.find((row) => row.key.startsWith('session-'));
    form.raw(row.key);
    read(form, 'candidate');
    choose(client, 'selected', form.output.events[0].value);
    assert.deepEqual(client.state.catalog, catalog);
    assert.deepEqual(client.state.command, command);
    assert.deepEqual(client.find('observe').input, { sessionId: candidate.id });
    assert.equal(client.state.watch, null);
    assert.equal(client.state.loading, true);
    const canonical = { ...candidate, title: 'Canonical owner title', revision: 2 };
    client.query('observe', { session: canonical, through: 0, run: null, parts: [] });
    assert.deepEqual(client.state.selected, canonical);
    assert.deepEqual(client.state.watch.input, { sessionId: canonical.id, after: 0 });
    assert.equal(client.find('history').input.sessionId, canonical.id);
    assert.deepEqual(client.state.catalog, catalog);
  });
  test(`${target} rejected palette session observation keeps native draft and admits no send`, () => {
    const client = new Client(rootModule.component).start();
    open(client);
    choose(client, 'selected', { ...session, id: 'unavailable' });
    client.complete(client.find('observe'), { status: 'failed', reason: 'session unavailable' });
    assert.equal(client.state.watch, null);
    assert.equal(
      modelSlots(client.output.frame.view).find((node) => node.binding === 'composer').props
        .enabled,
      false,
    );
    client.component('send', draft);
    assert.deepEqual(client.output.actions, []);
    assert.equal(client.state.command, null);
  });
}

for (const target of ['desktop', 'tui']) {
  const { component: picker } = await import(
    pathToFileURL(resolve(root, `dist/packages/directory-picker/${target}.js`))
  );
  const rootModule = await import(
    pathToFileURL(resolve(root, `dist/packages/workspace/${target}.js`))
  );
  rootModule.activate({ locale: 'en' });
  const props = { theme: values.appearance[0], locale: 'en', open: true, busy: false };
  const directory = (name, segments = [], rootId = 'root-0') => ({
    name,
    path: '/projects/' + [name, ...segments].join('/'),
    selection: { rootId, segments, directoryIdentity: 'sha256:' + 'b'.repeat(64) },
  });
  const parent = directory('Maka');
  const child = directory('Child', ['child']);
  const children = (directory, children = [], after = null) => ({
    kind: 'children',
    directory,
    children,
    after,
  });
  const viewport = (form) =>
    modelSlots(form.output.frame.view).find((node) => node.key === 'directory-rows').props;
  const nativeQuery = (form, text = '', revision = 1) =>
    form.input({
      kind: 'component',
      child: 'directory-query',
      name: 'changed',
      event: modelObservation(text, revision),
    });
  const readQuery = (form, text = '') =>
    form.complete(form.find('read'), {
      status: 'succeeded',
      value: { text, metadata: form.state.observation.snapshot.text },
    });
  const press = (form, key) => {
    form.raw(`${form.state.pageKey}-${key}`);
    if (form.output.actions.some((action) => action.method === 'read'))
      readQuery(form, form.state.query);
  };
  const openRow = (form) => {
    form.input({ kind: 'component', child: 'directory-query', name: 'submitted', event: {} });
    if (form.output.actions.some((action) => action.method === 'read'))
      readQuery(form, form.state.query);
  };
  const rootChoice = (client, name, event = {}) =>
    client.input({ kind: 'component', child: 'directory-picker', name, event });

  test(`${target} picker browses original observed roots and ancestors through keyboard and native ReadingArea`, () => {
    const form = new FormClient(picker, props);
    assert.deepEqual(
      form.output.frame.view.shortcuts.map((shortcut) => shortcut.key),
      ['escape'],
    );
    const queryScope = form.output.frame.view.children.find(
      (node) => node.key === 'directory-query-scope',
    );
    assert.deepEqual(
      queryScope.shortcuts.map((shortcut) => shortcut.key),
      ['down', 'up', 'home', 'end'],
    );
    assert.equal(queryScope.children[0].key, 'directory-query');
    assert.equal(queryScope.children[0].props.autoFocus, true);
    assert.deepEqual(form.find('directories').input, { kind: 'roots' });
    press(form, 'choose');
    assert.deepEqual(form.output.events, []);
    nativeQuery(form);
    form.query('directories', { kind: 'roots', roots: [directory('Other', [], 'root-1'), parent] });
    form.raw('down');
    assert.equal(viewport(form).command.target.key, `${form.state.pageKey}-1`);
    openRow(form);
    assert.deepEqual(form.find('directories').input, {
      kind: 'children',
      selection: parent.selection,
      after: null,
    });
    form.query('directories', children(parent, [child]));
    form.input({
      kind: 'component',
      child: 'directory-rows',
      name: 'input',
      event: { kind: 'raw', key: `${form.state.pageKey}-0`, event: {} },
    });
    readQuery(form);
    assert.deepEqual(form.find('directories').input.selection, child.selection);
    form.query('directories', children(child));
    press(form, 'parent');
    assert.deepEqual(form.find('directories').input.selection, parent.selection);
    form.query('directories', children(parent));
    press(form, 'choose');
    assert.deepEqual(form.output.events, [{ name: 'selected', value: parent.selection }]);
    press(form, 'parent');
    assert.deepEqual(form.find('directories').input, { kind: 'roots' });
  });

  test(`${target} picker keeps one bounded page and blocks stale selection during failed navigation`, () => {
    const form = new FormClient(picker, props);
    nativeQuery(form);
    form.query('directories', { kind: 'roots', roots: [parent] });
    openRow(form);
    const first = Array.from({ length: 32 }, (_, at) => directory(`Child ${at}`, [`child-${at}`]));
    form.query('directories', children(parent, first, 'child-31'));
    assert.equal(viewport(form).content.children.length, 32);
    form.raw('last');
    assert.equal(viewport(form).command.target.key, `${form.state.pageKey}-31`);
    press(form, 'more');
    assert.deepEqual(form.find('directories').input, {
      kind: 'children',
      selection: parent.selection,
      after: 'child-31',
    });
    press(form, 'choose');
    assert.deepEqual(form.output.events, []);
    form.complete(form.find('directories'), { status: 'failed', diagnostic: 'changed' });
    press(form, 'choose');
    assert.deepEqual(form.output.events, []);
    assert.equal(
      viewport(form).content.children.length,
      32,
      'failed query retains visible old facts',
    );
    form.raw('refresh');
    assert.equal(form.find('directories').input.after, null);
    form.query('directories', children(parent, [child]));
    assert.equal(
      viewport(form).content.children.length,
      1,
      'next result replaces the original bounded page',
    );
    form.update({ busy: true });
    press(form, 'choose');
    openRow(form);
    assert.deepEqual(form.output.events, []);
    assert.deepEqual(form.output.actions, []);
    form.update({ busy: false });
    press(form, 'choose');
    assert.deepEqual(form.output.events, [{ name: 'selected', value: parent.selection }]);
  });

  test(`${target} closed and reopened picker ignores its original late directory reply`, () => {
    const form = new FormClient(picker, { ...props, open: false });
    assert.deepEqual(form.output.actions, []);
    form.update({ open: true });
    const old = form.find('directories');
    form.raw('close');
    assert.deepEqual(form.output.events, [{ name: 'closed', value: {} }]);
    form.update({ open: false });
    form.update({ open: true });
    const fresh = form.output.actions[0];
    assert.notEqual(fresh.key, old.key);
    form.complete(old, {
      status: 'succeeded',
      value: { kind: 'roots', roots: [directory('Stale')] },
    });
    assert.equal(form.state.page, null);
    assert.equal(form.state.pending.key, fresh.key);
    form.complete(fresh, { status: 'succeeded', value: { kind: 'roots', roots: [parent] } });
    assert.deepEqual(form.state.page.roots, [parent]);
  });

  test(`${target} picker page identity prevents stale row and create callbacks from rebinding`, () => {
    const form = new FormClient(picker, props);
    nativeQuery(form);
    form.query('directories', { kind: 'roots', roots: [parent] });
    const rootKey = viewport(form).content.children[0].key;
    form.raw(rootKey);
    readQuery(form);
    form.query('directories', children(parent, [child], 'child'));
    const oldRow = viewport(form).content.children[0].key;
    const oldChoose = `${form.state.pageKey}-choose`;
    press(form, 'more');
    form.query('directories', children(parent, [directory('Different', ['different'])]));
    assert.notEqual(viewport(form).content.children[0].key, oldRow);
    form.raw(oldRow);
    assert.deepEqual(form.output.actions, []);
    form.raw(oldChoose);
    assert.deepEqual(form.output.actions, []);
    assert.deepEqual(form.output.events, []);
    form.raw(rootKey);
    assert.deepEqual(form.output.actions, []);
  });

  test(`${target} picker original text capture preserves page row identity and superseded choices cannot select`, () => {
    const form = new FormClient(picker, props);
    nativeQuery(form);
    form.query('directories', { kind: 'roots', roots: [directory('Other'), parent] });
    const originalKey = viewport(form).content.children[1].key;
    nativeQuery(form, 'Maka', 2);
    const originalRead = form.find('read');
    assert.deepEqual(originalRead.resources.text, {
      child: 'directory-query',
      method: 'snapshot',
      input: { kind: 'at_revision', owner: 'model-editor', revision: 2 },
    });
    readQuery(form, 'Maka');
    assert.equal(
      viewport(form).content.children[0].key,
      originalKey,
      'filter retains original page index',
    );
    assert.equal(viewport(form).content.children.length, 1);
    form.input({ kind: 'component', child: 'directory-query', name: 'submitted', event: {} });
    const choice = form.find('read');
    nativeQuery(form, 'Other', 3);
    form.complete(choice, {
      status: 'succeeded',
      value: { text: 'Maka', metadata: modelObservation('Maka', 2).snapshot.text },
    });
    assert.deepEqual(form.output.events, []);
    assert.ok(
      form.output.actions.every((action) => action.method === 'read'),
      'a superseded read cannot browse its old selected folder',
    );
    form.complete(form.find('read'), { status: 'failed', diagnostic: 'capture unavailable' });
    assert.equal(form.state.queryError, 'unavailable');
    form.raw(originalKey);
    assert.deepEqual(form.output.actions, []);
    form.raw('refresh');
    readQuery(form, 'Other');
    assert.equal(viewport(form).content.children.length, 1);
    nativeQuery(form, 'x'.repeat(1025), 4);
    assert.equal(form.state.queryError, 'long');
    assert.deepEqual(form.output.actions, []);
  });

  test(`${target} Root retains both ordinary dialog slots while switching and cancelling`, () => {
    const client = new Client(rootModule.component).start();
    const mounted = () =>
      modelSlots(client.output.frame.view.content).map((node) => ({
        key: node.key,
        binding: node.binding,
      }));
    const closed = mounted();
    client.raw('palette-open');
    assert.deepEqual(mounted(), closed);
    client.input({
      kind: 'component',
      child: 'command-palette',
      name: 'invoked',
      event: { id: 'create-workspace' },
    });
    assert.deepEqual(mounted(), closed);
    const slots = modelSlots(client.output.frame.view.content);
    assert.equal(slots.find((node) => node.key === 'command-palette').props.open, false);
    assert.equal(slots.find((node) => node.key === 'directory-picker').props.open, true);
    rootChoice(client, 'closed');
    client.raw('palette-open');
    assert.deepEqual(mounted(), closed);
    assert.equal(
      modelSlots(client.output.frame.view.content).find((node) => node.key === 'command-palette')
        .props.open,
      true,
    );
  });

  test(`${target} Root uses one controlled dialog and original recorded create without changing retained execution`, () => {
    const client = new Client(rootModule.component).start({
      run: { invocationId: 'original-run', state: 'running' },
    });
    const original = structuredClone({
      epoch: client.state.epoch,
      selected: client.state.selected,
      window: client.state.window,
      run: client.state.run,
      command: client.state.command,
      watch: client.output.subscriptions,
    });
    client.raw('palette-open');
    client.input({
      kind: 'component',
      child: 'command-palette',
      name: 'invoked',
      event: { id: 'create-workspace' },
    });
    assert.equal(client.state.dialog, 'directories');
    assert.equal(
      modelSlots(client.output.frame.view.content).find((node) => node.key === 'directory-picker')
        .binding,
      'directories',
    );
    assert.deepEqual(client.output.actions, []);
    client.raw('palette-open');
    client.raw('create');
    assert.equal(client.state.dialog, 'directories');
    assert.deepEqual(client.output.actions, []);
    rootChoice(client, 'selected', child.selection);
    assert.equal(client.state.dialog, null);
    const create = client.find('create');
    assert.deepEqual(create.input, {
      title: 'New conversation',
      labels: [],
      workspace: { kind: 'directory', selection: child.selection },
    });
    assert.deepEqual(
      {
        epoch: client.state.epoch,
        selected: client.state.selected,
        window: client.state.window,
        run: client.state.run,
        command: client.state.command,
        watch: client.output.subscriptions,
      },
      original,
    );
    rootChoice(client, 'selected', parent.selection);
    assert.deepEqual(client.output.actions, [], 'closed component cannot replay creation');
    client.complete(create, returned({ kind: 'rejected', reason: 'workspace_unavailable' }));
    assert.deepEqual(
      {
        epoch: client.state.epoch,
        selected: client.state.selected,
        window: client.state.window,
        run: client.state.run,
        command: client.state.command,
        watch: client.output.subscriptions,
      },
      original,
    );
    assert.equal(client.state.uncertain, null);
  });

  test(`${target} Root rechecks current creation guard before accepting an asynchronous picker choice`, () => {
    const client = new Client(rootModule.component).start();
    client.raw('create-workspace');
    client.component('refresh');
    const listing = client.find('list');
    rootChoice(client, 'selected', child.selection);
    assert.equal(client.state.dialog, 'directories');
    assert.deepEqual(client.output.actions, []);
    client.complete(listing, {
      status: 'succeeded',
      value: { revision: 'new-catalog', sessions: [session], before: null, after: null },
    });
    rootChoice(client, 'closed');
    rootChoice(client, 'selected', child.selection);
    assert.deepEqual(client.output.actions, []);
    client.raw('create');
    assert.deepEqual(
      client.find('create').input.workspace,
      { kind: 'session', sessionId: session.id },
      'ordinary New still reuses immutable original workspace',
    );
  });
}

class DetailsClient extends FormClient {
  constructor(component) {
    super(component, {
      theme: values.appearance[0],
      locale: 'en',
      open: true,
      busy: false,
      session,
    });
    this.texts = new Map();
    this.observe('title', '');
    this.observe('labels', '');
  }
  observe(name, text, status = 'applied', marked = null) {
    this.texts.set(name, text);
    const field = this.state.fields[name],
      command = field.command;
    const revision = (field.observation?.revision ?? 0) + 1;
    return this.input({
      kind: 'component',
      child: 'details-' + name,
      name: 'changed',
      event: {
        revision,
        snapshot: {
          text: { owner: 'details-native-' + name, revision, bytes: Buffer.byteLength(text) },
          selection: { start: Buffer.byteLength(text), end: Buffer.byteLength(text) },
          reversed: false,
          marked,
        },
        receipt: command
          ? {
              owner: command.owner,
              sequence: command.sequence,
              revision,
              outcome: status === 'applied' ? { status } : { status, reason: 'revision' },
            }
          : null,
      },
    });
  }
  commands() {
    for (const name of ['title', 'labels']) {
      const command = this.state.fields[name].command;
      if (command) this.observe(name, command.edit.text);
    }
    return this;
  }
  capture() {
    this.raw('save');
    return [...this.actions.values()]
      .filter((a) => a.binding === 'reader')
      .map((action) => ({
        action,
        text: this.texts.get(action.resources.text.child.slice(8)),
      }));
  }
  readAll() {
    const captures = this.capture();
    for (const { action, text } of captures)
      this.complete(action, {
        status: 'succeeded',
        value: {
          text,
          metadata: {
            owner: action.resources.text.input.owner,
            revision: action.resources.text.input.revision,
            bytes: action.input.end,
          },
        },
      });
    return captures;
  }
}
const metadataRef = { ...reference, method: 'metadata' };
const metadataReply = (value) => ({
  status: 'operation',
  reply: {
    reference: metadataRef,
    receipt: {
      kind: 'recorded',
      outcome: { kind: 'returned', value },
      cleanup: { kind: 'confirmed' },
    },
  },
});
for (const target of ['desktop', 'tui']) {
  const { component: details } = await import(
    pathToFileURL(resolve(root, `dist/packages/session-details/${target}.js`))
  );
  const slots = (form) => modelSlots(form.output.frame.view);
  const button = (form, key) => {
    const find = (node) =>
      node.key === key ? node : (node.children ?? []).map(find).find(Boolean);
    return find(form.output.frame.view);
  };
  test(`${target} metadata waits for original initialization and recovers failed labels without replacing the slot`, () => {
    const form = new DetailsClient(details);
    assert.equal(button(form, 'save').disabled, true);
    const owner = slots(form).find((n) => n.key === 'details-labels').key;
    form.observe('title', session.title);
    form.observe('labels', '', 'conflict');
    assert.equal(button(form, 'retry-labels').disabled, false);
    assert.equal(button(form, 'save').disabled, true);
    form.raw('retry-labels');
    form.commands();
    assert.equal(slots(form).find((n) => n.key === owner).key, owner);
    assert.equal(button(form, 'save').disabled, false);
    form.observe('labels', '未完成', 'applied', { start: 0, end: 3 });
    form.raw('save');
    assert.equal(form.output.actions.length, 0);
  });
  test(`${target} metadata captures both original revisions and freezes CAS despite run props and later typing`, () => {
    const form = new DetailsClient(details).commands();
    form.observe('title', 'Thoughtful 🙂');
    form.observe('labels', 'rust\r\n设计');
    const captures = form.capture();
    assert.equal(captures.length, 2);
    const captured = structuredClone(form.state.current.captured);
    form.update({ session: { ...session, revision: 9 } });
    form.observe('title', 'More typing');
    for (const { action, text } of captures)
      form.complete(action, {
        status: 'succeeded',
        value: { text, metadata: captured[action.resources.text.child.slice(8)].snapshot.text },
      });
    const mutation = form.find('metadata');
    assert.deepEqual(mutation.input, {
      sessionId: session.id,
      expectedRevision: 1,
      title: 'Thoughtful 🙂',
      labels: ['rust', '设计'],
    });
    const observation = structuredClone(form.state.fields.title.observation);
    form.complete(
      mutation,
      metadataReply({
        kind: 'committed',
        session: { ...session, title: 'Thoughtful 🙂', labels: ['rust', '设计'], revision: 10 },
      }),
    );
    assert.equal(form.state.target.revision, 10);
    assert.deepEqual(form.state.fields.title.observation, observation);
    assert.equal(form.state.fields.title.command, null);
    assert.deepEqual(form.output.events, [{ name: 'changed', value: { sessionId: session.id } }]);
  });
  test(`${target} failed capture and closed or switched target never reach metadata`, () => {
    for (const change of ['failure', 'close', 'switch']) {
      const form = new DetailsClient(details).commands();
      const captures = form.capture();
      if (change === 'failure')
        form.complete(captures[0].action, {
          status: 'failed',
          reason: 'snapshot_revision_changed',
        });
      else
        form.update(
          change === 'close' ? { open: false } : { session: { ...session, id: 'other' } },
        );
      for (const { action, text } of captures)
        if (form.actions.has(action.key))
          form.complete(action, {
            status: 'succeeded',
            value: {
              text,
              metadata: {
                owner: action.resources.text.input.owner,
                revision: action.resources.text.input.revision,
                bytes: action.input.end,
              },
            },
          });
      assert.equal(
        [...form.actions.values()].some((a) => a.method === 'metadata'),
        false,
      );
      assert.equal(form.state.current, null);
    }
  });
  test(`${target} conflicts require explicit native review and keep fields through basis adoption`, () => {
    const form = new DetailsClient(details).commands();
    form.observe('title', 'My draft');
    form.readAll();
    form.complete(
      form.find('metadata'),
      metadataReply({ kind: 'revision_conflict', expected: 1, actual: 3 }),
    );
    assert.equal(form.state.conflict, true);
    form.raw('save');
    assert.equal(form.output.actions.length, 0);
    form.update({ session: { ...session, revision: 8 } });
    assert.equal(form.state.target.revision, 1);
    form.raw('review');
    form.query('observe', {
      session: { ...session, revision: 8, title: 'Canonical' },
      through: 0,
      parts: [],
      run: null,
    });
    const fields = structuredClone(form.state.fields);
    form.raw('use-version');
    assert.deepEqual(form.state.fields, fields);
    assert.equal(form.state.target.revision, 8);
    assert.equal(form.output.actions.length, 0);
    form.readAll();
    assert.equal(form.find('metadata').input.title, 'My draft');
  });
  test(`${target} target changes only discard drafts through confirmed reload`, () => {
    const form = new DetailsClient(details).commands();
    form.observe('title', 'Old draft');
    form.update({ session: { ...session, id: 'new', revision: 2, title: 'New target' } });
    form.raw('review');
    assert.equal(form.find('observe').input.sessionId, 'new');
    form.query('observe', { session: form.props.session, through: 0, parts: [], run: null });
    form.raw('use-version');
    assert.equal(form.state.target.id, session.id);
    assert.equal(form.state.fields.title.command, null);
    form.raw('reload');
    assert.equal(form.state.fields.title.command, null);
    form.raw('use-version');
    assert.equal(form.state.target.id, 'new');
    assert.equal(form.state.fields.title.command.edit.text, 'New target');
    assert.equal(button(form, 'save').disabled, true);
    form.commands();
    assert.equal(button(form, 'save').disabled, false);
  });
  test(`${target} native validation rejection allows correction and never holds an unknown operation`, () => {
    const form = new DetailsClient(details).commands();
    form.observe('title', ' ');
    form.readAll();
    assert.equal(form.find('metadata').input.title, ' ');
    form.complete(
      form.find('metadata'),
      metadataReply({ kind: 'rejected', reason: 'title_invalid' }),
    );
    assert.equal(form.state.current, null);
    assert.equal(form.state.notice, 'detailsInvalid');
    form.observe('title', 'Fixed');
    form.readAll();
    assert.equal(form.find('metadata').input.title, 'Fixed');
  });
  test(`${target} unknown metadata reads exact original receipt and bounded history cannot adopt a basis`, () => {
    const form = new DetailsClient(details).commands();
    form.readAll();
    form.complete(form.find('metadata'), {
      status: 'operation',
      reply: {
        reference: metadataRef,
        receipt: {
          kind: 'recorded',
          outcome: { kind: 'unknown', diagnostic: 'lost' },
          cleanup: { kind: 'confirmed' },
        },
      },
    });
    form.update({ session: { ...session, revision: 20 } });
    form.raw('save');
    assert.equal(form.output.actions.length, 0);
    form.raw('check');
    const check = form.find('read');
    assert.deepEqual(check.input, metadataRef);
    form.complete(check, {
      status: 'succeeded',
      value: {
        ...metadataReply({ kind: 'committed', session: { ...session, revision: 20 } }).reply,
        reference: { ...metadataRef, operation: uuid(900) },
      },
    });
    assert.ok(form.state.current);
    assert.equal(form.state.target.revision, 1);
    form.raw('history');
    assert.deepEqual(form.find('page').input, { after: 0, limit: 4 });
    form.query('page', [{ cursor: 5, reference: { ...metadataRef, operation: uuid(5) } }]);
    form.raw('earlier-5');
    const earlier = form.find('read');
    form.complete(earlier, {
      status: 'succeeded',
      value: {
        ...metadataReply({ kind: 'committed', session: { ...session, revision: 50 } }).reply,
        reference: earlier.input,
      },
    });
    assert.equal(form.state.target.revision, 1);
    assert.ok(form.state.current);
    form.raw('history-back');
    form.raw('check');
    form.query(
      'read',
      metadataReply({ kind: 'committed', session: { ...session, revision: 21 } }).reply,
    );
    assert.equal(form.state.current, null);
    assert.equal(form.state.target.revision, 21);
    assert.equal(form.state.fields.title.command, null);
  });
}

for (const target of ['desktop', 'tui']) {
  const rootModule = await import(
    pathToFileURL(resolve(root, `dist/packages/workspace/${target}.js`))
  );
  rootModule.activate({ locale: 'en' });
  const changed = (client) =>
    client.input({
      kind: 'component',
      child: 'session-details',
      name: 'changed',
      event: { sessionId: session.id },
    });
  const nativeFacts = (client) =>
    structuredClone({
      epoch: client.state.epoch,
      selected: client.state.selected,
      run: client.state.run,
      window: client.state.window,
      command: client.state.command,
      watch: client.output.subscriptions,
    });
  test(`${target} Root metadata dialog retains running conversation and semantically blocks switching`, () => {
    const client = new Client(rootModule.component).start({
      run: { invocationId: 'run', state: 'running' },
    });
    client.component('draft', draft);
    const original = nativeFacts(client);
    client.raw('details-open');
    assert.equal(client.state.dialog, 'details');
    const mounted = modelSlots(client.output.frame.view.content);
    assert.equal(mounted.find((n) => n.key === 'session-details').props.session.id, session.id);
    assert.equal(mounted.find((n) => n.key === 'session-details').props.open, true);
    assert.deepEqual(nativeFacts(client), original);
    client.component('select', { sessionId: session.id });
    client.raw('create');
    client.raw('settings');
    assert.equal(client.output.actions.length, 0);
    assert.equal(client.state.dialog, 'details');
    assert.deepEqual(nativeFacts(client), original);
    client.input({ kind: 'component', child: 'session-details', name: 'closed', event: {} });
    assert.equal(client.state.dialog, null);
    assert.deepEqual(nativeFacts(client), original);
  });
  test(`${target} Root refreshes native metadata without replacing epoch and rejects older metadata replies`, () => {
    const client = new Client(rootModule.component).start();
    const original = nativeFacts(client);
    changed(client);
    const first = client.find('observe');
    changed(client);
    const second = [...client.actions.values()].filter((a) => a.method === 'observe').at(-1);
    assert.notEqual(first.key, second.key);
    client.complete(second, {
      status: 'succeeded',
      value: {
        session: { ...session, title: 'Canonical newer', revision: 5 },
        through: 0,
        parts: [],
        run: null,
      },
    });
    client.complete(first, {
      status: 'succeeded',
      value: {
        session: { ...session, title: 'Canonical older', revision: 2 },
        through: 0,
        parts: [],
        run: null,
      },
    });
    assert.equal(client.state.selected.title, 'Canonical newer');
    assert.equal(client.state.selected.revision, 5);
    assert.equal(client.state.epoch, original.epoch);
    assert.deepEqual(client.state.command, original.command);
    assert.deepEqual(client.state.window, original.window);
    assert.deepEqual(client.output.subscriptions, original.watch);
    const before = structuredClone(client.state.selected);
    client.input({
      kind: 'component',
      child: 'session-details',
      name: 'changed',
      event: { sessionId: 'other' },
    });
    assert.ok(client.output.actions.every((a) => a.method === 'list'));
    assert.deepEqual(client.state.selected, before);
  });
}

for (const target of ['tui', 'desktop']) {
  test(`${target} help is a replaceable presentation with no business authority`, async () => {
    const { component, activate } = await import(
      pathToFileURL(resolve(root, `dist/packages/help/${target}.js`))
    );
    const props = { theme: values.appearance[0], locale: 'en', open: true };
    let result = component.mount(props, {});
    assert.equal(activate().Help.kind, 'component');
    assert.deepEqual(result.actions, []);
    assert.deepEqual(result.subscriptions, []);
    assert.deepEqual(result.frame.view.shortcuts, []);
    const rows = modelSlots(result.frame.view).find((node) => node.key === 'help-reading').props
      .content;
    assert.equal(rows.scroll, 'manual');
    assert.match(rows.children[0].text, target === 'tui' ? /^Enter/ : /^Ctrl\/⌘ Enter/);
    result = component.event(
      result.frame.state,
      { kind: 'raw', key: 'close', event: {} },
      props,
      {},
    );
    assert.deepEqual(result.events, [{ name: 'closed', value: {} }]);
    result = component.event(
      result.frame.state,
      { kind: 'raw', key: 'close', event: {} },
      { ...props, open: false },
      {},
    );
    assert.deepEqual(result.events, []);
    const manifest = JSON.parse(
      readFileSync(resolve(root, 'dist/packages/help/maka.extension.json'), 'utf8'),
    );
    assert.deepEqual(Object.keys(manifest.application.entries[target].imports), ['reading']);
  });
  test(`${target} F1 keeps the underlying route and command owner across help dismissal`, async () => {
    const { component } = await import(
      pathToFileURL(resolve(root, `dist/packages/workspace/${target}.js`))
    );
    const client = new Client(component).start();
    for (const route of [null, 'settings', 'deployment-open']) {
      if (route) client.raw(route);
      const before = structuredClone(client.state);
      client.raw('help-open');
      assert.equal(client.state.dialog, 'help');
      assert.equal(client.state.settingsOpen, before.settingsOpen);
      assert.equal(client.state.deploymentOpen, before.deploymentOpen);
      assert.equal(client.state.navigationOpen, before.navigationOpen);
      assert.deepEqual(client.state.command, before.command);
      assert.deepEqual(client.state.watch, before.watch);
      assert.deepEqual(client.output.actions, []);
      client.raw('create');
      client.raw('palette-open');
      assert.equal(client.state.dialog, 'help');
      assert.deepEqual(client.output.actions, []);
      client.input({ kind: 'component', child: 'help', name: 'closed', event: {} });
      assert.equal(client.state.dialog, null);
      assert.equal(client.state.settingsOpen, before.settingsOpen);
      assert.equal(client.state.deploymentOpen, before.deploymentOpen);
      if (route)
        client.input({
          kind: 'component',
          child: route === 'settings' ? 'connections' : 'deployment',
          name: 'closed',
          event: {},
        });
    }
    client.raw('palette-open');
    client.input({ kind: 'component', child: 'help', name: 'closed', event: {} });
    client.raw('help-open');
    assert.equal(client.state.dialog, 'palette', 'F1 cannot replace another modal');
    client.input({
      kind: 'component',
      child: 'command-palette',
      name: 'invoked',
      event: { id: 'help-open' },
    });
    assert.equal(client.state.dialog, 'help', 'palette can explicitly select help');
    client.input({ kind: 'component', child: 'command-palette', name: 'closed', event: {} });
    assert.equal(client.state.dialog, 'help', 'old modal dismissal cannot dismiss help');
  });
}

test('queue revisions advance independently of transcript delivery and reject stale snapshots', () => {
  const client = new Client().start();
  const queue = (revision, preview) =>
    client.input({
      kind: 'action',
      key: client.state.watch.key,
      event: {
        status: 'updated',
        value: {
          kind: 'queue',
          queue: {
            revision,
            entries: [{ messageId: 'queued', kind: 'followup', preview }],
          },
        },
      },
    });
  client.page([], 5);
  queue(3, 'original');
  assert.equal(client.state.watch.last, 5);
  assert.equal(client.state.latestKnown, 5);
  assert.equal(client.state.queue.entries[0].preview, 'original');
  client.page([], 8);
  assert.equal(client.state.queue.revision, 3);
  queue(4, 'edited');
  queue(3, 'late');
  assert.equal(client.state.queue.entries[0].preview, 'edited');
  assert.equal(client.state.watch.last, 8);
});

test('TUI queue controls retain original revision and lane and working-send keeps draft receipts', async () => {
  const terminal = await import(pathToFileURL(resolve(root, 'dist/packages/workspace/tui.js')));
  terminal.activate({ locale: 'en' });
  const client = new Client(terminal.component).start({
    run: { invocationId: 'run', state: 'running' },
  });
  client.component('send', draft);
  assert.equal(
    client.output.actions.length,
    0,
    'working-send waits for original queue observation',
  );
  const queue = {
    revision: 7,
    entries: [
      { messageId: 'first', kind: 'followup', preview: 'first' },
      { messageId: 'steering', kind: 'steering', preview: 'steering' },
      { messageId: 'second', kind: 'followup', preview: 'second' },
    ],
  };
  client.input({
    kind: 'action',
    key: client.state.watch.key,
    event: { status: 'updated', value: { kind: 'queue', queue } },
  });
  const rows = modelSlots(client.output.frame.view).find((node) => node.binding === 'rows');
  assert.deepEqual(
    rows.props.rows.map((row) => row.key),
    ['first', 'steering', 'second'],
  );
  const activate = (revision, row, action) =>
    client.input({
      kind: 'component',
      key: 'queue-action',
      child: 'queue',
      name: 'activated',
      event: { revision, row, action },
    });
  activate(6, 'first', 'retract');
  assert.equal(client.output.actions.length, 0, 'old row never mutates a newer queue');
  activate(7, 'second', 'up');
  const reorder = client.find('editQueue');
  assert.deepEqual(reorder.input, {
    sessionId: session.id,
    expectedRevision: 7,
    edit: { kind: 'reorder', messageIds: ['second', 'first'] },
  });
  client.complete(reorder, returned({ kind: 'committed', revision: 8 }));
  assert.equal(client.state.queue.revision, 7, 'only canonical watch changes queue state');
  activate(7, 'first', 'promote');
  const promote = client.find('editQueue');
  assert.deepEqual(promote.input.edit, { kind: 'promote', messageId: 'first' });
  client.complete(promote, returned({ kind: 'revision_conflict', expected: 7, actual: 8 }));
  assert.equal(client.state.notice, 'queueChanged');
  client.component('send', draft);
  const read = client.find('read');
  assert.equal(read.resources.text.method, 'snapshot');
  client.query('read', {
    text: 'Hello',
    metadata: { owner: 'original-editor', revision: 4, bytes: 5 },
  });
  const send = client.find('send');
  assert.equal(client.state.command, null);
  client.complete(
    send,
    returned({ kind: 'queued', sessionId: session.id, messageId: 'queued', revision: 9 }),
  );
  assert.equal(client.state.command.owner, 'original-editor');
  assert.equal(client.state.command.basedOn, 4);
  assert.equal(client.state.run.invocationId, 'run');
});

async function queueEditorClient() {
  const terminal = await import(pathToFileURL(resolve(root, 'dist/packages/workspace/tui.js')));
  terminal.activate({ locale: 'en' });
  const client = new Client(terminal.component).start({
    run: { invocationId: 'run', state: 'running' },
  });
  client.queue = (revision = 7) =>
    client.input({
      kind: 'action',
      key: client.state.watch.key,
      event: {
        status: 'updated',
        value: {
          kind: 'queue',
          queue: {
            revision,
            entries: [{ messageId: 'queued', kind: 'followup', preview: 'Hello' }],
          },
        },
      },
    });
  client.edit = () =>
    client.input({
      kind: 'component',
      child: 'queue',
      name: 'activated',
      event: { revision: client.state.queue.revision, row: 'queued', action: 'edit' },
    });
  client.editor = (name, event = {}) =>
    client.input({ kind: 'component', child: 'queue-editor', name, event });
  client.ready = () => {
    client.query('queueMessage', {
      kind: 'ready',
      revision: client.state.queueEdit.target.expectedRevision,
      message: 'Hello',
    });
    const command = client.state.queueEdit.command;
    const observation = structuredClone(draft);
    observation.receipt = {
      owner: command.owner,
      sequence: command.sequence,
      outcome: { status: 'applied' },
    };
    client.editor('changed', observation);
    return observation;
  };
  client.queue();
  client.editor('changed', draft); // Native owner announced while its retained slot was hidden.
  client.edit();
  return client;
}

test('queue body initialization and save retain the original target and CAS revision', async () => {
  const client = await queueEditorClient();
  assert.deepEqual(client.find('queueMessage').input, {
    sessionId: session.id,
    messageId: 'queued',
    expectedRevision: 7,
  });
  client.editor('save', draft);
  assert.equal(client.output.actions.length, 0, 'body load does not enable save');
  const observation = client.ready();
  assert.equal(client.state.queueEdit.phase, 'ready');
  client.queue(8);
  client.editor('save', observation);
  const read = client.find('read');
  assert.deepEqual(read.resources.text, {
    child: 'queue-editor',
    method: 'snapshot',
    input: { kind: 'at_revision', owner: draft.snapshot.text.owner, revision: 4 },
  });
  client.query('read', { text: 'Hello', metadata: draft.snapshot.text });
  assert.deepEqual(client.find('editQueue').input, {
    sessionId: session.id,
    expectedRevision: 7,
    edit: { kind: 'update', messageId: 'queued', message: 'Hello' },
  });
  client.complete(
    client.find('editQueue'),
    returned({ kind: 'revision_conflict', expected: 7, actual: 8 }),
  );
  assert.equal(client.state.dialog, 'queue');
  assert.equal(client.state.queueEdit.target.expectedRevision, 7);
  assert.equal(client.state.queueEdit.command, null, 'conflict never resets the draft');
  assert.deepEqual(client.state.queueDraft, observation);
  client.editor('reload');
  assert.equal(
    client.find('queueMessage').input.expectedRevision,
    8,
    'only explicit discard/reload adopts watch version',
  );
});

test('queue initialization requires Applied and a body delivered before the native announcement is retained', async () => {
  const client = await queueEditorClient();
  client.output.frame.state.queueDraft = null;
  client.query('queueMessage', { kind: 'ready', revision: 7, message: 'Hello' });
  assert.equal(client.state.queueEdit.command, null);
  client.editor('changed', draft);
  const command = client.state.queueEdit.command;
  assert.equal(command.edit.text, 'Hello');
  client.editor('save', draft);
  assert.equal(client.output.actions.length, 0);
  client.editor('changed', {
    ...draft,
    receipt: {
      owner: command.owner,
      sequence: command.sequence,
      outcome: { status: 'conflict', reason: 'revision' },
    },
  });
  assert.equal(client.state.queueEdit.phase, 'failed');
  client.editor('save', draft);
  assert.equal(
    client.output.actions.length,
    0,
    'a rejected reset never saves stale retained editor text',
  );
});

for (const stage of ['body', 'capture'])
  test(`cancelled queue ${stage} completion cannot mutate or initialize a later editor`, async () => {
    const client = await queueEditorClient();
    let action;
    if (stage === 'body') action = client.find('queueMessage');
    else {
      const observation = client.ready();
      client.editor('save', observation);
      action = client.find('read');
    }
    client.raw('workspace-dialog');
    assert.equal(client.state.queueEdit, null);
    client.complete(action, {
      status: 'succeeded',
      value:
        stage === 'body'
          ? { kind: 'ready', revision: 7, message: 'late' }
          : { text: 'Hello', metadata: draft.snapshot.text },
    });
    assert.equal(client.output.actions.length, 0);
    assert.equal(client.state.queueEdit, null);
  });

for (const chunk of [
  { text: 'Hello', metadata: { ...draft.snapshot.text, owner: 'other' } },
  { text: '界界界界界', metadata: draft.snapshot.text },
])
  test('queue snapshot rejects mismatched owner or actual UTF-8 length', async () => {
    const client = await queueEditorClient();
    const observation = client.ready();
    client.editor('save', observation);
    client.query('read', chunk);
    assert.equal(client.output.actions.length, 0);
    assert.equal(client.state.notice, 'draftKept');
    assert.equal(client.state.queueEdit.phase, 'ready');
  });

test('unknown queue update checks the original receipt without replay and success closes only the captured draft', async () => {
  const client = await queueEditorClient();
  const observation = client.ready();
  client.editor('save', observation);
  client.query('read', { text: 'Hello', metadata: draft.snapshot.text });
  const update = client.find('editQueue');
  const ref = { ...reference, method: 'editQueue' };
  client.complete(update, {
    status: 'operation',
    reply: { reference: ref, receipt: { kind: 'unknown' } },
  });
  client.editor('save', observation);
  assert.equal(client.output.actions.length, 0);
  client.editor('reload');
  assert.equal(client.output.actions.length, 0);
  client.editor('check');
  const check = client.find('read');
  assert.equal(check.binding, 'receipts');
  assert.deepEqual(check.input, ref);
  client.complete(check, {
    status: 'succeeded',
    value: { ...returned({ kind: 'committed', revision: 8 }).reply, reference: ref },
  });
  assert.equal(client.state.dialog, null);
  assert.equal(client.state.queueEdit, null);
  assert.equal(client.state.command, null, 'queue editing never touches Composer');
});

test('queue committed receipt preserves a newer native draft and cancelled operation cannot close another modal', async () => {
  for (const change of ['newer', 'cancel']) {
    const client = await queueEditorClient();
    const observation = client.ready();
    client.editor('save', observation);
    client.query('read', { text: 'Hello', metadata: draft.snapshot.text });
    const update = client.find('editQueue');
    if (change === 'newer') client.editor('changed', { ...observation, revision: 5 });
    else {
      client.editor('closed');
      client.raw('help-open');
    }
    client.complete(update, returned({ kind: 'committed', revision: 8 }));
    assert.equal(client.state.dialog, change === 'newer' ? 'queue' : 'help');
  }
});

test('queue-editor retains the native Textarea, forwards exact capture and keeps Enter as newline', async () => {
  const { component } = await import(
    pathToFileURL(resolve(root, 'dist/packages/queue-editor/tui.js'))
  );
  const props = {
    theme: values.appearance[0],
    locale: 'en',
    open: true,
    ready: true,
    busy: false,
    uncertain: false,
    checking: false,
    status: '',
    command: null,
  };
  let result = component.mount(props, {});
  const field = modelSlots(result.frame.view).find((node) => node.binding === 'editor');
  assert.equal(field.props.submit, 'primary_enter');
  assert.equal(field.props.softWrap, true);
  assert.deepEqual(field.props.terminalRows, { min: 3, max: 10 });
  const fieldContainer = result.frame.view.children.find((node) => node.key === 'field');
  assert.deepEqual(fieldContainer.layout, { shrink: true });
  const changed = { kind: 'component', child: 'draft', name: 'changed', event: draft };
  result = component.event(result.frame.state, changed, props, {});
  result = component.event(result.frame.state, { ...changed, name: 'submitted' }, props, {});
  assert.deepEqual(
    result.events.map((e) => e.name),
    ['changed', 'save'],
  );
  result = component.event(
    result.frame.state,
    { ...changed, name: 'submitted' },
    { ...props, busy: true },
    {},
  );
  assert.deepEqual(
    result.events.map((e) => e.name),
    ['changed'],
  );
  const input = { kind: 'at_revision', owner: 'original-editor', revision: 4 };
  assert.deepEqual(component.capture(result.frame.state, props, 'snapshot', input), {
    kind: 'forward',
    source: { child: 'draft', method: 'snapshot', input },
  });
});
