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

// Generated from Rust runtime schemas. Run npm run schema:generate -w @maka-agent/plugin-sdk.
import type { Contract } from './generated/runtime.js';

export const contracts = {
  label: {
    name: 'maka.ui.Label',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'LabelProps',
        type: 'object',
        properties: {
          text: {
            type: 'string',
            maxLength: 4096,
          },
          emphasis: {
            type: 'boolean',
            default: false,
          },
        },
        additionalProperties: false,
        required: ['text'],
      },
      events: {},
      captures: {},
    },
  },
  editor: {
    name: 'maka.ui.Editor',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'EditorProps',
        type: 'object',
        properties: {
          initialText: {
            type: 'string',
            maxLength: 65536,
            default: '',
          },
          placeholder: {
            type: 'string',
            maxLength: 512,
            default: '',
          },
          command: {
            description:
              'None issues no new command; an accepted command remains pending until\nits matching receipt commits. Clearing props is not cancellation.',
            anyOf: [
              {
                $ref: '#/$defs/EditorCommand',
              },
              {
                type: 'null',
              },
            ],
            default: null,
          },
        },
        additionalProperties: false,
        $defs: {
          EditorCommand: {
            type: 'object',
            properties: {
              sequence: {
                type: 'integer',
                format: 'uint64',
                minimum: 1,
                maximum: 9007199254740991,
              },
              basedOn: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 9007199254740991,
              },
              edit: {
                $ref: '#/$defs/EditorEdit',
              },
            },
            additionalProperties: false,
            required: ['sequence', 'basedOn', 'edit'],
          },
          EditorEdit: {
            oneOf: [
              {
                type: 'object',
                properties: {
                  text: {
                    type: 'string',
                    maxLength: 8192,
                  },
                  kind: {
                    type: 'string',
                    const: 'replace',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'text'],
              },
              {
                type: 'object',
                properties: {
                  text: {
                    type: 'string',
                    maxLength: 8192,
                  },
                  kind: {
                    type: 'string',
                    const: 'reset',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'text'],
              },
            ],
          },
        },
      },
      events: {
        changed: {
          $schema: 'https://json-schema.org/draft/2020-12/schema',
          title: 'EditorObservation',
          type: 'object',
          properties: {
            revision: {
              type: 'integer',
              format: 'uint64',
              minimum: 0,
            },
            snapshot: {
              $ref: '#/$defs/EditorSnapshot',
            },
            receipt: {
              anyOf: [
                {
                  $ref: '#/$defs/EditorReceipt',
                },
                {
                  type: 'null',
                },
              ],
            },
          },
          additionalProperties: false,
          required: ['revision', 'snapshot'],
          $defs: {
            EditorSnapshot: {
              type: 'object',
              properties: {
                text: {
                  $ref: '#/$defs/EditorTextRef',
                },
                selection: {
                  $ref: '#/$defs/TextRange',
                },
                reversed: {
                  type: 'boolean',
                },
                marked: {
                  anyOf: [
                    {
                      $ref: '#/$defs/TextRange',
                    },
                    {
                      type: 'null',
                    },
                  ],
                },
              },
              additionalProperties: false,
              required: ['text', 'selection', 'reversed'],
            },
            EditorTextRef: {
              type: 'object',
              properties: {
                owner: {
                  type: 'string',
                },
                revision: {
                  type: 'integer',
                  format: 'uint64',
                  minimum: 0,
                },
                bytes: {
                  type: 'integer',
                  format: 'uint',
                  minimum: 0,
                },
              },
              additionalProperties: false,
              required: ['owner', 'revision', 'bytes'],
            },
            TextRange: {
              type: 'object',
              properties: {
                start: {
                  type: 'integer',
                  format: 'uint',
                  minimum: 0,
                },
                end: {
                  type: 'integer',
                  format: 'uint',
                  minimum: 0,
                },
              },
              additionalProperties: false,
              required: ['start', 'end'],
            },
            EditorReceipt: {
              type: 'object',
              properties: {
                sequence: {
                  type: 'integer',
                  format: 'uint64',
                  minimum: 0,
                },
                revision: {
                  type: 'integer',
                  format: 'uint64',
                  minimum: 0,
                },
                outcome: {
                  $ref: '#/$defs/EditorOutcome',
                },
              },
              additionalProperties: false,
              required: ['sequence', 'revision', 'outcome'],
            },
            EditorOutcome: {
              oneOf: [
                {
                  type: 'object',
                  properties: {
                    status: {
                      type: 'string',
                      const: 'applied',
                    },
                  },
                  required: ['status'],
                  additionalProperties: false,
                },
                {
                  type: 'object',
                  properties: {
                    reason: {
                      $ref: '#/$defs/EditorConflict',
                    },
                    status: {
                      type: 'string',
                      const: 'conflict',
                    },
                  },
                  additionalProperties: false,
                  required: ['status', 'reason'],
                },
              ],
            },
            EditorConflict: {
              type: 'string',
              enum: ['revision', 'composition', 'sequence'],
            },
          },
        },
      },
      captures: {
        snapshot: {
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'EditorCapture',
            oneOf: [
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'current',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
              {
                type: 'object',
                properties: {
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  kind: {
                    type: 'string',
                    const: 'at_revision',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'revision'],
              },
            ],
          },
          resource: {
            name: 'maka.text.utf8',
            version: 1,
          },
        },
      },
    },
  },
  textReader: {
    name: 'maka.text.Reader',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'service',
      methods: {
        read: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'ReadText',
            type: 'object',
            properties: {
              start: {
                type: 'integer',
                format: 'uint',
                minimum: 0,
              },
              end: {
                type: 'integer',
                format: 'uint',
                minimum: 0,
              },
            },
            additionalProperties: false,
            required: ['start', 'end'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'TextChunk',
            type: 'object',
            properties: {
              text: {
                type: 'string',
              },
              metadata: true,
            },
            additionalProperties: false,
            required: ['text', 'metadata'],
          },
          resources: {
            text: {
              name: 'maka.text.utf8',
              version: 1,
            },
          },
        },
      },
    },
  },
  callerHistory: {
    name: 'maka.operations.CallerHistory',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'service',
      methods: {
        page: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'CallerHistoryPage',
            type: 'object',
            properties: {
              after: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 9007199254740991,
              },
              limit: {
                type: 'integer',
                format: 'uint',
                minimum: 1,
                maximum: 128,
              },
            },
            additionalProperties: false,
            required: ['after', 'limit'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'Array_of_CallerIntent',
            type: 'array',
            items: {
              $ref: '#/$defs/CallerIntent',
            },
            $defs: {
              CallerIntent: {
                type: 'object',
                properties: {
                  cursor: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  caller: {
                    $ref: '#/$defs/DeploymentRef',
                  },
                  consumer: {
                    $ref: '#/$defs/InstanceId',
                  },
                  reference: {
                    $ref: '#/$defs/RecoveryRef',
                  },
                },
                additionalProperties: false,
                required: ['cursor', 'caller', 'consumer', 'reference'],
              },
              DeploymentRef: {
                type: 'object',
                properties: {
                  stateDomain: {
                    type: 'string',
                  },
                  deployment: {
                    type: 'string',
                    format: 'uuid',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['stateDomain', 'deployment', 'revision'],
              },
              InstanceId: {
                description:
                  'Hierarchical, stable source keys. Display aliases never become data identities.',
                type: 'array',
                items: {
                  type: 'string',
                  format: 'uuid',
                },
              },
              RecoveryRef: {
                type: 'object',
                properties: {
                  source: {
                    $ref: '#/$defs/DeploymentRef',
                  },
                  namespace: {
                    $ref: '#/$defs/InstanceId',
                  },
                  export: {
                    type: 'string',
                  },
                  method: {
                    type: 'string',
                  },
                  operation: {
                    type: 'string',
                    format: 'uuid',
                  },
                  fingerprint: {
                    type: 'string',
                  },
                  contract: {
                    $ref: '#/$defs/Contract',
                  },
                  recovery: {
                    $ref: '#/$defs/RecoveryProtocol',
                  },
                },
                additionalProperties: false,
                required: [
                  'source',
                  'namespace',
                  'export',
                  'method',
                  'operation',
                  'fingerprint',
                  'contract',
                  'recovery',
                ],
              },
              Contract: {
                description:
                  'Public contract definitions are package data, not live registration records.',
                type: 'object',
                properties: {
                  name: {
                    type: 'string',
                  },
                  version: {
                    type: 'integer',
                    format: 'uint32',
                    minimum: 0,
                  },
                  transport: {
                    $ref: '#/$defs/Transport',
                  },
                  interface: {
                    $ref: '#/$defs/Interface',
                  },
                },
                additionalProperties: false,
                required: ['name', 'version', 'transport', 'interface'],
              },
              Transport: {
                type: 'string',
                enum: ['local', 'remote'],
              },
              Interface: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      methods: {
                        type: 'object',
                        additionalProperties: {
                          $ref: '#/$defs/Method',
                        },
                      },
                      kind: {
                        type: 'string',
                        const: 'service',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'methods'],
                  },
                  {
                    type: 'object',
                    properties: {
                      props: true,
                      events: {
                        type: 'object',
                        additionalProperties: true,
                      },
                      captures: {
                        type: 'object',
                        additionalProperties: {
                          $ref: '#/$defs/Capture',
                        },
                      },
                      kind: {
                        type: 'string',
                        const: 'component',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'props', 'events', 'captures'],
                  },
                  {
                    type: 'object',
                    properties: {
                      schema: true,
                      kind: {
                        type: 'string',
                        const: 'value',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'schema'],
                  },
                  {
                    type: 'object',
                    properties: {
                      resource: {
                        type: 'string',
                      },
                      schema: true,
                      kind: {
                        type: 'string',
                        const: 'resource',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'resource', 'schema'],
                  },
                ],
              },
              Method: {
                type: 'object',
                properties: {
                  mode: {
                    $ref: '#/$defs/MethodMode',
                  },
                  input: true,
                  output: true,
                  resources: {
                    type: 'object',
                    additionalProperties: {
                      $ref: '#/$defs/ResourceType',
                    },
                  },
                },
                additionalProperties: false,
                required: ['mode', 'input', 'output', 'resources'],
              },
              MethodMode: {
                type: 'string',
                enum: ['query', 'mutation', 'subscription'],
              },
              ResourceType: {
                description:
                  'Nominal read-only resource shape, not a resource id or authorization token.',
                type: 'object',
                properties: {
                  name: {
                    type: 'string',
                  },
                  version: {
                    type: 'integer',
                    format: 'uint32',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['name', 'version'],
              },
              Capture: {
                type: 'object',
                properties: {
                  input: true,
                  resource: {
                    $ref: '#/$defs/ResourceType',
                  },
                },
                additionalProperties: false,
                required: ['input', 'resource'],
              },
              RecoveryProtocol: {
                type: 'string',
                enum: ['journal_v1'],
              },
            },
          },
          resources: {},
        },
      },
    },
  },
  receiptReader: {
    name: 'maka.operations.ReceiptReader',
    version: 1,
    transport: 'remote',
    interface: {
      kind: 'service',
      methods: {
        read: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'RecoveryRef',
            type: 'object',
            properties: {
              source: {
                $ref: '#/$defs/DeploymentRef',
              },
              namespace: {
                $ref: '#/$defs/InstanceId',
              },
              export: {
                type: 'string',
              },
              method: {
                type: 'string',
              },
              operation: {
                type: 'string',
                format: 'uuid',
              },
              fingerprint: {
                type: 'string',
              },
              contract: {
                $ref: '#/$defs/Contract',
              },
              recovery: {
                $ref: '#/$defs/RecoveryProtocol',
              },
            },
            additionalProperties: false,
            required: [
              'source',
              'namespace',
              'export',
              'method',
              'operation',
              'fingerprint',
              'contract',
              'recovery',
            ],
            $defs: {
              DeploymentRef: {
                type: 'object',
                properties: {
                  stateDomain: {
                    type: 'string',
                  },
                  deployment: {
                    type: 'string',
                    format: 'uuid',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['stateDomain', 'deployment', 'revision'],
              },
              InstanceId: {
                description:
                  'Hierarchical, stable source keys. Display aliases never become data identities.',
                type: 'array',
                items: {
                  type: 'string',
                  format: 'uuid',
                },
              },
              Contract: {
                description:
                  'Public contract definitions are package data, not live registration records.',
                type: 'object',
                properties: {
                  name: {
                    type: 'string',
                  },
                  version: {
                    type: 'integer',
                    format: 'uint32',
                    minimum: 0,
                  },
                  transport: {
                    $ref: '#/$defs/Transport',
                  },
                  interface: {
                    $ref: '#/$defs/Interface',
                  },
                },
                additionalProperties: false,
                required: ['name', 'version', 'transport', 'interface'],
              },
              Transport: {
                type: 'string',
                enum: ['local', 'remote'],
              },
              Interface: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      methods: {
                        type: 'object',
                        additionalProperties: {
                          $ref: '#/$defs/Method',
                        },
                      },
                      kind: {
                        type: 'string',
                        const: 'service',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'methods'],
                  },
                  {
                    type: 'object',
                    properties: {
                      props: true,
                      events: {
                        type: 'object',
                        additionalProperties: true,
                      },
                      captures: {
                        type: 'object',
                        additionalProperties: {
                          $ref: '#/$defs/Capture',
                        },
                      },
                      kind: {
                        type: 'string',
                        const: 'component',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'props', 'events', 'captures'],
                  },
                  {
                    type: 'object',
                    properties: {
                      schema: true,
                      kind: {
                        type: 'string',
                        const: 'value',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'schema'],
                  },
                  {
                    type: 'object',
                    properties: {
                      resource: {
                        type: 'string',
                      },
                      schema: true,
                      kind: {
                        type: 'string',
                        const: 'resource',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'resource', 'schema'],
                  },
                ],
              },
              Method: {
                type: 'object',
                properties: {
                  mode: {
                    $ref: '#/$defs/MethodMode',
                  },
                  input: true,
                  output: true,
                  resources: {
                    type: 'object',
                    additionalProperties: {
                      $ref: '#/$defs/ResourceType',
                    },
                  },
                },
                additionalProperties: false,
                required: ['mode', 'input', 'output', 'resources'],
              },
              MethodMode: {
                type: 'string',
                enum: ['query', 'mutation', 'subscription'],
              },
              ResourceType: {
                description:
                  'Nominal read-only resource shape, not a resource id or authorization token.',
                type: 'object',
                properties: {
                  name: {
                    type: 'string',
                  },
                  version: {
                    type: 'integer',
                    format: 'uint32',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['name', 'version'],
              },
              Capture: {
                type: 'object',
                properties: {
                  input: true,
                  resource: {
                    $ref: '#/$defs/ResourceType',
                  },
                },
                additionalProperties: false,
                required: ['input', 'resource'],
              },
              RecoveryProtocol: {
                type: 'string',
                enum: ['journal_v1'],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'OperationReply',
            type: 'object',
            properties: {
              reference: {
                $ref: '#/$defs/RecoveryRef',
              },
              receipt: {
                $ref: '#/$defs/Receipt',
              },
            },
            additionalProperties: false,
            required: ['reference', 'receipt'],
            $defs: {
              RecoveryRef: {
                type: 'object',
                properties: {
                  source: {
                    $ref: '#/$defs/DeploymentRef',
                  },
                  namespace: {
                    $ref: '#/$defs/InstanceId',
                  },
                  export: {
                    type: 'string',
                  },
                  method: {
                    type: 'string',
                  },
                  operation: {
                    type: 'string',
                    format: 'uuid',
                  },
                  fingerprint: {
                    type: 'string',
                  },
                  contract: {
                    $ref: '#/$defs/Contract',
                  },
                  recovery: {
                    $ref: '#/$defs/RecoveryProtocol',
                  },
                },
                additionalProperties: false,
                required: [
                  'source',
                  'namespace',
                  'export',
                  'method',
                  'operation',
                  'fingerprint',
                  'contract',
                  'recovery',
                ],
              },
              DeploymentRef: {
                type: 'object',
                properties: {
                  stateDomain: {
                    type: 'string',
                  },
                  deployment: {
                    type: 'string',
                    format: 'uuid',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['stateDomain', 'deployment', 'revision'],
              },
              InstanceId: {
                description:
                  'Hierarchical, stable source keys. Display aliases never become data identities.',
                type: 'array',
                items: {
                  type: 'string',
                  format: 'uuid',
                },
              },
              Contract: {
                description:
                  'Public contract definitions are package data, not live registration records.',
                type: 'object',
                properties: {
                  name: {
                    type: 'string',
                  },
                  version: {
                    type: 'integer',
                    format: 'uint32',
                    minimum: 0,
                  },
                  transport: {
                    $ref: '#/$defs/Transport',
                  },
                  interface: {
                    $ref: '#/$defs/Interface',
                  },
                },
                additionalProperties: false,
                required: ['name', 'version', 'transport', 'interface'],
              },
              Transport: {
                type: 'string',
                enum: ['local', 'remote'],
              },
              Interface: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      methods: {
                        type: 'object',
                        additionalProperties: {
                          $ref: '#/$defs/Method',
                        },
                      },
                      kind: {
                        type: 'string',
                        const: 'service',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'methods'],
                  },
                  {
                    type: 'object',
                    properties: {
                      props: true,
                      events: {
                        type: 'object',
                        additionalProperties: true,
                      },
                      captures: {
                        type: 'object',
                        additionalProperties: {
                          $ref: '#/$defs/Capture',
                        },
                      },
                      kind: {
                        type: 'string',
                        const: 'component',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'props', 'events', 'captures'],
                  },
                  {
                    type: 'object',
                    properties: {
                      schema: true,
                      kind: {
                        type: 'string',
                        const: 'value',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'schema'],
                  },
                  {
                    type: 'object',
                    properties: {
                      resource: {
                        type: 'string',
                      },
                      schema: true,
                      kind: {
                        type: 'string',
                        const: 'resource',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'resource', 'schema'],
                  },
                ],
              },
              Method: {
                type: 'object',
                properties: {
                  mode: {
                    $ref: '#/$defs/MethodMode',
                  },
                  input: true,
                  output: true,
                  resources: {
                    type: 'object',
                    additionalProperties: {
                      $ref: '#/$defs/ResourceType',
                    },
                  },
                },
                additionalProperties: false,
                required: ['mode', 'input', 'output', 'resources'],
              },
              MethodMode: {
                type: 'string',
                enum: ['query', 'mutation', 'subscription'],
              },
              ResourceType: {
                description:
                  'Nominal read-only resource shape, not a resource id or authorization token.',
                type: 'object',
                properties: {
                  name: {
                    type: 'string',
                  },
                  version: {
                    type: 'integer',
                    format: 'uint32',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['name', 'version'],
              },
              Capture: {
                type: 'object',
                properties: {
                  input: true,
                  resource: {
                    $ref: '#/$defs/ResourceType',
                  },
                },
                additionalProperties: false,
                required: ['input', 'resource'],
              },
              RecoveryProtocol: {
                type: 'string',
                enum: ['journal_v1'],
              },
              Receipt: {
                description:
                  "A caller's receipt observation, never an execution grant. Unavailable makes\nno claim about the journal's contents or physical cleanup.",
                oneOf: [
                  {
                    description:
                      'An authorized lookup found no row at the exact original reference.',
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'not_recorded',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    properties: {
                      outcome: {
                        $ref: '#/$defs/OperationOutcome',
                      },
                      cleanup: {
                        $ref: '#/$defs/OperationCleanup',
                      },
                      kind: {
                        type: 'string',
                        const: 'recorded',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'outcome', 'cleanup'],
                  },
                  {
                    type: 'object',
                    properties: {
                      diagnostic: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'unavailable',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'diagnostic'],
                  },
                ],
              },
              OperationOutcome: {
                description:
                  'Unresolved means no durable result is known, including after process loss.',
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'unresolved',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    properties: {
                      value: true,
                      kind: {
                        type: 'string',
                        const: 'returned',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'value'],
                  },
                  {
                    type: 'object',
                    properties: {
                      diagnostic: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'unknown',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'diagnostic'],
                  },
                ],
              },
              OperationCleanup: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'unconfirmed',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'confirmed',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    properties: {
                      diagnostic: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'failed',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'diagnostic'],
                  },
                ],
              },
            },
          },
          resources: {},
        },
      },
    },
  },
  desktopSurface: {
    name: 'maka.ui.surface.desktop',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'resource',
      resource: 'surface.desktop',
      schema: {
        type: 'object',
        additionalProperties: false,
      },
    },
  },
  terminalSurface: {
    name: 'maka.ui.surface.tui',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'resource',
      resource: 'surface.tui',
      schema: {
        type: 'object',
        additionalProperties: false,
      },
    },
  },
} satisfies Record<string, Contract>;
