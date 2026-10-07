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

//! Original Kit plain-text participant, with native focus and exact Copy.
use crate::{
    LiveInput, Slots, Widget,
    appearance::{color, font},
};
use gpui_kit::base::{SelectableText, TextSelection, TextSelectionHandle};
use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Window, div, px, relative,
};
use maka_ui::{
    ComponentCall, ComponentOutput, Frame, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView, source::*,
};
use serde_json::json;
use std::{any::Any, collections::BTreeMap, rc::Rc};
struct Owner {
    handle: TextSelectionHandle,
    focus: FocusHandle,
    props: SourceTextProps,
    visible: bool,
}
impl Owner {
    fn enabled(&self) -> bool {
        self.visible && !self.props.disabled
    }
    fn new(
        props: &SourceTextProps,
        input: LiveInput,
        window: &Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let handle = TextSelectionHandle::new(props.source.clone(), cx);
        handle.preserve_byte_offsets(cx);
        let keep = input
            .0
            .borrow()
            .as_ref()
            .expect("live source mount")
            .clone();
        let owner = cx.new(|cx| {
            cx.on_release(move |_, _| drop(keep)).detach();
            Owner {
                handle: handle.clone(),
                focus: cx.focus_handle(),
                props: props.clone(),
                visible: true,
            }
        });
        let weak = owner.downgrade();
        handle.focus_with(
            move |window, cx| {
                if let Some(owner) = weak.upgrade()
                    && owner.read(cx).enabled()
                {
                    owner.read(cx).focus.clone().focus(window, cx);
                }
            },
            cx,
        );
        handle.refresh_window_on_change(window, cx).detach();
        owner
    }
    fn apply(&mut self, props: &SourceTextProps, window: &mut Window, cx: &mut Context<Self>) {
        if self.props.source != props.source {
            if !props.source.starts_with(&self.props.source) {
                self.handle.set_enabled(false, cx);
            }
            self.handle.set_fallback_copy_text(props.source.clone(), cx);
        }
        self.props = props.clone();
        self.enable(self.visible, window, cx);
        cx.notify();
    }
    fn enable(&mut self, visible: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.visible = visible;
        self.handle.set_enabled(self.enabled(), cx);
        if !self.enabled() && self.focus.contains_focused(window, cx) {
            window.blur(cx);
        }
    }
}
impl Render for Owner {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let copy = cx.entity().downgrade();
        let select = copy.clone();
        let a = &self.props.appearance;
        div()
            .w_full()
            .track_focus(&self.focus)
            .key_context("TextView")
            .font(font(&a.font))
            .text_size(px(a.font.size))
            .line_height(relative(a.font.line_height))
            .text_color(color(a.foreground))
            .on_action(move |_: &gpui_kit::base::input::Copy, window, cx| {
                let Some(owner) = copy.upgrade() else {
                    cx.propagate();
                    return;
                };
                if !owner.read(cx).enabled() {
                    cx.propagate();
                    return;
                }
                let handle = owner.read(cx).handle.clone();
                if let Some(text) = handle
                    .selected_text_in_scope(window, cx)
                    .filter(|text| !text.is_empty())
                {
                    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text));
                } else {
                    cx.propagate();
                }
            })
            .on_action(move |_: &gpui_kit::base::input::SelectAll, window, cx| {
                let Some(owner) = select.upgrade() else {
                    cx.propagate();
                    return;
                };
                if !owner.read(cx).enabled() {
                    cx.propagate();
                    return;
                }
                let handle = owner.read(cx).handle.clone();
                if !handle.activate_scope(window, cx) {
                    cx.propagate();
                    return;
                }
                TextSelection::clear(window, cx);
                handle.set_local_selection(true, cx);
                window.refresh();
            })
            .child(
                SelectableText::with_handle(
                    "source",
                    self.handle.clone(),
                    self.props.source.clone(),
                )
                .selection_color(color(self.props.selection))
                .focus_handle(&self.focus),
            )
    }
}
struct Recipe(SourceTextProps);
impl Widget for Recipe {
    fn create(&self, input: LiveInput, window: &mut Window, cx: &mut App) -> Box<dyn Any> {
        Box::new(Owner::new(&self.0, input, window, cx))
    }
    fn commit(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<Owner>>()
            .expect("source owner")
            .update(cx, |owner, cx| owner.apply(&self.0, window, cx));
    }
    fn presentation(&self, state: &mut dyn Any, visible: bool, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<Owner>>()
            .expect("source owner")
            .update(cx, |owner, cx| owner.enable(visible, window, cx));
    }
    fn deactivate(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        self.presentation(state, false, window, cx);
    }
    fn render(
        &self,
        state: &dyn Any,
        _: &Slots,
        _: &mut Window,
        _: &mut App,
    ) -> maka_ui::Result<AnyElement> {
        Ok(state
            .downcast_ref::<Entity<Owner>>()
            .expect("source owner")
            .clone()
            .into_any_element())
    }
}
struct Component;
impl NativeComponent<dyn Widget> for Component {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let (props, revision) = match call {
            ComponentCall::Mount { props } => (props, 0),
            ComponentCall::Update { props, state } => (
                props,
                state["revision"].as_u64().expect("source revision") + 1,
            ),
            ComponentCall::Event { .. } => {
                return Err(maka_ui::Error::Invalid(
                    "SourceText has no component events".into(),
                ));
            }
        };
        let declaration = props.clone();
        let props: SourceTextProps = serde_json::from_value(props.clone())
            .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
        Ok(Frame {
            revision,
            state: json!({"revision":revision}),
            surface: None,
            view: NativeView::<dyn Widget>::new(Rc::new(Recipe(props)), BTreeMap::new(), vec![])?
                .with_declaration(declaration)?,
        }
        .output(vec![]))
    }
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register("maka.ui.source", |_, _| {
            Ok(BTreeMap::from([(
                "SourceText".into(),
                NativeExport {
                    contract: source_contract(),
                    value: NativeCapability::Component(Rc::new(Component)),
                },
            )]))
        })
        .expect("unique source implementation");
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::base::{ElementExt, TextSelectionLayer, TextSelectionScopeId};
    use gpui_kit::{TestAppContext, point};
    struct Root {
        owner: Entity<Owner>,
        scope: TextSelectionScopeId,
    }
    impl Render for Root {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(320.))
                .h(px(120.))
                .child(TextSelectionLayer)
                .child(self.owner.clone().text_selection_scope(self.scope))
        }
    }
    #[gpui::test]
    fn literal_source_select_all_and_copy_preserve_whitespace_scope_and_owner(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let source = "  **literal** <tag> 中文😀\n\n  ";
        let props: SourceTextProps = serde_json::from_value(json!({ "source":source,
            "appearance":{"font":{"family":"Helvetica","fallbacks":[],"size":16,"lineHeight":1.5,"weight":400},"foreground":0x242724ffu32},
            "selection":0x4466ff55u32,"disabled":false })).unwrap();
        let scope = TextSelectionScopeId::new();
        let (root, window_cx) = cx.add_window_view(|window, cx| {
            let input = LiveInput::new(
                Rc::new(std::cell::RefCell::new(Some(crate::Input(Rc::new(
                    |_, _, _, _| Ok(Default::default()),
                ))))),
                crate::NativeBudget::default(),
                std::sync::Arc::new(maka_plugins::call::Resources::default()),
            );
            Root {
                owner: Owner::new(&props, input, window, cx),
                scope,
            }
        });
        window_cx.update(|window, cx| window.draw(cx).clear(cx));
        let owner = window_cx.update(|_, cx| root.read(cx).owner.clone());
        let participant = window_cx.update(|_, cx| owner.read(cx).handle.entity_id());
        window_cx.update(|window, cx| {
            owner.read(cx).focus.clone().focus(window, cx);
            owner.read(cx).focus.clone().dispatch_action(
                &gpui_kit::base::input::SelectAll,
                window,
                cx,
            );
            assert!(TextSelection::has_selection_in_scope(scope, window, cx));
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("sentinel".into()));
            owner
                .read(cx)
                .focus
                .clone()
                .dispatch_action(&gpui_kit::base::input::Copy, window, cx);
            assert_eq!(cx.read_from_clipboard().unwrap().text().unwrap(), source);
            window.draw(cx).clear(cx);
            assert_eq!(TextSelection::selected_text(window, cx), source);
            assert!(!TextSelection::has_selection_in_scope(
                TextSelectionScopeId::new(),
                window,
                cx
            ));
            assert_eq!(owner.read(cx).handle.entity_id(), participant);
            let mut appended = props.clone();
            appended.source.push_str("append");
            owner.update(cx, |owner, cx| owner.apply(&appended, window, cx));
            assert!(TextSelection::has_selection_in_scope(scope, window, cx));
            window.draw(cx).clear(cx);
            assert_eq!(
                owner
                    .read(cx)
                    .handle
                    .clone()
                    .selected_text_in_scope(window, cx),
                Some(appended.source)
            );
            owner.update(cx, |owner, cx| owner.enable(false, window, cx));
            assert!(!TextSelection::has_selection_in_scope(scope, window, cx));
            assert!(window.focused(cx).is_none());
            let mut next = props.clone();
            next.source = "replacement".into();
            owner.update(cx, |owner, cx| {
                owner.apply(&next, window, cx);
                owner.enable(true, window, cx);
            });
            window.draw(cx).clear(cx);
            assert_eq!(owner.read(cx).handle.entity_id(), participant);
            assert!(
                owner
                    .read(cx)
                    .handle
                    .clone()
                    .selected_text_in_scope(window, cx)
                    .is_none()
            );
        });
        window_cx.simulate_mouse_down(
            point(px(1.), px(12.)),
            gpui_kit::MouseButton::Left,
            Default::default(),
        );
        window_cx.simulate_mouse_move(
            point(px(200.), px(12.)),
            Some(gpui_kit::MouseButton::Left),
            Default::default(),
        );
        window_cx.simulate_mouse_up(
            point(px(200.), px(12.)),
            gpui_kit::MouseButton::Left,
            Default::default(),
        );
        window_cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(
                owner
                    .read(cx)
                    .handle
                    .clone()
                    .selected_text_in_scope(window, cx)
                    .as_deref(),
                Some("replacement")
            );
            assert!(TextSelection::has_selection_in_scope(scope, window, cx));
        });
    }
    struct Peers {
        owners: Vec<(Entity<Owner>, TextSelectionScopeId)>,
        height: Rc<std::cell::Cell<f32>>,
    }
    impl Render for Peers {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let height = self.height.clone();
            div()
                .flex()
                .flex_col()
                .w(px(100.))
                .child(TextSelectionLayer)
                .children(self.owners.iter().map(move |(owner, scope)| {
                    let height = height.clone();
                    div()
                        .w_full()
                        .child(owner.clone())
                        .on_prepaint(move |bounds, _, _| height.set(f32::from(bounds.size.height)))
                        .text_selection_scope(*scope)
                }))
        }
    }
    #[gpui::test]
    fn source_copy_spans_same_scope_and_excludes_other_scope(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let make = |source: &str| {
            serde_json::from_value::<SourceTextProps>(json!({"source":source,
            "appearance":{"font":{"family":"Helvetica","fallbacks":[],"size":16,"lineHeight":1.5,"weight":400},"foreground":0x242724ffu32},"selection":0x4466ff55u32,"disabled":false})).unwrap()
        };
        let scope = TextSelectionScopeId::new();
        let (root, w) = cx.add_window_view(|window, cx| {
            let mut owners = vec![];
            for (source, scope) in [
                ("one", scope),
                ("two", scope),
                ("other", TextSelectionScopeId::new()),
            ] {
                let input = LiveInput::new(
                    Rc::new(std::cell::RefCell::new(Some(crate::Input(Rc::new(
                        |_, _, _, _| Ok(Default::default()),
                    ))))),
                    crate::NativeBudget::default(),
                    std::sync::Arc::new(maka_plugins::call::Resources::default()),
                );
                owners.push((Owner::new(&make(source), input, window, cx), scope));
            }
            Peers {
                owners,
                height: Default::default(),
            }
        });
        w.update(|window, cx| window.draw(cx).clear(cx));
        w.simulate_mouse_down(
            point(px(1.), px(12.)),
            gpui_kit::MouseButton::Left,
            Default::default(),
        );
        w.simulate_mouse_move(
            point(px(90.), px(36.)),
            Some(gpui_kit::MouseButton::Left),
            Default::default(),
        );
        w.simulate_mouse_up(
            point(px(90.), px(36.)),
            gpui_kit::MouseButton::Left,
            Default::default(),
        );
        w.update(|window, cx| {
            let first = root.read(cx).owners[0].0.clone();
            let other = root.read(cx).owners[2].0.clone();
            other.read(cx).handle.clone().set_local_selection(true, cx);
            let handle = first.read(cx).handle.clone();
            assert_eq!(
                handle.selected_text_in_scope(window, cx).as_deref(),
                Some("one\ntwo")
            );
            first.read(cx).focus.clone().focus(window, cx);
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("sentinel".into()));
            first
                .read(cx)
                .focus
                .clone()
                .dispatch_action(&gpui_kit::base::input::Copy, window, cx);
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().unwrap(),
                "one\ntwo"
            );
        });
    }
    #[gpui::test]
    fn selected_literal_word_keeps_byte_range_when_append_wraps_it(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let props: SourceTextProps=serde_json::from_value(json!({"source":"alpha wor",
            "appearance":{"font":{"family":"Helvetica","fallbacks":[],"size":16,"lineHeight":1.5,"weight":400},"foreground":0x242724ffu32},"selection":0x4466ff55u32,"disabled":false})).unwrap();
        let (root, w) = cx.add_window_view(|window, cx| {
            let input = LiveInput::new(
                Rc::new(std::cell::RefCell::new(Some(crate::Input(Rc::new(
                    |_, _, _, _| Ok(Default::default()),
                ))))),
                crate::NativeBudget::default(),
                std::sync::Arc::new(maka_plugins::call::Resources::default()),
            );
            Peers {
                owners: vec![(
                    Owner::new(&props, input, window, cx),
                    TextSelectionScopeId::new(),
                )],
                height: Default::default(),
            }
        });
        w.update(|window, cx| window.draw(cx).clear(cx));
        w.simulate_mouse_down(
            point(px(70.), px(12.)),
            gpui_kit::MouseButton::Left,
            Default::default(),
        );
        w.simulate_mouse_move(
            point(px(90.), px(12.)),
            Some(gpui_kit::MouseButton::Left),
            Default::default(),
        );
        w.simulate_mouse_up(
            point(px(90.), px(12.)),
            gpui_kit::MouseButton::Left,
            Default::default(),
        );
        w.update(|window, cx| {
            let owner = root.read(cx).owners[0].0.clone();
            let handle = owner.read(cx).handle.clone();
            let height = root.read(cx).height.clone();
            let before = height.get();
            let selected = handle.selected_text_in_scope(window, cx).unwrap();
            assert_eq!(selected, "or");
            let mut append = props.clone();
            append.source.push_str("ldmore");
            owner.update(cx, |owner, cx| owner.apply(&append, window, cx));
            window.draw(cx).clear(cx);
            assert!(
                height.get() > before,
                "append must actually wrap the selected word to another line"
            );
            assert_eq!(handle.selected_text_in_scope(window, cx).unwrap(), selected);
        });
        // Same old screen point now resolves to another byte after wrapping.
        // Shift-click keeps the original byte anchor, not a collapsed geometry pair.
        let shift = gpui_kit::Modifiers {
            shift: true,
            ..Default::default()
        };
        w.simulate_mouse_down(point(px(70.), px(12.)), gpui_kit::MouseButton::Left, shift);
        w.simulate_mouse_up(point(px(70.), px(12.)), gpui_kit::MouseButton::Left, shift);
        w.update(|window, cx| {
            let owner = root.read(cx).owners[0].0.clone();
            let handle = owner.read(cx).handle.clone();
            assert_eq!(
                handle.selected_text_in_scope(window, cx).as_deref(),
                Some("w")
            );
            assert!(TextSelection::has_selection_in_scope(
                root.read(cx).owners[0].1,
                window,
                cx
            ));
        });
    }

    struct Mixed {
        source: Entity<Owner>,
        document: crate::markdown::TestDocument,
        scope: TextSelectionScopeId,
    }
    impl Render for Mixed {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let slots = Slots::new(
                vec![],
                crate::Input(Rc::new(|_, _, _, _| Ok(Default::default()))),
                |_, _, _| unreachable!(),
            );
            div()
                .flex()
                .flex_col()
                .w(px(320.))
                .child(TextSelectionLayer)
                .child(self.source.clone().text_selection_scope(self.scope))
                .child(
                    self.document
                        .widget
                        .render(self.document.state.as_ref(), &slots, window, cx)
                        .unwrap()
                        .text_selection_scope(self.scope),
                )
        }
    }
    #[gpui::test]
    fn source_copy_includes_original_markdown_callback_in_the_same_scope(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let props: SourceTextProps=serde_json::from_value(json!({"source":"one",
            "appearance":{"font":{"family":"Helvetica","fallbacks":[],"size":16,"lineHeight":1.5,"weight":400},"foreground":0x242724ffu32},"selection":0x4466ff55u32,"disabled":false})).unwrap();
        let text = serde_json::to_value(&props.appearance).unwrap();
        let markdown: maka_ui::markdown::MarkdownProps=serde_json::from_value(json!({"source":"**two**","selectable":true,"disabled":false,
            "appearance":{"body":text,"code":text,"headings":[text,text,text,text,text,text],"muted":0x444444ffu32,"link":0x004499ffu32,"selection":0x4466ff55u32,"codeBackground":0,"tableBackground":0,"border":0,"dark":false,"paragraphGap":8}})).unwrap();
        let (root, w) = cx.add_window_view(|window, cx| {
            let input = || {
                LiveInput::new(
                    Rc::new(std::cell::RefCell::new(Some(crate::Input(Rc::new(
                        |_, _, _, _| Ok(Default::default()),
                    ))))),
                    crate::NativeBudget::default(),
                    std::sync::Arc::new(maka_plugins::call::Resources::default()),
                )
            };
            Mixed {
                source: Owner::new(&props, input(), window, cx),
                document: crate::markdown::test_document(&markdown, input(), window, cx),
                scope: TextSelectionScopeId::new(),
            }
        });
        w.update(|window, cx| window.draw(cx).clear(cx));
        w.simulate_mouse_down(
            point(px(1.), px(12.)),
            gpui_kit::MouseButton::Left,
            Default::default(),
        );
        w.simulate_mouse_move(
            point(px(200.), px(36.)),
            Some(gpui_kit::MouseButton::Left),
            Default::default(),
        );
        w.simulate_mouse_up(
            point(px(200.), px(36.)),
            gpui_kit::MouseButton::Left,
            Default::default(),
        );
        w.update(|window, cx| {
            let source = root.read(cx).source.clone();
            let handle = source.read(cx).handle.clone();
            assert_eq!(
                handle.selected_text_in_scope(window, cx).as_deref(),
                Some("one\ntwo\n")
            );
            source.read(cx).focus.clone().focus(window, cx);
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("sentinel".into()));
            source
                .read(cx)
                .focus
                .clone()
                .dispatch_action(&gpui_kit::base::input::Copy, window, cx);
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().unwrap(),
                "one\ntwo\n"
            );
        });
    }
}
