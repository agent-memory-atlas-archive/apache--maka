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

import { build } from 'esbuild';
import { mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, resolve } from 'node:path';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const destination = resolve(root, 'dist');
await rm(destination, { recursive: true, force: true });
await mkdir(destination, { recursive: true });
// These authoring modules use the real generated runtime contracts. No second
// schema, package digest or native availability registry lives in this builder.
await build({
  entryPoints: [
    resolve(root, 'src/contracts.ts'),
    resolve(root, 'src/composition.ts'),
    resolve(root, '../plugin-sdk/src/index.ts'),
  ],
  outdir: resolve(destination, 'authoring'),
  bundle: true,
  platform: 'node',
  format: 'esm',
  outExtension: { '.js': '.mjs' },
  logLevel: 'silent',
});
const own = await import(
  pathToFileURL(resolve(destination, 'authoring/agent-product/src/contracts.mjs'))
);
const sdk = await import(pathToFileURL(resolve(destination, 'authoring/plugin-sdk/src/index.mjs')));
const required = (contract) => ({ contract, availability: 'required' });
const one = (contract) => ({ contract, cardinality: { kind: 'one' } });
const empty = own.object({});
const native = (implementation, exports, configuration = empty) => ({
  implementation: { kind: 'native', implementation },
  configuration,
  imports: {},
  exports,
});
const specifications = {
  workspace: {
    contract: own.workspaceContract,
    export: 'Workspace',
    module: 'workspace',
    imports: {
      agent: one(sdk.contracts.agent),
      receipts: one(sdk.contracts.receiptReader),
      history: one(sdk.contracts.callerHistory),
      reader: one(sdk.contracts.textReader),
      composer: one(own.composerContract),
      navigation: one(own.navigationContract),
      timeline: one(own.timelineContract),
      appearance: one(own.themeContract),
    },
    configuration: own.object({ locale: own.locale }),
  },
  composer: {
    contract: own.composerContract,
    export: 'Composer',
    module: 'components',
    imports: { editor: one(sdk.contracts.textarea) },
  },
  navigation: {
    contract: own.navigationContract,
    export: 'Navigation',
    module: 'components',
    imports: {},
  },
  timeline: {
    contract: own.timelineContract,
    export: 'Timeline',
    module: 'components',
    imports: {},
  },
  appearance: {
    contract: own.themeContract,
    export: 'Appearance',
    module: 'theme',
    imports: {},
    configuration: own.object({ scheme: { enum: ['light', 'dark'] } }),
  },
};
const packages = [];
for (const [name, spec] of Object.entries(specifications)) {
  const directory = resolve(destination, 'packages', name);
  await mkdir(directory, { recursive: true });
  const entries = {};
  for (const target of ['desktop', 'tui']) {
    const module = target + '.js';
    let contents;
    if (name === 'appearance')
      contents = `export {activate} from '${resolve(root, 'src/theme.ts')}';`;
    else if (name === 'workspace')
      contents = `import {workspace} from '${resolve(root, 'src/workspace.ts')}';import {workspaceContract} from '${resolve(root, 'src/contracts.ts')}';export let component;export function activate(configuration){component=workspace('${target}',configuration.locale);return {Workspace:{kind:'component',contract:workspaceContract,component:['component']}};}`;
    else
      contents = `import {components,exports} from '${resolve(root, 'src/components.ts')}';export const component=components('${target}').${name};export function activate(){const declaration=exports().${spec.export};return {${spec.export}:{...declaration,component:['component']}};}`;
    await build({
      stdin: { contents, resolveDir: root, sourcefile: `${name}-${target}.ts`, loader: 'ts' },
      outfile: resolve(directory, module),
      bundle: true,
      format: 'esm',
      platform: 'neutral',
      target: 'es2022',
      logLevel: 'silent',
      banner: {
        js: (await readFile(resolve(root, 'src/view.ts'), 'utf8')).split('import type')[0].trim(),
      },
    });
    entries[target] = {
      implementation: { kind: 'javascript', module, realm: 'dedicated' },
      configuration: spec.configuration ?? empty,
      imports: { ...spec.imports },
      exports: { [spec.export]: required(spec.contract) },
      ...(name === 'appearance' ? {} : { scene: { name: `maka.${target}.scene`, version: 1 } }),
    };
  }
  // A surface is an explicit capability in the root's checked plan.
  if (name === 'workspace')
    for (const target of ['desktop', 'tui'])
      entries[target].imports.surface = one(
        target === 'desktop' ? sdk.contracts.desktopSurface : sdk.contracts.terminalSurface,
      );
  await writeFile(
    resolve(directory, 'maka.extension.json'),
    JSON.stringify({
      schemaVersion: 2,
      id: `maka.agent.${name}`,
      displayName: name,
      application: { entries },
    }) + '\n',
  );
  packages.push(name);
}
const mechanical = {
  history: {
    entries: Object.fromEntries(
      ['desktop', 'tui'].map((target) => [
        target,
        native('maka.operations.history', { History: required(sdk.contracts.callerHistory) }),
      ]),
    ),
  },
  surface: {
    entries: Object.fromEntries(
      ['desktop', 'tui'].map((target) => [
        target,
        native(`maka.ui.surface.${target}`, {
          Surface: required(
            target === 'desktop' ? sdk.contracts.desktopSurface : sdk.contracts.terminalSurface,
          ),
        }),
      ]),
    ),
  },
  editor: {
    entries: Object.fromEntries(
      ['desktop', 'tui'].map((target) => [
        target,
        native('maka.ui.textarea', { Textarea: required(sdk.contracts.textarea) }),
      ]),
    ),
  },
  reader: {
    entries: Object.fromEntries(
      ['desktop', 'tui'].map((target) => [
        target,
        native('maka.text.reader', { Reader: required(sdk.contracts.textReader) }),
      ]),
    ),
  },
  agent: {
    entries: {
      host: native(
        'maka.agent.api',
        { agent: required(sdk.contracts.agent) },
        sdk.configurations.agent,
      ),
    },
  },
  receipts: {
    entries: {
      host: native('maka.operations.receipts', { Receipts: required(sdk.contracts.receiptReader) }),
    },
  },
  gateway: {
    entries: {
      host: {
        ...native('maka.application.gateway', {}),
        imports: { agent: one(sdk.contracts.agent), receipts: one(sdk.contracts.receiptReader) },
      },
    },
  },
};
for (const [name, application] of Object.entries(mechanical)) {
  const directory = resolve(destination, 'packages', name);
  await mkdir(directory, { recursive: true });
  await writeFile(
    resolve(directory, 'maka.extension.json'),
    JSON.stringify({ schemaVersion: 2, id: `maka.agent.${name}`, displayName: name, application }) +
      '\n',
  );
  packages.push(name);
}
await writeFile(resolve(destination, 'packages.json'), JSON.stringify(packages, null, 2) + '\n');
console.log(`${packages.length} ordinary plugin packages built`);
