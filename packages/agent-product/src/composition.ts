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
  'composer',
  'navigation',
  'timeline',
  'appearance',
  'surface',
  'editor',
  'reader',
  'history',
  'models',
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
}
/** Product source assembled exclusively from ordinary checked plugin bindings.
 * Keys and the Host deployment belong to the installation, not to a build.
 */
export function composition(configuration: CompositionConfiguration): {
  host: CompositionInput;
  ui: CompositionInput;
  remoteExports: RemoteObservation[];
  connections: unknown;
} {
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
      agent: node('agent', { ...configuration.agent }),
      settings: node('settings'),
      receipts: node('receipts'),
      gateway: node(
        'gateway',
        {},
        {
          agent: [local('agent', 'agent')],
          settings: [local('settings', 'Settings')],
          receipts: [local('receipts', 'Receipts')],
        },
      ),
    },
  };
  const ui: CompositionInput = {
    environment: { location: 'ui', presentation: configuration.presentation },
    instances: {
      workspace: node(
        'workspace',
        { locale: configuration.locale },
        {
          agent: [remote('agent', 'agent')],
          receipts: [remote('receipts', 'Receipts')],
          reader: [local('reader', 'Reader')],
          history: [local('history', 'History')],
          composer: [local('composer', 'Composer')],
          models: [local('models', 'Models')],
          connections: [local('connections', 'Connections')],
          navigation: [local('navigation', 'Navigation')],
          timeline: [local('timeline', 'Timeline')],
          appearance: [local('appearance', 'Appearance')],
          surface: [local('surface', 'Surface')],
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
      models: node('models', {}, { settings: [remote('settings', 'Settings')] }),
      composer: node('composer', {}, { editor: [local('editor', 'Textarea')] }),
      navigation: node('navigation'),
      timeline: node('timeline'),
      appearance: node('appearance', { scheme: configuration.scheme }),
      surface: node('surface'),
      editor: node('editor'),
      reader: node('reader'),
      history: node('history'),
    },
    launch: local('workspace', 'Workspace'),
  };
  const remoteExports: RemoteObservation[] = [
    {
      capability: { deployment: 'local', instance: [keys.agent], export: 'agent' },
      declaration: { contract: contracts.agent, availability: 'required' },
    },
    {
      capability: { deployment: 'local', instance: [keys.receipts], export: 'Receipts' },
      declaration: { contract: contracts.receiptReader, availability: 'required' },
    },
  ];
  remoteExports.push({
    capability: { deployment: 'local', instance: [keys.settings], export: 'Settings' },
    declaration: { contract: contracts.settings, availability: 'required' },
  });
  const connections = {
    hosts: [
      {
        name: 'local',
        root: configuration.hostRoot,
        deployment: configuration.hostDeployment,
        consumer: [keys.gateway],
        imports: {
          agent: { methods: Object.keys(contracts.agent.interface.methods), recovery: true },
          receipts: { methods: ['read'], recovery: true },
          settings: { methods: Object.keys(contracts.settings.interface.methods), recovery: true },
        },
      },
    ],
  };
  return { host, ui, remoteExports, connections };
}
