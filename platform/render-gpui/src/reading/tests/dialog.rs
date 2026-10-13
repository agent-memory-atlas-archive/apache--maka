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
use gpui_kit::base::{TextSelection, TextSelectionHandle};
use gpui_kit::{AnyWindowHandle, Entity, FocusHandle};
use maka_ui_runtime::{appearance::Color, source::SourceTextProps};

fn props(source: &str) -> SourceTextProps {
    SourceTextProps {
        animate: false,
        terminal: None,
        focus_on_pointer: true,
        source: source.into(),
        appearance: appearance().body,
        selection: Color(0x4466ff55),
        disabled: false,
    }
}
fn source(open: bool) -> Value {
    scene_source(
        json!([{"kind":"dialog","key":"modal","label":"Foreground","open":open,
        "background":{"kind":"component","key":"background","binding":"source","member":0,"props":{},"on":{}},
        "content":{"kind":"component","key":"foreground","binding":"source","member":0,"props":{},"on":{}}}]),
    )
}
struct Fixture {
    view: Entity<Harness>,
    window: AnyWindowHandle,
    background: TextSelectionHandle,
    foreground: TextSelectionHandle,
    scope: TextSelectionScopeId,
    background_focus: FocusHandle,
    foreground_focus: FocusHandle,
    background_update: crate::source::TestSourceUpdate,
}
fn draw(cx: &mut TestAppContext, window: AnyWindowHandle) {
    cx.update_window(window, |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
}
fn fixture(cx: &mut TestAppContext) -> Fixture {
    cx.update(gpui_kit::init);
    let handles = Rc::new(RefCell::new(None));
    let out = handles.clone();
    let (view, _) = cx.add_window_view(move |window, cx| {
        let bg = crate::source::test_source(&props("  background\n中文🙂  "), input(), window, cx);
        let fg = crate::source::test_source(&props("  foreground\n选中🙂  "), input(), window, cx);
        let recipe = compile_recipe(&source(false), None);
        let mut state = recipe.create(input(), window, cx);
        recipe.presentation(state.as_mut(), true, window, cx);
        *out.borrow_mut() = Some((
            bg.handle.clone(),
            fg.handle.clone(),
            owner(state.as_ref()).read(cx).scope,
            bg.focus.clone(),
            fg.focus.clone(),
            bg.update,
        ));
        let slots = Slots::new(
            vec!["background".into(), "foreground".into()],
            Input::testing(Rc::new(|_, _, _, _| Ok(()))),
            move |key, window, cx| match key {
                "background" => bg
                    .widget
                    .render(bg.state.as_ref(), &empty_slots(), window, cx),
                "foreground" => fg
                    .widget
                    .render(fg.state.as_ref(), &empty_slots(), window, cx),
                _ => unreachable!(),
            },
        );
        Harness {
            recipe,
            state,
            slots,
            renders: Default::default(),
            block_long_press: Default::default(),
            foreground: None,
        }
    });
    let window = cx.windows()[0];
    draw(cx, window);
    let (background, foreground, scope, background_focus, foreground_focus, background_update) =
        handles.borrow_mut().take().unwrap();
    Fixture {
        view,
        window,
        background,
        foreground,
        scope,
        background_focus,
        foreground_focus,
        background_update,
    }
}
fn open(cx: &mut TestAppContext, f: &Fixture, open: bool) {
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let recipe = compile_recipe(&source(open), None);
            recipe.commit(view.state.as_mut(), window, cx);
            view.recipe = recipe;
            cx.notify();
        })
    })
    .unwrap();
}
#[gpui::test]
fn controlled_dialog_foreground_source_does_not_inherit_background_reading_document(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        assert!(f.background.activate_scope(window, cx));
        f.background.set_local_selection(true, cx);
    })
    .unwrap();
    draw(cx, f.window);
    let bg = cx.update(|cx| f.background.group(cx).unwrap());
    assert_eq!(
        cx.update(|cx| bg.has_selection_in_scope(f.scope, cx)),
        Some(true)
    );
    open(cx, &f, true);
    draw(cx, f.window);
    cx.update_window(f.window, |_, window, cx| {
        assert!(f.foreground.activate_scope(window, cx));
        f.foreground.set_local_selection(true, cx);
    })
    .unwrap();
    draw(cx, f.window);
    cx.update_window(f.window, |_, window, cx| {
        let fg = f.foreground.group(cx).unwrap();
        assert!(!fg.same_owner(&bg));
        assert_eq!(
            fg.has_selection_in_scope(f.scope, cx),
            Some(false),
            "foreground is a separate native document, even within a Reading"
        );
        assert_eq!(bg.has_selection_in_scope(f.scope, cx), Some(true));
        assert_eq!(
            f.foreground.selected_text_in_scope(window, cx).as_deref(),
            Some("  foreground\n选中🙂  ")
        );
        assert_eq!(
            f.background.selected_text_in_scope(window, cx).as_deref(),
            Some("  background\n中文🙂  ")
        );
    })
    .unwrap();
    open(cx, &f, false);
    draw(cx, f.window);
    cx.update_window(f.window, |_, window, cx| {
        assert_eq!(
            TextSelection::selected_text(window, cx),
            "  background\n中文🙂  "
        );
        assert_eq!(bg.has_selection_in_scope(f.scope, cx), Some(true));
    })
    .unwrap();
}

#[gpui::test]
fn controlled_dialog_copy_and_reading_tail_follow_original_selection_groups(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.background_focus.focus(window, cx)
    })
    .unwrap();
    draw(cx, f.window);
    cx.simulate_keystrokes(f.window, "cmd-a cmd-c");
    draw(cx, f.window);
    cx.update(|cx| {
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("  background\n中文🙂  ")
        )
    });
    cx.update_window(f.window, |_, window, cx| {
        let view = f.view.read(cx);
        let reading = owner(view.state.as_ref());
        assert!(
            !reading.read(cx).eligible(window, cx),
            "completed background selection prevents tail continuation"
        );
    })
    .unwrap();
    open(cx, &f, true);
    draw(cx, f.window);
    cx.update_window(f.window, |_, window, cx| {
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("sentinel".into()));
        f.background_focus
            .dispatch_action(&gpui_kit::base::input::Copy, window, cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("sentinel")
        );
        f.foreground_focus.focus(window, cx);
    })
    .unwrap();
    draw(cx, f.window);
    cx.simulate_keystrokes(f.window, "cmd-a cmd-c");
    draw(cx, f.window);
    cx.update(|cx| {
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("  foreground\n选中🙂  ")
        )
    });
    cx.update_window(f.window, |_, window, cx| {
        let view = f.view.read(cx);
        assert!(!owner(view.state.as_ref()).read(cx).eligible(window, cx));
    })
    .unwrap();
    open(cx, &f, false);
    draw(cx, f.window);
    cx.simulate_keystrokes(f.window, "cmd-c");
    cx.update(|cx| {
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("  background\n中文🙂  ")
        )
    });
}
#[gpui::test]
fn controlled_dialog_close_does_not_restore_disabled_literal_source(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.background_focus.focus(window, cx)
    })
    .unwrap();
    draw(cx, f.window);
    open(cx, &f, true);
    draw(cx, f.window);
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                let next = compile_recipe(&source(false), None);
                next.commit(view.state.as_mut(), window, cx);
                view.recipe = next;
                cx.notify();
            });
            let mut disabled = props("  background\n中文🙂  ");
            disabled.disabled = true;
            (f.background_update)(&disabled, window, cx);
        })
        .unwrap();
        draw(cx, f.window);
        cx.update_window(f.window, |_, window, cx| {
            assert!(window.focused(cx).is_none())
        })
        .unwrap();
    });
}

#[gpui::test]
fn controlled_dialog_foreground_reading_establishes_its_original_document(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let (handle, scope) = cx
        .update_window(f.window, |_, window, cx| {
            let fg =
                crate::source::test_source(&props("  foreground\n选中🙂  "), input(), window, cx);
            let scene = scene_source(json!([{
                "kind": "component", "key": "text", "binding": "source",
                "member": 0, "props": {}, "on": {}
            }]));
            let reading = compile_recipe(&scene, None);
            let mut state = reading.create(input(), window, cx);
            reading.presentation(state.as_mut(), true, window, cx);
            let scope = owner(state.as_ref()).read(cx).scope;
            let handle = fg.handle.clone();
            let inner = Slots::new(
                vec!["text".into()],
                Input::testing(Rc::new(|_, _, _, _| Ok(()))),
                move |_, window, cx| {
                    fg.widget
                        .render(fg.state.as_ref(), &empty_slots(), window, cx)
                },
            );
            f.view.update(cx, |view, cx| {
                let old = view.slots.clone();
                view.slots = Slots::new(
                    vec!["background".into(), "foreground".into()],
                    old.input.clone(),
                    move |key, window, cx| {
                        if key == "foreground" {
                            reading.render(state.as_ref(), &inner, window, cx)
                        } else {
                            old.render(key, window, cx)
                        }
                    },
                );
                cx.notify();
            });
            (handle, scope)
        })
        .unwrap();
    open(cx, &f, true);
    draw(cx, f.window);
    assert_ne!(scope, f.scope);
    cx.update_window(f.window, |_, window, cx| {
        assert!(handle.activate_scope(window, cx));
        handle.set_local_selection(true, cx);
    })
    .unwrap();
    draw(cx, f.window);
    cx.update_window(f.window, |_, window, cx| {
        let group = handle.group(cx).unwrap();
        assert_eq!(group.has_selection_in_scope(scope, cx), Some(true));
        assert_eq!(group.has_selection_in_scope(f.scope, cx), Some(false));
        assert_eq!(
            handle.selected_text_in_scope(window, cx).as_deref(),
            Some("  foreground\n选中🙂  ")
        );
    })
    .unwrap();
}

// Link interaction remains available even when a Markdown document is not selectable.
#[gpui::test]
fn controlled_dialog_restores_nonselectable_markdown_but_not_disabled_document(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let (document, focus) = cx
        .update_window(f.window, |_, window, cx| {
            let document = crate::markdown::test_document(
                &maka_ui_runtime::markdown::MarkdownProps {
                    animate: false,
                    ascii: false,
                    source: "[Original link](https://example.com)".into(),
                    appearance: appearance(),
                    selectable: false,
                    disabled: false,
                    focus_on_pointer: true,
                    open_web_links: true,
                    bubble_links: true,
                },
                input(),
                window,
                cx,
            );
            let state = document.document.clone();
            let focus = state.read(cx).focus_handle().clone();
            f.view.update(cx, |view, cx| {
                let old = view.slots.clone();
                view.slots = Slots::new(
                    vec!["background".into(), "foreground".into()],
                    old.input.clone(),
                    move |key, window, cx| {
                        if key == "background" {
                            document.widget.render(
                                document.state.as_ref(),
                                &empty_slots(),
                                window,
                                cx,
                            )
                        } else {
                            old.render(key, window, cx)
                        }
                    },
                );
                cx.notify();
            });
            (state, focus)
        })
        .unwrap();
    draw(cx, f.window);
    cx.update_window(f.window, |_, window, cx| focus.focus(window, cx))
        .unwrap();
    draw(cx, f.window);
    open(cx, &f, true);
    draw(cx, f.window);
    open(cx, &f, false);
    draw(cx, f.window);
    cx.update_window(f.window, |_, window, _| assert!(focus.is_focused(window)))
        .unwrap();
    open(cx, &f, true);
    draw(cx, f.window);
    cx.with_paused_automatic_drawing(|cx| {
        open(cx, &f, false);
        cx.update(|cx| document.update(cx, |state, cx| state.set_interaction_enabled(false, cx)));
        draw(cx, f.window);
        cx.update_window(f.window, |_, window, cx| {
            assert!(window.focused(cx).is_none())
        })
        .unwrap();
    });
}
