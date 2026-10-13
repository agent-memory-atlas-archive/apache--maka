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
const { activate } = await import('../dist/packages/appearance/tui.js');
const theme = activate({ scheme: 'dark' }).Appearance.value;
const { component } = await import('../dist/packages/timeline/tui.js');

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
