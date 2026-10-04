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
import { paint, slot, output, fill, pixels, type Target, type Node } from './view.js';
interface Revision {
  revision: number;
}
interface Draft extends Revision {
  observation: EditorObservation | null;
}
export function components(target: Target): {
  composer: Component<Draft, ComposerProps>;
  navigation: Component<Revision, NavigationProps>;
  timeline: Component<Revision, TimelineProps>;
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
          ...props.sessions.map((session, index) =>
            ui.box('session-' + index, [ui.button(session.id, session.title, props.busy)], {
              surface: session.id === props.selected,
              style: { radius: 8 },
            }),
          ),
          ...(props.more ? [ui.button('more', text.moreSessions, props.busy)] : []),
        ],
        {
          surface: true,
          padding: target === 'desktop' ? 20 : 1,
          style: { height: fill, width: fill },
        },
      ),
    );
  };
  const timelineFrame = (state: Revision, props: TimelineProps) => {
    const ui = paint(target, props.theme),
      text = copy(props.locale);
    const messages: Node[] = props.messages.map((message, index) =>
      ui.box(
        'message-' + index,
        [
          ui.text(
            'role-' + index,
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
            ? [ui.text('continued-' + index, text.partial, 12, props.theme.muted)]
            : []),
          ui.text('text-' + index, message.text, 16),
        ],
        {
          gap: target === 'desktop' ? 8 : 0,
          style: { width: fill, maxWidth: 760 },
          surface: message.role === 'user',
          padding: target === 'desktop' && message.role === 'user' ? 16 : 0,
        },
      ),
    );
    return output(
      state,
      ui.box(
        'timeline',
        [
          ...(!messages.length && !props.loading
            ? [
                ui.text('welcome', text.welcome, 30, props.theme.text, 600),
                ui.text('welcome-copy', text.welcomeCopy, 16, props.theme.muted),
              ]
            : messages),
          ...(props.loading ? [ui.text('loading', text.loading, 14, props.theme.muted)] : []),
          ...(props.more ? [ui.button('more', text.more)] : []),
        ],
        {
          gap: target === 'desktop' ? 24 : 1,
          style: {
            width: fill,
            height: fill,
            padding: { top: 24, right: 24, bottom: 24, left: 24 },
          },
          scroll: true,
        },
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
      mount: (props) => timelineFrame({ revision: 0 }, props),
      update: (state, props) => timelineFrame({ revision: state.revision + 1 }, props),
      event(state, input, props) {
        const out = timelineFrame({ revision: state.revision + 1 }, props);
        if (input.kind === 'raw' && input.key === 'more' && props.more && !props.loading)
          out.events.push({ name: 'more', value: {} });
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
