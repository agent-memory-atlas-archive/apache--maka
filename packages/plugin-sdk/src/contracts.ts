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
          EditorCommand: {
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
                $ref: '#/$defs/EditorEdit',
              },
            },
            additionalProperties: false,
            required: ['owner', 'sequence', 'basedOn', 'edit'],
          },
          EditorEdit: {
            oneOf: [
              {
                type: 'object',
                properties: {
                  text: {
                    type: 'string',
                    maxLength: 65536,
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
                    maxLength: 65536,
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
          required: ['revision', 'snapshot', 'receipt'],
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
              required: ['text', 'selection', 'reversed', 'marked'],
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
                  $ref: '#/$defs/EditorOutcome',
                },
              },
              additionalProperties: false,
              required: ['owner', 'sequence', 'revision', 'outcome'],
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
              enum: ['owner', 'representation', 'revision', 'composition', 'sequence'],
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
  textInput: {
    name: 'maka.ui.TextInput',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'InputProps',
        type: 'object',
        properties: {
          appearance: {
            $ref: '#/$defs/InputAppearance',
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
          tabIndex: {
            type: 'integer',
            format: 'int16',
            minimum: -1,
            maximum: 32767,
          },
          placeholder: {
            type: 'string',
            maxLength: 512,
            default: '',
          },
          command: {
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
        required: ['appearance', 'accessibilityLabel', 'readOnly', 'disabled', 'tabIndex'],
        $defs: {
          InputAppearance: {
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
          EditorCommand: {
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
                $ref: '#/$defs/EditorEdit',
              },
            },
            additionalProperties: false,
            required: ['owner', 'sequence', 'basedOn', 'edit'],
          },
          EditorEdit: {
            oneOf: [
              {
                type: 'object',
                properties: {
                  text: {
                    type: 'string',
                    maxLength: 65536,
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
                    maxLength: 65536,
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
          required: ['revision', 'snapshot', 'receipt'],
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
              required: ['text', 'selection', 'reversed', 'marked'],
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
                  $ref: '#/$defs/EditorOutcome',
                },
              },
              additionalProperties: false,
              required: ['owner', 'sequence', 'revision', 'outcome'],
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
              enum: ['owner', 'representation', 'revision', 'composition', 'sequence'],
            },
          },
        },
        submitted: {
          type: 'object',
          additionalProperties: false,
          properties: {},
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
  secretInput: {
    name: 'maka.ui.SecretInput',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'InputProps',
        type: 'object',
        properties: {
          appearance: {
            $ref: '#/$defs/InputAppearance',
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
          tabIndex: {
            type: 'integer',
            format: 'int16',
            minimum: -1,
            maximum: 32767,
          },
          placeholder: {
            type: 'string',
            maxLength: 512,
            default: '',
          },
          command: {
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
        required: ['appearance', 'accessibilityLabel', 'readOnly', 'disabled', 'tabIndex'],
        $defs: {
          InputAppearance: {
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
          EditorCommand: {
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
                $ref: '#/$defs/EditorEdit',
              },
            },
            additionalProperties: false,
            required: ['owner', 'sequence', 'basedOn', 'edit'],
          },
          EditorEdit: {
            oneOf: [
              {
                type: 'object',
                properties: {
                  text: {
                    type: 'string',
                    maxLength: 65536,
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
                    maxLength: 65536,
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
          required: ['revision', 'snapshot', 'receipt'],
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
              required: ['text', 'selection', 'reversed', 'marked'],
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
                  $ref: '#/$defs/EditorOutcome',
                },
              },
              additionalProperties: false,
              required: ['owner', 'sequence', 'revision', 'outcome'],
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
              enum: ['owner', 'representation', 'revision', 'composition', 'sequence'],
            },
          },
        },
        submitted: {
          type: 'object',
          additionalProperties: false,
          properties: {},
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
        connection: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'ConnectionVersionBasis',
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
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'SettingsConnectionDetail',
            oneOf: [
              {
                type: 'object',
                properties: {
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
                    const: 'stale',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'actual'],
              },
              {
                type: 'object',
                properties: {
                  target: {
                    $ref: '#/$defs/ConnectionCredentialTarget',
                  },
                  name: {
                    type: 'string',
                  },
                  enabled: {
                    type: 'boolean',
                  },
                  enabledModelCount: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                  },
                  kind: {
                    type: 'string',
                    const: 'ready',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'target', 'name', 'enabled', 'enabledModelCount'],
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
              ConnectionCredentialTarget: {
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
                  slug: {
                    type: 'string',
                  },
                  provider: {
                    $ref: '#/$defs/Identity',
                  },
                  configuration: {
                    type: 'object',
                  },
                },
                additionalProperties: false,
                required: ['connectionId', 'revision', 'slug', 'provider', 'configuration'],
              },
              Identity: {
                description:
                  'Durable recipient, not a registration lease. Re-loading the same package can\nrestore it; another package, Entry or scope cannot inherit its credentials.',
                type: 'object',
                properties: {
                  packageId: {
                    type: 'string',
                  },
                  entryId: {
                    type: 'string',
                  },
                  scope: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['packageId', 'entryId', 'scope', 'name'],
              },
            },
          },
          resources: {},
        },
        connections: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'SettingsConnectionsRead',
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
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                    maximum: 1024,
                  },
                  kind: {
                    type: 'string',
                    const: 'continue',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'revision', 'offset'],
              },
            ],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'SettingsConnectionsPage',
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
                  defaultTarget: {
                    anyOf: [
                      {
                        $ref: '#/$defs/ConnectionTarget',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  connections: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/SettingsConnectionSummary',
                    },
                    maxItems: 16,
                  },
                  next: {
                    type: ['integer', 'null'],
                    format: 'uint',
                    minimum: 0,
                  },
                  kind: {
                    type: 'string',
                    const: 'ready',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'revision', 'defaultTarget', 'connections', 'next'],
              },
            ],
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
              SettingsConnectionSummary: {
                type: 'object',
                properties: {
                  basis: {
                    $ref: '#/$defs/ConnectionVersionBasis',
                  },
                  slug: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                  provider: {
                    $ref: '#/$defs/Identity',
                  },
                  enabled: {
                    type: 'boolean',
                  },
                  enabledModelCount: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['basis', 'slug', 'name', 'provider', 'enabled', 'enabledModelCount'],
              },
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
              Identity: {
                description:
                  'Durable recipient, not a registration lease. Re-loading the same package can\nrestore it; another package, Entry or scope cannot inherit its credentials.',
                type: 'object',
                properties: {
                  packageId: {
                    type: 'string',
                  },
                  entryId: {
                    type: 'string',
                  },
                  scope: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['packageId', 'entryId', 'scope', 'name'],
              },
            },
          },
          resources: {},
        },
        create: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'CreateCatalogConnectionInput',
            type: 'object',
            properties: {
              expectedCatalogRevision: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
              },
              connection: {
                $ref: '#/$defs/ConnectionCatalogEntryDraft',
              },
            },
            additionalProperties: false,
            required: ['expectedCatalogRevision', 'connection'],
            $defs: {
              ConnectionCatalogEntryDraft: {
                type: 'object',
                properties: {
                  slug: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                  provider: {
                    $ref: '#/$defs/Identity',
                  },
                  configuration: {
                    type: 'object',
                  },
                  enabled: {
                    type: 'boolean',
                  },
                  enabledModelIds: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                  modelOverrides: {
                    type: 'object',
                    additionalProperties: {
                      $ref: '#/$defs/ModelOverride',
                    },
                  },
                  requestBodyOverlay: {
                    type: 'object',
                  },
                },
                additionalProperties: false,
                required: [
                  'slug',
                  'name',
                  'provider',
                  'configuration',
                  'enabled',
                  'enabledModelIds',
                ],
              },
              Identity: {
                description:
                  'Durable recipient, not a registration lease. Re-loading the same package can\nrestore it; another package, Entry or scope cannot inherit its credentials.',
                type: 'object',
                properties: {
                  packageId: {
                    type: 'string',
                  },
                  entryId: {
                    type: 'string',
                  },
                  scope: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['packageId', 'entryId', 'scope', 'name'],
              },
              ModelOverride: {
                type: 'object',
                properties: {
                  adapter: {
                    description:
                      "Public adapter contribution name; omitted selects the wire's default.",
                    type: 'string',
                  },
                  codeMode: {
                    description:
                      'Omitted means model-specific defaults; false is an explicit opt-out.',
                    type: 'boolean',
                  },
                  applyPatch: {
                    description:
                      'Independent of Code Mode: select patch editing or structured Edit/Write.',
                    type: 'boolean',
                  },
                  thinkingLevels: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/ThinkingLevel',
                    },
                  },
                  defaultThinkingLevel: {
                    $ref: '#/$defs/ThinkingLevel',
                  },
                  vision: {
                    type: 'boolean',
                  },
                  contextWindow: {
                    description:
                      'Override total input-plus-output capacity, without subtracting a reply budget.',
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  compactionThreshold: {
                    description:
                      'Optional override of the automatic compaction threshold, in retained tokens.\nBy default Host uses 85% of the smaller total window/input ceiling.',
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  inputLimit: {
                    description:
                      'Independent input ceiling; also caps the default compaction input budget.',
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  maxOutputTokens: {
                    description:
                      'Requested per-step output budget, subject to provider and remaining-capacity caps.',
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  displayName: {
                    type: 'string',
                  },
                  description: {
                    type: 'string',
                  },
                  apiProtocol: {
                    $ref: '#/$defs/ApiProtocol',
                  },
                  knowledgeCutoff: {
                    type: 'string',
                  },
                  capabilities: {
                    $ref: '#/$defs/ModelOverrideCapabilities',
                  },
                  modalities: {
                    $ref: '#/$defs/ModelModalities',
                  },
                  serviceTier: {
                    $ref: '#/$defs/RelayServiceTier',
                  },
                },
                additionalProperties: false,
              },
              ThinkingLevel: {
                type: 'string',
                enum: ['off', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max', 'ultra'],
              },
              ApiProtocol: {
                type: 'string',
                enum: ['openai-chat', 'openai-responses', 'anthropic-messages'],
              },
              ModelOverrideCapabilities: {
                description: 'Vision has one declaration owner: ModelOverride::vision.',
                type: 'object',
                properties: {
                  chat: {
                    type: 'boolean',
                  },
                  reasoning: {
                    type: 'boolean',
                  },
                  functionCalling: {
                    type: 'boolean',
                  },
                  parallelToolCalls: {
                    type: 'boolean',
                  },
                  imageGeneration: {
                    type: 'boolean',
                  },
                  webSearch: {
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
              },
              ModelModalities: {
                type: 'object',
                properties: {
                  input: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/ModelModality',
                    },
                  },
                  output: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/ModelModality',
                    },
                  },
                },
                additionalProperties: false,
                required: ['input', 'output'],
              },
              ModelModality: {
                type: 'string',
                enum: ['text', 'image', 'audio', 'pdf', 'video'],
              },
              RelayServiceTier: {
                type: 'string',
                enum: ['fast'],
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
                  reason: {
                    $ref: '#/$defs/ConfigurationRejection',
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'reason'],
              },
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
              ConfigurationRejection: {
                description:
                  'A confirmed rejection contains no provider diagnostics or user input.',
                type: 'string',
                enum: ['invalid_input', 'invalid_configuration', 'provider_unavailable'],
              },
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
        credentialStatus: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'CredentialVaultQueryInput',
            type: 'object',
            properties: {
              locator: {
                $ref: '#/$defs/CredentialLocator',
              },
            },
            additionalProperties: false,
            required: ['locator'],
            $defs: {
              CredentialLocator: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      connectionId: {
                        type: 'string',
                      },
                      kind: {
                        $ref: '#/$defs/ConnectionCredentialKind',
                      },
                      scope: {
                        type: 'string',
                        const: 'connection',
                      },
                    },
                    additionalProperties: false,
                    required: ['scope', 'connectionId', 'kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        $ref: '#/$defs/PasswordKind',
                      },
                      scope: {
                        type: 'string',
                        const: 'network_proxy',
                      },
                    },
                    additionalProperties: false,
                    required: ['scope', 'kind'],
                  },
                ],
              },
              ConnectionCredentialKind: {
                type: 'string',
                enum: ['provider', 'request_headers'],
              },
              PasswordKind: {
                type: 'string',
                enum: ['password'],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'CredentialVaultQueryResult',
            oneOf: [
              {
                type: 'object',
                properties: {
                  status: {
                    $ref: '#/$defs/CredentialStatus',
                  },
                  kind: {
                    type: 'string',
                    const: 'status',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'status'],
              },
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'connection_not_found',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
            ],
            $defs: {
              CredentialStatus: {
                type: 'object',
                properties: {
                  locator: {
                    $ref: '#/$defs/CredentialLocator',
                  },
                  configured: {
                    type: 'boolean',
                  },
                  credentialId: {
                    type: ['string', 'null'],
                  },
                  revision: {
                    type: ['integer', 'null'],
                    format: 'uint64',
                    minimum: 0,
                  },
                  updatedAt: {
                    type: ['integer', 'null'],
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['locator', 'configured', 'credentialId', 'revision', 'updatedAt'],
              },
              CredentialLocator: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      connectionId: {
                        type: 'string',
                      },
                      kind: {
                        $ref: '#/$defs/ConnectionCredentialKind',
                      },
                      scope: {
                        type: 'string',
                        const: 'connection',
                      },
                    },
                    additionalProperties: false,
                    required: ['scope', 'connectionId', 'kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        $ref: '#/$defs/PasswordKind',
                      },
                      scope: {
                        type: 'string',
                        const: 'network_proxy',
                      },
                    },
                    additionalProperties: false,
                    required: ['scope', 'kind'],
                  },
                ],
              },
              ConnectionCredentialKind: {
                type: 'string',
                enum: ['provider', 'request_headers'],
              },
              PasswordKind: {
                type: 'string',
                enum: ['password'],
              },
            },
          },
          resources: {},
        },
        deleteCredential: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'DeleteCredentialInput',
            type: 'object',
            properties: {
              expected: {
                $ref: '#/$defs/CredentialVersionBasis',
              },
              expectedConnection: {
                description: 'Required for connection credentials; null for network credentials.',
                anyOf: [
                  {
                    $ref: '#/$defs/ConnectionCredentialTarget',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
            },
            additionalProperties: false,
            required: ['expected', 'expectedConnection'],
            $defs: {
              CredentialVersionBasis: {
                type: 'object',
                properties: {
                  locator: {
                    $ref: '#/$defs/CredentialLocator',
                  },
                  credentialId: {
                    type: 'string',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['locator', 'credentialId', 'revision'],
              },
              CredentialLocator: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      connectionId: {
                        type: 'string',
                      },
                      kind: {
                        $ref: '#/$defs/ConnectionCredentialKind',
                      },
                      scope: {
                        type: 'string',
                        const: 'connection',
                      },
                    },
                    additionalProperties: false,
                    required: ['scope', 'connectionId', 'kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        $ref: '#/$defs/PasswordKind',
                      },
                      scope: {
                        type: 'string',
                        const: 'network_proxy',
                      },
                    },
                    additionalProperties: false,
                    required: ['scope', 'kind'],
                  },
                ],
              },
              ConnectionCredentialKind: {
                type: 'string',
                enum: ['provider', 'request_headers'],
              },
              PasswordKind: {
                type: 'string',
                enum: ['password'],
              },
              ConnectionCredentialTarget: {
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
                  slug: {
                    type: 'string',
                  },
                  provider: {
                    $ref: '#/$defs/Identity',
                  },
                  configuration: {
                    type: 'object',
                  },
                },
                additionalProperties: false,
                required: ['connectionId', 'revision', 'slug', 'provider', 'configuration'],
              },
              Identity: {
                description:
                  'Durable recipient, not a registration lease. Re-loading the same package can\nrestore it; another package, Entry or scope cannot inherit its credentials.',
                type: 'object',
                properties: {
                  packageId: {
                    type: 'string',
                  },
                  entryId: {
                    type: 'string',
                  },
                  scope: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['packageId', 'entryId', 'scope', 'name'],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'CredentialMutationResult',
            oneOf: [
              {
                type: 'object',
                properties: {
                  reason: {
                    $ref: '#/$defs/ConfigurationRejection',
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'reason'],
              },
              {
                type: 'object',
                properties: {
                  vaultRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  status: {
                    $ref: '#/$defs/CredentialStatus',
                  },
                  kind: {
                    type: 'string',
                    const: 'committed',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'vaultRevision', 'status'],
              },
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'connection_not_found',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
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
                  expected: {
                    anyOf: [
                      {
                        $ref: '#/$defs/CredentialVersionBasis',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  actual: {
                    anyOf: [
                      {
                        $ref: '#/$defs/CredentialVersionBasis',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  kind: {
                    type: 'string',
                    const: 'credential_stale',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'expected', 'actual'],
              },
            ],
            $defs: {
              ConfigurationRejection: {
                description:
                  'A confirmed rejection contains no provider diagnostics or user input.',
                type: 'string',
                enum: ['invalid_input', 'invalid_configuration', 'provider_unavailable'],
              },
              CredentialStatus: {
                type: 'object',
                properties: {
                  locator: {
                    $ref: '#/$defs/CredentialLocator',
                  },
                  configured: {
                    type: 'boolean',
                  },
                  credentialId: {
                    type: ['string', 'null'],
                  },
                  revision: {
                    type: ['integer', 'null'],
                    format: 'uint64',
                    minimum: 0,
                  },
                  updatedAt: {
                    type: ['integer', 'null'],
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['locator', 'configured', 'credentialId', 'revision', 'updatedAt'],
              },
              CredentialLocator: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      connectionId: {
                        type: 'string',
                      },
                      kind: {
                        $ref: '#/$defs/ConnectionCredentialKind',
                      },
                      scope: {
                        type: 'string',
                        const: 'connection',
                      },
                    },
                    additionalProperties: false,
                    required: ['scope', 'connectionId', 'kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        $ref: '#/$defs/PasswordKind',
                      },
                      scope: {
                        type: 'string',
                        const: 'network_proxy',
                      },
                    },
                    additionalProperties: false,
                    required: ['scope', 'kind'],
                  },
                ],
              },
              ConnectionCredentialKind: {
                type: 'string',
                enum: ['provider', 'request_headers'],
              },
              PasswordKind: {
                type: 'string',
                enum: ['password'],
              },
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
              CredentialVersionBasis: {
                type: 'object',
                properties: {
                  locator: {
                    $ref: '#/$defs/CredentialLocator',
                  },
                  credentialId: {
                    type: 'string',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['locator', 'credentialId', 'revision'],
              },
            },
          },
          resources: {},
        },
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
        providers: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'SettingsProvidersRead',
            type: 'object',
            properties: {
              offset: {
                type: 'integer',
                format: 'uint',
                minimum: 0,
                maximum: 1024,
              },
            },
            additionalProperties: false,
            required: ['offset'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'SettingsProvidersPage',
            type: 'object',
            properties: {
              providers: {
                type: 'array',
                items: {
                  $ref: '#/$defs/SettingsProvider',
                },
                maxItems: 8,
              },
              next: {
                type: ['integer', 'null'],
                format: 'uint',
                minimum: 0,
              },
            },
            additionalProperties: false,
            required: ['providers', 'next'],
            $defs: {
              SettingsProvider: {
                type: 'object',
                properties: {
                  identity: {
                    $ref: '#/$defs/Identity',
                  },
                  descriptor: {
                    $ref: '#/$defs/Descriptor',
                  },
                },
                additionalProperties: false,
                required: ['identity', 'descriptor'],
              },
              Identity: {
                description:
                  'Durable recipient, not a registration lease. Re-loading the same package can\nrestore it; another package, Entry or scope cannot inherit its credentials.',
                type: 'object',
                properties: {
                  packageId: {
                    type: 'string',
                  },
                  entryId: {
                    type: 'string',
                  },
                  scope: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['packageId', 'entryId', 'scope', 'name'],
              },
              Descriptor: {
                type: 'object',
                properties: {
                  label: {
                    type: 'string',
                  },
                  customEndpoint: {
                    description: 'Offers a user-selected endpoint instead of a named service.',
                    type: 'boolean',
                    default: false,
                  },
                  configurationSchema: true,
                  configurationDefaults: true,
                  authentication: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/Method',
                    },
                  },
                  anonymous: {
                    description:
                      "Whether the provider supports a connection without a stored credential.\nPresentation metadata; authorization remains the provider's decision.",
                    type: 'boolean',
                  },
                  discovery: {
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: [
                  'label',
                  'customEndpoint',
                  'configurationSchema',
                  'configurationDefaults',
                  'authentication',
                  'anonymous',
                  'discovery',
                ],
              },
              Method: {
                type: 'object',
                properties: {
                  id: {
                    type: 'string',
                  },
                  label: {
                    type: 'string',
                  },
                  inputSchema: {
                    description:
                      'Secret-bearing form input never enters the connection configuration.',
                  },
                  interactive: {
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['id', 'label', 'inputSchema', 'interactive'],
              },
            },
          },
          resources: {},
        },
        remove: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'RemoveCatalogConnectionInput',
            type: 'object',
            properties: {
              expected: {
                $ref: '#/$defs/ConnectionVersionBasis',
              },
            },
            additionalProperties: false,
            required: ['expected'],
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
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'CatalogMutationResult',
            oneOf: [
              {
                type: 'object',
                properties: {
                  reason: {
                    $ref: '#/$defs/ConfigurationRejection',
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'reason'],
              },
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
              ConfigurationRejection: {
                description:
                  'A confirmed rejection contains no provider diagnostics or user input.',
                type: 'string',
                enum: ['invalid_input', 'invalid_configuration', 'provider_unavailable'],
              },
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
        saveCredential: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'SettingsSaveCredential',
            type: 'object',
            properties: {
              expected: {
                $ref: '#/$defs/ConnectionCredentialTarget',
              },
              expectedCredential: {
                anyOf: [
                  {
                    $ref: '#/$defs/CredentialVersionBasis',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
            },
            additionalProperties: false,
            required: ['expected', 'expectedCredential'],
            $defs: {
              ConnectionCredentialTarget: {
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
                  slug: {
                    type: 'string',
                  },
                  provider: {
                    $ref: '#/$defs/Identity',
                  },
                  configuration: {
                    type: 'object',
                  },
                },
                additionalProperties: false,
                required: ['connectionId', 'revision', 'slug', 'provider', 'configuration'],
              },
              Identity: {
                description:
                  'Durable recipient, not a registration lease. Re-loading the same package can\nrestore it; another package, Entry or scope cannot inherit its credentials.',
                type: 'object',
                properties: {
                  packageId: {
                    type: 'string',
                  },
                  entryId: {
                    type: 'string',
                  },
                  scope: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['packageId', 'entryId', 'scope', 'name'],
              },
              CredentialVersionBasis: {
                type: 'object',
                properties: {
                  locator: {
                    $ref: '#/$defs/CredentialLocator',
                  },
                  credentialId: {
                    type: 'string',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['locator', 'credentialId', 'revision'],
              },
              CredentialLocator: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      connectionId: {
                        type: 'string',
                      },
                      kind: {
                        $ref: '#/$defs/ConnectionCredentialKind',
                      },
                      scope: {
                        type: 'string',
                        const: 'connection',
                      },
                    },
                    additionalProperties: false,
                    required: ['scope', 'connectionId', 'kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        $ref: '#/$defs/PasswordKind',
                      },
                      scope: {
                        type: 'string',
                        const: 'network_proxy',
                      },
                    },
                    additionalProperties: false,
                    required: ['scope', 'kind'],
                  },
                ],
              },
              ConnectionCredentialKind: {
                type: 'string',
                enum: ['provider', 'request_headers'],
              },
              PasswordKind: {
                type: 'string',
                enum: ['password'],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'SettingsCredentialSaveResult',
            oneOf: [
              {
                type: 'object',
                properties: {
                  connection: {
                    $ref: '#/$defs/ConnectionIdentity',
                  },
                  kind: {
                    type: 'string',
                    const: 'saved',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'connection'],
              },
              {
                type: 'object',
                properties: {
                  reason: {
                    $ref: '#/$defs/SettingsCredentialRejection',
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'reason'],
              },
            ],
            $defs: {
              ConnectionIdentity: {
                type: 'object',
                properties: {
                  connectionId: {
                    type: 'string',
                  },
                  slug: {
                    type: 'string',
                  },
                  provider: {
                    $ref: '#/$defs/Identity',
                  },
                },
                additionalProperties: false,
                required: ['connectionId', 'slug', 'provider'],
              },
              Identity: {
                description:
                  'Durable recipient, not a registration lease. Re-loading the same package can\nrestore it; another package, Entry or scope cannot inherit its credentials.',
                type: 'object',
                properties: {
                  packageId: {
                    type: 'string',
                  },
                  entryId: {
                    type: 'string',
                  },
                  scope: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['packageId', 'entryId', 'scope', 'name'],
              },
              SettingsCredentialRejection: {
                type: 'string',
                enum: [
                  'invalid_input',
                  'invalid_secret',
                  'secret_unavailable',
                  'provider_unavailable',
                  'authentication_unavailable',
                  'attempt_conflict',
                  'connection_changed',
                  'credential_changed',
                  'capacity',
                  'slug_taken',
                  'authentication',
                ],
              },
            },
          },
          resources: {
            secret: {
              name: 'maka.text.utf8',
              version: 1,
            },
          },
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
                  reason: {
                    $ref: '#/$defs/ConfigurationRejection',
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'reason'],
              },
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
              ConfigurationRejection: {
                description:
                  'A confirmed rejection contains no provider diagnostics or user input.',
                type: 'string',
                enum: ['invalid_input', 'invalid_configuration', 'provider_unavailable'],
              },
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
        update: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'UpdateCatalogConnectionInput',
            type: 'object',
            properties: {
              expected: {
                $ref: '#/$defs/ConnectionVersionBasis',
              },
              changes: {
                $ref: '#/$defs/ConnectionCatalogEntryUpdate',
              },
            },
            additionalProperties: false,
            required: ['expected', 'changes'],
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
              ConnectionCatalogEntryUpdate: {
                type: 'object',
                properties: {
                  name: {
                    type: 'string',
                  },
                  configuration: {
                    type: 'object',
                  },
                  enabled: {
                    type: 'boolean',
                  },
                  enabledModelIds: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                  modelOverrides: {
                    $ref: '#/$defs/Nullable_Map_of_ModelOverride',
                  },
                  requestBodyOverlay: {
                    type: ['object', 'null'],
                  },
                },
                additionalProperties: false,
              },
              Nullable_Map_of_ModelOverride: {
                type: ['object', 'null'],
                additionalProperties: {
                  $ref: '#/$defs/ModelOverride',
                },
              },
              ModelOverride: {
                type: 'object',
                properties: {
                  adapter: {
                    description:
                      "Public adapter contribution name; omitted selects the wire's default.",
                    type: 'string',
                  },
                  codeMode: {
                    description:
                      'Omitted means model-specific defaults; false is an explicit opt-out.',
                    type: 'boolean',
                  },
                  applyPatch: {
                    description:
                      'Independent of Code Mode: select patch editing or structured Edit/Write.',
                    type: 'boolean',
                  },
                  thinkingLevels: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/ThinkingLevel',
                    },
                  },
                  defaultThinkingLevel: {
                    $ref: '#/$defs/ThinkingLevel',
                  },
                  vision: {
                    type: 'boolean',
                  },
                  contextWindow: {
                    description:
                      'Override total input-plus-output capacity, without subtracting a reply budget.',
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  compactionThreshold: {
                    description:
                      'Optional override of the automatic compaction threshold, in retained tokens.\nBy default Host uses 85% of the smaller total window/input ceiling.',
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  inputLimit: {
                    description:
                      'Independent input ceiling; also caps the default compaction input budget.',
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  maxOutputTokens: {
                    description:
                      'Requested per-step output budget, subject to provider and remaining-capacity caps.',
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  displayName: {
                    type: 'string',
                  },
                  description: {
                    type: 'string',
                  },
                  apiProtocol: {
                    $ref: '#/$defs/ApiProtocol',
                  },
                  knowledgeCutoff: {
                    type: 'string',
                  },
                  capabilities: {
                    $ref: '#/$defs/ModelOverrideCapabilities',
                  },
                  modalities: {
                    $ref: '#/$defs/ModelModalities',
                  },
                  serviceTier: {
                    $ref: '#/$defs/RelayServiceTier',
                  },
                },
                additionalProperties: false,
              },
              ThinkingLevel: {
                type: 'string',
                enum: ['off', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max', 'ultra'],
              },
              ApiProtocol: {
                type: 'string',
                enum: ['openai-chat', 'openai-responses', 'anthropic-messages'],
              },
              ModelOverrideCapabilities: {
                description: 'Vision has one declaration owner: ModelOverride::vision.',
                type: 'object',
                properties: {
                  chat: {
                    type: 'boolean',
                  },
                  reasoning: {
                    type: 'boolean',
                  },
                  functionCalling: {
                    type: 'boolean',
                  },
                  parallelToolCalls: {
                    type: 'boolean',
                  },
                  imageGeneration: {
                    type: 'boolean',
                  },
                  webSearch: {
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
              },
              ModelModalities: {
                type: 'object',
                properties: {
                  input: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/ModelModality',
                    },
                  },
                  output: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/ModelModality',
                    },
                  },
                },
                additionalProperties: false,
                required: ['input', 'output'],
              },
              ModelModality: {
                type: 'string',
                enum: ['text', 'image', 'audio', 'pdf', 'video'],
              },
              RelayServiceTier: {
                type: 'string',
                enum: ['fast'],
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
                  reason: {
                    $ref: '#/$defs/ConfigurationRejection',
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'reason'],
              },
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
              ConfigurationRejection: {
                description:
                  'A confirmed rejection contains no provider diagnostics or user input.',
                type: 'string',
                enum: ['invalid_input', 'invalid_configuration', 'provider_unavailable'],
              },
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
