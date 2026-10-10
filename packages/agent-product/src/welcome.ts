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

// Original welcome geometry (d8cff2b). Product paths stay outside the renderer.
import type { MorphMarkConfiguration } from '../../plugin-sdk/src/index.js';
const point = (x: number, y: number): [number, number] => [(x - 512) / 400, (y - 503.5) / 400];
const arc = (x: number, y: number, rx: number, ry: number, half: boolean): [number, number][] => {
  const steps = half ? 12 : 24;
  return Array.from({ length: steps + 1 }, (_, i) => {
    const angle = Math.PI + (i / steps) * (half ? Math.PI : Math.PI * 2);
    return [x + rx * Math.cos(angle), y + ry * Math.sin(angle)];
  });
};
export const welcomeMark: MorphMarkConfiguration = {
  period: 20000,
  color: 0x47a3e2,
  shapes: [
    {
      paths: [
        [point(359.1, 537), point(512, 274), point(664.9, 537)],
        [point(180, 845), point(359.1, 537), point(512, 800)],
        [point(844, 845), point(664.9, 537), point(512, 800)],
        [point(405, 162), point(619, 162)],
      ],
      radius: 35 / 400,
      miters: [1, 2],
      columns: 144,
    },
    {
      paths: [
        [
          [-1.6, 0.48],
          [-1.6, -0.28],
        ],
        [...arc(-1.4, -0.1, 0.2, 0.2, true), [-1.2, 0.48]],
        [...arc(-1.0, -0.1, 0.2, 0.2, true), [-0.8, 0.48]],
        arc(-0.3, 0.1, 0.24, 0.38, false),
        [
          [-0.06, -0.28],
          [-0.06, 0.48],
        ],
        [
          [0.26, -0.66],
          [0.26, 0.48],
        ],
        [
          [0.75, -0.28],
          [0.26, 0.1],
          [0.78, 0.48],
        ],
        arc(1.26, 0.1, 0.24, 0.38, false),
        [
          [1.5, -0.28],
          [1.5, 0.48],
        ],
      ],
      radius: 0.055,
      miters: [],
      columns: 224,
    },
  ],
};
