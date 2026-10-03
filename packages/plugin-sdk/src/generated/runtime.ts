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

export type RuntimeProtocol =
  | CompositionInput
  | Deployment
  | PackageDeclaration
  | PackageRef
  | Contract
  | Import
  | Export
  | UiExport
  | ValueBindings
  | ComponentInput
  | ComponentOutput
  | CapturePlan
  | ActionOutcome
  | CallerHistoryPage
  | CallerIntent
  | LabelProps
  | EditorProps
  | EditorObservation
  | EditorCapture
  | ReadText
  | TextChunk
  | HostActivation
  | HostRequest
  | HostObservation
  | OperationObservation
  | OperationReply
  | ResourceMetadata;
export type ExecutionLocation = 'host' | 'ui';
export type PresentationTarget = 'tui' | 'desktop';
/**
 * Templates can select an entry in their own immutable package without a
 * self-referential content hash. Deployment roots must select a pinned revision.
 */
export type PackageSelection =
  | {
      package: PackageRef;
      source: 'pinned';
    }
  | {
      source: 'current';
    };
export type PackageUse = 'entry' | 'composition';
export type Target =
  | {
      path: string[];
      export: string;
      kind: 'instance';
    }
  | {
      name: string;
      kind: 'import';
    }
  | {
      deployment: string;
      instance: InstanceId;
      export: string;
      kind: 'remote';
    };
/**
 * Hierarchical, stable source keys. Display aliases never become data identities.
 */
export type InstanceId = string[];
export type Transport = 'local' | 'remote';
export type Interface =
  | {
      methods: {
        [k: string]: Method | undefined;
      };
      kind: 'service';
    }
  | {
      props: unknown;
      events: {
        [k: string]: unknown | undefined;
      };
      captures: {
        [k: string]: Capture | undefined;
      };
      kind: 'component';
    }
  | {
      schema: unknown;
      kind: 'value';
    }
  | {
      resource: string;
      schema: unknown;
      kind: 'resource';
    };
export type MethodMode = 'query' | 'mutation' | 'subscription';
export type Availability = 'required' | 'optional';
export type Implementation =
  | {
      module: string;
      realm: Realm;
      kind: 'javascript';
    }
  | {
      implementation: string;
      kind: 'native';
    };
export type Realm = 'shared' | 'dedicated';
export type Cardinality =
  | {
      kind: 'one';
    }
  | {
      kind: 'optional';
    }
  | {
      min: number;
      max: number;
      kind: 'many';
    };
/**
 * Actual exports from a local JavaScript entry. Values are immutable for this
 * activation; component paths are captured before publication.
 */
export type UiExport =
  | {
      contract: Contract;
      component: string[];
      kind: 'component';
    }
  | {
      contract: Contract;
      value: unknown;
      kind: 'value';
    };
/**
 * Input origin is assigned by the application, never by plugin event payloads.
 */
export type ComponentInput =
  | {
      key: string;
      event: unknown;
      kind: 'raw';
    }
  | {
      key: string;
      event: unknown;
      kind: 'action';
    }
  | {
      key: string;
      child: string;
      name: string;
      event: unknown;
      kind: 'component';
    };
export type Surface =
  | {
      binding: string;
      title: string;
      width: number;
      height: number;
      kind: 'window';
    }
  | {
      binding: string;
      kind: 'terminal';
    };
export type UiNode =
  | {
      key: string;
      direction: Direction;
      gap: number;
      padding: number;
      background: number;
      children: UiNode[];
      kind: 'stack';
    }
  | {
      key: string;
      text: string;
      color: number;
      kind: 'text';
    }
  | {
      key: string;
      label: string;
      kind: 'button';
    }
  | {
      key: string;
      binding: string;
      member: number;
      props: unknown;
      on: {
        [k: string]: string | undefined;
      };
      kind: 'component';
    };
export type Direction = 'row' | 'column';
export type CapturePlan =
  | {
      source: CaptureSource;
      kind: 'forward';
    }
  | {
      kind: 'native';
    };
export type ActionOutcome =
  | {
      reason: string;
      status: 'not_accepted';
    }
  | {
      value: unknown;
      status: 'succeeded';
    }
  | {
      reason: string;
      status: 'failed';
    }
  | {
      reply: OperationReply;
      status: 'operation';
    };
export type RecoveryProtocol = 'journal_v1';
/**
 * A caller's receipt observation, never an execution grant. Unavailable makes
 * no claim about the journal's contents or physical cleanup.
 */
export type Receipt =
  | {
      kind: 'not_recorded';
    }
  | {
      outcome: OperationOutcome;
      cleanup: OperationCleanup;
      kind: 'recorded';
    }
  | {
      diagnostic: string;
      kind: 'unavailable';
    };
/**
 * Unresolved means no durable result is known, including after process loss.
 */
export type OperationOutcome =
  | {
      kind: 'unresolved';
    }
  | {
      value: unknown;
      kind: 'returned';
    }
  | {
      diagnostic: string;
      kind: 'unknown';
    };
export type OperationCleanup =
  | {
      kind: 'unconfirmed';
    }
  | {
      kind: 'confirmed';
    }
  | {
      diagnostic: string;
      kind: 'failed';
    };
export type EditorEdit =
  | {
      text: string;
      kind: 'replace';
    }
  | {
      text: string;
      kind: 'reset';
    };
export type EditorOutcome =
  | {
      status: 'applied';
    }
  | {
      reason: EditorConflict;
      status: 'conflict';
    };
export type EditorConflict = 'revision' | 'composition' | 'sequence';
export type EditorCapture =
  | {
      kind: 'current';
    }
  | {
      revision: number;
      kind: 'at_revision';
    };
/**
 * Resource names select slots in the current request, not durable grants.
 */
export type HostRequest =
  | {
      import: string;
      provider: number;
      method: string;
      input: unknown;
      attachments: {
        [k: string]: string | undefined;
      };
      operation: 'call';
    }
  | {
      import: string;
      provider: number;
      method: string;
      input: unknown;
      operationId: string;
      operation: 'mutate';
    }
  | {
      reference: RecoveryRef;
      operation: 'recover';
    }
  | {
      slot: string;
      operation: 'metadata';
    }
  | {
      slot: string;
      start: number;
      end: number;
      operation: 'read';
    };
export type Identity =
  | {
      invocation: Invocation;
      operationId?: string | null;
      kind: 'agent';
      [k: string]: unknown | undefined;
    }
  | {
      requestId: string;
      kind: 'remote';
      [k: string]: unknown | undefined;
    }
  | {
      grant: Id;
      kind: 'background';
      [k: string]: unknown | undefined;
    };
/**
 * Resolving a reference requires the original package/scope, a live activation,
 * an unrevoked Host record and the issuing principal's current authorization.
 */
export type Id = string;

/**
 * Durable deployment input. Plans are derived and never persisted as overrides.
 */
export interface CompositionInput {
  environment: Environment;
  instances: {
    [k: string]: CompositionNode | undefined;
  };
  launch?: Target | null;
}
export interface Environment {
  location: ExecutionLocation;
  presentation?: PresentationTarget | null;
}
export interface CompositionNode {
  key: string;
  package: PackageSelection;
  usePackage: PackageUse;
  configuration?: Configuration;
  parameters?: Configuration1;
  bindings?: {
    [k: string]: Target[] | undefined;
  };
  disabled?: boolean;
  overrides?: Override[];
}
export interface PackageRef {
  id: string;
  revision: string;
}
export interface Configuration {
  values?: {
    [k: string]: unknown | undefined;
  };
  /**
   * Configuration field -> parameter name of the containing template.
   */
  parameters?: {
    [k: string]: string | undefined;
  };
}
export interface Configuration1 {
  values?: {
    [k: string]: unknown | undefined;
  };
  /**
   * Configuration field -> parameter name of the containing template.
   */
  parameters?: {
    [k: string]: string | undefined;
  };
}
/**
 * An override belongs to its containing structural instance. Missing paths fail;
 * removal/upgrade never guesses how to discard independent user intent.
 */
export interface Override {
  path: string[];
  package?: PackageRef | null;
  usePackage?: PackageUse | null;
  parameters?: {
    [k: string]: unknown | undefined;
  };
  configuration?: {
    [k: string]: unknown | undefined;
  };
  bindings?: {
    [k: string]: Target[] | undefined;
  };
  disabled?: boolean | null;
}
export interface Deployment {
  reference: DeploymentRef;
  input: CompositionInput;
  remoteExports: RemoteObservation[];
}
export interface DeploymentRef {
  stateDomain: string;
  deployment: string;
  revision: number;
}
/**
 * A static compatibility observation, never a remote registration or grant.
 */
export interface RemoteObservation {
  capability: RemoteCapability;
  declaration: Export;
}
export interface RemoteCapability {
  deployment: string;
  instance: InstanceId;
  export: string;
}
export interface Export {
  contract: Contract;
  availability: Availability;
}
/**
 * Public contract definitions are package data, not live registration records.
 */
export interface Contract {
  name: string;
  version: number;
  transport: Transport;
  interface: Interface;
}
export interface Method {
  mode: MethodMode;
  input: unknown;
  output: unknown;
  resources: {
    [k: string]: ResourceType | undefined;
  };
}
/**
 * Nominal read-only resource shape, not a resource id or authorization token.
 */
export interface ResourceType {
  name: string;
  version: number;
}
export interface Capture {
  input: unknown;
  resource: ResourceType;
}
export interface PackageDeclaration {
  entries?: {
    desktop?: EntryDeclaration;
    host?: EntryDeclaration;
    tui?: EntryDeclaration;
  };
  composition?: Template | null;
}
export interface EntryDeclaration {
  implementation: Implementation;
  configuration: unknown;
  imports?: {
    [k: string]: Import | undefined;
  };
  exports?: {
    [k: string]: Export | undefined;
  };
}
export interface Import {
  contract: Contract;
  cardinality: Cardinality;
}
export interface Template {
  parameters?: {
    [k: string]: Parameter | undefined;
  };
  imports?: {
    [k: string]: Import | undefined;
  };
  exports?: {
    [k: string]: Target | undefined;
  };
  children: {
    [k: string]: CompositionNode | undefined;
  };
}
export interface Parameter {
  schema: unknown;
  default?: unknown;
}
/**
 * Read-only data copied from this activation's captured value imports.
 * The keys and member order come from its checked plan, never guest routing.
 */
export interface ValueBindings {
  [k: string]: unknown[] | undefined;
}
/**
 * One callback's result. Events are finite transaction intents, never retained
 * in a committed Frame or replayed during rendering.
 */
export interface ComponentOutput {
  frame: Frame;
  events: Emission[];
  actions: Action[];
}
export interface Frame {
  revision: number;
  state: unknown;
  surface?: Surface | null;
  view: UiNode;
}
export interface Emission {
  name: string;
  value: unknown;
}
/**
 * An inert intent. Resource slots name capture requests, never bearer handles.
 */
export interface Action {
  key: string;
  binding: string;
  member: number;
  method: string;
  input: unknown;
  resources: {
    [k: string]: CaptureSource | undefined;
  };
}
export interface CaptureSource {
  child: string;
  method: string;
  input: unknown;
}
export interface OperationReply {
  reference: RecoveryRef;
  receipt: Receipt;
}
export interface RecoveryRef {
  source: DeploymentRef;
  namespace: InstanceId;
  export: string;
  method: string;
  operation: string;
  fingerprint: string;
  contract: Contract;
  recovery: RecoveryProtocol;
}
export interface CallerHistoryPage {
  after: number;
  limit: number;
}
export interface CallerIntent {
  cursor: number;
  caller: DeploymentRef;
  consumer: InstanceId;
  reference: RecoveryRef;
}
export interface LabelProps {
  text: string;
  emphasis?: boolean;
}
export interface EditorProps {
  initialText?: string;
  placeholder?: string;
  /**
   * None issues no new command; an accepted command remains pending until
   * its matching receipt commits. Clearing props is not cancellation.
   */
  command?: EditorCommand | null;
}
export interface EditorCommand {
  sequence: number;
  basedOn: number;
  edit: EditorEdit;
}
export interface EditorObservation {
  revision: number;
  snapshot: EditorSnapshot;
  receipt?: EditorReceipt | null;
}
export interface EditorSnapshot {
  text: EditorTextRef;
  selection: TextRange;
  reversed: boolean;
  marked?: TextRange | null;
}
export interface EditorTextRef {
  owner: string;
  revision: number;
  bytes: number;
}
export interface TextRange {
  start: number;
  end: number;
}
export interface EditorReceipt {
  sequence: number;
  revision: number;
  outcome: EditorOutcome;
}
export interface ReadText {
  start: number;
  end: number;
}
export interface TextChunk {
  text: string;
  metadata: unknown;
}
export interface HostActivation {
  contract: Contract;
  service: string[];
}
/**
 * Actual callback observation. Neither the source nor receipt is a grant.
 */
export interface HostObservation {
  source: Identity;
  operation?: RecoveryRef | null;
}
export interface Invocation {
  session_id: string;
  turn_id: string;
  run_id: string;
  invocation_id: string;
  [k: string]: unknown | undefined;
}
export interface OperationObservation {
  reference: RecoveryRef;
  outcome: OperationOutcome;
  cleanup: OperationCleanup;
}
export interface ResourceMetadata {
  type: ResourceType;
  bytes: number;
  metadata: unknown;
}
