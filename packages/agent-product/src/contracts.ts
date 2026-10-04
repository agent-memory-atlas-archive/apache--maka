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
  type SettingsModelChoice,
  type TextareaAppearance,
  type TextareaMenuAppearance,
  type TextareaCommand,
  type TextareaObservation,
  type Role,
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
  menu: TextareaMenuAppearance;
}
export interface Message {
  id: string;
  role: Role;
  text: string;
  continued: boolean;
}
export interface ModelsProps {
  theme: Theme;
  locale: string;
  selection: SettingsModelChoice | null;
  busy: boolean;
}
export interface NavigationProps {
  theme: Theme;
  locale: string;
  sessions: AgentSession[];
  selected: string | null;
  busy: boolean;
  more: boolean;
}
export interface TimelineProps {
  theme: Theme;
  locale: string;
  messages: Message[];
  loading: boolean;
  more: boolean;
  live: boolean;
}
export interface ComposerProps {
  theme: Theme;
  locale: string;
  busy: boolean;
  running: boolean;
  enabled: boolean;
  command: TextareaCommand | null;
}
// Reuse the exact native schemas; this package defines only its own interfaces.
const editor = native.textarea.interface;
const session = native.agent.interface.methods.list.output;
const defs = { ...editor.props.$defs, ...session.$defs };
const themeSchema = object({
  background: { type: 'integer', minimum: 0, maximum: 4294967295 },
  surface: { type: 'integer', minimum: 0, maximum: 4294967295 },
  text: { type: 'integer', minimum: 0, maximum: 4294967295 },
  muted: { type: 'integer', minimum: 0, maximum: 4294967295 },
  accent: { type: 'integer', minimum: 0, maximum: 4294967295 },
  border: { type: 'integer', minimum: 0, maximum: 4294967295 },
  selected: { type: 'integer', minimum: 0, maximum: 4294967295 },
  textarea: editor.props.properties.appearance,
  menu: { $ref: '#/$defs/TextareaMenuAppearance' },
});
const messageSchema = object({
  id: string,
  role: { enum: ['user', 'assistant', 'tool_call', 'tool_result'] },
  text: { type: 'string', maxLength: 24000 },
  continued: { type: 'boolean' },
});
const withDefs = (schema: Schema) => ({
  ...schema,
  $defs: { ...defs, Theme: themeSchema, Message: messageSchema },
});
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
export const navigationContract = component(
  'maka.agent.Navigation',
  object({
    theme: { $ref: '#/$defs/Theme' },
    locale,
    sessions: { type: 'array', maxItems: 128, items: { $ref: '#/$defs/AgentSession' } },
    selected: { anyOf: [string, { type: 'null' }] },
    busy: { type: 'boolean' },
    more: { type: 'boolean' },
  }),
  { create: object({}), select: object({ sessionId: string }), more: object({}) },
);
export const timelineContract = component(
  'maka.agent.Timeline',
  object({
    theme: { $ref: '#/$defs/Theme' },
    locale,
    messages: { type: 'array', maxItems: 32, items: { $ref: '#/$defs/Message' } },
    loading: { type: 'boolean' },
    more: { type: 'boolean' },
    live: { type: 'boolean' },
  }),
  { more: object({}) },
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
export type ComposerObservation = TextareaObservation;

const modelChoice = native.settings.interface.methods.models.output.$defs.SettingsModelChoice;
const modelDefs = native.settings.interface.methods.models.output.$defs;
export const modelsContract: Contract = {
  name: 'maka.agent.Models',
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
          busy: { type: 'boolean' },
        }),
      ),
      $defs: { ...defs, Theme: themeSchema, ...modelDefs },
    },
    events: { selected: { ...modelChoice, $defs: modelDefs } },
    captures: {},
  },
};
