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
        required: ['confirmation', 'name', 'description', 'import', 'method'],
      },
      AgentToolConfirmation: {
        type: 'string',
        enum: ['required', 'preapproved'],
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
    },
  },
} as const;
