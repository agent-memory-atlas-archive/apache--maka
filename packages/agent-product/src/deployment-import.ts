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
  FileObservation,
  PackageInstalled,
} from '../../plugin-sdk/src/index.js';
import type { ConnectionsProps } from './contracts.js';
import { copy } from './locale.js';
import { sameValue } from './value.js';
import { fill, pixels, type Presentation, type Node } from './view.js';

export interface InstallIntent {
  source: DeploymentRef;
  file: FileObservation;
}
export interface PackageImport {
  open: boolean;
  observation: FileObservation | null;
  confirmation: InstallIntent | null;
  installed: PackageInstalled | null;
}
export function packageImport(): PackageImport {
  return { open: false, observation: null, confirmation: null, installed: null };
}
export function canInstall(importer: PackageImport, busy: boolean, conflict: boolean) {
  return (
    !busy && !conflict && importer.observation?.phase === 'ready' && !!importer.observation.file
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
  const file = confirmation?.file.file ?? observed?.file;
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
            { row: true, gap: target === 'desktop' ? 8 : 1, terminal: { wrap: true } },
          ),
          ui.text('deployment-import-help', text.deploymentInstallHelp, 13, props.theme.muted),
          ...(notice ? [ui.text('deployment-import-notice', notice, 13, props.theme.muted)] : []),
        ],
        { gap: target === 'desktop' ? 8 : 1, style: { shrink: 0 } },
      ),
      ui.box(
        'deployment-import-body',
        [
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
            { visible: target === 'tui' || !confirmation },
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
                busy || conflict || !sameValue(confirmation.file, observed),
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
                  busy || conflict || !confirmation || !sameValue(confirmation.file, observed),
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
