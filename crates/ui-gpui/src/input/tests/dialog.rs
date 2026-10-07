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

use super::super::InputRecipe;
use super::*;
use crate::{Slots, Widget};
use std::any::Any;

struct SceneDialog {
    recipe: Rc<dyn Widget>,
    scene: Box<dyn Any>,
    background: Box<dyn Any>,
    query: Box<dyn Any>,
    query_recipe: InputRecipe,
    slots: Slots,
}
fn source(open: bool) -> Value {
    json!({"kind":"dialog","key":"modal","label":"Search","open":open,
        "background":{"kind":"component","key":"background","binding":"input","member":0,"props":{},"on":{}},
        "content":{"kind":"component","key":"query","binding":"input","member":0,"props":{},"on":{}}})
}
impl SceneDialog {
    fn replace(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        let recipe = crate::scene::compile(
            &source(open),
            std::time::Instant::now() + std::time::Duration::from_secs(1),
        )
        .unwrap()
        .view;
        recipe.commit(self.scene.as_mut(), window, cx);
        self.recipe = recipe;
        self.query_recipe
            .presentation(self.query.as_mut(), open, window, cx);
        cx.notify();
    }
    fn background(&self) -> Entity<InputOwner> {
        self.background
            .downcast_ref::<Entity<InputOwner>>()
            .unwrap()
            .clone()
    }
    fn query(&self) -> Entity<InputOwner> {
        self.query
            .downcast_ref::<Entity<InputOwner>>()
            .unwrap()
            .clone()
    }
}
impl Render for SceneDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.recipe
            .render(self.scene.as_ref(), &self.slots, window, cx)
            .unwrap()
    }
}
fn fixture(
    cx: &mut TestAppContext,
) -> (
    Entity<SceneDialog>,
    AnyWindowHandle,
    Rc<RefCell<Vec<Value>>>,
) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    let input = Input(Rc::new(move |key, value, _, _| {
        if key == "modal" {
            seen.borrow_mut().push(value);
        }
        Ok(Default::default())
    }));
    let (entity, _) = cx.add_window_view(move |window, cx| {
        gpui_kit::init(cx);
        let live = LiveInput::new(
            Rc::new(RefCell::new(Some(input.clone()))),
            crate::NativeBudget::default(),
            std::sync::Arc::new(maka_plugins::call::Resources::default()),
        );
        let background_recipe = InputRecipe {
            props: props(),
            secret: false,
        };
        let mut query_props = props();
        query_props.auto_focus = true;
        query_props.accessibility_label = "Search query".into();
        let query_recipe = InputRecipe {
            props: query_props,
            secret: false,
        };
        let mut background = background_recipe.create(live.clone(), window, cx);
        let mut query = query_recipe.create(live.clone(), window, cx);
        background_recipe.presentation(background.as_mut(), true, window, cx);
        query_recipe.presentation(query.as_mut(), false, window, cx);
        let recipe = crate::scene::compile(
            &source(false),
            std::time::Instant::now() + std::time::Duration::from_secs(1),
        )
        .unwrap()
        .view;
        let mut scene = recipe.create(live, window, cx);
        recipe.presentation(scene.as_mut(), true, window, cx);
        let bg = background
            .downcast_ref::<Entity<InputOwner>>()
            .unwrap()
            .clone();
        let fg = query.downcast_ref::<Entity<InputOwner>>().unwrap().clone();
        SceneDialog {
            recipe,
            scene,
            background,
            query,
            query_recipe,
            slots: Slots::new(
                vec!["background".into(), "query".into()],
                input,
                move |key, _, _| {
                    Ok(div()
                        .w(px(320.))
                        .h(px(40.))
                        .child(match key {
                            "background" => bg.clone(),
                            "query" => fg.clone(),
                            _ => return Err(maka_ui::Error::Invalid("slot".into())),
                        })
                        .into_any_element())
                },
            ),
        }
    });
    let window = cx.windows()[0];
    draw(cx, window);
    (entity, window, events)
}
#[gpui::test]
fn controlled_dialog_query_first_input_and_ime_precede_controlled_dismiss(cx: &mut TestAppContext) {
    let (root, window, events) = fixture(cx);
    let (bg, fg) = cx.update(|cx| {
        let root = root.read(cx);
        (root.background(), root.query())
    });
    cx.update_window(window, |_, window, cx| {
        bg.read(cx)
            .state
            .clone()
            .update(cx, |s, cx| s.focus(window, cx))
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_input(window, "draft");
    draw(cx, window);
    let before = cx.update(|cx| bg.read(cx).observation_snapshot());
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(window, |_, window, cx| {
            root.update(cx, |root, cx| root.replace(true, window, cx))
        })
        .unwrap();
        cx.with_platform_input_handler(window, |handler| {
            assert!(handler.selected_text_range(true).is_none());
            handler.replace_text_in_range(None, "blocked before paint");
        })
        .unwrap();
    });
    cx.update(|cx| assert_eq!(bg.read(cx).observation_snapshot(), before));
    draw(cx, window);
    cx.simulate_input(window, "find");
    draw(cx, window);
    cx.update(|cx| {
        assert_eq!(fg.read(cx).state.read(cx).value().as_ref(), "find");
        assert_eq!(bg.read(cx).observation_snapshot(), before);
    });
    cx.update_window(window, |_, window, cx| {
        fg.update(cx, |fg, cx| {
            fg.replace_and_mark_text_in_range(None, "ni", None, window, cx)
        })
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_keystrokes(window, "escape");
    assert!(events.borrow().is_empty(), "IME consumes first Escape");
    cx.simulate_keystrokes(window, "escape");
    assert_eq!(
        *events.borrow(),
        vec![json!({"kind":"dismiss","reason":"escape"})]
    );
    cx.update_window(window, |_, window, cx| {
        root.update(cx, |root, cx| root.replace(false, window, cx))
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_input(window, "!");
    draw(cx, window);
    cx.update(|cx| assert_eq!(bg.read(cx).state.read(cx).value().as_ref(), "draft!"));
}
#[gpui::test]
fn controlled_dialog_close_and_reopen_keeps_original_query_owner(cx: &mut TestAppContext) {
    let (root, window, _) = fixture(cx);
    let fg = cx.update(|cx| root.read(cx).query());
    cx.update_window(window, |_, window, cx| {
        root.update(cx, |root, cx| root.replace(true, window, cx))
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_input(window, "retained");
    draw(cx, window);
    let original = cx.update(|cx| fg.read(cx).owner_id().to_owned());
    cx.update_window(window, |_, window, cx| {
        root.update(cx, |root, cx| {
            root.replace(false, window, cx);
            root.replace(true, window, cx);
        })
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_input(window, " query");
    draw(cx, window);
    cx.update(|cx| {
        assert_eq!(fg.read(cx).owner_id(), original);
        assert_eq!(
            fg.read(cx).state.read(cx).value().as_ref(),
            "retained query"
        );
    });
}

#[gpui::test]
fn controlled_dialog_close_and_hide_keeps_retained_original_input_unfocused(
    cx: &mut TestAppContext,
) {
    let (root, window, _) = fixture(cx);
    let bg = cx.update(|cx| root.read(cx).background());
    cx.update_window(window, |_, window, cx| {
        bg.read(cx)
            .state
            .clone()
            .update(cx, |s, cx| s.focus(window, cx))
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_input(window, "retained draft");
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        root.update(cx, |root, cx| root.replace(true, window, cx))
    })
    .unwrap();
    draw(cx, window);
    cx.with_paused_automatic_drawing(|cx|{
        cx.update_window(window,|_,window,cx|root.update(cx,|root,cx|{
            root.replace(false,window,cx);
            let mut hidden=source(false);
            hidden["background"]=json!({"kind":"stack","key":"hidden-background","direction":"column","gap":0,"padding":0,"background":0,"visible":false,"children":[hidden["background"].clone()]});
            let recipe=crate::scene::compile(&hidden,std::time::Instant::now()+std::time::Duration::from_secs(1)).unwrap().view;
            recipe.commit(root.scene.as_mut(),window,cx);root.recipe=recipe;
            InputRecipe{props:props(),secret:false}.presentation(root.background.as_mut(),false,window,cx);
        })).unwrap();
        cx.update_window(window,|_,window,cx|assert!(window.focused(cx).is_none())).unwrap();
        draw(cx,window);
        cx.update_window(window,|_,window,cx|assert!(window.focused(cx).is_none())).unwrap();
    });
    cx.update(|cx| {
        assert_eq!(
            bg.read(cx).state.read(cx).value().as_ref(),
            "retained draft"
        )
    });
}

fn close_with_background_policy(cx: &mut TestAppContext, disabled: bool, read_only: bool) {
    let (root, window, _) = fixture(cx);
    let bg = cx.update(|cx| root.read(cx).background());
    cx.update_window(window, |_, window, cx| {
        bg.read(cx)
            .state
            .clone()
            .update(cx, |s, cx| s.focus(window, cx))
    })
    .unwrap();
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        root.update(cx, |root, cx| root.replace(true, window, cx))
    })
    .unwrap();
    draw(cx, window);
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(window, |_, window, cx| {
            root.update(cx, |root, cx| {
                root.replace(false, window, cx);
                let mut next = props();
                next.disabled = disabled;
                next.read_only = read_only;
                InputRecipe {
                    props: next,
                    secret: false,
                }
                .commit(root.background.as_mut(), window, cx);
            })
        })
        .unwrap();
        draw(cx, window);
        cx.update_window(window, |_, window, cx| {
            let focused = bg
                .read(cx)
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window);
            assert_eq!(focused, !disabled);
            if disabled {
                assert!(window.focused(cx).is_none());
            }
        })
        .unwrap();
    });
}
#[gpui::test]
fn controlled_dialog_close_does_not_restore_disabled_original_input(cx: &mut TestAppContext) {
    close_with_background_policy(cx, true, false);
}
#[gpui::test]
fn controlled_dialog_close_restores_readonly_original_input(cx: &mut TestAppContext) {
    close_with_background_policy(cx, false, true);
}

fn reparent_retained_dialog(cx: &mut TestAppContext, foreground: bool) {
    let (root, window, _) = fixture(cx);
    let fg = cx.update(|cx| root.read(cx).query());
    cx.update_window(window, |_, window, cx| {
        root.update(cx, |root, cx| root.replace(true, window, cx))
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_input(window, "original");
    draw(cx, window);
    let text = json!({"kind":"text","key":"outer-text","text":"Outer","color":0});
    let recipe = if foreground {
        json!({"kind":"dialog","key":"outer","label":"Outer","open":true,"background":text,"content":source(true)})
    } else {
        json!({"kind":"dialog","key":"outer","label":"Outer","open":true,"background":source(true),"content":text})
    };
    cx.update_window(window, |_, window, cx| {
        root.update(cx, |root, cx| {
            let next = crate::scene::compile(
                &recipe,
                std::time::Instant::now() + std::time::Duration::from_secs(1),
            )
            .unwrap()
            .view;
            next.commit(root.scene.as_mut(), window, cx);
            root.recipe = next;
            cx.notify();
        })
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_input(window, "!");
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        let state = fg.read(cx).state.read(cx);
        assert_eq!(
            state.presentation().focus_handle().is_focused(window),
            foreground
        );
        assert_eq!(
            state.value().as_ref(),
            if foreground { "original!" } else { "original" }
        );
    })
    .unwrap();
}
#[gpui::test]
fn controlled_dialog_retained_foreground_reparent_keeps_original_query_focus(
    cx: &mut TestAppContext,
) {
    reparent_retained_dialog(cx, true);
}
#[gpui::test]
fn controlled_dialog_retained_background_reparent_cannot_reclaim_input(cx: &mut TestAppContext) {
    reparent_retained_dialog(cx, false);
}

#[gpui::test]
fn controlled_dialog_inner_close_restores_original_outer_query(cx: &mut TestAppContext) {
    let (root, window, _) = fixture(cx);
    let query = cx.update(|cx| root.read(cx).query());
    cx.update_window(window, |_, window, cx| {
        root.update(cx, |root, cx| root.replace(true, window, cx))
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_input(window, "query");
    draw(cx, window);
    let mut nested = source(true);
    nested["content"] = json!({"kind":"stack","key":"outer-body","direction":"column","gap":0,"padding":0,"background":0,
        "children":[nested["content"].clone(),{"kind":"dialog","key":"inner","label":"Inner","open":true,
            "background":{"kind":"text","key":"inner-background","text":"Background","color":0},
            "content":{"kind":"text","key":"inner-content","text":"Inner","color":0}}]});
    cx.update_window(window, |_, window, cx| {
        root.update(cx, |root, cx| {
            let next = crate::scene::compile(
                &nested,
                std::time::Instant::now() + std::time::Duration::from_secs(1),
            )
            .unwrap()
            .view;
            next.commit(root.scene.as_mut(), window, cx);
            root.recipe = next;
            cx.notify();
        })
    })
    .unwrap();
    draw(cx, window);
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(window, |_, window, cx| {
            root.update(cx, |root, cx| root.replace(true, window, cx))
        })
        .unwrap();
        draw(cx, window);
    });
    cx.simulate_input(window, "!");
    draw(cx, window);
    cx.update(|cx| assert_eq!(query.read(cx).state.read(cx).value().as_ref(), "query!"));
}

#[gpui::test]
fn controlled_dialog_retained_marked_query_has_platform_handler_in_the_completed_frame(
    cx: &mut TestAppContext,
) {
    let (root, window, _) = fixture(cx);
    cx.update_window(window, |_, window, cx| {
        root.update(cx, |root, cx| root.replace(true, window, cx))
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_input(window, "original");
    draw(cx, window);
    cx.with_paused_automatic_drawing(|cx| {
        cx.with_platform_input_handler(window, |handler| handler.replace_and_mark_text_in_range(None, "中", Some(1..1))).unwrap();
        let before = cx.with_platform_input_handler(window, |handler| (handler.text_for_range(0..9, &mut None), handler.marked_text_range(), handler.selected_text_range(true).map(|selection| (selection.range, selection.reversed)))).unwrap();
        assert!(before.1.is_some());
        let next = json!({"kind":"dialog","key":"outer","label":"Outer","open":true,
            "background":{"kind":"text","key":"outer-background","text":"Outer","color":0},"content":source(true)});
        cx.update_window(window, |_, window, cx| root.update(cx, |root, cx| {
            let next = crate::scene::compile(&next, std::time::Instant::now() + std::time::Duration::from_secs(1)).unwrap().view;
            next.commit(root.scene.as_mut(), window, cx); root.recipe = next; cx.notify();
        })).unwrap();
        draw(cx, window);
        let after = cx.with_platform_input_handler(window, |handler| (handler.text_for_range(0..9, &mut None), handler.marked_text_range(), handler.selected_text_range(true).map(|selection| (selection.range, selection.reversed))));
        assert_eq!(after, Some(before), "one completed paint owns the same original IME handler");
    });
}

#[gpui::test]
fn controlled_dialog_close_publishes_restored_background_handler_in_the_completed_frame(
    cx: &mut TestAppContext,
) {
    let (root, window, _) = fixture(cx);
    let background = cx.update(|cx| root.read(cx).background());
    cx.update_window(window, |_, window, cx| {
        background
            .read(cx)
            .state
            .clone()
            .update(cx, |state, cx| state.focus(window, cx))
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_input(window, "draft");
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        root.update(cx, |root, cx| root.replace(true, window, cx))
    })
    .unwrap();
    draw(cx, window);
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(window, |_, window, cx| {
            root.update(cx, |root, cx| root.replace(false, window, cx))
        })
        .unwrap();
        draw(cx, window);
        cx.with_platform_input_handler(window, |handler| handler.replace_text_in_range(None, "!"))
            .expect("restored handler belongs to this completed frame");
        cx.update(|cx| {
            assert_eq!(
                background.read(cx).state.read(cx).value().as_ref(),
                "draft!"
            )
        });
    });
}
