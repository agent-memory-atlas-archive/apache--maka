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
const appearance = await import('../dist/packages/appearance/tui.js');
const theme = appearance.activate({ scheme: 'dark' }).Appearance.value;
const module = await import('../dist/packages/agent-screen/tui.js');
module.activate({ locale: 'en' });
const values = { appearance: [theme] };
const dataSource = {
  stateDomain: 'a'.repeat(64),
  deployment: '00000000-0000-0000-0000-000000000001',
  namespace: ['00000000-0000-0000-0000-000000000002'],
};
const session = (id) => ({
  id,
  title: id,
  labels: [],
  revision: 1,
  updatedAt: 1,
  archived: false,
  activity: 'idle',
  boundaryRevision: 0,
  hasUnread: false,
  dataSource,
  permissions: { access: 'workspace_write', requestPermissions: true },
  workspace: { path: '/project', directoryIdentity: 'a'.repeat(64), identity: 'workspace-a' },
});
const chunk = (sequence, text, extra = {}) => ({
  messageId: 'message-' + sequence,
  turnId: 'turn-1',
  timestamp: 1,
  role: 'assistant',
  field: 'body',
  sequence,
  offset: 0,
  totalBytes: Buffer.byteLength(text),
  text,
  tool: null,
  ...extra,
});
const page = (through, chunks, extra = {}) => ({
  kind: 'ready',
  through,
  chunks,
  before: null,
  after: null,
  ...extra,
});
function nodes(node) {
  return node && typeof node === 'object'
    ? [
        node,
        ...Object.values(node).flatMap((v) => (Array.isArray(v) ? v.flatMap(nodes) : nodes(v))),
      ]
    : [];
}
function client() {
  let out = module.component.mount({}, values);
  const c = {
    get out() {
      return out;
    },
    get state() {
      return out.frame.state;
    },
    event(input) {
      out = module.component.event(c.state, input, {}, values);
      return c;
    },
    raw(key) {
      return c.event({ kind: 'raw', key, event: { kind: 'activate' } });
    },
    child(child, name, event = {}) {
      return c.event({ kind: 'component', key: name, child, name, event });
    },
    reply(action, value) {
      assert.ok(action);
      return c.event({ kind: 'action', key: action.key, event: { status: 'succeeded', value } });
    },
    action(method) {
      return out.actions.find((a) => a.method === method);
    },
    observe(through, run = null) {
      return c.reply(c.action('observe'), { session: c.state.selected, through, parts: [], run });
    },
    history(value) {
      return c.reply(c.action('history'), value);
    },
    select(id) {
      return c.child('navigation', 'select', { sessionId: id });
    },
    find(key) {
      return nodes(out.frame.view).find((node) => node.key === key);
    },
    reader() {
      return c.state.visits.find((visit) => visit.id === c.state.selected?.id).key;
    },
  };
  c.reply(c.action('page'), { entries: [], nextAfter: 0 });
  c.event({
    kind: 'action',
    key: c.state.catalogWatch.key,
    event: {
      status: 'updated',
      value: {
        sessions: [session('Task A'), session('Task B'), session('Task C'), session('Task D')],
        before: null,
        after: null,
      },
    },
  });
  return c;
}

test('switching tasks preserves the native reading owner, page, search basis and auxiliary context', () => {
  const c = client()
    .observe(20)
    .history(page(20, [chunk(10, 'Older Chinese 中文🙂'), chunk(18, 'Newer answer')]));
  const first = c.reader();
  c.raw('search-open');
  const searchEpoch = c.find(first).props.search.epoch;
  c.raw('outputs-open');
  assert.equal(c.state.inspector, 'outputs');
  const landing = structuredClone(c.state.landing);
  c.select('Task B')
    .observe(2)
    .history(page(2, [chunk(1, 'Task B answer')]));
  assert.notEqual(c.reader(), first);
  assert.ok(c.find(first), 'the original reading owner remains mounted while hidden');
  assert.equal(c.find('task-reader-' + first).visible, false);
  c.select('Task A');
  assert.equal(c.reader(), first);
  assert.deepEqual(c.state.landing, landing);
  assert.equal(c.state.window.page.through, 20);
  assert.equal(c.state.inspector, 'outputs');
  assert.equal(
    c.find(first).props.search.epoch,
    searchEpoch,
    'returning does not reset the search input',
  );
  c.observe(25);
  assert.equal(
    c.action('history'),
    undefined,
    'background progress does not replace the saved reading page',
  );
  assert.equal(c.find(first).props.latest, true);
  assert.ok(c.find('task-latest'));
  c.child('timeline-noncurrent', 'source', { sequence: 10 });
  assert.equal(
    c.state.source,
    null,
    'late callbacks from another task do not redirect this reader',
  );
});

test('ended turns describe completion of execution without asserting successful delivery', () => {
  const c = client()
    .observe(20)
    .history(
      page(20, [
        chunk(10, 'Result', {
          timing: { startedAt: 1, finished: { at: 2, outcome: 'completed' } },
        }),
      ]),
    );
  assert.equal(c.find('task-state').text, 'Turn ended');
  assert.equal(c.find('task-face'), undefined);
  c.select('Task B')
    .observe(30)
    .history(
      page(30, [
        chunk(20, 'Provider failed', {
          timing: { startedAt: 1, finished: { at: 2, outcome: 'failed' } },
        }),
      ]),
    );
  assert.equal(c.find('task-state').text, 'Run failed');
  assert.equal(c.find('task-continue').focus_child, 'composer');
});

test('a restored empty task loads new failure facts without requiring a tail receipt', () => {
  const c = client().observe(0).history(page(0, []));
  c.select('Task B')
    .observe(1)
    .history(page(1, [chunk(1, 'Another task')]));
  c.select('Task A').observe(0);
  c.event({
    kind: 'action',
    key: c.state.watch.key,
    event: {
      status: 'updated',
      value: {
        kind: 'events',
        through: 30,
        nextAfter: 30,
        events: [
          { sequence: 1, invocationId: 'failed-run', turnId: 'turn-1', fact: { kind: 'opened' } },
          {
            sequence: 30,
            invocationId: 'failed-run',
            turnId: 'turn-1',
            fact: { kind: 'ended', failed: true },
          },
        ],
      },
    },
  });
  assert.equal(c.action('history').input.through, 30);
  c.history(
    page(30, [
      chunk(20, 'Failed request', {
        role: 'user',
        timing: { startedAt: 1, finished: { at: 2, outcome: 'failed' } },
      }),
    ]),
  );
  assert.equal(c.find('task-state').text, 'Run failed');
  assert.equal(c.find('task-continue').focus_child, 'composer');
  assert.equal(c.find('task-introduction').visible, false);
});

test('failure arriving during the initial empty history read is not stranded behind the introduction', () => {
  const c = client().observe(0);
  const pending = c.action('history');
  c.event({
    kind: 'action',
    key: c.state.watch.key,
    event: {
      status: 'updated',
      value: {
        kind: 'events',
        through: 30,
        nextAfter: 30,
        events: [
          { sequence: 1, invocationId: 'failed-run', turnId: 'turn-1', fact: { kind: 'opened' } },
          {
            sequence: 30,
            invocationId: 'failed-run',
            turnId: 'turn-1',
            fact: { kind: 'ended', failed: true },
          },
        ],
      },
    },
  });
  c.reply(pending, page(0, []));
  assert.equal(c.action('history').input.through, 30);
  c.history(
    page(30, [
      chunk(20, 'Failed request', {
        role: 'user',
        timing: { startedAt: 1, finished: { at: 2, outcome: 'failed' } },
      }),
    ]),
  );
  assert.equal(c.find('task-state').text, 'Run failed');
  assert.equal(c.find('task-introduction').visible, false);
});

test('a failure with no assistant reply traverses empty state pages and retains the failure timing', () => {
  const c = client()
    .observe(30)
    .history(page(30, [], { before: { sequence: 25, offset: 0, field: 'body' } }));
  assert.deepEqual(c.action('history').input.position, {
    kind: 'before',
    cursor: { sequence: 25, offset: 0, field: 'body' },
  });
  assert.equal(c.action('history').input.through, 30);
  c.history(
    page(
      30,
      [
        chunk(20, 'The failed request', {
          role: 'user',
          timing: { startedAt: 1, finished: { at: 2, outcome: 'failed' } },
        }),
      ],
      { after: { sequence: 25, offset: 0, field: 'body' } },
    ),
  );
  assert.equal(c.find('task-state').text, 'Run failed');
  assert.equal(c.find('task-continue').focus_child, 'composer');
  assert.equal(c.state.window.latest, true);
});

test('returning to a streaming task preserves its visible prefix until latest is explicitly requested', () => {
  const c = client()
    .observe(1, { invocationId: 'run-a', turnId: 'turn-1', state: 'running' })
    .history(page(1, [chunk(1, 'Task goal', { role: 'user' })]));
  c.event({
    kind: 'action',
    key: c.state.watch.key,
    event: {
      status: 'updated',
      value: {
        kind: 'events',
        through: 3,
        nextAfter: 3,
        events: [
          {
            sequence: 2,
            invocationId: 'run-a',
            turnId: 'turn-1',
            fact: {
              kind: 'part_started',
              messageId: 'stream-a',
              stepId: 'step-a',
              partId: 'text',
              textKind: 'text',
            },
          },
          {
            sequence: 3,
            invocationId: 'run-a',
            turnId: 'turn-1',
            fact: { kind: 'delta', stepId: 'step-a', partId: 'text', text: '正在阅读的中文🙂' },
          },
        ],
      },
    },
  });
  const reader = c.reader();
  const visible = c.find(reader).props.messages;
  assert.ok(visible.some((message) => message.text === '正在阅读的中文🙂'));
  c.select('Task B').observe(1).history(page(1, []));
  assert.deepEqual(
    c.find(reader).props.messages,
    visible,
    'hidden readers retain streaming content too',
  );
  c.select('Task A').observe(50);
  c.child(reader, 'tailReady', { landing: c.state.landing.sequence, readingRevision: 9 });
  assert.equal(
    c.action('history'),
    undefined,
    'returning at the old tail does not jump to new results',
  );
  assert.deepEqual(c.find(reader).props.messages, visible);
  c.raw('task-latest');
  assert.equal(c.action('history').input.through, 50);
});

test('output review opens original structured evidence and full output at the same history fence', () => {
  const tool = {
    toolUseId: 'write-1',
    call: {
      sequence: 4,
      name: 'Write',
      title: null,
      presentation: 'maka.workspace.Files@1/write',
      pending: false,
      dispatched: true,
    },
    result: { sequence: 8, settlement: 'returned', isError: false, image: false },
  };
  const c = client()
    .observe(20)
    .history(
      page(20, [
        chunk(4, 'write arguments', { role: 'tool_call', tool }),
        chunk(8, 'write result', { role: 'tool_result', tool }),
        chunk(10, 'Delivery description'),
      ]),
    );
  c.raw('outputs-open');
  const evidence = nodes(c.out.frame.view).find((node) => node.binding === 'tools');
  assert.equal(evidence.props.expanded, false);
  assert.deepEqual(evidence.props.context, { sessionId: 'Task A', through: 20 });
  c.event({ kind: 'raw', key: 'output-item-write-1', event: { kind: 'submit' } });
  assert.equal(
    nodes(c.out.frame.view).find((node) => node.binding === 'tools').props.expanded,
    true,
  );
  c.raw('output-result-4');
  assert.equal(c.state.inspector, null);
  assert.equal(c.action('source').input.through, 20);
  assert.equal(c.action('source').input.source.sequence, 8);
  assert.equal(c.action('source').input.field, 'result');
});

test('long task visits stay within the UI call envelope while recent readers remain retained', () => {
  const c = client()
    .observe(20)
    .history(page(20, [chunk(10, '界🙂'.repeat(12000))]));
  c.select('Task B')
    .observe(30)
    .history(page(30, [chunk(20, '中文'.repeat(12000))]));
  assert.ok(c.state.visits.length >= 2);
  c.select('Task C')
    .observe(40)
    .history(page(40, [chunk(30, 'long output '.repeat(9000))]));
  c.select('Task D')
    .observe(50)
    .history(page(50, [chunk(40, 'long output '.repeat(9000))]));
  assert.ok(c.state.visits.length <= 3);
  assert.ok(Buffer.byteLength(JSON.stringify(c.out)) < 1024 * 1024);
});
