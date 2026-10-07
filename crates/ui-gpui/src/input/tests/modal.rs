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

use super::*;
use crate::gpui::prelude::FluentBuilder;
use gpui_kit::base::Root;
use gpui_kit::{
    App, FocusHandle, InputScope, Modifiers, StatefulInteractiveElement, StyleRefinement,
    VisualTestContext, accesskit, point,
};

struct Foreground {
    scope: Option<InputScope>,
    nested: Option<InputScope>,
    nested_focus: FocusHandle,
    nested_query: Entity<InputOwner>,
    nested_clicks: Rc<Cell<usize>>,
    interrupt_click: bool,
    focus: FocusHandle,
    query: Entity<InputOwner>,
    first: FocusHandle,
    second: FocusHandle,
    clicks: Rc<Cell<usize>>,
    renders: Rc<Cell<usize>>,
    paints: Rc<Cell<usize>>,
    ax_callbacks: Rc<Cell<usize>>,
}
impl Render for Foreground {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get() + 1);
        let clicks = self.clicks.clone();
        let paints = self.paints.clone();
        let interrupt = self.interrupt_click;
        let ax_callbacks = self.ax_callbacks.clone();
        let nested_clicks = self.nested_clicks.clone();
        self.scope.as_ref().unwrap().element(
            div()
                .id("foreground")
                .track_focus(&self.focus)
                .w(px(400.))
                .h(px(128.))
                .flex()
                .flex_col()
                .child(gpui_kit::canvas(
                    |_, _, _| (),
                    move |_, _, _, _| paints.set(paints.get() + 1),
                ))
                .child(
                    div()
                        .h(px(48.))
                        .w_full()
                        .flex_shrink_0()
                        .child(self.query.clone()),
                )
                .child(
                    div()
                        .id("foreground-first")
                        .role(Role::Button)
                        .aria_label("Foreground first")
                        .track_focus(&self.first)
                        .h(px(40.))
                        .w_full()
                        .on_click(move |_, window, cx| {
                            if interrupt {
                                let focus = cx.focus_handle();
                                drop(window.input_scope(&focus, cx));
                            }
                        })
                        .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
                )
                .child(
                    div()
                        .id("foreground-second")
                        .role(Role::Button)
                        .aria_label("Foreground second")
                        .track_focus(&self.second)
                        .h(px(40.))
                        .w_full()
                        .on_a11y_action(accesskit::Action::Click, move |_, window, cx| {
                            if interrupt {
                                let focus = cx.focus_handle();
                                drop(window.input_scope(&focus, cx));
                            }
                        })
                        .on_a11y_action(accesskit::Action::Click, move |_, _, _| {
                            ax_callbacks.set(ax_callbacks.get() + 1)
                        }),
                )
                .when_some(self.nested.as_ref(), |body, scope| {
                    body.child(
                        scope.element(
                            div()
                                .id("nested-foreground")
                                .role(Role::Button)
                                .aria_label("Nested foreground")
                                .track_focus(&self.nested_focus)
                                .h(px(48.))
                                .w_full()
                                .flex_shrink_0()
                                .child(self.nested_query.clone())
                                .on_click(move |_, _, _| {
                                    nested_clicks.set(nested_clicks.get() + 1)
                                }),
                        ),
                    )
                }),
        )
    }
}
struct ModalRoot {
    background: Entity<InputOwner>,
    button: FocusHandle,
    panel: Entity<Foreground>,
    open: bool,
    clicks: Rc<Cell<usize>>,
    keys: Rc<Cell<usize>>,
    pointer: Rc<Cell<usize>>,
    interrupt_key: bool,
    interrupt_action: bool,
    dispatches: Rc<Cell<usize>>,
}
impl Render for ModalRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (clicks, keys) = (self.clicks.clone(), self.keys.clone());
        let interrupt = self.interrupt_key;
        let interrupt_action = self.interrupt_action;
        let (key_dispatches, action_dispatches) =
            (self.dispatches.clone(), self.dispatches.clone());
        let content = div()
            .id("modal-test-root")
            .key_context("ModalReplay")
            .on_action(move |_: &ShortChord, window, cx| {
                action_dispatches.set(action_dispatches.get() + 1);
                if interrupt_action {
                    let focus = cx.focus_handle();
                    drop(window.input_scope(&focus, cx));
                }
            })
            .w(px(400.))
            .h(px(240.))
            .flex()
            .flex_col()
            .on_key_down(move |_, window, cx| {
                key_dispatches.set(key_dispatches.get() + 1);
                if interrupt {
                    let focus = cx.focus_handle();
                    drop(window.input_scope(&focus, cx));
                }
            })
            .on_key_down(move |_, _, _| keys.set(keys.get() + 1))
            .child(
                div()
                    .h(px(48.))
                    .w_full()
                    .flex_shrink_0()
                    .child(self.background.clone()),
            )
            .child(
                div()
                    .id("background-button")
                    .role(Role::Button)
                    .aria_label("Background")
                    .track_focus(&self.button)
                    .h(px(40.))
                    .w_full()
                    .flex_shrink_0()
                    .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
            )
            .when(self.open, |root| {
                root.child(
                    self.panel
                        .clone()
                        .cached(StyleRefinement::default().w(px(400.)).h(px(128.))),
                )
            });
        TrailingListener {
            child: content.into_any_element(),
            pointer: self.pointer.clone(),
        }
    }
}
struct Fixture {
    root: Entity<ModalRoot>,
    window: AnyWindowHandle,
    background: Entity<InputOwner>,
    panel: Entity<Foreground>,
    clicks: Rc<Cell<usize>>,
    foreground_clicks: Rc<Cell<usize>>,
    keys: Rc<Cell<usize>>,
    pointer: Rc<Cell<usize>>,
    renders: Rc<Cell<usize>>,
    paints: Rc<Cell<usize>>,
    ax_callbacks: Rc<Cell<usize>>,
    _live: LiveInput,
}
fn fixture(cx: &mut TestAppContext) -> Fixture {
    let live = LiveInput::new(
        Rc::new(RefCell::new(Some(Input(Rc::new(|_, _, _, _| {
            Ok(Default::default())
        }))))),
        crate::NativeBudget::default(),
        std::sync::Arc::new(maka_plugins::call::Resources::default()),
    );
    let (clicks, foreground_clicks, keys, renders, paints, pointer, ax_callbacks) = (
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
    );
    let (root, window, background, panel) = cx.update(|cx| {
        let mut handles = None;
        let window = cx
            .open_window(Default::default(), |window, cx| {
                gpui_kit::init(cx);
                let background = InputOwner::new(&props(), live.clone(), false, window, cx);
                let mut query_props = props();
                query_props.auto_focus = true;
                let query = InputOwner::new(&query_props, live.clone(), false, window, cx);
                let nested_query = InputOwner::new(&query_props, live.clone(), false, window, cx);
                let panel = cx.new(|cx| Foreground {
                    scope: None,
                    nested: None,
                    nested_focus: cx.focus_handle().tab_stop(true),
                    nested_query,
                    nested_clicks: Rc::new(Cell::new(0)),
                    interrupt_click: false,
                    focus: cx.focus_handle().tab_stop(false),
                    query,
                    first: cx.focus_handle().tab_stop(true).tab_index(1),
                    second: cx.focus_handle().tab_stop(true).tab_index(2),
                    clicks: foreground_clicks.clone(),
                    renders: renders.clone(),
                    paints: paints.clone(),
                    ax_callbacks: ax_callbacks.clone(),
                });
                let root = cx.new(|cx| ModalRoot {
                    background: background.clone(),
                    button: cx.focus_handle().tab_stop(true),
                    panel: panel.clone(),
                    open: false,
                    clicks: clicks.clone(),
                    keys: keys.clone(),
                    pointer: pointer.clone(),
                    interrupt_key: false,
                    interrupt_action: false,
                    dispatches: Rc::new(Cell::new(0)),
                });
                handles = Some((root.clone(), background, panel));
                cx.new(|cx| Root::new(root, window, cx))
            })
            .unwrap();
        let (root, background, panel) = handles.unwrap();
        (root, window.into(), background, panel)
    });
    draw(cx, window);
    Fixture {
        root,
        window,
        background,
        panel,
        clicks,
        foreground_clicks,
        keys,
        pointer,
        renders,
        paints,
        ax_callbacks,
        _live: live,
    }
}
fn open(cx: &mut TestAppContext, f: &Fixture) {
    cx.update_window(f.window, |_, window, cx| {
        f.panel.update(cx, |panel, cx| {
            panel.scope = Some(window.input_scope(&panel.focus, cx));
            cx.notify();
        });
        f.root.update(cx, |root, cx| {
            root.open = true;
            cx.notify();
        });
    })
    .unwrap();
}
fn close(cx: &mut TestAppContext, f: &Fixture) {
    cx.update(|cx| {
        f.panel.update(cx, |panel, cx| {
            panel.scope.take();
            cx.notify();
        });
        f.root.update(cx, |root, cx| {
            root.open = false;
            cx.notify();
        });
    });
}
fn target(cx: &mut TestAppContext, window: AnyWindowHandle, label: &str) -> accesskit::NodeId {
    cx.accessibility_update(window)
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some(label))
        .unwrap()
        .0
}
fn ax(
    cx: &mut TestAppContext,
    window: AnyWindowHandle,
    target_node: accesskit::NodeId,
    action: accesskit::Action,
) {
    cx.simulate_accessibility_action(
        window,
        accesskit::ActionRequest {
            action,
            target_node,
            target_tree: accesskit::TreeId::ROOT,
            data: None,
        },
    );
}

#[gpui::test]
fn modal_input_scope_keeps_native_tab_and_first_editor_input_inside_distinct_foreground_group(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    assert!(cx.text_input_state_changes(f.window).is_empty());
    open(cx, &f);
    draw(cx, f.window);
    cx.update_window(f.window, |_, window, cx| {
        let panel = f.panel.read(cx);
        let query = panel
            .query
            .read(cx)
            .state
            .read(cx)
            .presentation()
            .focus_handle()
            .clone();
        assert_ne!(query, panel.focus);
        assert!(query.is_focused(window));
        let background = f.root.read(cx).button.clone();
        window.focus(&background, cx);
        assert!(
            query.is_focused(window),
            "programmatic background fallback cannot steal foreground focus"
        );
    })
    .unwrap();
    assert_eq!(
        cx.text_input_state_changes(f.window),
        [gpui_kit::TextInputStateChange::FocusGained]
    );
    cx.simulate_input(f.window, "first");
    cx.update(|cx| {
        assert_eq!(
            f.panel
                .read(cx)
                .query
                .read(cx)
                .state
                .read(cx)
                .value()
                .as_ref(),
            "first"
        )
    });
    cx.simulate_keystrokes(f.window, "tab");
    cx.update_window(f.window, |_, window, cx| {
        assert!(f.panel.read(cx).first.is_focused(window))
    })
    .unwrap();
    cx.simulate_keystrokes(f.window, "tab shift-tab");
    cx.update_window(f.window, |_, window, cx| {
        assert!(f.panel.read(cx).first.is_focused(window))
    })
    .unwrap();
    cx.simulate_keystrokes(f.window, "x");
    assert_eq!(
        f.keys.get(),
        0,
        "background ancestor raw handlers are outside the original scoped path"
    );
}

#[gpui::test]
fn modal_input_scope_blocks_direct_ax_and_pointer_across_retained_paint_and_close_reopen_aba(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.activate_accessibility(f.window);
    draw(cx, f.window);
    let background = target(cx, f.window, "Background");
    cx.with_paused_automatic_drawing(|cx| {
        open(cx, &f);
        ax(cx, f.window, background, accesskit::Action::Click);
        assert_eq!(
            f.clicks.get(),
            0,
            "opening blocks the old frame before repaint"
        );
    });
    draw(cx, f.window);
    let foreground = target(cx, f.window, "Foreground first");
    ax(cx, f.window, foreground, accesskit::Action::Click);
    assert_eq!(f.foreground_clicks.get(), 1);
    for _ in 0..3 {
        draw(cx, f.window);
        ax(cx, f.window, background, accesskit::Action::Click);
        assert_eq!(f.clicks.get(), 0);
    }
    VisualTestContext::from_window(f.window, cx)
        .simulate_click(point(px(10.), px(60.)), Modifiers::default());
    assert_eq!(
        f.clicks.get(),
        0,
        "uncovered underlay pointer is refused independently of occlusion"
    );
    cx.with_paused_automatic_drawing(|cx| {
        close(cx, &f);
        open(cx, &f);
        ax(cx, f.window, foreground, accesskit::Action::Click);
        assert_eq!(
            f.foreground_clicks.get(),
            1,
            "old foreground cannot follow a new opening at the same focus id"
        );
    });
    draw(cx, f.window);
    let current = target(cx, f.window, "Foreground first");
    ax(cx, f.window, current, accesskit::Action::Click);
    assert_eq!(f.foreground_clicks.get(), 2);
    close(cx, &f);
    draw(cx, f.window);
    ax(cx, f.window, background, accesskit::Action::Click);
    assert_eq!(f.clicks.get(), 1);
}

#[gpui::test]
fn modal_input_scope_replays_the_original_cached_foreground_without_opening_underlay_input(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    open(cx, &f);
    draw(cx, f.window);
    cx.update_window(f.window, |_, window, cx| {
        let first = f.panel.read(cx).first.clone();
        window.focus(&first, cx);
    })
    .unwrap();
    draw(cx, f.window);
    let builds = f.renders.get();
    let paints = f.paints.get();
    for _ in 0..3 {
        cx.update(|cx| f.root.update(cx, |_, cx| cx.notify()));
        cx.update_window(f.window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
        cx.run_until_parked();
        cx.update_window(f.window, |_, window, cx| {
            assert!(!window.input_focus_is_admitted(&f.root.read(cx).button));
            assert!(window.input_focus_is_admitted(&f.panel.read(cx).first));
        })
        .unwrap();
    }
    assert_eq!(
        f.paints.get(),
        paints,
        "original foreground paint records are replayed without painting its child"
    );
    assert_eq!(
        f.renders.get(),
        builds,
        "original opaque foreground remains retained"
    );
}

#[gpui::test]
fn revoked_held_foreground_cannot_regain_programmatic_focus(cx: &mut TestAppContext) {
    let f = fixture(cx);
    open(cx, &f);
    draw(cx, f.window);
    cx.update(|cx| f.panel.read(cx).scope.as_ref().unwrap().revoke());
    // Successfully paint the still-held foreground; its original weak opening
    // stays dead even though no live global exclusivity is published.
    draw(cx, f.window);
    cx.update_window(f.window, |_, window, cx| {
        let background = f.root.read(cx).button.clone();
        let dead = f.panel.read(cx).first.clone();
        window.focus(&background, cx);
        assert!(background.is_focused(window));
        window.focus(&dead, cx);
        assert!(background.is_focused(window));
        assert!(!window.input_focus_is_admitted(&dead));
    })
    .unwrap();
}

// This original parent registers its listener after painting its child. It
// must not inherit the child's final foreground dispatch node.
struct TrailingListener {
    child: gpui_kit::AnyElement,
    pointer: Rc<Cell<usize>>,
}
impl IntoElement for TrailingListener {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl gpui_kit::Element for TrailingListener {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<gpui_kit::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&gpui_kit::GlobalElementId>,
        _: Option<&gpui_kit::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui_kit::LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&gpui_kit::GlobalElementId>,
        _: Option<&gpui_kit::InspectorElementId>,
        _: gpui_kit::Bounds<gpui_kit::Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&gpui_kit::GlobalElementId>,
        _: Option<&gpui_kit::InspectorElementId>,
        _: gpui_kit::Bounds<gpui_kit::Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
        let pointer = self.pointer.clone();
        window.on_mouse_event(move |_: &gpui_kit::MouseDownEvent, phase, _, _| {
            if phase.bubble() {
                pointer.set(pointer.get() + 1);
            }
        });
    }
}

#[gpui::test]
fn original_parent_listener_registered_after_child_paint_stays_outside_foreground(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    open(cx, &f);
    draw(cx, f.window);
    VisualTestContext::from_window(f.window, cx)
        .simulate_click(point(px(10.), px(150.)), Modifiers::default());
    assert_eq!(
        f.foreground_clicks.get(),
        1,
        "actual foreground pointer still arrives"
    );
    assert_eq!(
        f.pointer.get(),
        0,
        "later parent registration cannot borrow child's foreground provenance"
    );
    close(cx, &f);
    draw(cx, f.window);
    VisualTestContext::from_window(f.window, cx)
        .simulate_click(point(px(10.), px(60.)), Modifiers::default());
    assert_eq!(f.pointer.get(), 1);
}

#[gpui::test]
fn reentrant_scope_change_stops_the_original_raw_key_dispatch(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.root.update(cx, |root, cx| {
            root.interrupt_key = true;
            cx.notify();
        });
        let background = f.root.read(cx).button.clone();
        window.focus(&background, cx);
    })
    .unwrap();
    draw(cx, f.window);
    cx.simulate_keystrokes(f.window, "x");
    assert_eq!(
        f.keys.get(),
        0,
        "later listener cannot receive an event retired by a modal opening"
    );
}

#[gpui::test]
fn modal_opening_refuses_original_background_platform_slot_before_paint_without_erasing_editor(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        let focus = f
            .background
            .read(cx)
            .state
            .read(cx)
            .presentation()
            .focus_handle()
            .clone();
        window.focus(&focus, cx);
    })
    .unwrap();
    draw(cx, f.window);
    let before = cx.with_paused_automatic_drawing(|cx| {
        cx.with_platform_input_handler(f.window, |handler| {
            handler.replace_text_in_range(None, "draft");
            handler.set_selected_text_range(1..3);
            handler.replace_and_mark_text_in_range(Some(1..3), "中", Some(1..1));
        })
        .unwrap();
        cx.with_platform_input_handler(f.window, |handler| {
            (
                handler.selected_text_range(true).unwrap(),
                handler.marked_text_range().unwrap(),
                handler.text_for_range(0..4, &mut None).unwrap(),
            )
        })
        .unwrap()
    });
    cx.with_paused_automatic_drawing(|cx| {
        open(cx, &f);
        cx.with_platform_input_handler(f.window, |handler| {
            assert!(handler.selected_text_range(true).is_none());
            assert_eq!(handler.marked_text_range(), None);
            assert_eq!(handler.text_for_range(0..4, &mut None), None);
            handler.replace_text_in_range(None, "bad");
            handler.replace_and_mark_text_in_range(None, "bad", None);
            handler.set_selected_text_range(0..0);
            handler.unmark_text();
            handler.paste(gpui_kit::ClipboardItem::new_string("bad".into()));
        })
        .unwrap();
        cx.update_window(f.window, |_, window, cx| {
            let state = f.background.read(cx).state.clone();
            state.update(cx, |state, cx| {
                let selection = state.selected_text_range(true, window, cx).unwrap();
                assert_eq!(selection.range, before.0.range);
                assert_eq!(selection.reversed, before.0.reversed);
                assert_eq!(state.marked_text_range(window, cx), Some(before.1.clone()));
                assert_eq!(state.value().as_ref(), before.2);
            });
        })
        .unwrap();
    });
    draw(cx, f.window);
    cx.with_platform_input_handler(f.window, |handler| {
        handler.replace_text_in_range(None, "query")
    })
    .unwrap();
    cx.update(|cx| {
        assert_eq!(
            f.panel
                .read(cx)
                .query
                .read(cx)
                .state
                .read(cx)
                .value()
                .as_ref(),
            "query"
        )
    });
}

gpui_kit::actions!(modal_test, [ScopeAction, ShortChord, LongChord]);
#[gpui::test]
fn reentrant_scope_revocation_preserves_original_global_action_listeners(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let opening = cx
        .update_window(f.window, |_, window, cx| {
            let focus = cx.focus_handle();
            window.input_scope(&focus, cx)
        })
        .unwrap();
    // No foreground declares this opening: the next successful original frame
    // restores normal dispatch while its revocable owner remains alive.
    draw(cx, f.window);
    let owner = Rc::new(Cell::new(Some(opening)));
    let revoker_calls = Rc::new(Cell::new(0));
    let later_calls = Rc::new(Cell::new(0));
    let (revoker, later) = (revoker_calls.clone(), later_calls.clone());
    cx.update(|cx| {
        cx.on_action(move |_: &ScopeAction, cx| {
            later.set(later.get() + 1);
            cx.propagate();
        });
        cx.on_action(move |_: &ScopeAction, cx| {
            revoker.set(revoker.get() + 1);
            drop(owner.take());
            cx.propagate();
        });
    });
    cx.dispatch_action(f.window, ScopeAction);
    assert_eq!(revoker_calls.get(), 1);
    assert_eq!(later_calls.get(), 0);
    cx.dispatch_action(f.window, ScopeAction);
    assert_eq!(
        revoker_calls.get(),
        2,
        "interruption cannot discard the removed native listener vector"
    );
    assert_eq!(later_calls.get(), 1);
}

#[gpui::test]
fn modal_click_revocation_stops_later_native_click_listeners(cx: &mut TestAppContext) {
    for kind in 0..3 {
        let f = fixture(cx);
        cx.activate_accessibility(f.window);
        open(cx, &f);
        cx.update(|cx| {
            f.panel.update(cx, |panel, cx| {
                panel.interrupt_click = true;
                cx.notify();
            })
        });
        draw(cx, f.window);
        if kind != 0 {
            let label = if kind == 1 {
                "Foreground first"
            } else {
                "Foreground second"
            };
            let first = target(cx, f.window, label);
            ax(cx, f.window, first, accesskit::Action::Click);
        } else {
            VisualTestContext::from_window(f.window, cx)
                .simulate_click(point(px(10.), px(150.)), Modifiers::default());
        }
        assert_eq!(
            f.foreground_clicks.get(),
            0,
            "later native click listener cannot receive the retired event (kind={kind})"
        );
        assert_eq!(
            f.ax_callbacks.get(),
            0,
            "same target AX listeners also stop after opening changes"
        );
        cx.update(|cx| {
            f.panel.update(cx, |panel, cx| {
                panel.interrupt_click = false;
                cx.notify();
            })
        });
        draw(cx, f.window);
        let first = target(cx, f.window, "Foreground first");
        ax(cx, f.window, first, accesskit::Action::Click);
        assert_eq!(
            f.foreground_clicks.get(),
            1,
            "original listeners survive revocation and fresh paint"
        );
        let second = target(cx, f.window, "Foreground second");
        ax(cx, f.window, second, accesskit::Action::Click);
        assert_eq!(f.ax_callbacks.get(), 1);
    }
}

#[gpui::test]
fn reentrant_scope_change_skips_later_keystroke_subscribers_without_removing_them(
    cx: &mut TestAppContext,
) {
    for intercept in [false, true] {
        let f = fixture(cx);
        let first_calls = Rc::new(Cell::new(0));
        let later_calls = Rc::new(Cell::new(0));
        let (first, later) = (first_calls.clone(), later_calls.clone());
        let _subscriptions = cx.update(|cx| {
            let first = move |_: &gpui_kit::KeystrokeEvent, window: &mut Window, cx: &mut App| {
                first.set(first.get() + 1);
                if first.get() == 1 {
                    let focus = cx.focus_handle();
                    drop(window.input_scope(&focus, cx));
                }
            };
            let later = move |_: &gpui_kit::KeystrokeEvent, _: &mut Window, _: &mut App| {
                later.set(later.get() + 1)
            };
            if intercept {
                [
                    cx.intercept_keystrokes(first),
                    cx.intercept_keystrokes(later),
                ]
            } else {
                [cx.observe_keystrokes(first), cx.observe_keystrokes(later)]
            }
        });
        cx.simulate_keystrokes(f.window, "x");
        assert_eq!(first_calls.get(), 1);
        assert_eq!(
            later_calls.get(),
            0,
            "old key does not reach a later subscriber (intercept={intercept})"
        );
        draw(cx, f.window);
        cx.simulate_keystrokes(f.window, "y");
        assert_eq!(first_calls.get(), 2);
        assert_eq!(
            later_calls.get(),
            1,
            "skipped subscriber remains for a new input"
        );
    }
}

#[gpui::test]
fn modal_interruption_stops_original_chord_replay_batch_and_its_observers(cx: &mut TestAppContext) {
    for action in [false, true] {
        let f = fixture(cx);
        let observations = Rc::new(Cell::new(0));
        let observed = observations.clone();
        let _observer = cx.update(|cx| {
            cx.clear_key_bindings();
            if action {
                cx.bind_keys([gpui_kit::KeyBinding::new(
                    "x",
                    ShortChord,
                    Some("ModalReplay"),
                )]);
            }
            cx.bind_keys([gpui_kit::KeyBinding::new(
                "x y q",
                LongChord,
                Some("ModalReplay"),
            )]);
            cx.observe_keystrokes(move |_, _, _| observed.set(observed.get() + 1))
        });
        cx.update_window(f.window, |_, window, cx| {
            f.root.update(cx, |root, cx| {
                root.interrupt_key = !action;
                root.interrupt_action = action;
                cx.notify();
            });
            let focus = f.root.read(cx).button.clone();
            window.focus(&focus, cx);
        })
        .unwrap();
        draw(cx, f.window);
        cx.simulate_keystrokes(f.window, "x y");
        cx.update_window(f.window, |_, window, cx| {
            assert!(
                window.has_pending_keystrokes(),
                "action={action}; contexts={:?}; dispatches={}; observations={}",
                window.context_stack(),
                f.root.read(cx).dispatches.get(),
                observations.get()
            )
        })
        .unwrap();
        assert_eq!(observations.get(), 0);
        cx.simulate_keystrokes(f.window, "z");
        let dispatches = cx.update(|cx| f.root.read(cx).dispatches.clone());
        assert_eq!(
            dispatches.get(),
            1,
            "one original replay retires the batch (action={action})"
        );
        assert_eq!(
            observations.get(),
            0,
            "retired chord cannot be observed as a new opening's input"
        );
        cx.update(|cx| {
            f.root.update(cx, |root, cx| {
                root.interrupt_key = false;
                root.interrupt_action = false;
                cx.notify();
            })
        });
        draw(cx, f.window);
        cx.simulate_keystrokes(f.window, "u");
        assert_eq!(dispatches.get(), 2);
        assert_eq!(observations.get(), 1);
    }
}

#[gpui::test]
fn modal_preserves_original_editor_selection_without_resuming_its_old_pointer_drag(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        let focus = f
            .background
            .read(cx)
            .state
            .read(cx)
            .presentation()
            .focus_handle()
            .clone();
        window.focus(&focus, cx);
    })
    .unwrap();
    draw(cx, f.window);
    cx.simulate_input(f.window, "Original draft 中英文");
    draw(cx, f.window);
    let mut pointer = VisualTestContext::from_window(f.window, cx);
    pointer.simulate_mouse_down(
        point(px(12.), px(20.)),
        gpui_kit::MouseButton::Left,
        Modifiers::default(),
    );
    pointer.simulate_mouse_move(
        point(px(55.), px(20.)),
        Some(gpui_kit::MouseButton::Left),
        Modifiers::default(),
    );
    let range = cx.update(|cx| f.background.read(cx).state.read(cx).selected_range());
    assert!(
        !range.is_empty(),
        "actual native pointer selects the original editor"
    );
    open(cx, &f);
    draw(cx, f.window);
    close(cx, &f);
    cx.update_window(f.window, |_, window, cx| {
        let focus = f
            .background
            .read(cx)
            .state
            .read(cx)
            .presentation()
            .focus_handle()
            .clone();
        window.focus(&focus, cx);
    })
    .unwrap();
    draw(cx, f.window);
    pointer.simulate_mouse_move(
        point(px(160.), px(20.)),
        Some(gpui_kit::MouseButton::Left),
        Modifiers::default(),
    );
    pointer.simulate_mouse_up(
        point(px(160.), px(20.)),
        gpui_kit::MouseButton::Left,
        Modifiers::default(),
    );
    cx.update(|cx| {
        let state = f.background.read(cx).state.read(cx);
        assert_eq!(state.selected_range(), range);
        assert_eq!(state.value().as_ref(), "Original draft 中英文");
    });
    pointer.simulate_mouse_down(
        point(px(12.), px(20.)),
        gpui_kit::MouseButton::Left,
        Modifiers::default(),
    );
    pointer.simulate_mouse_move(
        point(px(130.), px(20.)),
        Some(gpui_kit::MouseButton::Left),
        Modifiers::default(),
    );
    pointer.simulate_mouse_up(
        point(px(130.), px(20.)),
        gpui_kit::MouseButton::Left,
        Modifiers::default(),
    );
    cx.update(|cx| assert_ne!(f.background.read(cx).state.read(cx).selected_range(), range));
}

#[gpui::test]
fn closing_nested_scope_restores_outer_admission_before_paint_without_reviving_old_opening(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.activate_accessibility(f.window);
    open(cx, &f);
    draw(cx, f.window);
    let background = target(cx, f.window, "Background");
    let outer = target(cx, f.window, "Foreground first");
    let outer_group = cx
        .update_window(f.window, |_, window, _| window.active_input_group())
        .unwrap()
        .unwrap();
    let global = Rc::new(Cell::new(0));
    cx.update(|cx| {
        let global = global.clone();
        cx.on_action(move |_: &ScopeAction, _| global.set(global.get() + 1));
    });
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(f.window, |_, window, cx| {
            f.panel.update(cx, |panel, cx| {
                panel.nested = Some(window.input_scope(&panel.nested_focus, cx));
                cx.notify();
            });
        })
        .unwrap();
        cx.with_platform_input_handler(f.window, |handler| {
            handler.replace_text_in_range(None, "blocked");
        })
        .unwrap();
        ax(cx, f.window, outer, accesskit::Action::Click);
        assert_eq!(f.foreground_clicks.get(), 0);
        cx.update(|cx| f.panel.read(cx).nested.as_ref().unwrap().revoke());
        cx.with_platform_input_handler(f.window, |handler| {
            handler.replace_text_in_range(None, "outer");
        })
        .unwrap();
        ax(cx, f.window, background, accesskit::Action::Click);
        ax(cx, f.window, outer, accesskit::Action::Click);
        assert_eq!(f.clicks.get(), 0, "outer remains exclusive without a draw");
        assert_eq!(f.foreground_clicks.get(), 1);
        cx.dispatch_action(f.window, ScopeAction);
        assert_eq!(
            global.get(),
            0,
            "global business actions remain outside outer"
        );
        cx.update(|cx| {
            assert_eq!(
                f.panel
                    .read(cx)
                    .query
                    .read(cx)
                    .state
                    .read(cx)
                    .value()
                    .as_ref(),
                "outer"
            );
        });
        cx.update_window(f.window, |_, window, cx| {
            f.panel.update(cx, |panel, cx| {
                panel.nested = Some(window.input_scope(&panel.nested_focus, cx));
                panel.nested_focus.focus(window, cx);
                cx.notify();
            });
        })
        .unwrap();
    });
    draw(cx, f.window);
    let nested = target(cx, f.window, "Nested foreground");
    ax(cx, f.window, nested, accesskit::Action::Click);
    cx.update(|cx| assert_eq!(f.panel.read(cx).nested_clicks.get(), 1));
    cx.with_paused_automatic_drawing(|cx| {
        cx.update(|cx| f.panel.read(cx).nested.as_ref().unwrap().revoke());
        cx.update_window(f.window, |_, window, cx| {
            let panel = f.panel.read(cx);
            assert_eq!(window.active_input_group(), Some(outer_group));
            assert!(window.input_focus_is_admitted(&panel.first));
            assert!(!window.input_focus_is_admitted(&panel.nested_focus));
            assert!(!window.input_focus_is_admitted(&f.root.read(cx).button));
            panel.first.clone().focus(window, cx);
        })
        .unwrap();
        ax(cx, f.window, background, accesskit::Action::Click);
        ax(cx, f.window, nested, accesskit::Action::Click);
        ax(cx, f.window, outer, accesskit::Action::Click);
        cx.simulate_keystrokes(f.window, "x");
        assert_eq!(f.clicks.get(), 0);
        assert_eq!(f.foreground_clicks.get(), 2);
        assert_eq!(f.keys.get(), 0, "raw keys cannot cross the outer barrier");
    });
    // The held dead child is still painted. Native declarations may observe it,
    // but cannot make it the foreground or make its old listeners live again.
    draw(cx, f.window);
    ax(cx, f.window, nested, accesskit::Action::Click);
    cx.update(|cx| assert_eq!(f.panel.read(cx).nested_clicks.get(), 1));
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(f.window, |_, window, cx| {
            f.panel.update(cx, |panel, cx| {
                panel.nested = Some(window.input_scope(&panel.nested_focus, cx));
                cx.notify();
            });
        })
        .unwrap();
        ax(cx, f.window, nested, accesskit::Action::Click);
        ax(cx, f.window, outer, accesskit::Action::Click);
        cx.update(|cx| assert_eq!(f.panel.read(cx).nested_clicks.get(), 1));
        assert_eq!(
            f.foreground_clicks.get(),
            2,
            "fresh inner opening is exclusive before paint"
        );
    });
    draw(cx, f.window);
    let fresh = target(cx, f.window, "Nested foreground");
    ax(cx, f.window, fresh, accesskit::Action::Click);
    cx.update(|cx| assert_eq!(f.panel.read(cx).nested_clicks.get(), 2));
    cx.with_paused_automatic_drawing(|cx| {
        cx.update(|cx| f.panel.read(cx).nested.as_ref().unwrap().revoke());
        cx.update(|cx| f.panel.read(cx).scope.as_ref().unwrap().revoke());
        ax(cx, f.window, background, accesskit::Action::Click);
        ax(cx, f.window, outer, accesskit::Action::Click);
        ax(cx, f.window, fresh, accesskit::Action::Click);
        assert_eq!(
            f.clicks.get(),
            1,
            "closing all scopes restores the original background"
        );
        assert_eq!(f.foreground_clicks.get(), 2);
        cx.update(|cx| assert_eq!(f.panel.read(cx).nested_clicks.get(), 2));
    });
}

#[gpui::test]
fn closing_inner_before_either_scope_paints_keeps_original_background_refused(
    cx: &mut TestAppContext,
) {
    for drop_inner in [false, true] {
        let f = fixture(cx);
        cx.activate_accessibility(f.window);
        cx.update_window(f.window, |_, window, cx| {
            let focus = f
                .background
                .read(cx)
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .clone();
            focus.focus(window, cx);
        })
        .unwrap();
        draw(cx, f.window);
        let background = target(cx, f.window, "Background");
        let global = Rc::new(Cell::new(0));
        cx.update(|cx| {
            let global = global.clone();
            cx.on_action(move |_: &ScopeAction, _| global.set(global.get() + 1));
        });
        cx.with_paused_automatic_drawing(|cx| {
            open(cx, &f);
            cx.update_window(f.window, |_, window, cx| {
                f.panel.update(cx, |panel, cx| {
                    panel.nested = Some(window.input_scope(&panel.nested_focus, cx));
                    if drop_inner {
                        panel.nested.take();
                    } else {
                        panel.nested.as_ref().unwrap().revoke();
                    }
                    cx.notify();
                });
            })
            .unwrap();
            ax(cx, f.window, background, accesskit::Action::Click);
            cx.dispatch_action(f.window, ScopeAction);
            cx.with_platform_input_handler(f.window, |handler| {
                handler.replace_text_in_range(None, "forbidden");
                assert!(handler.selected_text_range(true).is_none());
            })
            .unwrap();
            cx.update_window(f.window, |_, window, cx| {
                let button = f.root.read(cx).button.clone();
                assert!(!window.input_focus_is_admitted(&button));
                button.focus(window, cx);
                assert!(!button.is_focused(window));
            })
            .unwrap();
            assert_eq!(f.clicks.get(), 0);
            assert_eq!(global.get(), 0);
            cx.update(|cx| assert_eq!(f.background.read(cx).state.read(cx).value().as_ref(), ""));
        });
        draw(cx, f.window);
        cx.update_window(f.window, |_, window, cx| {
            let query = f
                .panel
                .read(cx)
                .query
                .read(cx)
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .clone();
            assert!(
                query.is_focused(window),
                "first actual outer paint offers its original editor"
            );
        })
        .unwrap();
        cx.simulate_input(f.window, "fresh outer");
        ax(cx, f.window, background, accesskit::Action::Click);
        assert_eq!(f.clicks.get(), 0);
        cx.update(|cx| {
            assert_eq!(
                f.panel
                    .read(cx)
                    .query
                    .read(cx)
                    .state
                    .read(cx)
                    .value()
                    .as_ref(),
                "fresh outer"
            )
        });
        close(cx, &f);
        draw(cx, f.window);
    }
}

#[gpui::test]
fn nested_first_paint_offers_autofocus_only_to_current_foreground_editor(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.activate_accessibility(f.window);
    let background = target(cx, f.window, "Background");
    cx.with_paused_automatic_drawing(|cx| {
        open(cx, &f);
        cx.update_window(f.window, |_, window, cx| {
            f.panel.update(cx, |panel, cx| {
                panel.nested = Some(window.input_scope(&panel.nested_focus, cx));
                cx.notify();
            });
        })
        .unwrap();
    });
    draw(cx, f.window);
    cx.update_window(f.window, |_, window, cx| {
        let panel = f.panel.read(cx);
        let inner = panel
            .nested_query
            .read(cx)
            .state
            .read(cx)
            .presentation()
            .focus_handle()
            .clone();
        let outer = panel
            .query
            .read(cx)
            .state
            .read(cx)
            .presentation()
            .focus_handle()
            .clone();
        assert_ne!(inner, panel.nested_focus);
        assert!(
            inner.is_focused(window),
            "outer offer must not consume the current inner opening's first offer"
        );
        assert!(!outer.is_focused(window));
        assert!(window.input_focus_is_admitted(&inner));
        assert!(!window.input_focus_is_admitted(&outer));
    })
    .unwrap();
    assert_eq!(
        cx.text_input_state_changes(f.window),
        [gpui_kit::TextInputStateChange::FocusGained]
    );
    cx.with_platform_input_handler(f.window, |handler| {
        handler.replace_text_in_range(None, "first inner")
    })
    .unwrap();
    let outer = target(cx, f.window, "Foreground first");
    ax(cx, f.window, background, accesskit::Action::Click);
    ax(cx, f.window, outer, accesskit::Action::Click);
    assert_eq!(f.clicks.get(), 0);
    assert_eq!(f.foreground_clicks.get(), 0);
    cx.update(|cx| {
        let panel = f.panel.read(cx);
        assert_eq!(
            panel.nested_query.read(cx).state.read(cx).value().as_ref(),
            "first inner"
        );
        assert_eq!(panel.query.read(cx).state.read(cx).value().as_ref(), "");
    });
    cx.update(|cx| {
        f.panel.update(cx, |panel, cx| {
            panel.nested.take();
            cx.notify();
        })
    });
    draw(cx, f.window);
    cx.update_window(f.window, |_, window, cx| {
        let outer = f
            .panel
            .read(cx)
            .query
            .read(cx)
            .state
            .read(cx)
            .presentation()
            .focus_handle()
            .clone();
        assert!(
            outer.is_focused(window),
            "the previously blocked original outer offer remains pending"
        );
    })
    .unwrap();
    cx.with_platform_input_handler(f.window, |handler| {
        handler.replace_text_in_range(None, "first outer")
    })
    .unwrap();
    cx.update(|cx| {
        let panel = f.panel.read(cx);
        assert_eq!(
            panel.query.read(cx).state.read(cx).value().as_ref(),
            "first outer"
        );
        assert_eq!(
            panel.nested_query.read(cx).state.read(cx).value().as_ref(),
            "first inner"
        );
    });
}
