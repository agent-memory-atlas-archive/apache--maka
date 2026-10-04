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
      model: {
        $ref: '#/$defs/AgentModelSelection',
      },
      provider: {
        description: 'Compiled API provider; never an endpoint, credential or mutable fallback.',
        type: 'string',
        minLength: 1,
        maxLength: 128,
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
    },
    additionalProperties: false,
    required: ['model', 'provider', 'maxActiveRuns', 'maxSteps'],
    $defs: {
      AgentModelSelection: {
        type: 'object',
        properties: {
          connectionId: {
            type: 'string',
            minLength: 1,
            maxLength: 128,
          },
          connectionSlug: {
            type: 'string',
            minLength: 1,
            maxLength: 128,
          },
          model: {
            type: 'string',
            minLength: 1,
            maxLength: 512,
          },
        },
        additionalProperties: false,
        required: ['connectionId', 'connectionSlug', 'model'],
      },
    },
  },
} as const;
