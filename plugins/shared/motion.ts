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

import type { TerminalAnimationFrame } from '../../packages/plugin-sdk/src/index.js';
export type ActivityAnimation = 'familiar' | 'spring' | 'orbit';
const familiar: [string, number][] = [
  ['o_o', 900],
  ['>_>', 350],
  ['o_o', 650],
  ['o_-', 120],
  ['o_o', 700],
  ['<_<', 350],
  ['o_o', 700],
  ['-_-', 100],
];
const spring: [string, number][] = [
  ['⣀⡀ ', 170],
  ['⣤  ', 65],
  ['⠰⠆ ', 90],
  [' ⠛ ', 105],
  [' ⠰⠆', 85],
  ['  ⣤', 65],
  [' ⢀⣀', 170],
  ['  ⣤', 65],
  [' ⠰⠆', 90],
  [' ⠛ ', 105],
  ['⠰⠆ ', 85],
  ['⣤  ', 65],
];
const orbit: [string, number][] = [
  ['⢁', 180],
  ['⡈', 180],
  ['⠔', 180],
  ['⠢', 180],
];
const springAscii: [string, number][] = [
  ['o  ', 150],
  [' o ', 150],
  ['  o', 150],
  [' o ', 150],
];
const orbitAscii: [string, number][] = [
  ['/', 180],
  ['-', 180],
  ['\\', 180],
  ['|', 180],
];
export function animation(
  kind: ActivityAnimation,
  ascii: boolean,
  enabled: boolean,
  prefix = '',
  suffix = '',
): TerminalAnimationFrame[] {
  const frames =
    kind === 'familiar'
      ? familiar
      : kind === 'spring'
        ? ascii
          ? springAscii
          : spring
        : ascii
          ? orbitAscii
          : orbit;
  return (enabled ? frames : frames.slice(0, 1)).map(([text, millis]) => ({
    text: prefix + text + suffix,
    millis,
  }));
}
