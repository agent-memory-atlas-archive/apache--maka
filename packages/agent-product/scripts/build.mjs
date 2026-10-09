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
import Ajv from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';
import standalone from 'ajv/dist/standalone/index.js';

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
// Compile the original Rust-generated input contracts; guests never compile
// schemas, coerce values, or keep a second hand-written ModelOverride schema.
const validator = new Ajv({
  code: { source: true, esm: true },
  strict: false,
  strictNumbers: true,
  formats: { uint64: true },
});
addFormats(validator);
for (const method of ['create', 'update'])
  validator.addSchema(sdk.contracts.settings.interface.methods[method].input, method);
const validationPath = resolve(destination, 'settings-validation.mjs');
await writeFile(validationPath, standalone(validator, { create: 'create', update: 'update' }));
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
  studio: {
    id: 'maka.studio.workbench',
    contract: own.studioContract,
    export: 'Workbench',
    module: 'studio',
    imports: {
      connections: one(own.connectionsContract),
      models: one(own.modelsContract),
      deployment: one(own.deploymentContract),
      appearance: one(own.themeContract),
    },
    configuration: own.object({ locale: own.locale }),
  },
  deployment: {
    contract: own.deploymentContract,
    export: 'Deployment',
    module: 'deployment',
    imports: {
      management: one(sdk.contracts.deploymentManager),
      receipts: one(sdk.contracts.receiptReader),
      history: one(sdk.contracts.callerHistory),
      editor: one(sdk.contracts.textarea),
      file: one(sdk.contracts.filePicker),
    },
  },
  workspace: {
    contract: own.workspaceContract,
    export: 'Workspace',
    module: 'workspace',
    imports: {
      agent: one(sdk.contracts.agent),
      models: one(own.modelsContract),
      connections: one(own.connectionsContract),
      deployment: one(own.deploymentContract),
      receipts: one(sdk.contracts.receiptReader),
      history: one(sdk.contracts.callerHistory),
      reader: one(sdk.contracts.textReader),
      composer: one(own.composerContract),
      palette: one(own.paletteContract),
      directories: one(own.directoryPickerContract),
      details: one(own.sessionDetailsContract),
      queueEditor: one(own.queueEditorContract),
      questionEditor: one(own.questionEditorContract),
      serviceApproval: one(own.serviceApprovalContract),
      permissionApproval: one(own.permissionApprovalContract),
      help: one(own.helpContract),
      navigation: one(own.navigationContract),
      timeline: one(own.timelineContract),
      imageViewer: one(own.imageViewerContract),
      appearance: one(own.themeContract),
    },
    configuration: own.object({ locale: own.locale }),
  },
  'image-viewer': {
    contract: own.imageViewerContract,
    export: 'ImageViewer',
    module: 'image-viewer',
    imports: { agent: one(sdk.contracts.agent) },
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
specifications.models = {
  contract: own.modelsContract,
  export: 'Models',
  module: 'models',
  imports: {
    settings: one(sdk.contracts.settings),
    defaults: one(own.modelDefaultContract),
    input: one(sdk.contracts.textInput),
    reader: one(sdk.contracts.textReader),
    reading: one(sdk.contracts.readingArea),
  },
};
specifications['command-palette'] = {
  contract: own.paletteContract,
  export: 'CommandPalette',
  module: 'palette',
  imports: {
    agent: one(sdk.contracts.agent),
    input: one(sdk.contracts.textInput),
    reader: one(sdk.contracts.textReader),
    reading: one(sdk.contracts.readingArea),
  },
};
specifications['directory-picker'] = {
  contract: own.directoryPickerContract,
  export: 'DirectoryPicker',
  module: 'directory-picker',
  imports: {
    agent: one(sdk.contracts.agent),
    reading: one(sdk.contracts.readingArea),
    input: one(sdk.contracts.textInput),
    reader: one(sdk.contracts.textReader),
  },
};
specifications.help = {
  contract: own.helpContract,
  export: 'Help',
  module: 'help',
  imports: { reading: one(sdk.contracts.readingArea) },
};
specifications['queue-editor'] = {
  contract: own.queueEditorContract,
  export: 'QueueEditor',
  module: 'queue-editor',
  imports: { editor: one(sdk.contracts.textarea) },
};
specifications['question-editor'] = {
  contract: own.questionEditorContract,
  export: 'QuestionEditor',
  module: 'question-editor',
  imports: { editor: one(sdk.contracts.textarea) },
};
specifications['permission-approval'] = {
  contract: own.permissionApprovalContract,
  export: 'PermissionApproval',
  module: 'permission-approval',
  imports: { reading: one(sdk.contracts.readingArea) },
};
specifications['service-approval'] = {
  contract: own.serviceApprovalContract,
  export: 'ServiceApproval',
  module: 'service-approval',
  imports: {
    agent: one(sdk.contracts.agent),
    timeline: one(own.timelineContract),
    reading: one(sdk.contracts.readingArea),
  },
};
specifications['session-details'] = {
  contract: own.sessionDetailsContract,
  export: 'SessionDetails',
  module: 'session-details',
  imports: {
    agent: one(sdk.contracts.agent),
    input: one(sdk.contracts.textInput),
    editor: one(sdk.contracts.textarea),
    reader: one(sdk.contracts.textReader),
    reading: one(sdk.contracts.readingArea),
    receipts: one(sdk.contracts.receiptReader),
    history: one(sdk.contracts.callerHistory),
  },
};
specifications['model-default'] = {
  contract: own.modelDefaultContract,
  export: 'ModelDefault',
  module: 'model-default',
  imports: {
    settings: one(sdk.contracts.settings),
    receipts: one(sdk.contracts.receiptReader),
    history: one(sdk.contracts.callerHistory),
  },
};
specifications.connections = {
  contract: own.connectionsContract,
  export: 'Connections',
  module: 'connections',
  imports: {
    settings: one(sdk.contracts.settings),
    credential: one(own.credentialContract),
    form: one(own.connectionFormContract),
  },
};
specifications['connection-form'] = {
  contract: own.connectionFormContract,
  export: 'ConnectionForm',
  module: 'connection-form',
  imports: {
    settings: one(sdk.contracts.settings),
    input: one(sdk.contracts.textInput),
    editor: one(sdk.contracts.textarea),
    reader: one(sdk.contracts.textReader),
    receipts: one(sdk.contracts.receiptReader),
    history: one(sdk.contracts.callerHistory),
  },
};
specifications.credential = {
  contract: own.credentialContract,
  export: 'Credential',
  module: 'credential',
  imports: {
    settings: one(sdk.contracts.settings),
    secret: one(sdk.contracts.secretInput),
    receipts: one(sdk.contracts.receiptReader),
    history: one(sdk.contracts.callerHistory),
  },
};
const packages = [];
const bundleInputs = new Set();
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
      contents = `import {workspace} from '${resolve(root, 'src/workspace.ts')}';import {workspaceContract} from '${resolve(root, 'src/contracts.ts')}';export let component;export function activate(configuration){component=workspace(presentations.${target},configuration.locale);return {Workspace:{kind:'component',contract:workspaceContract,component:['component']}};}`;
    else if (name === 'studio')
      contents = `import {studio,activate as exports} from '${resolve(root, 'src/studio.ts')}';export let component;export function activate(configuration){component=studio(presentations.${target},configuration.locale);return exports();}`;
    else if (name === 'connection-form')
      contents = `import {connectionForm,activate} from '${resolve(root, 'src/connection-form.ts')}';import * as validation from '${validationPath}';export const component=connectionForm(presentations.${target},validation);export {activate};`;
    else if (spec.module === 'components')
      contents = `import {components,exports} from '${resolve(root, 'src/components.ts')}';export const component=components(presentations.${target}).${name};export function activate(){const declaration=exports().${spec.export};return {${spec.export}:{...declaration,component:['component']}};}`;
    else {
      const factory = spec.module.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
      contents = `import {${factory},activate} from '${resolve(root, 'src/' + spec.module + '.ts')}';export const component=${factory}(presentations.${target});export {activate};`;
    }
    if (name !== 'appearance')
      contents = `import {presentations} from '${resolve(root, 'src/view.ts')}';` + contents;
    const bundled = await build({
      absWorkingDir: root,
      stdin: { contents, resolveDir: root, sourcefile: `${name}-${target}.ts`, loader: 'ts' },
      outfile: resolve(directory, module),
      bundle: true,
      format: 'esm',
      platform: 'neutral',
      target: 'es2022',
      logLevel: 'silent',
      metafile: true,
      banner: {
        js: (await readFile(resolve(root, 'src/view.ts'), 'utf8')).split('\n\n', 1)[0].trim(),
      },
    });
    for (const input of Object.keys(bundled.metafile.inputs))
      if (!input.startsWith('<')) bundleInputs.add(resolve(root, input));
    entries[target] = {
      implementation: { kind: 'javascript', module, realm: 'dedicated' },
      configuration: spec.configuration ?? empty,
      imports: { ...spec.imports },
      exports: { [spec.export]: required(spec.contract) },
      ...(name === 'appearance' ? {} : { scene: { name: `maka.${target}.scene`, version: 1 } }),
    };
  }
  if (name === 'workspace') entries.tui.imports.rows = one(sdk.contracts.rows);
  if (name === 'image-viewer') entries.desktop.imports.image = one(sdk.contracts.image);
  if (name === 'timeline')
    for (const target of ['desktop', 'tui'])
      entries[target].imports.reading = one(sdk.contracts.readingArea);
  // A surface is an explicit capability in the root's checked plan.
  if (name === 'workspace' || name === 'studio')
    for (const target of ['desktop', 'tui'])
      entries[target].imports.surface = one(
        target === 'desktop' ? sdk.contracts.desktopSurface : sdk.contracts.terminalSurface,
      );
  await writeFile(
    resolve(directory, 'maka.extension.json'),
    JSON.stringify({
      schemaVersion: 2,
      id: spec.id ?? `maka.agent.${name}`,
      displayName: name,
      application: { entries },
    }) + '\n',
  );
  packages.push(name);
}
const mechanical = {
  rows: { entries: { tui: native('maka.ui.rows', { Rows: required(sdk.contracts.rows) }) } },
  'studio-gateway': {
    id: 'maka.studio.gateway',
    entries: {
      host: {
        ...native('maka.application.gateway', {}),
        imports: {
          settings: one(sdk.contracts.settings),
          receipts: one(sdk.contracts.receiptReader),
        },
      },
    },
  },
  'deployment-manager': {
    entries: {
      host: native('maka.application.ui-management', {
        Management: required(sdk.contracts.deploymentManager),
      }),
    },
  },
  'deployment-gateway': {
    entries: {
      host: {
        ...native('maka.application.gateway', {}),
        imports: {
          management: one(sdk.contracts.deploymentManager),
          receipts: one(sdk.contracts.receiptReader),
        },
      },
    },
  },
  file: {
    entries: Object.fromEntries(
      ['desktop', 'tui'].map((target) => [
        target,
        native('maka.ui.file', { FilePicker: required(sdk.contracts.filePicker) }),
      ]),
    ),
  },
  image: {
    entries: { desktop: native('maka.ui.image', { Image: required(sdk.contracts.image) }) },
  },
  'reading-area': {
    entries: {
      desktop: {
        ...native('maka.ui.reading', { ReadingArea: required(sdk.contracts.readingArea) }),
        imports: {
          markdown: { contract: sdk.contracts.markdown, cardinality: { kind: 'optional' } },
          source: { contract: sdk.contracts.sourceText, cardinality: { kind: 'optional' } },
        },
      },
      tui: {
        ...native('maka.ui.reading', { ReadingArea: required(sdk.contracts.readingArea) }),
        imports: {
          source: { contract: sdk.contracts.sourceText, cardinality: { kind: 'optional' } },
        },
      },
    },
  },
  'source-text': {
    entries: {
      desktop: native('maka.ui.source', { SourceText: required(sdk.contracts.sourceText) }),
      tui: native('maka.ui.source', { SourceText: required(sdk.contracts.sourceText) }),
    },
  },
  markdown: {
    entries: {
      desktop: native('maka.ui.markdown', { Markdown: required(sdk.contracts.markdown) }),
    },
  },
  input: {
    entries: Object.fromEntries(
      ['desktop', 'tui'].map((target) => [
        target,
        native('maka.ui.input', {
          TextInput: required(sdk.contracts.textInput),
          SecretInput: required(sdk.contracts.secretInput),
        }),
      ]),
    ),
  },
  settings: {
    entries: { host: native('maka.settings', { Settings: required(sdk.contracts.settings) }) },
  },
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
      host: {
        ...native(
          'maka.agent.api',
          { agent: required(sdk.contracts.agent) },
          sdk.configurations.agent,
        ),
        imports: {
          questions: { contract: sdk.contracts.questions, cardinality: { kind: 'optional' } },
          files: { contract: sdk.contracts.workspaceFiles, cardinality: { kind: 'optional' } },
          processes: {
            contract: sdk.contracts.workspaceProcesses,
            cardinality: { kind: 'optional' },
          },
        },
      },
    },
  },
  'workspace-processes': {
    entries: {
      host: native(
        'maka.workspace.processes',
        {
          processes: required(sdk.contracts.workspaceProcesses),
          terminal: required(sdk.contracts.terminal),
        },
        sdk.configurations.workspaceProcesses,
      ),
    },
  },
  'workspace-files': {
    entries: {
      host: native('maka.workspace.files', { files: required(sdk.contracts.workspaceFiles) }),
    },
  },
  questions: {
    entries: {
      host: native('maka.interactions.questions', { questions: required(sdk.contracts.questions) }),
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
        imports: {
          agent: one(sdk.contracts.agent),
          settings: one(sdk.contracts.settings),
          receipts: one(sdk.contracts.receiptReader),
        },
      },
    },
  },
};
for (const [name, { id = `maka.agent.${name}`, ...application }] of Object.entries(mechanical)) {
  const directory = resolve(destination, 'packages', name);
  await mkdir(directory, { recursive: true });
  await writeFile(
    resolve(directory, 'maka.extension.json'),
    JSON.stringify({ schemaVersion: 2, id, displayName: name, application }) + '\n',
  );
  packages.push(name);
}
await writeFile(resolve(destination, 'packages.json'), JSON.stringify(packages, null, 2) + '\n');
await writeFile(
  resolve(destination, 'bundle-inputs.json'),
  JSON.stringify([...bundleInputs].sort()) + '\n',
);
console.log(`${packages.length} ordinary plugin packages built`);
