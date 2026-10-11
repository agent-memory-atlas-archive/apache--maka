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
  markdown: {
    name: 'maka.ui.Markdown',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'MarkdownProps',
        type: 'object',
        properties: {
          ascii: {
            type: 'boolean',
            default: false,
          },
          animate: {
            type: 'boolean',
            default: false,
          },
          source: {
            type: 'string',
            maxLength: 65536,
          },
          appearance: {
            $ref: '#/$defs/MarkdownAppearance',
          },
          selectable: {
            type: 'boolean',
          },
          disabled: {
            type: 'boolean',
          },
          focusOnPointer: {
            description: 'Whether pointer selection also moves keyboard focus to this document.',
            type: 'boolean',
            default: true,
          },
        },
        additionalProperties: false,
        required: ['source', 'appearance', 'selectable', 'disabled'],
        $defs: {
          MarkdownAppearance: {
            type: 'object',
            properties: {
              terminal: {
                anyOf: [
                  {
                    $ref: '#/$defs/TerminalPalette',
                  },
                  {
                    type: 'null',
                  },
                ],
                default: null,
              },
              body: {
                $ref: '#/$defs/TextAppearance',
              },
              code: {
                $ref: '#/$defs/TextAppearance',
              },
              headings: {
                description: 'Typography for heading levels 1 through 6, in source order.',
                type: 'array',
                items: {
                  $ref: '#/$defs/TextAppearance',
                },
                minItems: 6,
                maxItems: 6,
              },
              muted: {
                $ref: '#/$defs/Color',
              },
              link: {
                $ref: '#/$defs/Color',
              },
              selection: {
                $ref: '#/$defs/Color',
              },
              codeBackground: {
                $ref: '#/$defs/Color',
              },
              tableBackground: {
                $ref: '#/$defs/Color',
              },
              border: {
                $ref: '#/$defs/Color',
              },
              dark: {
                type: 'boolean',
              },
              paragraphGap: {
                description: 'Paragraph spacing in logical pixels.',
                type: 'number',
                format: 'float',
                minimum: 0,
                maximum: 128,
              },
            },
            additionalProperties: false,
            required: [
              'body',
              'code',
              'headings',
              'muted',
              'link',
              'selection',
              'codeBackground',
              'tableBackground',
              'border',
              'dark',
              'paragraphGap',
            ],
          },
          TerminalPalette: {
            description:
              'Semantic terminal colors for rich text and editors, supplied by the product.',
            type: 'object',
            properties: {
              background: {
                $ref: '#/$defs/TerminalColor',
              },
              foreground: {
                $ref: '#/$defs/TerminalColor',
              },
              surface: {
                $ref: '#/$defs/TerminalColor',
              },
              accent: {
                $ref: '#/$defs/TerminalColor',
              },
              thinking: {
                $ref: '#/$defs/TerminalColor',
              },
              success: {
                $ref: '#/$defs/TerminalColor',
              },
              warning: {
                $ref: '#/$defs/TerminalColor',
              },
              error: {
                $ref: '#/$defs/TerminalColor',
              },
              muted: {
                $ref: '#/$defs/TerminalColor',
              },
              subtle: {
                $ref: '#/$defs/TerminalColor',
              },
              border: {
                $ref: '#/$defs/TerminalColor',
              },
              scrollbar: {
                $ref: '#/$defs/TerminalColor',
              },
              selection: {
                $ref: '#/$defs/TerminalColor',
              },
              selectionText: {
                $ref: '#/$defs/TerminalColor',
              },
              search: {
                $ref: '#/$defs/TerminalColor',
              },
              searchActive: {
                $ref: '#/$defs/TerminalColor',
              },
              searchText: {
                $ref: '#/$defs/TerminalColor',
              },
              syntax: {
                type: 'array',
                items: {
                  $ref: '#/$defs/TerminalColor',
                },
                minItems: 7,
                maxItems: 7,
              },
              terminal: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: [
              'background',
              'foreground',
              'surface',
              'accent',
              'thinking',
              'success',
              'warning',
              'error',
              'muted',
              'subtle',
              'border',
              'scrollbar',
              'selection',
              'selectionText',
              'search',
              'searchActive',
              'searchText',
              'syntax',
              'terminal',
            ],
          },
          TerminalColor: {
            description:
              "Terminal colors retain the user's ANSI palette and default background.\nNumeric values are RGB; named values are emitted as terminal attributes.",
            anyOf: [
              {
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 16777215,
              },
              {
                $ref: '#/$defs/TerminalNamedColor',
              },
            ],
          },
          TerminalNamedColor: {
            type: 'string',
            enum: [
              'reset',
              'black',
              'red',
              'green',
              'yellow',
              'blue',
              'magenta',
              'cyan',
              'gray',
              'dark_gray',
              'light_red',
              'light_green',
              'light_yellow',
              'light_blue',
              'light_magenta',
              'light_cyan',
              'white',
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
        },
      },
      events: {
        link: {
          $schema: 'https://json-schema.org/draft/2020-12/schema',
          title: 'MarkdownLink',
          type: 'object',
          properties: {
            url: {
              type: 'string',
              maxLength: 65536,
            },
          },
          additionalProperties: false,
          required: ['url'],
        },
      },
      captures: {},
    },
  },
  markdownText: {
    name: 'maka.text.Markdown',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'service',
      methods: {
        plain: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'MarkdownTextInput',
            description:
              "Bounded source-to-readable-text conversion using the presenter's Markdown semantics.",
            type: 'object',
            properties: {
              source: {
                type: 'string',
                maxLength: 65536,
              },
            },
            additionalProperties: false,
            required: ['source'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'MarkdownTextResult',
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
                    const: 'ready',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'text'],
              },
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'too_large',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
            ],
          },
          resources: {},
        },
      },
    },
  },
  diff: {
    name: 'maka.ui.Diff',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'DiffProps',
        type: 'object',
        properties: {
          content: {
            $ref: '#/$defs/DiffContent',
          },
          appearance: {
            $ref: '#/$defs/DiffAppearance',
          },
          ascii: {
            type: 'boolean',
          },
          disabled: {
            type: 'boolean',
          },
          focusOnPointer: {
            type: 'boolean',
          },
        },
        additionalProperties: false,
        required: ['content', 'appearance', 'ascii', 'disabled', 'focusOnPointer'],
        $defs: {
          DiffContent: {
            description:
              'Complete bounded comparisons and structured patches reuse native parsers.',
            oneOf: [
              {
                type: 'object',
                properties: {
                  source: {
                    type: 'string',
                    maxLength: 65536,
                  },
                  rows: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/DiffRow',
                    },
                    maxItems: 8192,
                  },
                  kind: {
                    type: 'string',
                    const: 'annotated',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'source', 'rows'],
              },
              {
                description:
                  'Combined inputs must be <= 8192 UTF-8 bytes and <= 256 lines, without NUL.',
                type: 'object',
                properties: {
                  before: {
                    type: 'string',
                    maxLength: 8192,
                  },
                  after: {
                    type: 'string',
                    maxLength: 8192,
                  },
                  language: {
                    type: ['string', 'null'],
                    maxLength: 128,
                  },
                  kind: {
                    type: 'string',
                    const: 'comparison',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'before', 'after'],
              },
              {
                description:
                  'Invalid, binary or oversized patches display literally, without diff gutters.',
                type: 'object',
                properties: {
                  source: {
                    type: 'string',
                    maxLength: 65536,
                  },
                  operation: {
                    $ref: '#/$defs/DiffPatch',
                  },
                  language: {
                    type: ['string', 'null'],
                    maxLength: 128,
                  },
                  kind: {
                    type: 'string',
                    const: 'patch',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'source', 'operation'],
              },
            ],
          },
          DiffRow: {
            type: 'object',
            properties: {
              start: {
                description: 'Half-open UTF-8 byte range in source. Rows are ordered and disjoint.',
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 65536,
              },
              end: {
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 65536,
              },
              kind: {
                $ref: '#/$defs/DiffKind',
              },
              language: {
                type: ['string', 'null'],
                maxLength: 128,
              },
            },
            additionalProperties: false,
            required: ['start', 'end', 'kind'],
          },
          DiffKind: {
            type: 'string',
            enum: ['removed', 'added', 'context', 'content'],
          },
          DiffPatch: {
            type: 'string',
            enum: ['create', 'update'],
          },
          DiffAppearance: {
            type: 'object',
            properties: {
              terminal: {
                anyOf: [
                  {
                    $ref: '#/$defs/TerminalPalette',
                  },
                  {
                    type: 'null',
                  },
                ],
                default: null,
              },
              text: {
                $ref: '#/$defs/TextAppearance',
              },
              background: {
                $ref: '#/$defs/Color',
              },
              added: {
                $ref: '#/$defs/Color',
              },
              removed: {
                $ref: '#/$defs/Color',
              },
              selection: {
                $ref: '#/$defs/Color',
              },
              dark: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: ['text', 'background', 'added', 'removed', 'selection', 'dark'],
          },
          TerminalPalette: {
            description:
              'Semantic terminal colors for rich text and editors, supplied by the product.',
            type: 'object',
            properties: {
              background: {
                $ref: '#/$defs/TerminalColor',
              },
              foreground: {
                $ref: '#/$defs/TerminalColor',
              },
              surface: {
                $ref: '#/$defs/TerminalColor',
              },
              accent: {
                $ref: '#/$defs/TerminalColor',
              },
              thinking: {
                $ref: '#/$defs/TerminalColor',
              },
              success: {
                $ref: '#/$defs/TerminalColor',
              },
              warning: {
                $ref: '#/$defs/TerminalColor',
              },
              error: {
                $ref: '#/$defs/TerminalColor',
              },
              muted: {
                $ref: '#/$defs/TerminalColor',
              },
              subtle: {
                $ref: '#/$defs/TerminalColor',
              },
              border: {
                $ref: '#/$defs/TerminalColor',
              },
              scrollbar: {
                $ref: '#/$defs/TerminalColor',
              },
              selection: {
                $ref: '#/$defs/TerminalColor',
              },
              selectionText: {
                $ref: '#/$defs/TerminalColor',
              },
              search: {
                $ref: '#/$defs/TerminalColor',
              },
              searchActive: {
                $ref: '#/$defs/TerminalColor',
              },
              searchText: {
                $ref: '#/$defs/TerminalColor',
              },
              syntax: {
                type: 'array',
                items: {
                  $ref: '#/$defs/TerminalColor',
                },
                minItems: 7,
                maxItems: 7,
              },
              terminal: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: [
              'background',
              'foreground',
              'surface',
              'accent',
              'thinking',
              'success',
              'warning',
              'error',
              'muted',
              'subtle',
              'border',
              'scrollbar',
              'selection',
              'selectionText',
              'search',
              'searchActive',
              'searchText',
              'syntax',
              'terminal',
            ],
          },
          TerminalColor: {
            description:
              "Terminal colors retain the user's ANSI palette and default background.\nNumeric values are RGB; named values are emitted as terminal attributes.",
            anyOf: [
              {
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 16777215,
              },
              {
                $ref: '#/$defs/TerminalNamedColor',
              },
            ],
          },
          TerminalNamedColor: {
            type: 'string',
            enum: [
              'reset',
              'black',
              'red',
              'green',
              'yellow',
              'blue',
              'magenta',
              'cyan',
              'gray',
              'dark_gray',
              'light_red',
              'light_green',
              'light_yellow',
              'light_blue',
              'light_magenta',
              'light_cyan',
              'white',
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
        },
      },
      events: {},
      captures: {},
    },
  },
  image: {
    name: 'maka.ui.Image',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'ImageProps',
        type: 'object',
        properties: {
          source: {
            anyOf: [
              {
                $ref: '#/$defs/ImageDescriptor',
              },
              {
                type: 'null',
              },
            ],
          },
          packet: {
            anyOf: [
              {
                $ref: '#/$defs/ImagePacket',
              },
              {
                type: 'null',
              },
            ],
          },
          alt: {
            type: 'string',
            maxLength: 4096,
          },
          fit: {
            $ref: '#/$defs/ImageFit',
          },
          height: {
            type: 'number',
            format: 'float',
            minimum: 1,
            maximum: 8192,
          },
          background: {
            $ref: '#/$defs/Color',
          },
          border: {
            $ref: '#/$defs/Color',
          },
          radius: {
            type: 'number',
            format: 'float',
            minimum: 0,
            maximum: 128,
          },
        },
        additionalProperties: false,
        required: ['alt', 'fit', 'height', 'background', 'border', 'radius'],
        $defs: {
          ImageDescriptor: {
            type: 'object',
            properties: {
              key: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              digest: {
                type: 'string',
                pattern: '^sha256:[a-f0-9]{64}$',
              },
              mimeType: {
                type: 'string',
                pattern: '^image/(png|jpeg|gif|webp|avif)$',
              },
              totalBytes: {
                type: 'integer',
                format: 'uint64',
                minimum: 1,
                maximum: 5242880,
              },
            },
            additionalProperties: false,
            required: ['key', 'digest', 'mimeType', 'totalBytes'],
          },
          ImagePacket: {
            type: 'object',
            properties: {
              owner: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              source: {
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
              offset: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 5242880,
              },
              data: {
                type: 'string',
                minLength: 4,
                maxLength: 43692,
                pattern: '^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$',
              },
            },
            additionalProperties: false,
            required: ['owner', 'source', 'sequence', 'offset', 'data'],
          },
          ImageFit: {
            type: 'string',
            enum: ['contain', 'cover'],
          },
          Color: {
            description: 'Packed RGBA, including explicit alpha zero.',
            type: 'integer',
            format: 'uint32',
            minimum: 0,
          },
        },
      },
      events: {
        observed: {
          $schema: 'https://json-schema.org/draft/2020-12/schema',
          title: 'ImageObservation',
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
            source: {
              type: ['string', 'null'],
            },
            received: {
              type: 'integer',
              format: 'uint64',
              minimum: 0,
            },
            phase: {
              $ref: '#/$defs/ImagePhase',
            },
            width: {
              type: ['integer', 'null'],
              format: 'uint32',
              minimum: 0,
            },
            height: {
              type: ['integer', 'null'],
              format: 'uint32',
              minimum: 0,
            },
            error: {
              anyOf: [
                {
                  $ref: '#/$defs/ImageError',
                },
                {
                  type: 'null',
                },
              ],
            },
            receipt: {
              anyOf: [
                {
                  $ref: '#/$defs/ImageReceipt',
                },
                {
                  type: 'null',
                },
              ],
            },
          },
          additionalProperties: false,
          required: ['owner', 'revision', 'received', 'phase'],
          $defs: {
            ImagePhase: {
              type: 'string',
              enum: ['empty', 'receiving', 'decoding', 'ready', 'failed'],
            },
            ImageError: {
              type: 'string',
              enum: [
                'budget_exhausted',
                'limit_exceeded',
                'source_conflict',
                'packet_rejected',
                'digest_mismatch',
                'unsupported_format',
                'invalid_image',
                'decode_failed',
              ],
            },
            ImageReceipt: {
              type: 'object',
              properties: {
                owner: {
                  type: 'string',
                },
                source: {
                  type: 'string',
                },
                sequence: {
                  type: 'integer',
                  format: 'uint64',
                  minimum: 0,
                },
                accepted: {
                  type: 'boolean',
                },
                error: {
                  anyOf: [
                    {
                      $ref: '#/$defs/ImageError',
                    },
                    {
                      type: 'null',
                    },
                  ],
                },
              },
              additionalProperties: false,
              required: ['owner', 'source', 'sequence', 'accepted'],
            },
          },
        },
      },
      captures: {},
    },
  },
  filePicker: {
    name: 'maka.ui.FilePicker',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'FilePickerProps',
        type: 'object',
        properties: {
          appearance: {
            $ref: '#/$defs/InputAppearance',
          },
          label: {
            type: 'string',
            minLength: 1,
            maxLength: 512,
          },
          pathLabel: {
            type: 'string',
            minLength: 1,
            maxLength: 512,
          },
          browseLabel: {
            type: 'string',
            minLength: 1,
            maxLength: 512,
          },
          parentLabel: {
            type: 'string',
            minLength: 1,
            maxLength: 512,
          },
          emptyLabel: {
            type: 'string',
            minLength: 1,
            maxLength: 512,
          },
          truncatedLabel: {
            type: 'string',
            minLength: 1,
            maxLength: 512,
          },
          placeholder: {
            type: 'string',
            maxLength: 512,
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
        },
        additionalProperties: false,
        required: [
          'appearance',
          'label',
          'pathLabel',
          'browseLabel',
          'parentLabel',
          'emptyLabel',
          'truncatedLabel',
          'placeholder',
          'disabled',
          'tabIndex',
        ],
        $defs: {
          InputAppearance: {
            type: 'object',
            properties: {
              ascii: {
                type: 'boolean',
                default: false,
              },
              terminal: {
                anyOf: [
                  {
                    $ref: '#/$defs/TerminalPalette',
                  },
                  {
                    type: 'null',
                  },
                ],
                default: null,
              },
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
                description:
                  'In TUI inputs, a positive width draws a one-cell border; zero has no inset.',
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
          TerminalPalette: {
            description:
              'Semantic terminal colors for rich text and editors, supplied by the product.',
            type: 'object',
            properties: {
              background: {
                $ref: '#/$defs/TerminalColor',
              },
              foreground: {
                $ref: '#/$defs/TerminalColor',
              },
              surface: {
                $ref: '#/$defs/TerminalColor',
              },
              accent: {
                $ref: '#/$defs/TerminalColor',
              },
              thinking: {
                $ref: '#/$defs/TerminalColor',
              },
              success: {
                $ref: '#/$defs/TerminalColor',
              },
              warning: {
                $ref: '#/$defs/TerminalColor',
              },
              error: {
                $ref: '#/$defs/TerminalColor',
              },
              muted: {
                $ref: '#/$defs/TerminalColor',
              },
              subtle: {
                $ref: '#/$defs/TerminalColor',
              },
              border: {
                $ref: '#/$defs/TerminalColor',
              },
              scrollbar: {
                $ref: '#/$defs/TerminalColor',
              },
              selection: {
                $ref: '#/$defs/TerminalColor',
              },
              selectionText: {
                $ref: '#/$defs/TerminalColor',
              },
              search: {
                $ref: '#/$defs/TerminalColor',
              },
              searchActive: {
                $ref: '#/$defs/TerminalColor',
              },
              searchText: {
                $ref: '#/$defs/TerminalColor',
              },
              syntax: {
                type: 'array',
                items: {
                  $ref: '#/$defs/TerminalColor',
                },
                minItems: 7,
                maxItems: 7,
              },
              terminal: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: [
              'background',
              'foreground',
              'surface',
              'accent',
              'thinking',
              'success',
              'warning',
              'error',
              'muted',
              'subtle',
              'border',
              'scrollbar',
              'selection',
              'selectionText',
              'search',
              'searchActive',
              'searchText',
              'syntax',
              'terminal',
            ],
          },
          TerminalColor: {
            description:
              "Terminal colors retain the user's ANSI palette and default background.\nNumeric values are RGB; named values are emitted as terminal attributes.",
            anyOf: [
              {
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 16777215,
              },
              {
                $ref: '#/$defs/TerminalNamedColor',
              },
            ],
          },
          TerminalNamedColor: {
            type: 'string',
            enum: [
              'reset',
              'black',
              'red',
              'green',
              'yellow',
              'blue',
              'magenta',
              'cyan',
              'gray',
              'dark_gray',
              'light_red',
              'light_green',
              'light_yellow',
              'light_blue',
              'light_magenta',
              'light_cyan',
              'white',
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
        },
      },
      events: {
        changed: {
          $schema: 'https://json-schema.org/draft/2020-12/schema',
          title: 'FileObservation',
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
            phase: {
              $ref: '#/$defs/FilePhase',
            },
            file: {
              anyOf: [
                {
                  $ref: '#/$defs/FileDescriptor',
                },
                {
                  type: 'null',
                },
              ],
            },
            error: {
              anyOf: [
                {
                  $ref: '#/$defs/FileError',
                },
                {
                  type: 'null',
                },
              ],
            },
          },
          additionalProperties: false,
          required: ['owner', 'revision', 'phase'],
          $defs: {
            FilePhase: {
              type: 'string',
              enum: ['empty', 'choosing', 'ready', 'failed'],
            },
            FileDescriptor: {
              type: 'object',
              properties: {
                name: {
                  type: 'string',
                  maxLength: 512,
                },
                bytes: {
                  type: 'integer',
                  format: 'uint',
                  minimum: 0,
                },
                digest: {
                  type: 'string',
                  minLength: 71,
                  maxLength: 71,
                },
              },
              additionalProperties: false,
              required: ['name', 'bytes', 'digest'],
            },
            FileError: {
              type: 'string',
              enum: [
                'invalid_file',
                'read_failed',
                'limit_exceeded',
                'budget_exhausted',
                'worker_failed',
                'clipboard_unavailable',
                'paste_rejected',
              ],
            },
          },
        },
      },
      captures: {
        snapshot: {
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'FileCapture',
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
            },
            additionalProperties: false,
            required: ['owner', 'revision'],
          },
          output: {
            kind: 'resource',
            resource: {
              name: 'maka.bytes',
              version: 1,
            },
          },
        },
      },
    },
  },
  sourceText: {
    name: 'maka.ui.SourceText',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'SourceTextProps',
        type: 'object',
        properties: {
          animate: {
            type: 'boolean',
            default: false,
          },
          terminal: {
            anyOf: [
              {
                $ref: '#/$defs/TerminalPalette',
              },
              {
                type: 'null',
              },
            ],
            default: null,
          },
          source: {
            type: 'string',
            maxLength: 65536,
          },
          appearance: {
            $ref: '#/$defs/TextAppearance',
          },
          selection: {
            $ref: '#/$defs/Color',
          },
          disabled: {
            type: 'boolean',
          },
          focusOnPointer: {
            description: 'Whether pointer selection also moves keyboard focus to this source.',
            type: 'boolean',
            default: true,
          },
        },
        additionalProperties: false,
        required: ['source', 'appearance', 'selection', 'disabled'],
        $defs: {
          TerminalPalette: {
            description:
              'Semantic terminal colors for rich text and editors, supplied by the product.',
            type: 'object',
            properties: {
              background: {
                $ref: '#/$defs/TerminalColor',
              },
              foreground: {
                $ref: '#/$defs/TerminalColor',
              },
              surface: {
                $ref: '#/$defs/TerminalColor',
              },
              accent: {
                $ref: '#/$defs/TerminalColor',
              },
              thinking: {
                $ref: '#/$defs/TerminalColor',
              },
              success: {
                $ref: '#/$defs/TerminalColor',
              },
              warning: {
                $ref: '#/$defs/TerminalColor',
              },
              error: {
                $ref: '#/$defs/TerminalColor',
              },
              muted: {
                $ref: '#/$defs/TerminalColor',
              },
              subtle: {
                $ref: '#/$defs/TerminalColor',
              },
              border: {
                $ref: '#/$defs/TerminalColor',
              },
              scrollbar: {
                $ref: '#/$defs/TerminalColor',
              },
              selection: {
                $ref: '#/$defs/TerminalColor',
              },
              selectionText: {
                $ref: '#/$defs/TerminalColor',
              },
              search: {
                $ref: '#/$defs/TerminalColor',
              },
              searchActive: {
                $ref: '#/$defs/TerminalColor',
              },
              searchText: {
                $ref: '#/$defs/TerminalColor',
              },
              syntax: {
                type: 'array',
                items: {
                  $ref: '#/$defs/TerminalColor',
                },
                minItems: 7,
                maxItems: 7,
              },
              terminal: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: [
              'background',
              'foreground',
              'surface',
              'accent',
              'thinking',
              'success',
              'warning',
              'error',
              'muted',
              'subtle',
              'border',
              'scrollbar',
              'selection',
              'selectionText',
              'search',
              'searchActive',
              'searchText',
              'syntax',
              'terminal',
            ],
          },
          TerminalColor: {
            description:
              "Terminal colors retain the user's ANSI palette and default background.\nNumeric values are RGB; named values are emitted as terminal attributes.",
            anyOf: [
              {
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 16777215,
              },
              {
                $ref: '#/$defs/TerminalNamedColor',
              },
            ],
          },
          TerminalNamedColor: {
            type: 'string',
            enum: [
              'reset',
              'black',
              'red',
              'green',
              'yellow',
              'blue',
              'magenta',
              'cyan',
              'gray',
              'dark_gray',
              'light_red',
              'light_green',
              'light_yellow',
              'light_blue',
              'light_magenta',
              'light_cyan',
              'white',
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
        },
      },
      events: {},
      captures: {},
    },
  },
  terminalView: {
    name: 'maka.ui.TerminalView',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        type: 'object',
        additionalProperties: false,
        properties: {
          source: {
            type: 'string',
            description:
              'Changed when the selected canonical resource changes, including read-only views.',
            minLength: 1,
            maxLength: 1024,
          },
          accessibilityLabel: {
            type: 'string',
            minLength: 1,
            maxLength: 512,
          },
          output: {
            anyOf: [
              {
                $ref: '#/$defs/ShellOutput',
              },
              {
                type: 'null',
              },
            ],
          },
          controller: {
            type: ['string', 'null'],
            format: 'uuid',
          },
          acknowledgement: {
            anyOf: [
              {
                $ref: '#/$defs/TerminalInputAcknowledgement',
              },
              {
                type: 'null',
              },
            ],
          },
          appearance: {
            $ref: '#/$defs/TextAppearance',
          },
          background: {
            $ref: '#/$defs/Color',
          },
          caret: {
            $ref: '#/$defs/Color',
          },
          rows: {
            type: 'integer',
            format: 'uint16',
            minimum: 1,
            maximum: 100,
          },
          tabIndex: {
            type: 'integer',
            format: 'int16',
            minimum: -1,
            maximum: 32767,
          },
        },
        required: [
          'source',
          'accessibilityLabel',
          'appearance',
          'background',
          'caret',
          'rows',
          'tabIndex',
        ],
        title: 'TerminalViewProps',
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        $defs: {
          ShellOutput: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  stdout: {
                    type: 'string',
                  },
                  stderr: {
                    type: 'string',
                  },
                  latestStream: {
                    $ref: '#/$defs/PipeStream',
                  },
                  stdoutTruncated: {
                    type: 'boolean',
                  },
                  stderrTruncated: {
                    type: 'boolean',
                  },
                  redacted: {
                    type: 'boolean',
                  },
                  mode: {
                    type: 'string',
                    const: 'pipes',
                  },
                },
                required: [
                  'mode',
                  'stdout',
                  'stderr',
                  'stdoutTruncated',
                  'stderrTruncated',
                  'redacted',
                ],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  screen: {
                    type: 'string',
                  },
                  scrollback: {
                    type: 'string',
                  },
                  lastAlternateScreen: {
                    type: 'string',
                  },
                  cols: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  rows: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  cursor: {
                    $ref: '#/$defs/TerminalCursor',
                  },
                  alternateScreen: {
                    type: 'boolean',
                  },
                  truncated: {
                    type: 'boolean',
                  },
                  redacted: {
                    type: 'boolean',
                  },
                  mode: {
                    type: 'string',
                    const: 'pty',
                  },
                },
                required: [
                  'mode',
                  'screen',
                  'scrollback',
                  'cols',
                  'rows',
                  'cursor',
                  'alternateScreen',
                  'truncated',
                  'redacted',
                ],
              },
            ],
          },
          PipeStream: {
            type: 'string',
            enum: ['stdout', 'stderr'],
          },
          TerminalCursor: {
            type: 'object',
            additionalProperties: false,
            properties: {
              x: {
                type: 'integer',
                format: 'uint16',
                minimum: 0,
                maximum: 65535,
              },
              y: {
                type: 'integer',
                format: 'uint16',
                minimum: 0,
                maximum: 65535,
              },
              visible: {
                type: 'boolean',
              },
            },
            required: ['x', 'y', 'visible'],
          },
          TerminalInputAcknowledgement: {
            type: 'object',
            additionalProperties: false,
            properties: {
              owner: {
                type: 'string',
                format: 'uuid',
              },
              controller: {
                type: 'string',
                format: 'uuid',
              },
              sequence: {
                type: 'integer',
                format: 'uint64',
                minimum: 1,
                maximum: 9007199254740991,
              },
              accepted: {
                type: 'boolean',
                description: 'True only after the exact original mutation has a successful result.',
              },
            },
            required: ['owner', 'controller', 'sequence', 'accepted'],
          },
          TextAppearance: {
            type: 'object',
            additionalProperties: false,
            properties: {
              font: {
                $ref: '#/$defs/FontAppearance',
              },
              foreground: {
                $ref: '#/$defs/Color',
              },
            },
            required: ['font', 'foreground'],
          },
          FontAppearance: {
            type: 'object',
            additionalProperties: false,
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
                type: 'number',
                format: 'float',
                description: 'Line height relative to this font size.',
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
            required: ['family', 'fallbacks', 'size', 'lineHeight', 'weight'],
          },
          FontFamily: {
            type: 'string',
            minLength: 1,
            maxLength: 128,
          },
          Color: {
            type: 'integer',
            format: 'uint32',
            minimum: 0,
            description: 'Packed RGBA, including explicit alpha zero.',
          },
        },
      },
      events: {
        observed: {
          type: 'object',
          additionalProperties: false,
          properties: {
            owner: {
              type: 'string',
              format: 'uuid',
            },
            revision: {
              type: 'integer',
              format: 'uint64',
              minimum: 0,
            },
            source: {
              type: 'string',
            },
            controller: {
              type: ['string', 'null'],
              format: 'uuid',
            },
            captured: {
              type: 'boolean',
            },
            batch: {
              anyOf: [
                {
                  $ref: '#/$defs/TerminalInputBatch',
                },
                {
                  type: 'null',
                },
              ],
            },
            error: {
              anyOf: [
                {
                  $ref: '#/$defs/TerminalViewError',
                },
                {
                  type: 'null',
                },
              ],
            },
          },
          required: ['owner', 'revision', 'source', 'captured'],
          title: 'TerminalViewObservation',
          $schema: 'https://json-schema.org/draft/2020-12/schema',
          $defs: {
            TerminalInputBatch: {
              type: 'object',
              additionalProperties: false,
              properties: {
                controller: {
                  type: 'string',
                  format: 'uuid',
                },
                sequence: {
                  type: 'integer',
                  format: 'uint64',
                  minimum: 1,
                  maximum: 9007199254740991,
                },
                actions: {
                  type: 'array',
                  items: {
                    $ref: '#/$defs/RawAction',
                  },
                  maxItems: 64,
                },
                size: {
                  anyOf: [
                    {
                      $ref: '#/$defs/TerminalSize',
                    },
                    {
                      type: 'null',
                    },
                  ],
                },
              },
              required: ['controller', 'sequence', 'actions'],
            },
            RawAction: {
              oneOf: [
                {
                  type: 'object',
                  additionalProperties: false,
                  properties: {
                    text: {
                      type: 'string',
                      minLength: 1,
                      pattern: '^[^\\x00-\\x1f\\x7f-\\x9f]+$',
                    },
                    type: {
                      type: 'string',
                      const: 'text',
                    },
                  },
                  required: ['type', 'text'],
                },
                {
                  type: 'object',
                  additionalProperties: false,
                  properties: {
                    text: {
                      type: 'string',
                      minLength: 1,
                      pattern: '^[^\\x00-\\x08\\x0b\\x0c\\x0e-\\x1f\\x7f-\\x9f]+$',
                    },
                    type: {
                      type: 'string',
                      const: 'paste',
                    },
                  },
                  required: ['type', 'text'],
                },
                {
                  type: 'object',
                  additionalProperties: false,
                  properties: {
                    key: {
                      type: 'string',
                      pattern:
                        '^(?:[ -~]|enter|escape|tab|backspace|arrow_(?:up|down|left|right)|home|end|insert|delete|page_(?:up|down)|f(?:[1-9]|1[0-2]))$',
                    },
                    modifiers: {
                      $ref: '#/$defs/TerminalModifiers',
                      default: [],
                    },
                    type: {
                      type: 'string',
                      const: 'key',
                    },
                  },
                  required: ['type', 'key'],
                },
                {
                  type: 'object',
                  additionalProperties: false,
                  properties: {
                    event: {
                      $ref: '#/$defs/RawEvent',
                    },
                    x: {
                      type: 'integer',
                      format: 'uint64',
                      minimum: 0,
                      maximum: 9007199254740991,
                    },
                    y: {
                      type: 'integer',
                      format: 'uint64',
                      minimum: 0,
                      maximum: 9007199254740991,
                    },
                    button: {
                      $ref: '#/$defs/MouseButton',
                      default: null,
                    },
                    direction: {
                      $ref: '#/$defs/ScrollDirection',
                      default: null,
                    },
                    modifiers: {
                      $ref: '#/$defs/TerminalModifiers',
                      default: [],
                    },
                    type: {
                      type: 'string',
                      const: 'mouse',
                    },
                  },
                  required: ['type', 'event', 'x', 'y'],
                },
              ],
            },
            TerminalModifiers: {
              type: 'array',
              items: {
                $ref: '#/$defs/Modifier',
              },
              uniqueItems: true,
              maxItems: 3,
            },
            Modifier: {
              type: 'string',
              enum: ['ctrl', 'alt', 'shift'],
            },
            RawEvent: {
              type: 'string',
              enum: ['click', 'press', 'release', 'move', 'scroll'],
            },
            MouseButton: {
              type: 'string',
              enum: ['left', 'middle', 'right'],
            },
            ScrollDirection: {
              type: 'string',
              enum: ['up', 'down'],
            },
            TerminalSize: {
              type: 'object',
              additionalProperties: false,
              properties: {
                cols: {
                  type: 'integer',
                  format: 'uint16',
                  minimum: 2,
                  maximum: 240,
                },
                rows: {
                  type: 'integer',
                  format: 'uint16',
                  minimum: 1,
                  maximum: 100,
                },
              },
              required: ['cols', 'rows'],
              description:
                'Shared by native PTY allocation, input validation and screen rendering.',
            },
            TerminalViewError: {
              type: 'string',
              enum: ['input_full', 'input_failed', 'budget_exhausted'],
            },
          },
        },
      },
      captures: {},
    },
  },
  selectionMenu: {
    name: 'maka.ui.SelectionMenu',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'SelectionMenuProps',
        type: 'object',
        properties: {
          appearance: {
            $ref: '#/$defs/MenuAppearance',
          },
          labels: {
            $ref: '#/$defs/SelectionMenuLabels',
          },
          disabled: {
            type: 'boolean',
          },
        },
        additionalProperties: false,
        required: ['appearance', 'labels', 'disabled'],
        $defs: {
          MenuAppearance: {
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
          SelectionMenuLabels: {
            type: 'object',
            properties: {
              title: {
                type: 'string',
                minLength: 1,
                maxLength: 512,
              },
              copy: {
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
            required: ['title', 'copy', 'selectAll'],
          },
        },
      },
      events: {},
      captures: {},
    },
  },
  rows: {
    name: 'maka.ui.Rows',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'RowsProps',
        type: 'object',
        properties: {
          terminal: {
            anyOf: [
              {
                $ref: '#/$defs/TerminalPalette',
              },
              {
                type: 'null',
              },
            ],
            default: null,
          },
          revision: {
            type: 'integer',
            format: 'uint64',
            minimum: 0,
          },
          entry: {
            description:
              'Select this row on keyboard entry. None retains the current selection,\nor selects the first row when there is no current selection.',
            type: ['string', 'null'],
          },
          rows: {
            type: 'array',
            items: {
              $ref: '#/$defs/Row',
            },
            maxItems: 64,
          },
          shortcuts: {
            type: 'array',
            items: {
              $ref: '#/$defs/Shortcut',
            },
            maxItems: 32,
          },
          maxRows: {
            type: 'integer',
            format: 'uint16',
            minimum: 1,
            maximum: 64,
          },
          compactBelow: {
            description:
              "Optional smaller row cap below the nearest declared layout viewport's\nheight, in cells; defaults to the surface when no viewport is declared.",
            anyOf: [
              {
                $ref: '#/$defs/CompactRows',
              },
              {
                type: 'null',
              },
            ],
            default: null,
          },
          tabIndex: {
            type: 'integer',
            format: 'int16',
            minimum: -1,
            maximum: 32767,
          },
          foreground: {
            $ref: '#/$defs/Color',
          },
          muted: {
            $ref: '#/$defs/Color',
          },
          selected: {
            $ref: '#/$defs/Color',
          },
          hovered: {
            $ref: '#/$defs/Color',
          },
          scrollbar: {
            $ref: '#/$defs/RowScrollbar',
          },
        },
        additionalProperties: false,
        required: [
          'revision',
          'rows',
          'shortcuts',
          'maxRows',
          'tabIndex',
          'foreground',
          'muted',
          'selected',
          'hovered',
          'scrollbar',
        ],
        $defs: {
          TerminalPalette: {
            description:
              'Semantic terminal colors for rich text and editors, supplied by the product.',
            type: 'object',
            properties: {
              background: {
                $ref: '#/$defs/TerminalColor',
              },
              foreground: {
                $ref: '#/$defs/TerminalColor',
              },
              surface: {
                $ref: '#/$defs/TerminalColor',
              },
              accent: {
                $ref: '#/$defs/TerminalColor',
              },
              thinking: {
                $ref: '#/$defs/TerminalColor',
              },
              success: {
                $ref: '#/$defs/TerminalColor',
              },
              warning: {
                $ref: '#/$defs/TerminalColor',
              },
              error: {
                $ref: '#/$defs/TerminalColor',
              },
              muted: {
                $ref: '#/$defs/TerminalColor',
              },
              subtle: {
                $ref: '#/$defs/TerminalColor',
              },
              border: {
                $ref: '#/$defs/TerminalColor',
              },
              scrollbar: {
                $ref: '#/$defs/TerminalColor',
              },
              selection: {
                $ref: '#/$defs/TerminalColor',
              },
              selectionText: {
                $ref: '#/$defs/TerminalColor',
              },
              search: {
                $ref: '#/$defs/TerminalColor',
              },
              searchActive: {
                $ref: '#/$defs/TerminalColor',
              },
              searchText: {
                $ref: '#/$defs/TerminalColor',
              },
              syntax: {
                type: 'array',
                items: {
                  $ref: '#/$defs/TerminalColor',
                },
                minItems: 7,
                maxItems: 7,
              },
              terminal: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: [
              'background',
              'foreground',
              'surface',
              'accent',
              'thinking',
              'success',
              'warning',
              'error',
              'muted',
              'subtle',
              'border',
              'scrollbar',
              'selection',
              'selectionText',
              'search',
              'searchActive',
              'searchText',
              'syntax',
              'terminal',
            ],
          },
          TerminalColor: {
            description:
              "Terminal colors retain the user's ANSI palette and default background.\nNumeric values are RGB; named values are emitted as terminal attributes.",
            anyOf: [
              {
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 16777215,
              },
              {
                $ref: '#/$defs/TerminalNamedColor',
              },
            ],
          },
          TerminalNamedColor: {
            type: 'string',
            enum: [
              'reset',
              'black',
              'red',
              'green',
              'yellow',
              'blue',
              'magenta',
              'cyan',
              'gray',
              'dark_gray',
              'light_red',
              'light_green',
              'light_yellow',
              'light_blue',
              'light_magenta',
              'light_cyan',
              'white',
            ],
          },
          Row: {
            type: 'object',
            properties: {
              key: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              text: {
                type: 'string',
                maxLength: 1024,
              },
              actions: {
                type: 'array',
                items: {
                  $ref: '#/$defs/RowAction',
                },
                maxItems: 8,
              },
            },
            additionalProperties: false,
            required: ['key', 'text', 'actions'],
          },
          RowAction: {
            type: 'object',
            properties: {
              key: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              label: {
                type: 'string',
                maxLength: 64,
              },
              enabled: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: ['key', 'label', 'enabled'],
          },
          Shortcut: {
            type: 'object',
            properties: {
              key: {
                type: 'string',
                pattern:
                  '^([a-z0-9]|escape|enter|tab|delete|space|up|down|left|right|home|end|pageup|pagedown|f([1-9]|1[0-2]))$',
              },
              modifiers: {
                type: 'array',
                items: {
                  $ref: '#/$defs/Modifier',
                },
                maxItems: 4,
              },
              input: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              enabled: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: ['key', 'modifiers', 'input', 'enabled'],
          },
          Modifier: {
            type: 'string',
            enum: ['control', 'primary', 'alt', 'shift', 'super'],
          },
          CompactRows: {
            type: 'object',
            properties: {
              height: {
                type: 'integer',
                format: 'uint16',
                minimum: 1,
                maximum: 65535,
              },
              maxRows: {
                type: 'integer',
                format: 'uint16',
                minimum: 1,
                maximum: 64,
              },
            },
            additionalProperties: false,
            required: ['height', 'maxRows'],
          },
          Color: {
            description: 'Packed RGBA, including explicit alpha zero.',
            type: 'integer',
            format: 'uint32',
            minimum: 0,
          },
          RowScrollbar: {
            type: 'object',
            properties: {
              thumb: {
                type: 'string',
                minLength: 1,
                maxLength: 1,
              },
              track: {
                type: 'string',
                minLength: 1,
                maxLength: 1,
              },
              thumbColor: {
                $ref: '#/$defs/Color',
              },
              trackColor: {
                $ref: '#/$defs/Color',
              },
            },
            additionalProperties: false,
            required: ['thumb', 'track', 'thumbColor', 'trackColor'],
          },
        },
      },
      events: {
        activated: {
          $schema: 'https://json-schema.org/draft/2020-12/schema',
          title: 'RowActivated',
          type: 'object',
          properties: {
            revision: {
              type: 'integer',
              format: 'uint64',
              minimum: 0,
            },
            row: {
              type: 'string',
            },
            action: {
              type: 'string',
            },
          },
          additionalProperties: false,
          required: ['revision', 'row', 'action'],
        },
      },
      captures: {},
    },
  },
  valueCheckpoint: {
    name: 'maka.ui.ValueCheckpoint',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'ValueCheckpointProps',
        type: 'object',
        properties: {
          key: {
            type: 'string',
            minLength: 1,
            maxLength: 512,
          },
          format: {
            type: 'string',
            minLength: 1,
            maxLength: 128,
          },
          command: {
            anyOf: [
              {
                $ref: '#/$defs/ValueCheckpointCommand',
              },
              {
                type: 'null',
              },
            ],
          },
        },
        additionalProperties: false,
        required: ['key', 'format'],
        $defs: {
          ValueCheckpointCommand: {
            oneOf: [
              {
                type: 'object',
                properties: {
                  sequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  kind: {
                    type: 'string',
                    const: 'read',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'sequence'],
              },
              {
                type: 'object',
                properties: {
                  sequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  expectedRevision: {
                    type: ['integer', 'null'],
                    format: 'uint64',
                    minimum: 0,
                  },
                  value: true,
                  kind: {
                    type: 'string',
                    const: 'write',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'sequence', 'value'],
              },
            ],
          },
        },
      },
      events: {
        changed: {
          $schema: 'https://json-schema.org/draft/2020-12/schema',
          title: 'ValueCheckpointObservation',
          type: 'object',
          properties: {
            sequence: {
              type: 'integer',
              format: 'uint64',
              minimum: 0,
            },
            state: {
              $ref: '#/$defs/ValueCheckpointState',
            },
          },
          additionalProperties: false,
          required: ['sequence', 'state'],
          $defs: {
            ValueCheckpointState: {
              oneOf: [
                {
                  type: 'object',
                  properties: {
                    kind: {
                      type: 'string',
                      const: 'loading',
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
                      const: 'saving',
                    },
                  },
                  required: ['kind'],
                  additionalProperties: false,
                },
                {
                  type: 'object',
                  properties: {
                    revision: {
                      type: ['integer', 'null'],
                      format: 'uint64',
                      minimum: 0,
                    },
                    value: true,
                    kind: {
                      type: 'string',
                      const: 'ready',
                    },
                  },
                  additionalProperties: false,
                  required: ['kind'],
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
      },
      captures: {},
    },
  },
  locale: {
    name: 'maka.ui.Locale',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'value',
      schema: {
        type: 'string',
        minLength: 1,
        maxLength: 128,
      },
    },
  },
  textFile: {
    name: 'maka.ui.TextFile',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'TextFileProps',
        type: 'object',
        properties: {
          command: {
            anyOf: [
              {
                $ref: '#/$defs/TextFileCommand',
              },
              {
                type: 'null',
              },
            ],
          },
        },
        additionalProperties: false,
        $defs: {
          TextFileCommand: {
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
              operation: {
                $ref: '#/$defs/TextFileOperation',
              },
            },
            additionalProperties: false,
            required: ['owner', 'sequence', 'operation'],
          },
          TextFileOperation: {
            oneOf: [
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'read',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
              {
                type: 'object',
                properties: {
                  expectedDigest: {
                    type: ['string', 'null'],
                  },
                  text: {
                    type: 'string',
                    maxLength: 16384,
                  },
                  kind: {
                    type: 'string',
                    const: 'write',
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
          title: 'TextFileObservation',
          type: 'object',
          properties: {
            owner: {
              type: 'string',
            },
            path: {
              type: ['string', 'null'],
            },
            sequence: {
              type: 'integer',
              format: 'uint64',
              minimum: 0,
            },
            state: {
              $ref: '#/$defs/TextFileState',
            },
          },
          additionalProperties: false,
          required: ['owner', 'sequence', 'state'],
          $defs: {
            TextFileState: {
              oneOf: [
                {
                  type: 'object',
                  properties: {
                    kind: {
                      type: 'string',
                      const: 'unavailable',
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
                      const: 'idle',
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
                      const: 'loading',
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
                      const: 'saving',
                    },
                  },
                  required: ['kind'],
                  additionalProperties: false,
                },
                {
                  type: 'object',
                  properties: {
                    digest: {
                      type: ['string', 'null'],
                    },
                    text: {
                      type: ['string', 'null'],
                    },
                    kind: {
                      type: 'string',
                      const: 'ready',
                    },
                  },
                  additionalProperties: false,
                  required: ['kind'],
                },
                {
                  type: 'object',
                  properties: {
                    error: {
                      $ref: '#/$defs/TextFileError',
                    },
                    kind: {
                      type: 'string',
                      const: 'failed',
                    },
                  },
                  additionalProperties: false,
                  required: ['kind', 'error'],
                },
              ],
            },
            TextFileError: {
              type: 'string',
              enum: [
                'read_failed',
                'write_failed',
                'invalid_file',
                'too_large',
                'invalid_utf8',
                'conflict',
                'worker_failed',
              ],
            },
          },
        },
      },
      captures: {},
    },
  },
  morphMark: {
    name: 'maka.ui.MorphMark',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'MorphMarkProps',
        type: 'object',
        properties: {
          background: {
            $ref: '#/$defs/TerminalColor',
          },
          foreground: {
            $ref: '#/$defs/TerminalColor',
          },
          motion: {
            type: 'boolean',
          },
          ascii: {
            type: 'boolean',
          },
        },
        additionalProperties: false,
        required: ['background', 'foreground', 'motion', 'ascii'],
        $defs: {
          TerminalColor: {
            description:
              "Terminal colors retain the user's ANSI palette and default background.\nNumeric values are RGB; named values are emitted as terminal attributes.",
            anyOf: [
              {
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 16777215,
              },
              {
                $ref: '#/$defs/TerminalNamedColor',
              },
            ],
          },
          TerminalNamedColor: {
            type: 'string',
            enum: [
              'reset',
              'black',
              'red',
              'green',
              'yellow',
              'blue',
              'magenta',
              'cyan',
              'gray',
              'dark_gray',
              'light_red',
              'light_green',
              'light_yellow',
              'light_blue',
              'light_magenta',
              'light_cyan',
              'white',
            ],
          },
        },
      },
      events: {},
      captures: {},
    },
  },
  readingArea: {
    name: 'maka.ui.ReadingArea',
    version: 1,
    transport: 'local',
    interface: {
      kind: 'component',
      props: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        title: 'ReadingProps',
        type: 'object',
        properties: {
          content: {
            description:
              "The presenter's scene language; each requires a scrolling column Stack.\nIts scroll policy and layout are the only presentation authority.",
          },
          command: {
            description:
              'Omission retains an accepted command; numbers increase per original mount.',
            anyOf: [
              {
                $ref: '#/$defs/ReadingCommand',
              },
              {
                type: 'null',
              },
            ],
          },
        },
        additionalProperties: false,
        required: ['content'],
        $defs: {
          ReadingCommand: {
            type: 'object',
            properties: {
              sequence: {
                type: 'integer',
                format: 'uint64',
                minimum: 1,
              },
              target: {
                $ref: '#/$defs/ReadingTarget',
              },
            },
            additionalProperties: false,
            required: ['sequence', 'target'],
          },
          ReadingTarget: {
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
                  kind: {
                    type: 'string',
                    const: 'end',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
              {
                description:
                  'Find over native readable descendants, including folded slots. SourceText uses\noriginal literal bytes; Markdown uses its width-independent rendered text.',
                type: 'object',
                properties: {
                  query: {
                    type: 'string',
                    maxLength: 512,
                  },
                  step: {
                    $ref: '#/$defs/FindStep',
                  },
                  kind: {
                    type: 'string',
                    const: 'find',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'query', 'step'],
              },
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'clear_find',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
              {
                description: 'Move the original terminal viewport by physical rows.',
                type: 'object',
                properties: {
                  rows: {
                    type: 'integer',
                    format: 'int32',
                    minimum: -65536,
                    maximum: 65536,
                  },
                  kind: {
                    type: 'string',
                    const: 'scroll_rows',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'rows'],
              },
              {
                description:
                  "Reveal a UTF-8 byte in the descendant's native readable text, as returned by Find.",
                type: 'object',
                properties: {
                  path: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                    minItems: 1,
                    maxItems: 32,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint32',
                    minimum: 0,
                  },
                  find_sequence: {
                    description:
                      'If supplied, reveal only while this accepted find retains its original physical intent.',
                    type: ['integer', 'null'],
                    format: 'uint64',
                    minimum: 0,
                    default: null,
                  },
                  kind: {
                    type: 'string',
                    const: 'reveal_source',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'path', 'offset'],
              },
              {
                description:
                  'TUI: focus and minimally reveal an enabled scene Item. Settlement waits\nfor its successful presentation; this never submits the item.',
                type: 'object',
                properties: {
                  key: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 128,
                  },
                  kind: {
                    type: 'string',
                    const: 'focus_item',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'key'],
              },
              {
                type: 'object',
                properties: {
                  key: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 128,
                  },
                  kind: {
                    type: 'string',
                    const: 'reveal_child',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'key'],
              },
            ],
          },
          FindStep: {
            type: 'string',
            enum: ['first', 'next', 'previous'],
          },
        },
      },
      events: {
        input: {
          $schema: 'https://json-schema.org/draft/2020-12/schema',
          title: 'ComponentInput',
          description:
            'Input origin is assigned by the application, never by plugin event payloads.',
          oneOf: [
            {
              type: 'object',
              properties: {
                key: {
                  type: 'string',
                },
                event: true,
                kind: {
                  type: 'string',
                  const: 'raw',
                },
              },
              additionalProperties: false,
              required: ['kind', 'key', 'event'],
            },
            {
              type: 'object',
              properties: {
                key: {
                  type: 'string',
                },
                event: true,
                kind: {
                  type: 'string',
                  const: 'action',
                },
              },
              additionalProperties: false,
              required: ['kind', 'key', 'event'],
            },
            {
              type: 'object',
              properties: {
                key: {
                  type: 'string',
                },
                child: {
                  type: 'string',
                },
                name: {
                  type: 'string',
                },
                event: true,
                kind: {
                  type: 'string',
                  const: 'component',
                },
              },
              additionalProperties: false,
              required: ['kind', 'key', 'child', 'name', 'event'],
            },
          ],
        },
        observed: {
          $schema: 'https://json-schema.org/draft/2020-12/schema',
          title: 'ReadingObservation',
          type: 'object',
          properties: {
            readingRevision: {
              description: 'Immediate native intent; can be newer than the painted viewport.',
              type: 'integer',
              format: 'uint64',
              minimum: 0,
            },
            viewport: {
              anyOf: [
                {
                  $ref: '#/$defs/ReadingViewport',
                },
                {
                  type: 'null',
                },
              ],
            },
            scroll: {
              anyOf: [
                {
                  $ref: '#/$defs/ReadingScroll',
                },
                {
                  type: 'null',
                },
              ],
              default: null,
            },
            selectedChildKey: {
              description:
                'Direct content child containing the native selection focus. Renderers\nwithout this observation return null. This key conveys no read authority.',
              type: ['string', 'null'],
              maxLength: 256,
            },
            pendingCommand: {
              type: ['integer', 'null'],
              format: 'uint64',
              minimum: 0,
            },
            receipt: {
              description: 'Only the most recently settled command is retained, without a ledger.',
              anyOf: [
                {
                  $ref: '#/$defs/ReadingReceipt',
                },
                {
                  type: 'null',
                },
              ],
            },
            find: {
              anyOf: [
                {
                  $ref: '#/$defs/ReadingFind',
                },
                {
                  type: 'null',
                },
              ],
            },
          },
          additionalProperties: false,
          required: ['readingRevision'],
          $defs: {
            ReadingViewport: {
              type: 'object',
              properties: {
                readingRevision: {
                  description: 'Explicit reading intent used by the actually painted children.',
                  type: 'integer',
                  format: 'uint64',
                  minimum: 0,
                },
                atStart: {
                  type: 'boolean',
                  default: false,
                },
                atEnd: {
                  type: 'boolean',
                },
                visibleChildKeys: {
                  type: 'array',
                  items: {
                    type: 'string',
                  },
                  maxItems: 4096,
                },
              },
              additionalProperties: false,
              required: ['readingRevision', 'atEnd', 'visibleChildKeys'],
            },
            ReadingScroll: {
              description:
                'Most recent physical scroll. Layout changes and programmatic landings do not\ncreate a new request, so a product can load adjacent content without loops.',
              type: 'object',
              properties: {
                readingRevision: {
                  type: 'integer',
                  format: 'uint64',
                  minimum: 0,
                },
                direction: {
                  $ref: '#/$defs/ReadingScrollDirection',
                },
              },
              additionalProperties: false,
              required: ['readingRevision', 'direction'],
            },
            ReadingScrollDirection: {
              type: 'string',
              enum: ['up', 'down'],
            },
            ReadingReceipt: {
              type: 'object',
              properties: {
                sequence: {
                  type: 'integer',
                  format: 'uint64',
                  minimum: 0,
                },
                status: {
                  $ref: '#/$defs/ReadingStatus',
                },
              },
              additionalProperties: false,
              required: ['sequence', 'status'],
            },
            ReadingStatus: {
              type: 'string',
              enum: ['applied', 'cancelled', 'rejected'],
            },
            ReadingFind: {
              type: 'object',
              properties: {
                sequence: {
                  type: 'integer',
                  format: 'uint64',
                  minimum: 0,
                },
                count: {
                  type: 'integer',
                  format: 'uint32',
                  minimum: 0,
                  maximum: 4096,
                },
                limited: {
                  type: 'boolean',
                },
                index: {
                  description: 'One-based current match, or null when no match remains.',
                  type: ['integer', 'null'],
                  format: 'uint32',
                  minimum: 0,
                },
                active: {
                  anyOf: [
                    {
                      $ref: '#/$defs/ReadingMatch',
                    },
                    {
                      type: 'null',
                    },
                  ],
                },
              },
              additionalProperties: false,
              required: ['sequence', 'count', 'limited'],
            },
            ReadingMatch: {
              type: 'object',
              properties: {
                path: {
                  type: 'array',
                  items: {
                    type: 'string',
                  },
                  minItems: 1,
                  maxItems: 32,
                },
                offset: {
                  type: 'integer',
                  format: 'uint32',
                  minimum: 0,
                },
              },
              additionalProperties: false,
              required: ['path', 'offset'],
            },
          },
        },
      },
      captures: {
        tail: {
          input: {
            type: 'null',
          },
          output: {
            kind: 'continuation',
          },
        },
      },
    },
  },
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
          terminalForeground: {
            anyOf: [
              {
                $ref: '#/$defs/TerminalColor',
              },
              {
                type: 'null',
              },
            ],
            default: null,
          },
          elapsed: {
            anyOf: [
              {
                $ref: '#/$defs/Elapsed',
              },
              {
                type: 'null',
              },
            ],
            default: null,
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
          TerminalColor: {
            description:
              "Terminal colors retain the user's ANSI palette and default background.\nNumeric values are RGB; named values are emitted as terminal attributes.",
            anyOf: [
              {
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 16777215,
              },
              {
                $ref: '#/$defs/TerminalNamedColor',
              },
            ],
          },
          TerminalNamedColor: {
            type: 'string',
            enum: [
              'reset',
              'black',
              'red',
              'green',
              'yellow',
              'blue',
              'magenta',
              'cyan',
              'gray',
              'dark_gray',
              'light_red',
              'light_green',
              'light_yellow',
              'light_blue',
              'light_magenta',
              'light_cyan',
              'white',
            ],
          },
          Elapsed: {
            type: 'object',
            properties: {
              from: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 9007199254740991,
              },
              to: {
                type: ['integer', 'null'],
                format: 'uint64',
                minimum: 0,
              },
            },
            additionalProperties: false,
            required: ['from', 'to'],
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
          terminalRows: {
            description:
              'Preferred terminal text rows, excluding native padding. A constrained\nviewport may shrink further. Desktop does not use terminal row sizing.',
            $ref: '#/$defs/TerminalTextareaRows',
            default: {
              min: 3,
              max: 8,
              maxViewportFraction: null,
            },
          },
          submit: {
            description: 'None preserves native multiline editing without a submit shortcut.',
            anyOf: [
              {
                $ref: '#/$defs/SubmitKey',
              },
              {
                type: 'null',
              },
            ],
            default: null,
          },
          tabIndex: {
            type: 'integer',
            format: 'int16',
            minimum: -1,
            maximum: 32767,
          },
          autoFocus: {
            description:
              'Offer native focus once when this original input becomes visible and\nenabled. Stable updates and viewport scrolling do not repeat the offer.',
            type: 'boolean',
            default: false,
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
          filePaste: {
            description:
              'Opt-in local file-path paste. The context is returned unchanged; it is\ncorrelation, never a filesystem path or an authorization grant.',
            anyOf: [
              {
                $ref: '#/$defs/FilePastePolicy',
              },
              {
                type: 'null',
              },
            ],
            default: null,
          },
          fileCommand: {
            anyOf: [
              {
                $ref: '#/$defs/FilePasteCommand',
              },
              {
                type: 'null',
              },
            ],
            default: null,
          },
          persistence: {
            anyOf: [
              {
                $ref: '#/$defs/EditorPersistence',
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
              ascii: {
                type: 'boolean',
                default: false,
              },
              terminal: {
                anyOf: [
                  {
                    $ref: '#/$defs/TerminalPalette',
                  },
                  {
                    type: 'null',
                  },
                ],
                default: null,
              },
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
                description:
                  'In TUI inputs, a positive width draws a one-cell border; zero has no inset.',
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
          TerminalPalette: {
            description:
              'Semantic terminal colors for rich text and editors, supplied by the product.',
            type: 'object',
            properties: {
              background: {
                $ref: '#/$defs/TerminalColor',
              },
              foreground: {
                $ref: '#/$defs/TerminalColor',
              },
              surface: {
                $ref: '#/$defs/TerminalColor',
              },
              accent: {
                $ref: '#/$defs/TerminalColor',
              },
              thinking: {
                $ref: '#/$defs/TerminalColor',
              },
              success: {
                $ref: '#/$defs/TerminalColor',
              },
              warning: {
                $ref: '#/$defs/TerminalColor',
              },
              error: {
                $ref: '#/$defs/TerminalColor',
              },
              muted: {
                $ref: '#/$defs/TerminalColor',
              },
              subtle: {
                $ref: '#/$defs/TerminalColor',
              },
              border: {
                $ref: '#/$defs/TerminalColor',
              },
              scrollbar: {
                $ref: '#/$defs/TerminalColor',
              },
              selection: {
                $ref: '#/$defs/TerminalColor',
              },
              selectionText: {
                $ref: '#/$defs/TerminalColor',
              },
              search: {
                $ref: '#/$defs/TerminalColor',
              },
              searchActive: {
                $ref: '#/$defs/TerminalColor',
              },
              searchText: {
                $ref: '#/$defs/TerminalColor',
              },
              syntax: {
                type: 'array',
                items: {
                  $ref: '#/$defs/TerminalColor',
                },
                minItems: 7,
                maxItems: 7,
              },
              terminal: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: [
              'background',
              'foreground',
              'surface',
              'accent',
              'thinking',
              'success',
              'warning',
              'error',
              'muted',
              'subtle',
              'border',
              'scrollbar',
              'selection',
              'selectionText',
              'search',
              'searchActive',
              'searchText',
              'syntax',
              'terminal',
            ],
          },
          TerminalColor: {
            description:
              "Terminal colors retain the user's ANSI palette and default background.\nNumeric values are RGB; named values are emitted as terminal attributes.",
            anyOf: [
              {
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 16777215,
              },
              {
                $ref: '#/$defs/TerminalNamedColor',
              },
            ],
          },
          TerminalNamedColor: {
            type: 'string',
            enum: [
              'reset',
              'black',
              'red',
              'green',
              'yellow',
              'blue',
              'magenta',
              'cyan',
              'gray',
              'dark_gray',
              'light_red',
              'light_green',
              'light_yellow',
              'light_blue',
              'light_magenta',
              'light_cyan',
              'white',
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
                $ref: '#/$defs/MenuAppearance',
              },
              labels: {
                $ref: '#/$defs/TextareaMenuLabels',
              },
            },
            additionalProperties: false,
            required: ['appearance', 'labels'],
          },
          MenuAppearance: {
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
          TerminalTextareaRows: {
            type: 'object',
            properties: {
              min: {
                type: 'integer',
                format: 'uint16',
                minimum: 1,
                maximum: 65535,
              },
              max: {
                type: 'integer',
                format: 'uint16',
                minimum: 1,
                maximum: 65535,
              },
              maxViewportFraction: {
                description:
                  "Optional cap relative to the nearest declared layout viewport's height,\nor the surface when none is declared. The minimum still applies.",
                type: ['number', 'null'],
                format: 'double',
                minimum: 0,
                maximum: 1,
                default: null,
              },
            },
            additionalProperties: false,
            required: ['min', 'max'],
          },
          SubmitKey: {
            description:
              'Product-selected native submit gesture. PrimaryEnter uses the platform\naccelerator on Desktop and Control+Enter in a terminal.',
            type: 'string',
            enum: ['enter', 'primary_enter', 'control_enter'],
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
              {
                description:
                  'Continue initialization after Reset, without creating undo history.\nEach chunk is bounded; the same native owner retains the full buffer.',
                type: 'object',
                properties: {
                  text: {
                    type: 'string',
                    maxLength: 65536,
                  },
                  kind: {
                    type: 'string',
                    const: 'append',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'text'],
              },
            ],
          },
          FilePastePolicy: {
            type: 'object',
            properties: {
              context: {
                type: 'string',
                minLength: 1,
                maxLength: 512,
              },
              maxFiles: {
                description: 'Zero still permits clipboard text, but no new file attachments.',
                type: 'integer',
                format: 'uint',
                minimum: 0,
                maximum: 8,
              },
            },
            additionalProperties: false,
            required: ['context', 'maxFiles'],
          },
          FilePasteCommand: {
            type: 'object',
            properties: {
              selection: {
                $ref: '#/$defs/FileCapture',
              },
              action: {
                $ref: '#/$defs/FilePasteAction',
              },
            },
            additionalProperties: false,
            required: ['selection', 'action'],
          },
          FileCapture: {
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
            },
            additionalProperties: false,
            required: ['owner', 'revision'],
          },
          FilePasteAction: {
            oneOf: [
              {
                description:
                  'Release this ready/failed item and continue the original physical paste.',
                type: 'string',
                const: 'next',
              },
              {
                description: 'Retry the original failed path, without a new path from the guest.',
                type: 'string',
                const: 'retry',
              },
              {
                description:
                  'Discard this batch; already captured resources retain their own custody.',
                type: 'string',
                const: 'cancel',
              },
            ],
          },
          EditorPersistence: {
            type: 'object',
            properties: {
              key: {
                type: 'string',
                minLength: 1,
                maxLength: 512,
              },
              format: {
                type: 'string',
                minLength: 1,
                maxLength: 96,
              },
              dataSource: true,
              metadata: true,
              restore: {
                anyOf: [
                  {
                    $ref: '#/$defs/EditorRestoration',
                  },
                  {
                    type: 'null',
                  },
                ],
                default: null,
              },
              retry: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 9007199254740991,
                default: 0,
              },
            },
            additionalProperties: false,
            required: ['key', 'format', 'dataSource', 'metadata'],
          },
          EditorRestoration: {
            type: 'object',
            properties: {
              owner: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              version: {
                type: 'integer',
                format: 'uint64',
                minimum: 1,
                maximum: 9007199254740991,
              },
              clearText: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: ['owner', 'version', 'clearText'],
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
              enum: [
                'persistence',
                'owner',
                'representation',
                'revision',
                'composition',
                'sequence',
              ],
            },
          },
        },
        checkpoint: {
          $schema: 'https://json-schema.org/draft/2020-12/schema',
          title: 'EditorCheckpointObservation',
          type: 'object',
          properties: {
            owner: {
              type: 'string',
            },
            state: {
              $ref: '#/$defs/EditorCheckpointState',
            },
          },
          additionalProperties: false,
          required: ['owner', 'state'],
          $defs: {
            EditorCheckpointState: {
              oneOf: [
                {
                  type: 'object',
                  properties: {
                    kind: {
                      type: 'string',
                      const: 'loading',
                    },
                  },
                  required: ['kind'],
                  additionalProperties: false,
                },
                {
                  type: 'object',
                  properties: {
                    version: {
                      type: 'integer',
                      format: 'uint64',
                      minimum: 0,
                    },
                    snapshot: {
                      $ref: '#/$defs/EditorSnapshot',
                    },
                    metadata: true,
                    kind: {
                      type: 'string',
                      const: 'restore',
                    },
                  },
                  additionalProperties: false,
                  required: ['kind', 'version', 'snapshot', 'metadata'],
                },
                {
                  type: 'object',
                  properties: {
                    saved: {
                      type: 'boolean',
                    },
                    initialized: {
                      type: 'boolean',
                    },
                    kind: {
                      type: 'string',
                      const: 'ready',
                    },
                  },
                  additionalProperties: false,
                  required: ['kind', 'saved', 'initialized'],
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
          },
        },
        files: {
          $schema: 'https://json-schema.org/draft/2020-12/schema',
          title: 'FilePasteObservation',
          type: 'object',
          properties: {
            context: {
              type: 'string',
              minLength: 1,
              maxLength: 512,
            },
            selection: {
              $ref: '#/$defs/FileObservation',
            },
            remaining: {
              description: 'Includes the currently reading, ready or failed item.',
              type: 'integer',
              format: 'uint',
              minimum: 0,
              maximum: 8,
            },
            retryable: {
              description:
                'A failed original path can be reread. Clipboard reads require a new gesture.',
              type: 'boolean',
            },
          },
          additionalProperties: false,
          required: ['context', 'selection', 'remaining', 'retryable'],
          $defs: {
            FileObservation: {
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
                phase: {
                  $ref: '#/$defs/FilePhase',
                },
                file: {
                  anyOf: [
                    {
                      $ref: '#/$defs/FileDescriptor',
                    },
                    {
                      type: 'null',
                    },
                  ],
                },
                error: {
                  anyOf: [
                    {
                      $ref: '#/$defs/FileError',
                    },
                    {
                      type: 'null',
                    },
                  ],
                },
              },
              additionalProperties: false,
              required: ['owner', 'revision', 'phase'],
            },
            FilePhase: {
              type: 'string',
              enum: ['empty', 'choosing', 'ready', 'failed'],
            },
            FileDescriptor: {
              type: 'object',
              properties: {
                name: {
                  type: 'string',
                  maxLength: 512,
                },
                bytes: {
                  type: 'integer',
                  format: 'uint',
                  minimum: 0,
                },
                digest: {
                  type: 'string',
                  minLength: 71,
                  maxLength: 71,
                },
              },
              additionalProperties: false,
              required: ['name', 'bytes', 'digest'],
            },
            FileError: {
              type: 'string',
              enum: [
                'invalid_file',
                'read_failed',
                'limit_exceeded',
                'budget_exhausted',
                'worker_failed',
                'clipboard_unavailable',
                'paste_rejected',
              ],
            },
          },
        },
        submitted: {
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
              enum: [
                'persistence',
                'owner',
                'representation',
                'revision',
                'composition',
                'sequence',
              ],
            },
          },
        },
      },
      captures: {
        file: {
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'FileCapture',
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
            },
            additionalProperties: false,
            required: ['owner', 'revision'],
          },
          output: {
            kind: 'resource',
            resource: {
              name: 'maka.bytes',
              version: 1,
            },
          },
        },
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
          output: {
            kind: 'resource',
            resource: {
              name: 'maka.text.utf8',
              version: 1,
            },
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
          autoFocus: {
            description:
              'Offer native focus once when this original input becomes visible and\nenabled. Stable updates and viewport scrolling do not repeat the offer.',
            type: 'boolean',
            default: false,
          },
          maxBytes: {
            description: 'UTF-8 byte capacity, fixed for this original native editor mount.',
            type: 'integer',
            format: 'uint32',
            minimum: 1,
            maximum: 65536,
            default: 65536,
          },
          terminalPaste: {
            description: 'Terminal bracketed-paste policy. Other native insertion remains literal.',
            $ref: '#/$defs/TerminalPaste',
            default: 'reject_controls',
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
          submitCopy: {
            description:
              'Clipboard default for physical Enter at this original editor revision.\nProgrammatic submitted events never execute this native default.',
            anyOf: [
              {
                $ref: '#/$defs/SubmitCopy',
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
              ascii: {
                type: 'boolean',
                default: false,
              },
              terminal: {
                anyOf: [
                  {
                    $ref: '#/$defs/TerminalPalette',
                  },
                  {
                    type: 'null',
                  },
                ],
                default: null,
              },
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
                description:
                  'In TUI inputs, a positive width draws a one-cell border; zero has no inset.',
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
          TerminalPalette: {
            description:
              'Semantic terminal colors for rich text and editors, supplied by the product.',
            type: 'object',
            properties: {
              background: {
                $ref: '#/$defs/TerminalColor',
              },
              foreground: {
                $ref: '#/$defs/TerminalColor',
              },
              surface: {
                $ref: '#/$defs/TerminalColor',
              },
              accent: {
                $ref: '#/$defs/TerminalColor',
              },
              thinking: {
                $ref: '#/$defs/TerminalColor',
              },
              success: {
                $ref: '#/$defs/TerminalColor',
              },
              warning: {
                $ref: '#/$defs/TerminalColor',
              },
              error: {
                $ref: '#/$defs/TerminalColor',
              },
              muted: {
                $ref: '#/$defs/TerminalColor',
              },
              subtle: {
                $ref: '#/$defs/TerminalColor',
              },
              border: {
                $ref: '#/$defs/TerminalColor',
              },
              scrollbar: {
                $ref: '#/$defs/TerminalColor',
              },
              selection: {
                $ref: '#/$defs/TerminalColor',
              },
              selectionText: {
                $ref: '#/$defs/TerminalColor',
              },
              search: {
                $ref: '#/$defs/TerminalColor',
              },
              searchActive: {
                $ref: '#/$defs/TerminalColor',
              },
              searchText: {
                $ref: '#/$defs/TerminalColor',
              },
              syntax: {
                type: 'array',
                items: {
                  $ref: '#/$defs/TerminalColor',
                },
                minItems: 7,
                maxItems: 7,
              },
              terminal: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: [
              'background',
              'foreground',
              'surface',
              'accent',
              'thinking',
              'success',
              'warning',
              'error',
              'muted',
              'subtle',
              'border',
              'scrollbar',
              'selection',
              'selectionText',
              'search',
              'searchActive',
              'searchText',
              'syntax',
              'terminal',
            ],
          },
          TerminalColor: {
            description:
              "Terminal colors retain the user's ANSI palette and default background.\nNumeric values are RGB; named values are emitted as terminal attributes.",
            anyOf: [
              {
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 16777215,
              },
              {
                $ref: '#/$defs/TerminalNamedColor',
              },
            ],
          },
          TerminalNamedColor: {
            type: 'string',
            enum: [
              'reset',
              'black',
              'red',
              'green',
              'yellow',
              'blue',
              'magenta',
              'cyan',
              'gray',
              'dark_gray',
              'light_red',
              'light_green',
              'light_yellow',
              'light_blue',
              'light_magenta',
              'light_cyan',
              'white',
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
          TerminalPaste: {
            type: 'string',
            enum: ['reject_controls', 'spaces'],
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
              {
                description:
                  'Continue initialization after Reset, without creating undo history.\nEach chunk is bounded; the same native owner retains the full buffer.',
                type: 'object',
                properties: {
                  text: {
                    type: 'string',
                    maxLength: 65536,
                  },
                  kind: {
                    type: 'string',
                    const: 'append',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'text'],
              },
            ],
          },
          SubmitCopy: {
            type: 'object',
            properties: {
              basis: {
                $ref: '#/$defs/EditorTextRef',
              },
              text: {
                type: 'string',
                minLength: 1,
                maxLength: 65536,
              },
            },
            additionalProperties: false,
            required: ['basis', 'text'],
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
              enum: [
                'persistence',
                'owner',
                'representation',
                'revision',
                'composition',
                'sequence',
              ],
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
          output: {
            kind: 'resource',
            resource: {
              name: 'maka.text.utf8',
              version: 1,
            },
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
          autoFocus: {
            description:
              'Offer native focus once when this original input becomes visible and\nenabled. Stable updates and viewport scrolling do not repeat the offer.',
            type: 'boolean',
            default: false,
          },
          maxBytes: {
            description: 'UTF-8 byte capacity, fixed for this original native editor mount.',
            type: 'integer',
            format: 'uint32',
            minimum: 1,
            maximum: 65536,
            default: 65536,
          },
          terminalPaste: {
            description: 'Terminal bracketed-paste policy. Other native insertion remains literal.',
            $ref: '#/$defs/TerminalPaste',
            default: 'reject_controls',
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
          submitCopy: {
            description:
              'Clipboard default for physical Enter at this original editor revision.\nProgrammatic submitted events never execute this native default.',
            anyOf: [
              {
                $ref: '#/$defs/SubmitCopy',
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
              ascii: {
                type: 'boolean',
                default: false,
              },
              terminal: {
                anyOf: [
                  {
                    $ref: '#/$defs/TerminalPalette',
                  },
                  {
                    type: 'null',
                  },
                ],
                default: null,
              },
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
                description:
                  'In TUI inputs, a positive width draws a one-cell border; zero has no inset.',
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
          TerminalPalette: {
            description:
              'Semantic terminal colors for rich text and editors, supplied by the product.',
            type: 'object',
            properties: {
              background: {
                $ref: '#/$defs/TerminalColor',
              },
              foreground: {
                $ref: '#/$defs/TerminalColor',
              },
              surface: {
                $ref: '#/$defs/TerminalColor',
              },
              accent: {
                $ref: '#/$defs/TerminalColor',
              },
              thinking: {
                $ref: '#/$defs/TerminalColor',
              },
              success: {
                $ref: '#/$defs/TerminalColor',
              },
              warning: {
                $ref: '#/$defs/TerminalColor',
              },
              error: {
                $ref: '#/$defs/TerminalColor',
              },
              muted: {
                $ref: '#/$defs/TerminalColor',
              },
              subtle: {
                $ref: '#/$defs/TerminalColor',
              },
              border: {
                $ref: '#/$defs/TerminalColor',
              },
              scrollbar: {
                $ref: '#/$defs/TerminalColor',
              },
              selection: {
                $ref: '#/$defs/TerminalColor',
              },
              selectionText: {
                $ref: '#/$defs/TerminalColor',
              },
              search: {
                $ref: '#/$defs/TerminalColor',
              },
              searchActive: {
                $ref: '#/$defs/TerminalColor',
              },
              searchText: {
                $ref: '#/$defs/TerminalColor',
              },
              syntax: {
                type: 'array',
                items: {
                  $ref: '#/$defs/TerminalColor',
                },
                minItems: 7,
                maxItems: 7,
              },
              terminal: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: [
              'background',
              'foreground',
              'surface',
              'accent',
              'thinking',
              'success',
              'warning',
              'error',
              'muted',
              'subtle',
              'border',
              'scrollbar',
              'selection',
              'selectionText',
              'search',
              'searchActive',
              'searchText',
              'syntax',
              'terminal',
            ],
          },
          TerminalColor: {
            description:
              "Terminal colors retain the user's ANSI palette and default background.\nNumeric values are RGB; named values are emitted as terminal attributes.",
            anyOf: [
              {
                type: 'integer',
                format: 'uint32',
                minimum: 0,
                maximum: 16777215,
              },
              {
                $ref: '#/$defs/TerminalNamedColor',
              },
            ],
          },
          TerminalNamedColor: {
            type: 'string',
            enum: [
              'reset',
              'black',
              'red',
              'green',
              'yellow',
              'blue',
              'magenta',
              'cyan',
              'gray',
              'dark_gray',
              'light_red',
              'light_green',
              'light_yellow',
              'light_blue',
              'light_magenta',
              'light_cyan',
              'white',
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
          TerminalPaste: {
            type: 'string',
            enum: ['reject_controls', 'spaces'],
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
              {
                description:
                  'Continue initialization after Reset, without creating undo history.\nEach chunk is bounded; the same native owner retains the full buffer.',
                type: 'object',
                properties: {
                  text: {
                    type: 'string',
                    maxLength: 65536,
                  },
                  kind: {
                    type: 'string',
                    const: 'append',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'text'],
              },
            ],
          },
          SubmitCopy: {
            type: 'object',
            properties: {
              basis: {
                $ref: '#/$defs/EditorTextRef',
              },
              text: {
                type: 'string',
                minLength: 1,
                maxLength: 65536,
              },
            },
            additionalProperties: false,
            required: ['basis', 'text'],
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
              enum: [
                'persistence',
                'owner',
                'representation',
                'revision',
                'composition',
                'sequence',
              ],
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
          output: {
            kind: 'resource',
            resource: {
              name: 'maka.text.utf8',
              version: 1,
            },
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
        catalog: {
          mode: 'subscription',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentList',
            type: 'object',
            properties: {
              filter: {
                $ref: '#/$defs/AgentSessionFilter',
              },
              position: {
                $ref: '#/$defs/AgentSessionPosition',
              },
            },
            additionalProperties: false,
            required: ['filter', 'position'],
            $defs: {
              AgentSessionFilter: {
                type: 'string',
                enum: ['all', 'needs_you'],
              },
              AgentSessionPosition: {
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
                      cursor: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 128,
                      },
                      kind: {
                        type: 'string',
                        const: 'at',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'cursor'],
                  },
                  {
                    type: 'object',
                    properties: {
                      revision: {
                        type: 'string',
                      },
                      cursor: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 128,
                      },
                      kind: {
                        type: 'string',
                        const: 'after',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'revision', 'cursor'],
                  },
                  {
                    type: 'object',
                    properties: {
                      revision: {
                        type: 'string',
                      },
                      cursor: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 128,
                      },
                      kind: {
                        type: 'string',
                        const: 'before',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'revision', 'cursor'],
                  },
                ],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentSessionPage',
            type: 'object',
            properties: {
              revision: {
                type: 'string',
              },
              needsYouCount: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
              },
              sessions: {
                type: 'array',
                items: {
                  $ref: '#/$defs/AgentSession',
                },
                maxItems: 32,
              },
              before: {
                type: ['string', 'null'],
              },
              after: {
                type: ['string', 'null'],
              },
            },
            additionalProperties: false,
            required: ['revision', 'needsYouCount', 'sessions', 'before', 'after'],
            $defs: {
              AgentSession: {
                type: 'object',
                properties: {
                  dataSource: {
                    $ref: '#/$defs/AgentDataSource',
                  },
                  permissions: {
                    $ref: '#/$defs/AgentWorkspacePermissions',
                  },
                  boundaryRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  id: {
                    type: 'string',
                  },
                  title: {
                    type: 'string',
                  },
                  labels: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                  workspace: {
                    $ref: '#/$defs/AgentWorkspace',
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
                  hasUnread: {
                    type: 'boolean',
                  },
                  activity: {
                    $ref: '#/$defs/AgentSessionActivity',
                  },
                },
                additionalProperties: false,
                required: [
                  'dataSource',
                  'permissions',
                  'boundaryRevision',
                  'id',
                  'title',
                  'labels',
                  'workspace',
                  'revision',
                  'updatedAt',
                  'archived',
                  'hasUnread',
                  'activity',
                ],
              },
              AgentDataSource: {
                description:
                  'Stable business-data origin, independent of package or activation revisions.',
                type: 'object',
                properties: {
                  stateDomain: {
                    type: 'string',
                  },
                  deployment: {
                    type: 'string',
                    format: 'uuid',
                  },
                  namespace: {
                    $ref: '#/$defs/InstanceId',
                  },
                },
                additionalProperties: false,
                required: ['stateDomain', 'deployment', 'namespace'],
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
              AgentWorkspacePermissions: {
                type: 'object',
                properties: {
                  access: {
                    $ref: '#/$defs/AgentWorkspaceAccess',
                  },
                  requestPermissions: {
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['access', 'requestPermissions'],
              },
              AgentWorkspaceAccess: {
                description:
                  'Access inside the original selected directory; never an ambient filesystem grant.',
                type: 'string',
                enum: ['read_only', 'workspace_write'],
              },
              AgentWorkspace: {
                type: 'object',
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                additionalProperties: false,
                required: ['path', 'directoryIdentity', 'identity'],
              },
              AgentSessionActivity: {
                type: 'string',
                enum: ['idle', 'working', 'waiting'],
              },
            },
          },
          resources: {},
        },
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
              labels: {
                type: 'array',
                items: {
                  type: 'string',
                },
                maxItems: 32,
              },
              workspace: {
                $ref: '#/$defs/AgentWorkspaceSelection',
              },
            },
            additionalProperties: false,
            required: ['title', 'labels', 'workspace'],
            $defs: {
              AgentWorkspaceSelection: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      selection: {
                        $ref: '#/$defs/AgentDirectorySelection',
                      },
                      kind: {
                        type: 'string',
                        const: 'directory',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'selection'],
                  },
                  {
                    type: 'object',
                    properties: {
                      sessionId: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 128,
                      },
                      kind: {
                        type: 'string',
                        const: 'session',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'sessionId'],
                  },
                ],
              },
              AgentDirectorySelection: {
                type: 'object',
                properties: {
                  rootId: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 64,
                  },
                  segments: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                    maxItems: 64,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                },
                additionalProperties: false,
                required: ['rootId', 'segments', 'directoryIdentity'],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentCreated',
            oneOf: [
              {
                type: 'object',
                properties: {
                  session: {
                    $ref: '#/$defs/AgentSession',
                  },
                  kind: {
                    type: 'string',
                    const: 'created',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'session'],
              },
              {
                type: 'object',
                properties: {
                  reason: {
                    $ref: '#/$defs/AgentRejection',
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
              AgentSession: {
                type: 'object',
                properties: {
                  dataSource: {
                    $ref: '#/$defs/AgentDataSource',
                  },
                  permissions: {
                    $ref: '#/$defs/AgentWorkspacePermissions',
                  },
                  boundaryRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  id: {
                    type: 'string',
                  },
                  title: {
                    type: 'string',
                  },
                  labels: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                  workspace: {
                    $ref: '#/$defs/AgentWorkspace',
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
                  hasUnread: {
                    type: 'boolean',
                  },
                  activity: {
                    $ref: '#/$defs/AgentSessionActivity',
                  },
                },
                additionalProperties: false,
                required: [
                  'dataSource',
                  'permissions',
                  'boundaryRevision',
                  'id',
                  'title',
                  'labels',
                  'workspace',
                  'revision',
                  'updatedAt',
                  'archived',
                  'hasUnread',
                  'activity',
                ],
              },
              AgentDataSource: {
                description:
                  'Stable business-data origin, independent of package or activation revisions.',
                type: 'object',
                properties: {
                  stateDomain: {
                    type: 'string',
                  },
                  deployment: {
                    type: 'string',
                    format: 'uuid',
                  },
                  namespace: {
                    $ref: '#/$defs/InstanceId',
                  },
                },
                additionalProperties: false,
                required: ['stateDomain', 'deployment', 'namespace'],
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
              AgentWorkspacePermissions: {
                type: 'object',
                properties: {
                  access: {
                    $ref: '#/$defs/AgentWorkspaceAccess',
                  },
                  requestPermissions: {
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['access', 'requestPermissions'],
              },
              AgentWorkspaceAccess: {
                description:
                  'Access inside the original selected directory; never an ambient filesystem grant.',
                type: 'string',
                enum: ['read_only', 'workspace_write'],
              },
              AgentWorkspace: {
                type: 'object',
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                additionalProperties: false,
                required: ['path', 'directoryIdentity', 'identity'],
              },
              AgentSessionActivity: {
                type: 'string',
                enum: ['idle', 'working', 'waiting'],
              },
              AgentRejection: {
                type: 'string',
                enum: [
                  'title_invalid',
                  'labels_invalid',
                  'workspace_unavailable',
                  'session_unavailable',
                  'closed',
                ],
              },
            },
          },
          resources: {},
        },
        directories: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentDirectories',
            oneOf: [
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'roots',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
              {
                type: 'object',
                properties: {
                  selection: {
                    $ref: '#/$defs/AgentDirectorySelection',
                  },
                  after: {
                    type: ['string', 'null'],
                  },
                  kind: {
                    type: 'string',
                    const: 'children',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'selection', 'after'],
              },
            ],
            $defs: {
              AgentDirectorySelection: {
                type: 'object',
                properties: {
                  rootId: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 64,
                  },
                  segments: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                    maxItems: 64,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                },
                additionalProperties: false,
                required: ['rootId', 'segments', 'directoryIdentity'],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentDirectoryPage',
            oneOf: [
              {
                type: 'object',
                properties: {
                  roots: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/AgentDirectory',
                    },
                    maxItems: 8,
                  },
                  kind: {
                    type: 'string',
                    const: 'roots',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'roots'],
              },
              {
                type: 'object',
                properties: {
                  directory: {
                    $ref: '#/$defs/AgentDirectory',
                  },
                  children: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/AgentDirectory',
                    },
                    maxItems: 32,
                  },
                  after: {
                    type: ['string', 'null'],
                  },
                  kind: {
                    type: 'string',
                    const: 'children',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'directory', 'children', 'after'],
              },
            ],
            $defs: {
              AgentDirectory: {
                type: 'object',
                properties: {
                  name: {
                    type: 'string',
                  },
                  path: {
                    type: 'string',
                  },
                  selection: {
                    $ref: '#/$defs/AgentDirectorySelection',
                  },
                },
                additionalProperties: false,
                required: ['name', 'path', 'selection'],
              },
              AgentDirectorySelection: {
                type: 'object',
                properties: {
                  rootId: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 64,
                  },
                  segments: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                    maxItems: 64,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                },
                additionalProperties: false,
                required: ['rootId', 'segments', 'directoryIdentity'],
              },
            },
          },
          resources: {},
        },
        editQueue: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentQueueMutation',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              expectedRevision: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 9007199254740991,
              },
              edit: {
                $ref: '#/$defs/AgentQueueAction',
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'expectedRevision', 'edit'],
            $defs: {
              AgentQueueAction: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      messageId: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 128,
                      },
                      kind: {
                        type: 'string',
                        const: 'retract',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'messageId'],
                  },
                  {
                    type: 'object',
                    properties: {
                      messageId: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 128,
                      },
                      kind: {
                        type: 'string',
                        const: 'promote',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'messageId'],
                  },
                  {
                    type: 'object',
                    properties: {
                      messageId: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 128,
                      },
                      message: {
                        type: 'string',
                        maxLength: 65536,
                      },
                      kind: {
                        type: 'string',
                        const: 'update',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'messageId', 'message'],
                  },
                  {
                    type: 'object',
                    properties: {
                      messageIds: {
                        type: 'array',
                        items: {
                          type: 'string',
                        },
                        minItems: 1,
                        maxItems: 64,
                      },
                      kind: {
                        type: 'string',
                        const: 'reorder',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'messageIds'],
                  },
                ],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentQueueMutationResult',
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
                    const: 'committed',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'revision'],
              },
              {
                type: 'object',
                properties: {
                  expected: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  actual: {
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
                required: ['kind', 'expected', 'actual'],
              },
              {
                type: 'object',
                properties: {
                  reason: {
                    $ref: '#/$defs/AgentQueueRejection',
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
              AgentQueueRejection: {
                type: 'string',
                enum: [
                  'closed',
                  'missing',
                  'in_flight',
                  'invalid_edit',
                  'execution_changed',
                  'not_running',
                ],
              },
            },
          },
          resources: {},
        },
        execution: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentExecutionRead',
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
            title: 'AgentExecutionResult',
            oneOf: [
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'missing',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
              {
                type: 'object',
                properties: {
                  execution: {
                    $ref: '#/$defs/AgentExecution',
                  },
                  kind: {
                    type: 'string',
                    const: 'found',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'execution'],
              },
            ],
            $defs: {
              AgentExecution: {
                description:
                  'Canonical physical execution receipt, independent of the current Session tip.',
                type: 'object',
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  turnId: {
                    type: 'string',
                  },
                  runId: {
                    type: 'string',
                  },
                  invocationId: {
                    type: 'string',
                  },
                  highWater: {
                    description: 'Run-local event count, not the root-global transcript cursor.',
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  state: {
                    $ref: '#/$defs/AgentExecutionState',
                  },
                },
                additionalProperties: false,
                required: ['sessionId', 'turnId', 'runId', 'invocationId', 'highWater', 'state'],
              },
              AgentExecutionState: {
                type: 'string',
                enum: [
                  'admitted',
                  'running',
                  'waiting',
                  'paused',
                  'completed',
                  'failed',
                  'cancelled',
                ],
              },
            },
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
                $ref: '#/$defs/HistoryPosition',
              },
              includeThinking: {
                type: 'boolean',
                default: false,
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'position'],
            $defs: {
              HistoryPosition: {
                description:
                  'Cursor sequence addresses a sparse transcript row; Event addresses the raw\nevent log. Before excludes its end and After includes its start. Returned\nchunks always retain ascending source order, including a reverse window.',
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
                      kind: {
                        type: 'string',
                        const: 'tail',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    properties: {
                      cursor: {
                        $ref: '#/$defs/Cursor',
                      },
                      kind: {
                        type: 'string',
                        const: 'before',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'cursor'],
                  },
                  {
                    type: 'object',
                    properties: {
                      cursor: {
                        $ref: '#/$defs/Cursor',
                      },
                      kind: {
                        type: 'string',
                        const: 'after',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'cursor'],
                  },
                  {
                    description:
                      'Locate canonical output at an execution event, including sparse rows.',
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
                ],
              },
              Cursor: {
                description:
                  "Byte offset in one message field's UTF-8 text, not a bearer capability.",
                type: 'object',
                properties: {
                  field: {
                    $ref: '#/$defs/HistoryField',
                    default: 'body',
                  },
                  sequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 1,
                    maximum: 9007199254740991,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                    maximum: 9007199254740991,
                  },
                },
                additionalProperties: false,
                required: ['sequence', 'offset'],
              },
              HistoryField: {
                description: 'A readable field in a conversation window, in presentation order.',
                type: 'string',
                enum: ['thinking', 'body'],
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
                  before: {
                    anyOf: [
                      {
                        $ref: '#/$defs/Cursor',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  after: {
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
                required: ['kind', 'through', 'chunks', 'before', 'after'],
              },
            ],
            $defs: {
              AgentHistoryChunk: {
                type: 'object',
                properties: {
                  timing: {
                    anyOf: [
                      {
                        $ref: '#/$defs/HistoryTurnTiming',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  field: {
                    $ref: '#/$defs/HistoryField',
                  },
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
                  tool: {
                    anyOf: [
                      {
                        $ref: '#/$defs/HistoryTool',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                },
                additionalProperties: false,
                required: [
                  'field',
                  'messageId',
                  'turnId',
                  'timestamp',
                  'role',
                  'sequence',
                  'offset',
                  'totalBytes',
                  'text',
                  'tool',
                ],
              },
              HistoryTurnTiming: {
                description:
                  'Recorded logical Turn boundaries at the same history fence. An unfinished\nhistorical boundary alone does not establish that execution is still live.',
                type: 'object',
                properties: {
                  startedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                    maximum: 9007199254740991,
                  },
                  finished: {
                    anyOf: [
                      {
                        $ref: '#/$defs/HistoryTurnFinished',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                },
                additionalProperties: false,
                required: ['startedAt', 'finished'],
              },
              HistoryTurnFinished: {
                type: 'object',
                properties: {
                  at: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                    maximum: 9007199254740991,
                  },
                  outcome: {
                    $ref: '#/$defs/HistoryTurnOutcome',
                  },
                },
                additionalProperties: false,
                required: ['at', 'outcome'],
              },
              HistoryTurnOutcome: {
                type: 'string',
                enum: ['completed', 'failed', 'aborted'],
              },
              HistoryField: {
                description: 'A readable field in a conversation window, in presentation order.',
                type: 'string',
                enum: ['thinking', 'body'],
              },
              Role: {
                description:
                  'Canonical message author/kind; the field identifies body versus thinking.\nTool body JSON contributes string values, never its keys.',
                type: 'string',
                enum: ['user', 'assistant', 'tool_call', 'tool_result'],
              },
              HistoryTool: {
                description:
                  'Canonical references at the requested fence, never execution authority.\nBoth references are present only for a unique same-Turn call/result pair\nwith matching origin and parentage. An unpaired row retains its own reference.',
                type: 'object',
                properties: {
                  toolUseId: {
                    type: 'string',
                  },
                  call: {
                    anyOf: [
                      {
                        $ref: '#/$defs/HistoryToolCall',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  result: {
                    anyOf: [
                      {
                        $ref: '#/$defs/HistoryToolResult',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                },
                additionalProperties: false,
                required: ['toolUseId', 'call', 'result'],
              },
              HistoryToolCall: {
                type: 'object',
                properties: {
                  presentation: {
                    description:
                      'Opaque display contract captured at dispatch; no execution authority.',
                    type: ['string', 'null'],
                    default: null,
                  },
                  sequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 1,
                    maximum: 9007199254740991,
                  },
                  name: {
                    type: 'string',
                  },
                  title: {
                    anyOf: [
                      {
                        $ref: '#/$defs/DisplayText',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  pending: {
                    description:
                      'This native Turn is still open at the history fence; not execution authority.',
                    type: 'boolean',
                  },
                  dispatched: {
                    description:
                      'A unique original Host dispatch is recorded at this fence; not liveness.',
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['sequence', 'name', 'title', 'pending', 'dispatched'],
              },
              DisplayText: {
                description:
                  'Presentation text is distinct from the stable identity used for navigation.',
                type: 'object',
                properties: {
                  fallback: {
                    type: 'string',
                  },
                  translations: {
                    type: 'object',
                    additionalProperties: {
                      type: 'string',
                    },
                    default: {},
                  },
                },
                additionalProperties: false,
                required: ['fallback'],
              },
              HistoryToolResult: {
                type: 'object',
                properties: {
                  settlement: {
                    anyOf: [
                      {
                        $ref: '#/$defs/ToolSettlement',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  sequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 1,
                    maximum: 9007199254740991,
                  },
                  isError: {
                    description: 'Does not distinguish failed from unknown effects.',
                    type: 'boolean',
                  },
                  image: {
                    description:
                      'Canonical content is an image; this does not grant artifact reads.',
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['settlement', 'sequence', 'isError', 'image'],
              },
              ToolSettlement: {
                description:
                  'Recorded Host settlement for presentation; returned is not business success.\nForeign/provider results do not establish this Host evidence.',
                type: 'string',
                enum: ['returned', 'failed', 'unknown', 'rejected'],
              },
              Cursor: {
                description:
                  "Byte offset in one message field's UTF-8 text, not a bearer capability.",
                type: 'object',
                properties: {
                  field: {
                    $ref: '#/$defs/HistoryField',
                    default: 'body',
                  },
                  sequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 1,
                    maximum: 9007199254740991,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                    maximum: 9007199254740991,
                  },
                },
                additionalProperties: false,
                required: ['sequence', 'offset'],
              },
            },
          },
          resources: {},
        },
        interaction: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentInteractionRead',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              requestId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'requestId'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'InteractionRecord',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
              },
              turnId: {
                type: 'string',
              },
              runId: {
                type: 'string',
              },
              requestId: {
                type: 'string',
              },
              createdAt: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
              },
              request: {
                $ref: '#/$defs/InteractionRequest',
              },
              outcome: {
                anyOf: [
                  {
                    $ref: '#/$defs/InteractionOutcome',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
            },
            additionalProperties: false,
            required: [
              'sessionId',
              'turnId',
              'runId',
              'requestId',
              'createdAt',
              'request',
              'outcome',
            ],
            $defs: {
              InteractionRequest: {
                oneOf: [
                  {
                    description:
                      'Consent for one already journaled service call; never a service grant.',
                    type: 'object',
                    properties: {
                      toolUseId: {
                        type: 'string',
                      },
                      toolName: {
                        type: 'string',
                      },
                      service: {
                        type: 'string',
                      },
                      method: {
                        type: 'string',
                      },
                      binding: {
                        type: 'string',
                      },
                      through: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'service_approval',
                      },
                    },
                    additionalProperties: false,
                    required: [
                      'kind',
                      'toolUseId',
                      'toolName',
                      'service',
                      'method',
                      'binding',
                      'through',
                    ],
                  },
                  {
                    type: 'object',
                    properties: {
                      toolUseId: {
                        type: ['string', 'null'],
                      },
                      baseRevision: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      request: {
                        $ref: '#/$defs/PermissionRequest',
                      },
                      kind: {
                        type: 'string',
                        const: 'permissions',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'toolUseId', 'baseRevision', 'request'],
                  },
                  {
                    type: 'object',
                    properties: {
                      toolUseId: {
                        type: 'string',
                      },
                      questions: {
                        type: 'array',
                        items: {
                          $ref: '#/$defs/InteractionQuestion',
                        },
                      },
                      kind: {
                        type: 'string',
                        const: 'question',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'toolUseId', 'questions'],
                  },
                  {
                    type: 'object',
                    properties: {
                      toolUseId: {
                        type: 'string',
                      },
                      message: {
                        type: 'string',
                      },
                      requester: {
                        $ref: '#/$defs/FormRequester',
                      },
                      fields: {
                        type: 'array',
                        items: {
                          $ref: '#/$defs/FormField',
                        },
                      },
                      kind: {
                        type: 'string',
                        const: 'form',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'toolUseId', 'message', 'requester', 'fields'],
                  },
                  {
                    type: 'object',
                    properties: {
                      toolUseId: {
                        type: 'string',
                      },
                      target: {
                        $ref: '#/$defs/GrantTarget',
                      },
                      kind: {
                        type: 'string',
                        const: 'client_capability',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'toolUseId', 'target'],
                  },
                ],
              },
              PermissionRequest: {
                description:
                  'The Host supplies the real operation description; it is display information,\nnot authority inferred from a command string or a plugin-supplied Session ID.',
                type: 'object',
                properties: {
                  reason: {
                    type: 'string',
                  },
                  command: {
                    anyOf: [
                      {
                        $ref: '#/$defs/PermissionCommand',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  permissions: {
                    $ref: '#/$defs/Permissions',
                  },
                },
                additionalProperties: false,
                required: ['reason', 'permissions'],
              },
              PermissionCommand: {
                type: 'object',
                properties: {
                  command: {
                    type: 'string',
                  },
                  cwd: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['command', 'cwd'],
              },
              Permissions: {
                type: 'object',
                properties: {
                  filesystem: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/Rule',
                    },
                    maxItems: 32,
                  },
                  network: {
                    $ref: '#/$defs/Network',
                  },
                },
                additionalProperties: false,
                required: ['filesystem', 'network'],
              },
              Rule: {
                description:
                  'Paths have executor-native meaning. Validate and materialize them on the\nexecution host, never on a remote Desktop. This document alone grants nothing.',
                type: 'object',
                properties: {
                  path: {
                    type: 'string',
                  },
                  scope: {
                    $ref: '#/$defs/Scope',
                  },
                  access: {
                    $ref: '#/$defs/Access',
                  },
                },
                additionalProperties: false,
                required: ['path', 'scope', 'access'],
              },
              Scope: {
                type: 'string',
                enum: ['subtree', 'exact'],
              },
              Access: {
                description:
                  'Equal-path conflict priority is different from authority breadth: deny wins.',
                type: 'string',
                enum: ['read', 'write', 'deny'],
              },
              Network: {
                oneOf: [
                  {
                    type: 'string',
                    enum: ['denied', 'allowed'],
                  },
                  {
                    type: 'object',
                    properties: {
                      restricted: {
                        type: 'object',
                        properties: {
                          destinations: {
                            type: 'array',
                            uniqueItems: true,
                            items: {
                              $ref: '#/$defs/Destination',
                            },
                            maxItems: 128,
                          },
                        },
                        additionalProperties: false,
                        required: ['destinations'],
                      },
                    },
                    required: ['restricted'],
                    additionalProperties: false,
                  },
                ],
              },
              Destination: {
                description:
                  'Destination authority, independent of the upstream proxy used to reach it.\nA domain grant does not authorize its subdomains or other ports.',
                type: 'object',
                properties: {
                  host: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 253,
                  },
                  port: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 1,
                    maximum: 65535,
                  },
                },
                additionalProperties: false,
                required: ['host', 'port'],
              },
              InteractionQuestion: {
                type: 'object',
                properties: {
                  question: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 1024,
                  },
                  options: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/QuestionOption',
                    },
                    minItems: 2,
                    maxItems: 3,
                  },
                },
                additionalProperties: false,
                required: ['question', 'options'],
              },
              QuestionOption: {
                type: 'object',
                properties: {
                  label: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 256,
                  },
                  description: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 512,
                  },
                },
                additionalProperties: false,
                required: ['label'],
              },
              FormRequester: {
                type: 'object',
                properties: {
                  name: {
                    type: 'string',
                  },
                  source: {
                    type: ['string', 'null'],
                  },
                },
                additionalProperties: false,
                required: ['name'],
              },
              FormField: {
                type: 'object',
                properties: {
                  name: {
                    type: 'string',
                  },
                  label: {
                    type: 'string',
                  },
                  required: {
                    type: 'boolean',
                  },
                  description: {
                    type: ['string', 'null'],
                  },
                },
                required: ['name', 'label', 'required'],
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      default: {
                        type: ['string', 'null'],
                      },
                      minLength: {
                        type: ['integer', 'null'],
                        format: 'uint',
                        minimum: 0,
                      },
                      maxLength: {
                        type: ['integer', 'null'],
                        format: 'uint',
                        minimum: 0,
                      },
                      format: {
                        anyOf: [
                          {
                            $ref: '#/$defs/FormFormat',
                          },
                          {
                            type: 'null',
                          },
                        ],
                      },
                      kind: {
                        type: 'string',
                        const: 'string',
                      },
                    },
                    required: ['kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      default: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      minimum: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      maximum: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      kind: {
                        type: 'string',
                        const: 'number',
                      },
                    },
                    required: ['kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      default: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      minimum: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      maximum: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      kind: {
                        type: 'string',
                        const: 'integer',
                      },
                    },
                    required: ['kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      default: {
                        type: ['boolean', 'null'],
                      },
                      kind: {
                        type: 'string',
                        const: 'boolean',
                      },
                    },
                    required: ['kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      options: {
                        type: 'array',
                        items: {
                          $ref: '#/$defs/FormOption',
                        },
                      },
                      default: {
                        type: ['string', 'null'],
                      },
                      kind: {
                        type: 'string',
                        const: 'single_select',
                      },
                    },
                    required: ['kind', 'options'],
                  },
                  {
                    type: 'object',
                    properties: {
                      options: {
                        type: 'array',
                        items: {
                          $ref: '#/$defs/FormOption',
                        },
                      },
                      default: {
                        type: ['array', 'null'],
                        items: {
                          type: 'string',
                        },
                      },
                      minItems: {
                        type: ['integer', 'null'],
                        format: 'uint',
                        minimum: 0,
                      },
                      maxItems: {
                        type: ['integer', 'null'],
                        format: 'uint',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'multi_select',
                      },
                    },
                    required: ['kind', 'options'],
                  },
                ],
              },
              FormFormat: {
                type: 'string',
                enum: ['email', 'uri', 'date', 'date-time'],
              },
              FormOption: {
                type: 'object',
                properties: {
                  value: {
                    type: 'string',
                  },
                  label: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['value', 'label'],
              },
              GrantTarget: {
                type: 'object',
                properties: {
                  providerId: {
                    type: 'string',
                  },
                  contractId: {
                    type: 'string',
                  },
                  serverId: {
                    type: 'string',
                  },
                  toolName: {
                    type: 'string',
                  },
                  capability: {
                    $ref: '#/$defs/GrantCapability',
                  },
                  scope: {
                    $ref: '#/$defs/GrantScope',
                  },
                },
                required: [
                  'providerId',
                  'contractId',
                  'serverId',
                  'toolName',
                  'capability',
                  'scope',
                ],
              },
              GrantCapability: {
                type: 'string',
                enum: ['browser', 'computer_use', 'desktop_mcp', 'mcp'],
              },
              GrantScope: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      origin: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'browser_origin',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'origin'],
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'capability',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      serverId: {
                        type: 'string',
                      },
                      toolName: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'mcp_tool',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'serverId', 'toolName'],
                  },
                ],
              },
              InteractionOutcome: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      decision: {
                        $ref: '#/$defs/Decision',
                      },
                      committedAt: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'service_approval_decision',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'decision', 'committedAt'],
                  },
                  {
                    type: 'object',
                    properties: {
                      decision: {
                        $ref: '#/$defs/Decision2',
                      },
                      committedAt: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'permissions_decision',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'decision', 'committedAt'],
                  },
                  {
                    type: 'object',
                    properties: {
                      answers: {
                        type: 'array',
                        items: {
                          type: ['string', 'null'],
                        },
                      },
                      committedAt: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'question_answer',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'answers', 'committedAt'],
                  },
                  {
                    type: 'object',
                    properties: {
                      committedAt: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'form_answer',
                      },
                    },
                    unevaluatedProperties: false,
                    oneOf: [
                      {
                        type: 'object',
                        properties: {
                          values: {
                            type: 'object',
                            additionalProperties: {
                              $ref: '#/$defs/FormValue',
                            },
                          },
                          action: {
                            type: 'string',
                            const: 'accept',
                          },
                        },
                        required: ['action', 'values'],
                      },
                      {
                        type: 'object',
                        properties: {
                          action: {
                            type: 'string',
                            const: 'decline',
                          },
                        },
                        required: ['action'],
                      },
                      {
                        type: 'object',
                        properties: {
                          action: {
                            type: 'string',
                            const: 'cancel',
                          },
                        },
                        required: ['action'],
                      },
                    ],
                    required: ['kind', 'committedAt'],
                  },
                  {
                    type: 'object',
                    properties: {
                      decision: {
                        $ref: '#/$defs/Decision',
                      },
                      committedAt: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'client_capability_decision',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'decision', 'committedAt'],
                  },
                  {
                    type: 'object',
                    properties: {
                      reason: {
                        $ref: '#/$defs/ClosureReason',
                      },
                      committedAt: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'closure',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'reason', 'committedAt'],
                  },
                ],
              },
              Decision: {
                type: 'string',
                enum: ['allow', 'deny'],
              },
              Decision2: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      decision: {
                        type: 'string',
                        const: 'deny',
                      },
                    },
                    required: ['decision'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    properties: {
                      permissions: {
                        $ref: '#/$defs/Permissions',
                      },
                      scope: {
                        $ref: '#/$defs/Scope2',
                      },
                      decision: {
                        type: 'string',
                        const: 'allow',
                      },
                    },
                    additionalProperties: false,
                    required: ['decision', 'permissions', 'scope'],
                  },
                ],
              },
              Scope2: {
                type: 'string',
                enum: ['once', 'turn', 'session'],
              },
              FormValue: {
                anyOf: [
                  {
                    type: 'string',
                  },
                  {
                    type: 'number',
                    format: 'double',
                  },
                  {
                    type: 'boolean',
                  },
                  {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                ],
              },
              ClosureReason: {
                type: 'string',
                enum: [
                  'turn_stopped',
                  'turn_terminal',
                  'producer_cancelled',
                  'timed_out',
                  'host_restarted',
                  'provider_disconnected',
                ],
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
              filter: {
                $ref: '#/$defs/AgentSessionFilter',
              },
              position: {
                $ref: '#/$defs/AgentSessionPosition',
              },
            },
            additionalProperties: false,
            required: ['filter', 'position'],
            $defs: {
              AgentSessionFilter: {
                type: 'string',
                enum: ['all', 'needs_you'],
              },
              AgentSessionPosition: {
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
                      cursor: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 128,
                      },
                      kind: {
                        type: 'string',
                        const: 'at',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'cursor'],
                  },
                  {
                    type: 'object',
                    properties: {
                      revision: {
                        type: 'string',
                      },
                      cursor: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 128,
                      },
                      kind: {
                        type: 'string',
                        const: 'after',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'revision', 'cursor'],
                  },
                  {
                    type: 'object',
                    properties: {
                      revision: {
                        type: 'string',
                      },
                      cursor: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 128,
                      },
                      kind: {
                        type: 'string',
                        const: 'before',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'revision', 'cursor'],
                  },
                ],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentSessionPage',
            type: 'object',
            properties: {
              revision: {
                type: 'string',
              },
              needsYouCount: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
              },
              sessions: {
                type: 'array',
                items: {
                  $ref: '#/$defs/AgentSession',
                },
                maxItems: 32,
              },
              before: {
                type: ['string', 'null'],
              },
              after: {
                type: ['string', 'null'],
              },
            },
            additionalProperties: false,
            required: ['revision', 'needsYouCount', 'sessions', 'before', 'after'],
            $defs: {
              AgentSession: {
                type: 'object',
                properties: {
                  dataSource: {
                    $ref: '#/$defs/AgentDataSource',
                  },
                  permissions: {
                    $ref: '#/$defs/AgentWorkspacePermissions',
                  },
                  boundaryRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  id: {
                    type: 'string',
                  },
                  title: {
                    type: 'string',
                  },
                  labels: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                  workspace: {
                    $ref: '#/$defs/AgentWorkspace',
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
                  hasUnread: {
                    type: 'boolean',
                  },
                  activity: {
                    $ref: '#/$defs/AgentSessionActivity',
                  },
                },
                additionalProperties: false,
                required: [
                  'dataSource',
                  'permissions',
                  'boundaryRevision',
                  'id',
                  'title',
                  'labels',
                  'workspace',
                  'revision',
                  'updatedAt',
                  'archived',
                  'hasUnread',
                  'activity',
                ],
              },
              AgentDataSource: {
                description:
                  'Stable business-data origin, independent of package or activation revisions.',
                type: 'object',
                properties: {
                  stateDomain: {
                    type: 'string',
                  },
                  deployment: {
                    type: 'string',
                    format: 'uuid',
                  },
                  namespace: {
                    $ref: '#/$defs/InstanceId',
                  },
                },
                additionalProperties: false,
                required: ['stateDomain', 'deployment', 'namespace'],
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
              AgentWorkspacePermissions: {
                type: 'object',
                properties: {
                  access: {
                    $ref: '#/$defs/AgentWorkspaceAccess',
                  },
                  requestPermissions: {
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['access', 'requestPermissions'],
              },
              AgentWorkspaceAccess: {
                description:
                  'Access inside the original selected directory; never an ambient filesystem grant.',
                type: 'string',
                enum: ['read_only', 'workspace_write'],
              },
              AgentWorkspace: {
                type: 'object',
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                additionalProperties: false,
                required: ['path', 'directoryIdentity', 'identity'],
              },
              AgentSessionActivity: {
                type: 'string',
                enum: ['idle', 'working', 'waiting'],
              },
            },
          },
          resources: {},
        },
        markRead: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentMarkRead',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              messageId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'messageId'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentMarkReadResult',
            oneOf: [
              {
                type: 'object',
                properties: {
                  session: {
                    $ref: '#/$defs/AgentSession',
                  },
                  kind: {
                    type: 'string',
                    const: 'observed',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'session'],
              },
              {
                type: 'object',
                properties: {
                  reason: {
                    $ref: '#/$defs/AgentRejection',
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
              AgentSession: {
                type: 'object',
                properties: {
                  dataSource: {
                    $ref: '#/$defs/AgentDataSource',
                  },
                  permissions: {
                    $ref: '#/$defs/AgentWorkspacePermissions',
                  },
                  boundaryRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  id: {
                    type: 'string',
                  },
                  title: {
                    type: 'string',
                  },
                  labels: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                  workspace: {
                    $ref: '#/$defs/AgentWorkspace',
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
                  hasUnread: {
                    type: 'boolean',
                  },
                  activity: {
                    $ref: '#/$defs/AgentSessionActivity',
                  },
                },
                additionalProperties: false,
                required: [
                  'dataSource',
                  'permissions',
                  'boundaryRevision',
                  'id',
                  'title',
                  'labels',
                  'workspace',
                  'revision',
                  'updatedAt',
                  'archived',
                  'hasUnread',
                  'activity',
                ],
              },
              AgentDataSource: {
                description:
                  'Stable business-data origin, independent of package or activation revisions.',
                type: 'object',
                properties: {
                  stateDomain: {
                    type: 'string',
                  },
                  deployment: {
                    type: 'string',
                    format: 'uuid',
                  },
                  namespace: {
                    $ref: '#/$defs/InstanceId',
                  },
                },
                additionalProperties: false,
                required: ['stateDomain', 'deployment', 'namespace'],
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
              AgentWorkspacePermissions: {
                type: 'object',
                properties: {
                  access: {
                    $ref: '#/$defs/AgentWorkspaceAccess',
                  },
                  requestPermissions: {
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['access', 'requestPermissions'],
              },
              AgentWorkspaceAccess: {
                description:
                  'Access inside the original selected directory; never an ambient filesystem grant.',
                type: 'string',
                enum: ['read_only', 'workspace_write'],
              },
              AgentWorkspace: {
                type: 'object',
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                additionalProperties: false,
                required: ['path', 'directoryIdentity', 'identity'],
              },
              AgentSessionActivity: {
                type: 'string',
                enum: ['idle', 'working', 'waiting'],
              },
              AgentRejection: {
                type: 'string',
                enum: [
                  'title_invalid',
                  'labels_invalid',
                  'workspace_unavailable',
                  'session_unavailable',
                  'closed',
                ],
              },
            },
          },
          resources: {},
        },
        metadata: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentMetadata',
            type: 'object',
            properties: {
              permissions: {
                description: 'Omission leaves the permission boundary and its approvals unchanged.',
                anyOf: [
                  {
                    $ref: '#/$defs/AgentPermissionChange',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              expectedRevision: {
                type: 'integer',
                format: 'uint64',
                minimum: 1,
                maximum: 9007199254740991,
              },
              title: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              labels: {
                type: 'array',
                items: {
                  type: 'string',
                },
                maxItems: 32,
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'expectedRevision', 'title', 'labels'],
            $defs: {
              AgentPermissionChange: {
                type: 'object',
                properties: {
                  settings: {
                    $ref: '#/$defs/AgentWorkspacePermissions',
                  },
                  revokeGrants: {
                    description:
                      'Invalidate additive approvals even when the base policy is unchanged.',
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['settings', 'revokeGrants'],
              },
              AgentWorkspacePermissions: {
                type: 'object',
                properties: {
                  access: {
                    $ref: '#/$defs/AgentWorkspaceAccess',
                  },
                  requestPermissions: {
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['access', 'requestPermissions'],
              },
              AgentWorkspaceAccess: {
                description:
                  'Access inside the original selected directory; never an ambient filesystem grant.',
                type: 'string',
                enum: ['read_only', 'workspace_write'],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentMetadataResult',
            oneOf: [
              {
                type: 'object',
                properties: {
                  session: {
                    $ref: '#/$defs/AgentSession',
                  },
                  kind: {
                    type: 'string',
                    const: 'committed',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'session'],
              },
              {
                type: 'object',
                properties: {
                  expected: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  actual: {
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
                required: ['kind', 'expected', 'actual'],
              },
              {
                type: 'object',
                properties: {
                  reason: {
                    $ref: '#/$defs/AgentRejection',
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
              AgentSession: {
                type: 'object',
                properties: {
                  dataSource: {
                    $ref: '#/$defs/AgentDataSource',
                  },
                  permissions: {
                    $ref: '#/$defs/AgentWorkspacePermissions',
                  },
                  boundaryRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  id: {
                    type: 'string',
                  },
                  title: {
                    type: 'string',
                  },
                  labels: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                  workspace: {
                    $ref: '#/$defs/AgentWorkspace',
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
                  hasUnread: {
                    type: 'boolean',
                  },
                  activity: {
                    $ref: '#/$defs/AgentSessionActivity',
                  },
                },
                additionalProperties: false,
                required: [
                  'dataSource',
                  'permissions',
                  'boundaryRevision',
                  'id',
                  'title',
                  'labels',
                  'workspace',
                  'revision',
                  'updatedAt',
                  'archived',
                  'hasUnread',
                  'activity',
                ],
              },
              AgentDataSource: {
                description:
                  'Stable business-data origin, independent of package or activation revisions.',
                type: 'object',
                properties: {
                  stateDomain: {
                    type: 'string',
                  },
                  deployment: {
                    type: 'string',
                    format: 'uuid',
                  },
                  namespace: {
                    $ref: '#/$defs/InstanceId',
                  },
                },
                additionalProperties: false,
                required: ['stateDomain', 'deployment', 'namespace'],
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
              AgentWorkspacePermissions: {
                type: 'object',
                properties: {
                  access: {
                    $ref: '#/$defs/AgentWorkspaceAccess',
                  },
                  requestPermissions: {
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['access', 'requestPermissions'],
              },
              AgentWorkspaceAccess: {
                description:
                  'Access inside the original selected directory; never an ambient filesystem grant.',
                type: 'string',
                enum: ['read_only', 'workspace_write'],
              },
              AgentWorkspace: {
                type: 'object',
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                additionalProperties: false,
                required: ['path', 'directoryIdentity', 'identity'],
              },
              AgentSessionActivity: {
                type: 'string',
                enum: ['idle', 'working', 'waiting'],
              },
              AgentRejection: {
                type: 'string',
                enum: [
                  'title_invalid',
                  'labels_invalid',
                  'workspace_unavailable',
                  'session_unavailable',
                  'closed',
                ],
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
                  dataSource: {
                    $ref: '#/$defs/AgentDataSource',
                  },
                  permissions: {
                    $ref: '#/$defs/AgentWorkspacePermissions',
                  },
                  boundaryRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  id: {
                    type: 'string',
                  },
                  title: {
                    type: 'string',
                  },
                  labels: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                  workspace: {
                    $ref: '#/$defs/AgentWorkspace',
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
                  hasUnread: {
                    type: 'boolean',
                  },
                  activity: {
                    $ref: '#/$defs/AgentSessionActivity',
                  },
                },
                additionalProperties: false,
                required: [
                  'dataSource',
                  'permissions',
                  'boundaryRevision',
                  'id',
                  'title',
                  'labels',
                  'workspace',
                  'revision',
                  'updatedAt',
                  'archived',
                  'hasUnread',
                  'activity',
                ],
              },
              AgentDataSource: {
                description:
                  'Stable business-data origin, independent of package or activation revisions.',
                type: 'object',
                properties: {
                  stateDomain: {
                    type: 'string',
                  },
                  deployment: {
                    type: 'string',
                    format: 'uuid',
                  },
                  namespace: {
                    $ref: '#/$defs/InstanceId',
                  },
                },
                additionalProperties: false,
                required: ['stateDomain', 'deployment', 'namespace'],
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
              AgentWorkspacePermissions: {
                type: 'object',
                properties: {
                  access: {
                    $ref: '#/$defs/AgentWorkspaceAccess',
                  },
                  requestPermissions: {
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['access', 'requestPermissions'],
              },
              AgentWorkspaceAccess: {
                description:
                  'Access inside the original selected directory; never an ambient filesystem grant.',
                type: 'string',
                enum: ['read_only', 'workspace_write'],
              },
              AgentWorkspace: {
                type: 'object',
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                additionalProperties: false,
                required: ['path', 'directoryIdentity', 'identity'],
              },
              AgentSessionActivity: {
                type: 'string',
                enum: ['idle', 'working', 'waiting'],
              },
              AgentRun: {
                type: 'object',
                properties: {
                  invocationId: {
                    type: 'string',
                  },
                  turnId: {
                    type: 'string',
                  },
                  state: {
                    $ref: '#/$defs/AgentRunState',
                  },
                },
                additionalProperties: false,
                required: ['invocationId', 'turnId', 'state'],
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
        pendingInteractions: {
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
            title: 'AgentPendingInteractions',
            type: 'object',
            properties: {
              revision: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
              },
              entries: {
                type: 'array',
                items: {
                  $ref: '#/$defs/AgentInteractionSummary',
                },
                maxItems: 16,
              },
            },
            additionalProperties: false,
            required: ['revision', 'entries'],
            $defs: {
              AgentInteractionSummary: {
                type: 'object',
                properties: {
                  requestId: {
                    type: 'string',
                  },
                  kind: {
                    $ref: '#/$defs/AgentInteractionKind',
                  },
                  toolUseId: {
                    type: ['string', 'null'],
                  },
                },
                additionalProperties: false,
                required: ['requestId', 'kind', 'toolUseId'],
              },
              AgentInteractionKind: {
                type: 'string',
                enum: ['service_approval', 'question', 'permissions', 'form', 'client_capability'],
              },
            },
          },
          resources: {},
        },
        queue: {
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
            title: 'AgentQueue',
            type: 'object',
            properties: {
              revision: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
              },
              entries: {
                type: 'array',
                items: {
                  $ref: '#/$defs/AgentQueueEntry',
                },
                maxItems: 64,
              },
            },
            additionalProperties: false,
            required: ['revision', 'entries'],
            $defs: {
              AgentQueueEntry: {
                type: 'object',
                properties: {
                  messageId: {
                    type: 'string',
                  },
                  kind: {
                    $ref: '#/$defs/AgentQueueKind',
                  },
                  preview: {
                    type: 'string',
                    maxLength: 80,
                  },
                },
                additionalProperties: false,
                required: ['messageId', 'kind', 'preview'],
              },
              AgentQueueKind: {
                type: 'string',
                enum: ['in_flight', 'steering', 'followup'],
              },
            },
          },
          resources: {},
        },
        queueMessage: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentQueueMessage',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              messageId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              expectedRevision: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 9007199254740991,
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'messageId', 'expectedRevision'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentQueueMessageResult',
            oneOf: [
              {
                type: 'object',
                properties: {
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  hasAttachments: {
                    type: 'boolean',
                  },
                  message: {
                    type: 'string',
                    maxLength: 65536,
                  },
                  kind: {
                    type: 'string',
                    const: 'ready',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'revision', 'hasAttachments', 'message'],
              },
              {
                type: 'object',
                properties: {
                  expected: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  actual: {
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
                required: ['kind', 'expected', 'actual'],
              },
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'missing',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
            ],
          },
          resources: {},
        },
        respond: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentInteractionRespond',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              requestId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              answer: {
                $ref: '#/$defs/InteractionAnswer',
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'requestId', 'answer'],
            $defs: {
              InteractionAnswer: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      decision: {
                        $ref: '#/$defs/Decision',
                      },
                      kind: {
                        type: 'string',
                        const: 'service_approval',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'decision'],
                  },
                  {
                    type: 'object',
                    properties: {
                      decision: {
                        $ref: '#/$defs/Decision2',
                      },
                      kind: {
                        type: 'string',
                        const: 'permissions',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'decision'],
                  },
                  {
                    type: 'object',
                    properties: {
                      answers: {
                        type: 'array',
                        items: {
                          type: ['string', 'null'],
                        },
                      },
                      kind: {
                        type: 'string',
                        const: 'question',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'answers'],
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'form',
                      },
                    },
                    unevaluatedProperties: false,
                    oneOf: [
                      {
                        type: 'object',
                        properties: {
                          values: {
                            type: 'object',
                            additionalProperties: {
                              $ref: '#/$defs/FormValue',
                            },
                          },
                          action: {
                            type: 'string',
                            const: 'accept',
                          },
                        },
                        required: ['action', 'values'],
                      },
                      {
                        type: 'object',
                        properties: {
                          action: {
                            type: 'string',
                            const: 'decline',
                          },
                        },
                        required: ['action'],
                      },
                      {
                        type: 'object',
                        properties: {
                          action: {
                            type: 'string',
                            const: 'cancel',
                          },
                        },
                        required: ['action'],
                      },
                    ],
                    required: ['kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      decision: {
                        $ref: '#/$defs/Decision',
                      },
                      kind: {
                        type: 'string',
                        const: 'client_capability',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'decision'],
                  },
                ],
              },
              Decision: {
                type: 'string',
                enum: ['allow', 'deny'],
              },
              Decision2: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      decision: {
                        type: 'string',
                        const: 'deny',
                      },
                    },
                    required: ['decision'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    properties: {
                      permissions: {
                        $ref: '#/$defs/Permissions',
                      },
                      scope: {
                        $ref: '#/$defs/Scope2',
                      },
                      decision: {
                        type: 'string',
                        const: 'allow',
                      },
                    },
                    additionalProperties: false,
                    required: ['decision', 'permissions', 'scope'],
                  },
                ],
              },
              Permissions: {
                type: 'object',
                properties: {
                  filesystem: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/Rule',
                    },
                    maxItems: 32,
                  },
                  network: {
                    $ref: '#/$defs/Network',
                  },
                },
                additionalProperties: false,
                required: ['filesystem', 'network'],
              },
              Rule: {
                description:
                  'Paths have executor-native meaning. Validate and materialize them on the\nexecution host, never on a remote Desktop. This document alone grants nothing.',
                type: 'object',
                properties: {
                  path: {
                    type: 'string',
                  },
                  scope: {
                    $ref: '#/$defs/Scope',
                  },
                  access: {
                    $ref: '#/$defs/Access',
                  },
                },
                additionalProperties: false,
                required: ['path', 'scope', 'access'],
              },
              Scope: {
                type: 'string',
                enum: ['subtree', 'exact'],
              },
              Access: {
                description:
                  'Equal-path conflict priority is different from authority breadth: deny wins.',
                type: 'string',
                enum: ['read', 'write', 'deny'],
              },
              Network: {
                oneOf: [
                  {
                    type: 'string',
                    enum: ['denied', 'allowed'],
                  },
                  {
                    type: 'object',
                    properties: {
                      restricted: {
                        type: 'object',
                        properties: {
                          destinations: {
                            type: 'array',
                            uniqueItems: true,
                            items: {
                              $ref: '#/$defs/Destination',
                            },
                            maxItems: 128,
                          },
                        },
                        additionalProperties: false,
                        required: ['destinations'],
                      },
                    },
                    required: ['restricted'],
                    additionalProperties: false,
                  },
                ],
              },
              Destination: {
                description:
                  'Destination authority, independent of the upstream proxy used to reach it.\nA domain grant does not authorize its subdomains or other ports.',
                type: 'object',
                properties: {
                  host: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 253,
                  },
                  port: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 1,
                    maximum: 65535,
                  },
                },
                additionalProperties: false,
                required: ['host', 'port'],
              },
              Scope2: {
                type: 'string',
                enum: ['once', 'turn', 'session'],
              },
              FormValue: {
                anyOf: [
                  {
                    type: 'string',
                  },
                  {
                    type: 'number',
                    format: 'double',
                  },
                  {
                    type: 'boolean',
                  },
                  {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                ],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentInteractionResult',
            oneOf: [
              {
                description:
                  'Includes a concurrent winner. Consumers inspect the canonical outcome.',
                type: 'object',
                properties: {
                  record: {
                    $ref: '#/$defs/InteractionRecord',
                  },
                  kind: {
                    type: 'string',
                    const: 'recorded',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'record'],
              },
              {
                type: 'object',
                properties: {
                  reason: {
                    $ref: '#/$defs/AgentInteractionRejection',
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
              InteractionRecord: {
                type: 'object',
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  turnId: {
                    type: 'string',
                  },
                  runId: {
                    type: 'string',
                  },
                  requestId: {
                    type: 'string',
                  },
                  createdAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  request: {
                    $ref: '#/$defs/InteractionRequest',
                  },
                  outcome: {
                    anyOf: [
                      {
                        $ref: '#/$defs/InteractionOutcome',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                },
                additionalProperties: false,
                required: [
                  'sessionId',
                  'turnId',
                  'runId',
                  'requestId',
                  'createdAt',
                  'request',
                  'outcome',
                ],
              },
              InteractionRequest: {
                oneOf: [
                  {
                    description:
                      'Consent for one already journaled service call; never a service grant.',
                    type: 'object',
                    properties: {
                      toolUseId: {
                        type: 'string',
                      },
                      toolName: {
                        type: 'string',
                      },
                      service: {
                        type: 'string',
                      },
                      method: {
                        type: 'string',
                      },
                      binding: {
                        type: 'string',
                      },
                      through: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'service_approval',
                      },
                    },
                    additionalProperties: false,
                    required: [
                      'kind',
                      'toolUseId',
                      'toolName',
                      'service',
                      'method',
                      'binding',
                      'through',
                    ],
                  },
                  {
                    type: 'object',
                    properties: {
                      toolUseId: {
                        type: ['string', 'null'],
                      },
                      baseRevision: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      request: {
                        $ref: '#/$defs/PermissionRequest',
                      },
                      kind: {
                        type: 'string',
                        const: 'permissions',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'toolUseId', 'baseRevision', 'request'],
                  },
                  {
                    type: 'object',
                    properties: {
                      toolUseId: {
                        type: 'string',
                      },
                      questions: {
                        type: 'array',
                        items: {
                          $ref: '#/$defs/InteractionQuestion',
                        },
                      },
                      kind: {
                        type: 'string',
                        const: 'question',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'toolUseId', 'questions'],
                  },
                  {
                    type: 'object',
                    properties: {
                      toolUseId: {
                        type: 'string',
                      },
                      message: {
                        type: 'string',
                      },
                      requester: {
                        $ref: '#/$defs/FormRequester',
                      },
                      fields: {
                        type: 'array',
                        items: {
                          $ref: '#/$defs/FormField',
                        },
                      },
                      kind: {
                        type: 'string',
                        const: 'form',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'toolUseId', 'message', 'requester', 'fields'],
                  },
                  {
                    type: 'object',
                    properties: {
                      toolUseId: {
                        type: 'string',
                      },
                      target: {
                        $ref: '#/$defs/GrantTarget',
                      },
                      kind: {
                        type: 'string',
                        const: 'client_capability',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'toolUseId', 'target'],
                  },
                ],
              },
              PermissionRequest: {
                description:
                  'The Host supplies the real operation description; it is display information,\nnot authority inferred from a command string or a plugin-supplied Session ID.',
                type: 'object',
                properties: {
                  reason: {
                    type: 'string',
                  },
                  command: {
                    anyOf: [
                      {
                        $ref: '#/$defs/PermissionCommand',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  permissions: {
                    $ref: '#/$defs/Permissions',
                  },
                },
                additionalProperties: false,
                required: ['reason', 'permissions'],
              },
              PermissionCommand: {
                type: 'object',
                properties: {
                  command: {
                    type: 'string',
                  },
                  cwd: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['command', 'cwd'],
              },
              Permissions: {
                type: 'object',
                properties: {
                  filesystem: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/Rule',
                    },
                    maxItems: 32,
                  },
                  network: {
                    $ref: '#/$defs/Network',
                  },
                },
                additionalProperties: false,
                required: ['filesystem', 'network'],
              },
              Rule: {
                description:
                  'Paths have executor-native meaning. Validate and materialize them on the\nexecution host, never on a remote Desktop. This document alone grants nothing.',
                type: 'object',
                properties: {
                  path: {
                    type: 'string',
                  },
                  scope: {
                    $ref: '#/$defs/Scope',
                  },
                  access: {
                    $ref: '#/$defs/Access',
                  },
                },
                additionalProperties: false,
                required: ['path', 'scope', 'access'],
              },
              Scope: {
                type: 'string',
                enum: ['subtree', 'exact'],
              },
              Access: {
                description:
                  'Equal-path conflict priority is different from authority breadth: deny wins.',
                type: 'string',
                enum: ['read', 'write', 'deny'],
              },
              Network: {
                oneOf: [
                  {
                    type: 'string',
                    enum: ['denied', 'allowed'],
                  },
                  {
                    type: 'object',
                    properties: {
                      restricted: {
                        type: 'object',
                        properties: {
                          destinations: {
                            type: 'array',
                            uniqueItems: true,
                            items: {
                              $ref: '#/$defs/Destination',
                            },
                            maxItems: 128,
                          },
                        },
                        additionalProperties: false,
                        required: ['destinations'],
                      },
                    },
                    required: ['restricted'],
                    additionalProperties: false,
                  },
                ],
              },
              Destination: {
                description:
                  'Destination authority, independent of the upstream proxy used to reach it.\nA domain grant does not authorize its subdomains or other ports.',
                type: 'object',
                properties: {
                  host: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 253,
                  },
                  port: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 1,
                    maximum: 65535,
                  },
                },
                additionalProperties: false,
                required: ['host', 'port'],
              },
              InteractionQuestion: {
                type: 'object',
                properties: {
                  question: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 1024,
                  },
                  options: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/QuestionOption',
                    },
                    minItems: 2,
                    maxItems: 3,
                  },
                },
                additionalProperties: false,
                required: ['question', 'options'],
              },
              QuestionOption: {
                type: 'object',
                properties: {
                  label: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 256,
                  },
                  description: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 512,
                  },
                },
                additionalProperties: false,
                required: ['label'],
              },
              FormRequester: {
                type: 'object',
                properties: {
                  name: {
                    type: 'string',
                  },
                  source: {
                    type: ['string', 'null'],
                  },
                },
                additionalProperties: false,
                required: ['name'],
              },
              FormField: {
                type: 'object',
                properties: {
                  name: {
                    type: 'string',
                  },
                  label: {
                    type: 'string',
                  },
                  required: {
                    type: 'boolean',
                  },
                  description: {
                    type: ['string', 'null'],
                  },
                },
                required: ['name', 'label', 'required'],
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      default: {
                        type: ['string', 'null'],
                      },
                      minLength: {
                        type: ['integer', 'null'],
                        format: 'uint',
                        minimum: 0,
                      },
                      maxLength: {
                        type: ['integer', 'null'],
                        format: 'uint',
                        minimum: 0,
                      },
                      format: {
                        anyOf: [
                          {
                            $ref: '#/$defs/FormFormat',
                          },
                          {
                            type: 'null',
                          },
                        ],
                      },
                      kind: {
                        type: 'string',
                        const: 'string',
                      },
                    },
                    required: ['kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      default: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      minimum: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      maximum: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      kind: {
                        type: 'string',
                        const: 'number',
                      },
                    },
                    required: ['kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      default: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      minimum: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      maximum: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      kind: {
                        type: 'string',
                        const: 'integer',
                      },
                    },
                    required: ['kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      default: {
                        type: ['boolean', 'null'],
                      },
                      kind: {
                        type: 'string',
                        const: 'boolean',
                      },
                    },
                    required: ['kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      options: {
                        type: 'array',
                        items: {
                          $ref: '#/$defs/FormOption',
                        },
                      },
                      default: {
                        type: ['string', 'null'],
                      },
                      kind: {
                        type: 'string',
                        const: 'single_select',
                      },
                    },
                    required: ['kind', 'options'],
                  },
                  {
                    type: 'object',
                    properties: {
                      options: {
                        type: 'array',
                        items: {
                          $ref: '#/$defs/FormOption',
                        },
                      },
                      default: {
                        type: ['array', 'null'],
                        items: {
                          type: 'string',
                        },
                      },
                      minItems: {
                        type: ['integer', 'null'],
                        format: 'uint',
                        minimum: 0,
                      },
                      maxItems: {
                        type: ['integer', 'null'],
                        format: 'uint',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'multi_select',
                      },
                    },
                    required: ['kind', 'options'],
                  },
                ],
              },
              FormFormat: {
                type: 'string',
                enum: ['email', 'uri', 'date', 'date-time'],
              },
              FormOption: {
                type: 'object',
                properties: {
                  value: {
                    type: 'string',
                  },
                  label: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['value', 'label'],
              },
              GrantTarget: {
                type: 'object',
                properties: {
                  providerId: {
                    type: 'string',
                  },
                  contractId: {
                    type: 'string',
                  },
                  serverId: {
                    type: 'string',
                  },
                  toolName: {
                    type: 'string',
                  },
                  capability: {
                    $ref: '#/$defs/GrantCapability',
                  },
                  scope: {
                    $ref: '#/$defs/GrantScope',
                  },
                },
                required: [
                  'providerId',
                  'contractId',
                  'serverId',
                  'toolName',
                  'capability',
                  'scope',
                ],
              },
              GrantCapability: {
                type: 'string',
                enum: ['browser', 'computer_use', 'desktop_mcp', 'mcp'],
              },
              GrantScope: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      origin: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'browser_origin',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'origin'],
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'capability',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      serverId: {
                        type: 'string',
                      },
                      toolName: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'mcp_tool',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'serverId', 'toolName'],
                  },
                ],
              },
              InteractionOutcome: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      decision: {
                        $ref: '#/$defs/Decision',
                      },
                      committedAt: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'service_approval_decision',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'decision', 'committedAt'],
                  },
                  {
                    type: 'object',
                    properties: {
                      decision: {
                        $ref: '#/$defs/Decision2',
                      },
                      committedAt: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'permissions_decision',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'decision', 'committedAt'],
                  },
                  {
                    type: 'object',
                    properties: {
                      answers: {
                        type: 'array',
                        items: {
                          type: ['string', 'null'],
                        },
                      },
                      committedAt: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'question_answer',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'answers', 'committedAt'],
                  },
                  {
                    type: 'object',
                    properties: {
                      committedAt: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'form_answer',
                      },
                    },
                    unevaluatedProperties: false,
                    oneOf: [
                      {
                        type: 'object',
                        properties: {
                          values: {
                            type: 'object',
                            additionalProperties: {
                              $ref: '#/$defs/FormValue',
                            },
                          },
                          action: {
                            type: 'string',
                            const: 'accept',
                          },
                        },
                        required: ['action', 'values'],
                      },
                      {
                        type: 'object',
                        properties: {
                          action: {
                            type: 'string',
                            const: 'decline',
                          },
                        },
                        required: ['action'],
                      },
                      {
                        type: 'object',
                        properties: {
                          action: {
                            type: 'string',
                            const: 'cancel',
                          },
                        },
                        required: ['action'],
                      },
                    ],
                    required: ['kind', 'committedAt'],
                  },
                  {
                    type: 'object',
                    properties: {
                      decision: {
                        $ref: '#/$defs/Decision',
                      },
                      committedAt: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'client_capability_decision',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'decision', 'committedAt'],
                  },
                  {
                    type: 'object',
                    properties: {
                      reason: {
                        $ref: '#/$defs/ClosureReason',
                      },
                      committedAt: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'closure',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'reason', 'committedAt'],
                  },
                ],
              },
              Decision: {
                type: 'string',
                enum: ['allow', 'deny'],
              },
              Decision2: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      decision: {
                        type: 'string',
                        const: 'deny',
                      },
                    },
                    required: ['decision'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    properties: {
                      permissions: {
                        $ref: '#/$defs/Permissions',
                      },
                      scope: {
                        $ref: '#/$defs/Scope2',
                      },
                      decision: {
                        type: 'string',
                        const: 'allow',
                      },
                    },
                    additionalProperties: false,
                    required: ['decision', 'permissions', 'scope'],
                  },
                ],
              },
              Scope2: {
                type: 'string',
                enum: ['once', 'turn', 'session'],
              },
              FormValue: {
                anyOf: [
                  {
                    type: 'string',
                  },
                  {
                    type: 'number',
                    format: 'double',
                  },
                  {
                    type: 'boolean',
                  },
                  {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                ],
              },
              ClosureReason: {
                type: 'string',
                enum: [
                  'turn_stopped',
                  'turn_terminal',
                  'producer_cancelled',
                  'timed_out',
                  'host_restarted',
                  'provider_disconnected',
                ],
              },
              AgentInteractionRejection: {
                type: 'string',
                enum: ['closed', 'unavailable', 'invalid_answer', 'unsupported'],
              },
            },
          },
          resources: {},
        },
        resume: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentResume',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              sourceInvocationId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              sourceHighWater: {
                description: 'Exact Run-local count returned by execution.',
                type: 'integer',
                format: 'uint64',
                minimum: 1,
                maximum: 9007199254740991,
              },
              model: {
                description:
                  'Explicit model selection for this new continuation; original input is replayed.',
                $ref: '#/$defs/ModelSelection',
              },
              thinkingLevel: {
                anyOf: [
                  {
                    $ref: '#/$defs/ThinkingLevel',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'sourceInvocationId', 'sourceHighWater', 'model'],
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
              ThinkingLevel: {
                type: 'string',
                enum: ['off', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max', 'ultra'],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentResumeResult',
            oneOf: [
              {
                type: 'object',
                properties: {
                  execution: {
                    $ref: '#/$defs/AgentExecution',
                  },
                  kind: {
                    type: 'string',
                    const: 'started',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'execution'],
              },
              {
                type: 'object',
                properties: {
                  reason: {
                    $ref: '#/$defs/AgentResumeParkReason',
                  },
                  kind: {
                    type: 'string',
                    const: 'parked',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'reason'],
              },
            ],
            $defs: {
              AgentExecution: {
                description:
                  'Canonical physical execution receipt, independent of the current Session tip.',
                type: 'object',
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  turnId: {
                    type: 'string',
                  },
                  runId: {
                    type: 'string',
                  },
                  invocationId: {
                    type: 'string',
                  },
                  highWater: {
                    description: 'Run-local event count, not the root-global transcript cursor.',
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  state: {
                    $ref: '#/$defs/AgentExecutionState',
                  },
                },
                additionalProperties: false,
                required: ['sessionId', 'turnId', 'runId', 'invocationId', 'highWater', 'state'],
              },
              AgentExecutionState: {
                type: 'string',
                enum: [
                  'admitted',
                  'running',
                  'waiting',
                  'paused',
                  'completed',
                  'failed',
                  'cancelled',
                ],
              },
              AgentResumeParkReason: {
                type: 'string',
                enum: [
                  'closed',
                  'archived',
                  'busy',
                  'capacity',
                  'source_missing',
                  'source_changed',
                  'source_not_resumable',
                  'source_unreadable',
                  'already_continued',
                  'workspace_changed',
                  'safety_check_failed',
                ],
              },
            },
          },
          resources: {},
        },
        search: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentHistorySearch',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              through: {
                description: 'Original raw-event fence, shared with history reads.',
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 9007199254740991,
              },
              after: {
                description:
                  'Exclusive transcript-row sequence returned by the previous batch; zero starts a scan.',
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 9007199254740991,
              },
              query: {
                description:
                  'Literal, case-sensitive query, additionally limited to 512 UTF-8 bytes.',
                type: 'string',
                minLength: 1,
                maxLength: 512,
              },
              includeInternal: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'through', 'after', 'query', 'includeInternal'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentHistorySearchPage',
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
                  matches: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/AgentHistoryMatch',
                    },
                    maxItems: 16,
                  },
                  nextAfter: {
                    description:
                      'Continue even when this batch has no matches. Null is the end of the scan.',
                    type: ['integer', 'null'],
                    format: 'uint64',
                    minimum: 0,
                  },
                  kind: {
                    type: 'string',
                    const: 'ready',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'through', 'matches', 'nextAfter'],
              },
            ],
            $defs: {
              AgentHistoryMatch: {
                type: 'object',
                properties: {
                  field: {
                    description:
                      'Exact canonical field to read through the source query; history body may omit it.',
                    $ref: '#/$defs/SourceField',
                  },
                  cursor: {
                    description:
                      'Use this exact cursor with HistoryPosition::After and the same raw-event fence.',
                    $ref: '#/$defs/Cursor',
                  },
                  preview: {
                    type: 'string',
                    maxLength: 384,
                  },
                },
                additionalProperties: false,
                required: ['field', 'cursor', 'preview'],
              },
              SourceField: {
                description:
                  'An explicitly selected canonical transcript field, never an execution grant.',
                oneOf: [
                  {
                    type: 'string',
                    enum: [
                      'body',
                      'thinking',
                      'tool_name',
                      'failure_message',
                      'error_class',
                      'arguments',
                      'result',
                    ],
                  },
                  {
                    description:
                      'The original service operation observation, separate from its returned value.',
                    type: 'string',
                    const: 'receipt',
                  },
                ],
              },
              Cursor: {
                description:
                  "Byte offset in one message field's UTF-8 text, not a bearer capability.",
                type: 'object',
                properties: {
                  field: {
                    $ref: '#/$defs/HistoryField',
                    default: 'body',
                  },
                  sequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 1,
                    maximum: 9007199254740991,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                    maximum: 9007199254740991,
                  },
                },
                additionalProperties: false,
                required: ['sequence', 'offset'],
              },
              HistoryField: {
                description: 'A readable field in a conversation window, in presentation order.',
                type: 'string',
                enum: ['thinking', 'body'],
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
              attachments: {
                description:
                  'IDs of immutable user uploads in this Session; metadata is resolved by the owner.',
                type: 'array',
                items: {
                  type: 'string',
                },
                maxItems: 8,
                default: [],
              },
              model: {
                $ref: '#/$defs/ModelSelection',
              },
              thinkingLevel: {
                description: 'Omission/null uses the configured model default; `off` is explicit.',
                anyOf: [
                  {
                    $ref: '#/$defs/ThinkingLevel',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              message: {
                type: 'string',
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
              ThinkingLevel: {
                type: 'string',
                enum: ['off', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max', 'ultra'],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentSendResult',
            oneOf: [
              {
                type: 'object',
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  reason: {
                    $ref: '#/$defs/AgentSkillInputRejection',
                  },
                  skills: {
                    anyOf: [
                      {
                        $ref: '#/$defs/SkillInvocationResult',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'sessionId', 'reason'],
              },
              {
                type: 'object',
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  invocationId: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'started',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'sessionId', 'invocationId'],
              },
              {
                type: 'object',
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  messageId: {
                    type: 'string',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  kind: {
                    type: 'string',
                    const: 'queued',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'sessionId', 'messageId', 'revision'],
              },
            ],
            $defs: {
              AgentSkillInputRejection: {
                type: 'string',
                enum: ['skills_unavailable', 'input_too_large'],
              },
              SkillInvocationResult: {
                type: 'object',
                properties: {
                  loaded: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/LoadedSkill',
                    },
                  },
                  failed: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/SkillInvocationFailure',
                    },
                  },
                  receipts: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/SkillInvocationReceipt',
                    },
                  },
                },
                additionalProperties: false,
                required: ['loaded', 'failed', 'receipts'],
              },
              LoadedSkill: {
                type: 'object',
                properties: {
                  id: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['id', 'name'],
              },
              SkillInvocationFailure: {
                anyOf: [
                  {
                    $ref: '#/$defs/SkillRequestFailure',
                  },
                  {
                    $ref: '#/$defs/SkillOverflowFailure',
                  },
                ],
              },
              SkillRequestFailure: {
                type: 'object',
                properties: {
                  request: {
                    type: 'string',
                  },
                  reason: {
                    $ref: '#/$defs/SkillFailureReason',
                  },
                },
                additionalProperties: false,
                required: ['request', 'reason'],
              },
              SkillFailureReason: {
                type: 'string',
                enum: [
                  'invalid_name',
                  'not_found',
                  'disabled',
                  'host_incompatible',
                  'resolution_failed',
                ],
              },
              SkillOverflowFailure: {
                type: 'object',
                properties: {
                  reason: {
                    $ref: '#/$defs/TooManyRequests',
                  },
                  requestLimit: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['reason', 'requestLimit'],
              },
              TooManyRequests: {
                type: 'string',
                enum: ['too_many_requests'],
              },
              SkillInvocationReceipt: {
                description:
                  'Boolean wire discriminants are validated here, not retained as mutable\nbusiness fields beside a contradictory enum variant.',
                anyOf: [
                  {
                    type: 'object',
                    properties: {
                      invocation: {
                        $ref: '#/$defs/SkillInvocationMode',
                      },
                      request: {
                        type: 'string',
                      },
                      success: {
                        const: true,
                      },
                      ref: {
                        type: 'string',
                      },
                      id: {
                        type: 'string',
                      },
                      name: {
                        type: 'string',
                      },
                      scope: {
                        $ref: '#/$defs/SkillScope',
                      },
                      source: {
                        $ref: '#/$defs/SkillSource',
                      },
                      truncated: {
                        type: 'boolean',
                      },
                    },
                    additionalProperties: false,
                    required: [
                      'invocation',
                      'request',
                      'success',
                      'ref',
                      'id',
                      'name',
                      'scope',
                      'source',
                      'truncated',
                    ],
                  },
                  {
                    type: 'object',
                    properties: {
                      invocation: {
                        $ref: '#/$defs/SkillInvocationMode',
                      },
                      request: {
                        type: 'string',
                      },
                      success: {
                        const: false,
                      },
                      reason: {
                        $ref: '#/$defs/SkillFailureReason',
                      },
                    },
                    additionalProperties: false,
                    required: ['invocation', 'request', 'success', 'reason'],
                  },
                  {
                    type: 'object',
                    properties: {
                      invocation: {
                        const: 'explicit',
                      },
                      success: {
                        const: false,
                      },
                      reason: {
                        $ref: '#/$defs/TooManyRequests',
                      },
                      requestLimit: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                    },
                    additionalProperties: false,
                    required: ['invocation', 'success', 'reason', 'requestLimit'],
                  },
                ],
              },
              SkillInvocationMode: {
                type: 'string',
                enum: ['explicit', 'model_tool'],
              },
              SkillScope: {
                type: 'string',
                enum: ['project', 'workspace', 'user', 'custom'],
              },
              SkillSource: {
                type: 'string',
                enum: ['maka', 'agents', 'legacy', 'custom'],
              },
            },
          },
          resources: {},
        },
        source: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentHistorySourceRead',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              through: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 9007199254740991,
              },
              source: {
                $ref: '#/$defs/AgentHistorySourceReference',
              },
              field: {
                $ref: '#/$defs/SourceField',
              },
              path: {
                description:
                  'Optional JSON keys/array indices within arguments, result or receipt.',
                type: ['array', 'null'],
                items: {
                  $ref: '#/$defs/HistorySourceSegment',
                },
                maxItems: 16,
                default: null,
              },
              position: {
                $ref: '#/$defs/SourcePosition',
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'through', 'source', 'field', 'position'],
            $defs: {
              AgentHistorySourceReference: {
                oneOf: [
                  {
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
                        const: 'sequence',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'sequence'],
                  },
                  {
                    type: 'object',
                    properties: {
                      toolUseId: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 256,
                      },
                      kind: {
                        type: 'string',
                        const: 'tool_use',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'toolUseId'],
                  },
                ],
              },
              SourceField: {
                description:
                  'An explicitly selected canonical transcript field, never an execution grant.',
                oneOf: [
                  {
                    type: 'string',
                    enum: [
                      'body',
                      'thinking',
                      'tool_name',
                      'failure_message',
                      'error_class',
                      'arguments',
                      'result',
                    ],
                  },
                  {
                    description:
                      'The original service operation observation, separate from its returned value.',
                    type: 'string',
                    const: 'receipt',
                  },
                ],
              },
              HistorySourceSegment: {
                description:
                  'A structural selector into an original JSON field. Keys and array indices\nremain distinct; neither locates a filesystem resource nor grants access.',
                anyOf: [
                  {
                    type: 'string',
                    maxLength: 256,
                  },
                  {
                    type: 'integer',
                    format: 'uint32',
                    minimum: 0,
                  },
                ],
              },
              SourcePosition: {
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
                      kind: {
                        type: 'string',
                        const: 'tail',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    properties: {
                      offset: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                        maximum: 9007199254740991,
                      },
                      kind: {
                        type: 'string',
                        const: 'before',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'offset'],
                  },
                  {
                    type: 'object',
                    properties: {
                      offset: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                        maximum: 9007199254740991,
                      },
                      kind: {
                        type: 'string',
                        const: 'after',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'offset'],
                  },
                ],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentHistorySourcePage',
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
                  source: {
                    $ref: '#/$defs/HistorySource',
                  },
                  filePath: {
                    description:
                      'Complete file-path evidence resolved on the Host for copying only.\nThis is display text, never a filesystem capability or open command.',
                    type: ['string', 'null'],
                    maxLength: 4096,
                  },
                  kind: {
                    type: 'string',
                    const: 'ready',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'through', 'source'],
              },
            ],
            $defs: {
              HistorySource: {
                description:
                  'One literal UTF-8 excerpt of an immutable field. JSON excerpts need not be\nstandalone JSON. Image descriptors carry no artifact read authorization.',
                type: 'object',
                properties: {
                  sequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 1,
                    maximum: 9007199254740991,
                  },
                  timestamp: {
                    description:
                      'Host-recorded milliseconds of this immutable row; not a client clock.',
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                    maximum: 9007199254740991,
                  },
                  role: {
                    $ref: '#/$defs/HistorySourceRole',
                  },
                  fields: {
                    description:
                      'Readable canonical fields of this same immutable row, in display order.',
                    type: 'array',
                    items: {
                      $ref: '#/$defs/SourceField',
                    },
                    minItems: 1,
                    maxItems: 7,
                  },
                  field: {
                    $ref: '#/$defs/SourceField',
                  },
                  path: {
                    description:
                      'Omission reads the encoded field. A selected JSON string is literal text.',
                    type: ['array', 'null'],
                    items: {
                      $ref: '#/$defs/HistorySourceSegment',
                    },
                    maxItems: 16,
                  },
                  format: {
                    $ref: '#/$defs/SourceFormat',
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                    maximum: 9007199254740991,
                  },
                  totalBytes: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                    maximum: 9007199254740991,
                  },
                  text: {
                    type: 'string',
                  },
                  tool: {
                    anyOf: [
                      {
                        $ref: '#/$defs/HistoryTool',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                },
                additionalProperties: false,
                required: [
                  'sequence',
                  'timestamp',
                  'role',
                  'fields',
                  'field',
                  'format',
                  'offset',
                  'totalBytes',
                  'text',
                  'tool',
                ],
              },
              HistorySourceRole: {
                description:
                  'Presentation role of the exact row, including diagnostics omitted from the\nordinary conversation window. This carries no execution authority.',
                type: 'string',
                enum: ['user', 'assistant', 'tool_call', 'tool_result', 'failure'],
              },
              SourceField: {
                description:
                  'An explicitly selected canonical transcript field, never an execution grant.',
                oneOf: [
                  {
                    type: 'string',
                    enum: [
                      'body',
                      'thinking',
                      'tool_name',
                      'failure_message',
                      'error_class',
                      'arguments',
                      'result',
                    ],
                  },
                  {
                    description:
                      'The original service operation observation, separate from its returned value.',
                    type: 'string',
                    const: 'receipt',
                  },
                ],
              },
              HistorySourceSegment: {
                description:
                  'A structural selector into an original JSON field. Keys and array indices\nremain distinct; neither locates a filesystem resource nor grants access.',
                anyOf: [
                  {
                    type: 'string',
                    maxLength: 256,
                  },
                  {
                    type: 'integer',
                    format: 'uint32',
                    minimum: 0,
                  },
                ],
              },
              SourceFormat: {
                type: 'string',
                enum: ['text', 'json', 'image_descriptor'],
              },
              HistoryTool: {
                description:
                  'Canonical references at the requested fence, never execution authority.\nBoth references are present only for a unique same-Turn call/result pair\nwith matching origin and parentage. An unpaired row retains its own reference.',
                type: 'object',
                properties: {
                  toolUseId: {
                    type: 'string',
                  },
                  call: {
                    anyOf: [
                      {
                        $ref: '#/$defs/HistoryToolCall',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  result: {
                    anyOf: [
                      {
                        $ref: '#/$defs/HistoryToolResult',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                },
                additionalProperties: false,
                required: ['toolUseId', 'call', 'result'],
              },
              HistoryToolCall: {
                type: 'object',
                properties: {
                  presentation: {
                    description:
                      'Opaque display contract captured at dispatch; no execution authority.',
                    type: ['string', 'null'],
                    default: null,
                  },
                  sequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 1,
                    maximum: 9007199254740991,
                  },
                  name: {
                    type: 'string',
                  },
                  title: {
                    anyOf: [
                      {
                        $ref: '#/$defs/DisplayText',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  pending: {
                    description:
                      'This native Turn is still open at the history fence; not execution authority.',
                    type: 'boolean',
                  },
                  dispatched: {
                    description:
                      'A unique original Host dispatch is recorded at this fence; not liveness.',
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['sequence', 'name', 'title', 'pending', 'dispatched'],
              },
              DisplayText: {
                description:
                  'Presentation text is distinct from the stable identity used for navigation.',
                type: 'object',
                properties: {
                  fallback: {
                    type: 'string',
                  },
                  translations: {
                    type: 'object',
                    additionalProperties: {
                      type: 'string',
                    },
                    default: {},
                  },
                },
                additionalProperties: false,
                required: ['fallback'],
              },
              HistoryToolResult: {
                type: 'object',
                properties: {
                  settlement: {
                    anyOf: [
                      {
                        $ref: '#/$defs/ToolSettlement',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  sequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 1,
                    maximum: 9007199254740991,
                  },
                  isError: {
                    description: 'Does not distinguish failed from unknown effects.',
                    type: 'boolean',
                  },
                  image: {
                    description:
                      'Canonical content is an image; this does not grant artifact reads.',
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: ['settlement', 'sequence', 'isError', 'image'],
              },
              ToolSettlement: {
                description:
                  'Recorded Host settlement for presentation; returned is not business success.\nForeign/provider results do not establish this Host evidence.',
                type: 'string',
                enum: ['returned', 'failed', 'unknown', 'rejected'],
              },
            },
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
        toolImage: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentToolImageRead',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              through: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 9007199254740991,
              },
              sequence: {
                type: 'integer',
                format: 'uint64',
                minimum: 1,
                maximum: 9007199254740991,
              },
              offset: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 5242880,
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'through', 'sequence', 'offset'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentToolImagePage',
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
                    minimum: 1,
                    maximum: 5242880,
                  },
                  mimeType: {
                    type: 'string',
                    pattern: '^image/(png|jpeg|gif|webp|avif)$',
                  },
                  digest: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  data: {
                    description: 'Standard padded Base64 of at most 32KiB of encoded image bytes.',
                    type: 'string',
                    maxLength: 43692,
                    pattern: '^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$',
                  },
                  kind: {
                    type: 'string',
                    const: 'ready',
                  },
                },
                additionalProperties: false,
                required: [
                  'kind',
                  'through',
                  'sequence',
                  'offset',
                  'totalBytes',
                  'mimeType',
                  'digest',
                  'data',
                ],
              },
            ],
          },
          resources: {},
        },
        uploadAttachment: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentUploadAttachment',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              name: {
                type: 'string',
                minLength: 1,
                maxLength: 512,
              },
              bytes: {
                type: 'integer',
                format: 'uint',
                minimum: 0,
                maximum: 52428800,
              },
              digest: {
                type: 'string',
                pattern: '^sha256:[a-f0-9]{64}$',
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'name', 'bytes', 'digest'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentAttachmentUploadResult',
            oneOf: [
              {
                type: 'object',
                properties: {
                  attachment: {
                    $ref: '#/$defs/AgentAttachment',
                  },
                  kind: {
                    type: 'string',
                    const: 'uploaded',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'attachment'],
              },
              {
                type: 'object',
                properties: {
                  reason: {
                    $ref: '#/$defs/AgentAttachmentRejection',
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
              AgentAttachment: {
                type: 'object',
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  id: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                  mimeType: {
                    type: 'string',
                  },
                  bytes: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                additionalProperties: false,
                required: ['sessionId', 'id', 'name', 'mimeType', 'bytes'],
              },
              AgentAttachmentRejection: {
                type: 'string',
                enum: ['invalid_file', 'busy', 'closed', 'archived'],
              },
            },
          },
          resources: {
            file: {
              name: 'maka.bytes',
              version: 1,
            },
          },
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
            oneOf: [
              {
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
                  kind: {
                    type: 'string',
                    const: 'events',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'through', 'nextAfter', 'events'],
              },
              {
                type: 'object',
                properties: {
                  queue: {
                    $ref: '#/$defs/AgentQueue',
                  },
                  kind: {
                    type: 'string',
                    const: 'queue',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'queue'],
              },
              {
                type: 'object',
                properties: {
                  pending: {
                    $ref: '#/$defs/AgentPendingInteractions',
                  },
                  kind: {
                    type: 'string',
                    const: 'interactions',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'pending'],
              },
            ],
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
                  turnId: {
                    type: 'string',
                  },
                  fact: {
                    $ref: '#/$defs/AgentDeliveryFact',
                  },
                },
                additionalProperties: false,
                required: ['sequence', 'invocationId', 'turnId', 'fact'],
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
              AgentQueue: {
                type: 'object',
                properties: {
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  entries: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/AgentQueueEntry',
                    },
                    maxItems: 64,
                  },
                },
                additionalProperties: false,
                required: ['revision', 'entries'],
              },
              AgentQueueEntry: {
                type: 'object',
                properties: {
                  messageId: {
                    type: 'string',
                  },
                  kind: {
                    $ref: '#/$defs/AgentQueueKind',
                  },
                  preview: {
                    type: 'string',
                    maxLength: 80,
                  },
                },
                additionalProperties: false,
                required: ['messageId', 'kind', 'preview'],
              },
              AgentQueueKind: {
                type: 'string',
                enum: ['in_flight', 'steering', 'followup'],
              },
              AgentPendingInteractions: {
                type: 'object',
                properties: {
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  entries: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/AgentInteractionSummary',
                    },
                    maxItems: 16,
                  },
                },
                additionalProperties: false,
                required: ['revision', 'entries'],
              },
              AgentInteractionSummary: {
                type: 'object',
                properties: {
                  requestId: {
                    type: 'string',
                  },
                  kind: {
                    $ref: '#/$defs/AgentInteractionKind',
                  },
                  toolUseId: {
                    type: ['string', 'null'],
                  },
                },
                additionalProperties: false,
                required: ['requestId', 'kind', 'toolUseId'],
              },
              AgentInteractionKind: {
                type: 'string',
                enum: ['service_approval', 'question', 'permissions', 'form', 'client_capability'],
              },
            },
          },
          resources: {},
        },
        work: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentWorkRead',
            type: 'object',
            properties: {
              sessionId: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
              target: {
                $ref: '#/$defs/AgentWorkTarget',
              },
              cursor: {
                anyOf: [
                  {
                    $ref: '#/$defs/AgentAnswerCursor',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
            },
            additionalProperties: false,
            required: ['sessionId', 'target'],
            $defs: {
              AgentWorkTarget: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      invocationId: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 128,
                      },
                      kind: {
                        type: 'string',
                        const: 'invocation',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'invocationId'],
                  },
                  {
                    type: 'object',
                    properties: {
                      messageId: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 128,
                      },
                      kind: {
                        type: 'string',
                        const: 'message',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'messageId'],
                  },
                ],
              },
              AgentAnswerCursor: {
                type: 'object',
                properties: {
                  invocationId: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 128,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                    maximum: 9007199254740991,
                  },
                },
                additionalProperties: false,
                required: ['invocationId', 'offset'],
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'AgentWork',
            oneOf: [
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'missing',
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
                    const: 'pending',
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
                    const: 'cancelled',
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
                    const: 'changed',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
              {
                type: 'object',
                properties: {
                  shared: {
                    type: 'boolean',
                  },
                  execution: {
                    $ref: '#/$defs/AgentExecution',
                  },
                  answer: {
                    anyOf: [
                      {
                        $ref: '#/$defs/AgentAnswerPage',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  pendingInteractions: {
                    description:
                      'Count for this exact Run; original Session interactions retain all authority.',
                    type: 'integer',
                    format: 'uint32',
                    minimum: 0,
                  },
                  kind: {
                    type: 'string',
                    const: 'delivered',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'shared', 'execution', 'pendingInteractions'],
              },
            ],
            $defs: {
              AgentExecution: {
                description:
                  'Canonical physical execution receipt, independent of the current Session tip.',
                type: 'object',
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  turnId: {
                    type: 'string',
                  },
                  runId: {
                    type: 'string',
                  },
                  invocationId: {
                    type: 'string',
                  },
                  highWater: {
                    description: 'Run-local event count, not the root-global transcript cursor.',
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  state: {
                    $ref: '#/$defs/AgentExecutionState',
                  },
                },
                additionalProperties: false,
                required: ['sessionId', 'turnId', 'runId', 'invocationId', 'highWater', 'state'],
              },
              AgentExecutionState: {
                type: 'string',
                enum: [
                  'admitted',
                  'running',
                  'waiting',
                  'paused',
                  'completed',
                  'failed',
                  'cancelled',
                ],
              },
              AgentAnswerPage: {
                type: 'object',
                properties: {
                  text: {
                    type: 'string',
                  },
                  next: {
                    anyOf: [
                      {
                        $ref: '#/$defs/AgentAnswerCursor',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                },
                additionalProperties: false,
                required: ['text'],
              },
              AgentAnswerCursor: {
                type: 'object',
                properties: {
                  invocationId: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 128,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                    maximum: 9007199254740991,
                  },
                },
                additionalProperties: false,
                required: ['invocationId', 'offset'],
              },
            },
          },
          resources: {},
        },
      },
    },
  },
  workhubCoordinator: {
    name: 'maka.workhub.Coordinator',
    version: 1,
    transport: 'remote',
    interface: {
      kind: 'service',
      methods: {
        control: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              controlId: {
                type: 'string',
                pattern: '^wc_[a-f0-9]{64}$',
              },
            },
            required: ['controlId'],
            title: 'WorkHubControlRead',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            type: 'object',
            additionalProperties: false,
            properties: {
              taskId: {
                type: 'string',
              },
              controlId: {
                type: 'string',
              },
              operation: {
                type: 'string',
                format: 'uuid',
              },
              status: {
                $ref: '#/$defs/WorkHubControlStatus',
              },
            },
            required: ['taskId', 'controlId', 'operation', 'status'],
            title: 'WorkHubControl',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              WorkHubControlStatus: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'pending',
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
                        const: 'unavailable',
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
                        const: 'uncertain',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      outcome: {
                        $ref: '#/$defs/WorkHubControlOutcome',
                      },
                      kind: {
                        type: 'string',
                        const: 'returned',
                      },
                    },
                    required: ['kind', 'outcome'],
                  },
                ],
              },
              WorkHubControlOutcome: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      result: {
                        $ref: '#/$defs/AgentStopped',
                      },
                      kind: {
                        type: 'string',
                        const: 'stop',
                      },
                    },
                    required: ['kind', 'result'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      result: {
                        $ref: '#/$defs/AgentResumeResult',
                      },
                      kind: {
                        type: 'string',
                        const: 'resume',
                      },
                    },
                    required: ['kind', 'result'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      result: {
                        $ref: '#/$defs/AgentQueueMutationResult',
                      },
                      kind: {
                        type: 'string',
                        const: 'retract',
                      },
                    },
                    required: ['kind', 'result'],
                  },
                ],
              },
              AgentStopped: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  requested: {
                    type: 'boolean',
                  },
                },
                required: ['requested'],
              },
              AgentResumeResult: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      execution: {
                        $ref: '#/$defs/AgentExecution',
                      },
                      kind: {
                        type: 'string',
                        const: 'started',
                      },
                    },
                    required: ['kind', 'execution'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      reason: {
                        $ref: '#/$defs/AgentResumeParkReason',
                      },
                      kind: {
                        type: 'string',
                        const: 'parked',
                      },
                    },
                    required: ['kind', 'reason'],
                  },
                ],
              },
              AgentExecution: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  turnId: {
                    type: 'string',
                  },
                  runId: {
                    type: 'string',
                  },
                  invocationId: {
                    type: 'string',
                  },
                  highWater: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                    description: 'Run-local event count, not the root-global transcript cursor.',
                  },
                  state: {
                    $ref: '#/$defs/AgentExecutionState',
                  },
                },
                required: ['sessionId', 'turnId', 'runId', 'invocationId', 'highWater', 'state'],
                description:
                  'Canonical physical execution receipt, independent of the current Session tip.',
              },
              AgentExecutionState: {
                type: 'string',
                enum: [
                  'admitted',
                  'running',
                  'waiting',
                  'paused',
                  'completed',
                  'failed',
                  'cancelled',
                ],
              },
              AgentResumeParkReason: {
                type: 'string',
                enum: [
                  'closed',
                  'archived',
                  'busy',
                  'capacity',
                  'source_missing',
                  'source_changed',
                  'source_not_resumable',
                  'source_unreadable',
                  'already_continued',
                  'workspace_changed',
                  'safety_check_failed',
                ],
              },
              AgentQueueMutationResult: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      revision: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'committed',
                      },
                    },
                    required: ['kind', 'revision'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      expected: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      actual: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'revision_conflict',
                      },
                    },
                    required: ['kind', 'expected', 'actual'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      reason: {
                        $ref: '#/$defs/AgentQueueRejection',
                      },
                      kind: {
                        type: 'string',
                        const: 'rejected',
                      },
                    },
                    required: ['kind', 'reason'],
                  },
                ],
              },
              AgentQueueRejection: {
                type: 'string',
                enum: [
                  'closed',
                  'missing',
                  'in_flight',
                  'invalid_edit',
                  'execution_changed',
                  'not_running',
                ],
              },
            },
          },
          resources: {},
        },
        controls: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              after: {
                type: ['string', 'null'],
              },
            },
            title: 'WorkHubControlsRead',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            type: 'object',
            additionalProperties: false,
            properties: {
              entries: {
                type: 'array',
                items: {
                  $ref: '#/$defs/WorkHubControlEntry',
                },
                maxItems: 16,
              },
              after: {
                type: ['string', 'null'],
              },
            },
            required: ['entries'],
            title: 'WorkHubControls',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              WorkHubControlEntry: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  taskId: {
                    type: 'string',
                  },
                  controlId: {
                    type: 'string',
                  },
                },
                required: ['taskId', 'controlId'],
              },
            },
          },
          resources: {},
        },
        coordinator: {
          mode: 'query',
          input: {
            type: 'object',
            properties: {},
            additionalProperties: false,
          },
          output: {
            oneOf: [
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'missing',
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
                    const: 'pending',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  session: {
                    $ref: '#/$defs/AgentSession',
                  },
                  kind: {
                    type: 'string',
                    const: 'ready',
                  },
                },
                required: ['kind', 'session'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reason: {
                    $ref: '#/$defs/AgentRejection',
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                required: ['kind', 'reason'],
              },
            ],
            title: 'WorkHubCoordinator',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              AgentSession: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  dataSource: {
                    $ref: '#/$defs/AgentDataSource',
                  },
                  permissions: {
                    $ref: '#/$defs/AgentWorkspacePermissions',
                  },
                  boundaryRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  id: {
                    type: 'string',
                  },
                  title: {
                    type: 'string',
                  },
                  labels: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                  workspace: {
                    $ref: '#/$defs/AgentWorkspace',
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
                  hasUnread: {
                    type: 'boolean',
                  },
                  activity: {
                    $ref: '#/$defs/AgentSessionActivity',
                  },
                },
                required: [
                  'dataSource',
                  'permissions',
                  'boundaryRevision',
                  'id',
                  'title',
                  'labels',
                  'workspace',
                  'revision',
                  'updatedAt',
                  'archived',
                  'hasUnread',
                  'activity',
                ],
              },
              AgentDataSource: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  stateDomain: {
                    type: 'string',
                  },
                  deployment: {
                    type: 'string',
                    format: 'uuid',
                  },
                  namespace: {
                    $ref: '#/$defs/InstanceId',
                  },
                },
                required: ['stateDomain', 'deployment', 'namespace'],
                description:
                  'Stable business-data origin, independent of package or activation revisions.',
              },
              InstanceId: {
                type: 'array',
                items: {
                  type: 'string',
                  format: 'uuid',
                },
                description:
                  'Hierarchical, stable source keys. Display aliases never become data identities.',
              },
              AgentWorkspacePermissions: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  access: {
                    $ref: '#/$defs/AgentWorkspaceAccess',
                  },
                  requestPermissions: {
                    type: 'boolean',
                  },
                },
                required: ['access', 'requestPermissions'],
              },
              AgentWorkspaceAccess: {
                type: 'string',
                enum: ['read_only', 'workspace_write'],
                description:
                  'Access inside the original selected directory; never an ambient filesystem grant.',
              },
              AgentWorkspace: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                required: ['path', 'directoryIdentity', 'identity'],
              },
              AgentSessionActivity: {
                type: 'string',
                enum: ['idle', 'working', 'waiting'],
              },
              AgentRejection: {
                type: 'string',
                enum: [
                  'title_invalid',
                  'labels_invalid',
                  'workspace_unavailable',
                  'session_unavailable',
                  'closed',
                ],
              },
            },
          },
          resources: {},
        },
        resolve: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              workspace: {
                $ref: '#/$defs/AgentDirectorySelection',
                description: 'Used only when freezing the first coordinator creation intent.',
              },
            },
            required: ['workspace'],
            title: 'WorkHubResolve',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              AgentDirectorySelection: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  rootId: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 64,
                  },
                  segments: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                    maxItems: 64,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                },
                required: ['rootId', 'segments', 'directoryIdentity'],
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'missing',
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
                    const: 'pending',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  session: {
                    $ref: '#/$defs/AgentSession',
                  },
                  kind: {
                    type: 'string',
                    const: 'ready',
                  },
                },
                required: ['kind', 'session'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reason: {
                    $ref: '#/$defs/AgentRejection',
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                required: ['kind', 'reason'],
              },
            ],
            title: 'WorkHubCoordinator',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              AgentSession: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  dataSource: {
                    $ref: '#/$defs/AgentDataSource',
                  },
                  permissions: {
                    $ref: '#/$defs/AgentWorkspacePermissions',
                  },
                  boundaryRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  id: {
                    type: 'string',
                  },
                  title: {
                    type: 'string',
                  },
                  labels: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                  workspace: {
                    $ref: '#/$defs/AgentWorkspace',
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
                  hasUnread: {
                    type: 'boolean',
                  },
                  activity: {
                    $ref: '#/$defs/AgentSessionActivity',
                  },
                },
                required: [
                  'dataSource',
                  'permissions',
                  'boundaryRevision',
                  'id',
                  'title',
                  'labels',
                  'workspace',
                  'revision',
                  'updatedAt',
                  'archived',
                  'hasUnread',
                  'activity',
                ],
              },
              AgentDataSource: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  stateDomain: {
                    type: 'string',
                  },
                  deployment: {
                    type: 'string',
                    format: 'uuid',
                  },
                  namespace: {
                    $ref: '#/$defs/InstanceId',
                  },
                },
                required: ['stateDomain', 'deployment', 'namespace'],
                description:
                  'Stable business-data origin, independent of package or activation revisions.',
              },
              InstanceId: {
                type: 'array',
                items: {
                  type: 'string',
                  format: 'uuid',
                },
                description:
                  'Hierarchical, stable source keys. Display aliases never become data identities.',
              },
              AgentWorkspacePermissions: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  access: {
                    $ref: '#/$defs/AgentWorkspaceAccess',
                  },
                  requestPermissions: {
                    type: 'boolean',
                  },
                },
                required: ['access', 'requestPermissions'],
              },
              AgentWorkspaceAccess: {
                type: 'string',
                enum: ['read_only', 'workspace_write'],
                description:
                  'Access inside the original selected directory; never an ambient filesystem grant.',
              },
              AgentWorkspace: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                required: ['path', 'directoryIdentity', 'identity'],
              },
              AgentSessionActivity: {
                type: 'string',
                enum: ['idle', 'working', 'waiting'],
              },
              AgentRejection: {
                type: 'string',
                enum: [
                  'title_invalid',
                  'labels_invalid',
                  'workspace_unavailable',
                  'session_unavailable',
                  'closed',
                ],
              },
            },
          },
          resources: {},
        },
        task: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              taskId: {
                type: 'string',
                pattern: '^wt_[a-f0-9]{64}$',
              },
              cursor: {
                anyOf: [
                  {
                    $ref: '#/$defs/AgentAnswerCursor',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
            },
            required: ['taskId'],
            title: 'WorkHubTaskRead',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              AgentAnswerCursor: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  invocationId: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 128,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                    maximum: 9007199254740991,
                  },
                },
                required: ['invocationId', 'offset'],
              },
            },
          },
          output: {
            type: 'object',
            additionalProperties: false,
            properties: {
              taskId: {
                type: 'string',
              },
              worker: {
                type: 'string',
              },
              operation: {
                type: 'string',
                format: 'uuid',
              },
              status: {
                $ref: '#/$defs/WorkHubTaskStatus',
              },
            },
            required: ['taskId', 'worker', 'operation', 'status'],
            title: 'WorkHubTask',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              WorkHubTaskStatus: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'pending',
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
                        const: 'uncertain',
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
                        const: 'unavailable',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      reason: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'rejected',
                      },
                    },
                    required: ['kind', 'reason'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sessionId: {
                        type: 'string',
                      },
                      invocationId: {
                        type: 'string',
                      },
                      progress: {
                        anyOf: [
                          {
                            $ref: '#/$defs/AgentWork',
                          },
                          {
                            type: 'null',
                          },
                        ],
                      },
                      kind: {
                        type: 'string',
                        const: 'started',
                      },
                    },
                    required: ['kind', 'sessionId', 'invocationId'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sessionId: {
                        type: 'string',
                      },
                      messageId: {
                        type: 'string',
                      },
                      progress: {
                        anyOf: [
                          {
                            $ref: '#/$defs/AgentWork',
                          },
                          {
                            type: 'null',
                          },
                        ],
                      },
                      kind: {
                        type: 'string',
                        const: 'queued',
                      },
                    },
                    required: ['kind', 'sessionId', 'messageId'],
                  },
                ],
              },
              AgentWork: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'missing',
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
                        const: 'pending',
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
                        const: 'cancelled',
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
                        const: 'changed',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      shared: {
                        type: 'boolean',
                      },
                      execution: {
                        $ref: '#/$defs/AgentExecution',
                      },
                      answer: {
                        anyOf: [
                          {
                            $ref: '#/$defs/AgentAnswerPage',
                          },
                          {
                            type: 'null',
                          },
                        ],
                      },
                      pendingInteractions: {
                        type: 'integer',
                        format: 'uint32',
                        minimum: 0,
                        description:
                          'Count for this exact Run; original Session interactions retain all authority.',
                      },
                      kind: {
                        type: 'string',
                        const: 'delivered',
                      },
                    },
                    required: ['kind', 'shared', 'execution', 'pendingInteractions'],
                  },
                ],
              },
              AgentExecution: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  turnId: {
                    type: 'string',
                  },
                  runId: {
                    type: 'string',
                  },
                  invocationId: {
                    type: 'string',
                  },
                  highWater: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                    description: 'Run-local event count, not the root-global transcript cursor.',
                  },
                  state: {
                    $ref: '#/$defs/AgentExecutionState',
                  },
                },
                required: ['sessionId', 'turnId', 'runId', 'invocationId', 'highWater', 'state'],
                description:
                  'Canonical physical execution receipt, independent of the current Session tip.',
              },
              AgentExecutionState: {
                type: 'string',
                enum: [
                  'admitted',
                  'running',
                  'waiting',
                  'paused',
                  'completed',
                  'failed',
                  'cancelled',
                ],
              },
              AgentAnswerPage: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  text: {
                    type: 'string',
                  },
                  next: {
                    anyOf: [
                      {
                        $ref: '#/$defs/AgentAnswerCursor',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                },
                required: ['text'],
              },
              AgentAnswerCursor: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  invocationId: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 128,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                    maximum: 9007199254740991,
                  },
                },
                required: ['invocationId', 'offset'],
              },
            },
          },
          resources: {},
        },
        tasks: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              after: {
                type: ['string', 'null'],
              },
            },
            title: 'WorkHubTasksRead',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            type: 'object',
            additionalProperties: false,
            properties: {
              entries: {
                type: 'array',
                items: {
                  $ref: '#/$defs/WorkHubTaskEntry',
                },
                maxItems: 16,
              },
              after: {
                type: ['string', 'null'],
              },
            },
            required: ['entries'],
            title: 'WorkHubTasks',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              WorkHubTaskEntry: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  taskId: {
                    type: 'string',
                  },
                  worker: {
                    type: 'string',
                  },
                },
                required: ['taskId', 'worker'],
              },
            },
          },
          resources: {},
        },
      },
    },
  },
  questions: {
    name: 'maka.interactions.Questions',
    version: 1,
    transport: 'remote',
    interface: {
      kind: 'service',
      methods: {
        ask: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              questions: {
                type: 'array',
                items: {
                  $ref: '#/$defs/InteractionQuestion',
                },
                minItems: 1,
                maxItems: 3,
              },
            },
            required: ['questions'],
            title: 'QuestionAsk',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              InteractionQuestion: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  question: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 1024,
                  },
                  options: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/QuestionOption',
                    },
                    minItems: 2,
                    maxItems: 3,
                  },
                },
                required: ['question', 'options'],
              },
              QuestionOption: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  label: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 256,
                  },
                  description: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 512,
                  },
                },
                required: ['label'],
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  answers: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/QuestionResponse',
                    },
                  },
                  kind: {
                    type: 'string',
                    const: 'answered',
                  },
                },
                required: ['kind', 'answers'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reason: {
                    $ref: '#/$defs/ClosureReason',
                  },
                  kind: {
                    type: 'string',
                    const: 'closed',
                  },
                },
                required: ['kind', 'reason'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reason: {
                    $ref: '#/$defs/QuestionRejection',
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                required: ['kind', 'reason'],
              },
            ],
            title: 'QuestionResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              QuestionResponse: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  question: {
                    type: 'string',
                  },
                  answer: {
                    type: ['string', 'null'],
                  },
                },
                required: ['question', 'answer'],
              },
              ClosureReason: {
                type: 'string',
                enum: [
                  'turn_stopped',
                  'turn_terminal',
                  'producer_cancelled',
                  'timed_out',
                  'host_restarted',
                  'provider_disconnected',
                ],
              },
              QuestionRejection: {
                type: 'string',
                enum: ['invalid_input', 'unavailable'],
              },
            },
          },
          resources: {},
        },
      },
    },
  },
  forms: {
    name: 'maka.interactions.Forms',
    version: 1,
    transport: 'remote',
    interface: {
      kind: 'service',
      methods: {
        ask: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              message: {
                type: 'string',
                minLength: 1,
                maxLength: 2048,
              },
              fields: {
                type: 'array',
                items: {
                  $ref: '#/$defs/FormField',
                },
                maxItems: 32,
              },
            },
            required: ['message', 'fields'],
            title: 'FormAsk',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              FormField: {
                type: 'object',
                properties: {
                  name: {
                    type: 'string',
                  },
                  label: {
                    type: 'string',
                  },
                  required: {
                    type: 'boolean',
                  },
                  description: {
                    type: ['string', 'null'],
                  },
                },
                required: ['name', 'label', 'required'],
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      default: {
                        type: ['string', 'null'],
                      },
                      minLength: {
                        type: ['integer', 'null'],
                        format: 'uint',
                        minimum: 0,
                      },
                      maxLength: {
                        type: ['integer', 'null'],
                        format: 'uint',
                        minimum: 0,
                      },
                      format: {
                        anyOf: [
                          {
                            $ref: '#/$defs/FormFormat',
                          },
                          {
                            type: 'null',
                          },
                        ],
                      },
                      kind: {
                        type: 'string',
                        const: 'string',
                      },
                    },
                    required: ['kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      default: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      minimum: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      maximum: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      kind: {
                        type: 'string',
                        const: 'number',
                      },
                    },
                    required: ['kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      default: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      minimum: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      maximum: {
                        type: ['number', 'null'],
                        format: 'double',
                      },
                      kind: {
                        type: 'string',
                        const: 'integer',
                      },
                    },
                    required: ['kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      default: {
                        type: ['boolean', 'null'],
                      },
                      kind: {
                        type: 'string',
                        const: 'boolean',
                      },
                    },
                    required: ['kind'],
                  },
                  {
                    type: 'object',
                    properties: {
                      options: {
                        type: 'array',
                        items: {
                          $ref: '#/$defs/FormOption',
                        },
                      },
                      default: {
                        type: ['string', 'null'],
                      },
                      kind: {
                        type: 'string',
                        const: 'single_select',
                      },
                    },
                    required: ['kind', 'options'],
                  },
                  {
                    type: 'object',
                    properties: {
                      options: {
                        type: 'array',
                        items: {
                          $ref: '#/$defs/FormOption',
                        },
                      },
                      default: {
                        type: ['array', 'null'],
                        items: {
                          type: 'string',
                        },
                      },
                      minItems: {
                        type: ['integer', 'null'],
                        format: 'uint',
                        minimum: 0,
                      },
                      maxItems: {
                        type: ['integer', 'null'],
                        format: 'uint',
                        minimum: 0,
                      },
                      kind: {
                        type: 'string',
                        const: 'multi_select',
                      },
                    },
                    required: ['kind', 'options'],
                  },
                ],
              },
              FormFormat: {
                type: 'string',
                enum: ['email', 'uri', 'date', 'date-time'],
              },
              FormOption: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: {
                    type: 'string',
                  },
                  label: {
                    type: 'string',
                  },
                },
                required: ['value', 'label'],
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  result: {
                    $ref: '#/$defs/FormResult',
                  },
                  kind: {
                    type: 'string',
                    const: 'answered',
                  },
                },
                required: ['kind', 'result'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reason: {
                    $ref: '#/$defs/ClosureReason',
                  },
                  kind: {
                    type: 'string',
                    const: 'closed',
                  },
                },
                required: ['kind', 'reason'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reason: {
                    $ref: '#/$defs/QuestionRejection',
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                required: ['kind', 'reason'],
              },
            ],
            title: 'FormResponse',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              FormResult: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      values: {
                        type: 'object',
                        additionalProperties: {
                          $ref: '#/$defs/FormValue',
                        },
                      },
                      action: {
                        type: 'string',
                        const: 'accept',
                      },
                    },
                    required: ['action', 'values'],
                  },
                  {
                    type: 'object',
                    properties: {
                      action: {
                        type: 'string',
                        const: 'decline',
                      },
                    },
                    required: ['action'],
                  },
                  {
                    type: 'object',
                    properties: {
                      action: {
                        type: 'string',
                        const: 'cancel',
                      },
                    },
                    required: ['action'],
                  },
                ],
              },
              FormValue: {
                anyOf: [
                  {
                    type: 'string',
                  },
                  {
                    type: 'number',
                    format: 'double',
                  },
                  {
                    type: 'boolean',
                  },
                  {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                ],
              },
              ClosureReason: {
                type: 'string',
                enum: [
                  'turn_stopped',
                  'turn_terminal',
                  'producer_cancelled',
                  'timed_out',
                  'host_restarted',
                  'provider_disconnected',
                ],
              },
              QuestionRejection: {
                type: 'string',
                enum: ['invalid_input', 'unavailable'],
              },
            },
          },
          resources: {},
        },
      },
    },
  },
  skillLibrary: {
    name: 'maka.skills.Library',
    version: 1,
    transport: 'remote',
    interface: {
      kind: 'service',
      methods: {
        apply: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              change: {
                $ref: '#/$defs/SkillLibraryChange',
              },
              digest: {
                type: 'string',
                minLength: 71,
                maxLength: 71,
              },
            },
            required: ['change', 'digest'],
            title: 'SkillLibraryReview',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              SkillLibraryChange: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  workspace: {
                    anyOf: [
                      {
                        $ref: '#/$defs/AgentWorkspace',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  kind: {
                    $ref: '#/$defs/SkillLibraryChangeKind',
                  },
                  reference: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 512,
                  },
                  revision: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 128,
                  },
                },
                required: ['kind', 'reference', 'revision'],
              },
              AgentWorkspace: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                required: ['path', 'directoryIdentity', 'identity'],
              },
              SkillLibraryChangeKind: {
                type: 'string',
                enum: ['install', 'update', 'delete'],
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reference: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'applied',
                  },
                },
                required: ['kind', 'reference'],
              },
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'changed',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reason: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                required: ['kind', 'reason'],
              },
            ],
            title: 'SkillLibraryChangeResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          resources: {},
        },
        deleteSource: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              change: {
                $ref: '#/$defs/SkillLibraryChange',
              },
              digest: {
                type: 'string',
                minLength: 71,
                maxLength: 71,
              },
            },
            required: ['change', 'digest'],
            title: 'SkillLibraryReview',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              SkillLibraryChange: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  workspace: {
                    anyOf: [
                      {
                        $ref: '#/$defs/AgentWorkspace',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  kind: {
                    $ref: '#/$defs/SkillLibraryChangeKind',
                  },
                  reference: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 512,
                  },
                  revision: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 128,
                  },
                },
                required: ['kind', 'reference', 'revision'],
              },
              AgentWorkspace: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                required: ['path', 'directoryIdentity', 'identity'],
              },
              SkillLibraryChangeKind: {
                type: 'string',
                enum: ['install', 'update', 'delete'],
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reference: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'applied',
                  },
                },
                required: ['kind', 'reference'],
              },
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'changed',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reason: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                required: ['kind', 'reason'],
              },
            ],
            title: 'SkillLibraryChangeResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          resources: {},
        },
        importSource: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              bytes: {
                type: 'integer',
                format: 'uint',
                minimum: 1,
                maximum: 1048576,
              },
              digest: {
                type: 'string',
                minLength: 71,
                maxLength: 71,
              },
            },
            required: ['bytes', 'digest'],
            title: 'SkillLibraryImportSource',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reference: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'applied',
                  },
                },
                required: ['kind', 'reference'],
              },
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'changed',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reason: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                required: ['kind', 'reason'],
              },
            ],
            title: 'SkillLibraryChangeResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          resources: {
            file: {
              name: 'maka.bytes',
              version: 1,
            },
            id: {
              name: 'maka.text.utf8',
              version: 1,
            },
          },
        },
        list: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              workspace: {
                anyOf: [
                  {
                    $ref: '#/$defs/AgentWorkspace',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
              query: {
                type: 'string',
                maxLength: 256,
              },
              cursor: {
                anyOf: [
                  {
                    $ref: '#/$defs/SkillLibraryCursor',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
            },
            required: ['query'],
            title: 'SkillLibraryList',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              AgentWorkspace: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                required: ['path', 'directoryIdentity', 'identity'],
              },
              SkillLibraryCursor: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  revision: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 128,
                  },
                  query: {
                    type: 'string',
                    maxLength: 256,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                    maximum: 16384,
                  },
                },
                required: ['revision', 'query', 'offset'],
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  writable: {
                    type: 'boolean',
                  },
                  preferencesWritable: {
                    type: 'boolean',
                  },
                  preferenceRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  revision: {
                    type: 'string',
                  },
                  entries: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/SkillLibraryEntry',
                    },
                  },
                  after: {
                    anyOf: [
                      {
                        $ref: '#/$defs/SkillLibraryCursor',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  diagnostics: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                  kind: {
                    type: 'string',
                    const: 'page',
                  },
                },
                required: [
                  'kind',
                  'writable',
                  'preferencesWritable',
                  'preferenceRevision',
                  'revision',
                  'entries',
                  'diagnostics',
                ],
              },
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'changed',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
            ],
            title: 'SkillLibraryPage',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              SkillLibraryEntry: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  bundled: {
                    type: 'boolean',
                  },
                  installed: {
                    type: 'boolean',
                  },
                  imported: {
                    type: 'boolean',
                    description:
                      'Imported authoring source; unavailable to the model until explicitly installed.',
                  },
                  preference: {
                    $ref: '#/$defs/SkillPreference',
                  },
                  reference: {
                    type: 'string',
                  },
                  source: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                  description: {
                    type: 'string',
                  },
                  valid: {
                    type: 'boolean',
                  },
                  shadowedBy: {
                    type: ['string', 'null'],
                  },
                  metadataTruncated: {
                    type: 'boolean',
                  },
                  issues: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                },
                required: [
                  'bundled',
                  'installed',
                  'imported',
                  'preference',
                  'reference',
                  'source',
                  'name',
                  'description',
                  'valid',
                  'metadataTruncated',
                  'issues',
                ],
              },
              SkillPreference: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  enabled: {
                    type: 'boolean',
                  },
                  pinned: {
                    type: 'boolean',
                  },
                },
                required: ['enabled', 'pinned'],
              },
              SkillLibraryCursor: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  revision: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 128,
                  },
                  query: {
                    type: 'string',
                    maxLength: 256,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                    maximum: 16384,
                  },
                },
                required: ['revision', 'query', 'offset'],
              },
            },
          },
          resources: {},
        },
        load: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              workspace: {
                anyOf: [
                  {
                    $ref: '#/$defs/AgentWorkspace',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
              tools: {
                type: 'array',
                items: {
                  type: 'string',
                },
                maxItems: 128,
              },
              input: {
                $ref: '#/$defs/SkillModelLoad',
              },
            },
            required: ['tools', 'input'],
            description:
              'Actual Agent registrations are supplied outside model-controlled arguments.\nThese names describe compatibility and confer no execution authority.',
            title: 'SkillToolRequest',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              AgentWorkspace: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                required: ['path', 'directoryIdentity', 'identity'],
              },
              SkillModelLoad: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  name: {
                    type: 'string',
                    description:
                      'Exact ref from search, or a skill id/name resolved before eligibility checks.',
                    minLength: 1,
                    maxLength: 512,
                  },
                  revision: {
                    type: 'string',
                    description:
                      'Catalog revision from search or a previous load/resource response.',
                    minLength: 1,
                    maxLength: 128,
                  },
                  resource: {
                    anyOf: [
                      {
                        $ref: '#/$defs/ReadInput',
                      },
                      {
                        type: 'null',
                      },
                    ],
                    description:
                      'UTF-8 resource path relative to this skill, or its exact continuation.',
                  },
                  instructionOffset: {
                    type: ['integer', 'null'],
                    format: 'uint',
                    minimum: 0,
                    description:
                      'UTF-8 byte offset from a returned instruction continuation; omit initially.',
                    maximum: 96000,
                  },
                },
                required: ['name', 'revision'],
              },
              ReadInput: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                    maximum: 9007199254740991,
                  },
                  limit: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 1,
                    maximum: 9007199254740991,
                  },
                },
                required: ['path'],
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                properties: {
                  revision: {
                    type: 'string',
                  },
                  metadata: {
                    $ref: '#/$defs/SkillMetadata',
                  },
                  instructions: {
                    type: 'string',
                  },
                  instructionOffset: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                  },
                  next: {
                    anyOf: [
                      {
                        $ref: '#/$defs/SkillModelLoad',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  relativePath: {
                    type: 'string',
                  },
                  declaredTools: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                  declaredToolsTruncated: {
                    type: 'boolean',
                  },
                  truncated: {
                    type: 'boolean',
                  },
                  receipt: {
                    $ref: '#/$defs/SkillInvocationReceipt',
                  },
                  kind: {
                    type: 'string',
                    const: 'loaded',
                  },
                },
                required: [
                  'kind',
                  'revision',
                  'metadata',
                  'instructions',
                  'instructionOffset',
                  'relativePath',
                  'declaredTools',
                  'declaredToolsTruncated',
                  'truncated',
                  'receipt',
                ],
                description:
                  'User-provided, lower-priority instructions. Declared tools are hints, not grants.',
              },
              {
                type: 'object',
                properties: {
                  revision: {
                    type: 'string',
                  },
                  reference: {
                    type: 'string',
                  },
                  path: {
                    type: 'string',
                  },
                  content: {
                    type: 'string',
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                  },
                  returnedLines: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                  },
                  totalLines: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                  },
                  partialLine: {
                    type: 'boolean',
                  },
                  next: {
                    anyOf: [
                      {
                        $ref: '#/$defs/SkillModelLoad',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  receipt: {
                    $ref: '#/$defs/SkillInvocationReceipt',
                  },
                  kind: {
                    type: 'string',
                    const: 'resource',
                  },
                },
                required: [
                  'kind',
                  'revision',
                  'reference',
                  'path',
                  'content',
                  'offset',
                  'returnedLines',
                  'totalLines',
                  'partialLine',
                  'receipt',
                ],
              },
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'changed',
                  },
                },
                required: ['kind'],
              },
              {
                type: 'object',
                properties: {
                  reason: {
                    $ref: '#/$defs/SkillFailureReason',
                  },
                  receipt: {
                    $ref: '#/$defs/SkillInvocationReceipt',
                  },
                  kind: {
                    type: 'string',
                    const: 'unavailable',
                  },
                },
                required: ['kind', 'reason', 'receipt'],
              },
              {
                type: 'object',
                properties: {
                  reason: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'resource_unavailable',
                  },
                },
                required: ['kind', 'reason'],
              },
            ],
            title: 'SkillModelLoadResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              SkillMetadata: {
                type: 'object',
                properties: {
                  ref: {
                    type: 'string',
                  },
                  id: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                  description: {
                    type: 'string',
                  },
                  scope: {
                    $ref: '#/$defs/SkillScope',
                  },
                  source: {
                    $ref: '#/$defs/SkillSource',
                  },
                  metadataTruncated: {
                    type: 'boolean',
                  },
                },
                required: [
                  'ref',
                  'id',
                  'name',
                  'description',
                  'scope',
                  'source',
                  'metadataTruncated',
                ],
              },
              SkillScope: {
                type: 'string',
                enum: ['project', 'workspace', 'user', 'custom'],
              },
              SkillSource: {
                type: 'string',
                enum: ['maka', 'agents', 'legacy', 'custom'],
              },
              SkillModelLoad: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  name: {
                    type: 'string',
                    description:
                      'Exact ref from search, or a skill id/name resolved before eligibility checks.',
                    minLength: 1,
                    maxLength: 512,
                  },
                  revision: {
                    type: 'string',
                    description:
                      'Catalog revision from search or a previous load/resource response.',
                    minLength: 1,
                    maxLength: 128,
                  },
                  resource: {
                    anyOf: [
                      {
                        $ref: '#/$defs/ReadInput',
                      },
                      {
                        type: 'null',
                      },
                    ],
                    description:
                      'UTF-8 resource path relative to this skill, or its exact continuation.',
                  },
                  instructionOffset: {
                    type: ['integer', 'null'],
                    format: 'uint',
                    minimum: 0,
                    description:
                      'UTF-8 byte offset from a returned instruction continuation; omit initially.',
                    maximum: 96000,
                  },
                },
                required: ['name', 'revision'],
              },
              ReadInput: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                    maximum: 9007199254740991,
                  },
                  limit: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 1,
                    maximum: 9007199254740991,
                  },
                },
                required: ['path'],
              },
              SkillInvocationReceipt: {
                anyOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      invocation: {
                        $ref: '#/$defs/SkillInvocationMode',
                      },
                      request: {
                        type: 'string',
                      },
                      success: {
                        const: true,
                      },
                      ref: {
                        type: 'string',
                      },
                      id: {
                        type: 'string',
                      },
                      name: {
                        type: 'string',
                      },
                      scope: {
                        $ref: '#/$defs/SkillScope',
                      },
                      source: {
                        $ref: '#/$defs/SkillSource',
                      },
                      truncated: {
                        type: 'boolean',
                      },
                    },
                    required: [
                      'invocation',
                      'request',
                      'success',
                      'ref',
                      'id',
                      'name',
                      'scope',
                      'source',
                      'truncated',
                    ],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      invocation: {
                        $ref: '#/$defs/SkillInvocationMode',
                      },
                      request: {
                        type: 'string',
                      },
                      success: {
                        const: false,
                      },
                      reason: {
                        $ref: '#/$defs/SkillFailureReason',
                      },
                    },
                    required: ['invocation', 'request', 'success', 'reason'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      invocation: {
                        const: 'explicit',
                      },
                      success: {
                        const: false,
                      },
                      reason: {
                        $ref: '#/$defs/TooManyRequests',
                      },
                      requestLimit: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                    },
                    required: ['invocation', 'success', 'reason', 'requestLimit'],
                  },
                ],
                description:
                  'Boolean wire discriminants are validated here, not retained as mutable\nbusiness fields beside a contradictory enum variant.',
              },
              SkillInvocationMode: {
                type: 'string',
                enum: ['explicit', 'model_tool'],
              },
              SkillFailureReason: {
                type: 'string',
                enum: [
                  'invalid_name',
                  'not_found',
                  'disabled',
                  'host_incompatible',
                  'resolution_failed',
                ],
              },
              TooManyRequests: {
                type: 'string',
                enum: ['too_many_requests'],
              },
            },
          },
          resources: {},
        },
        prepare: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              workspace: {
                anyOf: [
                  {
                    $ref: '#/$defs/AgentWorkspace',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
              tools: {
                type: 'array',
                items: {
                  type: 'string',
                },
                maxItems: 128,
              },
              input: {
                $ref: '#/$defs/SkillPrepare',
              },
            },
            required: ['tools', 'input'],
            description:
              'Actual Agent registrations are supplied outside model-controlled arguments.\nThese names describe compatibility and confer no execution authority.',
            title: 'SkillToolRequest',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              AgentWorkspace: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                required: ['path', 'directoryIdentity', 'identity'],
              },
              SkillPrepare: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                    maxLength: 65536,
                  },
                  loadTool: {
                    type: ['string', 'null'],
                    description:
                      'Actual selected tools bound to this Library; these names grant no access.',
                    minLength: 1,
                    maxLength: 128,
                  },
                  searchTool: {
                    type: ['string', 'null'],
                    minLength: 1,
                    maxLength: 128,
                  },
                },
                required: ['message'],
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  text: {
                    type: ['string', 'null'],
                    description: 'None preserves the original message exactly.',
                  },
                  record: {
                    $ref: '#/$defs/SkillPreparationRecord',
                  },
                  requiredTools: {
                    type: 'array',
                    uniqueItems: true,
                    items: {
                      type: 'string',
                    },
                  },
                  kind: {
                    type: 'string',
                    const: 'ready',
                  },
                },
                required: ['kind', 'record', 'requiredTools'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  invocation: {
                    $ref: '#/$defs/SkillInvocationResult',
                  },
                  kind: {
                    type: 'string',
                    const: 'blocked',
                  },
                },
                required: ['kind', 'invocation'],
              },
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'input_too_large',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
            ],
            title: 'SkillPreparation',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              SkillPreparationRecord: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  revision: {
                    type: 'string',
                  },
                  metadataPrompt: {
                    type: 'string',
                  },
                  invocation: {
                    $ref: '#/$defs/SkillInvocationResult',
                  },
                },
                required: ['revision', 'metadataPrompt', 'invocation'],
              },
              SkillInvocationResult: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  loaded: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/LoadedSkill',
                    },
                  },
                  failed: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/SkillInvocationFailure',
                    },
                  },
                  receipts: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/SkillInvocationReceipt',
                    },
                  },
                },
                required: ['loaded', 'failed', 'receipts'],
              },
              LoadedSkill: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  id: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                },
                required: ['id', 'name'],
              },
              SkillInvocationFailure: {
                anyOf: [
                  {
                    $ref: '#/$defs/SkillRequestFailure',
                  },
                  {
                    $ref: '#/$defs/SkillOverflowFailure',
                  },
                ],
              },
              SkillRequestFailure: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  request: {
                    type: 'string',
                  },
                  reason: {
                    $ref: '#/$defs/SkillFailureReason',
                  },
                },
                required: ['request', 'reason'],
              },
              SkillFailureReason: {
                type: 'string',
                enum: [
                  'invalid_name',
                  'not_found',
                  'disabled',
                  'host_incompatible',
                  'resolution_failed',
                ],
              },
              SkillOverflowFailure: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reason: {
                    $ref: '#/$defs/TooManyRequests',
                  },
                  requestLimit: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                required: ['reason', 'requestLimit'],
              },
              TooManyRequests: {
                type: 'string',
                enum: ['too_many_requests'],
              },
              SkillInvocationReceipt: {
                anyOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      invocation: {
                        $ref: '#/$defs/SkillInvocationMode',
                      },
                      request: {
                        type: 'string',
                      },
                      success: {
                        const: true,
                      },
                      ref: {
                        type: 'string',
                      },
                      id: {
                        type: 'string',
                      },
                      name: {
                        type: 'string',
                      },
                      scope: {
                        $ref: '#/$defs/SkillScope',
                      },
                      source: {
                        $ref: '#/$defs/SkillSource',
                      },
                      truncated: {
                        type: 'boolean',
                      },
                    },
                    required: [
                      'invocation',
                      'request',
                      'success',
                      'ref',
                      'id',
                      'name',
                      'scope',
                      'source',
                      'truncated',
                    ],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      invocation: {
                        $ref: '#/$defs/SkillInvocationMode',
                      },
                      request: {
                        type: 'string',
                      },
                      success: {
                        const: false,
                      },
                      reason: {
                        $ref: '#/$defs/SkillFailureReason',
                      },
                    },
                    required: ['invocation', 'request', 'success', 'reason'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      invocation: {
                        const: 'explicit',
                      },
                      success: {
                        const: false,
                      },
                      reason: {
                        $ref: '#/$defs/TooManyRequests',
                      },
                      requestLimit: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                      },
                    },
                    required: ['invocation', 'success', 'reason', 'requestLimit'],
                  },
                ],
                description:
                  'Boolean wire discriminants are validated here, not retained as mutable\nbusiness fields beside a contradictory enum variant.',
              },
              SkillInvocationMode: {
                type: 'string',
                enum: ['explicit', 'model_tool'],
              },
              SkillScope: {
                type: 'string',
                enum: ['project', 'workspace', 'user', 'custom'],
              },
              SkillSource: {
                type: 'string',
                enum: ['maka', 'agents', 'legacy', 'custom'],
              },
            },
          },
          resources: {},
        },
        preview: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              workspace: {
                anyOf: [
                  {
                    $ref: '#/$defs/AgentWorkspace',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
              kind: {
                $ref: '#/$defs/SkillLibraryChangeKind',
              },
              reference: {
                type: 'string',
                minLength: 1,
                maxLength: 512,
              },
              revision: {
                type: 'string',
                minLength: 1,
                maxLength: 128,
              },
            },
            required: ['kind', 'reference', 'revision'],
            title: 'SkillLibraryChange',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              AgentWorkspace: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                required: ['path', 'directoryIdentity', 'identity'],
              },
              SkillLibraryChangeKind: {
                type: 'string',
                enum: ['install', 'update', 'delete'],
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  review: {
                    $ref: '#/$defs/SkillLibraryReview',
                  },
                  target: {
                    type: 'string',
                  },
                  entries: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                  },
                  paths: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                  pathsTruncated: {
                    type: 'boolean',
                  },
                  locallyModified: {
                    type: 'boolean',
                  },
                  kind: {
                    type: 'string',
                    const: 'ready',
                  },
                },
                required: [
                  'kind',
                  'review',
                  'target',
                  'entries',
                  'paths',
                  'pathsTruncated',
                  'locallyModified',
                ],
              },
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'changed',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reason: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                required: ['kind', 'reason'],
              },
            ],
            title: 'SkillLibraryPreview',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              SkillLibraryReview: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  change: {
                    $ref: '#/$defs/SkillLibraryChange',
                  },
                  digest: {
                    type: 'string',
                    minLength: 71,
                    maxLength: 71,
                  },
                },
                required: ['change', 'digest'],
              },
              SkillLibraryChange: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  workspace: {
                    anyOf: [
                      {
                        $ref: '#/$defs/AgentWorkspace',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  kind: {
                    $ref: '#/$defs/SkillLibraryChangeKind',
                  },
                  reference: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 512,
                  },
                  revision: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 128,
                  },
                },
                required: ['kind', 'reference', 'revision'],
              },
              AgentWorkspace: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                required: ['path', 'directoryIdentity', 'identity'],
              },
              SkillLibraryChangeKind: {
                type: 'string',
                enum: ['install', 'update', 'delete'],
              },
            },
          },
          resources: {},
        },
        read: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              workspace: {
                anyOf: [
                  {
                    $ref: '#/$defs/AgentWorkspace',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
              reference: {
                type: 'string',
                minLength: 1,
                maxLength: 512,
              },
              revision: {
                type: 'string',
                description: 'Complete catalog revision from the list/read response.',
                minLength: 1,
                maxLength: 128,
              },
              offset: {
                type: 'integer',
                format: 'uint',
                minimum: 0,
                maximum: 16777216,
              },
            },
            required: ['reference', 'revision', 'offset'],
            title: 'SkillLibraryRead',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              AgentWorkspace: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                required: ['path', 'directoryIdentity', 'identity'],
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  writable: {
                    type: 'boolean',
                  },
                  preferencesWritable: {
                    type: 'boolean',
                  },
                  preferenceRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  entry: {
                    $ref: '#/$defs/SkillLibraryEntry',
                  },
                  revision: {
                    type: 'string',
                  },
                  text: {
                    type: 'string',
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                  },
                  before: {
                    type: ['integer', 'null'],
                    format: 'uint',
                    minimum: 0,
                  },
                  after: {
                    type: ['integer', 'null'],
                    format: 'uint',
                    minimum: 0,
                  },
                  totalBytes: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                  },
                  kind: {
                    type: 'string',
                    const: 'page',
                  },
                },
                required: [
                  'kind',
                  'writable',
                  'preferencesWritable',
                  'preferenceRevision',
                  'entry',
                  'revision',
                  'text',
                  'offset',
                  'totalBytes',
                ],
              },
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'changed',
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
                    const: 'unavailable',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
            ],
            title: 'SkillLibraryDocument',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              SkillLibraryEntry: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  bundled: {
                    type: 'boolean',
                  },
                  installed: {
                    type: 'boolean',
                  },
                  imported: {
                    type: 'boolean',
                    description:
                      'Imported authoring source; unavailable to the model until explicitly installed.',
                  },
                  preference: {
                    $ref: '#/$defs/SkillPreference',
                  },
                  reference: {
                    type: 'string',
                  },
                  source: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                  description: {
                    type: 'string',
                  },
                  valid: {
                    type: 'boolean',
                  },
                  shadowedBy: {
                    type: ['string', 'null'],
                  },
                  metadataTruncated: {
                    type: 'boolean',
                  },
                  issues: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                },
                required: [
                  'bundled',
                  'installed',
                  'imported',
                  'preference',
                  'reference',
                  'source',
                  'name',
                  'description',
                  'valid',
                  'metadataTruncated',
                  'issues',
                ],
              },
              SkillPreference: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  enabled: {
                    type: 'boolean',
                  },
                  pinned: {
                    type: 'boolean',
                  },
                },
                required: ['enabled', 'pinned'],
              },
            },
          },
          resources: {},
        },
        search: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              workspace: {
                anyOf: [
                  {
                    $ref: '#/$defs/AgentWorkspace',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
              tools: {
                type: 'array',
                items: {
                  type: 'string',
                },
                maxItems: 128,
              },
              input: {
                $ref: '#/$defs/SkillModelSearch',
              },
            },
            required: ['tools', 'input'],
            description:
              'Actual Agent registrations are supplied outside model-controlled arguments.\nThese names describe compatibility and confer no execution authority.',
            title: 'SkillToolRequest',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              AgentWorkspace: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  path: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 4096,
                  },
                  directoryIdentity: {
                    type: 'string',
                    pattern: '^sha256:[a-f0-9]{64}$',
                  },
                  identity: {
                    type: 'string',
                    pattern: '^workspace:v1:[A-Fa-f0-9-]{36}$',
                  },
                },
                required: ['path', 'directoryIdentity', 'identity'],
              },
              SkillModelSearch: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  query: {
                    type: 'string',
                    maxLength: 4096,
                  },
                  limit: {
                    type: ['integer', 'null'],
                    format: 'uint',
                    minimum: 1,
                    maximum: 8,
                  },
                },
                required: ['query'],
              },
            },
          },
          output: {
            type: 'object',
            properties: {
              revision: {
                type: 'string',
              },
              query: {
                type: 'string',
              },
              queryTruncated: {
                type: 'boolean',
              },
              matches: {
                type: 'array',
                items: {
                  $ref: '#/$defs/SearchMatch',
                },
              },
              totalEligible: {
                type: 'integer',
                format: 'uint',
                minimum: 0,
              },
              matchedCount: {
                type: 'integer',
                format: 'uint',
                minimum: 0,
              },
              truncated: {
                type: 'boolean',
              },
            },
            required: [
              'revision',
              'query',
              'queryTruncated',
              'matches',
              'totalEligible',
              'matchedCount',
              'truncated',
            ],
            title: 'SkillModelSearchResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              SearchMatch: {
                type: 'object',
                properties: {
                  ref: {
                    type: 'string',
                  },
                  id: {
                    type: 'string',
                  },
                  name: {
                    type: 'string',
                  },
                  description: {
                    type: 'string',
                  },
                  scope: {
                    $ref: '#/$defs/SkillScope',
                  },
                  source: {
                    $ref: '#/$defs/SkillSource',
                  },
                  metadataTruncated: {
                    type: 'boolean',
                  },
                  score: {
                    type: 'integer',
                    format: 'uint32',
                    minimum: 0,
                  },
                },
                required: [
                  'ref',
                  'id',
                  'name',
                  'description',
                  'scope',
                  'source',
                  'metadataTruncated',
                  'score',
                ],
              },
              SkillScope: {
                type: 'string',
                enum: ['project', 'workspace', 'user', 'custom'],
              },
              SkillSource: {
                type: 'string',
                enum: ['maka', 'agents', 'legacy', 'custom'],
              },
            },
          },
          resources: {},
        },
        setPreference: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              reference: {
                type: 'string',
                minLength: 1,
                maxLength: 512,
              },
              revision: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 9007199254740991,
              },
              preference: {
                $ref: '#/$defs/SkillPreference',
              },
            },
            required: ['reference', 'revision', 'preference'],
            title: 'SkillLibrarySetPreference',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              SkillPreference: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  enabled: {
                    type: 'boolean',
                  },
                  pinned: {
                    type: 'boolean',
                  },
                },
                required: ['enabled', 'pinned'],
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  kind: {
                    type: 'string',
                    const: 'applied',
                  },
                },
                required: ['kind', 'revision'],
              },
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'changed',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reason: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                required: ['kind', 'reason'],
              },
            ],
            title: 'SkillLibraryPreferenceResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          resources: {},
        },
      },
    },
  },
  skillOutcomes: {
    name: 'maka.skills.Outcomes',
    version: 1,
    transport: 'remote',
    interface: {
      kind: 'service',
      methods: {
        read: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
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
            title: 'RecoveryRef',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              DeploymentRef: {
                type: 'object',
                additionalProperties: false,
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
                required: ['stateDomain', 'deployment', 'revision'],
              },
              InstanceId: {
                type: 'array',
                items: {
                  type: 'string',
                  format: 'uuid',
                },
                description:
                  'Hierarchical, stable source keys. Display aliases never become data identities.',
              },
              Contract: {
                type: 'object',
                additionalProperties: false,
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
                required: ['name', 'version', 'transport', 'interface'],
                description:
                  'Public contract definitions are package data, not live registration records.',
              },
              Transport: {
                type: 'string',
                enum: ['local', 'remote'],
              },
              Interface: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
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
                    required: ['kind', 'methods'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
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
                    required: ['kind', 'props', 'events', 'captures'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      schema: true,
                      kind: {
                        type: 'string',
                        const: 'value',
                      },
                    },
                    required: ['kind', 'schema'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
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
                    required: ['kind', 'resource', 'schema'],
                  },
                ],
              },
              Method: {
                type: 'object',
                additionalProperties: false,
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
                required: ['mode', 'input', 'output', 'resources'],
              },
              MethodMode: {
                type: 'string',
                enum: ['query', 'mutation', 'subscription'],
              },
              ResourceType: {
                type: 'object',
                additionalProperties: false,
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
                required: ['name', 'version'],
                description:
                  'Nominal read-only resource shape, not a resource id or authorization token.',
              },
              Capture: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  input: true,
                  output: {
                    $ref: '#/$defs/CaptureOutput',
                  },
                },
                required: ['input', 'output'],
              },
              CaptureOutput: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      resource: {
                        $ref: '#/$defs/ResourceType',
                      },
                      kind: {
                        type: 'string',
                        const: 'resource',
                      },
                    },
                    required: ['kind', 'resource'],
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'continuation',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                ],
                description:
                  'Resource captures enter checked service slots; continuations remain in the\noriginal presentation owner and only decide admission of query feedback.',
              },
              RecoveryProtocol: {
                type: 'string',
                enum: ['journal_v1'],
              },
            },
          },
          output: {
            type: 'object',
            additionalProperties: false,
            properties: {
              original: {
                $ref: '#/$defs/OperationReply',
              },
              published: {
                anyOf: [
                  {
                    $ref: '#/$defs/SkillLibraryChangeResult',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
            },
            required: ['original'],
            description:
              'Canonical publication evidence alongside the unmodified original Host facts.',
            title: 'SkillLibraryOutcome',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              OperationReply: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  reference: {
                    $ref: '#/$defs/RecoveryRef',
                  },
                  receipt: {
                    $ref: '#/$defs/Receipt',
                  },
                },
                required: ['reference', 'receipt'],
              },
              RecoveryRef: {
                type: 'object',
                additionalProperties: false,
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
                additionalProperties: false,
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
                required: ['stateDomain', 'deployment', 'revision'],
              },
              InstanceId: {
                type: 'array',
                items: {
                  type: 'string',
                  format: 'uuid',
                },
                description:
                  'Hierarchical, stable source keys. Display aliases never become data identities.',
              },
              Contract: {
                type: 'object',
                additionalProperties: false,
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
                required: ['name', 'version', 'transport', 'interface'],
                description:
                  'Public contract definitions are package data, not live registration records.',
              },
              Transport: {
                type: 'string',
                enum: ['local', 'remote'],
              },
              Interface: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
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
                    required: ['kind', 'methods'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
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
                    required: ['kind', 'props', 'events', 'captures'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      schema: true,
                      kind: {
                        type: 'string',
                        const: 'value',
                      },
                    },
                    required: ['kind', 'schema'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
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
                    required: ['kind', 'resource', 'schema'],
                  },
                ],
              },
              Method: {
                type: 'object',
                additionalProperties: false,
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
                required: ['mode', 'input', 'output', 'resources'],
              },
              MethodMode: {
                type: 'string',
                enum: ['query', 'mutation', 'subscription'],
              },
              ResourceType: {
                type: 'object',
                additionalProperties: false,
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
                required: ['name', 'version'],
                description:
                  'Nominal read-only resource shape, not a resource id or authorization token.',
              },
              Capture: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  input: true,
                  output: {
                    $ref: '#/$defs/CaptureOutput',
                  },
                },
                required: ['input', 'output'],
              },
              CaptureOutput: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      resource: {
                        $ref: '#/$defs/ResourceType',
                      },
                      kind: {
                        type: 'string',
                        const: 'resource',
                      },
                    },
                    required: ['kind', 'resource'],
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'continuation',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                ],
                description:
                  'Resource captures enter checked service slots; continuations remain in the\noriginal presentation owner and only decide admission of query feedback.',
              },
              RecoveryProtocol: {
                type: 'string',
                enum: ['journal_v1'],
              },
              Receipt: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'not_recorded',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                    description:
                      'An authorized lookup found no row at the exact original reference.',
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
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
                    required: ['kind', 'outcome', 'cleanup'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      diagnostic: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'unavailable',
                      },
                    },
                    required: ['kind', 'diagnostic'],
                  },
                ],
                description:
                  "A caller's receipt observation, never an execution grant. Unavailable makes\nno claim about the journal's contents or physical cleanup.",
              },
              OperationOutcome: {
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
                    additionalProperties: false,
                    properties: {
                      value: true,
                      kind: {
                        type: 'string',
                        const: 'returned',
                      },
                    },
                    required: ['kind', 'value'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      diagnostic: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'unknown',
                      },
                    },
                    required: ['kind', 'diagnostic'],
                  },
                ],
                description:
                  'Unresolved means no durable result is known, including after process loss.',
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
                    additionalProperties: false,
                    properties: {
                      diagnostic: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'failed',
                      },
                    },
                    required: ['kind', 'diagnostic'],
                  },
                ],
              },
              SkillLibraryChangeResult: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      reference: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'applied',
                      },
                    },
                    required: ['kind', 'reference'],
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'changed',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      reason: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'rejected',
                      },
                    },
                    required: ['kind', 'reason'],
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
  workspaceFiles: {
    name: 'maka.workspace.Files',
    version: 1,
    transport: 'remote',
    interface: {
      kind: 'service',
      methods: {
        applyPatch: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              callId: {
                type: 'string',
                minLength: 1,
                maxLength: 4096,
              },
              operation: {
                $ref: '#/$defs/NativeOperation',
              },
            },
            required: ['callId', 'operation'],
            title: 'NativeInput',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              NativeOperation: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      path: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 4096,
                      },
                      diff: {
                        type: 'string',
                        maxLength: 1048576,
                      },
                      type: {
                        type: 'string',
                        const: 'create_file',
                      },
                    },
                    required: ['type', 'path', 'diff'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      path: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 4096,
                      },
                      diff: {
                        type: 'string',
                        maxLength: 1048576,
                      },
                      type: {
                        type: 'string',
                        const: 'update_file',
                      },
                    },
                    required: ['type', 'path', 'diff'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      path: {
                        type: 'string',
                        minLength: 1,
                        maxLength: 4096,
                      },
                      type: {
                        type: 'string',
                        const: 'delete_file',
                      },
                    },
                    required: ['type', 'path'],
                  },
                ],
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: true,
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
                description:
                  'A known completed failure. Uncertain external effects use the original receipt.',
              },
            ],
            title: 'WorkspaceFileResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          resources: {},
        },
        edit: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              path: {
                type: 'string',
                minLength: 1,
                maxLength: 4096,
              },
              old_string: {
                type: 'string',
                minLength: 1,
                maxLength: 1048576,
              },
              new_string: {
                type: 'string',
                maxLength: 1048576,
              },
            },
            required: ['path', 'old_string', 'new_string'],
            title: 'EditInput',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: true,
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
                description:
                  'A known completed failure. Uncertain external effects use the original receipt.',
              },
            ],
            title: 'WorkspaceFileResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          resources: {},
        },
        glob: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              pattern: {
                type: 'string',
                minLength: 1,
              },
              cwd: {
                type: 'string',
                minLength: 1,
                default: '.',
              },
            },
            required: ['pattern'],
            title: 'Input',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: true,
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
                description:
                  'A known completed failure. Uncertain external effects use the original receipt.',
              },
            ],
            title: 'WorkspaceFileResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          resources: {},
        },
        grep: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              pattern: {
                type: 'string',
                maxLength: 8192,
              },
              path: {
                type: 'string',
                default: '.',
              },
              glob: {
                type: 'string',
                maxLength: 4096,
              },
            },
            required: ['pattern'],
            title: 'Input',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: true,
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
                description:
                  'A known completed failure. Uncertain external effects use the original receipt.',
              },
            ],
            title: 'WorkspaceFileResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          resources: {},
        },
        readImage: {
          mode: 'subscription',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              path: {
                type: 'string',
                minLength: 1,
              },
              offset: {
                type: 'integer',
                format: 'uint',
                minimum: 0,
                maximum: 9007199254740991,
              },
              limit: {
                type: 'integer',
                format: 'uint',
                minimum: 1,
                maximum: 9007199254740991,
              },
            },
            required: ['path'],
            title: 'ReadInput',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  mimeType: {
                    type: 'string',
                    pattern: '^image/(png|jpeg|gif|webp)$',
                  },
                  totalBytes: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 1,
                    maximum: 5242880,
                  },
                  offset: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                    maximum: 5242879,
                  },
                  data: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 43692,
                  },
                  kind: {
                    type: 'string',
                    const: 'chunk',
                  },
                },
                required: ['kind', 'mimeType', 'totalBytes', 'offset', 'data'],
              },
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'complete',
                  },
                },
                required: ['kind'],
                additionalProperties: false,
              },
            ],
            description:
              'Bounded image bytes from a selected subscription. Complete is a producer\nreturn value, never an emitted item or an executor-cleanup acknowledgement.',
            title: 'AgentToolImageFrame',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          resources: {},
        },
        readText: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              path: {
                type: 'string',
                minLength: 1,
              },
              offset: {
                type: 'integer',
                format: 'uint',
                minimum: 0,
                maximum: 9007199254740991,
              },
              limit: {
                type: 'integer',
                format: 'uint',
                minimum: 1,
                maximum: 9007199254740991,
              },
            },
            required: ['path'],
            title: 'ReadInput',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: true,
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
                description:
                  'A known completed failure. Uncertain external effects use the original receipt.',
              },
            ],
            title: 'WorkspaceFileResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          resources: {},
        },
        write: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              path: {
                type: 'string',
                minLength: 1,
                maxLength: 4096,
              },
              content: {
                type: 'string',
                maxLength: 1048576,
              },
            },
            required: ['path', 'content'],
            title: 'WriteInput',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: true,
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
                description:
                  'A known completed failure. Uncertain external effects use the original receipt.',
              },
            ],
            title: 'WorkspaceFileResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          resources: {},
        },
      },
    },
  },
  workspaceProcesses: {
    name: 'maka.workspace.Processes',
    version: 1,
    transport: 'remote',
    interface: {
      kind: 'service',
      methods: {
        execute: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              command: {
                type: 'string',
                minLength: 1,
                maxLength: 65536,
              },
              login: {
                type: 'boolean',
                description: "Load the shell's login profile. Set false to skip it.",
                default: true,
              },
              timeout_ms: {
                type: ['integer', 'null'],
                format: 'uint64',
                minimum: 1,
                maximum: 86400000,
              },
              run_in_background: {
                type: 'boolean',
                default: false,
              },
              pty: {
                type: 'boolean',
                description: 'Allocate a terminal; requires run_in_background=true.',
                default: false,
              },
              additional_permissions: {
                anyOf: [
                  {
                    $ref: '#/$defs/Permissions',
                  },
                  {
                    type: 'null',
                  },
                ],
                description:
                  'Request additional filesystem or network access before this command.\nOn Linux/Windows, request an existing parent directory when creating a new path.',
              },
              justification: {
                type: ['string', 'null'],
                description:
                  'Explain why the additional access is necessary. Required with additional_permissions.',
              },
            },
            required: ['command'],
            title: 'Execute',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              Permissions: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  filesystem: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/Rule',
                    },
                    maxItems: 32,
                  },
                  network: {
                    $ref: '#/$defs/Network',
                  },
                },
                required: ['filesystem', 'network'],
              },
              Rule: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  path: {
                    type: 'string',
                  },
                  scope: {
                    $ref: '#/$defs/Scope',
                  },
                  access: {
                    $ref: '#/$defs/Access',
                  },
                },
                required: ['path', 'scope', 'access'],
                description:
                  'Paths have executor-native meaning. Validate and materialize them on the\nexecution host, never on a remote Desktop. This document alone grants nothing.',
              },
              Scope: {
                type: 'string',
                enum: ['subtree', 'exact'],
              },
              Access: {
                type: 'string',
                enum: ['read', 'write', 'deny'],
                description:
                  'Equal-path conflict priority is different from authority breadth: deny wins.',
              },
              Network: {
                oneOf: [
                  {
                    type: 'string',
                    enum: ['denied', 'allowed'],
                  },
                  {
                    type: 'object',
                    properties: {
                      restricted: {
                        type: 'object',
                        properties: {
                          destinations: {
                            type: 'array',
                            uniqueItems: true,
                            items: {
                              $ref: '#/$defs/Destination',
                            },
                            maxItems: 128,
                          },
                        },
                        additionalProperties: false,
                        required: ['destinations'],
                      },
                    },
                    required: ['restricted'],
                    additionalProperties: false,
                  },
                ],
              },
              Destination: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  host: {
                    type: 'string',
                    minLength: 1,
                    maxLength: 253,
                  },
                  port: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 1,
                    maximum: 65535,
                  },
                },
                required: ['host', 'port'],
                description:
                  'Destination authority, independent of the upstream proxy used to reach it.\nA domain grant does not authorize its subdomains or other ports.',
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: true,
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
              },
            ],
            title: 'WorkspaceProcessResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          resources: {},
        },
        read: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              ref: {
                type: 'string',
              },
            },
            required: ['ref'],
            title: 'Reference',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: true,
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
              },
            ],
            title: 'WorkspaceProcessResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          resources: {},
        },
        stop: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              ref: {
                type: 'string',
              },
            },
            required: ['ref'],
            title: 'Reference',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: true,
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
              },
            ],
            title: 'WorkspaceProcessResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          resources: {},
        },
        writeStdin: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            required: ['ref'],
            properties: {
              ref: {
                type: 'string',
              },
              input: {
                type: 'string',
                description: 'Raw terminal input, exclusive with actions.',
              },
              actions: {
                type: ['array', 'null'],
                maxItems: 64,
                items: {
                  type: 'object',
                  additionalProperties: false,
                  required: ['type'],
                  properties: {
                    type: {
                      enum: ['text', 'key', 'mouse'],
                    },
                    text: {
                      type: ['string', 'number', 'null'],
                    },
                    key: {
                      type: ['string', 'number', 'null'],
                    },
                    event: {
                      type: ['string', 'number', 'null'],
                    },
                    x: {
                      type: ['number', 'string', 'null'],
                    },
                    y: {
                      type: ['number', 'string', 'null'],
                    },
                    button: {
                      type: ['string', 'number', 'null'],
                    },
                    direction: {
                      type: ['string', 'number', 'null'],
                    },
                    modifiers: {
                      type: ['array', 'null'],
                      items: {
                        enum: ['ctrl', 'alt', 'shift'],
                      },
                    },
                  },
                },
              },
              size: {
                type: ['object', 'null'],
                additionalProperties: false,
                properties: {
                  cols: {
                    type: ['number', 'null'],
                  },
                  rows: {
                    type: ['number', 'null'],
                  },
                },
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: true,
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
              },
            ],
            title: 'WorkspaceProcessResult',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          resources: {},
        },
      },
    },
  },
  terminal: {
    name: 'maka.workspace.Terminal',
    version: 1,
    transport: 'remote',
    interface: {
      kind: 'service',
      methods: {
        acquire: {
          mode: 'subscription',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              sessionId: {
                type: 'string',
              },
              ref: {
                type: 'string',
              },
            },
            required: ['sessionId', 'ref'],
            title: 'TerminalTarget',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: {
                    $ref: '#/$defs/TerminalLeaseUpdate',
                  },
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
              },
            ],
            title: 'TerminalOutcome_TerminalLeaseUpdate',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              TerminalLeaseUpdate: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  controllerId: {
                    type: 'string',
                    format: 'uuid',
                  },
                  nextSequence: {
                    type: ['integer', 'null'],
                    format: 'uint64',
                    minimum: 0,
                  },
                  held: {
                    type: 'boolean',
                  },
                  resource: {
                    $ref: '#/$defs/ResourceUpdate',
                  },
                },
                required: ['controllerId', 'held', 'resource'],
              },
              ResourceUpdate: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  ownership: {
                    $ref: '#/$defs/Ownership',
                  },
                  sourceTurnId: {
                    type: 'string',
                  },
                  sourceToolCallId: {
                    type: 'string',
                  },
                  result: {
                    $ref: '#/$defs/ShellSnapshot',
                  },
                },
                required: ['sessionId', 'ownership', 'sourceTurnId', 'sourceToolCallId', 'result'],
              },
              Ownership: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'local',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sourceSessionId: {
                        type: 'string',
                      },
                      ownerSessionId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'source_owned',
                      },
                    },
                    required: ['kind', 'sourceSessionId', 'ownerSessionId'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sourceSessionId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'source_unavailable',
                      },
                    },
                    required: ['kind', 'sourceSessionId'],
                  },
                ],
              },
              ShellSnapshot: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  kind: {
                    $ref: '#/$defs/SnapshotKind',
                  },
                  ref: {
                    type: 'string',
                  },
                  mode: {
                    $ref: '#/$defs/ShellMode',
                  },
                  status: {
                    $ref: '#/$defs/ShellStatus',
                  },
                  cwd: {
                    type: 'string',
                  },
                  cmd: {
                    type: 'string',
                  },
                  startedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  updatedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  completedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  timeoutMs: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  exitCode: {
                    type: 'integer',
                    format: 'int64',
                  },
                  failureMessage: {
                    type: 'string',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  output: {
                    $ref: '#/$defs/ShellOutput',
                  },
                },
                required: [
                  'kind',
                  'ref',
                  'mode',
                  'status',
                  'cwd',
                  'cmd',
                  'startedAt',
                  'updatedAt',
                  'revision',
                ],
              },
              SnapshotKind: {
                type: 'string',
                enum: ['shell_run'],
              },
              ShellMode: {
                type: 'string',
                enum: ['pipes', 'pty'],
              },
              ShellStatus: {
                type: 'string',
                enum: [
                  'starting',
                  'running',
                  'completed',
                  'failed',
                  'timed_out',
                  'cancelled',
                  'orphaned',
                ],
              },
              ShellOutput: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      stdout: {
                        type: 'string',
                      },
                      stderr: {
                        type: 'string',
                      },
                      latestStream: {
                        $ref: '#/$defs/PipeStream',
                      },
                      stdoutTruncated: {
                        type: 'boolean',
                      },
                      stderrTruncated: {
                        type: 'boolean',
                      },
                      redacted: {
                        type: 'boolean',
                      },
                      mode: {
                        type: 'string',
                        const: 'pipes',
                      },
                    },
                    required: [
                      'mode',
                      'stdout',
                      'stderr',
                      'stdoutTruncated',
                      'stderrTruncated',
                      'redacted',
                    ],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      screen: {
                        type: 'string',
                      },
                      scrollback: {
                        type: 'string',
                      },
                      lastAlternateScreen: {
                        type: 'string',
                      },
                      cols: {
                        type: 'integer',
                        format: 'uint16',
                        minimum: 0,
                        maximum: 65535,
                      },
                      rows: {
                        type: 'integer',
                        format: 'uint16',
                        minimum: 0,
                        maximum: 65535,
                      },
                      cursor: {
                        $ref: '#/$defs/TerminalCursor',
                      },
                      alternateScreen: {
                        type: 'boolean',
                      },
                      truncated: {
                        type: 'boolean',
                      },
                      redacted: {
                        type: 'boolean',
                      },
                      mode: {
                        type: 'string',
                        const: 'pty',
                      },
                    },
                    required: [
                      'mode',
                      'screen',
                      'scrollback',
                      'cols',
                      'rows',
                      'cursor',
                      'alternateScreen',
                      'truncated',
                      'redacted',
                    ],
                  },
                ],
              },
              PipeStream: {
                type: 'string',
                enum: ['stdout', 'stderr'],
              },
              TerminalCursor: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  x: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  y: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  visible: {
                    type: 'boolean',
                  },
                },
                required: ['x', 'y', 'visible'],
              },
            },
          },
          resources: {},
        },
        control: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              controller: {
                $ref: '#/$defs/TerminalController',
              },
              sequence: {
                type: 'integer',
                format: 'uint64',
                minimum: 1,
                maximum: 9007199254740991,
              },
              actions: {
                type: 'array',
                items: {
                  $ref: '#/$defs/RawAction',
                },
                maxItems: 64,
                default: [],
              },
              size: {
                anyOf: [
                  {
                    $ref: '#/$defs/TerminalSize',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
            },
            required: ['controller', 'sequence'],
            title: 'TerminalControl',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              TerminalController: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  target: {
                    $ref: '#/$defs/TerminalTarget',
                  },
                  controllerId: {
                    type: 'string',
                    format: 'uuid',
                  },
                },
                required: ['target', 'controllerId'],
              },
              TerminalTarget: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  ref: {
                    type: 'string',
                  },
                },
                required: ['sessionId', 'ref'],
              },
              RawAction: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      text: {
                        type: 'string',
                        minLength: 1,
                        pattern: '^[^\\x00-\\x1f\\x7f-\\x9f]+$',
                      },
                      type: {
                        type: 'string',
                        const: 'text',
                      },
                    },
                    required: ['type', 'text'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      text: {
                        type: 'string',
                        minLength: 1,
                        pattern: '^[^\\x00-\\x08\\x0b\\x0c\\x0e-\\x1f\\x7f-\\x9f]+$',
                      },
                      type: {
                        type: 'string',
                        const: 'paste',
                      },
                    },
                    required: ['type', 'text'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      key: {
                        type: 'string',
                        pattern:
                          '^(?:[ -~]|enter|escape|tab|backspace|arrow_(?:up|down|left|right)|home|end|insert|delete|page_(?:up|down)|f(?:[1-9]|1[0-2]))$',
                      },
                      modifiers: {
                        $ref: '#/$defs/TerminalModifiers',
                        default: [],
                      },
                      type: {
                        type: 'string',
                        const: 'key',
                      },
                    },
                    required: ['type', 'key'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      event: {
                        $ref: '#/$defs/RawEvent',
                      },
                      x: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                        maximum: 9007199254740991,
                      },
                      y: {
                        type: 'integer',
                        format: 'uint64',
                        minimum: 0,
                        maximum: 9007199254740991,
                      },
                      button: {
                        $ref: '#/$defs/MouseButton',
                        default: null,
                      },
                      direction: {
                        $ref: '#/$defs/ScrollDirection',
                        default: null,
                      },
                      modifiers: {
                        $ref: '#/$defs/TerminalModifiers',
                        default: [],
                      },
                      type: {
                        type: 'string',
                        const: 'mouse',
                      },
                    },
                    required: ['type', 'event', 'x', 'y'],
                  },
                ],
              },
              TerminalModifiers: {
                type: 'array',
                items: {
                  $ref: '#/$defs/Modifier',
                },
                uniqueItems: true,
                maxItems: 3,
              },
              Modifier: {
                type: 'string',
                enum: ['ctrl', 'alt', 'shift'],
              },
              RawEvent: {
                type: 'string',
                enum: ['click', 'press', 'release', 'move', 'scroll'],
              },
              MouseButton: {
                type: 'string',
                enum: ['left', 'middle', 'right'],
              },
              ScrollDirection: {
                type: 'string',
                enum: ['up', 'down'],
              },
              TerminalSize: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  cols: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 2,
                    maximum: 240,
                  },
                  rows: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 1,
                    maximum: 100,
                  },
                },
                required: ['cols', 'rows'],
                description:
                  'Shared by native PTY allocation, input validation and screen rendering.',
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: {
                    $ref: '#/$defs/TerminalControlReceipt',
                  },
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
              },
            ],
            title: 'TerminalOutcome_TerminalControlReceipt',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              TerminalControlReceipt: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  controllerId: {
                    type: 'string',
                    format: 'uuid',
                  },
                  sequence: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  acceptedBytes: {
                    type: 'integer',
                    format: 'uint',
                    minimum: 0,
                  },
                  resized: {
                    type: 'boolean',
                  },
                  resizeChanged: {
                    type: 'boolean',
                  },
                },
                required: ['controllerId', 'sequence', 'acceptedBytes', 'resized', 'resizeChanged'],
              },
            },
          },
          resources: {},
        },
        list: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              sessionId: {
                type: 'string',
              },
              offset: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 9007199254740991,
                default: 0,
              },
            },
            required: ['sessionId'],
            title: 'TerminalList',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: {
                    $ref: '#/$defs/TerminalPage',
                  },
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
              },
            ],
            title: 'TerminalOutcome_TerminalPage',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              TerminalPage: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  resources: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/ResourceUpdate',
                    },
                  },
                  revision: {
                    type: 'string',
                  },
                  total: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  nextOffset: {
                    type: ['integer', 'null'],
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                required: ['resources', 'revision', 'total'],
              },
              ResourceUpdate: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  ownership: {
                    $ref: '#/$defs/Ownership',
                  },
                  sourceTurnId: {
                    type: 'string',
                  },
                  sourceToolCallId: {
                    type: 'string',
                  },
                  result: {
                    $ref: '#/$defs/ShellSnapshot',
                  },
                },
                required: ['sessionId', 'ownership', 'sourceTurnId', 'sourceToolCallId', 'result'],
              },
              Ownership: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'local',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sourceSessionId: {
                        type: 'string',
                      },
                      ownerSessionId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'source_owned',
                      },
                    },
                    required: ['kind', 'sourceSessionId', 'ownerSessionId'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sourceSessionId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'source_unavailable',
                      },
                    },
                    required: ['kind', 'sourceSessionId'],
                  },
                ],
              },
              ShellSnapshot: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  kind: {
                    $ref: '#/$defs/SnapshotKind',
                  },
                  ref: {
                    type: 'string',
                  },
                  mode: {
                    $ref: '#/$defs/ShellMode',
                  },
                  status: {
                    $ref: '#/$defs/ShellStatus',
                  },
                  cwd: {
                    type: 'string',
                  },
                  cmd: {
                    type: 'string',
                  },
                  startedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  updatedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  completedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  timeoutMs: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  exitCode: {
                    type: 'integer',
                    format: 'int64',
                  },
                  failureMessage: {
                    type: 'string',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  output: {
                    $ref: '#/$defs/ShellOutput',
                  },
                },
                required: [
                  'kind',
                  'ref',
                  'mode',
                  'status',
                  'cwd',
                  'cmd',
                  'startedAt',
                  'updatedAt',
                  'revision',
                ],
              },
              SnapshotKind: {
                type: 'string',
                enum: ['shell_run'],
              },
              ShellMode: {
                type: 'string',
                enum: ['pipes', 'pty'],
              },
              ShellStatus: {
                type: 'string',
                enum: [
                  'starting',
                  'running',
                  'completed',
                  'failed',
                  'timed_out',
                  'cancelled',
                  'orphaned',
                ],
              },
              ShellOutput: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      stdout: {
                        type: 'string',
                      },
                      stderr: {
                        type: 'string',
                      },
                      latestStream: {
                        $ref: '#/$defs/PipeStream',
                      },
                      stdoutTruncated: {
                        type: 'boolean',
                      },
                      stderrTruncated: {
                        type: 'boolean',
                      },
                      redacted: {
                        type: 'boolean',
                      },
                      mode: {
                        type: 'string',
                        const: 'pipes',
                      },
                    },
                    required: [
                      'mode',
                      'stdout',
                      'stderr',
                      'stdoutTruncated',
                      'stderrTruncated',
                      'redacted',
                    ],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      screen: {
                        type: 'string',
                      },
                      scrollback: {
                        type: 'string',
                      },
                      lastAlternateScreen: {
                        type: 'string',
                      },
                      cols: {
                        type: 'integer',
                        format: 'uint16',
                        minimum: 0,
                        maximum: 65535,
                      },
                      rows: {
                        type: 'integer',
                        format: 'uint16',
                        minimum: 0,
                        maximum: 65535,
                      },
                      cursor: {
                        $ref: '#/$defs/TerminalCursor',
                      },
                      alternateScreen: {
                        type: 'boolean',
                      },
                      truncated: {
                        type: 'boolean',
                      },
                      redacted: {
                        type: 'boolean',
                      },
                      mode: {
                        type: 'string',
                        const: 'pty',
                      },
                    },
                    required: [
                      'mode',
                      'screen',
                      'scrollback',
                      'cols',
                      'rows',
                      'cursor',
                      'alternateScreen',
                      'truncated',
                      'redacted',
                    ],
                  },
                ],
              },
              PipeStream: {
                type: 'string',
                enum: ['stdout', 'stderr'],
              },
              TerminalCursor: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  x: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  y: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  visible: {
                    type: 'boolean',
                  },
                },
                required: ['x', 'y', 'visible'],
              },
            },
          },
          resources: {},
        },
        read: {
          mode: 'query',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              sessionId: {
                type: 'string',
              },
              ref: {
                type: 'string',
              },
            },
            required: ['sessionId', 'ref'],
            title: 'TerminalTarget',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: {
                    $ref: '#/$defs/ResourceUpdate',
                  },
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
              },
            ],
            title: 'TerminalOutcome_ResourceUpdate',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              ResourceUpdate: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  ownership: {
                    $ref: '#/$defs/Ownership',
                  },
                  sourceTurnId: {
                    type: 'string',
                  },
                  sourceToolCallId: {
                    type: 'string',
                  },
                  result: {
                    $ref: '#/$defs/ShellSnapshot',
                  },
                },
                required: ['sessionId', 'ownership', 'sourceTurnId', 'sourceToolCallId', 'result'],
              },
              Ownership: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'local',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sourceSessionId: {
                        type: 'string',
                      },
                      ownerSessionId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'source_owned',
                      },
                    },
                    required: ['kind', 'sourceSessionId', 'ownerSessionId'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sourceSessionId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'source_unavailable',
                      },
                    },
                    required: ['kind', 'sourceSessionId'],
                  },
                ],
              },
              ShellSnapshot: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  kind: {
                    $ref: '#/$defs/SnapshotKind',
                  },
                  ref: {
                    type: 'string',
                  },
                  mode: {
                    $ref: '#/$defs/ShellMode',
                  },
                  status: {
                    $ref: '#/$defs/ShellStatus',
                  },
                  cwd: {
                    type: 'string',
                  },
                  cmd: {
                    type: 'string',
                  },
                  startedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  updatedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  completedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  timeoutMs: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  exitCode: {
                    type: 'integer',
                    format: 'int64',
                  },
                  failureMessage: {
                    type: 'string',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  output: {
                    $ref: '#/$defs/ShellOutput',
                  },
                },
                required: [
                  'kind',
                  'ref',
                  'mode',
                  'status',
                  'cwd',
                  'cmd',
                  'startedAt',
                  'updatedAt',
                  'revision',
                ],
              },
              SnapshotKind: {
                type: 'string',
                enum: ['shell_run'],
              },
              ShellMode: {
                type: 'string',
                enum: ['pipes', 'pty'],
              },
              ShellStatus: {
                type: 'string',
                enum: [
                  'starting',
                  'running',
                  'completed',
                  'failed',
                  'timed_out',
                  'cancelled',
                  'orphaned',
                ],
              },
              ShellOutput: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      stdout: {
                        type: 'string',
                      },
                      stderr: {
                        type: 'string',
                      },
                      latestStream: {
                        $ref: '#/$defs/PipeStream',
                      },
                      stdoutTruncated: {
                        type: 'boolean',
                      },
                      stderrTruncated: {
                        type: 'boolean',
                      },
                      redacted: {
                        type: 'boolean',
                      },
                      mode: {
                        type: 'string',
                        const: 'pipes',
                      },
                    },
                    required: [
                      'mode',
                      'stdout',
                      'stderr',
                      'stdoutTruncated',
                      'stderrTruncated',
                      'redacted',
                    ],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      screen: {
                        type: 'string',
                      },
                      scrollback: {
                        type: 'string',
                      },
                      lastAlternateScreen: {
                        type: 'string',
                      },
                      cols: {
                        type: 'integer',
                        format: 'uint16',
                        minimum: 0,
                        maximum: 65535,
                      },
                      rows: {
                        type: 'integer',
                        format: 'uint16',
                        minimum: 0,
                        maximum: 65535,
                      },
                      cursor: {
                        $ref: '#/$defs/TerminalCursor',
                      },
                      alternateScreen: {
                        type: 'boolean',
                      },
                      truncated: {
                        type: 'boolean',
                      },
                      redacted: {
                        type: 'boolean',
                      },
                      mode: {
                        type: 'string',
                        const: 'pty',
                      },
                    },
                    required: [
                      'mode',
                      'screen',
                      'scrollback',
                      'cols',
                      'rows',
                      'cursor',
                      'alternateScreen',
                      'truncated',
                      'redacted',
                    ],
                  },
                ],
              },
              PipeStream: {
                type: 'string',
                enum: ['stdout', 'stderr'],
              },
              TerminalCursor: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  x: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  y: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  visible: {
                    type: 'boolean',
                  },
                },
                required: ['x', 'y', 'visible'],
              },
            },
          },
          resources: {},
        },
        release: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              target: {
                $ref: '#/$defs/TerminalTarget',
              },
              controllerId: {
                type: 'string',
                format: 'uuid',
              },
            },
            required: ['target', 'controllerId'],
            title: 'TerminalController',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              TerminalTarget: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  ref: {
                    type: 'string',
                  },
                },
                required: ['sessionId', 'ref'],
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: {
                    $ref: '#/$defs/TerminalReleased',
                  },
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
              },
            ],
            title: 'TerminalOutcome_TerminalReleased',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              TerminalReleased: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  controllerId: {
                    type: 'string',
                    format: 'uuid',
                  },
                  released: {
                    type: 'boolean',
                  },
                },
                required: ['controllerId', 'released'],
              },
            },
          },
          resources: {},
        },
        start: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              sessionId: {
                type: 'string',
              },
              command: {
                type: ['string', 'null'],
                description:
                  'Omit for an interactive shell; an explicit command runs with bounded pipes.',
              },
              size: {
                anyOf: [
                  {
                    $ref: '#/$defs/TerminalSize',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
            },
            required: ['sessionId'],
            title: 'TerminalStart',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              TerminalSize: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  cols: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 2,
                    maximum: 240,
                  },
                  rows: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 1,
                    maximum: 100,
                  },
                },
                required: ['cols', 'rows'],
                description:
                  'Shared by native PTY allocation, input validation and screen rendering.',
              },
            },
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: {
                    $ref: '#/$defs/ResourceUpdate',
                  },
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
              },
            ],
            title: 'TerminalOutcome_ResourceUpdate',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              ResourceUpdate: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  ownership: {
                    $ref: '#/$defs/Ownership',
                  },
                  sourceTurnId: {
                    type: 'string',
                  },
                  sourceToolCallId: {
                    type: 'string',
                  },
                  result: {
                    $ref: '#/$defs/ShellSnapshot',
                  },
                },
                required: ['sessionId', 'ownership', 'sourceTurnId', 'sourceToolCallId', 'result'],
              },
              Ownership: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'local',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sourceSessionId: {
                        type: 'string',
                      },
                      ownerSessionId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'source_owned',
                      },
                    },
                    required: ['kind', 'sourceSessionId', 'ownerSessionId'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sourceSessionId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'source_unavailable',
                      },
                    },
                    required: ['kind', 'sourceSessionId'],
                  },
                ],
              },
              ShellSnapshot: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  kind: {
                    $ref: '#/$defs/SnapshotKind',
                  },
                  ref: {
                    type: 'string',
                  },
                  mode: {
                    $ref: '#/$defs/ShellMode',
                  },
                  status: {
                    $ref: '#/$defs/ShellStatus',
                  },
                  cwd: {
                    type: 'string',
                  },
                  cmd: {
                    type: 'string',
                  },
                  startedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  updatedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  completedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  timeoutMs: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  exitCode: {
                    type: 'integer',
                    format: 'int64',
                  },
                  failureMessage: {
                    type: 'string',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  output: {
                    $ref: '#/$defs/ShellOutput',
                  },
                },
                required: [
                  'kind',
                  'ref',
                  'mode',
                  'status',
                  'cwd',
                  'cmd',
                  'startedAt',
                  'updatedAt',
                  'revision',
                ],
              },
              SnapshotKind: {
                type: 'string',
                enum: ['shell_run'],
              },
              ShellMode: {
                type: 'string',
                enum: ['pipes', 'pty'],
              },
              ShellStatus: {
                type: 'string',
                enum: [
                  'starting',
                  'running',
                  'completed',
                  'failed',
                  'timed_out',
                  'cancelled',
                  'orphaned',
                ],
              },
              ShellOutput: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      stdout: {
                        type: 'string',
                      },
                      stderr: {
                        type: 'string',
                      },
                      latestStream: {
                        $ref: '#/$defs/PipeStream',
                      },
                      stdoutTruncated: {
                        type: 'boolean',
                      },
                      stderrTruncated: {
                        type: 'boolean',
                      },
                      redacted: {
                        type: 'boolean',
                      },
                      mode: {
                        type: 'string',
                        const: 'pipes',
                      },
                    },
                    required: [
                      'mode',
                      'stdout',
                      'stderr',
                      'stdoutTruncated',
                      'stderrTruncated',
                      'redacted',
                    ],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      screen: {
                        type: 'string',
                      },
                      scrollback: {
                        type: 'string',
                      },
                      lastAlternateScreen: {
                        type: 'string',
                      },
                      cols: {
                        type: 'integer',
                        format: 'uint16',
                        minimum: 0,
                        maximum: 65535,
                      },
                      rows: {
                        type: 'integer',
                        format: 'uint16',
                        minimum: 0,
                        maximum: 65535,
                      },
                      cursor: {
                        $ref: '#/$defs/TerminalCursor',
                      },
                      alternateScreen: {
                        type: 'boolean',
                      },
                      truncated: {
                        type: 'boolean',
                      },
                      redacted: {
                        type: 'boolean',
                      },
                      mode: {
                        type: 'string',
                        const: 'pty',
                      },
                    },
                    required: [
                      'mode',
                      'screen',
                      'scrollback',
                      'cols',
                      'rows',
                      'cursor',
                      'alternateScreen',
                      'truncated',
                      'redacted',
                    ],
                  },
                ],
              },
              PipeStream: {
                type: 'string',
                enum: ['stdout', 'stderr'],
              },
              TerminalCursor: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  x: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  y: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  visible: {
                    type: 'boolean',
                  },
                },
                required: ['x', 'y', 'visible'],
              },
            },
          },
          resources: {},
        },
        stop: {
          mode: 'mutation',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              sessionId: {
                type: 'string',
              },
              ref: {
                type: 'string',
              },
            },
            required: ['sessionId', 'ref'],
            title: 'TerminalTarget',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: {
                    $ref: '#/$defs/TerminalStopped',
                  },
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
              },
            ],
            title: 'TerminalOutcome_TerminalStopped',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              TerminalStopped: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  applied: {
                    type: 'boolean',
                  },
                  resource: {
                    $ref: '#/$defs/ResourceUpdate',
                  },
                },
                required: ['applied', 'resource'],
              },
              ResourceUpdate: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  ownership: {
                    $ref: '#/$defs/Ownership',
                  },
                  sourceTurnId: {
                    type: 'string',
                  },
                  sourceToolCallId: {
                    type: 'string',
                  },
                  result: {
                    $ref: '#/$defs/ShellSnapshot',
                  },
                },
                required: ['sessionId', 'ownership', 'sourceTurnId', 'sourceToolCallId', 'result'],
              },
              Ownership: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'local',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sourceSessionId: {
                        type: 'string',
                      },
                      ownerSessionId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'source_owned',
                      },
                    },
                    required: ['kind', 'sourceSessionId', 'ownerSessionId'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sourceSessionId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'source_unavailable',
                      },
                    },
                    required: ['kind', 'sourceSessionId'],
                  },
                ],
              },
              ShellSnapshot: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  kind: {
                    $ref: '#/$defs/SnapshotKind',
                  },
                  ref: {
                    type: 'string',
                  },
                  mode: {
                    $ref: '#/$defs/ShellMode',
                  },
                  status: {
                    $ref: '#/$defs/ShellStatus',
                  },
                  cwd: {
                    type: 'string',
                  },
                  cmd: {
                    type: 'string',
                  },
                  startedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  updatedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  completedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  timeoutMs: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  exitCode: {
                    type: 'integer',
                    format: 'int64',
                  },
                  failureMessage: {
                    type: 'string',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  output: {
                    $ref: '#/$defs/ShellOutput',
                  },
                },
                required: [
                  'kind',
                  'ref',
                  'mode',
                  'status',
                  'cwd',
                  'cmd',
                  'startedAt',
                  'updatedAt',
                  'revision',
                ],
              },
              SnapshotKind: {
                type: 'string',
                enum: ['shell_run'],
              },
              ShellMode: {
                type: 'string',
                enum: ['pipes', 'pty'],
              },
              ShellStatus: {
                type: 'string',
                enum: [
                  'starting',
                  'running',
                  'completed',
                  'failed',
                  'timed_out',
                  'cancelled',
                  'orphaned',
                ],
              },
              ShellOutput: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      stdout: {
                        type: 'string',
                      },
                      stderr: {
                        type: 'string',
                      },
                      latestStream: {
                        $ref: '#/$defs/PipeStream',
                      },
                      stdoutTruncated: {
                        type: 'boolean',
                      },
                      stderrTruncated: {
                        type: 'boolean',
                      },
                      redacted: {
                        type: 'boolean',
                      },
                      mode: {
                        type: 'string',
                        const: 'pipes',
                      },
                    },
                    required: [
                      'mode',
                      'stdout',
                      'stderr',
                      'stdoutTruncated',
                      'stderrTruncated',
                      'redacted',
                    ],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      screen: {
                        type: 'string',
                      },
                      scrollback: {
                        type: 'string',
                      },
                      lastAlternateScreen: {
                        type: 'string',
                      },
                      cols: {
                        type: 'integer',
                        format: 'uint16',
                        minimum: 0,
                        maximum: 65535,
                      },
                      rows: {
                        type: 'integer',
                        format: 'uint16',
                        minimum: 0,
                        maximum: 65535,
                      },
                      cursor: {
                        $ref: '#/$defs/TerminalCursor',
                      },
                      alternateScreen: {
                        type: 'boolean',
                      },
                      truncated: {
                        type: 'boolean',
                      },
                      redacted: {
                        type: 'boolean',
                      },
                      mode: {
                        type: 'string',
                        const: 'pty',
                      },
                    },
                    required: [
                      'mode',
                      'screen',
                      'scrollback',
                      'cols',
                      'rows',
                      'cursor',
                      'alternateScreen',
                      'truncated',
                      'redacted',
                    ],
                  },
                ],
              },
              PipeStream: {
                type: 'string',
                enum: ['stdout', 'stderr'],
              },
              TerminalCursor: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  x: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  y: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  visible: {
                    type: 'boolean',
                  },
                },
                required: ['x', 'y', 'visible'],
              },
            },
          },
          resources: {},
        },
        watch: {
          mode: 'subscription',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              sessionId: {
                type: 'string',
              },
              ref: {
                type: 'string',
              },
            },
            required: ['sessionId', 'ref'],
            title: 'TerminalTarget',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: {
                    $ref: '#/$defs/ResourceUpdate',
                  },
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
              },
            ],
            title: 'TerminalOutcome_ResourceUpdate',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              ResourceUpdate: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  ownership: {
                    $ref: '#/$defs/Ownership',
                  },
                  sourceTurnId: {
                    type: 'string',
                  },
                  sourceToolCallId: {
                    type: 'string',
                  },
                  result: {
                    $ref: '#/$defs/ShellSnapshot',
                  },
                },
                required: ['sessionId', 'ownership', 'sourceTurnId', 'sourceToolCallId', 'result'],
              },
              Ownership: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'local',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sourceSessionId: {
                        type: 'string',
                      },
                      ownerSessionId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'source_owned',
                      },
                    },
                    required: ['kind', 'sourceSessionId', 'ownerSessionId'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sourceSessionId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'source_unavailable',
                      },
                    },
                    required: ['kind', 'sourceSessionId'],
                  },
                ],
              },
              ShellSnapshot: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  kind: {
                    $ref: '#/$defs/SnapshotKind',
                  },
                  ref: {
                    type: 'string',
                  },
                  mode: {
                    $ref: '#/$defs/ShellMode',
                  },
                  status: {
                    $ref: '#/$defs/ShellStatus',
                  },
                  cwd: {
                    type: 'string',
                  },
                  cmd: {
                    type: 'string',
                  },
                  startedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  updatedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  completedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  timeoutMs: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  exitCode: {
                    type: 'integer',
                    format: 'int64',
                  },
                  failureMessage: {
                    type: 'string',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  output: {
                    $ref: '#/$defs/ShellOutput',
                  },
                },
                required: [
                  'kind',
                  'ref',
                  'mode',
                  'status',
                  'cwd',
                  'cmd',
                  'startedAt',
                  'updatedAt',
                  'revision',
                ],
              },
              SnapshotKind: {
                type: 'string',
                enum: ['shell_run'],
              },
              ShellMode: {
                type: 'string',
                enum: ['pipes', 'pty'],
              },
              ShellStatus: {
                type: 'string',
                enum: [
                  'starting',
                  'running',
                  'completed',
                  'failed',
                  'timed_out',
                  'cancelled',
                  'orphaned',
                ],
              },
              ShellOutput: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      stdout: {
                        type: 'string',
                      },
                      stderr: {
                        type: 'string',
                      },
                      latestStream: {
                        $ref: '#/$defs/PipeStream',
                      },
                      stdoutTruncated: {
                        type: 'boolean',
                      },
                      stderrTruncated: {
                        type: 'boolean',
                      },
                      redacted: {
                        type: 'boolean',
                      },
                      mode: {
                        type: 'string',
                        const: 'pipes',
                      },
                    },
                    required: [
                      'mode',
                      'stdout',
                      'stderr',
                      'stdoutTruncated',
                      'stderrTruncated',
                      'redacted',
                    ],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      screen: {
                        type: 'string',
                      },
                      scrollback: {
                        type: 'string',
                      },
                      lastAlternateScreen: {
                        type: 'string',
                      },
                      cols: {
                        type: 'integer',
                        format: 'uint16',
                        minimum: 0,
                        maximum: 65535,
                      },
                      rows: {
                        type: 'integer',
                        format: 'uint16',
                        minimum: 0,
                        maximum: 65535,
                      },
                      cursor: {
                        $ref: '#/$defs/TerminalCursor',
                      },
                      alternateScreen: {
                        type: 'boolean',
                      },
                      truncated: {
                        type: 'boolean',
                      },
                      redacted: {
                        type: 'boolean',
                      },
                      mode: {
                        type: 'string',
                        const: 'pty',
                      },
                    },
                    required: [
                      'mode',
                      'screen',
                      'scrollback',
                      'cols',
                      'rows',
                      'cursor',
                      'alternateScreen',
                      'truncated',
                      'redacted',
                    ],
                  },
                ],
              },
              PipeStream: {
                type: 'string',
                enum: ['stdout', 'stderr'],
              },
              TerminalCursor: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  x: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  y: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  visible: {
                    type: 'boolean',
                  },
                },
                required: ['x', 'y', 'visible'],
              },
            },
          },
          resources: {},
        },
        watchList: {
          mode: 'subscription',
          input: {
            type: 'object',
            additionalProperties: false,
            properties: {
              sessionId: {
                type: 'string',
              },
              offset: {
                type: 'integer',
                format: 'uint64',
                minimum: 0,
                maximum: 9007199254740991,
                default: 0,
              },
            },
            required: ['sessionId'],
            title: 'TerminalList',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
          },
          output: {
            oneOf: [
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  value: {
                    $ref: '#/$defs/TerminalPage',
                  },
                  kind: {
                    type: 'string',
                    const: 'completed',
                  },
                },
                required: ['kind', 'value'],
              },
              {
                type: 'object',
                additionalProperties: false,
                properties: {
                  message: {
                    type: 'string',
                  },
                  kind: {
                    type: 'string',
                    const: 'failed',
                  },
                },
                required: ['kind', 'message'],
              },
            ],
            title: 'TerminalOutcome_TerminalPage',
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            $defs: {
              TerminalPage: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  resources: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/ResourceUpdate',
                    },
                  },
                  revision: {
                    type: 'string',
                  },
                  total: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  nextOffset: {
                    type: ['integer', 'null'],
                    format: 'uint64',
                    minimum: 0,
                  },
                },
                required: ['resources', 'revision', 'total'],
              },
              ResourceUpdate: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  sessionId: {
                    type: 'string',
                  },
                  ownership: {
                    $ref: '#/$defs/Ownership',
                  },
                  sourceTurnId: {
                    type: 'string',
                  },
                  sourceToolCallId: {
                    type: 'string',
                  },
                  result: {
                    $ref: '#/$defs/ShellSnapshot',
                  },
                },
                required: ['sessionId', 'ownership', 'sourceTurnId', 'sourceToolCallId', 'result'],
              },
              Ownership: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'local',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sourceSessionId: {
                        type: 'string',
                      },
                      ownerSessionId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'source_owned',
                      },
                    },
                    required: ['kind', 'sourceSessionId', 'ownerSessionId'],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      sourceSessionId: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'source_unavailable',
                      },
                    },
                    required: ['kind', 'sourceSessionId'],
                  },
                ],
              },
              ShellSnapshot: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  kind: {
                    $ref: '#/$defs/SnapshotKind',
                  },
                  ref: {
                    type: 'string',
                  },
                  mode: {
                    $ref: '#/$defs/ShellMode',
                  },
                  status: {
                    $ref: '#/$defs/ShellStatus',
                  },
                  cwd: {
                    type: 'string',
                  },
                  cmd: {
                    type: 'string',
                  },
                  startedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  updatedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  completedAt: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  timeoutMs: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  exitCode: {
                    type: 'integer',
                    format: 'int64',
                  },
                  failureMessage: {
                    type: 'string',
                  },
                  revision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  output: {
                    $ref: '#/$defs/ShellOutput',
                  },
                },
                required: [
                  'kind',
                  'ref',
                  'mode',
                  'status',
                  'cwd',
                  'cmd',
                  'startedAt',
                  'updatedAt',
                  'revision',
                ],
              },
              SnapshotKind: {
                type: 'string',
                enum: ['shell_run'],
              },
              ShellMode: {
                type: 'string',
                enum: ['pipes', 'pty'],
              },
              ShellStatus: {
                type: 'string',
                enum: [
                  'starting',
                  'running',
                  'completed',
                  'failed',
                  'timed_out',
                  'cancelled',
                  'orphaned',
                ],
              },
              ShellOutput: {
                oneOf: [
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      stdout: {
                        type: 'string',
                      },
                      stderr: {
                        type: 'string',
                      },
                      latestStream: {
                        $ref: '#/$defs/PipeStream',
                      },
                      stdoutTruncated: {
                        type: 'boolean',
                      },
                      stderrTruncated: {
                        type: 'boolean',
                      },
                      redacted: {
                        type: 'boolean',
                      },
                      mode: {
                        type: 'string',
                        const: 'pipes',
                      },
                    },
                    required: [
                      'mode',
                      'stdout',
                      'stderr',
                      'stdoutTruncated',
                      'stderrTruncated',
                      'redacted',
                    ],
                  },
                  {
                    type: 'object',
                    additionalProperties: false,
                    properties: {
                      screen: {
                        type: 'string',
                      },
                      scrollback: {
                        type: 'string',
                      },
                      lastAlternateScreen: {
                        type: 'string',
                      },
                      cols: {
                        type: 'integer',
                        format: 'uint16',
                        minimum: 0,
                        maximum: 65535,
                      },
                      rows: {
                        type: 'integer',
                        format: 'uint16',
                        minimum: 0,
                        maximum: 65535,
                      },
                      cursor: {
                        $ref: '#/$defs/TerminalCursor',
                      },
                      alternateScreen: {
                        type: 'boolean',
                      },
                      truncated: {
                        type: 'boolean',
                      },
                      redacted: {
                        type: 'boolean',
                      },
                      mode: {
                        type: 'string',
                        const: 'pty',
                      },
                    },
                    required: [
                      'mode',
                      'screen',
                      'scrollback',
                      'cols',
                      'rows',
                      'cursor',
                      'alternateScreen',
                      'truncated',
                      'redacted',
                    ],
                  },
                ],
              },
              PipeStream: {
                type: 'string',
                enum: ['stdout', 'stderr'],
              },
              TerminalCursor: {
                type: 'object',
                additionalProperties: false,
                properties: {
                  x: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  y: {
                    type: 'integer',
                    format: 'uint16',
                    minimum: 0,
                    maximum: 65535,
                  },
                  visible: {
                    type: 'boolean',
                  },
                },
                required: ['x', 'y', 'visible'],
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
                  editable: {
                    description:
                      'Editing controls implemented by this Settings service for the provider.',
                    type: 'boolean',
                  },
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
                required: [
                  'editable',
                  'basis',
                  'slug',
                  'name',
                  'provider',
                  'enabled',
                  'enabledModelCount',
                ],
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
                  query: {
                    type: 'string',
                    maxLength: 256,
                    default: '',
                  },
                  kind: {
                    type: 'string',
                    const: 'start',
                  },
                },
                additionalProperties: false,
                required: ['kind'],
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
                  query: {
                    type: 'string',
                    maxLength: 256,
                  },
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
                required: ['query', 'connection', 'model'],
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
                  defaultModel: {
                    $ref: '#/$defs/SettingsDefaultModel',
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
                required: ['kind', 'revision', 'choices', 'defaultModel', 'next'],
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
                  thinkingLevels: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/ThinkingLevel',
                    },
                    maxItems: 8,
                  },
                  defaultThinkingLevel: {
                    anyOf: [
                      {
                        $ref: '#/$defs/ThinkingLevel',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                },
                additionalProperties: false,
                required: [
                  'selection',
                  'connectionName',
                  'label',
                  'thinkingLevels',
                  'defaultThinkingLevel',
                ],
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
              ThinkingLevel: {
                type: 'string',
                enum: ['off', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max', 'ultra'],
              },
              SettingsDefaultModel: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'none',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                  {
                    type: 'object',
                    properties: {
                      target: {
                        $ref: '#/$defs/ConnectionTarget',
                      },
                      kind: {
                        type: 'string',
                        const: 'unavailable',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'target'],
                  },
                  {
                    type: 'object',
                    properties: {
                      choice: {
                        $ref: '#/$defs/SettingsModelChoice',
                      },
                      kind: {
                        type: 'string',
                        const: 'ready',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'choice'],
                  },
                ],
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
              SettingsModelCursor: {
                type: 'object',
                properties: {
                  query: {
                    type: 'string',
                    maxLength: 256,
                  },
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
                required: ['query', 'connection', 'model'],
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
                  recovery: true,
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
                  output: {
                    $ref: '#/$defs/CaptureOutput',
                  },
                },
                additionalProperties: false,
                required: ['input', 'output'],
              },
              CaptureOutput: {
                description:
                  'Resource captures enter checked service slots; continuations remain in the\noriginal presentation owner and only decide admission of query feedback.',
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      resource: {
                        $ref: '#/$defs/ResourceType',
                      },
                      kind: {
                        type: 'string',
                        const: 'resource',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'resource'],
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'continuation',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                ],
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
  deploymentManager: {
    name: 'maka.application.DeploymentManager',
    version: 1,
    transport: 'remote',
    interface: {
      kind: 'service',
      methods: {
        install: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'PackageInstall',
            type: 'object',
            properties: {
              expectedRevision: {
                type: 'integer',
                format: 'uint64',
                minimum: 1,
                maximum: 9007199254740990,
              },
            },
            additionalProperties: false,
            required: ['expectedRevision'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'ManagementResultForPackageInstalled',
            description:
              "A deterministic refusal is a recorded business result. Uncertain SQL/worker\nfailures remain in the original operation journal's Unknown outcome.",
            oneOf: [
              {
                type: 'object',
                properties: {
                  value: {
                    $ref: '#/$defs/PackageInstalled',
                  },
                  status: {
                    type: 'string',
                    const: 'committed',
                  },
                },
                additionalProperties: false,
                required: ['status', 'value'],
              },
              {
                type: 'object',
                properties: {
                  currentRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  status: {
                    type: 'string',
                    const: 'conflict',
                  },
                },
                additionalProperties: false,
                required: ['status', 'currentRevision'],
              },
              {
                type: 'object',
                properties: {
                  reason: {
                    $ref: '#/$defs/ManagementRejection',
                  },
                  diagnostic: {
                    type: ['string', 'null'],
                    maxLength: 512,
                  },
                  status: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                additionalProperties: false,
                required: ['status', 'reason'],
              },
            ],
            $defs: {
              PackageInstalled: {
                type: 'object',
                properties: {
                  source: {
                    $ref: '#/$defs/DeploymentRef',
                  },
                  package: {
                    $ref: '#/$defs/PackageRef',
                  },
                },
                additionalProperties: false,
                required: ['source', 'package'],
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
              PackageRef: {
                type: 'object',
                properties: {
                  id: {
                    type: 'string',
                  },
                  revision: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['id', 'revision'],
              },
              ManagementRejection: {
                type: 'string',
                enum: [
                  'invalid_bundle',
                  'invalid_document',
                  'invalid_source',
                  'package_unavailable',
                  'execution_environment',
                  'instance_unavailable',
                  'parent_disabled',
                ],
              },
            },
          },
          resources: {
            package: {
              name: 'maka.bytes',
              version: 1,
            },
          },
        },
        instances: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'DeploymentInstances',
            description:
              'A fixed committed source for continuation; no runtime or permission claim.',
            oneOf: [
              {
                type: 'object',
                properties: {
                  kind: {
                    type: 'string',
                    const: 'start',
                  },
                },
                additionalProperties: false,
                required: ['kind'],
              },
              {
                type: 'object',
                properties: {
                  source: {
                    $ref: '#/$defs/DeploymentRef',
                  },
                  after: {
                    anyOf: [
                      {
                        $ref: '#/$defs/InstanceId',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  kind: {
                    type: 'string',
                    const: 'continue',
                  },
                },
                additionalProperties: false,
                required: ['kind', 'source'],
              },
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
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'ManagedInstancesPage',
            type: 'object',
            properties: {
              source: {
                $ref: '#/$defs/DeploymentRef',
              },
              instances: {
                type: 'array',
                items: {
                  $ref: '#/$defs/ManagedInstance',
                },
                maxItems: 8,
              },
              next: {
                anyOf: [
                  {
                    $ref: '#/$defs/InstanceId',
                  },
                  {
                    type: 'null',
                  },
                ],
              },
            },
            additionalProperties: false,
            required: ['source', 'instances'],
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
              ManagedInstance: {
                type: 'object',
                properties: {
                  instance: {
                    $ref: '#/$defs/InstanceId',
                  },
                  path: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                  package: {
                    $ref: '#/$defs/PackageRef',
                  },
                  structural: {
                    type: 'boolean',
                  },
                  disabled: {
                    type: 'boolean',
                  },
                  parentDisabled: {
                    type: 'boolean',
                  },
                },
                additionalProperties: false,
                required: [
                  'instance',
                  'path',
                  'package',
                  'structural',
                  'disabled',
                  'parentDisabled',
                ],
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
              PackageRef: {
                type: 'object',
                properties: {
                  id: {
                    type: 'string',
                  },
                  revision: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['id', 'revision'],
              },
            },
          },
          resources: {},
        },
        read: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'DeploymentRead',
            type: 'object',
            additionalProperties: false,
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'ManagedDeployment',
            description:
              'Committed desired source and available immutable packages, not runtime readiness.',
            type: 'object',
            properties: {
              source: {
                $ref: '#/$defs/Deployment',
              },
              packages: {
                type: 'array',
                items: {
                  $ref: '#/$defs/PackageRef',
                },
              },
            },
            additionalProperties: false,
            required: ['source', 'packages'],
            $defs: {
              Deployment: {
                type: 'object',
                properties: {
                  reference: {
                    $ref: '#/$defs/DeploymentRef',
                  },
                  input: {
                    $ref: '#/$defs/CompositionInput',
                  },
                  remoteExports: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/RemoteObservation',
                    },
                  },
                },
                additionalProperties: false,
                required: ['reference', 'input', 'remoteExports'],
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
              CompositionInput: {
                description:
                  'Durable deployment input. Plans are derived and never persisted as overrides.',
                type: 'object',
                properties: {
                  environment: {
                    $ref: '#/$defs/Environment',
                  },
                  instances: {
                    type: 'object',
                    additionalProperties: {
                      $ref: '#/$defs/CompositionNode',
                    },
                  },
                  launch: {
                    anyOf: [
                      {
                        $ref: '#/$defs/Target',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  recovery: {
                    description:
                      'Explicit ordinary fallback. Selection grants no extra capabilities.',
                    anyOf: [
                      {
                        $ref: '#/$defs/Target',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                },
                additionalProperties: false,
                required: ['environment', 'instances'],
              },
              Environment: {
                type: 'object',
                properties: {
                  location: {
                    $ref: '#/$defs/ExecutionLocation',
                  },
                  presentation: {
                    anyOf: [
                      {
                        $ref: '#/$defs/PresentationTarget',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                },
                additionalProperties: false,
                required: ['location'],
              },
              ExecutionLocation: {
                type: 'string',
                enum: ['host', 'ui'],
              },
              PresentationTarget: {
                type: 'string',
                enum: ['tui', 'desktop'],
              },
              CompositionNode: {
                type: 'object',
                properties: {
                  key: {
                    type: 'string',
                    format: 'uuid',
                  },
                  package: {
                    $ref: '#/$defs/PackageSelection',
                  },
                  usePackage: {
                    $ref: '#/$defs/PackageUse',
                  },
                  configuration: {
                    $ref: '#/$defs/Configuration',
                    default: {
                      values: {},
                      parameters: {},
                    },
                  },
                  parameters: {
                    $ref: '#/$defs/Configuration',
                    default: {
                      values: {},
                      parameters: {},
                    },
                  },
                  bindings: {
                    type: 'object',
                    additionalProperties: {
                      type: 'array',
                      items: {
                        $ref: '#/$defs/Target',
                      },
                    },
                    default: {},
                  },
                  disabled: {
                    type: 'boolean',
                    default: false,
                  },
                  overrides: {
                    type: 'array',
                    items: {
                      $ref: '#/$defs/Override',
                    },
                    default: [],
                  },
                },
                additionalProperties: false,
                required: ['key', 'package', 'usePackage'],
              },
              PackageSelection: {
                description:
                  'Templates can select an entry in their own immutable package without a\nself-referential content hash. Deployment roots must select a pinned revision.',
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      package: {
                        $ref: '#/$defs/PackageRef',
                      },
                      source: {
                        type: 'string',
                        const: 'pinned',
                      },
                    },
                    additionalProperties: false,
                    required: ['source', 'package'],
                  },
                  {
                    type: 'object',
                    properties: {
                      source: {
                        type: 'string',
                        const: 'current',
                      },
                    },
                    required: ['source'],
                    additionalProperties: false,
                  },
                ],
              },
              PackageRef: {
                type: 'object',
                properties: {
                  id: {
                    type: 'string',
                  },
                  revision: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['id', 'revision'],
              },
              PackageUse: {
                type: 'string',
                enum: ['entry', 'composition'],
              },
              Configuration: {
                type: 'object',
                properties: {
                  values: {
                    type: 'object',
                    additionalProperties: true,
                    default: {},
                  },
                  parameters: {
                    description:
                      'Configuration field -> parameter name of the containing template.',
                    type: 'object',
                    additionalProperties: {
                      type: 'string',
                    },
                    default: {},
                  },
                },
                additionalProperties: false,
              },
              Target: {
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      path: {
                        type: 'array',
                        items: {
                          type: 'string',
                        },
                      },
                      export: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'instance',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'path', 'export'],
                  },
                  {
                    type: 'object',
                    properties: {
                      name: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'import',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'name'],
                  },
                  {
                    type: 'object',
                    properties: {
                      deployment: {
                        type: 'string',
                      },
                      instance: {
                        $ref: '#/$defs/InstanceId',
                      },
                      export: {
                        type: 'string',
                      },
                      kind: {
                        type: 'string',
                        const: 'remote',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'deployment', 'instance', 'export'],
                  },
                ],
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
              Override: {
                description:
                  'An override belongs to its containing structural instance. Missing paths fail;\nremoval/upgrade never guesses how to discard independent user intent.',
                type: 'object',
                properties: {
                  path: {
                    type: 'array',
                    items: {
                      type: 'string',
                    },
                  },
                  package: {
                    anyOf: [
                      {
                        $ref: '#/$defs/PackageRef',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  usePackage: {
                    anyOf: [
                      {
                        $ref: '#/$defs/PackageUse',
                      },
                      {
                        type: 'null',
                      },
                    ],
                  },
                  parameters: {
                    type: 'object',
                    additionalProperties: true,
                    default: {},
                  },
                  configuration: {
                    type: 'object',
                    additionalProperties: true,
                    default: {},
                  },
                  bindings: {
                    type: 'object',
                    additionalProperties: {
                      type: 'array',
                      items: {
                        $ref: '#/$defs/Target',
                      },
                    },
                    default: {},
                  },
                  disabled: {
                    type: ['boolean', 'null'],
                  },
                },
                additionalProperties: false,
                required: ['path'],
              },
              RemoteObservation: {
                description:
                  'A static compatibility observation, never a remote registration or grant.',
                type: 'object',
                properties: {
                  capability: {
                    $ref: '#/$defs/RemoteCapability',
                  },
                  declaration: {
                    $ref: '#/$defs/Export',
                  },
                },
                additionalProperties: false,
                required: ['capability', 'declaration'],
              },
              RemoteCapability: {
                type: 'object',
                properties: {
                  deployment: {
                    type: 'string',
                  },
                  instance: {
                    $ref: '#/$defs/InstanceId',
                  },
                  export: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
                required: ['deployment', 'instance', 'export'],
              },
              Export: {
                type: 'object',
                properties: {
                  contract: {
                    $ref: '#/$defs/Contract',
                  },
                  availability: {
                    $ref: '#/$defs/Availability',
                  },
                },
                additionalProperties: false,
                required: ['contract', 'availability'],
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
                  output: {
                    $ref: '#/$defs/CaptureOutput',
                  },
                },
                additionalProperties: false,
                required: ['input', 'output'],
              },
              CaptureOutput: {
                description:
                  'Resource captures enter checked service slots; continuations remain in the\noriginal presentation owner and only decide admission of query feedback.',
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      resource: {
                        $ref: '#/$defs/ResourceType',
                      },
                      kind: {
                        type: 'string',
                        const: 'resource',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'resource'],
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'continuation',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                ],
              },
              Availability: {
                type: 'string',
                enum: ['required', 'optional'],
              },
            },
          },
          resources: {},
        },
        readDocument: {
          mode: 'query',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'DeploymentDocumentRead',
            type: 'object',
            properties: {
              source: {
                $ref: '#/$defs/DeploymentRef',
              },
              offset: {
                type: 'integer',
                format: 'uint',
                minimum: 0,
                maximum: 1048576,
              },
            },
            additionalProperties: false,
            required: ['source', 'offset'],
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
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'DeploymentDocumentChunk',
            type: 'object',
            properties: {
              source: {
                $ref: '#/$defs/DeploymentRef',
              },
              offset: {
                type: 'integer',
                format: 'uint',
                minimum: 0,
              },
              totalBytes: {
                type: 'integer',
                format: 'uint',
                minimum: 0,
              },
              text: {
                type: 'string',
                maxLength: 65536,
              },
              next: {
                type: ['integer', 'null'],
                format: 'uint',
                minimum: 0,
              },
            },
            additionalProperties: false,
            required: ['source', 'offset', 'totalBytes', 'text'],
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
            },
          },
          resources: {},
        },
        replace: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'DeploymentReplace',
            type: 'object',
            properties: {
              expectedRevision: {
                type: 'integer',
                format: 'uint64',
                minimum: 1,
                maximum: 9007199254740990,
              },
            },
            additionalProperties: false,
            required: ['expectedRevision'],
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'ManagementResultForDeploymentRef',
            description:
              "A deterministic refusal is a recorded business result. Uncertain SQL/worker\nfailures remain in the original operation journal's Unknown outcome.",
            oneOf: [
              {
                type: 'object',
                properties: {
                  value: {
                    $ref: '#/$defs/DeploymentRef',
                  },
                  status: {
                    type: 'string',
                    const: 'committed',
                  },
                },
                additionalProperties: false,
                required: ['status', 'value'],
              },
              {
                type: 'object',
                properties: {
                  currentRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  status: {
                    type: 'string',
                    const: 'conflict',
                  },
                },
                additionalProperties: false,
                required: ['status', 'currentRevision'],
              },
              {
                type: 'object',
                properties: {
                  reason: {
                    $ref: '#/$defs/ManagementRejection',
                  },
                  diagnostic: {
                    type: ['string', 'null'],
                    maxLength: 512,
                  },
                  status: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                additionalProperties: false,
                required: ['status', 'reason'],
              },
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
              ManagementRejection: {
                type: 'string',
                enum: [
                  'invalid_bundle',
                  'invalid_document',
                  'invalid_source',
                  'package_unavailable',
                  'execution_environment',
                  'instance_unavailable',
                  'parent_disabled',
                ],
              },
            },
          },
          resources: {
            document: {
              name: 'maka.text.utf8',
              version: 1,
            },
          },
        },
        setEnabled: {
          mode: 'mutation',
          input: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'InstanceSetEnabled',
            type: 'object',
            properties: {
              expectedRevision: {
                type: 'integer',
                format: 'uint64',
                minimum: 1,
                maximum: 9007199254740990,
              },
              instance: {
                $ref: '#/$defs/InstanceId',
              },
              enabled: {
                type: 'boolean',
              },
            },
            additionalProperties: false,
            required: ['expectedRevision', 'instance', 'enabled'],
            $defs: {
              InstanceId: {
                description:
                  'Hierarchical, stable source keys. Display aliases never become data identities.',
                type: 'array',
                items: {
                  type: 'string',
                  format: 'uuid',
                },
              },
            },
          },
          output: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            title: 'ManagementResultForDeploymentRef',
            description:
              "A deterministic refusal is a recorded business result. Uncertain SQL/worker\nfailures remain in the original operation journal's Unknown outcome.",
            oneOf: [
              {
                type: 'object',
                properties: {
                  value: {
                    $ref: '#/$defs/DeploymentRef',
                  },
                  status: {
                    type: 'string',
                    const: 'committed',
                  },
                },
                additionalProperties: false,
                required: ['status', 'value'],
              },
              {
                type: 'object',
                properties: {
                  currentRevision: {
                    type: 'integer',
                    format: 'uint64',
                    minimum: 0,
                  },
                  status: {
                    type: 'string',
                    const: 'conflict',
                  },
                },
                additionalProperties: false,
                required: ['status', 'currentRevision'],
              },
              {
                type: 'object',
                properties: {
                  reason: {
                    $ref: '#/$defs/ManagementRejection',
                  },
                  diagnostic: {
                    type: ['string', 'null'],
                    maxLength: 512,
                  },
                  status: {
                    type: 'string',
                    const: 'rejected',
                  },
                },
                additionalProperties: false,
                required: ['status', 'reason'],
              },
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
              ManagementRejection: {
                type: 'string',
                enum: [
                  'invalid_bundle',
                  'invalid_document',
                  'invalid_source',
                  'package_unavailable',
                  'execution_environment',
                  'instance_unavailable',
                  'parent_disabled',
                ],
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
                  output: {
                    $ref: '#/$defs/CaptureOutput',
                  },
                },
                additionalProperties: false,
                required: ['input', 'output'],
              },
              CaptureOutput: {
                description:
                  'Resource captures enter checked service slots; continuations remain in the\noriginal presentation owner and only decide admission of query feedback.',
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      resource: {
                        $ref: '#/$defs/ResourceType',
                      },
                      kind: {
                        type: 'string',
                        const: 'resource',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'resource'],
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'continuation',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                ],
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
                  output: {
                    $ref: '#/$defs/CaptureOutput',
                  },
                },
                additionalProperties: false,
                required: ['input', 'output'],
              },
              CaptureOutput: {
                description:
                  'Resource captures enter checked service slots; continuations remain in the\noriginal presentation owner and only decide admission of query feedback.',
                oneOf: [
                  {
                    type: 'object',
                    properties: {
                      resource: {
                        $ref: '#/$defs/ResourceType',
                      },
                      kind: {
                        type: 'string',
                        const: 'resource',
                      },
                    },
                    additionalProperties: false,
                    required: ['kind', 'resource'],
                  },
                  {
                    type: 'object',
                    properties: {
                      kind: {
                        type: 'string',
                        const: 'continuation',
                      },
                    },
                    required: ['kind'],
                    additionalProperties: false,
                  },
                ],
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
