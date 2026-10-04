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
          appearance: {
            $ref: '#/$defs/TextAppearance',
          },
        },
        additionalProperties: false,
        required: ['text', 'appearance'],
        $defs: {
          TextAppearance: {
            type: 'object',
            properties: {
              font: {
                $ref: '#/$defs/FontAppearance',
              },
              foreground: {
                $ref: '#/$defs/Color',
              },
            },
            additionalProperties: false,
            required: ['font', 'foreground'],
          },
          FontAppearance: {
            type: 'object',
            properties: {
              family: {
                $ref: '#/$defs/FontFamily',
              },
              fallbacks: {
                type: 'array',
                items: {
                  $ref: '#/$defs/FontFamily',
                },
                maxItems: 8,
              },
              size: {
                type: 'number',
                format: 'float',
                minimum: 6,
                maximum: 96,
              },
              lineHeight: {
                description: 'Line height relative to this font size.',
                type: 'number',
                format: 'float',
                minimum: 1,
                maximum: 3,
              },
              weight: {
                type: 'integer',
                format: 'uint16',
                minimum: 1,
                maximum: 1000,
              },
            },
            additionalProperties: false,
            required: ['family', 'fallbacks', 'size', 'lineHeight', 'weight'],
          },
          FontFamily: {
            type: 'string',
            minLength: 1,
            maxLength: 128,
          },
          Color: {
            description: 'Packed RGBA, including explicit alpha zero.',
            type: 'integer',
            format: 'uint32',
            minimum: 0,
          },
        },
      },
      events: {},
      captures: {},
    },
  },
  textarea: {
    name: 'maka.ui.Textarea',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'TextareaProps',
        type: 'object',
        properties: {
          appearance: {
            $ref: '#/$defs/TextareaAppearance',
          },
          menu: {
            anyOf: [
              {
                $ref: '#/$defs/TextareaMenu',
              },
              {
                type: 'null',
              },
            ],
            default: null,
          },
          accessibilityLabel: {
            type: 'string',
            minLength: 1,
            maxLength: 512,
          },
          readOnly: {
            type: 'boolean',
          },
          disabled: {
            type: 'boolean',
          },
          softWrap: {
            type: 'boolean',
          },
          tabIndex: {
            type: 'integer',
            format: 'int16',
            minimum: -1,
            maximum: 32767,
          },
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
                $ref: '#/$defs/TextareaCommand',
              },
              {
                type: 'null',
              },
            ],
            default: null,
          },
        },
        additionalProperties: false,
        required: [
          'appearance',
          'accessibilityLabel',
          'readOnly',
          'disabled',
          'softWrap',
          'tabIndex',
        ],
        $defs: {
          TextareaAppearance: {
            type: 'object',
            properties: {
              text: {
                $ref: '#/$defs/TextAppearance',
              },
              mutedForeground: {
                $ref: '#/$defs/Color',
              },
              background: {
                $ref: '#/$defs/Color',
              },
              selection: {
                $ref: '#/$defs/Color',
              },
              caret: {
                $ref: '#/$defs/Color',
              },
              border: {
                $ref: '#/$defs/Color',
              },
              focusedBorder: {
                $ref: '#/$defs/Color',
              },
              borderWidth: {
                type: 'number',
                format: 'float',
                minimum: 0,
                maximum: 8,
              },
              radius: {
                type: 'number',
                format: 'float',
                minimum: 0,
                maximum: 64,
              },
              padding: {
                $ref: '#/$defs/Insets',
              },
              scrollbar: {
                $ref: '#/$defs/ScrollbarAppearance',
              },
            },
            additionalProperties: false,
            required: [
              'text',
              'mutedForeground',
              'background',
              'selection',
              'caret',
              'border',
              'focusedBorder',
              'borderWidth',
              'radius',
              'padding',
              'scrollbar',
            ],
          },
          TextAppearance: {
            type: 'object',
            properties: {
              font: {
                $ref: '#/$defs/FontAppearance',
              },
              foreground: {
                $ref: '#/$defs/Color',
              },
            },
            additionalProperties: false,
            required: ['font', 'foreground'],
          },
          FontAppearance: {
            type: 'object',
            properties: {
              family: {
                $ref: '#/$defs/FontFamily',
              },
              fallbacks: {
                type: 'array',
                items: {
                  $ref: '#/$defs/FontFamily',
                },
                maxItems: 8,
              },
              size: {
                type: 'number',
                format: 'float',
                minimum: 6,
                maximum: 96,
              },
              lineHeight: {
                description: 'Line height relative to this font size.',
                type: 'number',
                format: 'float',
                minimum: 1,
                maximum: 3,
              },
              weight: {
                type: 'integer',
                format: 'uint16',
                minimum: 1,
                maximum: 1000,
              },
            },
            additionalProperties: false,
            required: ['family', 'fallbacks', 'size', 'lineHeight', 'weight'],
          },
          FontFamily: {
            type: 'string',
            minLength: 1,
            maxLength: 128,
          },
          Color: {
            description: 'Packed RGBA, including explicit alpha zero.',
            type: 'integer',
            format: 'uint32',
            minimum: 0,
          },
          Insets: {
            type: 'object',
            properties: {
              top: {
                type: 'number',
                format: 'float',
                minimum: 0,
                maximum: 128,
              },
              right: {
                type: 'number',
                format: 'float',
                minimum: 0,
                maximum: 128,
              },
              bottom: {
                type: 'number',
                format: 'float',
                minimum: 0,
                maximum: 128,
              },
              left: {
                type: 'number',
                format: 'float',
                minimum: 0,
                maximum: 128,
              },
            },
            additionalProperties: false,
            required: ['top', 'right', 'bottom', 'left'],
          },
          ScrollbarAppearance: {
            type: 'object',
            properties: {
              mode: {
                $ref: '#/$defs/ScrollbarMode',
              },
              entrance: {
                $ref: '#/$defs/ScrollbarEntrance',
              },
              idleMillis: {
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 60000,
              },
              enterMillis: {
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 1000,
              },
              exitMillis: {
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 1000,
              },
              expandMillis: {
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 1000,
              },
              trackWidth: {
                type: 'number',
                format: 'float',
                minimum: 1,
                maximum: 64,
              },
              track: {
                $ref: '#/$defs/Color',
              },
              trackHover: {
                $ref: '#/$defs/Color',
              },
              trackActive: {
                $ref: '#/$defs/Color',
              },
              thumb: {
                $ref: '#/$defs/ScrollbarThumb',
              },
              thumbHover: {
                $ref: '#/$defs/ScrollbarThumb',
              },
              thumbActive: {
                $ref: '#/$defs/ScrollbarThumb',
              },
            },
            additionalProperties: false,
            required: [
              'mode',
              'entrance',
              'idleMillis',
              'enterMillis',
              'exitMillis',
              'expandMillis',
              'trackWidth',
              'track',
              'trackHover',
              'trackActive',
              'thumb',
              'thumbHover',
              'thumbActive',
            ],
          },
          ScrollbarMode: {
            type: 'string',
            enum: ['scrolling', 'hover', 'always'],
          },
          ScrollbarEntrance: {
            type: 'string',
            enum: ['fade', 'slide_and_fade'],
          },
          ScrollbarThumb: {
            type: 'object',
            properties: {
              color: {
                $ref: '#/$defs/Color',
              },
              width: {
                type: 'number',
                format: 'float',
                minimum: 0,
                maximum: 64,
              },
              inset: {
                type: 'number',
                format: 'float',
                minimum: 0,
                maximum: 64,
              },
              radius: {
                type: 'number',
                format: 'float',
                minimum: 0,
                maximum: 32,
              },
              minLength: {
                type: 'number',
                format: 'float',
                minimum: 1,
                maximum: 4096,
              },
            },
            additionalProperties: false,
            required: ['color', 'width', 'inset', 'radius', 'minLength'],
          },
          TextareaMenu: {
            description:
              'Local presentation for user-initiated native editing actions. This is not\na command interface: clipboard access still requires native activation.',
            type: 'object',
            properties: {
              appearance: {
                $ref: '#/$defs/TextareaMenuAppearance',
              },
              labels: {
                $ref: '#/$defs/TextareaMenuLabels',
              },
            },
            additionalProperties: false,
            required: ['appearance', 'labels'],
          },
          TextareaMenuAppearance: {
            type: 'object',
            properties: {
              text: {
                $ref: '#/$defs/TextAppearance',
              },
              background: {
                $ref: '#/$defs/Color',
              },
              border: {
                $ref: '#/$defs/Color',
              },
              activeBackground: {
                $ref: '#/$defs/Color',
              },
              activeForeground: {
                $ref: '#/$defs/Color',
              },
              disabledForeground: {
                $ref: '#/$defs/Color',
              },
              borderWidth: {
                type: 'number',
                format: 'float',
                minimum: 0,
                maximum: 8,
              },
              radius: {
                type: 'number',
                format: 'float',
                minimum: 0,
                maximum: 64,
              },
              padding: {
                $ref: '#/$defs/Insets',
              },
              rowPadding: {
                $ref: '#/$defs/Insets',
              },
              width: {
                type: 'number',
                format: 'float',
                minimum: 32,
                maximum: 1024,
              },
              windowMargin: {
                type: 'number',
                format: 'float',
                minimum: 0,
                maximum: 128,
              },
            },
            additionalProperties: false,
            required: [
              'text',
              'background',
              'border',
              'activeBackground',
              'activeForeground',
              'disabledForeground',
              'borderWidth',
              'radius',
              'padding',
              'rowPadding',
              'width',
              'windowMargin',
            ],
          },
          TextareaMenuLabels: {
            type: 'object',
            properties: {
              title: {
                type: 'string',
                minLength: 1,
                maxLength: 512,
              },
              undo: {
                type: 'string',
                minLength: 1,
                maxLength: 512,
              },
              redo: {
                type: 'string',
                minLength: 1,
                maxLength: 512,
              },
              cut: {
                type: 'string',
                minLength: 1,
                maxLength: 512,
              },
              copy: {
                type: 'string',
                minLength: 1,
                maxLength: 512,
              },
              paste: {
                type: 'string',
                minLength: 1,
                maxLength: 512,
              },
              selectAll: {
                type: 'string',
                minLength: 1,
                maxLength: 512,
              },
            },
            additionalProperties: false,
            required: ['title', 'undo', 'redo', 'cut', 'copy', 'paste', 'selectAll'],
          },
          TextareaCommand: {
            type: 'object',
            properties: {
              owner: {
                description: 'Correlation with the original native editor, never a resource grant.',
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
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
                $ref: '#/$defs/TextareaEdit',
              },
            },
            additionalProperties: false,
            required: ['owner', 'sequence', 'basedOn', 'edit'],
          },
          TextareaEdit: {
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
          title: 'TextareaObservation',
          type: 'object',
          properties: {
            revision: {
              type: 'integer',
              format: 'uint64',
              minimum: 0,
            },
            snapshot: {
              $ref: '#/$defs/TextareaSnapshot',
            },
            receipt: {
              anyOf: [
                {
                  $ref: '#/$defs/TextareaReceipt',
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
            TextareaSnapshot: {
              type: 'object',
              properties: {
                text: {
                  $ref: '#/$defs/TextareaTextRef',
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
            TextareaTextRef: {
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
            TextareaReceipt: {
              type: 'object',
              properties: {
                owner: {
                  type: 'string',
                },
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
                  $ref: '#/$defs/TextareaOutcome',
                },
              },
              additionalProperties: false,
              required: ['owner', 'sequence', 'revision', 'outcome'],
            },
            TextareaOutcome: {
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
                      $ref: '#/$defs/TextareaConflict',
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
            TextareaConflict: {
              type: 'string',
              enum: ['owner', 'representation', 'revision', 'composition', 'sequence'],
            },
          },
        },
      },
      captures: {
        snapshot: {
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'TextareaCapture',
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
                  owner: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 128,
                  },
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
                required: ['kind', 'owner', 'revision'],
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
  agent: {
    name: 'maka.agent.Agent',
    version: 1,
    transport: 'remote',
    interface: {
      kind: 'service',
      methods: {
        create: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentCreate',
            type: 'object',
            properties: {
              title: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
            },
            additionalProperties: false,
            required: ['title'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentSession',
            type: 'object',
            properties: {
              id: {
                type: 'string',
              },
              title: {
                type: 'string',
              },
              revision: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
              },
              updatedAt: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
              },
              archived: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: ['id', 'title', 'revision', 'updatedAt', 'archived'],
          },
          resources: {},
        },
        history: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentHistoryRead',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              through: {
                type: ['integer', 'null'],
                format: 'uint64',
                minimum: 0,
                maximum: 9007199254740991,
              },
              position: {
                $ref: '#/$defs/AgentHistoryPosition',
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'position'],
            $defs: {
              AgentHistoryPosition: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'start',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    description:
                      'Inclusive event-log sequence. The service maps to its sparse transcript rows.',
                    type: 'object',
                    properties: {
                      sequence: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 1,
                        maximum: 9007199254740991,
                      },
                      kind: {
                        type: 'string',
                        const: 'event',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'sequence'],
                  },
                  {
                    type: 'object',
                    properties: {
                      cursor: {
                        $ref: '#/$defs/Cursor',
                      },
                      kind: {
                        type: 'string',
                        const: 'cursor',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'cursor'],
                  },
                ],
              },
              Cursor: {
                description: "Byte offset in one message's UTF-8 text, not a bearer capability.",
                type: 'object',
                properties: {
                  sequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['sequence', 'offset'],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentHistoryPage',
            oneOf: [
              {
                type: 'object',
                properties: {
                  through: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  kind: {
                    type: 'string',
                    const: 'preparing',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'through'],
              },
              {
                type: 'object',
                properties: {
                  through: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  chunks: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/AgentHistoryChunk',
                    },
                  },
                  next: {
                    anyOf: [
                      {
                        $ref: '#/$defs/Cursor',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  kind: {
                    type: 'string',
                    const: 'ready',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'through', 'chunks'],
              },
            ],
            $defs: {
              AgentHistoryChunk: {
                type: 'object',
                properties: {
                  messageId: {
                    type: 'string',
                  },
                  turnId: {
                    type: 'string',
                  },
                  timestamp: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  role: {
                    $ref: '#/$defs/Role',
                  },
                  sequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  totalBytes: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  text: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: [
                  'messageId',
                  'turnId',
                  'timestamp',
                  'role',
                  'sequence',
                  'offset',
                  'totalBytes',
                  'text',
                ],
              },
              Role: {
                description:
                  'Text visible in the durable conversation; reasoning and runtime metadata are\nnot conversation text. Tool JSON contributes string values, never its keys.',
                type: 'string',
                enum: ['user', 'assistant', 'tool_call', 'tool_result'],
              },
              Cursor: {
                description: "Byte offset in one message's UTF-8 text, not a bearer capability.",
                type: 'object',
                properties: {
                  sequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['sequence', 'offset'],
              },
            },
          },
          resources: {},
        },
        list: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentList',
            type: 'object',
            properties: {
              revision: {
                type: ['string', 'null'],
              },
              cursor: {
                type: ['string', 'null'],
                minLength: 1,
                maxLength: 128,
              },
            },
            additionalProperties: false,
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentSessionPage',
            type: 'object',
            properties: {
              revision: {
                type: 'string',
              },
              sessions: {
                type: 'array',
                items: {
                  $ref: '#/$defs/AgentSession',
                },
              },
              nextCursor: {
                type: ['string', 'null'],
              },
            },
            additionalProperties: false,
            required: ['revision', 'sessions'],
            $defs: {
              AgentSession: {
                type: 'object',
                properties: {
                  id: {
                    type: 'string',
                  },
                  title: {
                    type: 'string',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  updatedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  archived: {
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['id', 'title', 'revision', 'updatedAt', 'archived'],
              },
            },
          },
          resources: {},
        },
        observe: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentSessionRead',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
            },
            additionalProperties: false,
            required: ['sessionId'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentObservation',
            type: 'object',
            properties: {
              session: {
                $ref: '#/$defs/AgentSession',
              },
              through: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
              },
              run: {
                anyOf: [
                  {
                    $ref: '#/$defs/AgentRun',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
              parts: {
                type: 'array',
                items: {
                  $ref: '#/$defs/AgentPart',
                },
              },
            },
            additionalProperties: false,
            required: ['session', 'through', 'parts'],
            $defs: {
              AgentSession: {
                type: 'object',
                properties: {
                  id: {
                    type: 'string',
                  },
                  title: {
                    type: 'string',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  updatedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  archived: {
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['id', 'title', 'revision', 'updatedAt', 'archived'],
              },
              AgentRun: {
                type: 'object',
                properties: {
                  invocationId: {
                    type: 'string',
                  },
                  state: {
                    $ref: '#/$defs/AgentRunState',
                  },
                },
                additionalProperties: false,
                required: ['invocationId', 'state'],
              },
              AgentRunState: {
                type: 'string',
                enum: ['admitted', 'running', 'waiting_for_user'],
              },
              AgentPart: {
                type: 'object',
                properties: {
                  startSequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  messageId: {
                    type: 'string',
                  },
                  stepId: {
                    type: 'string',
                  },
                  partId: {
                    type: 'string',
                  },
                  textKind: {
                    $ref: '#/$defs/AgentContentKind',
                  },
                },
                additionalProperties: false,
                required: ['startSequence', 'messageId', 'stepId', 'partId', 'textKind'],
              },
              AgentContentKind: {
                type: 'string',
                enum: ['text', 'thinking'],
              },
            },
          },
          resources: {},
        },
        send: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentSend',
            type: 'object',
            properties: {
              model: {
                $ref: '#/$defs/ModelSelection',
              },
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              message: {
                type: 'string',
                minLength: 1,
                maxLength: 65536,
              },
            },
            additionalProperties: false,
            required: ['model', 'sessionId', 'message'],
            $defs: {
              ModelSelection: {
                description:
                  'Exact public selection observed by a caller, checked once at run admission.',
                type: 'object',
                properties: {
                  connectionId: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 128,
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 1,
                    maximum: 9007199254740991,
                  },
                  modelId: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 512,
                  },
                },
                additionalProperties: false,
                required: ['connectionId', 'revision', 'modelId'],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentStarted',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
              },
              invocationId: {
                type: 'string',
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'invocationId'],
          },
          resources: {},
        },
        stop: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentStop',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              invocationId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'invocationId'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentStopped',
            type: 'object',
            properties: {
              requested: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: ['requested'],
          },
          resources: {},
        },
        watch: {
          mode: 'subscription',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentWatch',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              after: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 9007199254740991,
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'after'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentDeliveryPage',
            type: 'object',
            properties: {
              through: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
              },
              nextAfter: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
              },
              events: {
                type: 'array',
                items: {
                  $ref: '#/$defs/AgentDelivery',
                },
              },
            },
            additionalProperties: false,
            required: ['through', 'nextAfter', 'events'],
            $defs: {
              AgentDelivery: {
                type: 'object',
                properties: {
                  sequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  invocationId: {
                    type: 'string',
                  },
                  fact: {
                    $ref: '#/$defs/AgentDeliveryFact',
                  },
                },
                additionalProperties: false,
                required: ['sequence', 'invocationId', 'fact'],
              },
              AgentDeliveryFact: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'opened',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    description:
                      'Canonical transcript facts changed; clients query bounded history.',
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'transcript_changed',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    properties: {
                      stepId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'step_ended',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'stepId'],
                  },
                  {
                    type: 'object',
                    properties: {
                      messageId: {
                        type: 'string',
                      },
                      stepId: {
                        type: 'string',
                      },
                      partId: {
                        type: 'string',
                      },
                      textKind: {
                        $ref: '#/$defs/AgentContentKind',
                      },
                      kind: {
                        type: 'string',
                        const: 'part_started',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'messageId', 'stepId', 'partId', 'textKind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      stepId: {
                        type: 'string',
                      },
                      partId: {
                        type: 'string',
                      },
                      text: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'delta',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'stepId', 'partId', 'text'],
                  },
                  {
                    type: 'object',
                    properties: {
                      stepId: {
                        type: 'string',
                      },
                      partId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'part_finished',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'stepId', 'partId'],
                  },
                  {
                    type: 'object',
                    properties: {
                      failed: {
                        type: 'boolean',
                      },
                      kind: {
                        type: 'string',
                        const: 'ended',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'failed'],
                  },
                ],
              },
              AgentContentKind: {
                type: 'string',
                enum: ['text', 'thinking'],
              },
            },
          },
          resources: {},
        },
      },
    },
  },
  settings: {
    name: 'maka.settings.Settings',
    version: 1,
    transport: 'remote',
    interface: {
      kind: 'service',
      methods: {
        models: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'SettingsModelsRead',
            oneOf: [
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'start',
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
                    maximum: 9007199254740991,
                  },
                  cursor: {
                    $ref: '#/$defs/SettingsModelCursor',
                  },
                  kind: {
                    type: 'string',
                    const: 'continue',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'revision', 'cursor'],
              },
            ],
            $defs: {
              SettingsModelCursor: {
                type: 'object',
                properties: {
                  connection: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                    maximum: 1024,
                  },
                  model: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                    maximum: 4096,
                  },
                },
                additionalProperties: false,
                required: ['connection', 'model'],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'SettingsModelsPage',
            oneOf: [
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
                    const: 'changed',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'revision'],
              },
              {
                type: 'object',
                properties: {
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  choices: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/SettingsModelChoice',
                    },
                    maxItems: 16,
                  },
                  next: {
                    anyOf: [
                      {
                        $ref: '#/$defs/SettingsModelCursor',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  kind: {
                    type: 'string',
                    const: 'ready',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'revision', 'choices', 'next'],
              },
            ],
            $defs: {
              SettingsModelChoice: {
                type: 'object',
                properties: {
                  selection: {
                    $ref: '#/$defs/ModelSelection',
                  },
                  connectionName: {
                    type: 'string',
                    maxLength: 256,
                  },
                  label: {
                    type: 'string',
                    maxLength: 512,
                  },
                  isDefault: {
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['selection', 'connectionName', 'label', 'isDefault'],
              },
              ModelSelection: {
                description:
                  'Exact public selection observed by a caller, checked once at run admission.',
                type: 'object',
                properties: {
                  connectionId: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 128,
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 1,
                    maximum: 9007199254740991,
                  },
                  modelId: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 512,
                  },
                },
                additionalProperties: false,
                required: ['connectionId', 'revision', 'modelId'],
              },
              SettingsModelCursor: {
                type: 'object',
                properties: {
                  connection: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                    maximum: 1024,
                  },
                  model: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                    maximum: 4096,
                  },
                },
                additionalProperties: false,
                required: ['connection', 'model'],
              },
            },
          },
          resources: {},
        },
        setDefault: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'SetDefaultConnectionTargetInput',
            type: 'object',
            properties: {
              expectedCatalogRevision: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
              },
              target: {
                anyOf: [
                  {
                    $ref: '#/$defs/ConnectionTarget',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
            },
            additionalProperties: false,
            required: ['expectedCatalogRevision', 'target'],
            $defs: {
              ConnectionTarget: {
                type: 'object',
                properties: {
                  connectionId: {
                    type: 'string',
                  },
                  modelId: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['connectionId', 'modelId'],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'CatalogMutationResult',
            oneOf: [
              {
                type: 'object',
                properties: {
                  catalogRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  connection: {
                    anyOf: [
                      {
                        $ref: '#/$defs/ConnectionVersionBasis',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  kind: {
                    type: 'string',
                    const: 'committed',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'catalogRevision'],
              },
              {
                type: 'object',
                properties: {
                  expectedRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  actualRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  kind: {
                    type: 'string',
                    const: 'revision_conflict',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'expectedRevision', 'actualRevision'],
              },
              {
                type: 'object',
                properties: {
                  slug: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'connection_exists',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'slug'],
              },
              {
                type: 'object',
                properties: {
                  expected: {
                    $ref: '#/$defs/ConnectionVersionBasis',
                  },
                  actual: {
                    anyOf: [
                      {
                        $ref: '#/$defs/ConnectionVersionBasis',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  kind: {
                    type: 'string',
                    const: 'connection_stale',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'expected', 'actual'],
              },
              {
                type: 'object',
                properties: {
                  target: {
                    $ref: '#/$defs/ConnectionTarget',
                  },
                  kind: {
                    type: 'string',
                    const: 'invalid_default_target',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'target'],
              },
            ],
            $defs: {
              ConnectionVersionBasis: {
                type: 'object',
                properties: {
                  connectionId: {
                    type: 'string',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['connectionId', 'revision'],
              },
              ConnectionTarget: {
                type: 'object',
                properties: {
                  connectionId: {
                    type: 'string',
                  },
                  modelId: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['connectionId', 'modelId'],
              },
            },
          },
          resources: {},
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
