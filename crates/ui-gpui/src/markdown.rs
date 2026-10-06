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

//! Original Kit document custody; immutable recipes carry only presentation inputs.
use crate::{
    LiveInput, Slots, Widget,
    appearance::{color, font},
};
use gpui_kit::base::text::{TextView, TextViewState, TextViewStyle};
use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, HighlightStyle, ImageCacheError, ImageSource,
    IntoElement, Render, Styled, Window, div, px, relative, rems,
};
use maka_ui::{
    ComponentCall, ComponentOutput, Emission, Frame, NativeCapability, NativeComponent,
    NativeExport, NativeRegistry, NativeView, markdown::*,
};
use serde_json::json;
use std::{any::Any, collections::BTreeMap, rc::Rc, sync::Arc};

struct Owner {
    document: Entity<TextViewState>,
    input: LiveInput,
    appearance: MarkdownAppearance,
    selectable: bool,
    disabled: bool,
    visible: bool,
}
impl Owner {
    fn new(props: &MarkdownProps, input: LiveInput, cx: &mut App) -> Entity<Self> {
        let keep = input
            .0
            .borrow()
            .as_ref()
            .expect("live Markdown mount")
            .clone();
        let document = cx.new(|cx| {
            let mut state = TextViewState::markdown(&props.source, cx);
            state.set_selectable(props.selectable, cx);
            state.set_interaction_enabled(!props.disabled, cx);
            cx.on_release(move |_, _| drop(keep)).detach();
            state
        });
        cx.new(|_| Self {
            document,
            input,
            appearance: props.appearance.clone(),
            selectable: props.selectable,
            disabled: props.disabled,
            visible: true,
        })
    }
    fn apply(&mut self, props: &MarkdownProps, window: &mut Window, cx: &mut Context<Self>) {
        self.document.update(cx, |state, cx| {
            state.set_text(&props.source, cx);
            if self.selectable != props.selectable {
                state.set_selectable(props.selectable, cx);
            }
            state.set_interaction_enabled(self.visible && !props.disabled, cx);
            if props.disabled && state.focus_handle().contains_focused(window, cx) {
                window.blur(cx);
            }
        });
        if self.selectable != props.selectable {
            cx.notify();
        }
        self.selectable = props.selectable;
        self.disabled = props.disabled;
        if self.appearance != props.appearance {
            self.appearance = props.appearance.clone();
            cx.notify();
        }
    }
    fn enable(&mut self, visible: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.visible = visible;
        self.document.update(cx, |state, cx| {
            state.set_interaction_enabled(visible && !self.disabled, cx);
            if !visible && state.focus_handle().contains_focused(window, cx) {
                window.blur(cx);
            }
        });
    }
}
fn web_target(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://"))
        && !url.chars().any(char::is_control)
}
impl Render for Owner {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let a = &self.appearance;
        let mut code = div()
            .font(font(&a.code.font))
            .text_size(px(a.code.font.size))
            .line_height(relative(a.code.font.line_height))
            .text_color(color(a.code.foreground));
        let headings = a.headings.clone();
        let style = TextViewStyle::default()
            .with_foreground(color(a.body.foreground))
            .with_muted_foreground(color(a.muted))
            .with_link(color(a.link))
            .with_selection(color(a.selection))
            .with_code_background(color(a.code_background))
            .with_table_background(color(a.table_background))
            .with_border(color(a.border))
            .with_dark(a.dark)
            .with_paragraph_gap(rems(a.paragraph_gap / f32::from(window.rem_size())))
            .with_code_font_family(a.code.font.family.0.clone())
            .with_code_font_size(px(a.code.font.size))
            .with_code_block(code.style().clone())
            .with_inline_code(HighlightStyle {
                color: Some(color(a.code.foreground)),
                background_color: Some(color(a.code_background)),
                ..Default::default()
            })
            .with_heading(move |level| {
                let heading = &headings[usize::from(level.saturating_sub(1)).min(5)];
                let mut element = div()
                    .font(font(&heading.font))
                    .text_size(px(heading.font.size))
                    .line_height(relative(heading.font.line_height))
                    .text_color(color(heading.foreground));
                element.style().clone()
            });
        let owner = cx.entity().downgrade();
        TextView::new(&self.document)
            .w_full()
            .selectable(self.selectable)
            .style(style)
            .font(font(&a.body.font))
            .text_size(px(a.body.font.size))
            .line_height(relative(a.body.font.line_height))
            // Literal document URLs do not confer network or filesystem access.
            .image_source(|_| {
                ImageSource::Custom(Arc::new(|_, _| {
                    Some(Err(ImageCacheError::Asset(
                        "document image requires an explicit resource".into(),
                    )))
                }))
            })
            .on_link_activation(move |url, _, window, cx| {
                let denied = gpui_kit::base::text::LinkDecision {
                    prevent_default: true,
                    stop_propagation: true,
                };
                let Some(owner) = owner.upgrade() else {
                    return denied;
                };
                let input = {
                    let owner = owner.read(cx);
                    if !owner.visible || owner.disabled {
                        return denied;
                    }
                    owner.input.0.borrow().clone()
                };
                let Some(input) = input else {
                    return denied;
                };
                let Ok(decision) = (input.0)("link", json!({"url":url.as_str()}), window, cx)
                else {
                    return denied;
                };
                // Original Kit rechecks occurrence admission after this synchronous batch.
                gpui_kit::base::text::LinkDecision {
                    prevent_default: decision.prevent_default || !web_target(url),
                    stop_propagation: decision.stop_propagation,
                }
            })
    }
}
struct Recipe(MarkdownProps);
impl Widget for Recipe {
    fn create(&self, input: LiveInput, _: &mut Window, cx: &mut App) -> Box<dyn Any> {
        Box::new(Owner::new(&self.0, input, cx))
    }
    fn commit(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<Owner>>()
            .expect("Markdown owner")
            .update(cx, |owner, cx| owner.apply(&self.0, window, cx));
    }
    fn presentation(&self, state: &mut dyn Any, visible: bool, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<Owner>>()
            .expect("Markdown owner")
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
            .expect("Markdown owner")
            .clone()
            .into_any_element())
    }
}
struct MarkdownComponent;
impl NativeComponent<dyn Widget> for MarkdownComponent {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let (props, revision, events) = match call {
            ComponentCall::Mount { props } => (props, 0, vec![]),
            ComponentCall::Update { state, props } => (
                props,
                state["revision"].as_u64().expect("Markdown revision") + 1,
                vec![],
            ),
            ComponentCall::Event {
                state,
                props,
                input,
                ..
            } => (
                props,
                state["revision"].as_u64().expect("Markdown revision") + 1,
                vec![Emission {
                    name: "link".into(),
                    value: input.clone(),
                }],
            ),
        };
        let props: MarkdownProps = serde_json::from_value(props.clone())
            .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
        Ok(Frame {
            revision,
            state: json!({"revision":revision}),
            surface: None,
            view: NativeView::<dyn Widget>::new(
                Rc::new(Recipe(props)),
                BTreeMap::from([("link".into(), link_schema())]),
                vec![],
            )?,
        }
        .output(events))
    }
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register("maka.ui.markdown", |_, _| {
            Ok(BTreeMap::from([(
                "Markdown".into(),
                NativeExport {
                    contract: markdown_contract(),
                    value: NativeCapability::Component(Rc::new(MarkdownComponent)),
                },
            )]))
        })
        .expect("unique native Markdown implementation");
}

#[cfg(test)]
pub(crate) struct TestDocument {
    pub widget: Rc<dyn Widget>,
    pub state: Box<dyn Any>,
    pub document: Entity<TextViewState>,
    pub notify: Box<dyn Fn(&mut App)>,
}

#[cfg(test)]
pub(crate) fn test_document(
    props: &MarkdownProps,
    input: LiveInput,
    window: &mut Window,
    cx: &mut App,
) -> TestDocument {
    let widget: Rc<dyn Widget> = Rc::new(Recipe(props.clone()));
    let mut state = widget.create(input, window, cx);
    widget.presentation(state.as_mut(), true, window, cx);
    widget.commit(state.as_mut(), window, cx);
    let original = state.downcast_ref::<Entity<Owner>>().unwrap().clone();
    let document = original.read(cx).document.clone();
    TestDocument {
        widget,
        state,
        document,
        notify: Box::new(move |cx| original.update(cx, |_, cx| cx.notify())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{
        ParentElement, TestAppContext, VisualContext, VisualTestContext, WeakEntity, accesskit,
    };
    use std::cell::{Cell, RefCell};

    fn props() -> MarkdownProps {
        let mut value = json!({
            "source":"# 中文 title\n\nA **bold** [link](https://example.test) and `code`.",
            "appearance": {
                "body":{"font":{"family":"Helvetica","fallbacks":[],"size":16,"lineHeight":1.55,"weight":400},"foreground":0x242724ffu32},
                "code":{"font":{"family":"Courier","fallbacks":[],"size":14,"lineHeight":1.55,"weight":400},"foreground":0x242724ffu32},
                "muted":0x656a63ffu32,"link":0x416653ffu32,"selection":0x41665340u32,
                "codeBackground":0xf0efebffu32,"tableBackground":0xfaf9f6ffu32,
                "border":0xd8ddd5ffu32,"dark":false,"paragraphGap":16
            },"selectable":true,"disabled":false
        });
        let body = value["appearance"]["body"].clone();
        value["appearance"]["headings"] = json!([
            body.clone(),
            body.clone(),
            body.clone(),
            body.clone(),
            body.clone(),
            body
        ]);
        serde_json::from_value(value).unwrap()
    }
    struct Root(Entity<Owner>);
    impl Render for Root {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(480.))
                .h(px(400.))
                .child(gpui_kit::base::TextSelectionLayer)
                .child(self.0.clone())
        }
    }
    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
    fn link(cx: &mut VisualTestContext) -> accesskit::NodeId {
        cx.cx
            .accessibility_update(cx.window_handle())
            .unwrap()
            .nodes
            .into_iter()
            .find(|(_, node)| node.role() == accesskit::Role::Link)
            .unwrap()
            .0
    }
    fn ax(cx: &mut VisualTestContext, id: accesskit::NodeId, action: accesskit::Action) {
        cx.cx.simulate_accessibility_action(
            cx.window_handle(),
            accesskit::ActionRequest {
                action,
                target_node: id,
                target_tree: accesskit::TreeId::ROOT,
                data: None,
            },
        );
    }
    #[gpui::test]
    fn committed_recipe_keeps_original_document_focus_and_native_copy(cx: &mut TestAppContext) {
        cx.update(gpui_kit::base::init);
        let input = LiveInput(Rc::new(RefCell::new(Some(crate::Input(Rc::new(
            |_, _, _, _| Ok(Default::default()),
        ))))));
        let (root, cx) = cx.add_window_view(|_, cx| Root(Owner::new(&props(), input, cx)));
        let owner = root.read_with(cx, |r, _| r.0.clone());
        cx.run_until_parked();
        draw(cx);
        let document = owner.read_with(cx, |o, _| o.document.clone());
        cx.update(|window, cx| {
            document.update(cx, |state, cx| {
                state.select_all(cx);
                state.focus_handle().focus(window, cx);
            });
        });
        cx.run_until_parked();
        draw(cx);
        let focused = cx.update(|window, cx| window.focused(cx).unwrap());
        assert_eq!(
            cx.update(gpui_kit::base::TextSelection::selected_text),
            "中文 title\nA bold link and code.\n"
        );
        let mut changed = props();
        changed.appearance.dark = true;
        changed.appearance.body.foreground = maka_ui::appearance::Color(0xe8eae6ff);
        cx.update(|window, cx| Recipe(changed.clone()).commit(&mut owner.clone(), window, cx));
        cx.run_until_parked();
        draw(cx);
        assert_eq!(
            owner.read_with(cx, |o, _| o.document.entity_id()),
            document.entity_id()
        );
        assert_eq!(cx.update(|window, cx| window.focused(cx).unwrap()), focused);
        assert_eq!(
            document.read_with(cx, |s, _| s.selected_text()),
            "中文 title\nA bold link and code.\n"
        );
        assert_eq!(
            cx.update(gpui_kit::base::TextSelection::selected_text),
            "中文 title\nA bold link and code.\n"
        );
        cx.dispatch_action(gpui_kit::base::input::Copy);
        assert_eq!(
            cx.update(|_, cx| cx.read_from_clipboard().unwrap().text().unwrap()),
            "中文 title\nA bold link and code."
        );
        changed.source.push_str("\n\nStreamed café 😀.");
        cx.update(|window, cx| Recipe(changed).commit(&mut owner.clone(), window, cx));
        cx.run_until_parked();
        draw(cx);
        assert_eq!(
            owner.read_with(cx, |o, _| o.document.entity_id()),
            document.entity_id()
        );
        cx.update(|_, cx| document.update(cx, |state, cx| state.select_all(cx)));
        cx.dispatch_action(gpui_kit::base::input::Copy);
        assert!(
            cx.read_from_clipboard()
                .unwrap()
                .text()
                .unwrap()
                .ends_with("Streamed café 😀.")
        );
    }
    #[gpui::test]
    fn original_ax_link_default_obeys_batch_failure_and_reentrant_disable(cx: &mut TestAppContext) {
        cx.update(gpui_kit::base::init);
        let mode = Rc::new(Cell::new(0));
        let locator = Rc::new(RefCell::new(None::<WeakEntity<Owner>>));
        let decision = mode.clone();
        let target = locator.clone();
        let input = LiveInput(Rc::new(RefCell::new(Some(crate::Input(Rc::new(
            move |key, event, window, cx| {
                assert_eq!(key, "link");
                assert!(
                    [
                        "https://example.test",
                        "javascript:alert(1)",
                        "file:///private/secret"
                    ]
                    .contains(&event["url"].as_str().unwrap())
                );
                match decision.get() {
                    0 => Ok(maka_ui::InputDecision {
                        prevent_default: true,
                        stop_propagation: false,
                    }),
                    1 => Err(maka_ui::Error::Invalid("failed plugin batch".into())),
                    2 => {
                        let owner = target.borrow().as_ref().unwrap().upgrade().unwrap();
                        let mut disabled = props();
                        disabled.disabled = true;
                        owner.update(cx, |o, cx| o.apply(&disabled, window, cx));
                        Ok(Default::default())
                    }
                    _ => Ok(Default::default()),
                }
            },
        ))))));
        let (root, cx) = cx.add_window_view(|_, cx| Root(Owner::new(&props(), input, cx)));
        let owner = root.read_with(cx, |r, _| r.0.clone());
        *locator.borrow_mut() = Some(owner.downgrade());
        cx.run_until_parked();
        cx.cx.activate_accessibility(cx.window_handle());
        draw(cx);
        let id = link(cx);
        ax(cx, id, accesskit::Action::Click);
        assert_eq!(cx.cx.opened_url(), None);
        mode.set(1);
        ax(cx, id, accesskit::Action::Click);
        assert_eq!(cx.cx.opened_url(), None);
        ax(cx, id, accesskit::Action::Focus);
        assert!(cx.update(|window, cx| window.focused(cx).is_some()));
        mode.set(2);
        ax(cx, id, accesskit::Action::Click);
        assert_eq!(cx.cx.opened_url(), None);
        assert!(cx.update(|window, cx| window.focused(cx).is_none()));
        // Restore the same document; only a fresh native paint admits its new lease.
        let handle = cx.window_handle();
        cx.with_paused_automatic_drawing(|cx| {
            cx.update_window(handle, |_, window, cx| {
                owner.update(cx, |o, cx| o.apply(&props(), window, cx))
            })
            .unwrap();
            mode.set(3);
            cx.simulate_accessibility_action(
                handle,
                accesskit::ActionRequest {
                    action: accesskit::Action::Click,
                    target_node: id,
                    target_tree: accesskit::TreeId::ROOT,
                    data: None,
                },
            );
            assert_eq!(cx.opened_url(), None);
        });
        draw(cx);
        for url in ["javascript:alert(1)", "file:///private/secret"] {
            let mut unsafe_props = props();
            unsafe_props.source = format!("[unsafe]({url})");
            cx.update(|window, cx| owner.update(cx, |o, cx| o.apply(&unsafe_props, window, cx)));
            cx.run_until_parked();
            draw(cx);
            let unsafe_link = link(cx);
            ax(cx, unsafe_link, accesskit::Action::Click);
            assert_eq!(cx.cx.opened_url(), None);
        }
        cx.update(|window, cx| owner.update(cx, |o, cx| o.apply(&props(), window, cx)));
        cx.run_until_parked();
        draw(cx);
        let fresh = link(cx);
        ax(cx, fresh, accesskit::Action::Click);
        assert_eq!(cx.cx.opened_url().as_deref(), Some("https://example.test"));
        assert!(!web_target("javascript:alert(1)"));
        assert!(!web_target("file:///private/secret"));
        assert!(!web_target("https://example.test\nfile:///secret"));
    }
}
