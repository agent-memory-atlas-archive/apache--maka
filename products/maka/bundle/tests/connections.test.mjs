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
const { activate } = await import('../dist/packages/appearance/desktop.js');
const theme = activate({ scheme: 'light' }).Appearance.value;
const identity = {
  packageId: 'maka.codex',
  entryId: 'maka.codex',
  scope: 'profile',
  name: 'chatgpt',
};
const provider = {
  identity,
  descriptor: {
    label: 'ChatGPT Subscription',
    customEndpoint: false,
    configurationDefaults: { baseUrl: 'https://chatgpt.com/backend-api/codex' },
    configurationSchema: {},
    anonymous: false,
    discovery: true,
    authentication: [
      { id: 'chatgpt', label: 'Sign in with ChatGPT', interactive: true, inputSchema: {} },
    ],
  },
};
const target = {
  connectionId: 'connection-a',
  revision: 7,
  slug: 'chatgpt',
  provider: identity,
  configuration: provider.descriptor.configurationDefaults,
};
async function client(name, platform, extra = {}) {
  const { component } = await import(`../dist/packages/${name}/${platform}.js`);
  const props = {
    theme,
    locale: 'en',
    shell: { back: 0 },
    connection: null,
    catalogRevision: 3,
    ...extra,
  };
  let out = component.mount(props, {});
  return {
    get out() {
      return out;
    },
    get state() {
      return out.frame.state;
    },
    event(input) {
      out = component.event(out.frame.state, input, props, {});
      return this;
    },
    raw(key, kind = 'submit') {
      return this.event({ kind: 'raw', key, event: { kind } });
    },
    reply(action, value) {
      assert.ok(action);
      return this.event({ kind: 'action', key: action.key, event: { status: 'succeeded', value } });
    },
    action(method) {
      return out.actions.find((a) => a.method === method);
    },
    query(child, revision, text) {
      return this.event({
        kind: 'component',
        child,
        name: 'changed',
        event: {
          revision,
          snapshot: {
            text: { owner: child, revision, bytes: Buffer.byteLength(text) },
            marked: null,
          },
        },
      });
    },
  };
}
for (const platform of ['tui', 'desktop']) {
  test(`${platform} provider search scans all pages and ignores stale editor reads`, async () => {
    const c = await client('connection-form', platform);
    c.reply(c.action('providers'), {
      providers: [
        {
          ...provider,
          identity: { ...identity, name: 'other' },
          descriptor: { ...provider.descriptor, label: 'Other', authentication: [] },
        },
      ],
      next: 1,
    });
    const second = c.action('providers');
    assert.equal(second.input.offset, 1);
    c.query('provider-query', 1, 'other');
    const old = c.action('read');
    c.query('provider-query', 2, 'oauth');
    c.reply(old, { metadata: { owner: 'provider-query', revision: 1, bytes: 5 }, text: 'other' });
    const latest = c.action('read');
    assert.equal(latest.resources.text.input.revision, 2);
    c.reply(latest, {
      metadata: { owner: 'provider-query', revision: 2, bytes: 5 },
      text: 'oauth',
    });
    c.reply(second, { providers: [provider], next: null });
    assert.equal(c.state.providers.length, 2);
    assert.equal(c.state.search.query, 'oauth');
    if (platform === 'tui') {
      c.raw('provider-1', 'activate');
      assert.equal(c.state.provider, null);
    }
    c.raw('provider-1');
    assert.deepEqual(c.state.provider.identity, identity);
    assert.equal(c.state.fields.endpoint.enabled, false);
  });
  test(`${platform} a visible provider stays clickable while search or catalog reads are pending`, async () => {
    const c = await client('connection-form', platform);
    c.reply(c.action('providers'), { providers: [provider], next: 1 });
    const next = c.action('providers');
    c.query('provider-query', 1, 'chatgpt');
    assert.ok(c.action('read'));
    c.raw('provider-0');
    assert.deepEqual(c.state.provider.identity, identity);
    c.reply(next, { providers: [], next: 2 });
    assert.equal(c.action('providers'), undefined, 'selection stops unnecessary catalog scanning');
  });
  test(`${platform} connections append one catalog revision and restart changed scans`, async () => {
    const c = await client('connections', platform);
    const row = {
      basis: { connectionId: 'a', revision: 1 },
      name: 'Alpha',
      slug: 'alpha',
      provider: identity,
      editable: true,
      enabled: true,
      enabledModelCount: 1,
    };
    c.reply(c.action('connections'), {
      kind: 'ready',
      revision: 3,
      connections: [row],
      next: 16,
      defaultTarget: null,
    });
    assert.deepEqual(c.action('connections').input, { kind: 'continue', revision: 3, offset: 16 });
    c.reply(c.action('connections'), {
      kind: 'ready',
      revision: 3,
      connections: [{ ...row, basis: { connectionId: 'b', revision: 2 }, name: 'Beta' }],
      next: null,
      defaultTarget: null,
    });
    assert.equal(c.state.connections.length, 2);
    c.query('connection-query', 1, 'Beta');
    c.reply(c.action('read'), {
      metadata: { owner: 'connection-query', revision: 1, bytes: 4 },
      text: 'Beta',
    });
    assert.equal(c.state.selected, 'b');
    c.raw('refresh');
    c.reply(c.action('connections'), {
      kind: 'ready',
      revision: 4,
      connections: [row],
      next: 16,
      defaultTarget: null,
    });
    assert.equal(
      c.state.selected,
      null,
      'a filtered-out first page must not select another connection',
    );
    c.reply(c.action('connections'), { kind: 'changed', revision: 5 });
    assert.deepEqual(c.action('connections').input, { kind: 'start' });
    assert.equal(c.state.connections.length, 0);
  });
  test(`${platform} ChatGPT login retains progress, cancellation and original receipt`, async () => {
    const c = await client('credential', platform, { target });
    const status = c.action('credentialStatus'),
      definition = c.action('provider');
    c.reply(definition, provider);
    c.reply(status, {
      kind: 'status',
      status: {
        locator: { scope: 'connection', connectionId: target.connectionId, kind: 'provider' },
        configured: false,
        credentialId: null,
        revision: null,
      },
    });
    c.raw('login-chatgpt');
    const login = c.action('loginCredential');
    assert.equal(login.binding, 'authentication');
    assert.deepEqual(login.resources, {});
    assert.deepEqual(login.input, {
      expected: target,
      expectedCredential: null,
      method: 'chatgpt',
    });
    c.event({
      kind: 'action',
      key: login.key,
      event: {
        status: 'updated',
        value: { url: 'https://auth.openai.com/codex/device', userCode: 'ABCD-EFGH' },
      },
    });
    assert.equal(c.state.loginPrompt.userCode, 'ABCD-EFGH');
    assert.ok(c.state.pending[login.key]);
    c.raw('login-cancel');
    assert.deepEqual(c.out.cancel, [login.key]);
    assert.equal(c.state.current.kind, 'login');
    c.event({
      kind: 'action',
      key: login.key,
      event: {
        status: 'operation',
        reply: {
          reference: { operation: 'original' },
          receipt: {
            kind: 'recorded',
            outcome: {
              kind: 'returned',
              value: { kind: 'failed', reason: 'cancelled', message: '' },
            },
            cleanup: { kind: 'confirmed' },
          },
        },
      },
    });
    assert.equal(c.state.current, null);
    assert.equal(c.state.loginPrompt, null);
    assert.equal(c.state.notice, 'loginCancelled');
    c.raw('login-chatgpt');
    const retry = c.action('loginCredential');
    c.event({
      kind: 'action',
      key: retry.key,
      event: {
        status: 'operation',
        reply: {
          reference: { operation: 'retry' },
          receipt: {
            kind: 'recorded',
            outcome: {
              kind: 'returned',
              value: {
                kind: 'saved',
                connection: {
                  connectionId: target.connectionId,
                  slug: target.slug,
                  provider: identity,
                },
              },
            },
            cleanup: { kind: 'confirmed' },
          },
        },
      },
    });
    assert.equal(c.state.notice, 'loginSaved');
    assert.equal(c.state.finished, true);
    assert.equal(c.state.command, null);
    assert.equal(
      c.out.events.some((e) => e.name === 'changed'),
      true,
    );
  });
}
