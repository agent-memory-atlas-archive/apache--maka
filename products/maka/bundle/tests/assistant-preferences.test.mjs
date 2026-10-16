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
import Ajv from 'ajv/dist/2020.js';
const { activate } = await import('../dist/packages/appearance/desktop.js');
const theme = activate({ scheme: 'light' }).Appearance.value;
const valid = new Ajv({ strict: false, formats: { uint64: true } }).compile(
  contracts.settings.interface.methods.updateAssistantPreferences.input,
);
for (const target of ['tui', 'desktop'])
  test(`Assistant ${target} privacy recovers the original operation without overwriting editor drafts`, async () => {
    const { component } = await import(`../dist/packages/assistant-preferences/${target}.js`);
    const props = { theme, locale: 'en', open: true, busy: false, shell: { back: 0 } };
    let out = component.mount(props, {});
    const event = (input) => (out = component.event(out.frame.state, input, props, {}));
    const raw = (key) => event({ kind: 'raw', key, event: {} });
    const finish = (value) =>
      event({ kind: 'action', key: out.actions[0].key, event: { status: 'succeeded', value } });
    const saved = {
      revision: 4,
      displayName: 'Saved',
      assistantTone: '',
      workspaceInstructions: true,
      incognitoActive: false,
      shellTitles: true,
    };
    finish([]);
    finish(saved);
    const fields = structuredClone(out.frame.state.fields);
    raw('privacy');
    const action = out.actions[0];
    assert.ok(valid(action.input), JSON.stringify(valid.errors));
    assert.deepEqual(action.input, {
      expectedRevision: 4,
      mutation: { kind: 'privacy', incognitoActive: true },
    });
    event({
      kind: 'action',
      key: action.key,
      event: { status: 'failed', diagnostic: 'response interrupted' },
    });
    raw('privacy');
    assert.equal(out.actions.length, 0);
    raw('history');
    const reference = {
      contract: { name: 'maka.settings.Settings', version: 1 },
      method: 'updateAssistantPreferences',
      operation: 'original-privacy-toggle',
    };
    finish([{ cursor: 2, reference, recovery: action.recovery }]);
    finish([]);
    assert.equal(out.actions[0].method, 'read');
    assert.deepEqual(out.actions[0].input, reference);
    finish({
      reference,
      receipt: {
        kind: 'recorded',
        cleanup: { kind: 'confirmed' },
        outcome: { kind: 'returned', value: { kind: 'committed', revision: 5 } },
      },
    });
    assert.equal(out.actions[0].method, 'assistantPreferences');
    finish({ ...saved, revision: 6, incognitoActive: true, displayName: 'Changed elsewhere' });
    assert.equal(out.frame.state.saved.revision, 6);
    assert.deepEqual(
      out.frame.state.fields,
      fields,
      'a policy toggle must retain the existing editor state',
    );
  });

for (const target of ['tui', 'desktop'])
  test(`Assistant ${target} shell titles toggle saves the observed preference revision`, async () => {
    const { component } = await import(`../dist/packages/assistant-preferences/${target}.js`);
    const props = { theme, locale: 'en', open: true, busy: false, shell: { back: 0 } };
    let out = component.mount(props, {});
    const finish = (value) => {
      out = component.event(
        out.frame.state,
        { kind: 'action', key: out.actions[0].key, event: { status: 'succeeded', value } },
        props,
        {},
      );
    };
    finish([]);
    const saved = {
      revision: 7,
      displayName: '',
      assistantTone: '',
      workspaceInstructions: true,
      incognitoActive: false,
      shellTitles: true,
    };
    finish(saved);
    const toggle = () => {
      out = component.event(
        out.frame.state,
        { kind: 'raw', key: 'shell-titles', event: {} },
        props,
        {},
      );
    };
    toggle();
    assert.deepEqual(
      out.actions.map(({ method, input }) => ({ method, input })),
      [
        {
          method: 'updateAssistantPreferences',
          input: { expectedRevision: 7, mutation: { kind: 'shell_titles', enabled: false } },
        },
      ],
    );
    assert.ok(valid(out.actions[0].input), JSON.stringify(valid.errors));
    out = component.event(
      out.frame.state,
      {
        kind: 'action',
        key: out.actions[0].key,
        event: {
          status: 'operation',
          reply: {
            reference: {
              contract: { name: 'maka.settings.Settings', version: 1 },
              method: 'updateAssistantPreferences',
              operation: 'shell-title-toggle',
            },
            receipt: {
              kind: 'recorded',
              cleanup: { kind: 'confirmed' },
              outcome: { kind: 'returned', value: { kind: 'committed', revision: 8 } },
            },
          },
        },
      },
      props,
      {},
    );
    finish({ ...saved, revision: 8, shellTitles: false });
    toggle();
    assert.deepEqual(out.actions[0].input, {
      expectedRevision: 8,
      mutation: { kind: 'shell_titles', enabled: true },
    });
  });
