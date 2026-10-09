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
  contracts as native,
  type Contract,
  type AgentSession,
  type InteractionRecord,
  type SettingsModelChoice,
  type ThinkingLevel,
  type SettingsDefaultModel,
  type TextareaAppearance,
  type MarkdownAppearance,
  type InputAppearance,
  type ConnectionCredentialTarget,
  type SettingsConnectionDetail,
  type MenuAppearance,
  type EditorCommand,
  type EditorObservation,
  type Role,
  type AgentHistoryChunk,
  type ToolSourceField,
  type SourceFormat,
} from '../../plugin-sdk/src/index.js';
export type Schema = Record<string, unknown>;
export const object = (properties: Schema, required = Object.keys(properties)): Schema => ({
  type: 'object',
  properties,
  required,
  additionalProperties: false,
});
export const string = { type: 'string', maxLength: 512 };
export const locale = { type: 'string', minLength: 1, maxLength: 128 };
export interface Theme {
  background: number;
  surface: number;
  text: number;
  muted: number;
  accent: number;
  border: number;
  selected: number;
  textarea: TextareaAppearance;
  input: InputAppearance;
  menu: MenuAppearance;
  markdown: MarkdownAppearance;
}
export interface Message {
  id: string;
  // Immutable byte-segment identity; 1–123 ASCII letters, digits, underscores or hyphens.
  contentKey: string;
  role: Role;
  text: string;
  tool: AgentHistoryChunk['tool'];
  continued: boolean;
  source: number | null;
  image: boolean;
}
export interface ImageViewerProps {
  theme: Theme;
  locale: string;
  active: boolean;
  image: { sessionId: string; through: number; sequence: number } | null;
}
export interface SelectedModel {
  choice: SettingsModelChoice;
  thinkingLevel: ThinkingLevel | null;
}
export interface ModelsProps {
  catalogEpoch: number;
  theme: Theme;
  locale: string;
  selection: SelectedModel | null;
  busy: boolean;
}
export interface PaletteCommand {
  id: string;
  label: string;
  hint: string;
  disabled: boolean;
}
export interface PaletteProps {
  theme: Theme;
  locale: string;
  open: boolean;
  busy: boolean;
  selectionDisabled: boolean;
  commands: PaletteCommand[];
}
export interface HelpProps {
  theme: Theme;
  locale: string;
  open: boolean;
}
export interface DirectoryPickerProps {
  theme: Theme;
  locale: string;
  open: boolean;
  busy: boolean;
}
export interface SessionDetailsProps {
  theme: Theme;
  locale: string;
  open: boolean;
  busy: boolean;
  session: AgentSession | null;
}
export type QuestionDraft =
  | { kind: 'option'; index: number }
  | { kind: 'skip' }
  | { kind: 'text'; observation: EditorObservation };
export interface QuestionEditorProps {
  theme: Theme;
  locale: string;
  open: boolean;
  busy: boolean;
  checking: boolean;
  status: string;
  record: InteractionRecord;
}
export interface ModelDefaultProps {
  theme: Theme;
  locale: string;
  selection: SettingsModelChoice | null;
  defaultModel: SettingsDefaultModel | null;
  catalogRevision: number | null;
  busy: boolean;
}
export interface NavigationProps {
  theme: Theme;
  locale: string;
  sessions: AgentSession[];
  selected: string | null;
  busy: boolean;
}
export interface SourcePage {
  kind: 'message' | ToolSourceField;
  format: SourceFormat;
  ready: boolean;
  key: string;
  text: string;
  atStart: boolean;
  atEnd: boolean;
  loading: boolean;
  failed: boolean;
  navigation: number;
  landing: 'start' | 'end';
}
export interface TimelineProps {
  theme: Theme;
  locale: string;
  messages: Message[];
  source: SourcePage | null;
  loading: boolean;
  older: boolean;
  newer: boolean;
  latest: boolean;
  landing: { sequence: number; edge: 'start' | 'end' };
  live: boolean;
}
export interface ServiceApprovalProps {
  theme: Theme;
  locale: string;
  open: boolean;
  busy: boolean;
  checking: boolean;
  status: string;
  record: InteractionRecord;
}
export interface QueueEditorProps {
  theme: Theme;
  locale: string;
  open: boolean;
  ready: boolean;
  busy: boolean;
  uncertain: boolean;
  checking: boolean;
  status: string;
  command: EditorCommand | null;
}
export interface ComposerProps {
  theme: Theme;
  locale: string;
  busy: boolean;
  running: boolean;
  enabled: boolean;
  command: EditorCommand | null;
}
// Reuse the exact native schemas; this package defines only its own interfaces.
const editor = native.textarea.interface;
const session = native.agent.interface.methods.list.output;
const defs = {
  ...native.agent.interface.methods.directories.output.$defs,
  ...editor.props.$defs,
  ...editor.events.changed.$defs,
  ...native.markdown.interface.props.$defs,
  ...native.secretInput.interface.props.$defs,
  ...session.$defs,
  ...native.agent.interface.methods.history.output.$defs,
  ...native.agent.interface.methods.toolSource.output.$defs,
  ...native.agent.interface.methods.respond.output.$defs,
};
const themeSchema = object({
  background: { type: 'integer', minimum: 0, maximum: 4294967295 },
  surface: { type: 'integer', minimum: 0, maximum: 4294967295 },
  text: { type: 'integer', minimum: 0, maximum: 4294967295 },
  muted: { type: 'integer', minimum: 0, maximum: 4294967295 },
  accent: { type: 'integer', minimum: 0, maximum: 4294967295 },
  border: { type: 'integer', minimum: 0, maximum: 4294967295 },
  selected: { type: 'integer', minimum: 0, maximum: 4294967295 },
  textarea: editor.props.properties.appearance,
  input: native.secretInput.interface.props.properties.appearance,
  menu: { $ref: '#/$defs/MenuAppearance' },
  markdown: { $ref: '#/$defs/MarkdownAppearance' },
});
const messageSchema = object({
  id: string,
  contentKey: { type: 'string', minLength: 1, maxLength: 123, pattern: '^[A-Za-z0-9_-]+$' },
  role: { enum: ['user', 'assistant', 'tool_call', 'tool_result'] },
  text: { type: 'string', maxLength: 24000 },
  tool: { anyOf: [{ $ref: '#/$defs/HistoryTool' }, { type: 'null' }] },
  continued: { type: 'boolean' },
  image: { type: 'boolean' },
  source: { anyOf: [{ type: 'integer', minimum: 1 }, { type: 'null' }] },
});
const withDefs = (schema: Schema, additional: Schema = {}): Schema => {
  const { $defs: local, ...body } = schema;
  const available: Record<string, unknown> = {
    ...defs,
    Theme: themeSchema,
    Message: messageSchema,
    ...additional,
    ...(local as Record<string, unknown>),
  };
  const used: Record<string, unknown> = {};
  const visit = (value: unknown): void => {
    if (!value || typeof value !== 'object') return;
    if ('$ref' in value && typeof value.$ref === 'string' && value.$ref.startsWith('#/$defs/')) {
      const name = value.$ref.slice('#/$defs/'.length);
      if (!Object.hasOwn(used, name)) {
        if (!Object.hasOwn(available, name)) throw new Error('Missing schema definition: ' + name);
        used[name] = available[name];
        visit(available[name]);
      }
    }
    for (const child of Object.values(value)) visit(child);
  };
  visit(body);
  return Object.keys(used).length ? { ...body, $defs: used } : body;
};
const component = (
  name: string,
  props: Schema,
  events: Record<string, Schema> = {},
  captures: Extract<Contract['interface'], { kind: 'component' }>['captures'] = {},
): Contract => ({
  name,
  version: 1,
  transport: 'local',
  interface: { kind: 'component', props: withDefs(props), events, captures },
});
export const workspaceContract = component('maka.agent.Workspace', object({}));
export const paletteContract = component(
  'maka.agent.CommandPalette',
  object({
    theme: { $ref: '#/$defs/Theme' },
    locale,
    open: { type: 'boolean' },
    busy: { type: 'boolean' },
    selectionDisabled: { type: 'boolean' },
    commands: {
      type: 'array',
      maxItems: 128,
      items: object({
        id: { type: 'string', minLength: 1, maxLength: 120, pattern: '^[A-Za-z0-9:._-]+$' },
        label: { type: 'string', minLength: 1, maxLength: 256 },
        hint: { type: 'string', maxLength: 64 },
        disabled: { type: 'boolean' },
      }),
    },
  }),
  {
    invoked: object({
      id: { type: 'string', minLength: 1, maxLength: 120, pattern: '^[A-Za-z0-9:._-]+$' },
    }),
    selected: withDefs({ $ref: '#/$defs/AgentSession' }),
    closed: object({}),
  },
);
export const helpContract = component(
  'maka.agent.Help',
  object({ theme: { $ref: '#/$defs/Theme' }, locale, open: { type: 'boolean' } }),
  { closed: object({}) },
);
export const directoryPickerContract = component(
  'maka.agent.DirectoryPicker',
  object({
    theme: { $ref: '#/$defs/Theme' },
    locale,
    open: { type: 'boolean' },
    busy: { type: 'boolean' },
  }),
  { selected: withDefs({ $ref: '#/$defs/AgentDirectorySelection' }), closed: object({}) },
);
export const sessionDetailsContract = component(
  'maka.agent.SessionDetails',
  object({
    theme: { $ref: '#/$defs/Theme' },
    locale,
    open: { type: 'boolean' },
    busy: { type: 'boolean' },
    session: { anyOf: [{ $ref: '#/$defs/AgentSession' }, { type: 'null' }] },
  }),
  { changed: object({ sessionId: string }), closed: object({}) },
);
export const studioContract = component('maka.studio.Workbench', object({}));
export const navigationContract = component(
  'maka.agent.Navigation',
  object({
    theme: { $ref: '#/$defs/Theme' },
    locale,
    sessions: { type: 'array', maxItems: 32, items: { $ref: '#/$defs/AgentSession' } },
    selected: { anyOf: [string, { type: 'null' }] },
    busy: { type: 'boolean' },
  }),
  { select: object({ sessionId: string }) },
);
export const imageViewerContract = component(
  'maka.agent.ImageViewer',
  object({
    theme: { $ref: '#/$defs/Theme' },
    locale,
    active: { type: 'boolean' },
    image: {
      anyOf: [
        object({
          sessionId: string,
          through: { type: 'integer', minimum: 1 },
          sequence: { type: 'integer', minimum: 1 },
        }),
        { type: 'null' },
      ],
    },
  }),
  { closed: object({}) },
);
export const timelineContract = component(
  'maka.agent.Timeline',
  object({
    theme: { $ref: '#/$defs/Theme' },
    locale,
    messages: { type: 'array', maxItems: 32, items: { $ref: '#/$defs/Message' } },
    source: {
      anyOf: [
        object({
          kind: { anyOf: [{ const: 'message' }, { $ref: '#/$defs/ToolSourceField' }] },
          format: { $ref: '#/$defs/SourceFormat' },
          ready: { type: 'boolean' },
          key: string,
          text: { type: 'string', maxLength: 8192 },
          atStart: { type: 'boolean' },
          atEnd: { type: 'boolean' },
          loading: { type: 'boolean' },
          failed: { type: 'boolean' },
          navigation: { type: 'integer', minimum: 1 },
          landing: { enum: ['start', 'end'] },
        }),
        { type: 'null' },
      ],
    },
    loading: { type: 'boolean' },
    older: { type: 'boolean' },
    newer: { type: 'boolean' },
    latest: { type: 'boolean' },
    landing: object({
      sequence: { type: 'integer', minimum: 0 },
      edge: { enum: ['start', 'end'] },
    }),
    live: { type: 'boolean' },
  }),
  {
    older: object({}),
    newer: object({}),
    latest: object({}),
    source: object({ sequence: { type: 'integer', minimum: 1 } }),
    toolSource: object({ sequence: { type: 'integer', minimum: 1 } }),
    viewImage: object({ sequence: { type: 'integer', minimum: 1 } }),
    sourceFirst: object({}),
    sourcePrevious: object({}),
    sourceNext: object({}),
    sourceLast: object({}),
    sourceBack: object({}),
    sourceRetry: object({}),
    tailReady: object({
      landing: { type: 'integer', minimum: 0 },
      readingRevision: { type: 'integer', minimum: 0 },
    }),
  },
  native.readingArea.interface.captures,
);
export const queueEditorContract = component(
  'maka.agent.QueueEditor',
  object({
    theme: { $ref: '#/$defs/Theme' },
    locale,
    open: { type: 'boolean' },
    ready: { type: 'boolean' },
    busy: { type: 'boolean' },
    uncertain: { type: 'boolean' },
    checking: { type: 'boolean' },
    status: string,
    command: editor.props.properties.command,
  }),
  {
    save: editor.events.changed,
    changed: editor.events.changed,
    closed: object({}),
    reload: object({}),
    check: object({}),
  },
  editor.captures,
);
export const questionEditorContract = component(
  'maka.agent.QuestionEditor',
  object({
    theme: { $ref: '#/$defs/Theme' },
    locale,
    open: { type: 'boolean' },
    busy: { type: 'boolean' },
    checking: { type: 'boolean' },
    status: string,
    record: { $ref: '#/$defs/InteractionRecord' },
  }),
  {
    closed: object({}),
    check: object({}),
    submit: withDefs(
      object({
        requestId: string,
        answers: {
          type: 'array',
          minItems: 1,
          maxItems: 3,
          items: {
            oneOf: [
              object({
                kind: { const: 'option' },
                index: { type: 'integer', minimum: 0, maximum: 2 },
              }),
              object({ kind: { const: 'skip' } }),
              object({ kind: { const: 'text' }, observation: editor.events.changed }),
            ],
          },
        },
      }),
    ),
  },
  {
    snapshot: {
      ...editor.captures.snapshot,
      input: withDefs(
        object({
          requestId: string,
          index: { type: 'integer', minimum: 0, maximum: 2 },
          snapshot: editor.captures.snapshot.input,
        }),
      ),
    },
  },
);
export const serviceApprovalContract = component(
  'maka.agent.ServiceApproval',
  object({
    theme: { $ref: '#/$defs/Theme' },
    locale,
    open: { type: 'boolean' },
    busy: { type: 'boolean' },
    checking: { type: 'boolean' },
    status: string,
    record: { $ref: '#/$defs/InteractionRecord' },
  }),
  {
    closed: object({}),
    check: object({}),
    submit: object({ requestId: string, decision: { enum: ['allow', 'deny'] } }),
  },
);
export const permissionApprovalContract = component(
  'maka.agent.PermissionApproval',
  object({
    theme: { $ref: '#/$defs/Theme' },
    locale,
    open: { type: 'boolean' },
    busy: { type: 'boolean' },
    checking: { type: 'boolean' },
    status: string,
    record: { $ref: '#/$defs/InteractionRecord' },
  }),
  {
    closed: object({}),
    check: object({}),
    submit: object({
      requestId: string,
      decision: { enum: ['allow', 'deny'] },
      scope: { enum: ['once', 'turn', 'session'] },
    }),
  },
);
export const composerContract = component(
  'maka.agent.Composer',
  object({
    theme: { $ref: '#/$defs/Theme' },
    locale,
    busy: { type: 'boolean' },
    running: { type: 'boolean' },
    enabled: { type: 'boolean' },
    command: editor.props.properties.command,
  }),
  { send: editor.events.changed, changed: editor.events.changed, stop: object({}) },
  editor.captures,
);
export const themeContract: Contract = {
  name: 'maka.agent.Appearance',
  version: 1,
  transport: 'local',
  interface: { kind: 'value', schema: withDefs(themeSchema) },
};
export type ComposerObservation = EditorObservation;

const modelDefs = native.settings.interface.methods.models.output.$defs;
const selectedModel = object({
  choice: { $ref: '#/$defs/SettingsModelChoice' },
  thinkingLevel: { anyOf: [{ $ref: '#/$defs/ThinkingLevel' }, { type: 'null' }] },
});
export const modelsContract: Contract = {
  name: 'maka.agent.Models',
  version: 1,
  transport: 'local',
  interface: {
    kind: 'component',
    props: {
      ...withDefs(
        object({
          catalogEpoch: { type: 'integer', minimum: 0 },
          theme: { $ref: '#/$defs/Theme' },
          locale,
          selection: { anyOf: [selectedModel, { type: 'null' }] },
          busy: { type: 'boolean' },
        }),
        modelDefs,
      ),
    },
    events: { selected: { ...selectedModel, $defs: modelDefs } },
    captures: {},
  },
};

export const modelDefaultContract: Contract = {
  name: 'maka.agent.ModelDefault',
  version: 1,
  transport: 'local',
  interface: {
    kind: 'component',
    props: {
      ...withDefs(
        object({
          theme: { $ref: '#/$defs/Theme' },
          locale,
          selection: { anyOf: [{ $ref: '#/$defs/SettingsModelChoice' }, { type: 'null' }] },
          defaultModel: { anyOf: [{ $ref: '#/$defs/SettingsDefaultModel' }, { type: 'null' }] },
          catalogRevision: { anyOf: [{ type: 'integer', minimum: 0 }, { type: 'null' }] },
          busy: { type: 'boolean' },
        }),
        modelDefs,
      ),
    },
    events: { changed: object({}) },
    captures: {},
  },
};
export interface ConnectionsProps {
  theme: Theme;
  locale: string;
}
export interface CredentialProps extends ConnectionsProps {
  target: ConnectionCredentialTarget;
}
export interface ConnectionFormProps extends ConnectionsProps {
  catalogRevision: number;
  connection: Extract<SettingsConnectionDetail, { kind: 'ready' }> | null;
}
export const connectionFormContract: Contract = {
  name: 'maka.agent.ConnectionForm',
  version: 1,
  transport: 'local',
  interface: {
    kind: 'component',
    props: {
      ...object({
        theme: { $ref: '#/$defs/Theme' },
        locale,
        catalogRevision: { type: 'integer', minimum: 0 },
        connection: {
          anyOf: [native.settings.interface.methods.connection.output.oneOf[1], { type: 'null' }],
        },
      }),
      $defs: {
        ...defs,
        ...native.settings.interface.methods.connection.output.$defs,
        Theme: themeSchema,
      },
    },
    events: { closed: object({}), changed: object({}) },
    captures: {},
  },
};
export const credentialContract: Contract = {
  name: 'maka.agent.Credential',
  version: 1,
  transport: 'local',
  interface: {
    kind: 'component',
    props: {
      ...object({
        theme: { $ref: '#/$defs/Theme' },
        locale,
        target: { $ref: '#/$defs/ConnectionCredentialTarget' },
      }),
      $defs: {
        ...defs,
        ...native.settings.interface.methods.saveCredential.input.$defs,
        Theme: themeSchema,
      },
    },
    events: { closed: object({}), changed: object({}) },
    captures: {},
  },
};
export const connectionsContract = component(
  'maka.agent.Connections',
  object({ theme: { $ref: '#/$defs/Theme' }, locale }),
  { closed: object({}), changed: object({}) },
);

export const deploymentContract = component(
  'maka.agent.Deployment',
  object({ theme: { $ref: '#/$defs/Theme' }, locale }),
  { closed: object({}) },
);
