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

// Schemas and built-in contracts come from the actual Rust application ABI.
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { compile } from 'json-schema-to-typescript';

const mode = process.argv[2];
if (!['generate', 'check'].includes(mode) || process.argv.length !== 3)
  throw new Error('Usage: generate.mjs <generate|check>');
const root = fileURLToPath(new URL('../../../', import.meta.url));
const catalog = JSON.parse(
  execFileSync(
    'cargo',
    [
      'run',
      '--locked',
      '--quiet',
      '-p',
      'maka-cli',
      '--bin',
      'maka',
      '--',
      'application',
      'schema',
    ],
    {
      cwd: root,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'inherit'],
      maxBuffer: 4 * 1024 * 1024,
    },
  ),
);
const header = readFileSync(new URL('../src/index.ts', import.meta.url), 'utf8').split(
  '\n\n',
  1,
)[0];
const banner =
  header +
  '\n\n// Generated from Rust runtime schemas. Run npm run schema:generate -w @maka-agent/plugin-sdk.\n';
const outputs = new Map();
outputs.set(
  new URL('../src/generated/runtime.ts', import.meta.url),
  await compile(catalog.schema, catalog.schema.title, {
    bannerComment: banner.trimEnd(),
    unknownAny: true,
    strictIndexSignatures: true,
    format: false,
  }),
);
outputs.set(
  new URL('../src/contracts.ts', import.meta.url),
  banner +
    "import type { Contract } from './generated/runtime.js';\n\n" +
    'export const contracts = ' +
    JSON.stringify(catalog.contracts, null, 2) +
    ' satisfies Record<string, Contract>;\n',
);
for (const [url, source] of outputs) {
  const content = execFileSync(
    process.execPath,
    [
      fileURLToPath(new URL('../../../node_modules/@biomejs/biome/bin/biome', import.meta.url)),
      'format',
      '--stdin-file-path',
      fileURLToPath(url),
    ],
    { cwd: root, input: source, encoding: 'utf8', maxBuffer: 4 * 1024 * 1024 },
  );
  if (mode === 'generate') {
    mkdirSync(new URL('./', url), { recursive: true });
    writeFileSync(url, content);
  } else if (readFileSync(url, 'utf8') !== content) {
    throw new Error(`Runtime declarations are stale: ${fileURLToPath(url)}`);
  }
}
console.log(
  `${outputs.size} runtime declaration files ${mode === 'check' ? 'verified' : 'generated'}`,
);
