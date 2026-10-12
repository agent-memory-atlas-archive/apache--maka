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

import { contracts } from '../../../packages/plugin-sdk/src/contracts.ts';
export default function ({ require }) {
  const Ajv = require('ajv/dist/2020.js').default;
  const addFormats = require('ajv-formats').default;
  const standalone = require('ajv/dist/standalone/index.js').default;
  const validator = new Ajv({
    code: { source: true, esm: true },
    strict: false,
    strictNumbers: true,
    formats: { uint64: true },
  });
  addFormats(validator);
  for (const method of ['create', 'update'])
    validator.addSchema(contracts.settings.interface.methods[method].input, method);
  return {
    'maka:settings-validation': standalone(validator, { create: 'create', update: 'update' }),
  };
}
