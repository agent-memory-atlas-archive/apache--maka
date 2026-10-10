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
export const configurations = {
  textFile: {
    $schema: 'https://json-schema.org/draft/2020-12/schema',
    title: 'TextFileConfiguration',
    type: 'object',
    properties: {
      path: {
        type: ['string', 'null'],
        minLength: 1,
        maxLength: 4096,
      },
    },
    additionalProperties: false,
  },
  morphMark: {
    $schema: 'https://json-schema.org/draft/2020-12/schema',
    title: 'MorphMarkConfiguration',
    type: 'object',
    properties: {
      shapes: {
        type: 'array',
        items: {
          $ref: '#/$defs/MorphShape',
        },
        minItems: 2,
        maxItems: 2,
      },
      color: {
        $ref: '#/$defs/TerminalColor',
      },
      period: {
        type: 'integer',
        format: 'uint32',
        minimum: 1000,
        maximum: 60000,
      },
    },
    additionalProperties: false,
    required: ['shapes', 'color', 'period'],
    $defs: {
      MorphShape: {
        type: 'object',
        properties: {
          paths: {
            type: 'array',
            items: {
              type: 'array',
              items: {
                type: 'array',
                items: {
                  type: 'number',
                  format: 'float',
                },
                minItems: 2,
                maxItems: 2,
              },
            },
            minItems: 1,
            maxItems: 32,
          },
          radius: {
            type: 'number',
            format: 'float',
            minimum: 0.01,
            maximum: 0.2,
          },
          miters: {
            type: 'array',
            items: {
              type: 'integer',
              format: 'uint',
              minimum: 0,
            },
            maxItems: 32,
          },
          columns: {
            type: 'integer',
            format: 'uint',
            minimum: 32,
            maximum: 256,
          },
        },
        additionalProperties: false,
        required: ['paths', 'radius', 'miters', 'columns'],
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
    },
  },
  agent: {
    $schema: 'https://json-schema.org/draft/2020-12/schema',
    title: 'AgentConfiguration',
    type: 'object',
    properties: {
      permissions: {
        description: 'Initial policy within the selected workspace capability.',
        $ref: '#/$defs/AgentWorkspacePermissions',
      },
      workspaceRoots: {
        description:
          'Installation-published directories, captured without creating workspace markers.',
        type: 'array',
        items: {
          $ref: '#/$defs/DirectoryRootSpec',
        },
        minItems: 1,
        maxItems: 8,
      },
      maxActiveRuns: {
        type: 'integer',
        format: 'uint',
        minimum: 1,
        maximum: 16,
      },
      maxSteps: {
        type: 'integer',
        format: 'uint',
        minimum: 1,
        maximum: 16,
      },
      tools: {
        description: 'Only explicitly selected, frozen application service methods become tools.',
        type: 'array',
        items: {
          $ref: '#/$defs/AgentServiceTool',
        },
        maxItems: 128,
      },
    },
    additionalProperties: false,
    required: ['permissions', 'workspaceRoots', 'maxActiveRuns', 'maxSteps', 'tools'],
    $defs: {
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
      DirectoryRootSpec: {
        type: 'object',
        properties: {
          label: {
            type: 'string',
          },
          path: {
            type: 'string',
          },
        },
        additionalProperties: false,
        required: ['label', 'path'],
      },
      AgentServiceTool: {
        type: 'object',
        properties: {
          input: {
            description:
              'Explicit input projection. Skill tools receive the actual Agent tool names,\noutside model-controlled arguments; this is compatibility data, not authority.',
            $ref: '#/$defs/AgentToolInput',
            default: 'json',
          },
          presentation: {
            description:
              'Explicit versioned renderer key, frozen with the call; never inferred from its name.',
            type: ['string', 'null'],
            minLength: 1,
            maxLength: 128,
            default: null,
          },
          title: {
            description:
              'Display only; the selected name, contract and binding still identify execution.',
            anyOf: [
              {
                $ref: '#/$defs/DisplayText',
              },
              {
                type: 'null',
              },
            ],
            default: null,
          },
          result: {
            description:
              'Explicit model projection; JSON data never implicitly grants image access.',
            $ref: '#/$defs/AgentToolResult',
          },
          confirmation: {
            description:
              'Additional user consent for this exact call; service grants remain independent.',
            $ref: '#/$defs/AgentToolConfirmation',
          },
          name: {
            type: 'string',
            minLength: 1,
            maxLength: 128,
          },
          description: {
            type: 'string',
            minLength: 1,
            maxLength: 4096,
          },
          import: {
            type: 'string',
            minLength: 1,
            maxLength: 256,
          },
          method: {
            type: 'string',
            minLength: 1,
            maxLength: 256,
          },
        },
        additionalProperties: false,
        required: ['result', 'confirmation', 'name', 'description', 'import', 'method'],
      },
      AgentToolInput: {
        type: 'string',
        enum: ['json', 'skill'],
      },
      DisplayText: {
        description: 'Presentation text is distinct from the stable identity used for navigation.',
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
      AgentToolResult: {
        type: 'string',
        enum: ['json', 'image'],
      },
      AgentToolConfirmation: {
        type: 'string',
        enum: ['required', 'preapproved'],
      },
    },
  },
  skillLibrary: {
    type: 'object',
    additionalProperties: false,
    properties: {
      manage: {
        type: 'boolean',
        description: "Enable installation into this instance's own private collection.",
        default: false,
      },
      preferences: {
        type: 'boolean',
        description: 'Allow edits to enabled/pinned preferences, independently of file management.',
        default: false,
      },
      sources: {
        type: 'array',
        items: {
          $ref: '#/$defs/SkillLibrarySource',
        },
        maxItems: 8,
      },
    },
    required: ['sources'],
    description: 'Installation-selected skill collections. No ambient home or workspace scan.',
    title: 'SkillLibraryConfiguration',
    $schema: 'https://json-schema.org/draft/2020-12/schema',
    $defs: {
      SkillLibrarySource: {
        type: 'object',
        additionalProperties: false,
        properties: {
          id: {
            type: 'string',
            pattern: '^[a-z0-9][a-z0-9_-]{0,63}$',
          },
          label: {
            type: 'string',
            minLength: 1,
            maxLength: 128,
          },
          path: {
            type: 'string',
            description: 'Absolute directory containing skill directories. Captured at activation.',
            minLength: 1,
            maxLength: 4096,
          },
        },
        required: ['id', 'label', 'path'],
      },
    },
  },
  workspaceProcesses: {
    type: 'object',
    additionalProperties: false,
    properties: {
      filesystem: {
        $ref: '#/$defs/Policy',
      },
      network: {
        $ref: '#/$defs/Network',
      },
      helper: {
        type: ['string', 'null'],
        description: 'Trusted CLI supporting the original platform sandbox bootstrap.',
      },
      sessionOwner: {
        anyOf: [
          {
            $ref: '#/$defs/InstanceId',
          },
          {
            type: 'null',
          },
        ],
        description:
          'Trusted Agent Session owner in this Root/deployment. This selects a data\nnamespace only; Terminal endpoints still require their own explicit grants.',
        default: null,
      },
    },
    required: ['filesystem', 'network'],
    description: 'Immutable installation ceiling selected by trusted native composition.',
    title: 'WorkspaceProcessConfiguration',
    $schema: 'https://json-schema.org/draft/2020-12/schema',
    $defs: {
      Policy: {
        type: 'object',
        additionalProperties: false,
        properties: {
          default: {
            $ref: '#/$defs/Access',
          },
          rules: {
            type: 'array',
            items: {
              $ref: '#/$defs/Rule',
            },
            maxItems: 512,
          },
          denyGlobs: {
            type: 'array',
            items: {
              type: 'string',
            },
            maxItems: 128,
          },
        },
        required: ['default', 'rules', 'denyGlobs'],
        description:
          'Longest path wins; at the same path exact wins over subtree, then deny wins\nover write over read. Deny globs are absolute and cannot be reopened by paths.\nMatching directories deny their descendants.\nHost file operations match at access time. Linux and Windows subprocesses\nsnapshot existing glob matches during preparation; macOS enforces them live.\nEmpty restricted policy denies all access, rather than falling back to defaults.',
      },
      Access: {
        type: 'string',
        enum: ['read', 'write', 'deny'],
        description: 'Equal-path conflict priority is different from authority breadth: deny wins.',
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
      InstanceId: {
        type: 'array',
        items: {
          type: 'string',
          format: 'uuid',
        },
        description:
          'Hierarchical, stable source keys. Display aliases never become data identities.',
      },
    },
  },
} as const;
