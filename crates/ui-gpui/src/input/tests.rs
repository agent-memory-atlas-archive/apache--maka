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

use super::InputOwner;
use crate::{Input, LiveInput};
use gpui_kit::{
    AnyWindowHandle, AppContext, ClipboardItem, Context, Entity, EntityInputHandler,
    InteractiveElement, IntoElement, ParentElement, Render, Role, Styled, TestAppContext, Window,
    div, px,
};
use maka_ui::{editor::*, input::InputProps};
use serde_json::{Value, json};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
struct Harness {
    owner: Entity<InputOwner>,
    bubble: Rc<Cell<usize>>,
}
impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let count = self.bubble.clone();
        div()
            .w(px(400.))
            .h(px(48.))
            .on_key_down(move |_, _, _| count.set(count.get() + 1))
            .child(self.owner.clone())
    }
}
fn props() -> InputProps {
    serde_json::from_value(json!({"appearance":{"text":{"font":{"family":".SystemUIFont","fallbacks":[],"size":14,"lineHeight":1.5,"weight":400},"foreground":0xffffffffu32},"mutedForeground":0xaaaaaaffu32,"background":0x101010ffu32,"selection":0x343434ffu32,"caret":0xffffffffu32,"border":0x333333ffu32,"focusedBorder":0x777777ffu32,"borderWidth":1,"radius":6,"padding":{"top":6,"right":8,"bottom":6,"left":8}},"accessibilityLabel":"Connection secret","readOnly":false,"disabled":false,"tabIndex":0,"placeholder":"API key"})).unwrap()
}
struct MountedInput {
    owner: Entity<InputOwner>,
    window: AnyWindowHandle,
    live: LiveInput,
    events: Rc<RefCell<Vec<(String, Value)>>>,
    bubble: Rc<Cell<usize>>,
}
fn harness(cx: &mut TestAppContext, secret: bool) -> MountedInput {
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    let live = LiveInput::new(
        Rc::new(RefCell::new(Some(Input(Rc::new(
            move |key, value, _, _| {
                seen.borrow_mut().push((key.to_owned(), value));
                Ok(Default::default())
            },
        ))))),
        crate::NativeBudget::default(),
        std::sync::Arc::new(maka_plugins::call::Resources::default()),
    );
    let bound = live.clone();
    let bubble = Rc::new(Cell::new(0));
    let parent = bubble.clone();
    let (harness, _) = cx.add_window_view(move |window, cx| {
        gpui_kit::init(cx);
        Harness {
            owner: InputOwner::new(&props(), bound, secret, window, cx),
            bubble: parent,
        }
    });
    let owner = cx.update(|cx| harness.read(cx).owner.clone());
    let window = cx.windows()[0];
    draw(cx, window);
    MountedInput {
        owner,
        window,
        live,
        events,
        bubble,
    }
}
fn draw(cx: &mut TestAppContext, window: AnyWindowHandle) {
    cx.update_window(window, |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
}
#[gpui::test]
fn secret_native_ingress_masks_ax_platform_and_clipboard_but_preserves_original_bytes(
    cx: &mut TestAppContext,
) {
    let MountedInput {
        owner,
        window,
        live: _live,
        events,
        bubble,
    } = harness(cx, true);
    let initial = events
        .borrow()
        .iter()
        .find(|(k, _)| k == "observe")
        .unwrap()
        .1
        .clone();
    assert_eq!(initial["snapshot"]["text"]["bytes"], 0);
    cx.activate_accessibility(window);
    cx.update_window(window, |_, window, cx| {
        owner
            .read(cx)
            .state
            .clone()
            .update(cx, |state, cx| state.focus(window, cx))
    })
    .unwrap();
    draw(cx, window);
    let secret = "secret-密钥🙂e\u{301}";
    cx.simulate_input(window, secret);
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        owner.update(cx, |owner, cx| {
            assert_eq!(owner.state.read(cx).value().as_ref(), secret);
            let mut adjusted = None;
            let masked = owner
                .text_for_range(0..secret.encode_utf16().count(), &mut adjusted, window, cx)
                .unwrap();
            assert_eq!(masked, "•".repeat(secret.encode_utf16().count()));
            assert_eq!(adjusted, Some(0..secret.encode_utf16().count()));
            let reference = owner.observation_snapshot().text;
            assert_eq!(
                owner
                    .read_text(&reference, 0..secret.len(), window, cx)
                    .unwrap(),
                secret
            );
        })
    })
    .unwrap();
    let update = cx.accessibility_update(window).unwrap();
    let node = update
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::PasswordInput)
        .unwrap();
    assert_eq!(node.1.value(), None);
    assert!(
        update
            .nodes
            .iter()
            .all(|(_, node)| node.value() != Some(secret))
    );
    assert!(
        events
            .borrow()
            .iter()
            .filter(|(key, _)| key == "observe")
            .all(|(_, value)| !value.to_string().contains(secret))
    );
    for raw in ["bad\nkey", "bad\rkey", "bad\tkey"] {
        cx.update_window(window, |_, window, cx| {
            owner.update(cx, |owner, cx| {
                owner.replace_text_in_range(None, raw, window, cx)
            })
        })
        .unwrap();
        cx.update_window(window, |_, window, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string(raw.into()));
            let state = owner.read(cx).state.clone();
            let focus = state.read(cx).presentation().focus_handle().clone();
            focus.dispatch_action(&gpui_kit::base::input::Paste, window, cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update(|cx| assert_eq!(owner.read(cx).state.read(cx).value().as_ref(), secret));
    }
    cx.update_window(window, |_, window, cx| {
        let state = owner.read(cx).state.clone();
        cx.write_to_clipboard(ClipboardItem::new_string("clipboard sentinel".into()));
        state.update(cx, |state, cx| {
            state.set_selected_range(0..secret.len(), cx)
        });
        let focus = state.read(cx).presentation().focus_handle().clone();
        focus.dispatch_action(&gpui_kit::base::input::Copy, window, cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("clipboard sentinel")
        );
    })
    .unwrap();
    draw(cx, window);
    let parent_before = bubble.get();
    cx.simulate_keystrokes(window, "enter");
    assert_eq!(bubble.get(), parent_before);
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|(key, _)| key == "submit")
            .count(),
        1
    );
}
#[gpui::test]
fn initial_prefill_receipts_invalid_reset_and_retirement_keep_original_owner(
    cx: &mut TestAppContext,
) {
    let MountedInput {
        owner,
        window,
        live,
        events,
        ..
    } = harness(cx, false);
    let initial = events
        .borrow()
        .iter()
        .find(|(key, _)| key == "observe")
        .unwrap()
        .1
        .clone();
    let id = initial["snapshot"]["text"]["owner"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut configured = props();
    configured.command = Some(EditorCommand {
        owner: id.clone(),
        sequence: 1,
        based_on: 0,
        edit: EditorEdit::Replace {
            text: "prefill".into(),
        },
    });
    cx.update_window(window, |_, window, cx| {
        owner.update(cx, |owner, cx| owner.apply(&configured, window, cx))
    })
    .unwrap();
    draw(cx, window);
    cx.update(|cx| {
        assert_eq!(owner.read(cx).state.read(cx).value().as_ref(), "prefill");
        assert!(owner.read(cx).state.read(cx).can_undo());
    });
    let revision = cx.update(|cx| owner.read(cx).revision);
    configured.command = Some(EditorCommand {
        owner: id.clone(),
        sequence: 2,
        based_on: revision,
        edit: EditorEdit::Reset {
            text: "invalid\nkey".into(),
        },
    });
    cx.update_window(window, |_, window, cx| {
        owner.update(cx, |owner, cx| owner.apply(&configured, window, cx))
    })
    .unwrap();
    draw(cx, window);
    cx.update(|cx| {
        assert_eq!(owner.read(cx).state.read(cx).value().as_ref(), "prefill");
        assert!(owner.read(cx).state.read(cx).can_undo());
        assert_eq!(
            owner.read(cx).receipt.as_ref().unwrap().outcome,
            EditorOutcome::Conflict {
                reason: EditorConflict::Representation
            }
        );
    });
    configured.command = Some(EditorCommand {
        owner: id,
        sequence: 3,
        based_on: revision,
        edit: EditorEdit::Reset {
            text: String::new(),
        },
    });
    cx.update_window(window, |_, window, cx| {
        owner.update(cx, |owner, cx| owner.apply(&configured, window, cx))
    })
    .unwrap();
    draw(cx, window);
    cx.update(|cx| {
        assert!(owner.read(cx).state.read(cx).value().is_empty());
        assert!(!owner.read(cx).state.read(cx).can_undo());
    });
    cx.update_window(window, |_, window, cx| {
        owner
            .read(cx)
            .state
            .clone()
            .update(cx, |state, cx| state.focus(window, cx))
    })
    .unwrap();
    draw(cx, window);
    live.0.borrow_mut().take();
    let before = events.borrow().len();
    cx.simulate_input(window, "retired");
    cx.simulate_keystrokes(window, "enter");
    draw(cx, window);
    cx.update(|cx| assert!(owner.read(cx).state.read(cx).value().is_empty()));
    assert_eq!(events.borrow().len(), before);
}

#[gpui::test]
fn retained_secret_presentation_preserves_custody_and_settles_hidden_reset(
    cx: &mut TestAppContext,
) {
    use crate::Widget;
    let MountedInput {
        owner,
        window,
        live,
        events,
        ..
    } = harness(cx, true);
    cx.activate_accessibility(window);
    cx.update_window(window, |_, window, cx| {
        owner
            .read(cx)
            .state
            .clone()
            .update(cx, |state, cx| state.focus(window, cx));
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_input(window, "Retained密钥🙂");
    draw(cx, window);
    let original = cx.update(|cx| owner.read(cx).observation_snapshot());
    let original_entity = owner.entity_id();
    let mut recipe = super::InputRecipe {
        props: props(),
        secret: true,
    };
    let mut resource: Box<dyn std::any::Any> = Box::new(owner.clone());
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(window, |_, window, cx| {
            recipe.presentation(resource.as_mut(), false, window, cx);
            assert!(owner.read(cx).state.read(cx).presentation().is_disabled());
            assert!(
                !owner
                    .read(cx)
                    .state
                    .read(cx)
                    .presentation()
                    .focus_handle()
                    .is_focused(window)
            );
            assert!(live.0.borrow().is_some());
            owner.update(cx, |owner, cx| {
                owner.replace_text_in_range(
                    Some(0.."Retained密钥🙂".encode_utf16().count()),
                    "Stale platform input",
                    window,
                    cx,
                );
                owner.replace_and_mark_text_in_range(
                    None,
                    "Stale marked input",
                    Some(0..1),
                    window,
                    cx,
                );
            });
        })
        .unwrap();
        cx.simulate_input(window, "Hidden ingress");
        cx.update(|cx| {
            assert_eq!(
                owner.read(cx).state.read(cx).value().as_ref(),
                "Retained密钥🙂"
            );
            assert!(
                !owner.read(cx).state.read(cx).can_undo(),
                "hidden editing actions are unavailable"
            );
        });
    });
    draw(cx, window);
    assert!(
        !cx.accessibility_update(window)
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == Role::PasswordInput)
    );
    recipe.props.command = Some(EditorCommand {
        owner: original.text.owner.clone(),
        sequence: 1,
        based_on: original.text.revision,
        edit: EditorEdit::Reset {
            text: String::new(),
        },
    });
    cx.update_window(window, |_, window, cx| {
        recipe.presentation(resource.as_mut(), true, window, cx);
        assert!(
            owner.read(cx).state.read(cx).can_undo(),
            "show restores the retained history"
        );
        recipe.presentation(resource.as_mut(), false, window, cx);
        recipe.commit(resource.as_mut(), window, cx);
        recipe.publish_pending(resource.as_mut(), window, cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert!(
        events
            .borrow()
            .iter()
            .any(|(key, value)| key == "observe" && value["receipt"]["sequence"] == 1)
    );
    cx.update(|cx| {
        assert!(owner.read(cx).state.read(cx).value().is_empty());
        assert!(!owner.read(cx).state.read(cx).can_undo());
    });
    cx.update_window(window, |_, window, cx| {
        recipe.presentation(resource.as_mut(), true, window, cx);
        assert!(!owner.read(cx).state.read(cx).presentation().is_disabled());
        owner
            .read(cx)
            .state
            .clone()
            .update(cx, |state, cx| state.focus(window, cx));
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_input(window, "Shown🙂");
    draw(cx, window);
    assert_eq!(owner.entity_id(), original_entity);
    cx.update(|cx| assert_eq!(owner.read(cx).state.read(cx).value().as_ref(), "Shown🙂"));
    assert!(
        cx.accessibility_update(window)
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == Role::PasswordInput && node.value().is_none())
    );
}

struct RevealHarness {
    old: Entity<InputOwner>,
    new: Entity<InputOwner>,
    cached_first: bool,
    hidden_new: bool,
}
impl Render for RevealHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let old = self
            .old
            .clone()
            .cached(gpui_kit::StyleRefinement::default().w(px(400.)).h(px(48.)))
            .into_any_element();
        let mut new = div().child(self.new.clone());
        if self.hidden_new {
            new.style().visibility = Some(gpui_kit::Visibility::Hidden);
        }
        let new = new.into_any_element();
        div()
            .w(px(400.))
            .h(px(160.))
            .flex()
            .flex_col()
            .children(if self.cached_first {
                [old, new]
            } else {
                [new, old]
            })
    }
}
fn empty_live() -> LiveInput {
    LiveInput::new(
        Rc::new(RefCell::new(Some(Input(Rc::new(|_, _, _, _| {
            Ok(Default::default())
        }))))),
        crate::NativeBudget::default(),
        std::sync::Arc::new(maka_plugins::call::Resources::default()),
    )
}
#[gpui::test]
fn reveal_focus_routes_the_first_platform_character_and_preserves_native_composition(
    cx: &mut TestAppContext,
) {
    use crate::Widget;
    for cached_first in [true, false] {
        cx.with_paused_automatic_drawing(|cx| {
            let (root, _) = cx.add_window_view(move |window, cx| {
                gpui_kit::init(cx);
                let old = InputOwner::new(&props(), empty_live(), false, window, cx);
                let mut offered = props();
                offered.auto_focus = true;
                let new = InputOwner::new(&offered, empty_live(), false, window, cx);
                super::InputRecipe {
                    props: offered,
                    secret: false,
                }
                .presentation(&mut new.clone(), false, window, cx);
                RevealHarness {
                    old,
                    new,
                    cached_first,
                    hidden_new: true,
                }
            });
            let window = cx.windows().last().copied().unwrap();
            let (old, new) = cx.update(|cx| (root.read(cx).old.clone(), root.read(cx).new.clone()));
            let mut offered = props();
            offered.auto_focus = true;
            let recipe = super::InputRecipe {
                props: offered.clone(),
                secret: false,
            };
            cx.update_window(window, |_, window, cx| {
                old.read(cx)
                    .state
                    .clone()
                    .update(cx, |state, cx| state.focus(window, cx));
                window.draw(cx).clear(cx);
            })
            .unwrap();
            cx.simulate_input(window, "Old buffer");
            cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
                .unwrap();
            let before = cx.update(|cx| old.read(cx).observation_snapshot());
            cx.update_window(window, |_, window, cx| {
                recipe.presentation(&mut new.clone(), true, window, cx);
                root.update(cx, |_, cx| cx.notify());
                window.draw(cx).clear(cx);
                assert!(
                    old.read(cx)
                        .state
                        .read(cx)
                        .presentation()
                        .focus_handle()
                        .is_focused(window),
                    "visually hidden ancestor cannot offer focus without actual paint"
                );
                assert!(
                    new.read(cx).focus.pending(),
                    "skipped native paint cannot consume the pending offer"
                );
                root.update(cx, |root, cx| {
                    root.hidden_new = false;
                    cx.notify();
                });
                window.draw(cx).clear(cx);
                assert!(
                    new.read(cx)
                        .state
                        .read(cx)
                        .presentation()
                        .focus_handle()
                        .is_focused(window)
                );
            })
            .unwrap();
            // No click, Tab, defer callback or second frame before this native ingress.
            cx.simulate_input(window, "First密钥🙂");
            cx.update(|cx| {
                assert_eq!(new.read(cx).state.read(cx).value().as_ref(), "First密钥🙂");
                assert_eq!(old.read(cx).observation_snapshot(), before);
            });
            cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
                .unwrap();
            cx.update_window(window, |_, window, cx| {
                old.read(cx)
                    .state
                    .clone()
                    .update(cx, |state, cx| state.focus(window, cx));
                new.update(cx, |owner, cx| owner.apply(&offered, window, cx));
                root.update(cx, |_, cx| cx.notify());
                window.draw(cx).clear(cx);
                assert!(
                    old.read(cx)
                        .state
                        .read(cx)
                        .presentation()
                        .focus_handle()
                        .is_focused(window),
                    "stable props must not steal manual focus"
                );
                recipe.presentation(&mut new.clone(), false, window, cx);
                old.update(cx, |owner, cx| {
                    owner.replace_and_mark_text_in_range(
                        None,
                        "composition",
                        Some(0..1),
                        window,
                        cx,
                    )
                });
                window.draw(cx).clear(cx);
                recipe.presentation(&mut new.clone(), true, window, cx);
                window.draw(cx).clear(cx);
                assert!(
                    old.read(cx)
                        .state
                        .read(cx)
                        .presentation()
                        .focus_handle()
                        .is_focused(window),
                    "original composition must keep focus during draw stash and retained replay"
                );
                old.update(cx, |owner, cx| owner.unmark_text(window, cx));
                root.update(cx, |_, cx| cx.notify());
                window.draw(cx).clear(cx);
                assert!(
                    old.read(cx)
                        .state
                        .read(cx)
                        .presentation()
                        .focus_handle()
                        .is_focused(window),
                    "blocked offer is consumed, never steals later"
                );
                recipe.presentation(&mut new.clone(), false, window, cx);
                recipe.presentation(&mut new.clone(), true, window, cx);
                window.draw(cx).clear(cx);
                assert!(
                    new.read(cx)
                        .state
                        .read(cx)
                        .presentation()
                        .focus_handle()
                        .is_focused(window)
                );
            })
            .unwrap();
            cx.simulate_input(window, "!");
            cx.update(|cx| {
                assert_eq!(new.read(cx).state.read(cx).value().as_ref(), "First密钥🙂!")
            });
            // AX validates the native focused field separately. Exact retained
            // handler handoff is guarded by Fast's counted custom-canvas test.
            cx.activate_accessibility(window);
            cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
                .unwrap();
            let ax = cx.accessibility_update(window).unwrap();
            let (new_node, _) = ax
                .nodes
                .iter()
                .find(|(_, node)| {
                    node.role() == Role::TextInput && node.value() == Some("First密钥🙂!")
                })
                .expect("new native AX input");
            assert_eq!(ax.focus, *new_node);
        });
    }
}

#[gpui::test]
fn simultaneous_reveal_offers_choose_one_painted_input_without_later_cycling(
    cx: &mut TestAppContext,
) {
    for cached_first in [true, false] {
        cx.with_paused_automatic_drawing(|cx| {
            let (root, _) = cx.add_window_view(move |window, cx| {
                gpui_kit::init(cx);
                let mut offered = props();
                offered.auto_focus = true;
                let old = InputOwner::new(&offered, empty_live(), false, window, cx);
                RevealHarness {
                    old,
                    new: InputOwner::new(&offered, empty_live(), false, window, cx),
                    cached_first,
                    hidden_new: false,
                }
            });
            let window = cx.windows().last().copied().unwrap();
            let (winner, loser) = cx.update(|cx| {
                let root = root.read(cx);
                if cached_first {
                    (root.old.clone(), root.new.clone())
                } else {
                    (root.new.clone(), root.old.clone())
                }
            });
            cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
                .unwrap();
            cx.simulate_input(window, "first");
            for _ in 0..3 {
                cx.update_window(window, |_, window, cx| {
                    root.update(cx, |_, cx| cx.notify());
                    window.draw(cx).clear(cx);
                    assert!(
                        winner
                            .read(cx)
                            .state
                            .read(cx)
                            .presentation()
                            .focus_handle()
                            .is_focused(window)
                    );
                })
                .unwrap();
            }
            cx.update(|cx| {
                assert_eq!(winner.read(cx).state.read(cx).value().as_ref(), "first");
                assert!(loser.read(cx).state.read(cx).value().is_empty());
            });
        });
    }
}

#[gpui::test]
fn single_line_arrows_bubble_without_changing_the_original_native_text(cx: &mut TestAppContext) {
    let MountedInput {
        owner,
        window,
        bubble,
        ..
    } = harness(cx, false);
    cx.update_window(window, |_, window, cx| {
        owner
            .read(cx)
            .state
            .clone()
            .update(cx, |state, cx| state.focus(window, cx));
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_input(window, "query");
    draw(cx, window);
    let before = cx.update(|cx| owner.read(cx).observation_snapshot());
    let previous = bubble.get();
    cx.simulate_keystrokes(window, "up down pageup pagedown");
    assert_eq!(bubble.get(), previous + 4);
    cx.update(|cx| assert_eq!(owner.read(cx).observation_snapshot(), before));
}
