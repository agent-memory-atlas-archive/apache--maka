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

import type {
  DeploymentRef,
  EditorObservation,
  TextareaProps,
  FileObservation,
  PackageInstalled,
} from '../../packages/plugin-sdk/src/index.js';
import type { ConnectionsProps } from './contracts.js';
import { copy } from './locale.js';
import { sameValue } from './value.js';
import { fill, pixels, type Presentation, type Node } from './view.js';

export type InstallIntent =
  | { source: DeploymentRef; file: FileObservation }
  | { source: DeploymentRef; url: string; captured: EditorObservation };
export interface PackageImport {
  open: boolean;
  mode: 'file' | 'url';
  urlObservation: EditorObservation | null;
  observation: FileObservation | null;
  confirmation: InstallIntent | null;
  installed: PackageInstalled | null;
}
export function packageImport(): PackageImport {
  return {
    open: false,
    mode: 'file',
    urlObservation: null,
    observation: null,
    confirmation: null,
    installed: null,
  };
}
export function canInstall(importer: PackageImport, busy: boolean, conflict: boolean) {
  if (busy || conflict) return false;
  if (importer.mode === 'file')
    return importer.observation?.phase === 'ready' && !!importer.observation.file;
  const snapshot = importer.urlObservation?.snapshot;
  return !!snapshot && !snapshot.marked && snapshot.text.bytes > 0 && snapshot.text.bytes <= 8192;
}
export function matchesImport(importer: PackageImport) {
  const intent = importer.confirmation;
  return (
    !!intent &&
    ('file' in intent
      ? importer.mode === 'file' && sameValue(intent.file, importer.observation)
      : importer.mode === 'url' && sameValue(intent.captured, importer.urlObservation))
  );
}
export function importView<N extends Node>(
  presentation: Presentation<N>,
  props: ConnectionsProps,
  importer: PackageImport,
  busy: boolean,
  conflict: boolean,
  notice: string | null,
  recoverable: boolean,
  pending: boolean,
) {
  const target = presentation.target;
  const ui = presentation.paint(props.theme),
    text = copy(props.locale),
    observed = importer.observation;
  const confirmation = importer.confirmation;
  const file =
    importer.mode === 'file'
      ? confirmation && 'file' in confirmation
        ? confirmation.file.file
        : observed?.file
      : undefined;
  const error = observed?.error;
  const failure =
    error === 'limit_exceeded'
      ? text.deploymentFileTooLarge
      : error === 'budget_exhausted'
        ? text.deploymentFileBusy
        : text.deploymentFileFailed;
  const panel = ui.box(
    'deployment-import-panel',
    [
      ui.box(
        'deployment-import-header',
        [
          ui.box(
            'deployment-import-heading',
            [
              ui.button(
                'deployment-import-back',
                text.back,
                false,
                false,
                target === 'tui' ? { background: 0, align: 'start' } : {},
              ),
              ui.text('deployment-import-title', text.deploymentInstall, 22, props.theme.text, 600),
            ],
            {
              visible: !props.shell,
              row: true,
              gap: target === 'desktop' ? 8 : 1,
              terminal: { wrap: true },
            },
          ),
          ui.text('deployment-import-help', text.deploymentInstallHelp, 13, props.theme.muted),
          ...(notice ? [ui.text('deployment-import-notice', notice, 13, props.theme.muted)] : []),
        ],
        { gap: target === 'desktop' ? 8 : 1, style: { shrink: 0 } },
      ),
      ui.box(
        'deployment-import-body',
        [
          ui.box(
            'deployment-import-source',
            [
              ui.button(
                'deployment-import-local',
                text.deploymentChooseFile,
                busy || !!confirmation,
                importer.mode === 'file',
              ),
              ui.button(
                'deployment-import-remote',
                text.deploymentUrl,
                busy || !!confirmation,
                importer.mode === 'url',
              ),
            ],
            { row: true, gap: 1, style: { wrap: true } },
          ),
          ui.box(
            'deployment-import-url-owner',
            [
              presentation.slot(
                'deployment-import-url',
                'editor',
                {
                  appearance: props.theme.textarea,
                  accessibilityLabel: text.deploymentUrl,
                  placeholder: 'https://example.com/plugin.maka-plugin',
                  tabIndex: 0,
                  readOnly: busy || !!confirmation,
                  disabled: false,
                  autoFocus: false,
                  softWrap: presentation.target === 'tui',
                  terminalRows: { min: 1, max: 2 },
                  submit: 'primary_enter',
                  command: null,
                  menu: {
                    appearance: props.theme.menu,
                    labels: {
                      title: text.edit,
                      undo: text.undo,
                      redo: text.redo,
                      cut: text.cut,
                      copy: text.copy,
                      paste: text.paste,
                      selectAll: text.selectAll,
                    },
                  },
                } satisfies TextareaProps,
                { changed: 'deployment-import-url-changed' },
              ),
            ],
            { visible: importer.mode === 'url' },
          ),
          ...(confirmation && 'url' in confirmation
            ? [ui.text('deployment-import-url-confirmed', confirmation.url, 13, props.theme.text)]
            : []),
          // The retained native child owns selection. Confirmation suspends it rather
          // than recreating it; snapshot still captures the exact observed revision.
          ui.box(
            'deployment-import-file-owner',
            [
              presentation.slot(
                'deployment-import-file',
                'file',
                {
                  appearance: props.theme.input,
                  label: text.deploymentChooseFile,
                  pathLabel: text.deploymentFilePath,
                  browseLabel: text.fileBrowse,
                  parentLabel: text.directoryParent,
                  emptyLabel: text.fileListEmpty,
                  truncatedLabel: text.fileListTruncated,
                  placeholder: text.deploymentFilePlaceholder,
                  disabled: busy || !!confirmation,
                  tabIndex: 0,
                },
                { changed: 'deployment-import-changed' },
              ),
            ],
            { visible: importer.mode === 'file' && (target === 'tui' || !confirmation) },
          ),
          ...(confirmation && target === 'desktop'
            ? [
                ui.text(
                  'deployment-import-question',
                  text.deploymentInstallQuestion,
                  17,
                  props.theme.text,
                  550,
                ),
                ui.text(
                  'deployment-import-confirm-help',
                  text.deploymentInstallHelp,
                  14,
                  props.theme.muted,
                ),
              ]
            : []),
          ...(file
            ? [
                ui.text('deployment-import-name', file.name, 15, props.theme.text, 550),
                ui.text('deployment-import-size', String(file.bytes) + ' B', 13, props.theme.muted),
              ]
            : []),
          ...(observed?.phase === 'choosing'
            ? [
                ui.text(
                  'deployment-import-loading',
                  text.deploymentFileLoading,
                  13,
                  props.theme.muted,
                ),
              ]
            : []),
          ...(observed?.phase === 'failed'
            ? [ui.text('deployment-import-failed', failure, 13, props.theme.muted)]
            : []),
          ...(importer.installed
            ? [
                ui.text(
                  'deployment-import-package',
                  importer.installed.package.id,
                  15,
                  props.theme.text,
                  550,
                ),
                ui.text(
                  'deployment-import-package-revision',
                  importer.installed.package.revision,
                  13,
                  props.theme.muted,
                ),
                ui.text(
                  'deployment-import-activation-help',
                  text.deploymentActivateHelp,
                  13,
                  props.theme.muted,
                ),
              ]
            : []),
        ],
        {
          scroll: 'manual',
          terminal: { grow: 1 },
          style: { width: fill, height: pixels(0), grow: 1, shrink: 1, minHeight: 0 },
        },
      ),
      ui.box(
        'deployment-import-footer',
        confirmation && target === 'desktop'
          ? [
              ui.button(
                'deployment-import-confirm',
                text.confirmDefault,
                busy || conflict || !matchesImport(importer),
                true,
              ),
              ui.button('deployment-import-cancel', text.cancel, busy),
            ]
          : [
              ui.button(
                'deployment-import-install',
                text.deploymentInstall,
                !canInstall(importer, busy, conflict),
                true,
              ),
              ...(conflict
                ? [
                    ui.button(
                      'deployment-import-refresh',
                      target === 'tui' ? (props.theme.ascii ? 'r' : '↻') : text.deploymentRefresh,
                      busy,
                      false,
                      target === 'tui' ? { background: 0 } : {},
                      undefined,
                      text.deploymentRefresh,
                    ),
                  ]
                : []),
              ...(recoverable ? [ui.button('deployment-import-check', text.check, pending)] : []),
              ui.button('deployment-import-history', text.deploymentHistory, pending),
              ...(importer.installed
                ? [ui.button('deployment-import-advanced', text.deploymentEdit, busy)]
                : []),
            ],
        {
          row: true,
          gap: target === 'desktop' ? 8 : 1,
          terminal: { wrap: true },
          style: { wrap: true, shrink: 0 },
        },
      ),
    ],
    {
      visible: importer.open,
      autoFocus: true,
      shortcuts:
        target === 'tui'
          ? [
              {
                key: 'escape',
                modifiers: [],
                input: 'deployment-import-back',
                enabled: true,
              },
            ]
          : [],
      style: { width: fill, height: fill },
      terminal: { grow: 1 },
      gap: target === 'desktop' ? 16 : 1,
    },
  );
  if (target === 'desktop') return panel;
  return ui.box(
    'deployment-import-page',
    [
      ui.dialog(
        'deployment-import-dialog',
        text.deploymentInstall,
        panel,
        ui.box(
          'deployment-import-dialog-content',
          [
            ui.box(
              'deployment-import-dialog-body',
              [
                ui.text(
                  'deployment-import-question',
                  text.deploymentInstallQuestion,
                  17,
                  props.theme.text,
                  550,
                ),
                ui.text(
                  'deployment-import-confirm-help',
                  text.deploymentInstallHelp,
                  14,
                  props.theme.muted,
                ),
                ...(confirmation && 'url' in confirmation
                  ? [
                      ui.text(
                        'deployment-import-confirm-url',
                        confirmation.url,
                        13,
                        props.theme.text,
                      ),
                    ]
                  : []),
                ...(file
                  ? [
                      ui.text(
                        'deployment-import-confirm-name',
                        file.name,
                        15,
                        props.theme.text,
                        550,
                      ),
                      ui.text(
                        'deployment-import-confirm-size',
                        String(file.bytes) + ' B',
                        13,
                        props.theme.muted,
                      ),
                    ]
                  : []),
              ],
              { scroll: 'manual', terminal: { grow: 1 }, gap: 1 },
            ),
            ui.box(
              'deployment-import-dialog-actions',
              [
                ui.button('deployment-import-cancel', text.cancel, busy),
                ui.button(
                  'deployment-import-confirm',
                  text.confirmDefault,
                  busy || conflict || !confirmation || !matchesImport(importer),
                  true,
                ),
              ],
              { row: true, gap: 1, terminal: { wrap: true } },
            ),
          ],
          { terminal: { grow: 1 } },
        ),
        !!confirmation,
        undefined,
        { width: 64, height: 14 },
      ),
    ],
    { visible: importer.open, gap: 0, terminal: { grow: 1 } },
  );
}
