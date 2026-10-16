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
const { component } = await import('../dist/packages/navigation/tui.js');
function nodes(node) {
  return node && typeof node === 'object'
    ? [
        node,
        ...Object.values(node).flatMap((v) => (Array.isArray(v) ? v.flatMap(nodes) : nodes(v))),
      ]
    : [];
}

test('the left session dot reflects unread state independently of activity and selection', () => {
  for (const ascii of [false, true]) {
    for (const activity of ['idle', 'working', 'waiting']) {
      for (const hasUnread of [false, true]) {
        for (const selected of [null, 'session']) {
          const out = component.mount({
            theme: { ...theme, ascii },
            locale: 'en',
            selected,
            sessions: [
              {
                id: 'session',
                title: 'Session',
                activity,
                hasUnread,
                archived: false,
                workspace: { identity: 'project', path: '/project' },
              },
            ],
            busy: false,
            loading: false,
            older: false,
            newer: false,
          });
          const glyph = nodes(out.frame.view).find((node) => node.key === 'session-glyph-0-0');
          assert.equal(glyph.text, hasUnread ? (ascii ? '  * ' : '  ● ') : '    ');
          assert.equal(glyph.animation, undefined, 'activity must not animate the unread slot');
          assert.equal(
            nodes(out.frame.view).some((node) => node.key === 'session-waiting-0-0'),
            activity === 'waiting',
            'the existing waiting label remains available',
          );
        }
      }
    }
  }
});
