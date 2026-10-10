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
  | FilePasteObservation
  | MarkdownProps
  | DiffProps
  | MarkdownLink
  | FilePickerProps
  | FileObservation
  | FileCapture
  | ImageProps
  | ImageObservation
  | SourceTextProps
  | TerminalViewProps
  | TerminalViewObservation
  | TerminalInputBatch
  | TerminalInputAcknowledgement
  | SelectionMenuProps
  | ReadingProps
  | ReadingObservation
  | RowsProps
  | ValueCheckpointProps
  | TextFileProps
  | TextFileObservation
  | MorphMarkProps
  | MorphMarkConfiguration
  | ValueCheckpointObservation
  | RowActivated
  | InputProps
  | EditorObservation
  | EditorCheckpointObservation
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
  | SkillLibraryConfiguration
  | SkillLibraryList
  | SkillLibraryPage
  | SkillLibraryRead
  | SkillLibraryDocument
  | SkillLibraryChange
  | SkillLibraryReview
  | SkillLibraryPreview
  | SkillLibraryChangeResult
  | SkillLibraryOutcome
  | SkillLibrarySetPreference
  | SkillLibraryPreferenceResult
  | SkillModelSearch
  | SkillModelLoad
  | SkillModelSearchResult
  | SkillModelLoadResult
  | SkillPrepare
  | SkillPreparation
  | SkillPreparationRecord
  | AgentList
  | AgentCreate
  | AgentCreated
  | AgentDirectories
  | AgentDirectoryPage
  | AgentMetadata
  | AgentMetadataResult
  | AgentSend
  | AgentStop
  | AgentUploadAttachment
  | AgentAttachment
  | AgentAttachmentUploadResult
  | AgentMarkRead
  | AgentMarkReadResult
  | AgentWatch
  | AgentPendingInteractions
  | AgentInteractionRead
  | AgentInteractionRespond
  | AgentInteractionResult
  | QuestionAsk
  | FormAsk
  | FormResponse
  | WorkspaceProcessConfiguration
  | TerminalList
  | TerminalTarget
  | TerminalStart
  | TerminalController
  | TerminalControl
  | TerminalPage
  | TerminalLeaseUpdate
  | TerminalControlReceipt
  | TerminalReleased
  | TerminalStopped
  | TerminalOutcome_TerminalPage
  | TerminalOutcome_ResourceUpdate
  | TerminalOutcome_TerminalLeaseUpdate
  | TerminalOutcome_TerminalControlReceipt
  | TerminalOutcome_TerminalReleased
  | TerminalOutcome_TerminalStopped
  | QuestionResult
  | InteractionRecord
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
  | AgentHistorySearch
  | AgentHistorySearchPage
  | AgentHistorySourceRead
  | AgentHistorySourcePage
  | AgentToolImageFrame
  | AgentToolImageRead
  | AgentToolImagePage
  | Read
  | HostActivation
  | HostRequest
  | HostObservation
  | OperationObservation
  | OperationReply
  | ResourceMetadata
  | MarkdownTextInput
  | MarkdownTextResult;
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
      /**
       * @maxItems 64
       */
      options: TerminalChoiceOption[];
      current?: string | null;
      disabled?: boolean;
      ascii?: boolean;
      foreground: TerminalColor;
      background: TerminalColor;
      border: TerminalColor;
      selected: TerminalColor;
      accent: TerminalColor;
      kind: 'choice';
    }
  | {
      key: string;
      content: TerminalScene;
      disabled?: boolean;
      current?: boolean;
      /**
       * Current-row fill; absent uses the terminal's reverse-video selection.
       */
      selected_color?: TerminalColor | null;
      selected_foreground?: TerminalColor | null;
      follow_focus?: boolean;
      submit?: boolean;
      /**
       * Emit submit for physical Space; focus and pointer activation stay separate.
       */
      submit_space?: boolean;
      kind: 'item';
    }
  | {
      key: string;
      /**
       * Native Copy selection entry; absent omits selection capture.
       */
      copy_selection_label?: string | null;
      background: TerminalScene;
      content: TerminalScene;
      ascii?: boolean;
      label: string;
      /**
       * Maximum foreground body dimensions, in terminal cells.
       */
      width: number;
      height: number;
      background_color: TerminalColor;
      border_color: TerminalColor;
      title_color?: TerminalColor | null;
      padding?: number;
      kind: 'context_menu';
    }
  | {
      key: string;
      background: TerminalScene;
      content: TerminalScene;
      open: boolean;
      ascii?: boolean;
      label: string;
      /**
       * Maximum foreground body dimensions, in terminal cells.
       */
      width: number;
      height: number;
      background_color: TerminalColor;
      border_color: TerminalColor;
      title_color?: TerminalColor | null;
      padding?: number;
      kind: 'dialog';
    }
  | {
      key: string;
      background: TerminalScene;
      content: TerminalScene;
      pane_width: number;
      inline_min_width: number;
      auto_inline_min_width: number;
      overlay: TerminalDialogAppearance;
      motion: boolean;
      command?: TerminalPaneCommand | null;
      kind: 'adaptive_pane';
    }
  | {
      key: string;
      direction: TerminalDirection;
      wide?: TerminalResponsiveStack | null;
      gap: number;
      padding: number;
      /**
       * Layout-only stacks inherit their parent; an explicit color fills the area.
       */
      background?: TerminalColor | null;
      children: TerminalScene[];
      /**
       * One cell outside padding; absent means no border or border inset.
       */
      border?: TerminalBorder | null;
      /**
       * Establish the allocated inner size as the descendants' layout viewport.
       * Children do not contribute intrinsic size; use grow/basis or an
       * externally allocated root area to supply this container's size.
       */
      viewport?: boolean;
      /**
       * One Tab stop; arrows navigate its items in declaration order.
       */
      focus_group?: boolean;
      /**
       * On structural reveal, focus the first visible enabled tab stop;
       * a scrollable container without tab stops receives focus itself.
       */
      auto_focus?: boolean;
      wheel?: TerminalWheelCommands | null;
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
      color: TerminalColor;
      /**
       * Render a single line, eliding overflow at a complete grapheme.
       */
      truncate?: boolean;
      /**
       * @maxItems 64
       */
      animation?: TerminalAnimationFrame[];
      align?: TerminalAlign & string;
      bold?: boolean;
      kind: 'text';
    }
  | {
      key: string;
      label: string;
      /**
       * Label alignment within the button hit area; absent centers it.
       */
      align?: TerminalAlign | null;
      hint?: string;
      /**
       * Enter a declared child's native focus target instead of emitting a click.
       */
      focus_child?: string | null;
      /**
       * Copy this committed text as the physical button's default action.
       * No clipboard access occurs from a synthetic component dispatch.
       */
      copy_text?: string | null;
      disabled?: boolean;
      /**
       * Absent foreground uses the terminal's default text color.
       */
      color?: TerminalColor | null;
      /**
       * Fill the whole button hit area at rest; absent inherits its parent.
       */
      background?: TerminalColor | null;
      /**
       * Hover lifts the fill without selecting or focusing the button.
       */
      hover_background?: TerminalColor | null;
      /**
       * Emphasize the label and hit area independently of keyboard focus.
       */
      bold?: boolean;
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
/**
 * Terminal colors retain the user's ANSI palette and default background.
 * Numeric values are RGB; named values are emitted as terminal attributes.
 */
export type TerminalColor = number | TerminalNamedColor;
export type TerminalNamedColor =
  | 'reset'
  | 'black'
  | 'red'
  | 'green'
  | 'yellow'
  | 'blue'
  | 'magenta'
  | 'cyan'
  | 'gray'
  | 'dark_gray'
  | 'light_red'
  | 'light_green'
  | 'light_yellow'
  | 'light_blue'
  | 'light_magenta'
  | 'light_cyan'
  | 'white';
export type TerminalPaneOperation = 'toggle' | 'dismiss_transient';
export type TerminalDirection = 'row' | 'column';
export type TerminalBorderStyle = 'rounded' | 'square' | 'ascii';
export type Modifier = 'control' | 'primary' | 'alt' | 'shift' | 'super';
/**
 * Native scroll policy; offsets and accepted viewport geometry remain native.
 */
export type TerminalScrollPolicy = 'none' | 'manual' | 'anchor' | 'follow_end';
export type TerminalAlign = 'start' | 'center' | 'end';
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
      /**
       * Physical native activation copies this frozen text after an accepted
       * click whose default action was not prevented.
       */
      copy_text?: string | null;
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
export type FilePasteAction = 'next' | 'retry' | 'cancel';
export type FilePhase = 'empty' | 'choosing' | 'ready' | 'failed';
export type FileError =
  | 'invalid_file'
  | 'read_failed'
  | 'limit_exceeded'
  | 'budget_exhausted'
  | 'worker_failed'
  | 'clipboard_unavailable'
  | 'paste_rejected';
/**
 * Complete bounded comparisons and structured patches reuse native parsers.
 */
export type DiffContent =
  | {
      source: string;
      /**
       * @maxItems 8192
       */
      rows: DiffRow[];
      kind: 'annotated';
    }
  | {
      before: string;
      after: string;
      language?: string | null;
      kind: 'comparison';
    }
  | {
      source: string;
      operation: DiffPatch;
      language?: string | null;
      kind: 'patch';
    };
export type DiffKind = 'removed' | 'added' | 'context' | 'content';
export type DiffPatch = 'create' | 'update';
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
export type ShellOutput =
  | {
      stdout: string;
      stderr: string;
      latestStream?: PipeStream;
      stdoutTruncated: boolean;
      stderrTruncated: boolean;
      redacted: boolean;
      mode: 'pipes';
    }
  | {
      screen: string;
      scrollback: string;
      lastAlternateScreen?: string;
      cols: number;
      rows: number;
      cursor: TerminalCursor;
      alternateScreen: boolean;
      truncated: boolean;
      redacted: boolean;
      mode: 'pty';
    };
export type PipeStream = 'stdout' | 'stderr';
export type RawAction =
  | {
      text: string;
      type: 'text';
    }
  | {
      text: string;
      type: 'paste';
    }
  | {
      key: string;
      modifiers?: TerminalModifiers;
      type: 'key';
    }
  | {
      event: RawEvent;
      x: number;
      y: number;
      button?: MouseButton;
      direction?: ScrollDirection;
      modifiers?: TerminalModifiers;
      type: 'mouse';
    };
/**
 * @maxItems 3
 */
export type TerminalModifiers =
  | []
  | [Modifier2]
  | [Modifier2, Modifier2]
  | [Modifier2, Modifier2, Modifier2];
export type Modifier2 = 'ctrl' | 'alt' | 'shift';
export type RawEvent = 'click' | 'press' | 'release' | 'move' | 'scroll';
export type MouseButton = 'left' | 'middle' | 'right';
export type ScrollDirection = 'up' | 'down';
export type TerminalViewError = 'input_full' | 'input_failed' | 'budget_exhausted';
export type ReadingTarget =
  | {
      kind: 'start';
    }
  | {
      kind: 'end';
    }
  | {
      query: string;
      step: FindStep;
      kind: 'find';
    }
  | {
      kind: 'clear_find';
    }
  | {
      rows: number;
      kind: 'scroll_rows';
    }
  | {
      /**
       * @minItems 1
       * @maxItems 32
       */
      path: [string, ...string[]];
      offset: number;
      /**
       * If supplied, reveal only while this accepted find retains its original physical intent.
       */
      find_sequence?: number | null;
      kind: 'reveal_source';
    }
  | {
      key: string;
      kind: 'focus_item';
    }
  | {
      key: string;
      kind: 'reveal_child';
    };
export type FindStep = 'first' | 'next' | 'previous';
export type ReadingScrollDirection = 'up' | 'down';
export type ReadingStatus = 'applied' | 'cancelled' | 'rejected';
export type ValueCheckpointCommand =
  | {
      sequence: number;
      kind: 'read';
    }
  | {
      sequence: number;
      expectedRevision?: number | null;
      value: unknown;
      kind: 'write';
    };
export type TextFileOperation =
  | {
      kind: 'read';
    }
  | {
      expectedDigest?: string | null;
      text: string;
      kind: 'write';
    };
export type TextFileState =
  | {
      kind: 'unavailable';
    }
  | {
      kind: 'idle';
    }
  | {
      kind: 'loading';
    }
  | {
      kind: 'saving';
    }
  | {
      digest?: string | null;
      text?: string | null;
      kind: 'ready';
    }
  | {
      error: TextFileError;
      kind: 'failed';
    };
export type TextFileError =
  | 'read_failed'
  | 'write_failed'
  | 'invalid_file'
  | 'too_large'
  | 'invalid_utf8'
  | 'conflict'
  | 'worker_failed';
export type ValueCheckpointState =
  | {
      kind: 'loading';
    }
  | {
      kind: 'saving';
    }
  | {
      revision?: number | null;
      value?: unknown;
      kind: 'ready';
    }
  | {
      diagnostic: string;
      kind: 'failed';
    };
export type TerminalPaste = 'reject_controls' | 'spaces';
export type EditorOutcome =
  | {
      status: 'applied';
    }
  | {
      reason: EditorConflict;
      status: 'conflict';
    };
export type EditorConflict =
  | 'persistence'
  | 'owner'
  | 'representation'
  | 'revision'
  | 'composition'
  | 'sequence';
export type EditorCheckpointState =
  | {
      kind: 'loading';
    }
  | {
      version: number;
      snapshot: EditorSnapshot;
      metadata: unknown;
      kind: 'restore';
    }
  | {
      saved: boolean;
      initialized: boolean;
      kind: 'ready';
    }
  | {
      diagnostic: string;
      kind: 'failed';
    };
export type EditorCapture =
  | {
      kind: 'current';
    }
  | {
      owner: string;
      revision: number;
      kind: 'at_revision';
    };
/**
 * Access inside the original selected directory; never an ambient filesystem grant.
 */
export type AgentWorkspaceAccess = 'read_only' | 'workspace_write';
export type AgentToolInput = 'json' | 'skill';
export type AgentToolResult = 'json' | 'image';
export type AgentToolConfirmation = 'required' | 'preapproved';
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
export type ThinkingLevel =
  | 'off'
  | 'minimal'
  | 'low'
  | 'medium'
  | 'high'
  | 'xhigh'
  | 'max'
  | 'ultra';
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
export type SkillLibraryPage =
  | {
      writable: boolean;
      preferencesWritable: boolean;
      preferenceRevision: number;
      revision: string;
      entries: SkillLibraryEntry[];
      after?: SkillLibraryCursor | null;
      diagnostics: string[];
      kind: 'page';
    }
  | {
      kind: 'changed';
    };
export type SkillLibraryDocument =
  | {
      writable: boolean;
      preferencesWritable: boolean;
      preferenceRevision: number;
      entry: SkillLibraryEntry;
      revision: string;
      text: string;
      offset: number;
      before?: number | null;
      after?: number | null;
      totalBytes: number;
      kind: 'page';
    }
  | {
      kind: 'changed';
    }
  | {
      kind: 'unavailable';
    };
export type SkillLibraryChangeKind = 'install' | 'update' | 'delete';
export type SkillLibraryPreview =
  | {
      review: SkillLibraryReview;
      target: string;
      entries: number;
      paths: string[];
      pathsTruncated: boolean;
      locallyModified: boolean;
      kind: 'ready';
    }
  | {
      kind: 'changed';
    }
  | {
      reason: string;
      kind: 'rejected';
    };
export type SkillLibraryChangeResult =
  | {
      reference: string;
      kind: 'applied';
    }
  | {
      kind: 'changed';
    }
  | {
      reason: string;
      kind: 'rejected';
    };
export type SkillLibraryPreferenceResult =
  | {
      revision: number;
      kind: 'applied';
    }
  | {
      kind: 'changed';
    }
  | {
      reason: string;
      kind: 'rejected';
    };
export type SkillScope = 'project' | 'workspace' | 'user' | 'custom';
export type SkillSource = 'maka' | 'agents' | 'legacy' | 'custom';
export type SkillModelLoadResult =
  | {
      revision: string;
      metadata: SkillMetadata;
      instructions: string;
      instructionOffset: number;
      next?: SkillModelLoad | null;
      relativePath: string;
      declaredTools: string[];
      declaredToolsTruncated: boolean;
      truncated: boolean;
      receipt: SkillInvocationReceipt;
      kind: 'loaded';
      [k: string]: unknown | undefined;
    }
  | {
      revision: string;
      reference: string;
      path: string;
      content: string;
      offset: number;
      returnedLines: number;
      totalLines: number;
      partialLine: boolean;
      next?: SkillModelLoad | null;
      receipt: SkillInvocationReceipt;
      kind: 'resource';
      [k: string]: unknown | undefined;
    }
  | {
      kind: 'changed';
      [k: string]: unknown | undefined;
    }
  | {
      reason: SkillFailureReason;
      receipt: SkillInvocationReceipt;
      kind: 'unavailable';
      [k: string]: unknown | undefined;
    }
  | {
      reason: string;
      kind: 'resource_unavailable';
      [k: string]: unknown | undefined;
    };
/**
 * Boolean wire discriminants are validated here, not retained as mutable
 * business fields beside a contradictory enum variant.
 */
export type SkillInvocationReceipt =
  | {
      invocation: SkillInvocationMode;
      request: string;
      success: true;
      ref: string;
      id: string;
      name: string;
      scope: SkillScope;
      source: SkillSource;
      truncated: boolean;
    }
  | {
      invocation: SkillInvocationMode;
      request: string;
      success: false;
      reason: SkillFailureReason;
    }
  | {
      invocation: 'explicit';
      success: false;
      reason: TooManyRequests;
      requestLimit: number;
    };
export type SkillInvocationMode = 'explicit' | 'model_tool';
export type SkillFailureReason =
  | 'invalid_name'
  | 'not_found'
  | 'disabled'
  | 'host_incompatible'
  | 'resolution_failed';
export type TooManyRequests = 'too_many_requests';
export type SkillPreparation =
  | {
      /**
       * None preserves the original message exactly.
       */
      text?: string | null;
      record: SkillPreparationRecord;
      requiredTools: string[];
      kind: 'ready';
    }
  | {
      invocation: SkillInvocationResult;
      kind: 'blocked';
    }
  | {
      kind: 'input_too_large';
    };
export type SkillInvocationFailure = SkillRequestFailure | SkillOverflowFailure;
export type AgentSessionFilter = 'all' | 'needs_you';
export type AgentSessionPosition =
  | {
      kind: 'start';
    }
  | {
      cursor: string;
      kind: 'at';
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
export type AgentSessionActivity = 'idle' | 'working' | 'waiting';
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
export type AgentAttachmentUploadResult =
  | {
      attachment: AgentAttachment;
      kind: 'uploaded';
    }
  | {
      reason: AgentAttachmentRejection;
      kind: 'rejected';
    };
export type AgentAttachmentRejection = 'invalid_file' | 'busy' | 'closed' | 'archived';
export type AgentMarkReadResult =
  | {
      session: AgentSession;
      kind: 'observed';
    }
  | {
      reason: AgentRejection;
      kind: 'rejected';
    };
export type AgentInteractionKind =
  | 'service_approval'
  | 'question'
  | 'permissions'
  | 'form'
  | 'client_capability';
export type InteractionAnswer =
  | {
      decision: Decision;
      kind: 'service_approval';
    }
  | {
      decision: Decision2;
      kind: 'permissions';
    }
  | {
      answers: (string | null)[];
      kind: 'question';
    }
  | ((
      | {
          values: {
            [k: string]: FormValue | undefined;
          };
          action: 'accept';
          [k: string]: unknown | undefined;
        }
      | {
          action: 'decline';
          [k: string]: unknown | undefined;
        }
      | {
          action: 'cancel';
          [k: string]: unknown | undefined;
        }
    ) & {
      kind: 'form';
      [k: string]: unknown | undefined;
    })
  | {
      decision: Decision;
      kind: 'client_capability';
    };
export type Decision = 'allow' | 'deny';
export type Decision2 =
  | {
      decision: 'deny';
    }
  | {
      permissions: Permissions;
      scope: Scope2;
      decision: 'allow';
    };
export type Scope = 'subtree' | 'exact';
/**
 * Equal-path conflict priority is different from authority breadth: deny wins.
 */
export type Access = 'read' | 'write' | 'deny';
export type Network =
  | ('denied' | 'allowed')
  | {
      restricted: {
        /**
         * @maxItems 128
         */
        destinations: Destination[];
      };
    };
export type Scope2 = 'once' | 'turn' | 'session';
export type FormValue = string | number | boolean | string[];
export type AgentInteractionResult =
  | {
      record: InteractionRecord;
      kind: 'recorded';
    }
  | {
      reason: AgentInteractionRejection;
      kind: 'rejected';
    };
export type InteractionRequest =
  | {
      toolUseId: string;
      toolName: string;
      service: string;
      method: string;
      binding: string;
      through: number;
      kind: 'service_approval';
    }
  | {
      toolUseId: string | null;
      baseRevision: number;
      request: PermissionRequest;
      kind: 'permissions';
    }
  | {
      toolUseId: string;
      questions: InteractionQuestion[];
      kind: 'question';
    }
  | {
      toolUseId: string;
      message: string;
      requester: FormRequester;
      fields: FormField[];
      kind: 'form';
    }
  | {
      toolUseId: string;
      target: GrantTarget;
      kind: 'client_capability';
    };
export type FormField = {
  name: string;
  label: string;
  required: boolean;
  description?: string | null;
  [k: string]: unknown | undefined;
} & FormField1;
export type FormField1 =
  | {
      default?: string | null;
      minLength?: number | null;
      maxLength?: number | null;
      format?: FormFormat | null;
      kind: 'string';
      [k: string]: unknown | undefined;
    }
  | {
      default?: number | null;
      minimum?: number | null;
      maximum?: number | null;
      kind: 'number';
      [k: string]: unknown | undefined;
    }
  | {
      default?: number | null;
      minimum?: number | null;
      maximum?: number | null;
      kind: 'integer';
      [k: string]: unknown | undefined;
    }
  | {
      default?: boolean | null;
      kind: 'boolean';
      [k: string]: unknown | undefined;
    }
  | {
      options: FormOption[];
      default?: string | null;
      kind: 'single_select';
      [k: string]: unknown | undefined;
    }
  | {
      options: FormOption[];
      default?: string[] | null;
      minItems?: number | null;
      maxItems?: number | null;
      kind: 'multi_select';
      [k: string]: unknown | undefined;
    };
export type FormFormat = 'email' | 'uri' | 'date' | 'date-time';
export type GrantCapability = 'browser' | 'computer_use' | 'desktop_mcp' | 'mcp';
export type GrantScope =
  | {
      origin: string;
      kind: 'browser_origin';
    }
  | {
      kind: 'capability';
    }
  | {
      serverId: string;
      toolName: string;
      kind: 'mcp_tool';
    };
export type InteractionOutcome =
  | {
      decision: Decision;
      committedAt: number;
      kind: 'service_approval_decision';
    }
  | {
      decision: Decision2;
      committedAt: number;
      kind: 'permissions_decision';
    }
  | {
      answers: (string | null)[];
      committedAt: number;
      kind: 'question_answer';
    }
  | ((
      | {
          values: {
            [k: string]: FormValue | undefined;
          };
          action: 'accept';
          [k: string]: unknown | undefined;
        }
      | {
          action: 'decline';
          [k: string]: unknown | undefined;
        }
      | {
          action: 'cancel';
          [k: string]: unknown | undefined;
        }
    ) & {
      committedAt: number;
      kind: 'form_answer';
      [k: string]: unknown | undefined;
    })
  | {
      decision: Decision;
      committedAt: number;
      kind: 'client_capability_decision';
    }
  | {
      reason: ClosureReason;
      committedAt: number;
      kind: 'closure';
    };
export type ClosureReason =
  | 'turn_stopped'
  | 'turn_terminal'
  | 'producer_cancelled'
  | 'timed_out'
  | 'host_restarted'
  | 'provider_disconnected';
export type AgentInteractionRejection = 'closed' | 'unavailable' | 'invalid_answer' | 'unsupported';
export type FormResponse =
  | {
      result: FormResult;
      kind: 'answered';
    }
  | {
      reason: ClosureReason;
      kind: 'closed';
    }
  | {
      reason: QuestionRejection;
      kind: 'rejected';
    };
export type FormResult =
  | {
      values: {
        [k: string]: FormValue | undefined;
      };
      action: 'accept';
      [k: string]: unknown | undefined;
    }
  | {
      action: 'decline';
      [k: string]: unknown | undefined;
    }
  | {
      action: 'cancel';
      [k: string]: unknown | undefined;
    };
export type QuestionRejection = 'invalid_input' | 'unavailable';
export type Ownership =
  | {
      kind: 'local';
    }
  | {
      sourceSessionId: string;
      ownerSessionId: string;
      kind: 'source_owned';
    }
  | {
      sourceSessionId: string;
      kind: 'source_unavailable';
    };
export type SnapshotKind = 'shell_run';
export type ShellMode = 'pipes' | 'pty';
export type ShellStatus =
  | 'starting'
  | 'running'
  | 'completed'
  | 'failed'
  | 'timed_out'
  | 'cancelled'
  | 'orphaned';
export type TerminalOutcome_TerminalPage =
  | {
      value: TerminalPage;
      kind: 'completed';
    }
  | {
      message: string;
      kind: 'failed';
    };
export type TerminalOutcome_ResourceUpdate =
  | {
      value: ResourceUpdate;
      kind: 'completed';
    }
  | {
      message: string;
      kind: 'failed';
    };
export type TerminalOutcome_TerminalLeaseUpdate =
  | {
      value: TerminalLeaseUpdate;
      kind: 'completed';
    }
  | {
      message: string;
      kind: 'failed';
    };
export type TerminalOutcome_TerminalControlReceipt =
  | {
      value: TerminalControlReceipt;
      kind: 'completed';
    }
  | {
      message: string;
      kind: 'failed';
    };
export type TerminalOutcome_TerminalReleased =
  | {
      value: TerminalReleased;
      kind: 'completed';
    }
  | {
      message: string;
      kind: 'failed';
    };
export type TerminalOutcome_TerminalStopped =
  | {
      value: TerminalStopped;
      kind: 'completed';
    }
  | {
      message: string;
      kind: 'failed';
    };
export type QuestionResult =
  | {
      answers: QuestionResponse[];
      kind: 'answered';
    }
  | {
      reason: ClosureReason;
      kind: 'closed';
    }
  | {
      reason: QuestionRejection;
      kind: 'rejected';
    };
export type AgentSendResult =
  | {
      sessionId: string;
      reason: AgentSkillInputRejection;
      skills?: SkillInvocationResult | null;
      kind: 'rejected';
    }
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
export type AgentSkillInputRejection = 'skills_unavailable' | 'input_too_large';
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
/**
 * A readable field in a conversation window, in presentation order.
 */
export type HistoryField = 'thinking' | 'body';
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
    }
  | {
      pending: AgentPendingInteractions;
      kind: 'interactions';
    };
export type AgentQueueKind = 'in_flight' | 'steering' | 'followup';
export type AgentQueueMessageResult =
  | {
      revision: number;
      hasAttachments: boolean;
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
export type HistoryTurnOutcome = 'completed' | 'failed' | 'aborted';
/**
 * Canonical message author/kind; the field identifies body versus thinking.
 * Tool body JSON contributes string values, never its keys.
 */
export type Role = 'user' | 'assistant' | 'tool_call' | 'tool_result';
/**
 * Recorded Host settlement for presentation; returned is not business success.
 * Foreign/provider results do not establish this Host evidence.
 */
export type ToolSettlement = 'returned' | 'failed' | 'unknown' | 'rejected';
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
export type AgentHistorySearchPage =
  | {
      through: number;
      kind: 'preparing';
    }
  | {
      through: number;
      /**
       * @maxItems 16
       */
      matches:
        | []
        | [AgentHistoryMatch]
        | [AgentHistoryMatch, AgentHistoryMatch]
        | [AgentHistoryMatch, AgentHistoryMatch, AgentHistoryMatch]
        | [AgentHistoryMatch, AgentHistoryMatch, AgentHistoryMatch, AgentHistoryMatch]
        | [
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
          ]
        | [
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
          ]
        | [
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
          ]
        | [
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
          ]
        | [
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
          ]
        | [
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
          ]
        | [
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
          ]
        | [
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
          ]
        | [
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
          ]
        | [
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
          ]
        | [
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
          ]
        | [
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
            AgentHistoryMatch,
          ];
      /**
       * Continue even when this batch has no matches. Null is the end of the scan.
       */
      nextAfter: number | null;
      kind: 'ready';
    };
/**
 * An explicitly selected canonical transcript field, never an execution grant.
 */
export type SourceField =
  | ('body' | 'thinking' | 'tool_name' | 'failure_message' | 'error_class' | 'arguments' | 'result')
  | 'receipt';
export type AgentHistorySourceReference =
  | {
      sequence: number;
      kind: 'sequence';
    }
  | {
      toolUseId: string;
      kind: 'tool_use';
    };
/**
 * A structural selector into an original JSON field. Keys and array indices
 * remain distinct; neither locates a filesystem resource nor grants access.
 */
export type HistorySourceSegment = string | number;
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
export type AgentHistorySourcePage =
  | {
      through: number;
      kind: 'preparing';
    }
  | {
      through: number;
      source: HistorySource;
      /**
       * Complete file-path evidence resolved on the Host for copying only.
       * This is display text, never a filesystem capability or open command.
       */
      filePath?: string | null;
      kind: 'ready';
    };
/**
 * Presentation role of the exact row, including diagnostics omitted from the
 * ordinary conversation window. This carries no execution authority.
 */
export type HistorySourceRole = 'user' | 'assistant' | 'tool_call' | 'tool_result' | 'failure';
export type SourceFormat = 'text' | 'json' | 'image_descriptor';
/**
 * Bounded image bytes from a selected subscription. Complete is a producer
 * return value, never an emitted item or an executor-cleanup acknowledgement.
 */
export type AgentToolImageFrame =
  | {
      mimeType: string;
      totalBytes: number;
      offset: number;
      data: string;
      kind: 'chunk';
    }
  | {
      kind: 'complete';
    };
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
export type MarkdownTextResult =
  | {
      text: string;
      kind: 'ready';
    }
  | {
      kind: 'too_large';
    };

/**
 * Durable deployment input. Plans are derived and never persisted as overrides.
 */
export interface CompositionInput {
  environment: Environment;
  instances: {
    [k: string]: CompositionNode | undefined;
  };
  launch?: Target | null;
  /**
   * Explicit ordinary fallback. Selection grants no extra capabilities.
   */
  recovery?: Target | null;
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
  recovery?: unknown;
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
export interface TerminalChoiceOption {
  key: string;
  label: string;
}
/**
 * Appearance shared by controlled dialogs and transient adaptive panes.
 */
export interface TerminalDialogAppearance {
  ascii?: boolean;
  label: string;
  /**
   * Maximum foreground body dimensions, in terminal cells.
   */
  width: number;
  height: number;
  background_color: TerminalColor;
  border_color: TerminalColor;
  title_color?: TerminalColor | null;
  padding?: number;
}
/**
 * A one-shot mechanical command. Repeated or older sequences are ignored.
 * Toggle uses the last successfully placed viewport; before first placement it
 * is consumed without changing state. Dismiss never changes inline preference.
 */
export interface TerminalPaneCommand {
  sequence: number;
  op: TerminalPaneOperation;
}
/**
 * Rearrange the same direct children at an allocated inner-width breakpoint.
 * Child overrides affect allocation in this stack, not a child's own wrapping.
 */
export interface TerminalResponsiveStack {
  min_width: number;
  direction: TerminalDirection;
  children?: {
    [k: string]: TerminalLayout | undefined;
  };
}
/**
 * Native terminal cell constraints on a child in its parent's main axis.
 * `grow` distributes remaining cells after fixed/intrinsic siblings; `basis`
 * supplies an explicit base instead of intrinsic length. `shrink` allows that
 * base to shrink proportionally when siblings exceed the available cells.
 * Row wrap preserves
 * source order and measures its rows before allocating the parent viewport.
 */
export interface TerminalLayout {
  grow?: number;
  basis?: number | null;
  basis_fraction?: TerminalFractionalBasis | null;
  shrink?: boolean;
  wrap?: boolean;
}
/**
 * Fraction of the allocated main axis, rounded down and clamped before growth
 * or shrinking. An unbounded axis uses the intrinsic length with the same bounds.
 */
export interface TerminalFractionalBasis {
  numerator: number;
  denominator: number;
  min?: number;
  max?: number | null;
}
/**
 * A one-cell frame. Colors follow the same RGB convention as terminal text.
 * Focus color follows any focused native target inside this stack.
 */
export interface TerminalBorder {
  breathing?: boolean;
  color: TerminalColor;
  focused_color: TerminalColor;
  style: TerminalBorderStyle;
}
/**
 * Explicit wheel actions in an original scrolling viewport. Offsets remain native.
 */
export interface TerminalWheelCommands {
  up: string;
  down: string;
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
export interface TerminalAnimationFrame {
  text: string;
  millis: number;
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
  recovery?: unknown;
}
export interface LabelProps {
  text: string;
  appearance: TextAppearance;
  terminalForeground?: TerminalColor | null;
  elapsed?: Elapsed | null;
}
export interface Elapsed {
  from: number;
  to: number | null;
}
export interface TextareaProps {
  appearance: TextareaAppearance;
  menu?: TextareaMenu | null;
  accessibilityLabel: string;
  readOnly: boolean;
  disabled: boolean;
  softWrap: boolean;
  /**
   * Preferred terminal text rows, excluding native padding. A constrained
   * viewport may shrink further. Desktop does not use terminal row sizing.
   */
  terminalRows?: TerminalTextareaRows;
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
  /**
   * Opt-in local file-path paste. The context is returned unchanged; it is
   * correlation, never a filesystem path or an authorization grant.
   */
  filePaste?: FilePastePolicy | null;
  fileCommand?: FilePasteCommand | null;
  persistence?: EditorPersistence | null;
}
export interface TextareaAppearance {
  ascii?: boolean;
  terminal?: TerminalPalette | null;
  text: TextAppearance;
  mutedForeground: Color;
  background: Color;
  selection: Color;
  caret: Color;
  border: Color;
  focusedBorder: Color;
  /**
   * In TUI inputs, a positive width draws a one-cell border; zero has no inset.
   */
  borderWidth: number;
  radius: number;
  padding: Insets;
  scrollbar: ScrollbarAppearance;
}
/**
 * Semantic terminal colors for rich text and editors, supplied by the product.
 */
export interface TerminalPalette {
  background: TerminalColor;
  foreground: TerminalColor;
  surface: TerminalColor;
  accent: TerminalColor;
  thinking: TerminalColor;
  success: TerminalColor;
  warning: TerminalColor;
  error: TerminalColor;
  muted: TerminalColor;
  subtle: TerminalColor;
  border: TerminalColor;
  scrollbar: TerminalColor;
  selection: TerminalColor;
  selectionText: TerminalColor;
  search: TerminalColor;
  searchActive: TerminalColor;
  searchText: TerminalColor;
  /**
   * @minItems 7
   * @maxItems 7
   */
  syntax: [
    TerminalColor,
    TerminalColor,
    TerminalColor,
    TerminalColor,
    TerminalColor,
    TerminalColor,
    TerminalColor,
  ];
  terminal: boolean;
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
export interface TerminalTextareaRows {
  min: number;
  max: number;
  /**
   * Optional cap relative to the nearest declared layout viewport's height,
   * or the surface when none is declared. The minimum still applies.
   */
  maxViewportFraction?: number | null;
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
export interface FilePastePolicy {
  context: string;
  /**
   * Zero still permits clipboard text, but no new file attachments.
   */
  maxFiles: number;
}
export interface FilePasteCommand {
  selection: FileCapture;
  action: FilePasteAction;
}
export interface FileCapture {
  owner: string;
  revision: number;
}
export interface EditorPersistence {
  key: string;
  format: string;
  dataSource: unknown;
  metadata: unknown;
  restore?: EditorRestoration | null;
  retry?: number;
}
export interface EditorRestoration {
  owner: string;
  version: number;
  clearText: boolean;
}
export interface FilePasteObservation {
  context: string;
  selection: FileObservation;
  /**
   * Includes the currently reading, ready or failed item.
   */
  remaining: number;
  /**
   * A failed original path can be reread. Clipboard reads require a new gesture.
   */
  retryable: boolean;
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
export interface MarkdownProps {
  ascii?: boolean;
  animate?: boolean;
  source: string;
  appearance: MarkdownAppearance;
  selectable: boolean;
  disabled: boolean;
  /**
   * Whether pointer selection also moves keyboard focus to this document.
   */
  focusOnPointer?: boolean;
}
export interface MarkdownAppearance {
  terminal?: TerminalPalette | null;
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
export interface DiffProps {
  content: DiffContent;
  appearance: DiffAppearance;
  ascii: boolean;
  disabled: boolean;
  focusOnPointer: boolean;
}
export interface DiffRow {
  /**
   * Half-open UTF-8 byte range in source. Rows are ordered and disjoint.
   */
  start: number;
  end: number;
  kind: DiffKind;
  language?: string | null;
}
export interface DiffAppearance {
  terminal?: TerminalPalette | null;
  text: TextAppearance;
  background: Color;
  added: Color;
  removed: Color;
  selection: Color;
  dark: boolean;
}
export interface MarkdownLink {
  url: string;
}
export interface FilePickerProps {
  appearance: InputAppearance;
  label: string;
  pathLabel: string;
  browseLabel: string;
  parentLabel: string;
  emptyLabel: string;
  truncatedLabel: string;
  placeholder: string;
  disabled: boolean;
  tabIndex: number;
}
export interface InputAppearance {
  ascii?: boolean;
  terminal?: TerminalPalette | null;
  text: TextAppearance;
  mutedForeground: Color;
  background: Color;
  selection: Color;
  caret: Color;
  border: Color;
  focusedBorder: Color;
  /**
   * In TUI inputs, a positive width draws a one-cell border; zero has no inset.
   */
  borderWidth: number;
  radius: number;
  padding: Insets;
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
  animate?: boolean;
  terminal?: TerminalPalette | null;
  source: string;
  appearance: TextAppearance;
  selection: Color;
  disabled: boolean;
  /**
   * Whether pointer selection also moves keyboard focus to this source.
   */
  focusOnPointer?: boolean;
}
export interface TerminalViewProps {
  /**
   * Changed when the selected canonical resource changes, including read-only views.
   */
  source: string;
  accessibilityLabel: string;
  output?: ShellOutput | null;
  controller?: string | null;
  acknowledgement?: TerminalInputAcknowledgement | null;
  appearance: TextAppearance;
  background: Color;
  caret: Color;
  rows: number;
  tabIndex: number;
}
export interface TerminalCursor {
  x: number;
  y: number;
  visible: boolean;
}
export interface TerminalInputAcknowledgement {
  owner: string;
  controller: string;
  sequence: number;
  /**
   * True only after the exact original mutation has a successful result.
   */
  accepted: boolean;
}
export interface TerminalViewObservation {
  owner: string;
  revision: number;
  source: string;
  controller?: string | null;
  captured: boolean;
  batch?: TerminalInputBatch | null;
  error?: TerminalViewError | null;
}
export interface TerminalInputBatch {
  controller: string;
  sequence: number;
  /**
   * @maxItems 64
   */
  actions: RawAction[];
  size?: TerminalSize | null;
}
/**
 * Shared by native PTY allocation, input validation and screen rendering.
 */
export interface TerminalSize {
  cols: number;
  rows: number;
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
  scroll?: ReadingScroll | null;
  /**
   * Direct content child containing the native selection focus. Renderers
   * without this observation return null. This key conveys no read authority.
   */
  selectedChildKey?: string | null;
  pendingCommand?: number | null;
  /**
   * Only the most recently settled command is retained, without a ledger.
   */
  receipt?: ReadingReceipt | null;
  find?: ReadingFind | null;
}
export interface ReadingViewport {
  /**
   * Explicit reading intent used by the actually painted children.
   */
  readingRevision: number;
  atStart?: boolean;
  atEnd: boolean;
  /**
   * @maxItems 4096
   */
  visibleChildKeys: string[];
}
/**
 * Most recent physical scroll. Layout changes and programmatic landings do not
 * create a new request, so a product can load adjacent content without loops.
 */
export interface ReadingScroll {
  readingRevision: number;
  direction: ReadingScrollDirection;
}
export interface ReadingReceipt {
  sequence: number;
  status: ReadingStatus;
}
export interface ReadingFind {
  sequence: number;
  count: number;
  limited: boolean;
  /**
   * One-based current match, or null when no match remains.
   */
  index?: number | null;
  active?: ReadingMatch | null;
}
export interface ReadingMatch {
  /**
   * @minItems 1
   * @maxItems 32
   */
  path: [string, ...string[]];
  offset: number;
}
export interface RowsProps {
  terminal?: TerminalPalette | null;
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
  /**
   * Optional smaller row cap below the nearest declared layout viewport's
   * height, in cells; defaults to the surface when no viewport is declared.
   */
  compactBelow?: CompactRows | null;
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
export interface CompactRows {
  height: number;
  maxRows: number;
}
export interface RowScrollbar {
  thumb: string;
  track: string;
  thumbColor: Color;
  trackColor: Color;
}
export interface ValueCheckpointProps {
  key: string;
  format: string;
  command?: ValueCheckpointCommand | null;
}
export interface TextFileProps {
  command?: TextFileCommand | null;
}
export interface TextFileCommand {
  owner: string;
  sequence: number;
  operation: TextFileOperation;
}
export interface TextFileObservation {
  owner: string;
  path?: string | null;
  sequence: number;
  state: TextFileState;
}
export interface MorphMarkProps {
  background: TerminalColor;
  foreground: TerminalColor;
  motion: boolean;
  ascii: boolean;
}
export interface MorphMarkConfiguration {
  /**
   * @minItems 2
   * @maxItems 2
   */
  shapes: [MorphShape, MorphShape];
  color: TerminalColor;
  period: number;
}
export interface MorphShape {
  /**
   * @minItems 1
   * @maxItems 32
   */
  paths: [[number, number][], ...[number, number][][]];
  radius: number;
  /**
   * @maxItems 32
   */
  miters: number[];
  columns: number;
}
export interface ValueCheckpointObservation {
  sequence: number;
  state: ValueCheckpointState;
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
  /**
   * UTF-8 byte capacity, fixed for this original native editor mount.
   */
  maxBytes?: number;
  /**
   * Terminal bracketed-paste policy. Other native insertion remains literal.
   */
  terminalPaste?: TerminalPaste & string;
  placeholder?: string;
  command?: EditorCommand | null;
  /**
   * Clipboard default for physical Enter at this original editor revision.
   * Programmatic submitted events never execute this native default.
   */
  submitCopy?: SubmitCopy | null;
}
export interface SubmitCopy {
  basis: EditorTextRef;
  text: string;
}
export interface EditorTextRef {
  owner: string;
  revision: number;
  bytes: number;
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
export interface EditorCheckpointObservation {
  owner: string;
  state: EditorCheckpointState;
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
   * Explicit Library import for immutable skill input preparation and metadata.
   * This does not select model tools or confer a service grant.
   */
  skillLibrary?: string | null;
  /**
   * Initial policy within the selected workspace capability.
   */
  permissions: AgentWorkspacePermissions;
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
export interface AgentWorkspacePermissions {
  access: AgentWorkspaceAccess;
  requestPermissions: boolean;
}
export interface DirectoryRootSpec {
  label: string;
  path: string;
}
export interface AgentServiceTool {
  /**
   * Explicit input projection. Skill tools receive the actual Agent tool names,
   * outside model-controlled arguments; this is compatibility data, not authority.
   */
  input?: AgentToolInput & string;
  /**
   * Explicit versioned renderer key, frozen with the call; never inferred from its name.
   */
  presentation?: string | null;
  /**
   * Display only; the selected name, contract and binding still identify execution.
   */
  title?: DisplayText | null;
  /**
   * Explicit model projection; JSON data never implicitly grants image access.
   */
  result: AgentToolResult;
  /**
   * Additional user consent for this exact call; service grants remain independent.
   */
  confirmation: AgentToolConfirmation;
  name: string;
  description: string;
  import: string;
  method: string;
}
/**
 * Presentation text is distinct from the stable identity used for navigation.
 */
export interface DisplayText {
  fallback: string;
  translations?: {
    [k: string]: string | undefined;
  };
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
  /**
   * @maxItems 8
   */
  thinkingLevels:
    | []
    | [ThinkingLevel]
    | [ThinkingLevel, ThinkingLevel]
    | [ThinkingLevel, ThinkingLevel, ThinkingLevel]
    | [ThinkingLevel, ThinkingLevel, ThinkingLevel, ThinkingLevel]
    | [ThinkingLevel, ThinkingLevel, ThinkingLevel, ThinkingLevel, ThinkingLevel]
    | [ThinkingLevel, ThinkingLevel, ThinkingLevel, ThinkingLevel, ThinkingLevel, ThinkingLevel]
    | [
        ThinkingLevel,
        ThinkingLevel,
        ThinkingLevel,
        ThinkingLevel,
        ThinkingLevel,
        ThinkingLevel,
        ThinkingLevel,
      ]
    | [
        ThinkingLevel,
        ThinkingLevel,
        ThinkingLevel,
        ThinkingLevel,
        ThinkingLevel,
        ThinkingLevel,
        ThinkingLevel,
        ThinkingLevel,
      ];
  defaultThinkingLevel: ThinkingLevel | null;
}
export interface ConnectionTarget {
  connectionId: string;
  modelId: string;
}
export interface SettingsConnectionSummary {
  /**
   * Editing controls implemented by this Settings service for the provider.
   */
  editable: boolean;
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
/**
 * Installation-selected skill collections. No ambient home or workspace scan.
 */
export interface SkillLibraryConfiguration {
  /**
   * Explicit user root; only its standard .maka/skills and .agents/skills collections are scanned.
   */
  home?: string | null;
  /**
   * Explicit ceilings for a request's selected project. An observation grants no access.
   *
   * @maxItems 8
   */
  projectRoots?:
    | []
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
  /**
   * Include the immutable skill instructions and resources shipped with this build.
   */
  bundled?: boolean;
  /**
   * Enable installation into this instance's own private collection.
   */
  manage?: boolean;
  /**
   * Allow edits to enabled/pinned preferences, independently of file management.
   */
  preferences?: boolean;
  /**
   * @maxItems 8
   */
  sources:
    | []
    | [SkillLibrarySource]
    | [SkillLibrarySource, SkillLibrarySource]
    | [SkillLibrarySource, SkillLibrarySource, SkillLibrarySource]
    | [SkillLibrarySource, SkillLibrarySource, SkillLibrarySource, SkillLibrarySource]
    | [
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
      ]
    | [
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
      ]
    | [
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
      ]
    | [
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
        SkillLibrarySource,
      ];
}
export interface SkillLibrarySource {
  id: string;
  label: string;
  /**
   * Absolute directory containing skill directories. Captured at activation.
   */
  path: string;
}
export interface SkillLibraryList {
  workspace?: AgentWorkspace | null;
  query: string;
  cursor?: SkillLibraryCursor | null;
}
export interface AgentWorkspace {
  path: string;
  directoryIdentity: string;
  identity: string;
}
export interface SkillLibraryCursor {
  revision: string;
  query: string;
  offset: number;
}
export interface SkillLibraryEntry {
  bundled: boolean;
  installed: boolean;
  preference: SkillPreference;
  reference: string;
  source: string;
  name: string;
  description: string;
  valid: boolean;
  shadowedBy?: string | null;
  metadataTruncated: boolean;
  issues: string[];
}
export interface SkillPreference {
  enabled: boolean;
  pinned: boolean;
}
export interface SkillLibraryRead {
  workspace?: AgentWorkspace | null;
  reference: string;
  /**
   * Complete catalog revision from the list/read response.
   */
  revision: string;
  offset: number;
}
export interface SkillLibraryChange {
  workspace?: AgentWorkspace | null;
  kind: SkillLibraryChangeKind;
  reference: string;
  revision: string;
}
export interface SkillLibraryReview {
  change: SkillLibraryChange;
  digest: string;
}
/**
 * Canonical publication evidence alongside the unmodified original Host facts.
 */
export interface SkillLibraryOutcome {
  original: OperationReply;
  published?: SkillLibraryChangeResult | null;
}
export interface SkillLibrarySetPreference {
  reference: string;
  revision: number;
  preference: SkillPreference;
}
export interface SkillModelSearch {
  query: string;
  limit?: number | null;
}
export interface SkillModelLoad {
  /**
   * Exact ref from search, or a skill id/name resolved before eligibility checks.
   */
  name: string;
  /**
   * Catalog revision from search or a previous load/resource response.
   */
  revision: string;
  /**
   * UTF-8 resource path relative to this skill, or its exact continuation.
   */
  resource?: ReadInput | null;
  /**
   * UTF-8 byte offset from a returned instruction continuation; omit initially.
   */
  instructionOffset?: number | null;
}
export interface ReadInput {
  path: string;
  offset?: number;
  limit?: number;
}
export interface SkillModelSearchResult {
  revision: string;
  query: string;
  queryTruncated: boolean;
  matches: SearchMatch[];
  totalEligible: number;
  matchedCount: number;
  truncated: boolean;
  [k: string]: unknown | undefined;
}
export interface SearchMatch {
  ref: string;
  id: string;
  name: string;
  description: string;
  scope: SkillScope;
  source: SkillSource;
  metadataTruncated: boolean;
  score: number;
  [k: string]: unknown | undefined;
}
export interface SkillMetadata {
  ref: string;
  id: string;
  name: string;
  description: string;
  scope: SkillScope;
  source: SkillSource;
  metadataTruncated: boolean;
  [k: string]: unknown | undefined;
}
export interface SkillPrepare {
  message: string;
  /**
   * Actual selected tools bound to this Library; these names grant no access.
   */
  loadTool?: string | null;
  searchTool?: string | null;
}
export interface SkillPreparationRecord {
  revision: string;
  metadataPrompt: string;
  invocation: SkillInvocationResult;
}
export interface SkillInvocationResult {
  loaded: LoadedSkill[];
  failed: SkillInvocationFailure[];
  receipts: SkillInvocationReceipt[];
}
export interface LoadedSkill {
  id: string;
  name: string;
}
export interface SkillRequestFailure {
  request: string;
  reason: SkillFailureReason;
}
export interface SkillOverflowFailure {
  reason: TooManyRequests;
  requestLimit: number;
}
export interface AgentList {
  filter: AgentSessionFilter;
  position: AgentSessionPosition;
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
  dataSource: AgentDataSource;
  permissions: AgentWorkspacePermissions;
  boundaryRevision: number;
  id: string;
  title: string;
  labels: string[];
  workspace: AgentWorkspace;
  revision: number;
  updatedAt: number;
  archived: boolean;
  hasUnread: boolean;
  activity: AgentSessionActivity;
}
/**
 * Stable business-data origin, independent of package or activation revisions.
 */
export interface AgentDataSource {
  stateDomain: string;
  deployment: string;
  namespace: InstanceId;
}
export interface AgentDirectory {
  name: string;
  path: string;
  selection: AgentDirectorySelection;
}
export interface AgentMetadata {
  /**
   * Omission leaves the permission boundary and its approvals unchanged.
   */
  permissions?: AgentPermissionChange | null;
  sessionId: string;
  expectedRevision: number;
  title: string;
  /**
   * @maxItems 32
   */
  labels: string[];
}
export interface AgentPermissionChange {
  settings: AgentWorkspacePermissions;
  /**
   * Invalidate additive approvals even when the base policy is unchanged.
   */
  revokeGrants: boolean;
}
export interface AgentSend {
  /**
   * IDs of immutable user uploads in this Session; metadata is resolved by the owner.
   *
   * @maxItems 8
   */
  attachments?:
    | []
    | [string]
    | [string, string]
    | [string, string, string]
    | [string, string, string, string]
    | [string, string, string, string, string]
    | [string, string, string, string, string, string]
    | [string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string];
  model: ModelSelection;
  /**
   * Omission/null uses the configured model default; `off` is explicit.
   */
  thinkingLevel?: ThinkingLevel | null;
  sessionId: string;
  message: string;
}
export interface AgentStop {
  sessionId: string;
  invocationId: string;
}
export interface AgentUploadAttachment {
  sessionId: string;
  name: string;
  bytes: number;
  digest: string;
}
export interface AgentAttachment {
  sessionId: string;
  id: string;
  name: string;
  mimeType: string;
  bytes: number;
}
export interface AgentMarkRead {
  sessionId: string;
  messageId: string;
}
export interface AgentWatch {
  sessionId: string;
  after: number;
}
export interface AgentPendingInteractions {
  revision: number;
  /**
   * @maxItems 16
   */
  entries:
    | []
    | [AgentInteractionSummary]
    | [AgentInteractionSummary, AgentInteractionSummary]
    | [AgentInteractionSummary, AgentInteractionSummary, AgentInteractionSummary]
    | [
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
      ]
    | [
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
      ]
    | [
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
      ]
    | [
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
      ]
    | [
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
      ]
    | [
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
      ]
    | [
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
      ]
    | [
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
      ]
    | [
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
      ]
    | [
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
      ]
    | [
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
      ]
    | [
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
      ]
    | [
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
        AgentInteractionSummary,
      ];
}
export interface AgentInteractionSummary {
  requestId: string;
  kind: AgentInteractionKind;
  toolUseId: string | null;
}
export interface AgentInteractionRead {
  sessionId: string;
  requestId: string;
}
export interface AgentInteractionRespond {
  sessionId: string;
  requestId: string;
  answer: InteractionAnswer;
}
export interface Permissions {
  /**
   * @maxItems 32
   */
  filesystem: Rule[];
  network: Network;
}
/**
 * Paths have executor-native meaning. Validate and materialize them on the
 * execution host, never on a remote Desktop. This document alone grants nothing.
 */
export interface Rule {
  path: string;
  scope: Scope;
  access: Access;
}
/**
 * Destination authority, independent of the upstream proxy used to reach it.
 * A domain grant does not authorize its subdomains or other ports.
 */
export interface Destination {
  host: string;
  port: number;
}
export interface InteractionRecord {
  sessionId: string;
  turnId: string;
  runId: string;
  requestId: string;
  createdAt: number;
  request: InteractionRequest;
  outcome: InteractionOutcome | null;
}
/**
 * The Host supplies the real operation description; it is display information,
 * not authority inferred from a command string or a plugin-supplied Session ID.
 */
export interface PermissionRequest {
  reason: string;
  command?: PermissionCommand | null;
  permissions: Permissions;
}
export interface PermissionCommand {
  command: string;
  cwd: string;
}
export interface InteractionQuestion {
  question: string;
  /**
   * @minItems 2
   * @maxItems 3
   */
  options: [QuestionOption, QuestionOption] | [QuestionOption, QuestionOption, QuestionOption];
}
export interface QuestionOption {
  label: string;
  description?: string;
}
export interface FormRequester {
  name: string;
  source?: string | null;
}
export interface FormOption {
  value: string;
  label: string;
}
export interface GrantTarget {
  providerId: string;
  contractId: string;
  serverId: string;
  toolName: string;
  capability: GrantCapability;
  scope: GrantScope;
  [k: string]: unknown | undefined;
}
export interface QuestionAsk {
  /**
   * @minItems 1
   * @maxItems 3
   */
  questions:
    | [InteractionQuestion]
    | [InteractionQuestion, InteractionQuestion]
    | [InteractionQuestion, InteractionQuestion, InteractionQuestion];
}
export interface FormAsk {
  message: string;
  /**
   * @maxItems 32
   */
  fields: FormField[];
}
/**
 * Immutable installation ceiling selected by trusted native composition.
 */
export interface WorkspaceProcessConfiguration {
  filesystem: Policy;
  network: Network;
  /**
   * Trusted CLI supporting the original platform sandbox bootstrap.
   */
  helper?: string | null;
  /**
   * Trusted Agent Session owner in this Root/deployment. This selects a data
   * namespace only; Terminal endpoints still require their own explicit grants.
   */
  sessionOwner?: InstanceId | null;
}
/**
 * Longest path wins; at the same path exact wins over subtree, then deny wins
 * over write over read. Deny globs are absolute and cannot be reopened by paths.
 * Matching directories deny their descendants.
 * Host file operations match at access time. Linux and Windows subprocesses
 * snapshot existing glob matches during preparation; macOS enforces them live.
 * Empty restricted policy denies all access, rather than falling back to defaults.
 */
export interface Policy {
  default: Access;
  /**
   * @maxItems 512
   */
  rules: Rule[];
  /**
   * @maxItems 128
   */
  denyGlobs: string[];
}
export interface TerminalList {
  sessionId: string;
  offset?: number;
}
export interface TerminalTarget {
  sessionId: string;
  ref: string;
}
export interface TerminalStart {
  sessionId: string;
  /**
   * Omit for an interactive shell; an explicit command runs with bounded pipes.
   */
  command?: string | null;
  size?: TerminalSize | null;
}
export interface TerminalController {
  target: TerminalTarget;
  controllerId: string;
}
export interface TerminalControl {
  controller: TerminalController;
  sequence: number;
  /**
   * @maxItems 64
   */
  actions?: RawAction[];
  size?: TerminalSize | null;
}
export interface TerminalPage {
  resources: ResourceUpdate[];
  revision: string;
  total: number;
  nextOffset?: number | null;
}
export interface ResourceUpdate {
  sessionId: string;
  ownership: Ownership;
  sourceTurnId: string;
  sourceToolCallId: string;
  result: ShellSnapshot;
}
export interface ShellSnapshot {
  kind: SnapshotKind;
  ref: string;
  mode: ShellMode;
  status: ShellStatus;
  cwd: string;
  cmd: string;
  startedAt: number;
  updatedAt: number;
  completedAt?: number;
  timeoutMs?: number;
  exitCode?: number;
  failureMessage?: string;
  revision: number;
  output?: ShellOutput;
}
export interface TerminalLeaseUpdate {
  controllerId: string;
  nextSequence?: number | null;
  held: boolean;
  resource: ResourceUpdate;
}
export interface TerminalControlReceipt {
  controllerId: string;
  sequence: number;
  acceptedBytes: number;
  resized: boolean;
  resizeChanged: boolean;
}
export interface TerminalReleased {
  controllerId: string;
  released: boolean;
}
export interface TerminalStopped {
  applied: boolean;
  resource: ResourceUpdate;
}
export interface QuestionResponse {
  question: string;
  answer: string | null;
}
export interface AgentSessionPage {
  revision: string;
  needsYouCount: number;
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
  turnId: string;
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
  turnId: string;
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
  includeThinking?: boolean;
}
/**
 * Byte offset in one message field's UTF-8 text, not a bearer capability.
 */
export interface Cursor {
  field?: HistoryField & string;
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
  timing?: HistoryTurnTiming | null;
  field: HistoryField;
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
/**
 * Recorded logical Turn boundaries at the same history fence. An unfinished
 * historical boundary alone does not establish that execution is still live.
 */
export interface HistoryTurnTiming {
  startedAt: number;
  finished: HistoryTurnFinished | null;
}
export interface HistoryTurnFinished {
  at: number;
  outcome: HistoryTurnOutcome;
}
/**
 * Canonical references at the requested fence, never execution authority.
 * Both references are present only for a unique same-Turn call/result pair
 * with matching origin and parentage. An unpaired row retains its own reference.
 */
export interface HistoryTool {
  toolUseId: string;
  call: HistoryToolCall | null;
  result: HistoryToolResult | null;
}
export interface HistoryToolCall {
  /**
   * Opaque display contract captured at dispatch; no execution authority.
   */
  presentation?: string | null;
  sequence: number;
  name: string;
  title: DisplayText | null;
  /**
   * This native Turn is still open at the history fence; not execution authority.
   */
  pending: boolean;
  /**
   * A unique original Host dispatch is recorded at this fence; not liveness.
   */
  dispatched: boolean;
}
export interface HistoryToolResult {
  settlement: ToolSettlement | null;
  sequence: number;
  /**
   * Does not distinguish failed from unknown effects.
   */
  isError: boolean;
  /**
   * Canonical content is an image; this does not grant artifact reads.
   */
  image: boolean;
}
export interface AgentHistorySearch {
  sessionId: string;
  /**
   * Original raw-event fence, shared with history reads.
   */
  through: number;
  /**
   * Exclusive transcript-row sequence returned by the previous batch; zero starts a scan.
   */
  after: number;
  /**
   * Literal, case-sensitive query, additionally limited to 512 UTF-8 bytes.
   */
  query: string;
  includeInternal: boolean;
}
export interface AgentHistoryMatch {
  /**
   * Exact canonical field to read through the source query; history body may omit it.
   */
  field: SourceField;
  /**
   * Use this exact cursor with HistoryPosition::After and the same raw-event fence.
   */
  cursor: Cursor;
  preview: string;
}
export interface AgentHistorySourceRead {
  sessionId: string;
  through: number;
  source: AgentHistorySourceReference;
  field: SourceField;
  /**
   * Optional JSON keys/array indices within arguments, result or receipt.
   *
   * @maxItems 16
   */
  path?:
    | []
    | [HistorySourceSegment]
    | [HistorySourceSegment, HistorySourceSegment]
    | [HistorySourceSegment, HistorySourceSegment, HistorySourceSegment]
    | [HistorySourceSegment, HistorySourceSegment, HistorySourceSegment, HistorySourceSegment]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | null;
  position: SourcePosition;
}
/**
 * One literal UTF-8 excerpt of an immutable field. JSON excerpts need not be
 * standalone JSON. Image descriptors carry no artifact read authorization.
 */
export interface HistorySource {
  sequence: number;
  /**
   * Host-recorded milliseconds of this immutable row; not a client clock.
   */
  timestamp: number;
  role: HistorySourceRole;
  /**
   * Readable canonical fields of this same immutable row, in display order.
   *
   * @minItems 1
   * @maxItems 7
   */
  fields:
    | [SourceField]
    | [SourceField, SourceField]
    | [SourceField, SourceField, SourceField]
    | [SourceField, SourceField, SourceField, SourceField]
    | [SourceField, SourceField, SourceField, SourceField, SourceField]
    | [SourceField, SourceField, SourceField, SourceField, SourceField, SourceField]
    | [SourceField, SourceField, SourceField, SourceField, SourceField, SourceField, SourceField];
  field: SourceField;
  /**
   * Omission reads the encoded field. A selected JSON string is literal text.
   *
   * @maxItems 16
   */
  path?:
    | []
    | [HistorySourceSegment]
    | [HistorySourceSegment, HistorySourceSegment]
    | [HistorySourceSegment, HistorySourceSegment, HistorySourceSegment]
    | [HistorySourceSegment, HistorySourceSegment, HistorySourceSegment, HistorySourceSegment]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | [
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
        HistorySourceSegment,
      ]
    | null;
  format: SourceFormat;
  offset: number;
  totalBytes: number;
  text: string;
  tool: HistoryTool | null;
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
  includeThinking?: boolean;
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
/**
 * Bounded source-to-readable-text conversion using the presenter's Markdown semantics.
 */
export interface MarkdownTextInput {
  source: string;
}
