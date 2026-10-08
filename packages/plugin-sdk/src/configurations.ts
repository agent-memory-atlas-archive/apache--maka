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
    required: ['workspaceRoots', 'maxActiveRuns', 'maxSteps', 'tools'],
    $defs: {
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
        required: ['name', 'description', 'import', 'method'],
      },
    },
  },
} as const;
