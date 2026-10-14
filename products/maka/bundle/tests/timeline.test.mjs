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
import Ajv from 'ajv/dist/2020.js';
const { activate } = await import('../dist/packages/appearance/tui.js');
const theme = activate({ scheme: 'dark' }).Appearance.value;
const { component, activate: timelineExports } = await import('../dist/packages/timeline/tui.js');

test('history fragments keep distinct controls and navigate the original tool evidence', () => {
  const tool = {
    toolUseId: 'computer-call',
    call: {
      sequence: 7,
      name: 'cua_repl',
      title: null,
      presentation: null,
      pending: false,
      dispatched: true,
    },
    result: null,
  };
  const props = {
    theme,
    locale: 'en',
    copyRequest: null,
    browse: null,
    timings: [],
    markReadEnabled: true,
    toolContext: { sessionId: 'session', through: 8 },
    waitingTools: [],
    liveTools: [],
    source: null,
    search: null,
    loading: false,
    older: false,
    newer: false,
    latest: false,
    landing: { sequence: 0, edge: 'end' },
    live: false,
    messages: [
      'call',
      'call-offset-64',
      'answer',
      'answer-offset-64',
      ...Array.from({ length: 5 }, (_, index) => 'later-answer-' + index),
    ].map((id, index) => ({
      id,
      contentKey: id,
      role: index < 2 ? 'tool_call' : 'assistant',
      text: 'Original fragment ' + index,
      timestamp: null,
      source: index < 2 ? 7 : 8,
      image: false,
      continued: true,
      tool: index < 2 ? tool : null,
    })),
  };
  props.timings = props.messages.map((message, index) => ({
    turnId: 'turn-' + index,
    afterMessageId: message.id,
    startedAt: index,
    finished: { at: index + 1, outcome: 'completed' },
  }));
  const schema = timelineExports().Timeline.contract.interface.props;
  const validTimings = new Ajv({ strict: false }).compile({
    ...schema.properties.timings,
    $defs: schema.$defs,
  });
  assert.equal(validTimings(props.timings), true, JSON.stringify(validTimings.errors));
  let out = component.mount(props, {});
  const raw = (key) => {
    out = component.event(
      out.frame.state,
      { kind: 'raw', key, event: { kind: 'activate' } },
      props,
      {},
    );
  };
  raw('toggle-call');
  raw('toggle-call-offset-64');
  const keys = [];
  const visit = (value) => {
    if (!value || typeof value !== 'object') return;
    if (typeof value.key === 'string') keys.push(value.key);
    for (const child of Object.values(value)) {
      if (Array.isArray(child)) child.forEach(visit);
      else visit(child);
    }
  };
  visit(out.frame.view);
  assert.equal(keys.filter((key) => key.startsWith('time-')).length, 9);
  assert.equal(
    new Set(keys).size,
    keys.length,
    'all rendered fragments must have unique control identities',
  );
  raw('tool-source-call-offset-64-arguments');
  assert.deepEqual(out.events, [
    { name: 'toolSource', value: { sequence: 7, field: 'arguments' } },
  ]);
  raw('source-answer-offset-64');
  assert.deepEqual(out.events, [{ name: 'source', value: { sequence: 8, field: 'body' } }]);
  raw('mark-read-answer-offset-64');
  assert.deepEqual(out.events, [
    { name: 'markRead', value: { sessionId: 'session', sequence: 8, field: 'body' } },
  ]);
  const find = (node, key) => {
    if (!node || typeof node !== 'object') return undefined;
    if (node.key === key) return node;
    for (const value of Object.values(node)) {
      for (const child of Array.isArray(value) ? value : [value]) {
        const found = find(child, key);
        if (found) return found;
      }
    }
  };
  for (const [settlement, isError, hint, tone] of [
    ['returned', false, 'Returned', 'success'],
    ['returned', true, 'Tool reported an error', 'error'],
    ['failed', false, 'Tool execution failed', 'error'],
    ['rejected', false, 'Not executed', 'error'],
    ['unknown', false, 'Tool outcome is unknown', 'warning'],
  ]) {
    props.messages[0].tool = {
      ...tool,
      result: { sequence: 8, isError, settlement, image: false },
    };
    out = component.update(out.frame.state, props, {});
    const marker = find(out.frame.view, 'marker-call');
    assert.equal(marker.hint, hint);
    assert.equal(marker.color, theme.terminal[tone]);
    assert.equal(find(out.frame.view, 'tool-status-call'), undefined);
  }
  const title = {
    fallback: 'Read skill · demo',
    translations: { 'zh-CN': '读取技能 · demo', 'zh-TW': '讀取技能 · demo' },
  };
  props.messages[0].tool.call.title = title;
  for (const locale of ['en', 'zh-CN', 'zh-TW']) {
    props.locale = locale;
    out = component.update(out.frame.state, props, {});
    assert.equal(
      find(out.frame.view, 'toggle-call').label,
      title.translations[locale] ?? title.fallback,
    );
  }
});

test('history prefetches before an edge without treating stale or commanded viewports as scrolling', () => {
  const props = {
    theme,
    locale: 'en',
    copyRequest: null,
    browse: null,
    timings: [],
    markReadEnabled: false,
    toolContext: null,
    waitingTools: [],
    liveTools: [],
    messages: Array.from({ length: 8 }, (_, i) => ({
      id: String(i),
      contentKey: String(i),
      role: 'assistant',
      text: 'Message ' + i,
      timestamp: null,
      source: i + 1,
      image: false,
      continued: false,
      tool: null,
    })),
    source: null,
    search: null,
    loading: false,
    older: true,
    newer: true,
    latest: false,
    landing: { sequence: 0, edge: 'end' },
    live: true,
  };
  let out = component.mount(props, {});
  const observed = (revision, direction, visibleChildKeys, extra = {}) => {
    out = component.event(
      out.frame.state,
      {
        kind: 'component',
        child: 'reading',
        name: 'observed',
        event: {
          readingRevision: revision,
          scroll: { readingRevision: revision, direction },
          pendingCommand: null,
          viewport: { readingRevision: revision, atStart: false, atEnd: false, visibleChildKeys },
          ...extra,
        },
      },
      props,
      {},
    );
    return out.events.filter((event) => event.name === 'older' || event.name === 'newer');
  };
  assert.deepEqual(observed(1, 'up', ['message-3', 'message-4']), []);
  assert.deepEqual(observed(2, 'up', ['message-1', 'message-2']), [{ name: 'older', value: {} }]);
  assert.deepEqual(observed(2, 'up', ['message-1']), [], 'one request for each original scroll');
  assert.deepEqual(observed(3, 'down', ['message-5', 'message-6']), [{ name: 'newer', value: {} }]);
  assert.deepEqual(
    observed(4, 'up', [], {
      viewport: {
        readingRevision: 3,
        atStart: true,
        atEnd: false,
        visibleChildKeys: ['message-0'],
      },
    }),
    [],
    'wait for the viewport actually produced by the scroll',
  );
  assert.deepEqual(observed(4, 'up', ['message-0'], { pendingCommand: 7 }), []);
  assert.deepEqual(observed(4, 'up', ['message-0']), [{ name: 'older', value: {} }]);
  props.messages = props.messages.slice(0, 2);
  assert.deepEqual(
    observed(5, 'up', ['message-1']),
    [],
    'do not evict the visible tail of a two-part message',
  );
  assert.deepEqual(observed(6, 'up', ['message-0']), [{ name: 'older', value: {} }]);
  assert.deepEqual(observed(7, 'down', ['message-0']), []);
});

test('floating output is one truncated row only away from the live end', () => {
  const props = {
    theme,
    locale: 'en',
    copyRequest: null,
    browse: null,
    timings: [],
    markReadEnabled: false,
    toolContext: null,
    waitingTools: [],
    liveTools: [],
    messages: [],
    source: null,
    search: null,
    loading: false,
    older: false,
    newer: false,
    latest: false,
    landing: { sequence: 0, edge: 'end' },
    live: true,
    activeOutput: {
      id: 'live',
      contentKey: 'live',
      role: 'thinking',
      text: 'First line\nSecond line',
      timestamp: null,
      source: null,
      image: false,
      continued: false,
      tool: null,
    },
  };
  let out = component.mount(props, {});
  const nodes = (n) =>
    n && typeof n === 'object'
      ? [n, ...Object.values(n).flatMap((v) => (Array.isArray(v) ? v.flatMap(nodes) : nodes(v)))]
      : [];
  const find = (key) => nodes(out.frame.view).find((n) => n.key === key);
  assert.equal(find('active-output'), undefined);
  const observe = (revision, atEnd) => {
    out = component.event(
      out.frame.state,
      {
        kind: 'component',
        child: 'reading',
        name: 'observed',
        event: {
          readingRevision: revision,
          pendingCommand: null,
          viewport: { readingRevision: revision, atStart: false, atEnd, visibleChildKeys: [] },
        },
      },
      props,
      {},
    );
  };
  observe(1, false);
  assert.equal(find('active-output-text').text, 'Thinking · First line Second line');
  assert.equal(find('active-output-text').truncate, true);
  assert.ok(find('reading-control-space'));
  assert.ok(find('reading-control-space-end'));
  assert.deepEqual(out.events.find((e) => e.name === 'detached')?.value, {
    landing: 0,
    value: true,
  });
  observe(2, true);
  assert.equal(find('active-output'), undefined);
  observe(3, false);
  props.activeOutput = null;
  out = component.update(out.frame.state, props, {});
  assert.equal(find('active-output'), undefined);
});

test('prompt header follows the visible answer, yields to user rows, and never flickers at a boundary', () => {
  const message = (id, role, text) => ({
    id,
    contentKey: id,
    role,
    text,
    source: Number(id),
    timestamp: 1,
    tool: null,
    image: false,
    continued: false,
  });
  const props = {
    theme,
    locale: 'en',
    copyRequest: null,
    browse: null,
    timings: [],
    markReadEnabled: false,
    toolContext: null,
    waitingTools: [],
    liveTools: [],
    source: null,
    search: null,
    loading: false,
    older: false,
    newer: false,
    latest: false,
    live: false,
    landing: { sequence: 0, edge: 'start' },
    messages: [
      message('1', 'user', 'First question'),
      message('2', 'assistant', 'First answer'),
      message('3', 'user', '?'),
      message('4', 'assistant', 'Latest answer'),
    ],
  };
  let out = component.mount(props, {});
  const nodes = (n) =>
    n && typeof n === 'object'
      ? [n, ...Object.values(n).flatMap((v) => (Array.isArray(v) ? v.flatMap(nodes) : nodes(v)))]
      : [];
  const find = (key) => nodes(out.frame.view).find((n) => n.key === key);
  const observe = (revision, first) => {
    out = component.event(
      out.frame.state,
      {
        kind: 'component',
        child: 'reading',
        name: 'observed',
        event: {
          readingRevision: revision,
          pendingCommand: null,
          viewport: {
            readingRevision: revision,
            atStart: false,
            atEnd: false,
            visibleChildKeys: ['message-' + first],
          },
        },
      },
      props,
      {},
    );
  };
  const events = timelineExports().Timeline.contract.interface.events;
  const ajv = new Ajv({ strict: false });
  const validateEvents = () => {
    for (const event of out.events) {
      const valid = ajv.compile(events[event.name]);
      assert.ok(valid(event.value), JSON.stringify(valid.errors));
    }
  };
  assert.equal(find('prompt-bar'), undefined);
  observe(1, '2');
  assert.equal(find('prompt-text').label, '❯ First question');
  out = component.event(out.frame.state, { kind: 'raw', key: 'prompt-text', event: {} }, props, {});
  assert.deepEqual(out.frame.state.command.target, { kind: 'reveal_child', key: 'message-1' });
  assert.ok(out.events.some((event) => event.name === 'detached'));
  validateEvents();
  observe(2, '3');
  assert.equal(find('prompt-bar'), undefined);
  observe(2, '2');
  assert.equal(find('prompt-bar'), undefined, 'layout feedback cannot reinsert the canceled row');
  observe(3, '4');
  assert.equal(
    find('prompt-text').label,
    '❯ ?',
    'short prompts remain valid context for their own answer',
  );
  observe(4, '1');
  assert.equal(find('prompt-bar'), undefined);
  props.messages = [message('4', 'assistant', 'Latest answer')];
  props.leadingPrompt = { source: 3, text: 'A question outside the retained page' };
  out = component.update(out.frame.state, props, {});
  observe(5, '4');
  assert.equal(find('prompt-text').label, '❯ A question outside the retained page');
  out = component.event(out.frame.state, { kind: 'raw', key: 'prompt-text', event: {} }, props, {});
  assert.ok(out.events.some((event) => event.name === 'prompt' && event.value.sequence === 3));
  validateEvents();
  assert.deepEqual(
    find('reading').props.content.children.map((n) => n.key),
    ['message-4'],
    'header is never copied or searched as transcript text',
  );
});
