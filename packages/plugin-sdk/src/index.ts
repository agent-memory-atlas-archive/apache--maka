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

/** Application authoring types. Runtime admission remains in Rust. */
export type * from './generated/runtime.js';
export { contracts } from './contracts.js';

import type { ComponentInput, ComponentOutput, CapturePlan } from './generated/runtime.js';
import type { HostRequest, HostObservation } from './generated/runtime.js';

/** State is private to this component; external props and events still need their contract. */
export type Output<State = unknown> = Omit<ComponentOutput, 'frame'> & {
  frame: Omit<ComponentOutput['frame'], 'state'> & { state: State };
};

/** Synchronous declarative callbacks; no promises or native ownership in a frame. */
export interface Component<State = unknown, Props = unknown> {
  mount(props: Props): Output<State>;
  update(state: State, props: Props): Output<State>;
  event(state: State, input: ComponentInput, props: Props): Output<State>;
  capture?(state: State, props: Props, method: string, input: unknown): CapturePlan;
}

/** Provided by the actual invocation. Retaining it does not retain request resources. */
export interface RequestContext {
  invoke(request: HostRequest): Promise<unknown>;
}

export type HostService = Record<
  string,
  (input: unknown, observation: HostObservation, context: RequestContext) => unknown
>;
