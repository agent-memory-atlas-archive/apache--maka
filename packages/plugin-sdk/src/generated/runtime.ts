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
  | ComponentSlot
  | CapturePlan
  | ActionOutcome
  | CallerHistoryPage
  | CallerIntent
  | LabelProps
  | TextareaProps
  | InputProps
  | EditorObservation
  | EditorCapture
  | ReadText
  | TextChunk
  | AgentConfiguration
  | ModelSelection
  | SettingsModelsRead
  | SettingsModelsPage
  | SettingsConnectionsRead
  | SettingsConnectionsPage
  | SettingsConnectionDetail
  | SettingsProvidersRead
  | SettingsProvidersPage
  | SettingsSaveCredential
  | SettingsCredentialSaveResult
  | CreateCatalogConnectionInput
  | UpdateCatalogConnectionInput
  | RemoveCatalogConnectionInput
  | CredentialVaultQueryInput
  | CredentialVaultQueryResult
  | DeleteCredentialInput
  | CredentialMutationResult
  | SetDefaultConnectionTargetInput
  | CatalogMutationResult
  | AgentList
  | AgentCreate
  | AgentSend
  | AgentStop
  | AgentWatch
  | AgentSession
  | AgentSessionPage
  | AgentStarted
  | AgentStopped
  | AgentDelivery
  | AgentDeliveryFact
  | AgentContentKind
  | AgentObservation
  | AgentSessionRead
  | AgentHistoryRead
  | AgentDeliveryPage
  | AgentHistoryChunk
  | AgentHistoryPage
  | Read
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
      value: unknown;
      status: 'updated';
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
export type FontFamily = string;
/**
 * Packed RGBA, including explicit alpha zero.
 */
export type Color = number;
export type ScrollbarMode = 'scrolling' | 'hover' | 'always';
export type ScrollbarEntrance = 'fade' | 'slide_and_fade';
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
export type EditorConflict = 'owner' | 'representation' | 'revision' | 'composition' | 'sequence';
export type EditorCapture =
  | {
      kind: 'current';
    }
  | {
      owner: string;
      revision: number;
      kind: 'at_revision';
    };
export type SettingsModelsRead =
  | {
      kind: 'start';
    }
  | {
      revision: number;
      cursor: SettingsModelCursor;
      kind: 'continue';
    };
export type SettingsModelsPage =
  | {
      revision: number;
      kind: 'changed';
    }
  | {
      revision: number;
      /**
       * @maxItems 16
       */
      choices:
        | []
        | [SettingsModelChoice]
        | [SettingsModelChoice, SettingsModelChoice]
        | [SettingsModelChoice, SettingsModelChoice, SettingsModelChoice]
        | [SettingsModelChoice, SettingsModelChoice, SettingsModelChoice, SettingsModelChoice]
        | [
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
          ]
        | [
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
          ]
        | [
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
          ]
        | [
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
          ]
        | [
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
          ]
        | [
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
          ]
        | [
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
          ]
        | [
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
          ]
        | [
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
          ]
        | [
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
          ]
        | [
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
          ]
        | [
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
            SettingsModelChoice,
          ];
      defaultModel: SettingsDefaultModel;
      next: SettingsModelCursor | null;
      kind: 'ready';
    };
export type SettingsDefaultModel =
  | {
      kind: 'none';
    }
  | {
      target: ConnectionTarget;
      kind: 'unavailable';
    }
  | {
      choice: SettingsModelChoice;
      kind: 'ready';
    };
export type SettingsConnectionsRead =
  | {
      kind: 'start';
    }
  | {
      revision: number;
      offset: number;
      kind: 'continue';
    };
export type SettingsConnectionsPage =
  | {
      revision: number;
      kind: 'changed';
    }
  | {
      revision: number;
      defaultTarget: ConnectionTarget | null;
      /**
       * @maxItems 16
       */
      connections:
        | []
        | [SettingsConnectionSummary]
        | [SettingsConnectionSummary, SettingsConnectionSummary]
        | [SettingsConnectionSummary, SettingsConnectionSummary, SettingsConnectionSummary]
        | [
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
          ]
        | [
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
          ]
        | [
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
          ]
        | [
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
          ]
        | [
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
          ]
        | [
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
          ]
        | [
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
          ]
        | [
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
          ]
        | [
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
          ]
        | [
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
          ]
        | [
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
          ]
        | [
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
          ]
        | [
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
            SettingsConnectionSummary,
          ];
      next: number | null;
      kind: 'ready';
    };
export type SettingsConnectionDetail =
  | {
      actual: ConnectionVersionBasis | null;
      kind: 'stale';
    }
  | {
      target: ConnectionCredentialTarget;
      name: string;
      enabled: boolean;
      enabledModelCount: number;
      kind: 'ready';
    };
export type CredentialLocator =
  | {
      connectionId: string;
      kind: ConnectionCredentialKind;
      scope: 'connection';
    }
  | {
      kind: PasswordKind;
      scope: 'network_proxy';
    };
export type ConnectionCredentialKind = 'provider' | 'request_headers';
export type PasswordKind = 'password';
export type SettingsCredentialSaveResult =
  | {
      connection: ConnectionIdentity;
      kind: 'saved';
    }
  | {
      reason: SettingsCredentialRejection;
      kind: 'rejected';
    };
export type SettingsCredentialRejection =
  | 'invalid_input'
  | 'invalid_secret'
  | 'secret_unavailable'
  | 'provider_unavailable'
  | 'authentication_unavailable'
  | 'attempt_conflict'
  | 'connection_changed'
  | 'credential_changed'
  | 'capacity'
  | 'slug_taken'
  | 'authentication';
export type ThinkingLevel =
  | 'off'
  | 'minimal'
  | 'low'
  | 'medium'
  | 'high'
  | 'xhigh'
  | 'max'
  | 'ultra';
export type ApiProtocol = 'openai-chat' | 'openai-responses' | 'anthropic-messages';
export type ModelModality = 'text' | 'image' | 'audio' | 'pdf' | 'video';
export type RelayServiceTier = 'fast';
export type Nullable_MapOf_ModelOverride = {
  [k: string]: ModelOverride | undefined;
} | null;
export type CredentialVaultQueryResult =
  | {
      status: CredentialStatus;
      kind: 'status';
    }
  | {
      kind: 'connection_not_found';
    };
export type CredentialMutationResult =
  | {
      reason: ConfigurationRejection;
      kind: 'rejected';
    }
  | {
      vaultRevision: number;
      status: CredentialStatus;
      kind: 'committed';
    }
  | {
      kind: 'connection_not_found';
    }
  | {
      expected: ConnectionVersionBasis;
      actual: ConnectionVersionBasis | null;
      kind: 'connection_stale';
    }
  | {
      expected: CredentialVersionBasis | null;
      actual: CredentialVersionBasis | null;
      kind: 'credential_stale';
    };
/**
 * A confirmed rejection contains no provider diagnostics or user input.
 */
export type ConfigurationRejection =
  | 'invalid_input'
  | 'invalid_configuration'
  | 'provider_unavailable';
export type CatalogMutationResult =
  | {
      reason: ConfigurationRejection;
      kind: 'rejected';
    }
  | {
      catalogRevision: number;
      connection?: ConnectionVersionBasis | null;
      kind: 'committed';
    }
  | {
      expectedRevision: number;
      actualRevision: number;
      kind: 'revision_conflict';
    }
  | {
      slug: string;
      kind: 'connection_exists';
    }
  | {
      expected: ConnectionVersionBasis;
      actual: ConnectionVersionBasis | null;
      kind: 'connection_stale';
    }
  | {
      target: ConnectionTarget;
      kind: 'invalid_default_target';
    };
export type AgentDeliveryFact =
  | {
      kind: 'opened';
    }
  | {
      kind: 'transcript_changed';
    }
  | {
      stepId: string;
      kind: 'step_ended';
    }
  | {
      messageId: string;
      stepId: string;
      partId: string;
      textKind: AgentContentKind;
      kind: 'part_started';
    }
  | {
      stepId: string;
      partId: string;
      text: string;
      kind: 'delta';
    }
  | {
      stepId: string;
      partId: string;
      kind: 'part_finished';
    }
  | {
      failed: boolean;
      kind: 'ended';
    };
export type AgentContentKind = 'text' | 'thinking';
export type AgentRunState = 'admitted' | 'running' | 'waiting_for_user';
export type AgentHistoryPosition =
  | {
      kind: 'start';
    }
  | {
      sequence: number;
      kind: 'event';
    }
  | {
      cursor: Cursor;
      kind: 'cursor';
    };
/**
 * Text visible in the durable conversation; reasoning and runtime metadata are
 * not conversation text. Tool JSON contributes string values, never its keys.
 */
export type Role = 'user' | 'assistant' | 'tool_call' | 'tool_result';
export type AgentHistoryPage =
  | {
      through: number;
      kind: 'preparing';
    }
  | {
      through: number;
      chunks: AgentHistoryChunk[];
      next?: Cursor | null;
      kind: 'ready';
    };
/**
 * Resource names select slots in the current request, not durable grants.
 */
export type HostRequest =
  | {
      value: unknown;
      operation: 'emit';
    }
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
      attachments: {
        [k: string]: string | undefined;
      };
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
export type Identity2 =
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
  /**
   * Nominal ABI of this JavaScript entry's component scenes. Native recipes
   * are checked through their compiled implementation availability instead.
   */
  scene?: SceneLanguage | null;
  configuration: unknown;
  imports?: {
    [k: string]: Import | undefined;
  };
  exports?: {
    [k: string]: Export | undefined;
  };
}
/**
 * Presenter-owned language identity. The kernel does not interpret its syntax.
 */
export interface SceneLanguage {
  name: string;
  version: number;
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
  /**
   * Complete current subscription demand. Empty removes previous demand.
   */
  subscriptions?: Action[];
  /**
   * Applies only to the original raw input, after the entire batch commits.
   * It is never retained in a frame or replayed by a renderer.
   */
  decision?: InputDecision | null;
}
export interface Frame {
  revision: number;
  state: unknown;
  surface?: Surface | null;
  view: unknown;
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
export interface InputDecision {
  preventDefault?: boolean;
  stopPropagation?: boolean;
}
/**
 * One child of a committed component, owned by a committed native presentation.
 * Binding resolution belongs to the parent activation and the common mount kernel.
 */
export interface ComponentSlot {
  key: string;
  binding: string;
  member: number;
  props: unknown;
  on: {
    [k: string]: string | undefined;
  };
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
  appearance: TextAppearance;
}
export interface TextAppearance {
  font: FontAppearance;
  foreground: Color;
}
export interface FontAppearance {
  family: FontFamily;
  /**
   * @maxItems 8
   */
  fallbacks:
    | []
    | [FontFamily]
    | [FontFamily, FontFamily]
    | [FontFamily, FontFamily, FontFamily]
    | [FontFamily, FontFamily, FontFamily, FontFamily]
    | [FontFamily, FontFamily, FontFamily, FontFamily, FontFamily]
    | [FontFamily, FontFamily, FontFamily, FontFamily, FontFamily, FontFamily]
    | [FontFamily, FontFamily, FontFamily, FontFamily, FontFamily, FontFamily, FontFamily]
    | [
        FontFamily,
        FontFamily,
        FontFamily,
        FontFamily,
        FontFamily,
        FontFamily,
        FontFamily,
        FontFamily,
      ];
  size: number;
  /**
   * Line height relative to this font size.
   */
  lineHeight: number;
  weight: number;
}
export interface TextareaProps {
  appearance: TextareaAppearance;
  menu?: TextareaMenu | null;
  accessibilityLabel: string;
  readOnly: boolean;
  disabled: boolean;
  softWrap: boolean;
  tabIndex: number;
  initialText?: string;
  placeholder?: string;
  /**
   * None issues no new command; an accepted command remains pending until
   * its matching receipt commits. Clearing props is not cancellation.
   */
  command?: EditorCommand | null;
}
export interface TextareaAppearance {
  text: TextAppearance;
  mutedForeground: Color;
  background: Color;
  selection: Color;
  caret: Color;
  border: Color;
  focusedBorder: Color;
  borderWidth: number;
  radius: number;
  padding: Insets;
  scrollbar: ScrollbarAppearance;
}
export interface Insets {
  top: number;
  right: number;
  bottom: number;
  left: number;
}
export interface ScrollbarAppearance {
  mode: ScrollbarMode;
  entrance: ScrollbarEntrance;
  idleMillis: number;
  enterMillis: number;
  exitMillis: number;
  expandMillis: number;
  trackWidth: number;
  track: Color;
  trackHover: Color;
  trackActive: Color;
  thumb: ScrollbarThumb;
  thumbHover: ScrollbarThumb;
  thumbActive: ScrollbarThumb;
}
export interface ScrollbarThumb {
  color: Color;
  width: number;
  inset: number;
  radius: number;
  minLength: number;
}
/**
 * Local presentation for user-initiated native editing actions. This is not
 * a command interface: clipboard access still requires native activation.
 */
export interface TextareaMenu {
  appearance: TextareaMenuAppearance;
  labels: TextareaMenuLabels;
}
export interface TextareaMenuAppearance {
  text: TextAppearance;
  background: Color;
  border: Color;
  activeBackground: Color;
  activeForeground: Color;
  disabledForeground: Color;
  borderWidth: number;
  radius: number;
  padding: Insets;
  rowPadding: Insets;
  width: number;
  windowMargin: number;
}
export interface TextareaMenuLabels {
  title: string;
  undo: string;
  redo: string;
  cut: string;
  copy: string;
  paste: string;
  selectAll: string;
}
export interface EditorCommand {
  /**
   * Correlation with the original native editor, never a resource grant.
   */
  owner: string;
  sequence: number;
  basedOn: number;
  edit: EditorEdit;
}
export interface InputProps {
  appearance: InputAppearance;
  accessibilityLabel: string;
  readOnly: boolean;
  disabled: boolean;
  tabIndex: number;
  placeholder?: string;
  command?: EditorCommand | null;
}
export interface InputAppearance {
  text: TextAppearance;
  mutedForeground: Color;
  background: Color;
  selection: Color;
  caret: Color;
  border: Color;
  focusedBorder: Color;
  borderWidth: number;
  radius: number;
  padding: Insets;
}
export interface EditorObservation {
  revision: number;
  snapshot: EditorSnapshot;
  receipt: EditorReceipt | null;
}
export interface EditorSnapshot {
  text: EditorTextRef;
  selection: TextRange;
  reversed: boolean;
  marked: TextRange | null;
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
  owner: string;
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
export interface AgentConfiguration {
  maxActiveRuns: number;
  maxSteps: number;
}
/**
 * Exact public selection observed by a caller, checked once at run admission.
 */
export interface ModelSelection {
  connectionId: string;
  revision: number;
  modelId: string;
}
export interface SettingsModelCursor {
  connection: number;
  model: number;
}
export interface SettingsModelChoice {
  selection: ModelSelection;
  connectionName: string;
  label: string;
}
export interface ConnectionTarget {
  connectionId: string;
  modelId: string;
}
export interface SettingsConnectionSummary {
  basis: ConnectionVersionBasis;
  slug: string;
  name: string;
  provider: Identity;
  enabled: boolean;
  enabledModelCount: number;
}
export interface ConnectionVersionBasis {
  connectionId: string;
  revision: number;
}
/**
 * Durable recipient, not a registration lease. Re-loading the same package can
 * restore it; another package, Entry or scope cannot inherit its credentials.
 */
export interface Identity {
  packageId: string;
  entryId: string;
  scope: string;
  name: string;
}
export interface ConnectionCredentialTarget {
  connectionId: string;
  revision: number;
  slug: string;
  provider: Identity;
  configuration: {
    [k: string]: unknown | undefined;
  };
}
export interface SettingsProvidersRead {
  offset: number;
}
export interface SettingsProvidersPage {
  /**
   * @maxItems 8
   */
  providers:
    | []
    | [SettingsProvider]
    | [SettingsProvider, SettingsProvider]
    | [SettingsProvider, SettingsProvider, SettingsProvider]
    | [SettingsProvider, SettingsProvider, SettingsProvider, SettingsProvider]
    | [SettingsProvider, SettingsProvider, SettingsProvider, SettingsProvider, SettingsProvider]
    | [
        SettingsProvider,
        SettingsProvider,
        SettingsProvider,
        SettingsProvider,
        SettingsProvider,
        SettingsProvider,
      ]
    | [
        SettingsProvider,
        SettingsProvider,
        SettingsProvider,
        SettingsProvider,
        SettingsProvider,
        SettingsProvider,
        SettingsProvider,
      ]
    | [
        SettingsProvider,
        SettingsProvider,
        SettingsProvider,
        SettingsProvider,
        SettingsProvider,
        SettingsProvider,
        SettingsProvider,
        SettingsProvider,
      ];
  next: number | null;
}
export interface SettingsProvider {
  identity: Identity;
  descriptor: Descriptor;
}
export interface Descriptor {
  label: string;
  /**
   * Offers a user-selected endpoint instead of a named service.
   */
  customEndpoint?: boolean;
  configurationSchema: unknown;
  configurationDefaults: unknown;
  authentication: Method2[];
  /**
   * Whether the provider supports a connection without a stored credential.
   * Presentation metadata; authorization remains the provider's decision.
   */
  anonymous: boolean;
  discovery: boolean;
}
export interface Method2 {
  id: string;
  label: string;
  /**
   * Secret-bearing form input never enters the connection configuration.
   */
  inputSchema: {
    [k: string]: unknown | undefined;
  };
  interactive: boolean;
}
export interface SettingsSaveCredential {
  expected: ConnectionCredentialTarget;
  expectedCredential: CredentialVersionBasis | null;
}
export interface CredentialVersionBasis {
  locator: CredentialLocator;
  credentialId: string;
  revision: number;
}
export interface ConnectionIdentity {
  connectionId: string;
  slug: string;
  provider: Identity;
}
export interface CreateCatalogConnectionInput {
  expectedCatalogRevision: number;
  connection: ConnectionCatalogEntryDraft;
}
export interface ConnectionCatalogEntryDraft {
  slug: string;
  name: string;
  provider: Identity;
  configuration: {
    [k: string]: unknown | undefined;
  };
  enabled: boolean;
  enabledModelIds: string[];
  modelOverrides?: {
    [k: string]: ModelOverride | undefined;
  };
  requestBodyOverlay?: {
    [k: string]: unknown | undefined;
  };
}
export interface ModelOverride {
  /**
   * Public adapter contribution name; omitted selects the wire's default.
   */
  adapter?: string;
  /**
   * Omitted means model-specific defaults; false is an explicit opt-out.
   */
  codeMode?: boolean;
  /**
   * Independent of Code Mode: select patch editing or structured Edit/Write.
   */
  applyPatch?: boolean;
  thinkingLevels?: ThinkingLevel[];
  defaultThinkingLevel?: ThinkingLevel;
  vision?: boolean;
  /**
   * Override total input-plus-output capacity, without subtracting a reply budget.
   */
  contextWindow?: number;
  /**
   * Optional override of the automatic compaction threshold, in retained tokens.
   * By default Host uses 85% of the smaller total window/input ceiling.
   */
  compactionThreshold?: number;
  /**
   * Independent input ceiling; also caps the default compaction input budget.
   */
  inputLimit?: number;
  /**
   * Requested per-step output budget, subject to provider and remaining-capacity caps.
   */
  maxOutputTokens?: number;
  displayName?: string;
  description?: string;
  apiProtocol?: ApiProtocol;
  knowledgeCutoff?: string;
  capabilities?: ModelOverrideCapabilities;
  modalities?: ModelModalities;
  serviceTier?: RelayServiceTier;
}
/**
 * Vision has one declaration owner: ModelOverride::vision.
 */
export interface ModelOverrideCapabilities {
  chat?: boolean;
  reasoning?: boolean;
  functionCalling?: boolean;
  parallelToolCalls?: boolean;
  imageGeneration?: boolean;
  webSearch?: boolean;
}
export interface ModelModalities {
  input: ModelModality[];
  output: ModelModality[];
}
export interface UpdateCatalogConnectionInput {
  expected: ConnectionVersionBasis;
  changes: ConnectionCatalogEntryUpdate;
}
export interface ConnectionCatalogEntryUpdate {
  name?: string;
  configuration?: {
    [k: string]: unknown | undefined;
  };
  enabled?: boolean;
  enabledModelIds?: string[];
  modelOverrides?: Nullable_MapOf_ModelOverride;
  requestBodyOverlay?: {
    [k: string]: unknown | undefined;
  } | null;
}
export interface RemoveCatalogConnectionInput {
  expected: ConnectionVersionBasis;
}
export interface CredentialVaultQueryInput {
  locator: CredentialLocator;
}
export interface CredentialStatus {
  locator: CredentialLocator;
  configured: boolean;
  credentialId: string | null;
  revision: number | null;
  updatedAt: number | null;
}
export interface DeleteCredentialInput {
  expected: CredentialVersionBasis;
  /**
   * Required for connection credentials; null for network credentials.
   */
  expectedConnection: ConnectionCredentialTarget | null;
}
export interface SetDefaultConnectionTargetInput {
  expectedCatalogRevision: number;
  target: ConnectionTarget | null;
}
export interface AgentList {
  revision?: string | null;
  cursor?: string | null;
}
export interface AgentCreate {
  title: string;
}
export interface AgentSend {
  model: ModelSelection;
  sessionId: string;
  message: string;
}
export interface AgentStop {
  sessionId: string;
  invocationId: string;
}
export interface AgentWatch {
  sessionId: string;
  after: number;
}
export interface AgentSession {
  id: string;
  title: string;
  revision: number;
  updatedAt: number;
  archived: boolean;
}
export interface AgentSessionPage {
  revision: string;
  sessions: AgentSession[];
  nextCursor?: string | null;
}
export interface AgentStarted {
  sessionId: string;
  invocationId: string;
}
export interface AgentStopped {
  requested: boolean;
}
export interface AgentDelivery {
  sequence: number;
  invocationId: string;
  fact: AgentDeliveryFact;
}
export interface AgentObservation {
  session: AgentSession;
  through: number;
  run?: AgentRun | null;
  parts: AgentPart[];
}
export interface AgentRun {
  invocationId: string;
  state: AgentRunState;
}
export interface AgentPart {
  startSequence: number;
  messageId: string;
  stepId: string;
  partId: string;
  textKind: AgentContentKind;
}
export interface AgentSessionRead {
  sessionId: string;
}
export interface AgentHistoryRead {
  sessionId: string;
  through?: number | null;
  position: AgentHistoryPosition;
}
/**
 * Byte offset in one message's UTF-8 text, not a bearer capability.
 */
export interface Cursor {
  sequence: number;
  offset: number;
}
export interface AgentDeliveryPage {
  through: number;
  nextAfter: number;
  events: AgentDelivery[];
}
export interface AgentHistoryChunk {
  messageId: string;
  turnId: string;
  timestamp: number;
  role: Role;
  sequence: number;
  offset: number;
  totalBytes: number;
  text: string;
}
/**
 * A stable log fence prevents a scan from chasing newly appended messages.
 */
export interface Read {
  sessionId: string;
  through?: number | null;
  cursor?: Cursor | null;
}
export interface HostActivation {
  contract: Contract;
  service: string[];
}
/**
 * Actual callback observation. Neither the source nor receipt is a grant.
 */
export interface HostObservation {
  source: Identity2;
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
