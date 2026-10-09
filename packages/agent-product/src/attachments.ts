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

import type { AgentAttachment, FileObservation } from '../../plugin-sdk/src/index.js';
import type { Theme } from './contracts.js';
import { copy } from './locale.js';
import { fill, type Node, type Presentation } from './view.js';

export interface AttachmentDrafts {
  files: AgentAttachment[];
  selection: { attempt: number; sessionId: string; file: FileObservation | null } | null;
}
export const attachmentFiles = (drafts: AttachmentDrafts, sessionId: string | undefined) =>
  drafts.files.filter((file) => file.sessionId === sessionId);
// Bound retained descriptors across navigation; never silently evict another draft.
export const attachmentCapacity = (drafts: AttachmentDrafts, sessionId: string) =>
  drafts.files.length < 64 && attachmentFiles(drafts, sessionId).length < 8;
export const attachmentChild = (drafts: AttachmentDrafts) =>
  'attachment-file-' + drafts.selection?.attempt;
export function attachmentView<N extends Node>(
  presentation: Presentation<N>,
  theme: Theme,
  locale: string,
  drafts: AttachmentDrafts,
  busy: boolean,
  enabled: boolean,
  notice: string | null,
  recoverable: boolean,
  checking: boolean,
) {
  const ui = presentation.paint(theme),
    text = copy(locale),
    selection = drafts.selection!;
  const observed = selection.file;
  const files = attachmentFiles(drafts, selection.sessionId);
  const capacity = attachmentCapacity(drafts, selection.sessionId);
  return ui.box(
    'attachment-panel',
    [
      ui.box(
        'attachment-body',
        [
          ui.text('attachment-help', text.attachmentHelp, 13, theme.muted),
          ...files.map((file) =>
            ui.box(
              'attachment-row-' + file.id,
              [
                ui.text(
                  'attachment-name-' + file.id,
                  `${file.name} · ${file.bytes} B`,
                  13,
                  theme.text,
                  400,
                  { truncate: true },
                ),
                ui.button('attachment-remove-' + file.id, text.attachmentRemove, busy),
              ],
              { row: true, terminal: { wrap: true }, style: { width: fill, wrap: true } },
            ),
          ),
          ...(!capacity
            ? [ui.text('attachment-limit', text.attachmentLimit, 13, theme.muted)]
            : []),
          presentation.slot(
            attachmentChild(drafts),
            'file',
            {
              appearance: theme.input,
              label: text.attachmentChoose,
              pathLabel: text.attachmentPath,
              browseLabel: text.fileBrowse,
              parentLabel: text.directoryParent,
              emptyLabel: text.fileListEmpty,
              truncatedLabel: text.fileListTruncated,
              placeholder: text.attachmentPathPlaceholder,
              disabled: busy || !enabled || !capacity,
              tabIndex: 0,
            },
            { changed: 'attachment-file-changed' },
          ),
          ...(observed?.phase === 'choosing'
            ? [ui.text('attachment-reading', text.deploymentFileLoading, 13, theme.muted)]
            : []),
          ...(observed?.phase === 'ready' && observed.file
            ? [
                ui.text(
                  'attachment-selected',
                  `${observed.file.name} · ${observed.file.bytes} B`,
                  13,
                  theme.text,
                ),
              ]
            : []),
          ...(observed?.phase === 'failed'
            ? [
                ui.text(
                  'attachment-file-error',
                  observed.error === 'limit_exceeded'
                    ? text.attachmentTooLarge
                    : text.attachmentFailed,
                  13,
                  theme.muted,
                ),
              ]
            : []),
          ...(notice ? [ui.text('attachment-notice', notice, 13, theme.muted)] : []),
        ],
        {
          scroll: 'manual',
          terminal: { grow: 1 },
          style: { width: fill, minHeight: 0, grow: 1, shrink: 1 },
        },
      ),
      ui.box(
        'attachment-controls',
        [
          ui.button('attachment-close', text.closeModels),
          ...(recoverable ? [ui.button('attachment-check', text.check, checking)] : []),
          ui.button(
            'attachment-upload',
            text.attachmentAdd,
            busy || !enabled || !capacity || observed?.phase !== 'ready' || !observed.file,
            true,
          ),
        ],
        { row: true, terminal: { wrap: true }, style: { wrap: true, shrink: 0 } },
      ),
    ],
    { terminal: { grow: 1 }, style: { width: fill, height: fill } },
  );
}
