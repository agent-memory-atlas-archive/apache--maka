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
use gpui_kit::{AccessibleAction, Role, accesskit};
use serde_json::json;

fn textarea(scene: &Scene, cx: &TestAppContext) -> accesskit::NodeId {
    cx.accessibility_update(scene.window)
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, node)| {
            node.role() == Role::MultilineTextInput && node.label() == Some("Write a draft")
        })
        .unwrap()
        .0
}
fn set_value(scene: &Scene, cx: &mut TestAppContext, target_node: accesskit::NodeId, value: &str) {
    cx.simulate_accessibility_action(
        scene.window,
        accesskit::ActionRequest {
            action: AccessibleAction::SetValue,
            target_tree: accesskit::TreeId::ROOT,
            target_node,
            data: Some(accesskit::ActionData::Value(value.into())),
        },
    );
}

#[gpui::test]
fn textarea_accessible_set_value_respects_live_composition_menu_and_editability(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, appearance::scoped_fixture());
    cx.activate_accessibility(scene.window);
    scene.draw(cx);
    let target = textarea(&scene, cx);
    let owner = scene.owner();
    let state = cx.update(|cx| owner.read(cx).state.clone());
    cx.with_paused_automatic_drawing(|cx| {
        let painted = cx.accessibility_update(scene.window).unwrap();
        cx.update_window(scene.window, |_, window, cx| {
            state.update(cx, |state, cx| {
                state.focus(window, cx);
                state.replace_and_mark_text_in_range(Some(0..5), "中文", Some(0..2), window, cx);
            });
        })
        .unwrap();
        assert_eq!(
            cx.accessibility_update(scene.window).unwrap().nodes,
            painted.nodes
        );
        // The old painted editable listener is still registered. The actual
        // platform action must check the composition that began after paint.
        set_value(&scene, cx, target, "Forbidden composition replacement");
        cx.update_window(scene.window, |_, window, cx| {
            state.update(cx, |state, cx| {
                assert_eq!(state.value().as_ref(), "中文");
                assert_eq!(state.marked_text_range(window, cx), Some(0..2));
                state.unmark_text(window, cx);
            });
        })
        .unwrap();
    });
    scene.draw(cx);
    cx.simulate_keystrokes(scene.window, "shift-f10");
    scene.draw(cx);
    assert!(
        cx.accessibility_update(scene.window)
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == Role::Menu)
    );
    set_value(&scene, cx, target, "Forbidden background menu replacement");
    cx.update(|cx| assert_eq!(state.read(cx).value().as_ref(), "中文"));
    cx.simulate_keystrokes(scene.window, "escape");
    scene.draw(cx);
    set_value(&scene, cx, target, "Accessible replacement");
    scene.draw(cx);
    cx.update_window(scene.window, |_, window, cx| {
        assert_eq!(state.read(cx).value().as_ref(), "Accessible replacement");
        assert!(state.read(cx).can_undo());
        let focus = state.read(cx).presentation().focus_handle().clone();
        focus.dispatch_action(&gpui_kit::base::input::Undo, window, cx);
    })
    .unwrap();
    scene.draw(cx);
    cx.update(|cx| assert_eq!(state.read(cx).value().as_ref(), "中文"));

    // Change native facts without repainting the old listener.
    cx.with_paused_automatic_drawing(|cx| {
        let painted = cx.accessibility_update(scene.window).unwrap();
        for readonly in [true, false] {
            cx.update_window(scene.window, |_, _, cx| {
                state.update(cx, |state, cx| {
                    state.set_readonly(readonly, cx);
                    state.set_disabled(!readonly, cx);
                })
            })
            .unwrap();
            assert_eq!(
                cx.accessibility_update(scene.window).unwrap().nodes,
                painted.nodes
            );
            set_value(
                &scene,
                cx,
                target,
                "Forbidden disabled or readonly replacement",
            );
            cx.update(|cx| assert_eq!(state.read(cx).value().as_ref(), "中文"));
        }
    });
    cx.update_window(scene.window, |_, window, cx| {
        state.update(cx, |state, cx| {
            state.set_disabled(false, cx);
            state.focus(window, cx);
        })
    })
    .unwrap();
    scene.draw(cx);
    // Retain the original Owner while its mount retires; a painted native
    // listener is not permission to write a resource whose LiveInput closed.
    cx.with_paused_automatic_drawing(|cx| {
        let painted = cx.accessibility_update(scene.window).unwrap();
        cx.update_window(scene.window, |_, window, cx| {
            let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
            scene
                .app
                .borrow_mut()
                .dispatch(&scene.root, revision, "toggle", json!(null))
                .unwrap();
            scene
                .surface
                .update(cx, |surface, cx| surface.sync_native(window, cx).unwrap());
            assert!(
                scene
                    .app
                    .borrow()
                    .child_mount(&scene.root, "editor")
                    .is_err()
            );
        })
        .unwrap();
        assert_eq!(
            cx.accessibility_update(scene.window).unwrap().nodes,
            painted.nodes
        );
        set_value(&scene, cx, target, "Forbidden retired replacement");
        cx.update(|cx| assert_eq!(state.read(cx).value().as_ref(), "中文"));
    });
    scene.close(cx);
}

fn text_runs(
    scene: &Scene,
    cx: &TestAppContext,
) -> (
    accesskit::NodeId,
    accesskit::Node,
    Vec<(accesskit::NodeId, accesskit::Node)>,
) {
    let update = cx.accessibility_update(scene.window).unwrap();
    let (id, parent) = update
        .nodes
        .iter()
        .find(|(_, node)| {
            node.role() == Role::MultilineTextInput && node.label() == Some("Write a draft")
        })
        .unwrap();
    let runs = parent
        .children()
        .iter()
        .filter_map(|id| {
            update
                .nodes
                .iter()
                .find(|(child, node)| child == id && node.role() == Role::TextRun)
                .cloned()
        })
        .collect();
    (*id, parent.clone(), runs)
}
fn set_selection(
    scene: &Scene,
    cx: &mut TestAppContext,
    target: accesskit::NodeId,
    selection: accesskit::TextSelection,
) {
    cx.simulate_accessibility_action(
        scene.window,
        accesskit::ActionRequest {
            action: AccessibleAction::SetTextSelection,
            target_tree: accesskit::TreeId::ROOT,
            target_node: target,
            data: Some(accesskit::ActionData::SetTextSelection(selection)),
        },
    );
}
fn selection(run: accesskit::NodeId, anchor: usize, focus: usize) -> accesskit::TextSelection {
    accesskit::TextSelection {
        anchor: accesskit::TextPosition {
            node: run,
            character_index: anchor,
        },
        focus: accesskit::TextPosition {
            node: run,
            character_index: focus,
        },
    }
}

#[gpui::test]
fn textarea_accessible_text_runs_select_original_unicode_and_empty_document(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, appearance::scoped_fixture());
    let owner = scene.owner();
    let state = cx.update(|cx| owner.read(cx).state.clone());
    cx.activate_accessibility(scene.window);
    cx.update_window(scene.window, |_, window, cx| {
        state.update(cx, |state, cx| {
            state.set_value("A中🙂e\u{301}\r\nZ", window, cx);
            state.set_selection_anchor_focus(11, 1, false, cx);
        });
    })
    .unwrap();
    scene.draw(cx);
    let (target, parent, runs) = text_runs(&scene, cx);
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].1.value(), Some("A中🙂e\u{301}\r\n"));
    assert_eq!(runs[0].1.character_lengths(), &[1, 3, 4, 1, 2, 2]);
    assert_eq!(runs[1].1.value(), Some("Z"));
    assert_eq!(runs[1].1.character_lengths(), &[1]);
    assert_eq!(parent.text_selection(), Some(&selection(runs[0].0, 5, 1)));
    assert!(parent.supports_action(AccessibleAction::SetTextSelection));
    assert!(runs.iter().any(|(_, node)| node.bounds().is_some()));
    for (_, run) in &runs {
        assert_eq!(
            run.character_lengths()
                .iter()
                .map(|n| *n as usize)
                .sum::<usize>(),
            run.value().unwrap().len()
        );
        if run.bounds().is_some() {
            assert_eq!(
                run.character_positions().unwrap().len(),
                run.character_lengths().len()
            );
            assert_eq!(
                run.character_widths().unwrap().len(),
                run.character_lengths().len()
            );
        }
    }
    let revision = cx.update(|cx| state.read(cx).document_revision());
    set_selection(&scene, cx, target, selection(runs[0].0, 4, 5));
    cx.update(|cx| {
        assert_eq!(state.read(cx).selected_range(), 9..11);
        assert_eq!(state.read(cx).document_revision(), revision);
    });
    // Read-only remains selectable through the original published listener.
    cx.update_window(scene.window, |_, _, cx| {
        state.update(cx, |s, cx| s.set_readonly(true, cx))
    })
    .unwrap();
    set_selection(&scene, cx, target, selection(runs[0].0, 5, 1));
    cx.update_window(scene.window, |_, window, cx| {
        state.update(cx, |s, cx| {
            assert_eq!(s.selected_range(), 1..11);
            let actual = s.selected_text_range(true, window, cx).unwrap();
            assert_eq!(actual.range, 1..6);
            assert!(actual.reversed);
        });
    })
    .unwrap();
    for invalid in [
        selection(runs[0].0, usize::MAX, 0),
        selection(accesskit::NodeId(0), 0, 0),
    ] {
        set_selection(&scene, cx, target, invalid);
        cx.update(|cx| assert_eq!(state.read(cx).selected_range(), 1..11));
    }
    cx.update_window(scene.window, |_, _, cx| {
        state.update(cx, |s, cx| s.set_disabled(true, cx))
    })
    .unwrap();
    set_selection(&scene, cx, target, selection(runs[0].0, 0, 0));
    cx.update(|cx| assert_eq!(state.read(cx).selected_range(), 1..11));
    cx.update_window(scene.window, |_, window, cx| {
        state.update(cx, |s, cx| {
            s.set_disabled(false, cx);
            s.set_readonly(false, cx);
            s.set_value("", window, cx);
        })
    })
    .unwrap();
    scene.draw(cx);
    let (target, parent, empty) = text_runs(&scene, cx);
    assert_eq!(empty.len(), 1);
    assert_eq!(empty[0].1.value(), Some(""));
    assert!(empty[0].1.character_lengths().is_empty());
    assert_eq!(parent.text_selection(), Some(&selection(empty[0].0, 0, 0)));
    set_selection(&scene, cx, target, selection(empty[0].0, 0, 0));
    cx.update(|cx| assert_eq!(state.read(cx).selected_range(), 0..0));
    scene.close(cx);
}

#[gpui::test]
fn textarea_accessible_selection_rejects_queued_old_text_and_reflow_positions(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, appearance::scoped_fixture());
    let owner = scene.owner();
    let state = cx.update(|cx| owner.read(cx).state.clone());
    cx.activate_accessibility(scene.window);
    scene.draw(cx);
    let (target, _, old) = text_runs(&scene, cx);
    let request = selection(old[0].0, 0, 1);
    cx.with_paused_automatic_drawing(|cx| {
        let painted = cx.accessibility_update(scene.window).unwrap();
        cx.update_window(scene.window, |_, window, cx| {
            state.update(cx, |s, cx| {
                s.set_value("12345", window, cx);
                s.set_selection_anchor_focus(3, 3, false, cx);
            })
        })
        .unwrap();
        assert_eq!(
            cx.accessibility_update(scene.window).unwrap().nodes,
            painted.nodes
        );
        // Before paint, the listener and run map are old but native revision is new.
        set_selection(&scene, cx, target, request);
        cx.update(|cx| assert_eq!(state.read(cx).selected_range(), 3..3));
    });
    scene.draw(cx);
    // After paint, the platform resolves the latest listener, not the listener
    // that existed when an assistive technology formed this ActionData.
    set_selection(&scene, cx, target, request);
    cx.update(|cx| assert_eq!(state.read(cx).selected_range(), 3..3));
    let text = "中🙂".repeat(200);
    cx.update_window(scene.window, |_, window, cx| {
        state.update(cx, |s, cx| {
            s.set_value(text.clone(), window, cx);
            s.set_selection_anchor_focus(0, 0, false, cx);
        })
    })
    .unwrap();
    scene.draw(cx);
    let (_, _, wide) = text_runs(&scene, cx);
    let wide_request = selection(wide[0].0, 1, 2);
    let revision = cx.update(|cx| state.read(cx).document_revision());
    cx.simulate_window_resize(scene.window, size(px(240.), px(400.)));
    scene.draw(cx);
    let (_, _, narrow) = text_runs(&scene, cx);
    assert_ne!(
        wide[0].1.value(),
        narrow[0].1.value(),
        "actual native soft-wrap partition changed"
    );
    assert!(narrow.iter().all(|(id, _)| *id != wide[0].0));
    assert_eq!(
        narrow
            .iter()
            .map(|(_, n)| n.value().unwrap())
            .collect::<String>(),
        text
    );
    cx.update(|cx| assert_eq!(state.read(cx).document_revision(), revision));
    set_selection(&scene, cx, target, wide_request);
    cx.update(|cx| assert_eq!(state.read(cx).selected_range(), 0..0));
    set_selection(&scene, cx, target, selection(narrow[0].0, 2, 1));
    cx.update(|cx| assert_eq!(state.read(cx).selected_range(), 3..7));
    scene.close(cx);
}

#[gpui::test]
fn textarea_accessible_geometry_words_and_wrap_affinity_match_native(cx: &mut TestAppContext) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, appearance::scoped_fixture());
    let owner = scene.owner();
    let state = cx.update(|cx| owner.read(cx).state.clone());
    cx.activate_accessibility(scene.window);
    cx.simulate_window_scale_factor_change(scene.window, 2.);
    appearance::event(&scene, json!({"softWrap":false}), cx);
    cx.update_window(scene.window, |_, window, cx| {
        state.update(cx, |s, cx| {
            s.set_value("can't a b\n\n".to_owned() + &"a".repeat(600), window, cx);
            // Keep the entire third logical line on one visual row to force the
            // bounded fragments to link within the same actual line.
            s.set_selection_anchor_focus(0, 0, false, cx);
        })
    })
    .unwrap();
    scene.draw(cx);
    let (target, _, runs) = text_runs(&scene, cx);
    assert_eq!(runs[0].1.word_starts(), &[0, 6, 8]);
    assert_eq!(
        runs[0]
            .1
            .character_widths()
            .unwrap()
            .last()
            .copied()
            .unwrap(),
        cx.update(
            |cx| state.read(cx).text_accessibility().snapshot().unwrap().runs[0]
                .geometry
                .as_ref()
                .unwrap()
                .character_widths
                .last()
                .copied()
                .unwrap()
        ) * 2.
    );
    assert!(runs[0].1.character_widths().unwrap().last().unwrap() > &0.);
    assert!(
        runs[1].1.bounds().is_none(),
        "unmeasured offscreen row has no invented geometry"
    );
    let fragments: Vec<_> = runs
        .iter()
        .filter(|(_, run)| {
            run.value().unwrap().starts_with('a') && run.value().unwrap().len() > 200
        })
        .collect();
    assert_eq!(fragments.len(), 2);
    assert_eq!(fragments[0].1.character_lengths().len(), 255);
    assert_eq!(fragments[0].1.next_on_line(), Some(fragments[1].0));
    assert_eq!(fragments[1].1.previous_on_line(), Some(fragments[0].0));
    assert!(fragments[1].1.word_starts().is_empty());
    // Compare the actual first character range measured by the input handler,
    // not merely two copies of the new projection.
    cx.update_window(scene.window, |_, window, cx| {
        state.update(cx, |s, cx| {
            let native = s
                .bounds_for_range(0..1, s.input_bounds(), window, cx)
                .unwrap();
            let rect = runs[0].1.bounds().unwrap();
            let positions = runs[0].1.character_positions().unwrap();
            let widths = runs[0].1.character_widths().unwrap();
            assert!(
                (rect.x0 + positions[0] as f64 - f32::from(native.left()) as f64 * 2.).abs() < 0.01
            );
            assert!((widths[0] as f64 - f32::from(native.size.width) as f64 * 2.).abs() < 0.01);
        })
    })
    .unwrap();
    cx.update_window(scene.window, |_, _, cx| {
        state.update(cx, |s, cx| {
            s.set_selection_anchor_focus(10, 10, false, cx);
        })
    })
    .unwrap();
    scene.draw(cx);
    let (_, _, empty_line) = text_runs(&scene, cx);
    assert_eq!(empty_line[1].1.value(), Some("\n"));
    assert!(empty_line[1].1.character_widths().unwrap()[0] > 0.);
    appearance::event(&scene, json!({"softWrap":true}), cx);
    cx.update_window(scene.window, |_, window, cx| {
        state.update(cx, |s, cx| {
            s.set_value("中🙂".repeat(80), window, cx);
            s.set_selection_anchor_focus(0, 0, false, cx);
            s.focus(window, cx);
        })
    })
    .unwrap();
    scene.draw(cx);
    let (_, _, wrapped) = text_runs(&scene, cx);
    assert!(wrapped.len() > 1);
    let end_index = wrapped[0].1.character_lengths().len();
    let end_byte = wrapped[0].1.value().unwrap().len();
    let line_end = selection(wrapped[0].0, end_index, end_index);
    set_selection(&scene, cx, target, line_end);
    scene.draw(cx);
    let (_, parent, updated) = text_runs(&scene, cx);
    assert_eq!(parent.text_selection(), Some(&line_end));
    cx.update(|cx| assert_eq!(state.read(cx).selected_range(), end_byte..end_byte));
    cx.update_window(scene.window, |_, _, cx| {
        let (caret, _) = state.read(cx).cursor_layout().unwrap();
        assert!(
            (f32::from(caret.origin.y + state.read(cx).scroll_offset().y) as f64 * 2.
                - updated[0].1.bounds().unwrap().y0)
                .abs()
                < updated[0].1.bounds().unwrap().height()
        );
    })
    .unwrap();
    // The other native position of that same byte belongs to the next row.
    let line_start = selection(updated[1].0, 0, 0);
    set_selection(&scene, cx, target, line_start);
    scene.draw(cx);
    let (_, parent, updated) = text_runs(&scene, cx);
    assert_eq!(parent.text_selection(), Some(&line_start));
    cx.update_window(scene.window, |_, _, cx| {
        let (caret, _) = state.read(cx).cursor_layout().unwrap();
        assert!(
            (f32::from(caret.origin.y + state.read(cx).scroll_offset().y) as f64 * 2.
                - updated[1].1.bounds().unwrap().y0)
                .abs()
                < updated[1].1.bounds().unwrap().height()
        );
    })
    .unwrap();
    scene.close(cx);
}

#[gpui::test]
fn textarea_accessible_selection_respects_composition_menu_foreign_and_retired_owner(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, appearance::scoped_fixture());
    let owner = scene.owner();
    let state = cx.update(|cx| owner.read(cx).state.clone());
    cx.activate_accessibility(scene.window);
    scene.draw(cx);
    cx.update_window(scene.window, |_, window, cx| {
        state.update(cx, |s, cx| {
            s.focus(window, cx);
            s.replace_and_mark_text_in_range(Some(0..5), "中文", Some(0..2), window, cx);
        })
    })
    .unwrap();
    scene.draw(cx);
    let (target, _, marked) = text_runs(&scene, cx);
    let before = cx.update(|cx| state.read(cx).selected_range());
    set_selection(&scene, cx, target, selection(marked[0].0, 0, 0));
    cx.update_window(scene.window, |_, window, cx| {
        state.update(cx, |s, cx| {
            assert_eq!(s.selected_range(), before);
            assert_eq!(s.marked_text_range(window, cx), Some(0..2));
            s.unmark_text(window, cx);
        })
    })
    .unwrap();
    scene.draw(cx);
    let (_, _, committed) = text_runs(&scene, cx);
    cx.simulate_keystrokes(scene.window, "shift-f10");
    scene.draw(cx);
    assert!(
        cx.accessibility_update(scene.window)
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.role() == Role::Menu)
    );
    set_selection(&scene, cx, target, selection(committed[0].0, 0, 0));
    cx.update(|cx| assert_eq!(state.read(cx).selected_range(), before));
    cx.simulate_keystrokes(scene.window, "escape");
    scene.draw(cx);
    let (_, _, committed) = text_runs(&scene, cx);
    let update = cx.accessibility_update(scene.window).unwrap();
    let other = &update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::MultilineTextInput && n.label() == Some("Other draft"))
        .unwrap()
        .1;
    let foreign = *other
        .children()
        .iter()
        .find(|id| {
            update
                .nodes
                .iter()
                .any(|(node, n)| node == *id && n.role() == Role::TextRun)
        })
        .unwrap();
    set_selection(&scene, cx, target, selection(foreign, 0, 0));
    cx.update(|cx| assert_eq!(state.read(cx).selected_range(), before));
    cx.with_paused_automatic_drawing(|cx| {
        let painted = cx.accessibility_update(scene.window).unwrap();
        cx.update_window(scene.window, |_, window, cx| {
            let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
            scene
                .app
                .borrow_mut()
                .dispatch(&scene.root, revision, "toggle", json!(null))
                .unwrap();
            scene
                .surface
                .update(cx, |surface, cx| surface.sync_native(window, cx).unwrap());
        })
        .unwrap();
        assert_eq!(
            cx.accessibility_update(scene.window).unwrap().nodes,
            painted.nodes
        );
        // Keep native entities alive while the original mount's custody closes.
        set_selection(&scene, cx, target, selection(committed[0].0, 0, 0));
        cx.update(|cx| {
            assert_eq!(state.read(cx).selected_range(), before);
            assert_eq!(state.read(cx).value().as_ref(), "中文");
        });
    });
    scene.close(cx);
}
