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
