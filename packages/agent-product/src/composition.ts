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

import {
  contracts,
  type AgentConfiguration,
  type CompositionInput,
  type CompositionNode,
  type PackageRef,
  type RemoteObservation,
  type Target as BindingTarget,
} from '../../plugin-sdk/src/index.js';
export const packageNames = [
  'workspace',
  'command-palette',
  'directory-picker',
  'session-details',
  'studio',
  'studio-gateway',
  'deployment',
  'deployment-manager',
  'deployment-gateway',
  'image-viewer',
  'image',
  'composer',
  'navigation',
  'timeline',
  'markdown',
  'source-text',
  'reading-area',
  'appearance',
  'surface',
  'editor',
  'file',
  'reader',
  'history',
  'models',
  'model-default',
  'settings',
  'connections',
  'credential',
  'connection-form',
  'input',
  'agent',
  'receipts',
  'gateway',
] as const;
export type PackageName = (typeof packageNames)[number];
export interface CompositionConfiguration {
  presentation: 'desktop' | 'tui';
  locale: string;
  scheme: 'light' | 'dark';
  packages: Record<PackageName, PackageRef>;
  keys: Record<PackageName, string>;
  agent: AgentConfiguration;
  hostRoot: string;
  hostDeployment: string;
  managementDeployment: string;
}
interface ApplicationSources {
  host: CompositionInput;
  managementHost: CompositionInput;
  ui: CompositionInput;
  remoteExports: RemoteObservation[];
  connections: unknown;
}
/** Shared ordinary settings/management graph; each product adds its own root and capabilities. */
function settingsComposition(
  configuration: Omit<CompositionConfiguration, 'agent'>,
  gateway: 'gateway' | 'studio-gateway',
) {
  const { packages, keys } = configuration;
  const local = (name: PackageName, exportName: string): BindingTarget => ({
    kind: 'instance',
    path: [name],
    export: exportName,
  });
  const remote = (name: 'agent' | 'settings' | 'receipts', exportName: string): BindingTarget => ({
    kind: 'remote',
    deployment: 'local',
    instance: [keys[name]],
    export: exportName,
  });
  const node = (
    name: PackageName,
    values: Record<string, unknown> = {},
    bindings: Record<string, BindingTarget[]> = {},
  ): CompositionNode => ({
    key: keys[name],
    package: { source: 'pinned', package: packages[name] },
    usePackage: 'entry',
    configuration: { values },
    bindings,
  });
  const host: CompositionInput = {
    environment: { location: 'host' },
    instances: {
      settings: node('settings'),
      receipts: node('receipts'),
      gateway: node(
        gateway,
        {},
        {
          settings: [local('settings', 'Settings')],
          receipts: [local('receipts', 'Receipts')],
        },
      ),
    },
  };
  const managementHost: CompositionInput = {
    environment: { location: 'host' },
    instances: {
      management: node('deployment-manager'),
      receipts: node('receipts'),
      gateway: node(
        'deployment-gateway',
        {},
        {
          management: [{ kind: 'instance', path: ['management'], export: 'Management' }],
          receipts: [local('receipts', 'Receipts')],
        },
      ),
    },
  };
  const ui: CompositionInput = {
    environment: { location: 'ui', presentation: configuration.presentation },
    instances: {
      deployment: node(
        'deployment',
        {},
        {
          editor: [local('editor', 'Textarea')],
          file: [local('file', 'FilePicker')],
          management: [
            {
              kind: 'remote',
              deployment: 'manager',
              instance: [keys['deployment-manager']],
              export: 'Management',
            },
          ],
          receipts: [
            {
              kind: 'remote',
              deployment: 'manager',
              instance: [keys.receipts],
              export: 'Receipts',
            },
          ],
          history: [local('history', 'History')],
        },
      ),
      connections: node(
        'connections',
        {},
        {
          settings: [remote('settings', 'Settings')],
          credential: [local('credential', 'Credential')],
          form: [local('connection-form', 'ConnectionForm')],
        },
      ),
      credential: node(
        'credential',
        {},
        {
          settings: [remote('settings', 'Settings')],
          secret: [local('input', 'SecretInput')],
          receipts: [remote('receipts', 'Receipts')],
          history: [local('history', 'History')],
        },
      ),
      input: node('input'),
      'connection-form': node(
        'connection-form',
        {},
        {
          settings: [remote('settings', 'Settings')],
          input: [local('input', 'TextInput')],
          editor: [local('editor', 'Textarea')],
          reader: [local('reader', 'Reader')],
          receipts: [remote('receipts', 'Receipts')],
          history: [local('history', 'History')],
        },
      ),
      models: node(
        'models',
        {},
        {
          settings: [remote('settings', 'Settings')],
          defaults: [local('model-default', 'ModelDefault')],
          reading: [local('reading-area', 'ReadingArea')],
          input: [local('input', 'TextInput')],
          reader: [local('reader', 'Reader')],
        },
      ),
      'model-default': node(
        'model-default',
        {},
        {
          settings: [remote('settings', 'Settings')],
          receipts: [remote('receipts', 'Receipts')],
          history: [local('history', 'History')],
        },
      ),
      appearance: node('appearance', { scheme: configuration.scheme }),
      surface: node('surface'),
      editor: node('editor'),
      file: node('file'),
      reader: node('reader'),
      'reading-area': node('reading-area'),
      history: node('history'),
    },
  };
  const remoteExports: RemoteObservation[] = [
    {
      capability: { deployment: 'local', instance: [keys.receipts], export: 'Receipts' },
      declaration: { contract: contracts.receiptReader, availability: 'required' },
    },
  ];
  remoteExports.push({
    capability: { deployment: 'local', instance: [keys.settings], export: 'Settings' },
    declaration: { contract: contracts.settings, availability: 'required' },
  });
  remoteExports.push(
    {
      capability: {
        deployment: 'manager',
        instance: [keys['deployment-manager']],
        export: 'Management',
      },
      declaration: { contract: contracts.deploymentManager, availability: 'required' },
    },
    {
      capability: { deployment: 'manager', instance: [keys.receipts], export: 'Receipts' },
      declaration: { contract: contracts.receiptReader, availability: 'required' },
    },
  );
  const connections = {
    hosts: [
      {
        name: 'manager',
        location: { kind: 'cohost' },
        deployment: configuration.managementDeployment,
        consumer: [keys['deployment-gateway']],
        imports: {
          management: {
            methods: ['instances', 'setEnabled', 'readDocument', 'replace', 'install'],
            recovery: false,
          },
          receipts: { methods: ['read'], recovery: true },
        },
      },
      {
        name: 'local',
        location: { kind: 'process', root: configuration.hostRoot },
        deployment: configuration.hostDeployment,
        consumer: [keys[gateway]],
        imports: {
          receipts: { methods: ['read'], recovery: true },
          settings: { methods: Object.keys(contracts.settings.interface.methods), recovery: true },
        },
      },
    ],
  };
  return { host, managementHost, ui, remoteExports, connections, local, remote, node };
}
/** Product source assembled exclusively from ordinary checked plugin bindings.
 * Keys and the Host deployment belong to the installation, not to a build.
 */
export function composition(configuration: CompositionConfiguration): ApplicationSources {
  const { keys } = configuration;
  const { host, managementHost, ui, remoteExports, connections, local, remote, node } =
    settingsComposition(configuration, 'gateway');
  host.instances.agent = node('agent', { ...configuration.agent });
  host.instances.gateway!.bindings!.agent = [local('agent', 'agent')];
  Object.assign(ui.instances, {
    workspace: node(
      'workspace',
      { locale: configuration.locale },
      {
        agent: [remote('agent', 'agent')],
        receipts: [remote('receipts', 'Receipts')],
        reader: [local('reader', 'Reader')],
        history: [local('history', 'History')],
        composer: [local('composer', 'Composer')],
        palette: [local('command-palette', 'CommandPalette')],
        directories: [local('directory-picker', 'DirectoryPicker')],
        details: [local('session-details', 'SessionDetails')],
        models: [local('models', 'Models')],
        connections: [local('connections', 'Connections')],
        deployment: [local('deployment', 'Deployment')],
        navigation: [local('navigation', 'Navigation')],
        timeline: [local('timeline', 'Timeline')],
        imageViewer: [local('image-viewer', 'ImageViewer')],
        appearance: [local('appearance', 'Appearance')],
        surface: [local('surface', 'Surface')],
      },
    ),
    'command-palette': node(
      'command-palette',
      {},
      {
        agent: [remote('agent', 'agent')],
        input: [local('input', 'TextInput')],
        reader: [local('reader', 'Reader')],
        reading: [local('reading-area', 'ReadingArea')],
      },
    ),
    'directory-picker': node(
      'directory-picker',
      {},
      {
        agent: [remote('agent', 'agent')],
        reading: [local('reading-area', 'ReadingArea')],
        input: [local('input', 'TextInput')],
        reader: [local('reader', 'Reader')],
      },
    ),
    'session-details': node(
      'session-details',
      {},
      {
        agent: [remote('agent', 'agent')],
        input: [local('input', 'TextInput')],
        editor: [local('editor', 'Textarea')],
        reader: [local('reader', 'Reader')],
        reading: [local('reading-area', 'ReadingArea')],
        receipts: [remote('receipts', 'Receipts')],
        history: [local('history', 'History')],
      },
    ),
    composer: node('composer', {}, { editor: [local('editor', 'Textarea')] }),
    navigation: node('navigation'),
    'image-viewer': node(
      'image-viewer',
      {},
      {
        agent: [remote('agent', 'agent')],
        ...(configuration.presentation === 'desktop' ? { image: [local('image', 'Image')] } : {}),
      },
    ),
    ...(configuration.presentation === 'desktop' ? { image: node('image') } : {}),
    timeline: node('timeline', {}, { reading: [local('reading-area', 'ReadingArea')] }),
    'source-text': node('source-text'),
    'reading-area': node(
      'reading-area',
      {},
      {
        source: [local('source-text', 'SourceText')],
        ...(configuration.presentation === 'desktop'
          ? { markdown: [local('markdown', 'Markdown')] }
          : {}),
      },
    ),
    ...(configuration.presentation === 'desktop' ? { markdown: node('markdown') } : {}),
  });
  ui.launch = local('workspace', 'Workspace');
  remoteExports.unshift({
    capability: { deployment: 'local', instance: [keys.agent], export: 'agent' },
    declaration: { contract: contracts.agent, availability: 'required' },
  });
  Object.assign(connections.hosts.find((host) => host.name === 'local')!.imports, {
    agent: { methods: Object.keys(contracts.agent.interface.methods), recovery: true },
  });
  return { host, managementHost, ui, remoteExports, connections };
}
/** Independent configuration workbench: no Agent capability or conversation UI. */
export function studioComposition(
  configuration: Omit<CompositionConfiguration, 'agent'>,
): ApplicationSources {
  const { host, managementHost, ui, remoteExports, connections, local, node } = settingsComposition(
    configuration,
    'studio-gateway',
  );
  ui.instances.studio = node(
    'studio',
    { locale: configuration.locale },
    {
      connections: [local('connections', 'Connections')],
      models: [local('models', 'Models')],
      deployment: [local('deployment', 'Deployment')],
      appearance: [local('appearance', 'Appearance')],
      surface: [local('surface', 'Surface')],
    },
  );
  ui.launch = local('studio', 'Workbench');
  return { host, managementHost, ui, remoteExports, connections };
}
