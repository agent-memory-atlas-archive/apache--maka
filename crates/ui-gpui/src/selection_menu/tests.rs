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
use crate::{Input, scene};
use gpui_kit::base::TextSelectionLayer;
use gpui_kit::{
    AnyWindowHandle, FocusHandle, InputScope, LongPressEvent, Modifiers, TestAppContext,
    TouchPhase, VisualTestContext, accesskit, point,
};
use maka_ui::{appearance::*, source::SourceTextProps};
use serde_json::Value;
use std::{
    cell::RefCell,
    sync::Arc,
    time::{Duration, Instant},
};
const BACKGROUND: &str = "  background\n中文🙂  ";
const FOREGROUND: &str = "  foreground\n选中🙂  ";
fn input() -> LiveInput {
    LiveInput::new(
        Rc::new(RefCell::new(Some(Input(Rc::new(|_, _, _, _| {
            Ok(Default::default())
        }))))),
        crate::NativeBudget::default(),
        Arc::new(maka_plugins::call::Resources::default()),
    )
}
fn text() -> TextAppearance {
    TextAppearance {
        font: FontAppearance {
            family: FontFamily("Helvetica".into()),
            fallbacks: vec![],
            size: 16.,
            line_height: 1.5,
            weight: 400,
        },
        foreground: Color(0x242724ff),
    }
}
fn props(title: &str) -> SelectionMenuProps {
    let padding = Insets {
        top: 4.,
        right: 4.,
        bottom: 4.,
        left: 4.,
    };
    SelectionMenuProps {
        disabled: false,
        labels: SelectionMenuLabels {
            title: title.into(),
            copy: format!("Copy {title}"),
            select_all: format!("All {title}"),
        },
        appearance: MenuAppearance {
            text: text(),
            background: Color(0xf9fafbff),
            border: Color(0x8899aaff),
            active_background: Color(0x4455aaff),
            active_foreground: Color(0xffffffff),
            disabled_foreground: Color(0x999999ff),
            border_width: 1.,
            radius: 8.,
            padding,
            row_padding: padding,
            width: 180.,
            window_margin: 8.,
        },
    }
}
fn slot(key: &str) -> Value {
    json!({"kind":"component","key":key,"binding":"native","member":0,"props":{},"on":{}})
}
fn node(open: bool, moved: bool) -> Value {
    let mut bg = vec![slot("bg-source")];
    if !moved {
        bg.push(slot("bg-menu"));
    }
    json!({"kind":"dialog","key":"dialog","label":"Selection foreground","open":open,
        "style":{"width":{"unit":"pixels","value":380},"padding":{"top":16,"right":16,"bottom":16,"left":16}},
        "background":{"kind":"stack","key":"bg","direction":"column","gap":0,"padding":0,"background":0,"children":bg},
        "content":{"kind":"stack","key":"fg","direction":"column","gap":0,"padding":0,"background":0,
            "children":[slot("fg-source"),slot(if moved {"bg-menu"} else {"fg-menu"})]}})
}
fn compile(source: &Value) -> Rc<dyn Widget> {
    scene::compile(source, Instant::now() + Duration::from_secs(1))
        .unwrap()
        .view
}
struct Root {
    recipe: Rc<dyn Widget>,
    state: Box<dyn Any>,
    slots: Slots,
    bg: Entity<Owner>,
    fg: Entity<Owner>,
    bg_text: gpui_kit::base::TextSelectionHandle,
    fg_text: gpui_kit::base::TextSelectionHandle,
    carrier: Entity<Carrier>,
    late: Option<Entity<Late>>,
}
impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut result = div().size_full().child(TextSelectionLayer).child(
            self.recipe
                .render(self.state.as_ref(), &self.slots, window, cx)
                .unwrap(),
        );
        if let Some(late) = &self.late {
            let state = late.clone();
            let before = late.read(cx).before.clone();
            let after = late.read(cx).after.clone();
            let group = self.bg.read(cx).group.clone().unwrap();
            result = result
                .child(canvas(
                    |_, _, _| (),
                    move |_, _, window, cx| {
                        state.update(cx, |state, cx| {
                            if state.armed && state.scope.is_none() {
                                state.scope = Some(window.input_scope(&state.focus, cx));
                            }
                        });
                    },
                ))
                .child(deferred(CurrentGroup {
                    group,
                    child: canvas(
                        move |_, _, _| before.set(before.get() + 1),
                        move |_, _, _, _| after.set(after.get() + 1),
                    )
                    .into_any_element(),
                }));
        }
        result
    }
}

fn fixture(cx: &mut TestAppContext, foreground: &str) -> (Entity<Root>, AnyWindowHandle) {
    fixture_with_ax(cx, foreground, true)
}
fn fixture_with_ax(
    cx: &mut TestAppContext,
    foreground: &str,
    accessible: bool,
) -> (Entity<Root>, AnyWindowHandle) {
    cx.update(gpui_kit::init);
    let (root, visual) = cx.add_window_view(|window, cx| {
        let source = |value: &str| SourceTextProps {
            source: value.into(),
            appearance: text(),
            selection: Color(0x4466ff55),
            disabled: false,
        };
        let bg_source = crate::source::test_source(&source(BACKGROUND), input(), window, cx);
        let fg_source = crate::source::test_source(&source(foreground), input(), window, cx);
        let bg_text = bg_source.handle.clone();
        let fg_text = fg_source.handle.clone();
        let bg_recipe = Rc::new(Recipe(props("Background")));
        let bg_state = bg_recipe.create(input(), window, cx);
        let fg_recipe = Rc::new(Recipe(props("Foreground")));
        let fg_state = fg_recipe.create(input(), window, cx);
        let bg = bg_state.downcast_ref::<Entity<Owner>>().unwrap().clone();
        let fg = fg_state.downcast_ref::<Entity<Owner>>().unwrap().clone();
        let carrier = cx.new(|_| Carrier {
            widget: bg_recipe,
            state: bg_state,
            renders: 0,
            paints: Default::default(),
        });
        let bg_view = carrier.clone();
        let slots = Slots::new(
            vec![
                "bg-source".into(),
                "fg-source".into(),
                "bg-menu".into(),
                "fg-menu".into(),
            ],
            Input(Rc::new(|_, _, _, _| Ok(Default::default()))),
            move |key, window, cx| {
                let empty = Slots::new(
                    vec![],
                    Input(Rc::new(|_, _, _, _| Ok(Default::default()))),
                    |_, _, _| unreachable!(),
                );
                match key {
                    "bg-source" => Ok(div()
                        .id("background-text-group")
                        .w_full()
                        .role(Role::Group)
                        .aria_label("Background text")
                        .child(bg_source.widget.render(
                            bg_source.state.as_ref(),
                            &empty,
                            window,
                            cx,
                        )?)
                        .into_any_element()),
                    "fg-source" => Ok(div()
                        .id("foreground-text-group")
                        .w_full()
                        .role(Role::Group)
                        .aria_label("Foreground text")
                        .child(fg_source.widget.render(
                            fg_source.state.as_ref(),
                            &empty,
                            window,
                            cx,
                        )?)
                        .into_any_element()),
                    "bg-menu" => Ok(bg_view
                        .clone()
                        .cached(div().size_0().style().clone())
                        .into_any_element()),
                    "fg-menu" => fg_recipe.render(fg_state.as_ref(), &empty, window, cx),
                    _ => unreachable!(),
                }
            },
        );
        let recipe = compile(&node(false, false));
        let mut state = recipe.create(input(), window, cx);
        recipe.presentation(state.as_mut(), true, window, cx);
        Root {
            recipe,
            state,
            slots,
            bg,
            fg,
            bg_text,
            fg_text,
            carrier,
            late: None,
        }
    });
    let window = visual.update(|window, _| window.window_handle());
    if accessible {
        cx.activate_accessibility(window);
    }
    cx.update_window(window, |_, window, _| window.refresh())
        .unwrap();
    draw(cx, window);
    (root, window)
}
fn draw(cx: &mut TestAppContext, window: AnyWindowHandle) {
    cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    cx.run_until_parked();
}
fn replace(
    cx: &mut TestAppContext,
    root: &Entity<Root>,
    window: AnyWindowHandle,
    open: bool,
    moved: bool,
) {
    cx.update_window(window, |_, window, cx| {
        root.update(cx, |root, cx| {
            let next = compile(&node(open, moved));
            next.commit(root.state.as_mut(), window, cx);
            root.recipe = next;
            cx.notify();
        })
    })
    .unwrap();
    draw(cx, window);
}
fn ax(
    cx: &mut TestAppContext,
    window: AnyWindowHandle,
    label: &str,
) -> Option<(accesskit::NodeId, accesskit::Rect)> {
    cx.accessibility_update(window)
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some(label))
        .map(|(id, node)| (*id, node.bounds().unwrap()))
}
fn action(cx: &mut TestAppContext, window: AnyWindowHandle, target: accesskit::NodeId) {
    cx.simulate_accessibility_action(
        window,
        accesskit::ActionRequest {
            action: AccessibleAction::Click,
            target_tree: accesskit::TreeId::ROOT,
            target_node: target,
            data: None,
        },
    );
    draw(cx, window);
}
fn click_action(cx: &mut TestAppContext, window: AnyWindowHandle, label: &str) {
    let target = ax(cx, window, label)
        .unwrap_or_else(|| {
            panic!(
                "missing {label}: {:?}",
                cx.accessibility_update(window).unwrap().nodes
            )
        })
        .0;
    action(cx, window, target);
}
fn press(cx: &mut TestAppContext, window: AnyWindowHandle, title: &str, x: f32) {
    let (_, rect) = ax(cx, window, title).unwrap_or_else(|| {
        panic!(
            "missing {title}: {:?}",
            cx.accessibility_update(window).unwrap().nodes
        )
    });
    let scale = cx
        .update_window(window, |_, window, _| window.scale_factor())
        .unwrap();
    let p = point(
        px(rect.x0 as f32 / scale + x),
        px(rect.y0 as f32 / scale + 12.),
    );
    let mut visual = VisualTestContext::from_window(window, cx);
    for phase in [TouchPhase::Started, TouchPhase::Ended] {
        visual.simulate_event(LongPressEvent {
            phase,
            start_position: p,
            position: p,
        });
    }
    draw(cx, window);
}
fn clipboard(cx: &mut TestAppContext) -> String {
    cx.update(|cx| cx.read_from_clipboard().unwrap().text().unwrap())
}
#[gpui::test]
fn original_foreground_menu_copy_and_select_all_preserve_background_selection(
    cx: &mut TestAppContext,
) {
    let (root, window) = fixture(cx, FOREGROUND);
    press(cx, window, "Background text", 20.);
    click_action(cx, window, "All Background");
    let old_copy = ax(cx, window, "Copy Background").unwrap().0;
    let bg = cx.update(|cx| root.read(cx).bg.read(cx).group.clone().unwrap());
    replace(cx, &root, window, true, false);
    assert!(ax(cx, window, "Copy Background").is_none());
    cx.update(|cx| cx.write_to_clipboard(ClipboardItem::new_string("sentinel".into())));
    action(cx, window, old_copy);
    assert_eq!(clipboard(cx), "sentinel");
    press(cx, window, "Foreground text", 20.);
    cx.update_window(window, |_, window, cx| {
        let fg = root.read(cx).fg.read(cx).group.as_ref().unwrap();
        assert!(!fg.same_owner(&bg));
        assert!(fg.is_current(window, cx));
    })
    .unwrap();
    click_action(cx, window, "All Foreground");
    let (_, copy) = ax(cx, window, "Copy Foreground").unwrap();
    let scale = cx
        .update_window(window, |_, window, _| window.scale_factor())
        .unwrap();
    VisualTestContext::from_window(window, cx).simulate_click(
        point(
            px(((copy.x0 + copy.x1) / 2.) as f32 / scale),
            px(((copy.y0 + copy.y1) / 2.) as f32 / scale),
        ),
        Modifiers::default(),
    );
    draw(cx, window);
    assert_eq!(clipboard(cx), FOREGROUND);
    cx.update_window(window, |_, window, cx| {
        assert_eq!(
            root.read(cx)
                .bg_text
                .clone()
                .selected_text_in_scope(window, cx)
                .as_deref(),
            Some(BACKGROUND)
        );
        assert!(!bg.is_current(window, cx));
    })
    .unwrap();
    replace(cx, &root, window, false, false);
    assert!(ax(cx, window, "Copy Foreground").is_none());
    click_action(cx, window, "Copy Background");
    assert_eq!(clipboard(cx), BACKGROUND);
}
#[gpui::test]
fn original_menu_copies_long_pressed_whitespace_without_trimming(cx: &mut TestAppContext) {
    for value in ["    ", "\u{3000}\u{3000}"] {
        let (root, window) = fixture(cx, value);
        replace(cx, &root, window, true, false);
        press(cx, window, "Foreground text", 2.);
        click_action(cx, window, "Copy Foreground");
        assert_eq!(clipboard(cx), value);
    }
}
#[gpui::test]
fn original_menu_reparent_binds_actual_foreground_group_after_paint(cx: &mut TestAppContext) {
    let (root, window) = fixture(cx, FOREGROUND);
    press(cx, window, "Background text", 20.);
    let old = cx.update(|cx| root.read(cx).bg.read(cx).group.clone().unwrap());
    replace(cx, &root, window, true, true);
    press(cx, window, "Foreground text", 20.);
    let current = cx.update(|cx| root.read(cx).bg.read(cx).group.clone().unwrap());
    assert!(!old.same_owner(&current));
    click_action(cx, window, "All Background");
    click_action(cx, window, "Copy Background");
    assert_eq!(clipboard(cx), FOREGROUND);
    replace(cx, &root, window, false, false);
    let restored = cx.update(|cx| root.read(cx).bg.read(cx).group.clone().unwrap());
    assert!(restored.same_owner(&old));
}
#[gpui::test]
fn original_menu_rejects_unpublished_revoked_barrier_and_disabled_owner(cx: &mut TestAppContext) {
    let (root, window) = fixture(cx, FOREGROUND);
    press(cx, window, "Background text", 20.);
    assert!(ax(cx, window, "Copy Background").is_some());
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(window, |_, window, cx| {
            let a = cx.focus_handle();
            let b = cx.focus_handle();
            let first = window.input_scope(&a, cx);
            let second = window.input_scope(&b, cx);
            drop(second);
            assert_eq!(window.active_input_group(), None);
            assert!(!window.input_group_is_admitted(None));
            drop(first);
        })
        .unwrap();
        draw(cx, window);
        assert!(ax(cx, window, "Copy Background").is_none());
    });
    // An ordinary changed native prop retires menu handlers and observation,
    // without clearing the original text's completed selection.
    cx.update(|cx| {
        root.read(cx).bg.clone().update(cx, |owner, cx| {
            owner.props.disabled = true;
            owner.presentation(true, cx);
        })
    });
    draw(cx, window);
    assert!(ax(cx, window, "Copy Background").is_none());
}

struct Late {
    focus: FocusHandle,
    scope: Option<InputScope>,
    armed: bool,
    before: Rc<std::cell::Cell<usize>>,
    after: Rc<std::cell::Cell<usize>>,
}
#[gpui::test]
fn original_deferred_menu_rechecks_group_after_prepaint_before_actual_paint(
    cx: &mut TestAppContext,
) {
    let (root, window) = fixture(cx, FOREGROUND);
    press(cx, window, "Background text", 20.);
    let before = Rc::new(std::cell::Cell::new(0));
    let after = Rc::new(std::cell::Cell::new(0));
    let late = cx.update(|cx| {
        cx.new(|cx| Late {
            focus: cx.focus_handle(),
            scope: None,
            armed: false,
            before: before.clone(),
            after: after.clone(),
        })
    });
    cx.update(|cx| {
        root.update(cx, |root, cx| {
            root.late = Some(late.clone());
            cx.notify();
        })
    });
    draw(cx, window);
    assert!(
        after.get() > 0,
        "the original admitted deferred surface paints normally"
    );
    before.set(0);
    after.set(0);
    cx.with_paused_automatic_drawing(|cx| {
        cx.update(|cx| {
            root.update(cx, |_, cx| {
                late.update(cx, |late, _| late.armed = true);
                cx.notify();
            })
        });
        draw(cx, window);
        assert!(
            before.get() > 0,
            "the menu was genuinely prepainted before the opening"
        );
        assert_eq!(
            after.get(),
            0,
            "an intervening original scope blocks actual deferred paint"
        );
    });
}

// A real optional cached native parent. It still renders the original widget
// and opaque state; it does not substitute a model, menu or selection adapter.
struct Carrier {
    widget: Rc<dyn Widget>,
    state: Box<dyn Any>,
    renders: usize,
    paints: Rc<std::cell::Cell<usize>>,
}
impl Render for Carrier {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.renders += 1;
        let paints = self.paints.clone();
        div()
            .size_0()
            .child(
                self.widget
                    .render(
                        self.state.as_ref(),
                        &Slots::new(
                            vec![],
                            Input(Rc::new(|_, _, _, _| Ok(Default::default()))),
                            |_, _, _| unreachable!(),
                        ),
                        window,
                        cx,
                    )
                    .unwrap(),
            )
            .child(canvas(
                |_, _, _| (),
                move |_, _, _, _| paints.set(paints.get() + 1),
            ))
    }
}
#[gpui::test]
fn original_cached_background_menu_is_kept_then_hidden_and_restored_by_dialog(
    cx: &mut TestAppContext,
) {
    let (root, window) = fixture_with_ax(cx, FOREGROUND, false);
    let p = point(px(20.), px(12.));
    let mut visual = VisualTestContext::from_window(window, cx);
    for phase in [TouchPhase::Started, TouchPhase::Ended] {
        visual.simulate_event(LongPressEvent {
            phase,
            start_position: p,
            position: p,
        });
    }
    draw(cx, window);
    let counts = |cx: &mut App| {
        let root = root.read(cx);
        (
            root.carrier.read(cx).renders,
            root.carrier.read(cx).paints.get(),
            root.bg.read(cx).renders,
            root.bg.read(cx).paints.get(),
        )
    };
    let before = cx.update(counts);
    assert!(before.3 > 0, "actual original background menu was painted");
    draw(cx, window);
    assert_eq!(
        cx.update(counts),
        before,
        "original parent and menu are genuinely kept with AX inactive"
    );
    replace(cx, &root, window, true, false);
    let opened = cx.update(counts);
    assert!(
        opened.0 > before.0 && opened.2 > before.2,
        "opening invalidates the original cached background"
    );
    assert_eq!(
        opened.3, before.3,
        "background menu produces no paint under the foreground"
    );
    replace(cx, &root, window, false, false);
    assert!(
        cx.update(counts).3 > opened.3,
        "closing restores the original menu from its own group"
    );
    cx.update_window(window, |_, window, cx| {
        assert_eq!(
            root.read(cx)
                .bg_text
                .clone()
                .selected_text_in_scope(window, cx)
                .as_deref(),
            Some("background")
        );
    })
    .unwrap();
}
#[gpui::test]
fn original_menu_copy_rechecks_native_owner_after_participant_callback(cx: &mut TestAppContext) {
    let (root, window) = fixture(cx, FOREGROUND);
    replace(cx, &root, window, true, false);
    press(cx, window, "Foreground text", 20.);
    let menu = cx.update(|cx| root.read(cx).fg.clone());
    let group = cx.update(|cx| menu.read(cx).group.clone().unwrap());
    cx.update(|cx| {
        root.read(cx).fg_text.clone().copy_with(
            move |cx| {
                menu.update(cx, |menu, cx| menu.presentation(false, cx));
                "native callback text".into()
            },
            cx,
        );
        cx.write_to_clipboard(ClipboardItem::new_string("sentinel".into()));
    });
    click_action(cx, window, "Copy Foreground");
    assert_eq!(clipboard(cx), "sentinel");
    assert!(
        cx.update(|cx| group.touch_selection(cx).unwrap().is_menu_open()),
        "retiring the caller must not close an original selection menu as a later effect"
    );
}

#[gpui::test]
fn original_menu_old_ax_handler_cannot_follow_same_owner_dialog_reopen(cx: &mut TestAppContext) {
    let (root, window) = fixture(cx, FOREGROUND);
    replace(cx, &root, window, true, false);
    press(cx, window, "Foreground text", 20.);
    let old = ax(cx, window, "Copy Foreground").unwrap().0;
    cx.update(|cx| cx.write_to_clipboard(ClipboardItem::new_string("sentinel".into())));
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(window, |_, window, cx| {
            root.update(cx, |root, cx| {
                for open in [false, true] {
                    let next = compile(&node(open, false));
                    next.commit(root.state.as_mut(), window, cx);
                    root.recipe = next;
                }
                cx.notify();
            })
        })
        .unwrap();
        cx.simulate_accessibility_action(
            window,
            accesskit::ActionRequest {
                action: AccessibleAction::Click,
                target_tree: accesskit::TreeId::ROOT,
                target_node: old,
                data: None,
            },
        );
        assert_eq!(
            clipboard(cx),
            "sentinel",
            "the original callback's scope was revoked before a fresh paint"
        );
        draw(cx, window);
    });
    press(cx, window, "Foreground text", 20.);
    click_action(cx, window, "All Foreground");
    click_action(cx, window, "Copy Foreground");
    assert_eq!(clipboard(cx), FOREGROUND);
}
#[gpui::test]
fn original_menu_is_bounded_in_small_viewport_without_losing_selection(cx: &mut TestAppContext) {
    let (_, window) = fixture(cx, FOREGROUND);
    press(cx, window, "Background text", 20.);
    cx.simulate_window_resize(window, gpui_kit::size(px(100.), px(90.)));
    draw(cx, window);
    let (_, menu) = ax(cx, window, "Background").unwrap();
    let scale = cx
        .update_window(window, |_, window, _| window.scale_factor())
        .unwrap() as f64;
    assert!(menu.x0 >= 0. && menu.y0 >= 0. && menu.x1 <= 100. * scale && menu.y1 <= 90. * scale);
    assert!(ax(cx, window, "Copy Background").is_some());
    click_action(cx, window, "Copy Background");
    assert_eq!(clipboard(cx), "background");
}
