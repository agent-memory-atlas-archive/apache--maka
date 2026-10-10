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
  type SkillLibraryConfiguration,
  type WorkspaceProcessConfiguration,
  type CompositionInput,
  type CompositionNode,
  type PackageRef,
  type RemoteObservation,
  type Target as BindingTarget,
} from '../../plugin-sdk/src/index.js';
import { welcomeMark } from './welcome.js';
export const packageNames = [
  'workspace',
  'recovery',
  'command-palette',
  'directory-picker',
  'skill-library',
  'skills',
  'skill-outcomes',
  'session-details',
  'queue-editor',
  'question-editor',
  'form-editor',
  'forms',
  'service-approval',
  'permission-approval',
  'questions',
  'workspace-files',
  'workspace-processes',
  'help',
  'studio',
  'studio-gateway',
  'recovery-gateway',
  'deployment',
  'deployment-manager',
  'deployment-gateway',
  'image-viewer',
  'image',
  'composer',
  'navigation',
  'timeline',
  'tool-details',
  'diff',
  'markdown',
  'markdown-text',
  'source-text',
  'terminal-view',
  'terminal-panel',
  'reading-area',
  'label',
  'rows',
  'checkpoint',
  'text-file',
  'morph-mark',
  'appearance',
  'locale',
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
export const instanceNames = [...packageNames, 'recovery-reading'] as const;
export type InstanceName = (typeof instanceNames)[number];
import type { AppearanceConfiguration } from './terminal-theme.js';
export interface CompositionConfiguration {
  presentation: 'desktop' | 'tui';
  locale: string;
  scheme: 'light' | 'dark';
  appearance?: Omit<AppearanceConfiguration, 'scheme'>;
  /** Fixed local custom-theme import/export file; absent disables file operations. */
  themeFile?: string;
  packages: Record<PackageName, PackageRef>;
  keys: Record<InstanceName, string>;
  agent: AgentConfiguration;
  processes?: WorkspaceProcessConfiguration;
  skills?: SkillLibraryConfiguration;
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
  const local = (name: InstanceName, exportName: string): BindingTarget => ({
    kind: 'instance',
    path: [name],
    export: exportName,
  });
  const remote = (
    name: 'agent' | 'settings' | 'receipts' | 'workspace-processes' | 'skills' | 'skill-outcomes',
    exportName: string,
  ): BindingTarget => ({
    kind: 'remote',
    deployment: 'local',
    instance: [keys[name]],
    export: exportName,
  });
  const node = (
    name: PackageName,
    values: Record<string, unknown> = {},
    bindings: Record<string, BindingTarget[]> = {},
    identity: InstanceName = name,
  ): CompositionNode => ({
    key: keys[identity],
    package: { source: 'pinned', package: packages[name] },
    usePackage: 'entry',
    configuration: { values },
    bindings,
  });
  const host: CompositionInput = {
    environment: { location: 'host' },
    instances: {
      settings: node('settings'),
      'skill-outcomes': node('skill-outcomes'),
      receipts: node('receipts'),
      'recovery-gateway': node(
        'recovery-gateway',
        {},
        {
          skillOutcomes: [local('skill-outcomes', 'Outcomes')],
          receipts: [local('receipts', 'Receipts')],
        },
      ),
      gateway: node(
        gateway,
        {},
        {
          settings: [local('settings', 'Settings')],
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
      recovery: node(
        'recovery',
        { locale: configuration.locale },
        {
          deployment: [local('deployment', 'Deployment')],
          history: [local('history', 'DeploymentHistory')],
          skillOutcomes: [remote('skill-outcomes', 'Outcomes')],
          receipts: [remote('receipts', 'Receipts')],
          managementReceipts: [
            {
              kind: 'remote',
              deployment: 'manager',
              instance: [keys.receipts],
              export: 'Receipts',
            },
          ],
          appearance: [local('appearance', 'Appearance')],
          locale: [local('locale', 'Locale')],
          reading: [local('recovery-reading', 'ReadingArea')],
          surface: [local('surface', 'Surface')],
        },
      ),
      'source-text': node('source-text'),
      deployment: node(
        'deployment',
        {},
        {
          reading: [local('recovery-reading', 'ReadingArea')],
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
          reading: [local('reading-area', 'ReadingArea')],
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
          reading: [local('reading-area', 'ReadingArea')],
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
      appearance: node('appearance', { scheme: configuration.scheme, ...configuration.appearance }),
      locale: node('locale'),
      surface: node('surface'),
      editor: node('editor'),
      file: node('file'),
      reader: node('reader'),
      'reading-area': node('reading-area', {}, { source: [local('source-text', 'SourceText')] }),
      'recovery-reading': node(
        'reading-area',
        {},
        { source: [local('source-text', 'SourceText')] },
        'recovery-reading',
      ),
      history: node('history'),
    },
    recovery: local('recovery', 'Recovery'),
  };
  const remoteExports: RemoteObservation[] = [
    {
      capability: { deployment: 'local', instance: [keys['skill-outcomes']], export: 'Outcomes' },
      declaration: { contract: contracts.skillOutcomes, availability: 'required' },
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
  const expose = (consumer: PackageName, name: string, methods: string[], recovery: boolean) => ({
    consumer: [keys[consumer]],
    import: name,
    grant: { methods, recovery },
  });
  const connections = {
    hosts: [
      {
        name: 'manager',
        location: { kind: 'cohost' },
        deployment: configuration.managementDeployment,
        endpoints: {
          management: expose(
            'deployment-gateway',
            'management',
            ['instances', 'setEnabled', 'readDocument', 'replace', 'install'],
            false,
          ),
          receipts: expose('deployment-gateway', 'receipts', ['read'], true),
        },
      },
      {
        name: 'local',
        location: { kind: 'process', root: configuration.hostRoot },
        deployment: configuration.hostDeployment,
        endpoints: {
          receipts: expose('recovery-gateway', 'receipts', ['read'], true),
          settings: expose(
            gateway,
            'settings',
            Object.keys(contracts.settings.interface.methods),
            true,
          ),
          skillOutcomes: expose('recovery-gateway', 'skillOutcomes', ['read'], true),
        },
      },
    ],
  };
  return { host, managementHost, ui, remoteExports, connections, local, remote, node, expose };
}
/** Product source assembled exclusively from ordinary checked plugin bindings.
 * Keys and the Host deployment belong to the installation, not to a build.
 */
export function composition(configuration: CompositionConfiguration): ApplicationSources {
  const { keys } = configuration;
  const { host, managementHost, ui, remoteExports, connections, local, remote, node, expose } =
    settingsComposition(configuration, 'gateway');
  const asksForms = configuration.agent.tools.some((tool) => tool.import === 'forms');
  const asksQuestions = configuration.agent.tools.some((tool) => tool.import === 'questions');
  const fileMethods = [
    ...new Set(
      configuration.agent.tools
        .filter((tool) => tool.import === 'files')
        .map((tool) => tool.method),
    ),
  ];
  const processMethods = [
    ...new Set(
      configuration.agent.tools
        .filter((tool) => tool.import === 'processes')
        .map((tool) => tool.method),
    ),
  ];
  const skillMethods = [
    ...new Set(
      configuration.agent.tools
        .filter((tool) => tool.import === 'skills')
        .map((tool) => tool.method),
    ),
  ];
  if (configuration.agent.skillLibrary != null && configuration.agent.skillLibrary !== 'skills')
    throw new Error('Skill input preparation selects the skills Library import');
  const skillCalls = [...skillMethods, ...(configuration.agent.skillLibrary ? ['prepare'] : [])];
  if (skillCalls.length && !configuration.skills)
    throw new Error('Skill tools require explicitly selected skill collections');
  if (skillMethods.some((method) => method !== 'search' && method !== 'load'))
    throw new Error('Model skill tools select only search and load');
  if (processMethods.length && !configuration.processes)
    throw new Error(
      'Process tools require an explicit installation filesystem and network ceiling',
    );
  host.instances.agent = node(
    'agent',
    {
      ...configuration.agent,
      // These imports bind known native v1 contracts. Freeze presentation in
      // the configuration; tool names and response shapes cannot opt into it.
      tools: configuration.agent.tools.map((tool) => {
        if (tool.import === 'skills') return { ...tool, input: 'skill' as const };
        if (tool.presentation !== undefined) return tool;
        const contract =
          tool.import === 'files' &&
          ['readText', 'glob', 'grep', 'write', 'edit', 'applyPatch'].includes(tool.method)
            ? 'maka.workspace.Files'
            : tool.import === 'processes' &&
                ['execute', 'read', 'writeStdin', 'stop'].includes(tool.method)
              ? 'maka.workspace.Processes'
              : null;
        return contract ? { ...tool, presentation: `${contract}@1/${tool.method}` } : tool;
      }),
    },
    {
      ...(skillCalls.length ? { skills: [local('skills', 'skills')] } : {}),
      ...(asksForms ? { forms: [local('forms', 'forms')] } : {}),
      ...(asksQuestions ? { questions: [local('questions', 'questions')] } : {}),
      ...(fileMethods.length ? { files: [local('workspace-files', 'files')] } : {}),
      ...(processMethods.length ? { processes: [local('workspace-processes', 'processes')] } : {}),
    },
  );
  if (asksForms) host.instances.forms = node('forms');
  if (asksQuestions) host.instances.questions = node('questions');
  if (fileMethods.length) host.instances['workspace-files'] = node('workspace-files');
  if (configuration.processes)
    host.instances['workspace-processes'] = node('workspace-processes', {
      ...configuration.processes,
      sessionOwner: [keys.agent],
    });
  host.instances.gateway!.bindings!.agent = [local('agent', 'agent')];
  if (configuration.processes)
    host.instances.gateway!.bindings!.terminal = [local('workspace-processes', 'terminal')];
  if (configuration.skills) {
    host.instances.skills = node('skills', { ...configuration.skills });
    host.instances.gateway!.bindings!.skills = [local('skills', 'skills')];
    ui.instances['skill-library'] = node(
      'skill-library',
      {},
      {
        skills: [remote('skills', 'skills')],
        outcomes: [remote('skill-outcomes', 'Outcomes')],
        receipts: [remote('receipts', 'Receipts')],
        reading: [local('reading-area', 'ReadingArea')],
        source: [local('source-text', 'SourceText')],
        input: [local('input', 'TextInput')],
        reader: [local('reader', 'Reader')],
      },
    );
    remoteExports.push({
      capability: { deployment: 'local', instance: [keys.skills], export: 'skills' },
      declaration: { contract: contracts.skillLibrary, availability: 'required' },
    });
    Object.assign(connections.hosts.find((host) => host.name === 'local')!.endpoints, {
      skills: expose(
        'gateway',
        'skills',
        [
          'list',
          'read',
          ...(configuration.skills.manage ? ['preview', 'apply'] : []),
          ...(configuration.skills.preferences ? ['setPreference'] : []),
        ],
        !!(configuration.skills.manage || configuration.skills.preferences),
      ),
    });
  }
  Object.assign(ui.instances, {
    workspace: node(
      'workspace',
      {
        locale: configuration.locale,
        terminals: !!configuration.processes,
        skills: !!configuration.skills,
        place: configuration.agent.workspaceRoots[0].path,
      },
      {
        agent: [remote('agent', 'agent')],
        reading: [local('reading-area', 'ReadingArea')],
        file: [local('file', 'FilePicker')],
        receipts: [remote('receipts', 'Receipts')],
        reader: [local('reader', 'Reader')],
        history: [local('history', 'History')],
        composer: [local('composer', 'Composer')],
        ...(configuration.presentation === 'tui'
          ? {
              rows: [local('rows', 'Rows')],
              checkpoint: [local('checkpoint', 'Checkpoint')],
              themeFile: [local('text-file', 'TextFile')],
              welcome: [local('morph-mark', 'MorphMark')],
              input: [local('input', 'TextInput')],
            }
          : {}),
        palette: [local('command-palette', 'CommandPalette')],
        directories: [local('directory-picker', 'DirectoryPicker')],
        details: [local('session-details', 'SessionDetails')],
        terminals: [local('terminal-panel', 'TerminalPanel')],
        ...(configuration.skills ? { skills: [local('skill-library', 'SkillLibrary')] } : {}),
        queueEditor: [local('queue-editor', 'QueueEditor')],
        formEditor: [local('form-editor', 'FormEditor')],
        questionEditor: [local('question-editor', 'QuestionEditor')],
        serviceApproval: [local('service-approval', 'ServiceApproval')],
        permissionApproval: [local('permission-approval', 'PermissionApproval')],
        help: [local('help', 'Help')],
        models: [local('models', 'Models')],
        connections: [local('connections', 'Connections')],
        deployment: [local('deployment', 'Deployment')],
        navigation: [local('navigation', 'Navigation')],
        timeline: [local('timeline', 'Timeline')],
        imageViewer: [local('image-viewer', 'ImageViewer')],
        appearance: [local('appearance', 'Appearance')],
        locale: [local('locale', 'Locale')],
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
    'terminal-view': node('terminal-view'),
    'terminal-panel': node(
      'terminal-panel',
      {},
      {
        ...(configuration.processes
          ? { terminal: [remote('workspace-processes', 'terminal')] }
          : {}),
        view: [local('terminal-view', 'TerminalView')],
        input: [local('input', 'TextInput')],
        reader: [local('reader', 'Reader')],
        reading: [local('reading-area', 'ReadingArea')],
        receipts: [remote('receipts', 'Receipts')],
        history: [local('history', 'History')],
      },
    ),
    help: node('help', {}, { reading: [local('reading-area', 'ReadingArea')] }),
    'queue-editor': node('queue-editor', {}, { editor: [local('editor', 'Textarea')] }),
    'form-editor': node(
      'form-editor',
      {},
      { editor: [local('editor', 'Textarea')], reader: [local('reader', 'Reader')] },
    ),
    'question-editor': node('question-editor', {}, { editor: [local('editor', 'Textarea')] }),
    'permission-approval': node(
      'permission-approval',
      {},
      { reading: [local('reading-area', 'ReadingArea')] },
    ),
    'service-approval': node(
      'service-approval',
      {},
      {
        agent: [remote('agent', 'agent')],
        timeline: [local('timeline', 'Timeline')],
        reading: [local('reading-area', 'ReadingArea')],
      },
    ),
    composer: node('composer', {}, { editor: [local('editor', 'Textarea')] }),
    ...(configuration.presentation === 'tui'
      ? {
          rows: node('rows'),
          checkpoint: node('checkpoint'),
          'text-file': node('text-file', { path: configuration.themeFile ?? null }),
          'morph-mark': node('morph-mark', { ...welcomeMark }),
        }
      : {}),
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
    timeline: node(
      'timeline',
      {},
      {
        reading: [local('reading-area', 'ReadingArea')],
        ...(configuration.presentation === 'tui'
          ? {
              input: [local('input', 'TextInput')],
              reader: [local('reader', 'Reader')],
              agent: [remote('agent', 'agent')],
              formatter: [local('markdown-text', 'MarkdownText')],
            }
          : {}),
      },
    ),
    'reading-area': node(
      'reading-area',
      {},
      {
        source: [local('source-text', 'SourceText')],
        markdown: [local('markdown', 'Markdown')],
        tools: [local('tool-details', 'ToolDetails')],
        label: [local('label', 'Label')],
      },
    ),
    markdown: node('markdown'),
    ...(configuration.presentation === 'tui' ? { 'markdown-text': node('markdown-text') } : {}),
    label: node('label'),
    'tool-details': node(
      'tool-details',
      {},
      {
        agent: [remote('agent', 'agent')],
        source: [local('source-text', 'SourceText')],
        ...(configuration.presentation === 'tui' ? { diff: [local('diff', 'Diff')] } : {}),
      },
    ),
    ...(configuration.presentation === 'tui' ? { diff: node('diff') } : {}),
  });
  ui.launch = local('workspace', 'Workspace');
  remoteExports.unshift({
    capability: { deployment: 'local', instance: [keys.agent], export: 'agent' },
    declaration: { contract: contracts.agent, availability: 'required' },
  });
  Object.assign(connections.hosts.find((host) => host.name === 'local')!.endpoints, {
    agent: expose('gateway', 'agent', Object.keys(contracts.agent.interface.methods), true),
  });
  if (configuration.processes) {
    remoteExports.push({
      capability: {
        deployment: 'local',
        instance: [keys['workspace-processes']],
        export: 'terminal',
      },
      declaration: { contract: contracts.terminal, availability: 'required' },
    });
    Object.assign(connections.hosts.find((host) => host.name === 'local')!.endpoints, {
      terminal: expose(
        'gateway',
        'terminal',
        Object.keys(contracts.terminal.interface.methods),
        true,
      ),
    });
  }
  if (
    asksForms ||
    asksQuestions ||
    fileMethods.length ||
    processMethods.length ||
    skillCalls.length
  )
    Object.assign(connections.hosts.find((host) => host.name === 'local')!, {
      native: [
        ...(skillCalls.length
          ? [
              {
                origin: [keys.agent],
                consumer: [keys.agent],
                provider: [keys.skills],
                export: 'skills',
                methods: skillCalls,
                recovery: false,
              },
            ]
          : []),
        ...(asksForms
          ? [
              {
                origin: [keys.agent],
                consumer: [keys.agent],
                provider: [keys.forms],
                export: 'forms',
                methods: ['ask'],
                recovery: true,
              },
            ]
          : []),
        ...(asksQuestions
          ? [
              {
                origin: [keys.agent],
                consumer: [keys.agent],
                provider: [keys.questions],
                export: 'questions',
                methods: ['ask'],
                recovery: true,
              },
            ]
          : []),
        ...(fileMethods.length
          ? [
              {
                origin: [keys.agent],
                consumer: [keys.agent],
                provider: [keys['workspace-files']],
                export: 'files',
                methods: fileMethods,
                recovery: true,
              },
            ]
          : []),
        ...(processMethods.length
          ? [
              {
                origin: [keys.agent],
                consumer: [keys.agent],
                provider: [keys['workspace-processes']],
                export: 'processes',
                methods: processMethods,
                recovery: true,
              },
            ]
          : []),
      ],
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
      locale: [local('locale', 'Locale')],
      surface: [local('surface', 'Surface')],
    },
  );
  ui.launch = local('studio', 'Workbench');
  return { host, managementHost, ui, remoteExports, connections };
}
