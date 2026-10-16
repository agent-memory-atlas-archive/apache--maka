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
  test(`Assistant ${target} mounts independently addressable preference editors`, async () => {
    const { component } = await import(`../dist/packages/assistant-preferences/${target}.js`);
    const props = { theme, locale: 'en', open: true, busy: false, shell: { back: 0 } };
    const out = component.mount(props, {});
    const editors = new Set();
    const visit = (node) => {
      if (!node || typeof node !== 'object') return;
      if (node.kind === 'component') {
        assert.ok(!editors.has(node.key), `duplicate editor identity: ${node.key}`);
        editors.add(node.key);
      }
      Object.values(node).forEach(visit);
    };
    visit(out.frame.view);
    assert.deepEqual(
      editors,
      new Set(['assistant-name', 'assistant-tone', 'assistant-compaction']),
    );
  });

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
      compactionPrompt: 'Default compression prompt',
      defaultCompactionPrompt: 'Default compression prompt',
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
      compactionPrompt: 'Default compression prompt',
      defaultCompactionPrompt: 'Default compression prompt',
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

for (const target of ['tui', 'desktop'])
  test(`Assistant ${target} saves a custom compression prompt and restores the built-in default`, async () => {
    const { component } = await import(`../dist/packages/assistant-preferences/${target}.js`);
    const props = { theme, locale: 'en', open: true, busy: false, shell: { back: 0 } };
    let out = component.mount(props, {});
    const event = (input) => (out = component.event(out.frame.state, input, props, {}));
    const raw = (key) => event({ kind: 'raw', key, event: {} });
    const finish = (value) =>
      event({ kind: 'action', key: out.actions[0].key, event: { status: 'succeeded', value } });
    const observed = (text, revision, receipt = null) => ({
      revision,
      snapshot: {
        text: { owner: 'prompt', revision, bytes: Buffer.byteLength(text) },
        marked: null,
      },
      receipt,
    });
    const field = (value) =>
      event({ kind: 'component', child: 'assistant-compaction', name: 'changed', event: value });
    const saved = {
      revision: 4,
      displayName: 'Saved',
      assistantTone: '',
      workspaceInstructions: true,
      incognitoActive: false,
      shellTitles: true,
      compactionPrompt: 'Built-in instructions',
      defaultCompactionPrompt: 'Built-in instructions',
    };
    finish([]);
    finish(saved);
    field(observed('', 0));
    const initial = out.frame.state.fields.compaction.command;
    assert.deepEqual(initial.edit, { kind: 'reset', text: 'Built-in instructions' });
    field(
      observed('Built-in instructions', 1, {
        owner: initial.owner,
        sequence: initial.sequence,
        outcome: { status: 'applied' },
      }),
    );
    const draft = observed('Keep 中文 decisions', 2);
    field(draft);
    raw('save-compaction');
    assert.equal(out.actions[0].resources.text.child, 'assistant-compaction');
    finish({ metadata: draft.snapshot.text, text: 'Keep 中文 decisions' });
    assert.deepEqual(out.actions[0].input, {
      expectedRevision: 4,
      mutation: { kind: 'compaction_prompt', prompt: 'Keep 中文 decisions' },
    });
    assert.equal(valid(out.actions[0].input), true, JSON.stringify(valid.errors));
    const commit = (revision) =>
      event({
        kind: 'action',
        key: out.actions[0].key,
        event: {
          status: 'operation',
          reply: {
            reference: {
              contract: { name: 'maka.settings.Settings', version: 1 },
              method: 'updateAssistantPreferences',
              operation: `prompt-${revision}`,
            },
            receipt: {
              kind: 'recorded',
              outcome: { kind: 'returned', value: { kind: 'committed', revision } },
            },
          },
        },
      });
    const unrelated = structuredClone(out.frame.state.fields.name);
    commit(5);
    finish({ ...saved, revision: 5, compactionPrompt: 'Keep 中文 decisions' });
    assert.deepEqual(out.frame.state.fields.name, unrelated);
    const savedReset = out.frame.state.fields.compaction.command;
    field(
      observed(savedReset.edit.text, 3, {
        owner: savedReset.owner,
        sequence: savedReset.sequence,
        outcome: { status: 'applied' },
      }),
    );
    raw('reset-compaction');
    assert.deepEqual(out.actions[0].input, {
      expectedRevision: 5,
      mutation: { kind: 'compaction_prompt', prompt: null },
    });
    commit(6);
    finish({ ...saved, revision: 6 });
    const pending = out.frame.state.fields.compaction.command;
    assert.deepEqual(pending.edit, { kind: 'reset', text: 'Built-in instructions' });
    field(
      observed(pending.edit.text, 4, {
        owner: pending.owner,
        sequence: pending.sequence,
        outcome: { status: 'applied' },
      }),
    );
    assert.equal(out.frame.state.saved.compactionPrompt, 'Built-in instructions');
  });
