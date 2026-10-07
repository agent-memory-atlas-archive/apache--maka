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

import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

const { values } = parseArgs({
  options: {
    configuration: { type: 'string' },
    output: { type: 'string' },
    maka: { type: 'string', default: 'maka' },
    product: { type: 'string', default: 'agent' },
  },
});
if (!values.configuration || !values.output)
  throw new Error(
    'Required: --configuration FILE --output DIRECTORY [--maka EXECUTABLE] [--product agent|studio]',
  );
if (!['agent', 'studio'].includes(values.product)) throw new Error('Unknown product');
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const { composition, studioComposition, packageNames } = await import(
  pathToFileURL(resolve(root, 'dist/authoring/agent-product/src/composition.mjs'))
);
const configuration = JSON.parse(await readFile(values.configuration, 'utf8'));
const packages = {};
const paths = packageNames.map((name) => resolve(root, 'dist/packages', name));
const run = (args) =>
  execFileSync(values.maka, ['application', ...args], {
    encoding: 'utf8',
    maxBuffer: 4 * 1024 * 1024,
  });
for (const [index, name] of packageNames.entries())
  packages[name] = JSON.parse(run(['inspect-package', '--package', paths[index]]));
const result = (values.product === 'studio' ? studioComposition : composition)({
  ...configuration,
  packages,
});
const output = resolve(values.output);
await mkdir(output, { recursive: true });
const write = async (name, value) => {
  const path = resolve(output, name + '.json');
  await writeFile(path, JSON.stringify(value, null, 2) + '\n');
  return path;
};
const host = await write('host', result.host);
const management = await write('management-host', result.managementHost);
const ui = await write('ui', result.ui);
const remote = await write('remote-exports', result.remoteExports);
await write('connections', result.connections);
await write('packages', packages);
const arguments_ = paths.flatMap((path) => ['--package', path]);
const check = (args) =>
  execFileSync(values.maka, ['application', 'check', ...args], {
    // The checker owns its derived plan. Authoring only needs the exit status,
    // not a second, buffered copy of every expanded contract and declaration.
    stdio: ['ignore', 'ignore', 'pipe'],
    maxBuffer: 4 * 1024 * 1024,
  });
check(['--input', host, ...arguments_]);
check(['--input', management, ...arguments_]);
check(['--input', ui, '--remote-exports', remote, ...arguments_]);
console.log(`Checked Host and ${configuration.presentation} compositions written to ${output}`);
