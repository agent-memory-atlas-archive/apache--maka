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
  Component,
  EditorObservation,
  TextareaProps,
  MarkdownProps,
  ReadingObservation,
  ReadingCommand,
  ComponentInput,
  UiExport,
} from '../../plugin-sdk/src/index.js';
import {
  composerContract,
  navigationContract,
  timelineContract,
  type ComposerProps,
  type NavigationProps,
  type TimelineProps,
} from './contracts.js';
import { copy } from './locale.js';
import { paint, slot, output, fill, pixels, compactLabel, type Target, type Node } from './view.js';
interface Revision {
  revision: number;
}
interface Draft extends Revision {
  observation: EditorObservation | null;
}
interface Reading extends Revision {
  observation: ReadingObservation | null;
  command: ReadingCommand | null;
  landing: TimelineProps['landing'];
}
export function components(target: Target): {
  composer: Component<Draft, ComposerProps>;
  navigation: Component<Revision, NavigationProps>;
  timeline: Component<Reading, TimelineProps>;
} {
  const composerFrame = (
    state: Draft,
    props: ComposerProps,
    events: Parameters<typeof output>[2] = [],
  ) => {
    const ui = paint(target, props.theme),
      text = copy(props.locale);
    const pending =
      props.command !== null &&
      (state.observation?.receipt?.owner !== props.command.owner ||
        state.observation?.receipt?.sequence !== props.command.sequence);
    const available =
      props.enabled &&
      !props.busy &&
      !props.running &&
      !pending &&
      !state.observation?.snapshot.marked &&
      !!state.observation?.snapshot.text.bytes;
    const textarea: TextareaProps = {
      appearance: props.theme.textarea,
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
      accessibilityLabel: text.draft,
      readOnly: false,
      disabled: false,
      softWrap: true,
      tabIndex: 0,
      initialText: '',
      placeholder: text.placeholder,
      command: props.command,
    };
    return output(
      state,
      ui.box(
        'composer',
        [
          ui.box('editor-area', [slot('draft', 'editor', textarea, { changed: 'changed' })], {
            style: { width: fill, height: pixels(112), shrink: 0 },
          }),
          ui.box(
            'composer-controls',
            [
              ui.text('hint', text.enterHint, 12, props.theme.muted),
              ...(props.running
                ? [ui.button('stop', text.stop, props.busy)]
                : [ui.button('send', text.send, !available || !state.observation, true)]),
            ],
            { row: true, style: { justify: 'between', align: 'center', width: fill } },
          ),
        ],
        { style: { width: fill } },
      ),
      events,
    );
  };
  const navigationFrame = (state: Revision, props: NavigationProps) => {
    const ui = paint(target, props.theme),
      text = copy(props.locale);
    return output(
      state,
      ui.box(
        'navigation',
        [
          ui.text('brand', 'maka', 24, props.theme.text, 650),
          ui.button('new', text.new, props.busy),
          ui.text('heading', text.conversations, 12, props.theme.muted, 550),
          ...props.sessions.map((session) => {
            const button = ui.button(
              session.id,
              target === 'desktop' ? session.title : compactLabel(session.title, 48),
              props.busy,
            );
            if (target === 'desktop')
              button.style = {
                ...(button.style as object),
                width: fill,
                minWidth: 0,
                justify: 'start',
                truncate: true,
              };
            return ui.box('session-' + session.id, [button], {
              surface: session.id === props.selected,
              style: { width: fill, minWidth: 0, radius: 8, shrink: 0 },
            });
          }),
          ...(props.more ? [ui.button('more', text.moreSessions, props.busy)] : []),
        ],
        {
          surface: true,
          padding: target === 'desktop' ? 20 : 1,
          style: { height: fill, width: fill },
          scroll: 'anchor',
        },
      ),
    );
  };
  const timelineFrame = (state: Reading, props: TimelineProps) => {
    const ui = paint(target, props.theme),
      text = copy(props.locale);
    const messages: Node[] = props.messages.map((message) =>
      ui.box(
        'message-' + message.id,
        [
          ui.text(
            'role-' + message.id,
            message.role === 'user'
              ? text.you
              : message.role === 'assistant'
                ? text.assistant
                : message.role,
            13,
            props.theme.muted,
            550,
          ),
          ...(message.continued
            ? [ui.text('continued-' + message.id, text.partial, 12, props.theme.muted)]
            : []),
          target === 'desktop' && message.role === 'assistant' && !message.continued
            ? slot(
                'text-' + message.id,
                'markdown',
                {
                  source: message.text,
                  appearance: props.theme.markdown,
                  selectable: true,
                  disabled: false,
                } satisfies MarkdownProps,
                {},
              )
            : target === 'desktop'
              ? slot(
                  'text-' + message.id,
                  'source',
                  {
                    source: message.text,
                    appearance: props.theme.markdown.body,
                    selection: props.theme.markdown.selection,
                    disabled: false,
                  },
                  {},
                )
              : ui.text('text-' + message.id, message.text, 16),
        ],
        {
          gap: target === 'desktop' ? 8 : 0,
          style: { width: fill, maxWidth: 760, shrink: 0 },
          surface: message.role === 'user',
          padding: target === 'desktop' && message.role === 'user' ? 16 : 0,
        },
      ),
    );
    const content = ui.box(
      'timeline',
      [
        ...(!messages.length && !props.loading
          ? [
              ui.text('welcome', text.welcome, 30, props.theme.text, 600),
              ui.text('welcome-copy', text.welcomeCopy, 16, props.theme.muted),
            ]
          : messages),
        ...(props.loading ? [ui.text('loading', text.loading, 14, props.theme.muted)] : []),
      ],
      {
        gap: target === 'desktop' ? 24 : 1,
        style: {
          width: fill,
          height: fill,
          padding: { top: 24, right: 24, bottom: 24, left: 24 },
        },
        scroll: props.live ? 'follow_end' : 'anchor',
      },
    );
    const controls: Node[] = [
      ...(props.older ? [ui.button('older', text.older, props.loading)] : []),
      ...(props.newer ? [ui.button('newer', text.newer, props.loading)] : []),
      ...(props.latest ? [ui.button('latest', text.latest, props.loading)] : []),
      ...(target === 'desktop' && state.observation?.viewport && !state.observation.viewport.atEnd
        ? [ui.button('end', text.toEnd, state.observation.pendingCommand != null)]
        : []),
    ];
    return output(
      state,
      ui.box(
        'timeline-frame',
        [
          target === 'desktop'
            ? slot(
                'reading',
                'reading',
                { content, command: state.command },
                { observed: 'reading-observed', input: 'reading-input' },
              )
            : content,
          ...(controls.length
            ? [
                ui.box('reading-controls', controls, {
                  row: true,
                  gap: 8,
                  style: { width: fill, shrink: 0, wrap: 'wrap' },
                }),
              ]
            : []),
        ],
        { gap: 8, style: { width: fill, height: fill } },
      ),
    );
  };
  return {
    composer: {
      mount: (props) => composerFrame({ revision: 0, observation: null }, props),
      update: (state, props) => composerFrame({ ...state, revision: state.revision + 1 }, props),
      event(state, input, props) {
        const next = { ...state, revision: state.revision + 1 };
        if (input.kind === 'component' && input.name === 'changed') {
          next.observation = input.event as EditorObservation;
          return composerFrame(next, props, [{ name: 'changed', value: next.observation }]);
        }
        if (
          input.kind === 'raw' &&
          input.key === 'send' &&
          props.enabled &&
          !props.busy &&
          !props.running &&
          next.observation &&
          !next.observation.snapshot.marked &&
          (!props.command ||
            (next.observation.receipt?.owner === props.command.owner &&
              next.observation.receipt.sequence === props.command.sequence))
        ) {
          return composerFrame(next, props, [{ name: 'send', value: next.observation }]);
        }
        if (input.kind === 'raw' && input.key === 'stop' && props.running && !props.busy)
          return composerFrame(next, props, [{ name: 'stop', value: {} }]);
        return composerFrame(next, props);
      },
      capture(_state, _props, method, input) {
        return { kind: 'forward', source: { child: 'draft', method, input } };
      },
    },
    navigation: {
      mount: (props) => navigationFrame({ revision: 0 }, props),
      update: (state, props) => navigationFrame({ revision: state.revision + 1 }, props),
      event(state, input, props) {
        const out = navigationFrame({ revision: state.revision + 1 }, props);
        if (input.kind === 'raw' && !props.busy) {
          if (input.key === 'new') out.events.push({ name: 'create', value: {} });
          else if (input.key === 'more') out.events.push({ name: 'more', value: {} });
          else {
            const session = props.sessions.find((session) => session.id === input.key);
            if (session) out.events.push({ name: 'select', value: { sessionId: session.id } });
          }
        }
        return out;
      },
    },
    timeline: {
      mount: (props) =>
        timelineFrame(
          {
            revision: 0,
            observation: null,
            command:
              props.landing.sequence > 0
                ? { sequence: 1, target: { kind: props.landing.edge } }
                : null,
            landing: props.landing,
          },
          props,
        ),
      update(state, props) {
        if (
          props.landing.sequence < state.landing.sequence ||
          (props.landing.sequence === state.landing.sequence &&
            props.landing.edge !== state.landing.edge)
        )
          throw new Error('Invalid reading landing sequence');
        return timelineFrame(
          {
            ...state,
            revision: state.revision + 1,
            command:
              state.landing.sequence !== props.landing.sequence
                ? {
                    sequence: (state.command?.sequence ?? 0) + 1,
                    target: { kind: props.landing.edge },
                  }
                : state.command,
            landing: props.landing,
          },
          props,
        );
      },
      event(state, input, props) {
        const next = { ...state, revision: state.revision + 1 };
        if (input.kind === 'component' && input.child === 'reading') {
          if (input.name === 'observed') next.observation = input.event as ReadingObservation;
          if (input.name === 'input') input = input.event as ComponentInput;
        }
        if (
          input.kind === 'raw' &&
          input.key === 'end' &&
          target === 'desktop' &&
          next.observation?.viewport &&
          !next.observation.viewport.atEnd &&
          next.observation.pendingCommand == null
        )
          next.command = { sequence: (next.command?.sequence ?? 0) + 1, target: { kind: 'end' } };
        const out = timelineFrame(next, props);
        if (input.kind === 'raw' && !props.loading) {
          if (input.key === 'older' && props.older) out.events.push({ name: 'older', value: {} });
          if (input.key === 'newer' && props.newer) out.events.push({ name: 'newer', value: {} });
          if (input.key === 'latest' && props.latest)
            out.events.push({ name: 'latest', value: {} });
        }
        return out;
      },
    },
  };
}
export function exports(): Record<string, UiExport> {
  return {
    Composer: { kind: 'component', contract: composerContract, component: ['composer'] },
    Navigation: { kind: 'component', contract: navigationContract, component: ['navigation'] },
    Timeline: { kind: 'component', contract: timelineContract, component: ['timeline'] },
  };
}
