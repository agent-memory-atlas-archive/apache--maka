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

/** Typed declarations for the actual native scene protocols. */
import type {
  DesktopScene,
  TerminalScene,
  ComponentSlot,
  ReadingProps,
  InputProps,
  TextareaProps,
  LabelProps,
  MarkdownProps,
  FilePickerProps,
  ImageProps,
  SourceTextProps,
  SelectionMenuProps,
} from './generated/runtime.js';

export type Presentation = 'desktop' | 'tui';
const sceneTarget: unique symbol = Symbol('maka.scene.target');
export interface SceneTarget<T extends Presentation> {
  readonly [sceneTarget]: T;
}

type Declaration<Wire, T extends Presentation> = Wire extends unknown
  ? {
      [K in keyof Wire]: K extends 'children'
        ? Scene<T>[]
        : K extends 'background' | 'content' | 'trigger'
          ? Wire[K] extends DesktopScene | TerminalScene
            ? Scene<T>
            : Wire[K]
          : Wire[K];
    } & SceneTarget<T>
  : never;
export type DesktopNode = Declaration<DesktopScene, 'desktop'>;
export type TerminalNode = Declaration<TerminalScene, 'tui'>;
export type Scene<T extends Presentation> = T extends 'desktop' ? DesktopNode : TerminalNode;
export type DesktopKind<K extends DesktopNode['kind']> = Extract<DesktopNode, { kind: K }>;
export type TerminalKind<K extends TerminalNode['kind']> = Extract<TerminalNode, { kind: K }>;
export type DesktopProps<K extends DesktopNode['kind']> = Props<DesktopKind<K>>;
export type TerminalProps<K extends TerminalNode['kind']> = Props<TerminalKind<K>>;
type Props<N extends SceneTarget<Presentation>> = Omit<N, 'kind' | typeof sceneTarget> &
  Partial<Pick<N, typeof sceneTarget>>;
export type ComponentDeclaration<P = unknown> = Omit<ComponentSlot, 'props' | 'member'> & {
  member?: number;
  props: P;
};
type ComponentFor<P, T extends Presentation> = ComponentDeclaration<P> & Partial<SceneTarget<T>>;
export type ReadingDeclaration<T extends Presentation> = Omit<ReadingProps, 'content'> & {
  content: Scene<T>;
};

// This symbol distinguishes authoring targets only. JSON ignores it; Rust
// continues to admit the original scene and component contracts.
function node<T extends Presentation, const N extends object>(
  target: T,
  value: N,
): N & SceneTarget<T> {
  const result = Object.assign(value, { [sceneTarget]: target });
  Object.defineProperty(result, sceneTarget, { enumerable: false });
  return result;
}

function component<T extends Presentation, P>(target: T, value: ComponentFor<P, T>) {
  return node(target, { ...value, member: value.member ?? 0, kind: 'component' });
}

export const ui = {
  desktop: {
    text(value: Props<DesktopKind<'text'>>): DesktopKind<'text'> {
      return node('desktop', { ...value, kind: 'text' });
    },
    button(value: Props<DesktopKind<'button'>>): DesktopKind<'button'> {
      return node('desktop', { ...value, kind: 'button' });
    },
    stack(value: Props<DesktopKind<'stack'>>): DesktopKind<'stack'> {
      return node('desktop', { ...value, kind: 'stack' });
    },
    dialog(value: Props<DesktopKind<'dialog'>>): DesktopKind<'dialog'> {
      return node('desktop', { ...value, kind: 'dialog' });
    },
    popup(value: Props<DesktopKind<'popup'>>): DesktopKind<'popup'> {
      return node('desktop', { ...value, kind: 'popup' });
    },
    canvas(value: Props<DesktopKind<'canvas'>>): DesktopKind<'canvas'> {
      return node('desktop', { ...value, kind: 'canvas' });
    },
    component<P>(value: ComponentFor<P, 'desktop'>): DesktopKind<'component'> {
      return component('desktop', value);
    },
    input(value: ComponentFor<InputProps, 'desktop'>): DesktopKind<'component'> {
      return component('desktop', value);
    },
    textarea(value: ComponentFor<TextareaProps, 'desktop'>): DesktopKind<'component'> {
      return component('desktop', value);
    },
    label(value: ComponentFor<LabelProps, 'desktop'>): DesktopKind<'component'> {
      return component('desktop', value);
    },
    markdown(value: ComponentFor<MarkdownProps, 'desktop'>): DesktopKind<'component'> {
      return component('desktop', value);
    },
    filePicker(value: ComponentFor<FilePickerProps, 'desktop'>): DesktopKind<'component'> {
      return component('desktop', value);
    },
    image(value: ComponentFor<ImageProps, 'desktop'>): DesktopKind<'component'> {
      return component('desktop', value);
    },
    sourceText(value: ComponentFor<SourceTextProps, 'desktop'>): DesktopKind<'component'> {
      return component('desktop', value);
    },
    selectionMenu(value: ComponentFor<SelectionMenuProps, 'desktop'>): DesktopKind<'component'> {
      return component('desktop', value);
    },
    reading(
      value: ComponentFor<ReadingDeclaration<'desktop'>, 'desktop'>,
    ): DesktopKind<'component'> {
      return component('desktop', value);
    },
  },
  terminal: {
    text(value: Props<TerminalKind<'text'>>): TerminalKind<'text'> {
      return node('tui', { ...value, kind: 'text' });
    },
    button(value: Props<TerminalKind<'button'>>): TerminalKind<'button'> {
      return node('tui', { ...value, kind: 'button' });
    },
    stack(value: Props<TerminalKind<'stack'>>): TerminalKind<'stack'> {
      return node('tui', { ...value, kind: 'stack' });
    },
    dialog(value: Props<TerminalKind<'dialog'>>): TerminalKind<'dialog'> {
      return node('tui', { ...value, kind: 'dialog' });
    },
    component<P>(value: ComponentFor<P, 'tui'>): TerminalKind<'component'> {
      return component('tui', value);
    },
    input(value: ComponentFor<InputProps, 'tui'>): TerminalKind<'component'> {
      return component('tui', value);
    },
    textarea(value: ComponentFor<TextareaProps, 'tui'>): TerminalKind<'component'> {
      return component('tui', value);
    },
    label(value: ComponentFor<LabelProps, 'tui'>): TerminalKind<'component'> {
      return component('tui', value);
    },
    markdown(value: ComponentFor<MarkdownProps, 'tui'>): TerminalKind<'component'> {
      return component('tui', value);
    },
    filePicker(value: ComponentFor<FilePickerProps, 'tui'>): TerminalKind<'component'> {
      return component('tui', value);
    },
    image(value: ComponentFor<ImageProps, 'tui'>): TerminalKind<'component'> {
      return component('tui', value);
    },
    sourceText(value: ComponentFor<SourceTextProps, 'tui'>): TerminalKind<'component'> {
      return component('tui', value);
    },
    selectionMenu(value: ComponentFor<SelectionMenuProps, 'tui'>): TerminalKind<'component'> {
      return component('tui', value);
    },
    reading(value: ComponentFor<ReadingDeclaration<'tui'>, 'tui'>): TerminalKind<'component'> {
      return component('tui', value);
    },
  },
};
