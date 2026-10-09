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
use crate::{Input, NativeBudget};
use gpui_kit::{Modifiers, TestAppContext, point};
use std::cell::RefCell;
fn props() -> FilePickerProps {
    serde_json::from_value(serde_json::json!({
        "appearance": {"text":{"font":{"family":".SystemUIFont","fallbacks":[],"size":14,"lineHeight":1.5,"weight":400},"foreground":0x242724ffu32},
            "mutedForeground":0x656a63ffu32,"background":0xfaf9f6ffu32,"selection":0xa7c4b0ffu32,"caret":0x242724ffu32,
            "border":0x656a63ffu32,"focusedBorder":0x416653ffu32,"borderWidth":1,"radius":8,"padding":{"top":8,"right":12,"bottom":8,"left":12}},
        "label":"Choose plugin file","pathLabel":"Local file path","browseLabel":"Browse","parentLabel":"Parent","emptyLabel":"No files or folders","truncatedLabel":"First 2048 entries","placeholder":"/absolute/path","disabled":false,"tabIndex":0
    })).unwrap()
}
struct Root {
    owner: Option<Entity<Owner>>,
}
impl Render for Root {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut root = div().size_full();
        if let Some(owner) = &self.owner {
            root = root.child(owner.clone());
        }
        root
    }
}
#[gpui_kit::test]
fn real_native_click_commits_one_prompt_and_retirement_keeps_original_dialog_custody(
    cx: &mut TestAppContext,
) {
    for mode in 0..3 {
        let props = props();
        let budget = NativeBudget::default();
        let group = Arc::new(maka_plugins::call::Resources::default());
        let keep = Rc::new(());
        let witness = Rc::downgrade(&keep);
        let gestures = Rc::new(RefCell::new(Vec::<FileChoose>::new()));
        let seen = gestures.clone();
        let live = LiveInput::new(
            Rc::new(RefCell::new(Some(Input(Rc::new(
                move |key, value, _, _| {
                    let _ = &keep;
                    if key == "choose" {
                        seen.borrow_mut()
                            .push(serde_json::from_value(value).unwrap());
                    }
                    Ok(Default::default())
                },
            ))))),
            budget.clone(),
            group.clone(),
        );
        let (root, cx) = cx.add_window_view(|_, cx| Root {
            owner: Some(Owner::new(&props, live.clone(), cx)),
        });
        let owner = root.read_with(cx, |root, _| root.owner.as_ref().unwrap().clone());
        cx.update(|window, cx| {
            owner.update(cx, |owner, cx| {
                owner.apply(
                    &FileView {
                        props: props.clone(),
                        request: Some("forged".into()),
                    },
                    window,
                    cx,
                )
            })
        });
        assert!(!cx.did_prompt_for_paths());
        cx.simulate_click(point(px(12.), px(12.)), Modifiers::default());
        assert_eq!(gestures.borrow().len(), 1);
        assert!(
            !cx.did_prompt_for_paths(),
            "native click must commit its request first"
        );
        let view = FileView {
            props: props.clone(),
            request: Some(gestures.borrow()[0].request.clone()),
        };
        cx.update(|window, cx| {
            owner.update(cx, |owner, cx| {
                owner.read_failure = mode == 2;
                owner.apply(&view, window, cx);
            });
            owner.update(cx, |owner, cx| owner.apply(&view, window, cx));
        });
        assert!(cx.did_prompt_for_paths());
        assert_eq!(budget.usage().2, 1);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.maka-extension");
        std::fs::write(&path, b"immutable selection").unwrap();
        if mode == 0 {
            live.0.borrow_mut().take();
            cx.update(|window, cx| {
                Recipe(view.clone()).deactivate(&mut owner.clone(), window, cx);
                root.update(cx, |r, cx| {
                    r.owner = None;
                    cx.notify();
                });
            });
            drop(owner);
            cx.run_until_parked();
            assert!(!group.seal().unwrap());
            assert!(witness.upgrade().is_some());
            assert_eq!(budget.usage().2, 1);
            cx.simulate_path_prompt_response(|options| {
                assert!(options.files && !options.directories && !options.multiple);
                None
            });
            cx.run_until_parked();
            assert!(group.seal().unwrap());
            assert!(witness.upgrade().is_none());
            assert_eq!(budget.usage(), (0, 0, 0));
        } else {
            cx.simulate_path_prompt_response(|_| Some(vec![path.clone()]));
            cx.run_until_parked();
            let observed = owner.read_with(cx, |owner, _| owner.selection.observation());
            if mode == 2 {
                assert_eq!(observed.phase, FilePhase::Failed);
                assert_eq!(observed.error, Some(FileError::WorkerFailed));
                assert!(group.seal().is_err());
            } else {
                assert_eq!(observed.phase, FilePhase::Ready);
                let capture = owner.read_with(cx, |owner, _| {
                    owner
                        .selection
                        .capture(&FileCapture {
                            owner: observed.owner,
                            revision: observed.revision,
                        })
                        .unwrap()
                });
                std::fs::write(&path, b"new contents").unwrap();
                assert_eq!(capture.read(0..19).unwrap(), b"immutable selection");
                assert!(group.seal().unwrap());
            }
            live.0.borrow_mut().take();
            cx.update(|window, cx| {
                Recipe(view).deactivate(&mut owner.clone(), window, cx);
                root.update(cx, |r, cx| {
                    r.owner = None;
                    cx.notify();
                });
            });
            drop(owner);
            cx.run_until_parked();
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
            });
            cx.run_until_parked();
            assert!(witness.upgrade().is_none());
            assert_eq!(budget.usage(), (0, 0, 0));
        }
    }
}

#[gpui_kit::test]
fn disabled_or_hidden_owner_rejects_old_native_focus_before_repaint(cx: &mut TestAppContext) {
    use gpui_kit::{AccessibleAction, VisualTestContext, accesskit};
    let props = props();
    let seen = Rc::new(RefCell::new(0));
    let clicks = seen.clone();
    let live = LiveInput::new(
        Rc::new(RefCell::new(Some(Input(Rc::new(move |key, _, _, _| {
            if key == "choose" {
                *clicks.borrow_mut() += 1;
            }
            Ok(Default::default())
        }))))),
        NativeBudget::default(),
        Arc::new(maka_plugins::call::Resources::default()),
    );
    let (root, cx) = cx.add_window_view(|_, cx| Root {
        owner: Some(Owner::new(&props, live.clone(), cx)),
    });
    let owner = root.read_with(cx, |root, _| root.owner.as_ref().unwrap().clone());
    let window = cx.update(|w, _| w.window_handle());
    cx.activate_accessibility(window);
    cx.update(|w, cx| {
        w.draw(cx).clear(cx);
    });
    let target = cx
        .accessibility_update(window)
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, n)| n.role() == accesskit::Role::Button)
        .unwrap()
        .0;
    for hidden in [false, true] {
        cx.cx.with_paused_automatic_drawing(|cx| {
            cx.update_window(window, |_, w, cx| {
                owner.update(cx, |owner, cx| {
                    owner.props.disabled = !hidden;
                    owner.presentation(!hidden, w, cx);
                    assert!(!owner.focus.tab_stop);
                    assert_eq!(owner.focus.tab_index, -1);
                })
            })
            .unwrap();
            VisualTestContext::from_window(window, cx)
                .simulate_click(point(px(12.), px(12.)), Modifiers::default());
            cx.simulate_accessibility_action(
                window,
                accesskit::ActionRequest {
                    action: AccessibleAction::Focus,
                    target_tree: accesskit::TreeId::ROOT,
                    target_node: target,
                    data: None,
                },
            );
            cx.simulate_keystrokes(window, "tab");
            cx.update_window(window, |_, w, cx| assert!(w.focused(cx).is_none()))
                .unwrap();
            assert_eq!(*seen.borrow(), 0);
            cx.update_window(window, |_, w, cx| {
                owner.update(cx, |owner, cx| {
                    owner.props.disabled = false;
                    owner.presentation(true, w, cx);
                })
            })
            .unwrap();
            cx.simulate_accessibility_action(
                window,
                accesskit::ActionRequest {
                    action: AccessibleAction::Focus,
                    target_tree: accesskit::TreeId::ROOT,
                    target_node: target,
                    data: None,
                },
            );
            cx.update_window(window, |_, w, cx| {
                assert!(owner.read(cx).focus.is_focused(w))
            })
            .unwrap();
        });
    }
    live.0.borrow_mut().take();
    cx.update(|w, cx| {
        owner.update(cx, |owner, cx| {
            owner.active = false;
            owner.presentation(false, w, cx);
        });
        root.update(cx, |root, cx| {
            root.owner = None;
            cx.notify();
        });
    });
}

#[gpui_kit::test]
fn original_press_cannot_reactivate_across_hidden_or_disabled_aba(cx: &mut TestAppContext) {
    use gpui_kit::{MouseButton, VisualTestContext};
    let props = props();
    let seen = Rc::new(RefCell::new(0));
    let clicks = seen.clone();
    let live = LiveInput::new(
        Rc::new(RefCell::new(Some(Input(Rc::new(move |key, _, _, _| {
            if key == "choose" {
                *clicks.borrow_mut() += 1;
            }
            Ok(Default::default())
        }))))),
        NativeBudget::default(),
        Arc::new(maka_plugins::call::Resources::default()),
    );
    let (root, cx) = cx.add_window_view(|_, cx| Root {
        owner: Some(Owner::new(&props, live.clone(), cx)),
    });
    let owner = root.read_with(cx, |root, _| root.owner.as_ref().unwrap().clone());
    let window = cx.update(|w, _| w.window_handle());
    cx.update(|w, cx| {
        w.draw(cx).clear(cx);
    });
    for hidden in [false, true] {
        for redraw in [false, true] {
            let before = *seen.borrow();
            cx.cx.with_paused_automatic_drawing(|cx| {
                VisualTestContext::from_window(window, cx).simulate_mouse_down(
                    point(px(12.), px(12.)),
                    MouseButton::Left,
                    Modifiers::default(),
                );
                cx.update_window(window, |_, w, cx| {
                    owner.update(cx, |owner, cx| {
                        owner.props.disabled = !hidden;
                        owner.presentation(!hidden, w, cx);
                        owner.props.disabled = false;
                        owner.presentation(true, w, cx);
                    })
                })
                .unwrap();
                if redraw {
                    cx.update_window(window, |_, w, cx| {
                        w.draw(cx).clear(cx);
                    })
                    .unwrap();
                }
                VisualTestContext::from_window(window, cx).simulate_mouse_up(
                    point(px(12.), px(12.)),
                    MouseButton::Left,
                    Modifiers::default(),
                );
                assert_eq!(*seen.borrow(), before, "original press was cancelled");
            });
            cx.simulate_click(point(px(12.), px(12.)), Modifiers::default());
            assert_eq!(
                *seen.borrow(),
                before + 1,
                "fresh native gesture remains admitted"
            );
        }
    }
}
