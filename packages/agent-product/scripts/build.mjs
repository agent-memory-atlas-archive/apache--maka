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

import { mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, resolve } from 'node:path';
import { createRequire } from 'node:module';
import { parseArgs } from 'node:util';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const dependencyRoot = process.env.MAKA_JS_DEPS || resolve(root, '../..');
const require = createRequire(resolve(dependencyRoot, 'package.json'));
const { build } = require('esbuild');
const Ajv = require('ajv/dist/2020.js').default;
const addFormats = require('ajv-formats').default;
const standalone = require('ajv/dist/standalone/index.js').default;
const lock = JSON.parse(await readFile(resolve(root, '../../package-lock.json'), 'utf8'));
for (const name of ['esbuild', 'ajv', 'ajv-formats'])
  if (require(`${name}/package.json`).version !== lock.packages[`node_modules/${name}`]?.version)
    throw new Error(`Product build dependency does not match repository lockfile: ${name}`);
// Cargo builds and source-checkout tests must never share mutable output files.
const direct = process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
const output = direct
  ? parseArgs({ options: { output: { type: 'string' } } }).values.output
  : undefined;
const destination = output ? resolve(output) : resolve(root, 'dist');
await rm(destination, { recursive: true, force: true });
await mkdir(destination, { recursive: true });
// These authoring modules use the real generated runtime contracts. No second
// schema, package digest or native availability registry lives in this builder.
const authoring = await build({
  absWorkingDir: root,
  entryPoints: [
    resolve(root, 'src/contracts.ts'),
    resolve(root, 'src/composition.ts'),
    resolve(root, '../plugin-sdk/src/index.ts'),
  ],
  outdir: resolve(destination, 'authoring'),
  bundle: true,
  nodePaths: [resolve(dependencyRoot, 'node_modules')],
  platform: 'node',
  format: 'esm',
  outExtension: { '.js': '.mjs' },
  logLevel: 'silent',
  metafile: true,
});
const own = await import(
  pathToFileURL(resolve(destination, 'authoring/agent-product/src/contracts.mjs'))
);
const sdk = await import(pathToFileURL(resolve(destination, 'authoring/plugin-sdk/src/index.mjs')));
// Evaluate the original schemas once during authoring. Literal exports let the
// guest bundler discard unused contracts instead of carrying every native schema
// and every product contract constructor into each isolated UI module.
const guestContracts = Object.entries(own)
  .filter(([, value]) => typeof value !== 'function')
  .map(([name, value]) => `export const ${name} = ${JSON.stringify(value)};`)
  .join('\n');
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
const settingsValidation = standalone(validator, { create: 'create', update: 'update' });
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
  recovery: {
    contract: own.recoveryContract,
    export: 'Recovery',
    module: 'recovery',
    imports: {
      deployment: one(own.deploymentContract),
      history: one(sdk.contracts.callerHistory),
      receipts: one(sdk.contracts.receiptReader),
      managementReceipts: one(sdk.contracts.receiptReader),
      skillOutcomes: one(sdk.contracts.skillOutcomes),
      reading: one(sdk.contracts.readingArea),
      appearance: one(own.themeContract),
      locale: one(sdk.contracts.locale),
    },
    configuration: own.object({ locale: own.locale }),
  },
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
      locale: one(sdk.contracts.locale),
    },
    configuration: own.object({ locale: own.locale }),
  },
  'deployment-settings': {
    contract: own.deploymentContract,
    export: 'Deployment',
    module: 'deployment-settings',
    imports: {
      interface: one(own.deploymentContract),
      services: one(own.deploymentContract),
    },
  },
  deployment: {
    contract: own.deploymentContract,
    export: 'Deployment',
    module: 'deployment',
    imports: {
      reading: one(sdk.contracts.readingArea),
      management: one(sdk.contracts.deploymentManager),
      reader: one(sdk.contracts.textReader),
      receipts: one(sdk.contracts.receiptReader),
      history: one(sdk.contracts.callerHistory),
      editor: one(sdk.contracts.textarea),
      file: one(sdk.contracts.filePicker),
    },
  },
  'workhub-root': {
    id: 'maka.workhub.workspace',
    contract: own.workhubRootContract,
    export: 'WorkHub',
    module: 'workhub-root',
    imports: {
      workhub: one(sdk.contracts.workhubCoordinator),
      coordinator: one(own.workspaceContract),
      worker: one(own.workspaceContract),
      directories: one(own.directoryPickerContract),
      reading: one(sdk.contracts.readingArea),
      source: one(sdk.contracts.sourceText),
      receipts: one(sdk.contracts.receiptReader),
      history: one(sdk.contracts.callerHistory),
      appearance: one(own.themeContract),
      locale: one(sdk.contracts.locale),
    },
    configuration: own.object(
      {
        locale: own.locale,
        worker: { type: 'string', const: 'worker' },
        initialRoute: { type: 'string', enum: ['coordinator', 'sessions'] },
      },
      ['locale', 'worker'],
    ),
  },
  workspace: {
    contract: own.workspaceContract,
    export: 'Workspace',
    module: 'workspace',
    imports: {
      agent: one(sdk.contracts.agent),
      reading: one(sdk.contracts.readingArea),
      file: one(sdk.contracts.filePicker),
      models: one(own.executionPickerContract),
      connections: one(own.connectionsContract),
      deployment: one(own.deploymentContract),
      receipts: one(sdk.contracts.receiptReader),
      history: one(sdk.contracts.callerHistory),
      reader: one(sdk.contracts.textReader),
      composer: one(own.composerContract),
      palette: one(own.paletteContract),
      directories: one(own.directoryPickerContract),
      projects: one(own.projectsContract),
      imports: one(own.importsContract),
      assistantPreferences: one(own.assistantPreferencesContract),
      networkSettings: one(own.networkSettingsContract),
      sessionDefaults: one(own.sessionDefaultsContract),
      webSettings: { contract: own.webSettingsContract, cardinality: { kind: 'optional' } },
      jevSettings: { contract: own.jevSettingsContract, cardinality: { kind: 'optional' } },
      subagentSettings: {
        contract: own.subagentSettingsContract,
        cardinality: { kind: 'optional' },
      },
      insights: one(own.insightsContract),
      recall: one(own.recallContract),
      scheduler: one(own.schedulerContract),
      apps: one(own.appsContract),
      details: one(own.sessionDetailsContract),
      terminals: one(own.terminalPanelContract),
      skills: {
        contract: own.skillLibraryContract,
        cardinality: { kind: 'optional' },
      },
      queueEditor: one(own.queueEditorContract),
      formEditor: one(own.formEditorContract),
      questionEditor: one(own.questionEditorContract),
      serviceApproval: one(own.serviceApprovalContract),
      permissionApproval: one(own.permissionApprovalContract),
      help: one(own.helpContract),
      navigation: one(own.navigationContract),
      timeline: one(own.timelineContract),
      imageViewer: one(own.imageViewerContract),
      appearance: one(own.themeContract),
      locale: one(sdk.contracts.locale),
    },
    configuration: own.object(
      {
        locale: own.locale,
        terminals: { type: 'boolean' },
        skills: { type: 'boolean' },
        scheduledTasks: { type: 'boolean' },
        web: { type: 'boolean' },
        jev: { type: 'boolean' },
        place: { type: 'string', maxLength: 4096 },
      },
      ['locale'],
    ),
  },
  apps: {
    contract: own.appsContract,
    export: 'Apps',
    module: 'apps',
    imports: {
      pages: {
        contract: own.applicationPageContract,
        cardinality: { kind: 'many', min: 0, max: 4096 },
      },
      management: one(sdk.contracts.deploymentManager),
      receipts: one(sdk.contracts.receiptReader),
      history: one(sdk.contracts.callerHistory),
      reading: one(sdk.contracts.readingArea),
      input: one(sdk.contracts.textInput),
      editor: one(sdk.contracts.textarea),
      reader: one(sdk.contracts.textReader),
      file: one(sdk.contracts.filePicker),
    },
    configuration: own.object({
      root: own.string,
      entries: { type: 'array', items: own.appEntrySchema },
    }),
  },
  'skill-library': {
    contract: own.skillLibraryContract,
    export: 'SkillLibrary',
    module: 'skill-library',
    imports: {
      skills: one(sdk.contracts.skillLibrary),
      outcomes: one(sdk.contracts.skillOutcomes),
      receipts: one(sdk.contracts.receiptReader),
      file: one(sdk.contracts.filePicker),
      input: one(sdk.contracts.textInput),
      reader: one(sdk.contracts.textReader),
      source: one(sdk.contracts.sourceText),
      reading: one(sdk.contracts.readingArea),
    },
  },
  'terminal-panel': {
    contract: own.terminalPanelContract,
    export: 'TerminalPanel',
    module: 'terminal-panel',
    imports: {
      terminal: { contract: sdk.contracts.terminal, cardinality: { kind: 'optional' } },
      view: one(sdk.contracts.terminalView),
      input: one(sdk.contracts.textInput),
      reader: one(sdk.contracts.textReader),
      reading: one(sdk.contracts.readingArea),
      receipts: one(sdk.contracts.receiptReader),
      history: one(sdk.contracts.callerHistory),
    },
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
    configuration: own.appearanceConfigurationSchema,
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
specifications['execution-picker'] = {
  contract: own.executionPickerContract,
  export: 'ExecutionPicker',
  module: 'execution-picker',
  imports: {
    models: one(own.modelsContract),
    agent: one(sdk.contracts.agent),
    receipts: one(sdk.contracts.receiptReader),
    input: one(sdk.contracts.textInput),
    reader: one(sdk.contracts.textReader),
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
specifications['session-defaults'] = {
  contract: own.sessionDefaultsContract,
  export: 'SessionDefaults',
  module: 'session-defaults',
  configuration: own.object({
    instance: { type: 'array', items: own.string, minItems: 1, maxItems: 16 },
  }),
  imports: {
    management: one(sdk.contracts.deploymentManager),
    receipts: one(sdk.contracts.receiptReader),
    history: one(sdk.contracts.callerHistory),
  },
};
specifications['assistant-preferences'] = {
  contract: own.assistantPreferencesContract,
  export: 'AssistantPreferences',
  module: 'assistant-preferences',
  imports: {
    settings: one(sdk.contracts.settings),
    input: one(sdk.contracts.textInput),
    editor: one(sdk.contracts.textarea),
    reader: one(sdk.contracts.textReader),
    receipts: one(sdk.contracts.receiptReader),
    history: one(sdk.contracts.callerHistory),
  },
};
specifications['network-settings'] = {
  contract: own.networkSettingsContract,
  export: 'NetworkSettings',
  module: 'network-settings',
  imports: {
    settings: one(sdk.contracts.settings),
    input: one(sdk.contracts.textInput),
    secret: one(sdk.contracts.secretInput),
    reader: one(sdk.contracts.textReader),
    receipts: one(sdk.contracts.receiptReader),
    history: one(sdk.contracts.callerHistory),
  },
};
specifications['subagent-settings'] = {
  contract: own.subagentSettingsContract,
  export: 'SubagentSettings',
  module: 'subagent-settings',
  imports: {
    workhub: one(sdk.contracts.workhubCoordinator),
    agent: one(sdk.contracts.agent),
    models: one(own.modelsContract),
    input: one(sdk.contracts.textInput),
    reader: one(sdk.contracts.textReader),
    receipts: one(sdk.contracts.receiptReader),
    history: one(sdk.contracts.callerHistory),
  },
};
specifications['jev-settings'] = {
  contract: own.jevSettingsContract,
  export: 'JevSettings',
  module: 'jev-settings',
  imports: {
    jev: one(sdk.contracts.workspaceJev),
    input: one(sdk.contracts.textInput),
    secret: one(sdk.contracts.secretInput),
    reader: one(sdk.contracts.textReader),
    receipts: one(sdk.contracts.receiptReader),
    history: one(sdk.contracts.callerHistory),
  },
};
specifications['web-settings'] = {
  contract: own.webSettingsContract,
  export: 'WebSettings',
  module: 'web-settings',
  imports: {
    web: one(sdk.contracts.workspaceWeb),
    secret: one(sdk.contracts.secretInput),
    receipts: one(sdk.contracts.receiptReader),
    history: one(sdk.contracts.callerHistory),
  },
};
specifications['insights'] = {
  contract: own.insightsContract,
  export: 'Insights',
  module: 'insights',
  imports: {
    settings: one(sdk.contracts.settings),
    input: one(sdk.contracts.textInput),
    reader: one(sdk.contracts.textReader),
    receipts: one(sdk.contracts.receiptReader),
    history: one(sdk.contracts.callerHistory),
  },
};
specifications.recall = {
  contract: own.recallContract,
  export: 'Recall',
  module: 'recall',
  imports: {
    agent: one(sdk.contracts.agent),
    reading: one(sdk.contracts.readingArea),
    input: one(sdk.contracts.textInput),
    reader: one(sdk.contracts.textReader),
  },
};
specifications.scheduler = {
  contract: own.schedulerContract,
  export: 'Scheduler',
  module: 'scheduler',
  imports: {
    agent: one(sdk.contracts.agent),
    reading: one(sdk.contracts.readingArea),
    input: one(sdk.contracts.textInput),
    editor: one(sdk.contracts.textarea),
    reader: one(sdk.contracts.textReader),
    models: one(own.modelsContract),
    receipts: one(sdk.contracts.receiptReader),
    history: one(sdk.contracts.callerHistory),
  },
};
specifications.projects = {
  contract: own.projectsContract,
  export: 'Projects',
  module: 'projects',
  imports: {
    agent: one(sdk.contracts.agent),
    directories: one(own.directoryPickerContract),
    reading: one(sdk.contracts.readingArea),
    input: one(sdk.contracts.textInput),
    reader: one(sdk.contracts.textReader),
    receipts: one(sdk.contracts.receiptReader),
    history: one(sdk.contracts.callerHistory),
  },
};
specifications['conversation-import'] = {
  contract: own.importsContract,
  export: 'Imports',
  module: 'imports',
  imports: {
    agent: one(sdk.contracts.agent),
    directories: one(own.directoryPickerContract),
    reading: one(sdk.contracts.readingArea),
    input: one(sdk.contracts.textInput),
    reader: one(sdk.contracts.textReader),
    receipts: one(sdk.contracts.receiptReader),
    history: one(sdk.contracts.callerHistory),
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
specifications['form-editor'] = {
  contract: own.formEditorContract,
  export: 'FormEditor',
  module: 'form-editor',
  imports: { editor: one(sdk.contracts.textarea), reader: one(sdk.contracts.textReader) },
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
specifications['tool-details'] = {
  contract: own.toolDetailsContract,
  export: 'ToolDetails',
  module: 'tool-details',
  imports: { agent: one(sdk.contracts.agent), source: one(sdk.contracts.sourceText) },
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
    reading: one(sdk.contracts.readingArea),
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
    reading: one(sdk.contracts.readingArea),
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
const bundleInputs = new Set(
  Object.keys(authoring.metafile.inputs).map((input) => resolve(root, input)),
);
for (const [name, spec] of Object.entries(specifications)) {
  const directory = resolve(destination, 'packages', name);
  await mkdir(directory, { recursive: true });
  const entries = {};
  for (const target of ['desktop', 'tui']) {
    const module = target + '.js';
    let contents;
    if (name === 'appearance')
      contents = `import {activate as appearance} from '${resolve(root, 'src/theme.ts')}';export function activate(configuration){return appearance(configuration,${target === 'tui'});}`;
    else if (name === 'workspace')
      contents = `import {workspace} from '${resolve(root, 'src/workspace.ts')}';import {workspaceContract} from '${resolve(root, 'src/contracts.ts')}';export let component;export function activate(configuration){component=workspace(presentations.${target},configuration.locale,configuration.terminals === true, configuration.place ?? '', configuration.skills === true, configuration.scheduledTasks === true, configuration.web === true, configuration.jev === true);return {Workspace:{kind:'component',contract:workspaceContract,component:['component']}};}`;
    else if (name === 'apps')
      contents = `import {apps,activate as exports} from '${resolve(root, 'src/apps.ts')}';export let component;export function activate(configuration){component=apps(presentations.${target},configuration);return exports();}`;
    else if (name === 'session-defaults')
      contents = `import {sessionDefaults,activate as exports} from '${resolve(root, 'src/session-defaults.ts')}';export let component;export function activate(configuration){component=sessionDefaults(presentations.${target},configuration.instance);return exports();}`;
    else if (name === 'workhub-root')
      contents = `import {workhubRoot,activate as exports} from '${resolve(root, 'src/workhub-root.ts')}';export let component;export function activate(configuration){component=workhubRoot(presentations.${target},configuration.locale,configuration.worker,configuration.initialRoute);return exports();}`;
    else if (name === 'studio' || name === 'recovery')
      contents = `import {${name},activate as exports} from '${resolve(root, 'src/' + name + '.ts')}';export let component;export function activate(configuration){component=${name}(presentations.${target},configuration.locale);return exports();}`;
    else if (name === 'connection-form')
      contents = `import {connectionForm,activate} from '${resolve(root, 'src/connection-form.ts')}';import * as validation from 'maka:settings-validation';export const component=connectionForm(presentations.${target},validation);export {activate};`;
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
      nodePaths: [resolve(dependencyRoot, 'node_modules')],
      format: 'esm',
      platform: 'neutral',
      target: 'es2022',
      logLevel: 'silent',
      metafile: true,
      plugins: [
        {
          name: 'product-contract-literals',
          setup(builder) {
            // The generated validator has a stable module identity, independent
            // of Cargo OUT_DIR or the source-checkout output directory.
            builder.onResolve({ filter: /^maka:settings-validation$/ }, () => ({
              path: 'settings-validation',
              namespace: 'product-generated',
            }));
            builder.onLoad({ filter: /.*/, namespace: 'product-generated' }, () => ({
              contents: settingsValidation,
              loader: 'js',
              resolveDir: root,
            }));
            builder.onLoad({ filter: /\/contracts\.ts$/ }, (args) =>
              args.path === resolve(root, 'src/contracts.ts')
                ? { contents: guestContracts, loader: 'js' }
                : undefined,
            );
          },
        },
      ],
      banner: {
        js: (await readFile(resolve(root, 'src/view.ts'), 'utf8')).split('\n\n', 1)[0].trim(),
      },
    });
    for (const input of Object.keys(bundled.metafile.inputs))
      if (!input.startsWith('<') && !input.startsWith('product-generated:'))
        bundleInputs.add(resolve(root, input));
    entries[target] = {
      implementation: { kind: 'javascript', module, realm: 'dedicated' },
      configuration: spec.configuration ?? empty,
      imports: { ...spec.imports },
      exports: { [spec.export]: required(spec.contract) },
      ...(name === 'appearance' ? {} : { scene: { name: `maka.${target}.scene`, version: 1 } }),
    };
  }
  if (name === 'workspace')
    Object.assign(entries.tui.imports, {
      rows: one(sdk.contracts.rows),
      checkpoint: one(sdk.contracts.valueCheckpoint),
      themeFile: one(sdk.contracts.textFile),
      welcome: one(sdk.contracts.morphMark),
      input: one(sdk.contracts.textInput),
    });
  if (name === 'deployment-settings' || name === 'deployment' || name === 'insights')
    entries.tui.imports.checkpoint = one(sdk.contracts.valueCheckpoint);
  if (name === 'tool-details') entries.tui.imports.diff = one(sdk.contracts.diff);
  if (name === 'image-viewer') entries.desktop.imports.image = one(sdk.contracts.image);
  if (name === 'timeline') {
    for (const target of ['desktop', 'tui'])
      entries[target].imports.reading = one(sdk.contracts.readingArea);
    Object.assign(entries.tui.imports, {
      formatter: one(sdk.contracts.markdownText),
      input: one(sdk.contracts.textInput),
      reader: one(sdk.contracts.textReader),
      agent: one(sdk.contracts.agent),
    });
  }
  // A surface is an explicit capability in the root's checked plan.
  if (name === 'workspace' || name === 'studio' || name === 'recovery' || name === 'workhub-root')
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
const agentImports = {
  executorSetups: {
    contract: sdk.contracts.executorSetup,
    cardinality: { kind: 'many', min: 0, max: 8 },
  },
  executors: { contract: sdk.contracts.executor, cardinality: { kind: 'many', min: 0, max: 8 } },
  web: { contract: sdk.contracts.workspaceWeb, cardinality: { kind: 'optional' } },
  skills: { contract: sdk.contracts.skillLibrary, cardinality: { kind: 'optional' } },
  forms: { contract: sdk.contracts.forms, cardinality: { kind: 'optional' } },
  questions: { contract: sdk.contracts.questions, cardinality: { kind: 'optional' } },
  files: { contract: sdk.contracts.workspaceFiles, cardinality: { kind: 'optional' } },
  processes: {
    contract: sdk.contracts.workspaceProcesses,
    cardinality: { kind: 'optional' },
  },
};
const mechanical = {
  locale: {
    entries: Object.fromEntries(
      ['desktop', 'tui'].map((target) => [
        target,
        native('maka.ui.locale', { Locale: required(sdk.contracts.locale) }),
      ]),
    ),
  },
  'terminal-view': {
    entries: Object.fromEntries(
      ['desktop', 'tui'].map((target) => [
        target,
        native('maka.ui.terminal', { TerminalView: required(sdk.contracts.terminalView) }),
      ]),
    ),
  },
  'morph-mark': {
    entries: {
      tui: native(
        'maka.ui.morph-mark',
        { MorphMark: required(sdk.contracts.morphMark) },
        sdk.configurations.morphMark,
      ),
    },
  },
  'text-file': {
    entries: {
      tui: native(
        'maka.ui.text-file',
        { TextFile: required(sdk.contracts.textFile) },
        sdk.configurations.textFile,
      ),
    },
  },
  checkpoint: {
    entries: {
      tui: native('maka.ui.checkpoint', { Checkpoint: required(sdk.contracts.valueCheckpoint) }),
    },
  },
  rows: { entries: { tui: native('maka.ui.rows', { Rows: required(sdk.contracts.rows) }) } },
  'recovery-gateway': {
    entries: {
      host: {
        ...native('maka.application.gateway', {}),
        imports: {
          skillOutcomes: one(sdk.contracts.skillOutcomes),
          receipts: one(sdk.contracts.receiptReader),
        },
      },
    },
  },
  'studio-gateway': {
    id: 'maka.studio.gateway',
    entries: {
      host: {
        ...native('maka.application.gateway', {}),
        imports: {
          settings: one(sdk.contracts.settings),
        },
      },
    },
  },
  'host-deployment-manager': {
    entries: {
      host: native('maka.application.management', {
        Management: required(sdk.contracts.deploymentManager),
      }),
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
          tools: { contract: own.toolDetailsContract, cardinality: { kind: 'optional' } },
          markdown: { contract: sdk.contracts.markdown, cardinality: { kind: 'optional' } },
          source: { contract: sdk.contracts.sourceText, cardinality: { kind: 'optional' } },
          label: { contract: sdk.contracts.label, cardinality: { kind: 'optional' } },
        },
      },
      tui: {
        ...native('maka.ui.reading', { ReadingArea: required(sdk.contracts.readingArea) }),
        imports: {
          tools: { contract: own.toolDetailsContract, cardinality: { kind: 'optional' } },
          markdown: { contract: sdk.contracts.markdown, cardinality: { kind: 'optional' } },
          source: { contract: sdk.contracts.sourceText, cardinality: { kind: 'optional' } },
          label: { contract: sdk.contracts.label, cardinality: { kind: 'optional' } },
        },
      },
    },
  },
  label: {
    entries: Object.fromEntries(
      ['desktop', 'tui'].map((target) => [
        target,
        native('maka.ui.label', { Label: required(sdk.contracts.label) }),
      ]),
    ),
  },
  diff: { entries: { tui: native('maka.ui.diff', { Diff: required(sdk.contracts.diff) }) } },
  'source-text': {
    entries: {
      desktop: native('maka.ui.source', { SourceText: required(sdk.contracts.sourceText) }),
      tui: native('maka.ui.source', { SourceText: required(sdk.contracts.sourceText) }),
    },
  },
  'markdown-text': {
    entries: {
      tui: native('maka.text.markdown.tui', { MarkdownText: required(sdk.contracts.markdownText) }),
    },
  },
  markdown: {
    entries: {
      desktop: native('maka.ui.markdown', { Markdown: required(sdk.contracts.markdown) }),
      tui: native('maka.ui.markdown', { Markdown: required(sdk.contracts.markdown) }),
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
        native('maka.operations.history', {
          History: required(sdk.contracts.callerHistory),
          DeploymentHistory: required(sdk.contracts.callerHistory),
        }),
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
        imports: agentImports,
      },
    },
  },
  workhub: {
    id: 'maka.workhub.api',
    entries: {
      host: {
        ...native(
          'maka.workhub.api',
          {
            agent: required(sdk.contracts.agent),
            workhub: required(sdk.contracts.workhubCoordinator),
          },
          sdk.configurations.workhub,
        ),
        imports: { ...agentImports, worker: one(sdk.contracts.agent) },
      },
    },
  },
  'workspace-web': {
    entries: {
      host: native(
        'maka.workspace.web',
        { web: required(sdk.contracts.workspaceWeb) },
        sdk.configurations.workspaceWeb,
      ),
    },
  },
  'workspace-jev': {
    entries: {
      host: native(
        'maka.workspace.jev',
        { jev: required(sdk.contracts.workspaceJev) },
        sdk.configurations.workspaceJev,
      ),
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
  'protocol-processes': {
    entries: {
      host: native(
        'maka.workspace.protocol-processes',
        { protocol: required(sdk.contracts.protocolProcesses) },
        sdk.configurations.workspaceProcesses,
      ),
    },
  },
  'acp-executor': {
    entries: {
      host: {
        ...native(
          'maka.agent.acp',
          {
            executor: required(sdk.contracts.executor),
            setup: required(sdk.contracts.executorSetup),
          },
          sdk.configurations.acpExecutor,
        ),
        imports: {
          processes: one(sdk.contracts.protocolProcesses),
          files: one(sdk.contracts.workspaceFiles),
          forms: one(sdk.contracts.forms),
        },
      },
    },
  },
  skills: {
    entries: {
      host: native(
        'maka.skills.library',
        { skills: required(sdk.contracts.skillLibrary) },
        sdk.configurations.skillLibrary,
      ),
    },
  },
  'workspace-files': {
    entries: {
      host: native('maka.workspace.files', { files: required(sdk.contracts.workspaceFiles) }),
    },
  },
  forms: {
    entries: { host: native('maka.interactions.forms', { forms: required(sdk.contracts.forms) }) },
  },
  questions: {
    entries: {
      host: native('maka.interactions.questions', { questions: required(sdk.contracts.questions) }),
    },
  },
  'skill-outcomes': {
    entries: {
      host: native('maka.skills.outcomes', { Outcomes: required(sdk.contracts.skillOutcomes) }),
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
          coordinator: { contract: sdk.contracts.agent, cardinality: { kind: 'optional' } },
          workhub: {
            contract: sdk.contracts.workhubCoordinator,
            cardinality: { kind: 'optional' },
          },
          coordinatorTerminal: {
            contract: sdk.contracts.terminal,
            cardinality: { kind: 'optional' },
          },
          terminal: { contract: sdk.contracts.terminal, cardinality: { kind: 'optional' } },
          jev: { contract: sdk.contracts.workspaceJev, cardinality: { kind: 'optional' } },
          web: { contract: sdk.contracts.workspaceWeb, cardinality: { kind: 'optional' } },
          skills: { contract: sdk.contracts.skillLibrary, cardinality: { kind: 'optional' } },
          settings: one(sdk.contracts.settings),
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
