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
  | DeploymentRead
  | PackageInstall
  | PackageInstalled
  | DeploymentReplace
  | ManagedDeployment
  | DeploymentInstances
  | ManagedInstancesPage
  | InstanceSetEnabled
  | DeploymentDocument
  | DeploymentDocumentRead
  | DeploymentDocumentChunk
  | ManagementResultForPackageInstalled
  | ManagementResultForDeploymentRef
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
  | TerminalScene
  | DesktopScene
  | DesktopCanvasInput
  | CapturePlan
  | ActionOutcome
  | CallerHistoryPage
  | CallerIntent
  | LabelProps
  | TextareaProps
  | MarkdownProps
  | MarkdownLink
  | FilePickerProps
  | FileObservation
  | FileCapture
  | ImageProps
  | ImageObservation
  | SourceTextProps
  | SelectionMenuProps
  | ReadingProps
  | ReadingObservation
  | RowsProps
  | RowActivated
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
  | AgentCreated
  | AgentDirectories
  | AgentDirectoryPage
  | AgentMetadata
  | AgentMetadataResult
  | AgentSend
  | AgentStop
  | AgentWatch
  | AgentSession
  | AgentSessionPage
  | AgentSendResult
  | AgentStopped
  | AgentDelivery
  | AgentDeliveryFact
  | AgentContentKind
  | AgentObservation
  | AgentSessionRead
  | AgentHistoryRead
  | AgentDeliveryPage
  | AgentQueue
  | AgentQueueMessage
  | AgentQueueMessageResult
  | AgentQueueMutation
  | AgentQueueMutationResult
  | AgentHistoryChunk
  | AgentHistoryPage
  | AgentToolSourceRead
  | AgentToolSourcePage
  | AgentToolImageRead
  | AgentToolImagePage
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
/**
 * Resource captures enter checked service slots; continuations remain in the
 * original presentation owner and only decide admission of query feedback.
 */
export type CaptureOutput =
  | {
      resource: ResourceType;
      kind: 'resource';
    }
  | {
      kind: 'continuation';
    };
export type Availability = 'required' | 'optional';
/**
 * A fixed committed source for continuation; no runtime or permission claim.
 */
export type DeploymentInstances =
  | {
      kind: 'start';
    }
  | {
      source: DeploymentRef;
      after?: InstanceId | null;
      kind: 'continue';
    };
/**
 * A deterministic refusal is a recorded business result. Uncertain SQL/worker
 * failures remain in the original operation journal's Unknown outcome.
 */
export type ManagementResultForPackageInstalled =
  | {
      value: PackageInstalled;
      status: 'committed';
    }
  | {
      currentRevision: number;
      status: 'conflict';
    }
  | {
      reason: ManagementRejection;
      diagnostic?: string | null;
      status: 'rejected';
    };
export type ManagementRejection =
  | 'invalid_bundle'
  | 'invalid_document'
  | 'invalid_source'
  | 'package_unavailable'
  | 'execution_environment'
  | 'instance_unavailable'
  | 'parent_disabled';
/**
 * A deterministic refusal is a recorded business result. Uncertain SQL/worker
 * failures remain in the original operation journal's Unknown outcome.
 */
export type ManagementResultForDeploymentRef =
  | {
      value: DeploymentRef;
      status: 'committed';
    }
  | {
      currentRevision: number;
      status: 'conflict';
    }
  | {
      reason: ManagementRejection;
      diagnostic?: string | null;
      status: 'rejected';
    };
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
export type TerminalScene =
  | {
      key: string;
      label: string;
      background: TerminalScene;
      content: TerminalScene;
      open: boolean;
      /**
       * Maximum foreground body dimensions, in terminal cells.
       */
      width: number;
      height: number;
      background_color: number;
      border_color: number;
      padding?: number;
      kind: 'dialog';
    }
  | {
      key: string;
      direction: TerminalDirection;
      gap: number;
      padding: number;
      background: number;
      children: TerminalScene[];
      /**
       * @maxItems 32
       */
      shortcuts?: Shortcut[];
      /**
       * @maxItems 32
       */
      focus_shortcuts?: FocusShortcut[];
      layout?: TerminalLayout;
      scroll?: TerminalScrollPolicy & string;
      visible?: boolean;
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
      disabled?: boolean;
      /**
       * Absent foreground uses the terminal's default text color.
       */
      color?: number | null;
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
export type TerminalDirection = 'row' | 'column';
export type Modifier = 'control' | 'primary' | 'alt' | 'shift' | 'super';
/**
 * Native scroll policy; offsets and accepted viewport geometry remain native.
 */
export type TerminalScrollPolicy = 'none' | 'manual' | 'anchor' | 'follow_end';
export type DesktopScene =
  | {
      key: string;
      label: string;
      background: DesktopScene;
      content: DesktopScene;
      open: boolean;
      /**
       * Styles the bounded foreground body; background layout remains original.
       */
      style?: DesktopStyle;
      backdrop?: Color;
      kind: 'dialog';
    }
  | {
      key: string;
      label: string;
      trigger: DesktopScene;
      content: DesktopScene;
      open: boolean;
      anchor?: DesktopAnchor & string;
      offset?: number;
      /**
       * Styles the floating content; the trigger keeps its own layout.
       */
      style?: DesktopStyle;
      kind: 'popup';
    }
  | {
      key: string;
      direction: DesktopDirection;
      gap: number;
      padding: number;
      background: number;
      children: DesktopScene[];
      /**
       * @maxItems 32
       */
      shortcuts?: Shortcut[];
      visible?: boolean;
      style?: DesktopStyle;
      scroll?: DesktopScroll & string;
      kind: 'stack';
    }
  | {
      key: string;
      text: string;
      color: number;
      style?: DesktopStyle;
      kind: 'text';
    }
  | {
      key: string;
      label: string;
      style?: DesktopStyle;
      hover?: DesktopStyle;
      active?: DesktopStyle;
      focus?: DesktopStyle;
      disabled?: boolean;
      tab_index?: number;
      kind: 'button';
    }
  | {
      key: string;
      contents: DesktopCanvasContents;
      style?: DesktopStyle;
      kind: 'canvas';
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
export type DesktopDimension =
  | {
      unit: 'pixels';
      value: number;
    }
  | {
      unit: 'fraction';
      value: number;
    }
  | {
      unit: 'auto';
    };
export type DesktopAlign = 'start' | 'center' | 'end' | 'stretch' | 'baseline';
export type DesktopJustify = 'start' | 'center' | 'end' | 'between' | 'around' | 'evenly';
export type FontFamily = string;
/**
 * Packed RGBA, including explicit alpha zero.
 */
export type Color = number;
export type DesktopAnchor =
  | 'top_left'
  | 'top_center'
  | 'top_right'
  | 'bottom_left'
  | 'bottom_center'
  | 'bottom_right'
  | 'left_center'
  | 'right_center';
export type DesktopDirection = 'row' | 'column';
/**
 * A scroll policy belongs to native presentation, never a plugin pixel mirror.
 */
export type DesktopScroll = 'none' | 'manual' | 'anchor' | 'follow_end';
export type DesktopCanvasCommand =
  | {
      rect: DesktopCanvasRect;
      color: number;
      radius?: number;
      kind: 'rect';
    }
  | {
      from: DesktopCanvasPoint;
      to: DesktopCanvasPoint;
      color: number;
      width: number;
      kind: 'line';
    }
  | {
      position: DesktopCanvasPoint;
      text: string;
      color: number;
      size: number;
      font?: string | null;
      kind: 'text';
    };
export type DesktopCanvasEventKind =
  | 'pointer_down'
  | 'drag_move'
  | 'drop'
  | 'key_down'
  | 'activate';
export type DesktopCanvasInput =
  | {
      region: string;
      position: DesktopCanvasPoint;
      gesture?: number | null;
      kind: 'pointer_down';
    }
  | {
      region: string;
      gesture: number;
      start: DesktopCanvasPoint;
      position: DesktopCanvasPoint;
      delta: DesktopCanvasPoint;
      kind: 'drag_move';
    }
  | {
      region: string;
      gesture: number;
      start: DesktopCanvasPoint;
      position: DesktopCanvasPoint;
      delta: DesktopCanvasPoint;
      target?: string | null;
      kind: 'drop';
    }
  | {
      region: string;
      key: string;
      modifiers: DesktopCanvasModifiers;
      kind: 'key_down';
    }
  | {
      region: string;
      kind: 'activate';
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
      status: 'stale';
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
export type ScrollbarMode = 'scrolling' | 'hover' | 'always';
export type ScrollbarEntrance = 'fade' | 'slide_and_fade';
/**
 * Product-selected native submit gesture. PrimaryEnter uses the platform
 * accelerator on Desktop and Control+Enter in a terminal.
 */
export type SubmitKey = 'enter' | 'primary_enter' | 'control_enter';
export type EditorEdit =
  | {
      text: string;
      kind: 'replace';
    }
  | {
      text: string;
      kind: 'reset';
    }
  | {
      text: string;
      kind: 'append';
    };
export type FilePhase = 'empty' | 'choosing' | 'ready' | 'failed';
export type FileError =
  | 'invalid_file'
  | 'read_failed'
  | 'limit_exceeded'
  | 'budget_exhausted'
  | 'worker_failed';
export type ImageFit = 'contain' | 'cover';
export type ImagePhase = 'empty' | 'receiving' | 'decoding' | 'ready' | 'failed';
export type ImageError =
  | 'budget_exhausted'
  | 'limit_exceeded'
  | 'source_conflict'
  | 'packet_rejected'
  | 'digest_mismatch'
  | 'unsupported_format'
  | 'invalid_image'
  | 'decode_failed';
export type ReadingTarget =
  | {
      kind: 'start';
    }
  | {
      kind: 'end';
    }
  | {
      key: string;
      kind: 'reveal_child';
    };
export type ReadingStatus = 'applied' | 'cancelled' | 'rejected';
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
      query?: string;
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
export type AgentList =
  | {
      kind: 'start';
    }
  | {
      revision: string;
      cursor: string;
      kind: 'after';
    }
  | {
      revision: string;
      cursor: string;
      kind: 'before';
    };
export type AgentWorkspaceSelection =
  | {
      selection: AgentDirectorySelection;
      kind: 'directory';
    }
  | {
      sessionId: string;
      kind: 'session';
    };
export type AgentCreated =
  | {
      session: AgentSession;
      kind: 'created';
    }
  | {
      reason: AgentRejection;
      kind: 'rejected';
    };
export type AgentRejection =
  | 'title_invalid'
  | 'labels_invalid'
  | 'workspace_unavailable'
  | 'session_unavailable'
  | 'closed';
export type AgentDirectories =
  | {
      kind: 'roots';
    }
  | {
      selection: AgentDirectorySelection;
      after: string | null;
      kind: 'children';
    };
export type AgentDirectoryPage =
  | {
      /**
       * @maxItems 8
       */
      roots:
        | []
        | [AgentDirectory]
        | [AgentDirectory, AgentDirectory]
        | [AgentDirectory, AgentDirectory, AgentDirectory]
        | [AgentDirectory, AgentDirectory, AgentDirectory, AgentDirectory]
        | [AgentDirectory, AgentDirectory, AgentDirectory, AgentDirectory, AgentDirectory]
        | [
            AgentDirectory,
            AgentDirectory,
            AgentDirectory,
            AgentDirectory,
            AgentDirectory,
            AgentDirectory,
          ]
        | [
            AgentDirectory,
            AgentDirectory,
            AgentDirectory,
            AgentDirectory,
            AgentDirectory,
            AgentDirectory,
            AgentDirectory,
          ]
        | [
            AgentDirectory,
            AgentDirectory,
            AgentDirectory,
            AgentDirectory,
            AgentDirectory,
            AgentDirectory,
            AgentDirectory,
            AgentDirectory,
          ];
      kind: 'roots';
    }
  | {
      directory: AgentDirectory;
      /**
       * @maxItems 32
       */
      children: AgentDirectory[];
      after: string | null;
      kind: 'children';
    };
export type AgentMetadataResult =
  | {
      session: AgentSession;
      kind: 'committed';
    }
  | {
      expected: number;
      actual: number;
      kind: 'revision_conflict';
    }
  | {
      reason: AgentRejection;
      kind: 'rejected';
    };
export type AgentSendResult =
  | {
      sessionId: string;
      invocationId: string;
      kind: 'started';
    }
  | {
      sessionId: string;
      messageId: string;
      revision: number;
      kind: 'queued';
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
/**
 * Cursor sequence addresses a sparse transcript row; Event addresses the raw
 * event log. Before excludes its end and After includes its start. Returned
 * chunks always retain ascending source order, including a reverse window.
 */
export type HistoryPosition =
  | {
      kind: 'start';
    }
  | {
      kind: 'tail';
    }
  | {
      cursor: Cursor;
      kind: 'before';
    }
  | {
      cursor: Cursor;
      kind: 'after';
    }
  | {
      sequence: number;
      kind: 'event';
    };
export type AgentDeliveryPage =
  | {
      through: number;
      nextAfter: number;
      events: AgentDelivery[];
      kind: 'events';
    }
  | {
      queue: AgentQueue;
      kind: 'queue';
    };
export type AgentQueueKind = 'in_flight' | 'steering' | 'followup';
export type AgentQueueMessageResult =
  | {
      revision: number;
      message: string;
      kind: 'ready';
    }
  | {
      expected: number;
      actual: number;
      kind: 'revision_conflict';
    }
  | {
      kind: 'missing';
    };
export type AgentQueueAction =
  | {
      messageId: string;
      kind: 'retract';
    }
  | {
      messageId: string;
      kind: 'promote';
    }
  | {
      messageId: string;
      message: string;
      kind: 'update';
    }
  | {
      /**
       * @minItems 1
       * @maxItems 64
       */
      messageIds: [string, ...string[]];
      kind: 'reorder';
    };
export type AgentQueueMutationResult =
  | {
      revision: number;
      kind: 'committed';
    }
  | {
      expected: number;
      actual: number;
      kind: 'revision_conflict';
    }
  | {
      reason: AgentQueueRejection;
      kind: 'rejected';
    };
export type AgentQueueRejection =
  | 'closed'
  | 'missing'
  | 'in_flight'
  | 'invalid_edit'
  | 'execution_changed'
  | 'not_running';
/**
 * Text visible in the durable conversation; reasoning and runtime metadata are
 * not conversation text. Tool JSON contributes string values, never its keys.
 */
export type Role = 'user' | 'assistant' | 'tool_call' | 'tool_result';
/**
 * Immutable canonical presentation facts, not execution or recovery authority.
 */
export type HistoryTool =
  | {
      toolUseId: string;
      toolName: string;
      kind: 'call';
    }
  | {
      toolUseId: string;
      toolName: string | null;
      /**
       * The presentation's error flag does not distinguish failed from unknown effects.
       */
      isError: boolean;
      /**
       * Canonical content is an image; this does not grant artifact reads.
       */
      image: boolean;
      kind: 'result';
    };
export type AgentHistoryPage =
  | {
      through: number;
      kind: 'preparing';
    }
  | {
      through: number;
      chunks: AgentHistoryChunk[];
      before: Cursor | null;
      after: Cursor | null;
      kind: 'ready';
    };
/**
 * An explicitly selected canonical tool field, never an execution grant.
 */
export type ToolSourceField = 'arguments' | 'result';
export type SourcePosition =
  | {
      kind: 'start';
    }
  | {
      kind: 'tail';
    }
  | {
      offset: number;
      kind: 'before';
    }
  | {
      offset: number;
      kind: 'after';
    };
export type AgentToolSourcePage =
  | {
      through: number;
      kind: 'preparing';
    }
  | {
      through: number;
      source: ToolSource;
      kind: 'ready';
    };
export type SourceFormat = 'text' | 'json' | 'image_descriptor';
export type AgentToolImagePage =
  | {
      through: number;
      kind: 'preparing';
    }
  | {
      through: number;
      sequence: number;
      offset: number;
      totalBytes: number;
      mimeType: string;
      digest: string;
      /**
       * Standard padded Base64 of at most 32KiB of encoded image bytes.
       */
      data: string;
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
  parameters?: Configuration;
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
  output: CaptureOutput;
}
export interface DeploymentRead {}
export interface PackageInstall {
  expectedRevision: number;
}
export interface PackageInstalled {
  source: DeploymentRef;
  package: PackageRef;
}
export interface DeploymentReplace {
  expectedRevision: number;
}
/**
 * Committed desired source and available immutable packages, not runtime readiness.
 */
export interface ManagedDeployment {
  source: Deployment;
  packages: PackageRef[];
}
export interface ManagedInstancesPage {
  source: DeploymentRef;
  /**
   * @maxItems 8
   */
  instances:
    | []
    | [ManagedInstance]
    | [ManagedInstance, ManagedInstance]
    | [ManagedInstance, ManagedInstance, ManagedInstance]
    | [ManagedInstance, ManagedInstance, ManagedInstance, ManagedInstance]
    | [ManagedInstance, ManagedInstance, ManagedInstance, ManagedInstance, ManagedInstance]
    | [
        ManagedInstance,
        ManagedInstance,
        ManagedInstance,
        ManagedInstance,
        ManagedInstance,
        ManagedInstance,
      ]
    | [
        ManagedInstance,
        ManagedInstance,
        ManagedInstance,
        ManagedInstance,
        ManagedInstance,
        ManagedInstance,
        ManagedInstance,
      ]
    | [
        ManagedInstance,
        ManagedInstance,
        ManagedInstance,
        ManagedInstance,
        ManagedInstance,
        ManagedInstance,
        ManagedInstance,
        ManagedInstance,
      ];
  next?: InstanceId | null;
}
export interface ManagedInstance {
  instance: InstanceId;
  path: string[];
  package: PackageRef;
  structural: boolean;
  disabled: boolean;
  parentDisabled: boolean;
}
export interface InstanceSetEnabled {
  expectedRevision: number;
  instance: InstanceId;
  enabled: boolean;
}
/**
 * Source document edited by one native buffer, never a second desired plan.
 */
export interface DeploymentDocument {
  input: CompositionInput;
  /**
   * @maxItems 128
   */
  remoteExports: RemoteObservation[];
  /**
   * @maxItems 128
   */
  packages: PackageRef[];
}
export interface DeploymentDocumentRead {
  source: DeploymentRef;
  offset: number;
}
export interface DeploymentDocumentChunk {
  source: DeploymentRef;
  offset: number;
  totalBytes: number;
  text: string;
  next?: number | null;
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
   * Applies to the original native input after its synchronous event route commits.
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
  continuations?: CaptureSource[];
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
export interface Shortcut {
  key: string;
  /**
   * @maxItems 4
   */
  modifiers:
    | []
    | [Modifier]
    | [Modifier, Modifier]
    | [Modifier, Modifier, Modifier]
    | [Modifier, Modifier, Modifier, Modifier];
  input: string;
  enabled: boolean;
}
/**
 * Route navigation yielded by a native editor to a declared child component.
 * Ordinary unhandled button keys do not activate this binding.
 */
export interface FocusShortcut {
  key: string;
  modifiers: Modifier[];
  child: string;
  enabled: boolean;
}
/**
 * Native terminal cell constraints on a child in its parent's main axis.
 * `grow` distributes remaining cells after fixed/intrinsic siblings; `basis`
 * supplies an explicit base instead of intrinsic length. Row wrap preserves
 * source order and measures its rows before allocating the parent viewport.
 */
export interface TerminalLayout {
  grow?: number;
  basis?: number | null;
  wrap?: boolean;
}
/**
 * Desktop layout and appearance are a presenter ABI, never kernel policy.
 * Unspecified refinements inherit from the explicit surrounding scene scope.
 */
export interface DesktopStyle {
  width?: DesktopDimension | null;
  height?: DesktopDimension | null;
  minWidth?: number | null;
  maxWidth?: number | null;
  minHeight?: number | null;
  maxHeight?: number | null;
  grow?: number | null;
  shrink?: number | null;
  wrap?: boolean | null;
  /**
   * Draw one measured line with an ellipsis; retain the complete text in AX.
   */
  truncate?: boolean;
  align?: DesktopAlign | null;
  justify?: DesktopJustify | null;
  text?: TextAppearance | null;
  background?: Color | null;
  border?: Color | null;
  borderWidth?: number | null;
  radius?: number | null;
  padding?: Insets | null;
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
export interface Insets {
  top: number;
  right: number;
  bottom: number;
  left: number;
}
export interface DesktopCanvasContents {
  commands: DesktopCanvasCommand[];
  regions: DesktopCanvasRegion[];
  focusColor?: number | null;
  dragColor?: number | null;
}
export interface DesktopCanvasRect {
  origin: DesktopCanvasPoint;
  width: number;
  height: number;
}
export interface DesktopCanvasPoint {
  x: number;
  y: number;
}
export interface DesktopCanvasRegion {
  key: string;
  rect: DesktopCanvasRect;
  /**
   * Corner radius of mechanical outlines; the hit area remains `rect`.
   */
  radius?: number;
  label: string;
  draggable?: boolean;
  disabled?: boolean;
  tabIndex?: number;
  listen?: DesktopCanvasEventKind[];
}
export interface DesktopCanvasModifiers {
  control: boolean;
  alt: boolean;
  shift: boolean;
  platform: boolean;
  function: boolean;
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
export interface TextareaProps {
  appearance: TextareaAppearance;
  menu?: TextareaMenu | null;
  accessibilityLabel: string;
  readOnly: boolean;
  disabled: boolean;
  softWrap: boolean;
  /**
   * None preserves native multiline editing without a submit shortcut.
   */
  submit?: SubmitKey | null;
  tabIndex: number;
  /**
   * Offer native focus once when this original input becomes visible and
   * enabled. Stable updates and viewport scrolling do not repeat the offer.
   */
  autoFocus?: boolean;
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
  appearance: MenuAppearance;
  labels: TextareaMenuLabels;
}
export interface MenuAppearance {
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
export interface MarkdownProps {
  source: string;
  appearance: MarkdownAppearance;
  selectable: boolean;
  disabled: boolean;
}
export interface MarkdownAppearance {
  body: TextAppearance;
  code: TextAppearance;
  /**
   * Typography for heading levels 1 through 6, in source order.
   *
   * @minItems 6
   * @maxItems 6
   */
  headings: [
    TextAppearance,
    TextAppearance,
    TextAppearance,
    TextAppearance,
    TextAppearance,
    TextAppearance,
  ];
  muted: Color;
  link: Color;
  selection: Color;
  codeBackground: Color;
  tableBackground: Color;
  border: Color;
  dark: boolean;
  /**
   * Paragraph spacing in logical pixels.
   */
  paragraphGap: number;
}
export interface MarkdownLink {
  url: string;
}
export interface FilePickerProps {
  appearance: InputAppearance;
  label: string;
  pathLabel: string;
  placeholder: string;
  disabled: boolean;
  tabIndex: number;
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
export interface FileObservation {
  owner: string;
  revision: number;
  phase: FilePhase;
  file?: FileDescriptor | null;
  error?: FileError | null;
}
export interface FileDescriptor {
  name: string;
  bytes: number;
  digest: string;
}
export interface FileCapture {
  owner: string;
  revision: number;
}
export interface ImageProps {
  source?: ImageDescriptor | null;
  packet?: ImagePacket | null;
  alt: string;
  fit: ImageFit;
  height: number;
  background: Color;
  border: Color;
  radius: number;
}
export interface ImageDescriptor {
  key: string;
  digest: string;
  mimeType: string;
  totalBytes: number;
}
export interface ImagePacket {
  owner: string;
  source: string;
  sequence: number;
  offset: number;
  data: string;
}
export interface ImageObservation {
  owner: string;
  revision: number;
  source?: string | null;
  received: number;
  phase: ImagePhase;
  width?: number | null;
  height?: number | null;
  error?: ImageError | null;
  receipt?: ImageReceipt | null;
}
export interface ImageReceipt {
  owner: string;
  source: string;
  sequence: number;
  accepted: boolean;
  error?: ImageError | null;
}
export interface SourceTextProps {
  source: string;
  appearance: TextAppearance;
  selection: Color;
  disabled: boolean;
}
export interface SelectionMenuProps {
  appearance: MenuAppearance;
  labels: SelectionMenuLabels;
  disabled: boolean;
}
export interface SelectionMenuLabels {
  title: string;
  copy: string;
  selectAll: string;
}
export interface ReadingProps {
  /**
   * The presenter's scene language; each requires a scrolling column Stack.
   * Its scroll policy and layout are the only presentation authority.
   */
  content: {
    [k: string]: unknown | undefined;
  };
  /**
   * Omission retains an accepted command; numbers increase per original mount.
   */
  command?: ReadingCommand | null;
}
export interface ReadingCommand {
  sequence: number;
  target: ReadingTarget;
}
export interface ReadingObservation {
  /**
   * Immediate native intent; can be newer than the painted viewport.
   */
  readingRevision: number;
  viewport?: ReadingViewport | null;
  pendingCommand?: number | null;
  /**
   * Only the most recently settled command is retained, without a ledger.
   */
  receipt?: ReadingReceipt | null;
}
export interface ReadingViewport {
  /**
   * Explicit reading intent used by the actually painted children.
   */
  readingRevision: number;
  atEnd: boolean;
  /**
   * @maxItems 4096
   */
  visibleChildKeys: string[];
}
export interface ReadingReceipt {
  sequence: number;
  status: ReadingStatus;
}
export interface RowsProps {
  revision: number;
  /**
   * Select this row on keyboard entry. None retains the current selection,
   * or selects the first row when there is no current selection.
   */
  entry?: string | null;
  /**
   * @maxItems 64
   */
  rows: Row[];
  /**
   * @maxItems 32
   */
  shortcuts: Shortcut[];
  maxRows: number;
  tabIndex: number;
  foreground: Color;
  muted: Color;
  selected: Color;
  hovered: Color;
  scrollbar: RowScrollbar;
}
export interface Row {
  key: string;
  text: string;
  /**
   * @maxItems 8
   */
  actions:
    | []
    | [RowAction]
    | [RowAction, RowAction]
    | [RowAction, RowAction, RowAction]
    | [RowAction, RowAction, RowAction, RowAction]
    | [RowAction, RowAction, RowAction, RowAction, RowAction]
    | [RowAction, RowAction, RowAction, RowAction, RowAction, RowAction]
    | [RowAction, RowAction, RowAction, RowAction, RowAction, RowAction, RowAction]
    | [RowAction, RowAction, RowAction, RowAction, RowAction, RowAction, RowAction, RowAction];
}
export interface RowAction {
  key: string;
  label: string;
  enabled: boolean;
}
export interface RowScrollbar {
  thumb: string;
  track: string;
  thumbColor: Color;
  trackColor: Color;
}
export interface RowActivated {
  revision: number;
  row: string;
  action: string;
}
export interface InputProps {
  appearance: InputAppearance;
  accessibilityLabel: string;
  readOnly: boolean;
  disabled: boolean;
  tabIndex: number;
  /**
   * Offer native focus once when this original input becomes visible and
   * enabled. Stable updates and viewport scrolling do not repeat the offer.
   */
  autoFocus?: boolean;
  placeholder?: string;
  command?: EditorCommand | null;
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
  /**
   * Installation-published directories, captured without creating workspace markers.
   *
   * @minItems 1
   * @maxItems 8
   */
  workspaceRoots:
    | [DirectoryRootSpec]
    | [DirectoryRootSpec, DirectoryRootSpec]
    | [DirectoryRootSpec, DirectoryRootSpec, DirectoryRootSpec]
    | [DirectoryRootSpec, DirectoryRootSpec, DirectoryRootSpec, DirectoryRootSpec]
    | [
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
      ]
    | [
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
      ]
    | [
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
      ]
    | [
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
        DirectoryRootSpec,
      ];
  maxActiveRuns: number;
  maxSteps: number;
  /**
   * Only explicitly selected, frozen application service methods become tools.
   *
   * @maxItems 128
   */
  tools: AgentServiceTool[];
}
export interface DirectoryRootSpec {
  label: string;
  path: string;
}
export interface AgentServiceTool {
  name: string;
  description: string;
  import: string;
  method: string;
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
  query: string;
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
export interface AgentCreate {
  title: string;
  /**
   * @maxItems 32
   */
  labels: string[];
  workspace: AgentWorkspaceSelection;
}
export interface AgentDirectorySelection {
  rootId: string;
  /**
   * @maxItems 64
   */
  segments: string[];
  directoryIdentity: string;
}
export interface AgentSession {
  id: string;
  title: string;
  labels: string[];
  workspace: AgentWorkspace;
  revision: number;
  updatedAt: number;
  archived: boolean;
}
export interface AgentWorkspace {
  path: string;
  directoryIdentity: string;
  identity: string;
}
export interface AgentDirectory {
  name: string;
  path: string;
  selection: AgentDirectorySelection;
}
export interface AgentMetadata {
  sessionId: string;
  expectedRevision: number;
  title: string;
  /**
   * @maxItems 32
   */
  labels: string[];
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
export interface AgentSessionPage {
  revision: string;
  /**
   * @maxItems 32
   */
  sessions: AgentSession[];
  before: string | null;
  after: string | null;
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
  position: HistoryPosition;
}
/**
 * Byte offset in one message's UTF-8 text, not a bearer capability.
 */
export interface Cursor {
  sequence: number;
  offset: number;
}
export interface AgentQueue {
  revision: number;
  /**
   * @maxItems 64
   */
  entries: AgentQueueEntry[];
}
export interface AgentQueueEntry {
  messageId: string;
  kind: AgentQueueKind;
  preview: string;
}
export interface AgentQueueMessage {
  sessionId: string;
  messageId: string;
  expectedRevision: number;
}
export interface AgentQueueMutation {
  sessionId: string;
  expectedRevision: number;
  edit: AgentQueueAction;
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
  tool: HistoryTool | null;
}
export interface AgentToolSourceRead {
  sessionId: string;
  through: number;
  sequence: number;
  field: ToolSourceField;
  position: SourcePosition;
}
/**
 * One literal UTF-8 excerpt of an immutable field. JSON excerpts need not be
 * standalone JSON. Image descriptors carry no artifact read authorization.
 */
export interface ToolSource {
  sequence: number;
  field: ToolSourceField;
  format: SourceFormat;
  offset: number;
  totalBytes: number;
  text: string;
  tool: HistoryTool;
}
export interface AgentToolImageRead {
  sessionId: string;
  through: number;
  sequence: number;
  offset: number;
}
/**
 * A stable raw-event fence prevents a window from chasing appended messages.
 */
export interface Read {
  sessionId: string;
  through?: number | null;
  position: HistoryPosition;
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
