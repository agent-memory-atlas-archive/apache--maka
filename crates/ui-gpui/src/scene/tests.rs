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
use crate::{Input, LiveInput, Slots, Widget};
use gpui_kit::{
    AccessibleAction, AnyWindowHandle, AppContext, Context, Entity, InteractiveElement,
    IntoElement, Modifiers, ParentElement, Render, Role, Styled, TestAppContext, VisualTestContext,
    accesskit, point, px,
};
use std::{
    any::Any,
    cell::{Cell, RefCell},
};

#[gpui::test]
#[ignore = "opt-in ordinary deployment component build and native scene ABI"]
fn packaged_deployment_pages_render_all_locales_and_operation_states(cx: &mut TestAppContext) {
    let repository = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let build = std::process::Command::new("node")
        .arg("packages/agent-product/scripts/build.mjs")
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = std::process::Command::new("node")
        .args(["--input-type=module", "-e", r#"
        import {component} from './packages/agent-product/dist/packages/deployment/desktop.js';
        import {activate} from './packages/agent-product/dist/packages/appearance/desktop.js';
        const frames=[];
        const source={stateDomain:'a'.repeat(64),deployment:'00000000-0000-0000-0000-000000000080',revision:7};
        const instances=Array.from({length:8},(_,i)=>({
          instance:['00000000-0000-0000-0000-'+String(i+1).padStart(12,'0')],
          path:['workspace','optional-'+i],package:{id:'component.optional-'+i,revision:'sha256-'+'b'.repeat(64)},
          structural:i===2,disabled:i===1,parentDisabled:i===1
        }));
        for(const scheme of ['light','dark']) for(const locale of ['en','zh-CN','zh-TW']) {
          const props={theme:activate({scheme}).Appearance.value,locale};
          let result=component.mount(props,{});
          const save=()=>frames.push(result.frame.view);
          const press=key=>{result=component.event(result.frame.state,{kind:'raw',key,event:null},props,{});save();};
          save();
          result=component.event(result.frame.state,{kind:'action',key:result.actions[0].key,event:{status:'succeeded',value:{source,instances,next:instances[7].instance}}},props,{}); save();
          press('deployment-toggle-0');
          press('deployment-confirm');
          result=component.event(result.frame.state,{kind:'action',key:result.actions[0].key,event:{status:'failed',message:'transport'}},props,{});save();
          press('deployment-history');
          result=component.mount(props,{});
          result=component.event(result.frame.state,{kind:'action',key:result.actions[0].key,event:{status:'succeeded',value:{source,instances,next:null}}},props,{});
          press('deployment-import');
          const observation={owner:'00000000-0000-0000-0000-000000000099',revision:4,phase:'ready',file:{name:'插件.maka-plugin',bytes:123,digest:'sha256:'+'c'.repeat(64)}};
          result=component.event(result.frame.state,{kind:'component',child:'deployment-import-file',name:'changed',key:'deployment-import-changed',event:observation},props,{});save();
          press('deployment-import-install');
          press('deployment-import-confirm');
          const reference={source,namespace:['00000000-0000-0000-0000-000000000090'],export:'Management',method:'install',operation:'00000000-0000-0000-0000-000000000098',fingerprint:'sha256-'+'b'.repeat(64),contract:(await import('./packages/agent-product/dist/authoring/plugin-sdk/src/index.mjs')).contracts.deploymentManager,recovery:'journal_v1'};
          result=component.event(result.frame.state,{kind:'action',key:result.actions[0].key,event:{status:'operation',reply:{reference,receipt:{kind:'unknown'}}}},props,{});save();
          press('deployment-import-check');
          result=component.event(result.frame.state,{kind:'action',key:result.actions[0].key,event:{status:'succeeded',value:{reference,receipt:{kind:'recorded',outcome:{kind:'returned',value:{status:'committed',value:{source:{...source,revision:8},package:{id:'local.selected',revision:'sha256-'+'d'.repeat(64)}}}},cleanup:{kind:'confirmed'}}}}},props,{});save();
        }
        console.log(JSON.stringify(frames));
        "#])
        .current_dir(repository)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let frames: Vec<Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(frames.len(), 78);
    let (entity, window, _) = harness(cx);
    // This checks the packaged scene ABI and exact child placement. FilePicker
    // owns separate native interaction/custody tests; this harness supplies its
    // explicitly declared slot without pretending to exercise a system panel.
    cx.update_window(window, |_, _, cx| {
        entity.update(cx, |h, _| {
            h.slots = Slots::new(
                vec!["deployment-import-file".into()],
                h.slots.input.clone(),
                |key, _, _| {
                    if key != "deployment-import-file" {
                        return Err(maka_ui::Error::Invalid("unknown slot".into()));
                    }
                    Ok(gpui_kit::div().w_full().h(px(36.)).into_any_element())
                },
            );
        });
    })
    .unwrap();
    for frame in frames {
        replace(cx, window, &entity, frame);
        draw(cx, window);
    }
}

#[gpui::test]
#[ignore = "opt-in ordinary Studio root build and native Desktop scene ABI"]
fn packaged_studio_root_renders_locales_and_retained_routes(cx: &mut TestAppContext) {
    let repository = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let build = std::process::Command::new("node")
        .arg("packages/agent-product/scripts/build.mjs")
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = std::process::Command::new("node")
        .args(["--input-type=module", "-e", r#"
        import * as root from './packages/agent-product/dist/packages/studio/desktop.js';
        import {activate} from './packages/agent-product/dist/packages/appearance/desktop.js';
        const frames=[];
        for(const scheme of ['light','dark']) for(const locale of ['en','zh-CN','zh-TW']) {
            root.activate({locale});
            const values={appearance:[activate({scheme}).Appearance.value]};
            let result=root.component.mount({},values);
            frames.push(result.frame.view);
            for(const route of ['connections','models','deployment']) {
                result=root.component.event(result.frame.state,{kind:'raw',key:'route-'+route,event:null},{},values);
                frames.push(result.frame.view);
            }
        }
        console.log(JSON.stringify(frames));
        "#]).current_dir(repository).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let frames: Vec<Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(frames.len(), 24);
    let (entity, window, _) = harness(cx);
    // Explicit declared child placeholders: this proves only root scene ABI,
    // slot visibility and geometry, not business controls or OS/visual acceptance.
    cx.update_window(window, |_, _, cx| {
        entity.update(cx, |h, _| {
            h.slots = Slots::new(
                vec!["connections".into(), "models".into(), "deployment".into()],
                h.slots.input.clone(),
                |key, _, _| {
                    if !["connections", "models", "deployment"].contains(&key) {
                        return Err(maka_ui::Error::Invalid("unknown Studio slot".into()));
                    }
                    Ok(gpui_kit::div().w_full().h(px(80.)).into_any_element())
                },
            );
        })
    })
    .unwrap();
    for frame in frames {
        replace(cx, window, &entity, frame);
        draw(cx, window);
    }
}

struct Harness {
    recipe: Rc<dyn Widget>,
    state: Box<dyn Any>,
    slots: Slots,
}
impl Render for Harness {
    fn render(
        &mut self,
        window: &mut gpui_kit::Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.recipe
            .render(self.state.as_ref(), &self.slots, window, cx)
            .unwrap()
    }
}
impl Harness {
    fn replace(&mut self, source: Value, window: &mut gpui_kit::Window, cx: &mut Context<Self>) {
        let recipe = compile(&source, Instant::now() + std::time::Duration::from_secs(1))
            .unwrap()
            .view;
        recipe.commit(self.state.as_mut(), window, cx);
        self.recipe = recipe;
        cx.notify();
    }
    fn state(&self) -> &SceneState {
        self.state.downcast_ref().unwrap()
    }
}
fn source(disabled: bool, moved: bool, removed: bool) -> Value {
    let children = vec![
        json!({"kind":"button","key":"action","label":"Run","disabled":disabled,"style":{"width":{"unit":"pixels","value":100},"height":{"unit":"pixels","value":40},"background":0x416653ffu32}}),
        json!({"kind":"stack","key":"scroll","direction":"column","gap":0,"padding":0,"background":0xffffff,"scroll":"manual","style":{"width":{"unit":"pixels","value":100},"height":{"unit":"pixels","value":40},"shrink":0},"children":[{"kind":"text","key":"long","text":"Long document","color":0,"style":{"height":{"unit":"pixels","value":400},"shrink":0}}]}),
    ];
    json!({"kind":"stack","key":"root","direction":"row","gap":0,"padding":0,"background":0xffffff,"style":{"width":{"unit":"pixels","value":200},"height":{"unit":"pixels","value":200}},"children":[
        {"kind":"stack","key":"a","direction":"column","gap":0,"padding":0,"background":0xffffff,"children":if !moved && !removed { children.clone() } else { vec![] }},
        {"kind":"stack","key":"b","direction":"column","gap":0,"padding":0,"background":0xffffff,"children":if moved && !removed { children } else { vec![] }}
    ]})
}
fn harness(cx: &mut TestAppContext) -> (Entity<Harness>, AnyWindowHandle, Rc<Cell<usize>>) {
    let clicks = Rc::new(Cell::new(0));
    let seen = clicks.clone();
    let input = Input(Rc::new(move |_, _, _, _| {
        seen.set(seen.get() + 1);
        Ok(Default::default())
    }));
    let (entity, _) = cx.add_window_view(move |window, cx| {
        let recipe = compile(
            &source(false, false, false),
            Instant::now() + std::time::Duration::from_secs(1),
        )
        .unwrap()
        .view;
        let live = LiveInput::new(
            Rc::new(RefCell::new(Some(input.clone()))),
            crate::NativeBudget::default(),
            std::sync::Arc::new(maka_plugins::call::Resources::default()),
        );
        let mut state = recipe.create(live, window, cx);
        recipe.presentation(state.as_mut(), true, window, cx);
        Harness {
            state,
            recipe,
            slots: Slots::new(vec![], input, |_, _, _| {
                Err(maka_ui::Error::Invalid("unknown slot".into()))
            }),
        }
    });
    let window = cx.windows()[0];
    draw(cx, window);
    (entity, window, clicks)
}
fn draw(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    cx.update_window(handle, |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
}
fn replace(
    cx: &mut TestAppContext,
    window: AnyWindowHandle,
    entity: &Entity<Harness>,
    value: Value,
) {
    cx.update_window(window, |_, window, cx| {
        entity.update(cx, |h, cx| h.replace(value, window, cx))
    })
    .unwrap();
}
#[gpui::test]
#[ignore = "opt-in packaged navigation build and native Desktop scene ABI"]
fn packaged_session_navigation_renders_bounded_pages_and_keeps_controls_accessible(
    cx: &mut TestAppContext,
) {
    let repository = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let build = std::process::Command::new("node")
        .arg("packages/agent-product/scripts/build.mjs")
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = std::process::Command::new("node").args(["--input-type=module","-e",r#"
        import {component} from './packages/agent-product/dist/packages/navigation/desktop.js';
        import {activate} from './packages/agent-product/dist/packages/appearance/desktop.js';
        const frames=[];
        for(const scheme of ['light','dark']) for(const locale of ['en','zh-CN','zh-TW']) for(const status of ['ready','busy','failed']) {
          const props={theme:activate({scheme}).Appearance.value,locale,sessions:Array.from({length:32},(_,i)=>({id:'conversation-'+i,title:'Conversation '+i,revision:1,updatedAt:1,archived:false})),selected:'conversation-0',previous:true,more:true,busy:status==='busy',failed:status==='failed'};
          frames.push(component.mount(props,{}).frame.view);
        }
        console.log(JSON.stringify(frames));
    "#]).current_dir(repository).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let frames: Vec<Value> = serde_json::from_slice(&output.stdout).unwrap();
    let (entity, window, _) = harness(cx);
    cx.activate_accessibility(window);
    for frame in frames {
        let controls: Vec<String> = frame["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["key"] == "catalog-controls")
            .unwrap()["children"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| node["label"].as_str().unwrap().to_owned())
            .collect();
        replace(cx, window, &entity, frame);
        draw(cx, window);
        for label in controls {
            let bounds = ax_bounds(cx, window, &label);
            assert!(bounds.x1 > bounds.x0 && bounds.y1 > bounds.y0);
        }
        cx.update_window(window, |_, _, cx| {
            let state = entity.read(cx).state();
            assert_eq!(state.buttons.len(), 36);
            assert!(state.scrolls.contains_key("session-list"));
            for key in ["previous", "more", "refresh"] {
                assert!(state.buttons.contains_key(key));
            }
        })
        .unwrap();
    }
}

#[gpui::test]
fn measured_truncation_keeps_full_accessible_text_and_adjacent_controls(cx: &mut TestAppContext) {
    let (entity, window, clicks) = harness(cx);
    let title = "一个很长的对话标题 · A🙂e\u{301} ".repeat(20);
    let model = "非常长的模型名称 · A🙂e\u{301} ".repeat(20);
    let source = |width| {
        json!({
            "kind":"stack","key":"root","direction":"row","gap":0,"padding":0,
            "background":0xffffff,"style":{"width":{"unit":"pixels","value":width}},
            "children":[
                {"kind":"text","key":"title","text":title,"color":0,
                 "style":{"width":{"unit":"pixels","value":0},"grow":1,"minWidth":0,"truncate":true}},
                {"kind":"button","key":"action","label":model,
                 "style":{"width":{"unit":"pixels","value":100},"minWidth":0,"height":{"unit":"pixels","value":40},"shrink":0,"truncate":true}},
                {"kind":"button","key":"other","label":"Settings",
                 "style":{"width":{"unit":"pixels","value":60},"height":{"unit":"pixels","value":40},"shrink":0}}
            ]
        })
    };
    cx.activate_accessibility(window);
    let mut original = None;
    for width in [240., 180., 240.] {
        replace(cx, window, &entity, source(width));
        draw(cx, window);
        let update = cx.accessibility_update(window).unwrap();
        let title_node = &update
            .nodes
            .iter()
            .find(|(_, node)| node.value() == Some(title.as_str()))
            .expect("the complete title is readable through accessibility")
            .1;
        assert_eq!(title_node.role(), Role::Label);
        let title_bounds = title_node.bounds().unwrap();
        let model_bounds = ax_bounds(cx, window, &model);
        let settings_bounds = ax_bounds(cx, window, "Settings");
        let scale = (model_bounds.x1 - model_bounds.x0) / 100.;
        assert!(title_bounds.x1 <= model_bounds.x0);
        assert!(model_bounds.x1 <= settings_bounds.x0);
        assert!(settings_bounds.x1 <= width * scale);
        assert_eq!(model_bounds.y1 - model_bounds.y0, 40. * scale);
        cx.update_window(window, |_, window, cx| {
            let handle = entity.read(cx).state().buttons["action"].focus.clone();
            if let Some(original) = &original {
                assert_eq!(&handle, original);
                assert!(handle.is_focused(window));
            } else {
                handle.focus(window, cx);
                original = Some(handle);
            }
        })
        .unwrap();
    }
    let bounds = ax_bounds(cx, window, "Settings");
    let scale = (bounds.x1 - bounds.x0) / 60.;
    VisualTestContext::from_window(window, cx).simulate_click(
        point(
            px(((bounds.x0 + bounds.x1) / 2. / scale) as f32),
            px(((bounds.y0 + bounds.y1) / 2. / scale) as f32),
        ),
        Modifiers::default(),
    );
    assert_eq!(clicks.get(), 1, "the adjacent action remains reachable");
}

#[gpui::test]
fn scene_wrapping_reflows_controls_without_replacing_focus(cx: &mut TestAppContext) {
    let (entity, window, clicks) = harness(cx);
    let source = |width| {
        json!({
            "kind":"stack","key":"root","direction":"row","gap":0,"padding":0,
            "background":0xffffff,"style":{"width":{"unit":"pixels","value":width},"wrap":true},
            "children":[
                {"kind":"button","key":"action","label":"Run","style":{"width":{"unit":"pixels","value":100},"height":{"unit":"pixels","value":40},"shrink":0}},
                {"kind":"button","key":"other","label":"Other","style":{"width":{"unit":"pixels","value":100},"height":{"unit":"pixels","value":40},"shrink":0}}
            ]
        })
    };
    replace(cx, window, &entity, source(220));
    cx.activate_accessibility(window);
    draw(cx, window);
    let bounds = |cx: &mut TestAppContext| {
        let update = cx.accessibility_update(window).unwrap();
        let find = |label| {
            update
                .nodes
                .iter()
                .find(|(_, n)| n.label() == Some(label))
                .unwrap()
                .1
                .bounds()
                .unwrap()
        };
        (find("Run"), find("Other"))
    };
    let (first, other) = bounds(cx);
    assert_eq!(first.y0, other.y0);
    assert!(other.x0 >= first.x1);
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(10.)), Modifiers::default());
    let focus = cx.update(|cx| entity.read(cx).state().buttons["action"].focus.clone());
    replace(cx, window, &entity, source(120));
    draw(cx, window);
    let (first, other) = bounds(cx);
    assert!(
        other.y0 >= first.y1,
        "second control wraps inside the narrow row"
    );
    assert_eq!(other.x0, first.x0);
    assert_eq!(other.x1, first.x1);
    cx.update_window(window, |_, w, cx| {
        assert_eq!(entity.read(cx).state().buttons["action"].focus, focus);
        assert!(focus.is_focused(w));
    })
    .unwrap();
    // AccessKit bounds use device pixels; pointer simulation uses logical
    // window coordinates. Read the actually laid-out wrapped control.
    let scale = (first.x1 - first.x0) / 100.;
    VisualTestContext::from_window(window, cx).simulate_click(
        point(
            px((other.x0 / scale + 5.) as f32),
            px((other.y0 / scale + 5.) as f32),
        ),
        Modifiers::default(),
    );
    assert_eq!(
        clicks.get(),
        2,
        "the wrapped control remains physically clickable"
    );
}

#[gpui::test]
fn scene_focus_and_scroll_follow_local_key_across_layout_ancestors(cx: &mut TestAppContext) {
    let (entity, window, clicks) = harness(cx);
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(10.)), Modifiers::default());
    assert_eq!(clicks.get(), 1, "native button positive control");
    let focus = cx.update(|cx| entity.read(cx).state().buttons["action"].focus.clone());
    cx.update_window(window, |_, w, cx| {
        assert!(focus.is_focused(w));
        entity.read(cx).state().scrolls["scroll"].set_offset(point(px(0.), px(-20.)));
    })
    .unwrap();
    replace(cx, window, &entity, source(false, true, false));
    draw(cx, window);
    cx.update_window(window, |_, w, cx| {
        let state = entity.read(cx).state();
        assert_eq!(state.buttons["action"].focus, focus);
        assert!(focus.is_focused(w));
        assert_eq!(state.scrolls["scroll"].offset(), point(px(0.), px(-20.)));
    })
    .unwrap();
}
#[gpui::test]
fn scene_disabled_and_removed_nodes_reject_old_pointer_keyboard_and_ax_focus(
    cx: &mut TestAppContext,
) {
    let (entity, window, clicks) = harness(cx);
    cx.activate_accessibility(window);
    draw(cx, window);
    let target = cx
        .accessibility_update(window)
        .unwrap()
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::Button && n.label() == Some("Run"))
        .unwrap()
        .0;
    let action = |cx: &mut TestAppContext, kind| {
        cx.simulate_accessibility_action(
            window,
            accesskit::ActionRequest {
                action: kind,
                target_tree: accesskit::TreeId::ROOT,
                target_node: target,
                data: None,
            },
        )
    };
    action(cx, AccessibleAction::Click);
    assert_eq!(clicks.get(), 1);
    cx.with_paused_automatic_drawing(|cx| {
        let painted = cx.accessibility_update(window).unwrap();
        replace(cx, window, &entity, source(true, false, false));
        assert_eq!(
            cx.accessibility_update(window).unwrap().nodes,
            painted.nodes
        );
        VisualTestContext::from_window(window, cx)
            .simulate_click(point(px(10.), px(10.)), Modifiers::default());
        action(cx, AccessibleAction::Focus);
        action(cx, AccessibleAction::Click);
        cx.simulate_keystrokes(window, "enter space tab enter");
        assert_eq!(
            clicks.get(),
            1,
            "all old enabled listeners consult live disabled facts"
        );
        cx.update_window(window, |_, w, cx| assert!(w.focused(cx).is_none()))
            .unwrap();
    });
    draw(cx, window);
    let update = cx.accessibility_update(window).unwrap();
    let button = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::Button && n.label() == Some("Run"))
        .unwrap();
    assert!(button.1.is_disabled());
    replace(cx, window, &entity, source(false, false, false));
    draw(cx, window);
    let update = cx.accessibility_update(window).unwrap();
    let target = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::Button && n.label() == Some("Run"))
        .unwrap()
        .0;
    cx.simulate_accessibility_action(
        window,
        accesskit::ActionRequest {
            action: AccessibleAction::Click,
            target_tree: accesskit::TreeId::ROOT,
            target_node: target,
            data: None,
        },
    );
    assert_eq!(
        clicks.get(),
        2,
        "re-enabled native action restores admission"
    );
    cx.with_paused_automatic_drawing(|cx| {
        replace(cx, window, &entity, source(false, false, true));
        VisualTestContext::from_window(window, cx)
            .simulate_click(point(px(10.), px(10.)), Modifiers::default());
        cx.simulate_accessibility_action(
            window,
            accesskit::ActionRequest {
                action: AccessibleAction::Focus,
                target_tree: accesskit::TreeId::ROOT,
                target_node: target,
                data: None,
            },
        );
        assert_eq!(clicks.get(), 2);
        cx.update_window(window, |_, w, cx| assert!(w.focused(cx).is_none()))
            .unwrap();
    });
}
#[gpui::test]
fn scene_slot_render_errors_are_returned_before_publication(cx: &mut TestAppContext) {
    let (_, window, _) = harness(cx);
    let recipe=compile(&json!({"kind":"component","key":"missing","binding":"child","member":0,"props":{},"on":{}}),Instant::now()+std::time::Duration::from_secs(1)).unwrap().view;
    cx.update_window(window, |_, w, cx| {
        let input = Input(Rc::new(|_, _, _, _| Ok(Default::default())));
        let mut state = recipe.create(
            LiveInput::new(
                Rc::new(RefCell::new(Some(input.clone()))),
                crate::NativeBudget::default(),
                std::sync::Arc::new(maka_plugins::call::Resources::default()),
            ),
            w,
            cx,
        );
        recipe.presentation(state.as_mut(), true, w, cx);
        let slots = Slots::new(vec!["missing".into()], input, |_, _, _| {
            Err(maka_ui::Error::Invalid("detached exact child".into()))
        });
        let error = recipe
            .render(state.as_ref(), &slots, w, cx)
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("detached exact child"));
    })
    .unwrap();
}
#[test]
fn scene_compilation_rejects_unsupported_shapes_expansion_deadline_and_style() {
    let deadline = || Instant::now() + std::time::Duration::from_secs(1);
    assert!(compile(&source(false, false, false), deadline()).is_ok());
    assert!(compile(&source(false, false, false), Instant::now()).is_err());
    let mut bad = source(false, false, false);
    bad["style"]["width"] = json!({"unit":"pixels","value":-1});
    assert!(compile(&bad, deadline()).is_err());
    let mut bad = source(false, false, false);
    bad["children"][0]["children"][0]["kind"] = json!("execute_service");
    assert!(compile(&bad, deadline()).is_err());
    let mut deep = json!({"kind":"text","key":"text","text":"x","color":0});
    for i in 0..33 {
        deep = json!({"kind":"stack","key":format!("s{i}"),"direction":"column","gap":0,"padding":0,"background":0,"children":[deep]});
    }
    assert!(compile(&deep, deadline()).is_err());
}

#[gpui::test]
fn retained_scene_recipe_cannot_restore_disabled_tab_admission(cx: &mut TestAppContext) {
    let (entity, window, clicks) = harness(cx);
    let old = cx.update(|cx| entity.read(cx).recipe.clone());
    replace(cx, window, &entity, source(true, false, false));
    // Exercise the actual older recipe against the original, current owner.
    cx.update(|cx| {
        entity.update(cx, |h, cx| {
            h.recipe = old;
            cx.notify();
        })
    });
    draw(cx, window);
    cx.simulate_keystrokes(window, "tab enter space");
    cx.update_window(window, |_, w, cx| {
        let state = entity.read(cx).state();
        assert!(!state.buttons["action"].focus.tab_stop);
        assert!(w.focused(cx).is_none());
    })
    .unwrap();
    assert_eq!(clicks.get(), 0);
}
#[gpui::test]
fn native_scene_deactivation_blurs_only_its_focus_with_old_listeners_retained(
    cx: &mut TestAppContext,
) {
    let (entity, window, clicks) = harness(cx);
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(10.)), Modifiers::default());
    assert_eq!(clicks.get(), 1);
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(window, |_, w, cx| {
            entity.update(cx, |h, cx| {
                let state = h.state.downcast_mut::<SceneState>().unwrap();
                assert!(state.buttons["action"].focus.is_focused(w));
                state.input.0.borrow_mut().take();
                h.recipe.deactivate(h.state.as_mut(), w, cx);
                assert!(w.focused(cx).is_none());
            })
        })
        .unwrap();
        VisualTestContext::from_window(window, cx)
            .simulate_click(point(px(10.), px(10.)), Modifiers::default());
        cx.simulate_keystrokes(window, "tab enter space");
        assert_eq!(clicks.get(), 1);
        cx.update_window(window, |_, w, cx| assert!(w.focused(cx).is_none()))
            .unwrap();
    });
}
#[test]
fn desktop_styles_reject_source_overflow_and_native_nonfinite_values() {
    let hidden = json!({"kind":"stack","key":"root","direction":"column","padding":0,"gap":0,"background":0,"visible":false,"children":(0..65).map(|n| json!({"kind":"button","key":format!("button-{n}"),"label":"Hidden"})).collect::<Vec<_>>()});
    assert!(
        compile(&hidden, Instant::now() + std::time::Duration::from_secs(1)).is_err(),
        "hidden controls still count against the original input quota"
    );
    for value in [1e300, -1e300, 16000.0001] {
        let mut scene = source(false, false, false);
        scene["style"]["minWidth"] = json!(value);
        assert!(compile(&scene, Instant::now() + std::time::Duration::from_secs(1)).is_err());
    }
    for value in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        assert!(
            Style {
                min_width: Some(value),
                ..Style::default()
            }
            .validate()
            .is_err()
        );
    }
}

#[gpui::test]
fn retained_presentation_hides_scene_controls_before_paint_and_restores_original_focus_owner(
    cx: &mut TestAppContext,
) {
    let (entity, window, clicks) = harness(cx);
    cx.activate_accessibility(window);
    draw(cx, window);
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(10.)), Modifiers::default());
    let focus = cx.update(|cx| entity.read(cx).state().buttons["action"].focus.clone());
    let target = cx
        .accessibility_update(window)
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("Run"))
        .unwrap()
        .0;
    cx.with_paused_automatic_drawing(|cx| {
        let mut hidden = source(false, false, false);
        hidden["children"][0]["visible"] = json!(false);
        replace(cx, window, &entity, hidden);
        VisualTestContext::from_window(window, cx)
            .simulate_click(point(px(10.), px(10.)), Modifiers::default());
        for action in [AccessibleAction::Focus, AccessibleAction::Click] {
            cx.simulate_accessibility_action(
                window,
                accesskit::ActionRequest {
                    action,
                    target_tree: accesskit::TreeId::ROOT,
                    target_node: target,
                    data: None,
                },
            );
        }
        cx.simulate_keystrokes(window, "tab enter space");
        assert_eq!(clicks.get(), 1);
        cx.update_window(window, |_, window, cx| {
            let state = entity.read(cx).state();
            assert_eq!(state.buttons["action"].focus, focus);
            assert!(!state.buttons["action"].admitted.get());
            assert!(
                state.input.0.borrow().is_some(),
                "hiding does not retire the capability"
            );
            assert!(window.focused(cx).is_none());
        })
        .unwrap();
    });
    draw(cx, window);
    assert!(
        !cx.accessibility_update(window)
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.label() == Some("Run"))
    );
    replace(cx, window, &entity, source(false, false, false));
    draw(cx, window);
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(10.)), Modifiers::default());
    assert_eq!(clicks.get(), 2);
    cx.with_paused_automatic_drawing(|cx| {
        // The ancestor scope can hide this unchanged native recipe as well.
        cx.update_window(window, |_, window, cx| {
            entity.update(cx, |h, cx| {
                h.recipe.presentation(h.state.as_mut(), false, window, cx)
            })
        })
        .unwrap();
        VisualTestContext::from_window(window, cx)
            .simulate_click(point(px(10.), px(10.)), Modifiers::default());
        cx.simulate_keystrokes(window, "tab enter space");
        assert_eq!(clicks.get(), 2);
    });
    cx.update_window(window, |_, window, cx| {
        entity.update(cx, |h, cx| {
            h.recipe.presentation(h.state.as_mut(), true, window, cx)
        })
    })
    .unwrap();
    draw(cx, window);
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(10.)), Modifiers::default());
    assert_eq!(clicks.get(), 3);
    cx.update(|cx| assert_eq!(entity.read(cx).state().buttons["action"].focus, focus));
}

fn popup_source(open: bool) -> Value {
    json!({"kind":"stack","key":"root","direction":"column","gap":0,"padding":0,"background":0xffffff,"children":[
        {"kind":"stack","key":"header","direction":"row","gap":0,"padding":0,"background":0xffffff,"children":[
            {"kind":"popup","key":"overlay","label":"Models","open":open,"anchor":"top_left","offset":8,
             "style":{"width":{"unit":"pixels","value":360},"maxHeight":140,"background":0xffeeccffu32},
             "trigger":{"kind":"button","key":"toggle","label":"Choose","style":{"width":{"unit":"pixels","value":100},"height":{"unit":"pixels","value":40}}},
             "content":{"kind":"button","key":"choice","label":"Choice","style":{"width":{"unit":"pixels","value":100},"height":{"unit":"pixels","value":40},"shrink":0}}}
        ]},
        {"kind":"button","key":"covered","label":"Covered","style":{"width":{"unit":"pixels","value":100},"height":{"unit":"pixels","value":40}}},
        {"kind":"button","key":"composer","label":"Composer","style":{"width":{"unit":"pixels","value":100},"height":{"unit":"pixels","value":40}}}
    ]})
}
type PopupEvents = Rc<RefCell<Vec<(String, Value)>>>;
fn popup_harness(
    cx: &mut TestAppContext,
) -> (
    Entity<Harness>,
    AnyWindowHandle,
    PopupEvents,
    Rc<Cell<bool>>,
) {
    let events = Rc::new(RefCell::new(vec![]));
    let observed = events.clone();
    let accept = Rc::new(Cell::new(true));
    let accepted = accept.clone();
    let scene = Rc::new(RefCell::new(popup_source(false)));
    let source = scene.clone();
    let owner: Rc<RefCell<Option<gpui_kit::WeakEntity<Harness>>>> = Rc::new(RefCell::new(None));
    let route = owner.clone();
    let input = Input(Rc::new(move |key, value, window, cx| {
        observed.borrow_mut().push((key.to_owned(), value.clone()));
        if !accepted.get() {
            return Err(maka_ui::Error::Invalid("reject original batch".into()));
        }
        if key == "toggle" || key == "overlay" {
            let mut scene = source.borrow_mut();
            let open = &mut scene["children"][0]["children"][0]["open"];
            *open = if key == "toggle" {
                json!(!open.as_bool().unwrap())
            } else {
                json!(false)
            };
            let next = scene.clone();
            route
                .borrow()
                .as_ref()
                .unwrap()
                .update(cx, |h, cx| h.replace(next, window, cx))
                .unwrap();
        }
        Ok(maka_ui::InputDecision {
            prevent_default: value["kind"] == "dismiss" && value["reason"] == "escape",
            stop_propagation: value["kind"] == "dismiss" && value["reason"] == "escape",
        })
    }));
    let (entity, _) = cx.add_window_view(move |window, cx| {
        let recipe = compile(
            &scene.borrow(),
            Instant::now() + std::time::Duration::from_secs(1),
        )
        .unwrap()
        .view;
        let mut state = recipe.create(
            LiveInput::new(
                Rc::new(RefCell::new(Some(input.clone()))),
                crate::NativeBudget::default(),
                std::sync::Arc::new(maka_plugins::call::Resources::default()),
            ),
            window,
            cx,
        );
        recipe.presentation(state.as_mut(), true, window, cx);
        Harness {
            state,
            recipe,
            slots: Slots::new(vec![], input, |_, _, _| {
                Err(maka_ui::Error::Invalid("unknown slot".into()))
            }),
        }
    });
    *owner.borrow_mut() = Some(entity.downgrade());
    let window = cx.windows()[0];
    cx.activate_accessibility(window);
    draw(cx, window);
    (entity, window, events, accept)
}
fn ax_bounds(cx: &mut TestAppContext, window: AnyWindowHandle, label: &str) -> accesskit::Rect {
    cx.accessibility_update(window)
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some(label))
        .unwrap()
        .1
        .bounds()
        .unwrap()
}
#[gpui::test]
fn popup_overlay_keeps_composer_geometry_and_trigger_toggle_is_one_committed_input(
    cx: &mut TestAppContext,
) {
    let (entity, window, events, _) = popup_harness(cx);
    let closed = ax_bounds(cx, window, "Composer");
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(10.)), Modifiers::default());
    draw(cx, window);
    draw(cx, window);
    assert_eq!(
        ax_bounds(cx, window, "Composer"),
        closed,
        "floating content adds no header height"
    );
    assert!(cx.update(|cx| entity.read(cx).state().popups["overlay"].admitted()));
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(10.)), Modifiers::default());
    draw(cx, window);
    assert!(!cx.update(|cx| entity.read(cx).state().popups["overlay"].admitted()));
    assert_eq!(
        events
            .borrow()
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        ["toggle", "toggle"]
    );
}
#[gpui::test]
fn popup_occludes_covered_control_and_outside_click_keeps_its_original_action(
    cx: &mut TestAppContext,
) {
    let (_, window, events, _) = popup_harness(cx);
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(10.)), Modifiers::default());
    draw(cx, window);
    draw(cx, window);
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(60.)), Modifiers::default());
    assert_eq!(
        events
            .borrow()
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        ["toggle", "choice"]
    );
    // The composer begins at y=80 and lies outside the 40px-high floating body.
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(100.)), Modifiers::default());
    assert_eq!(
        events
            .borrow()
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        ["toggle", "choice", "overlay", "composer"]
    );
    assert_eq!(
        events.borrow()[2].1,
        json!({"kind":"dismiss","reason":"outside"})
    );
}
#[gpui::test]
fn popup_rejected_escape_keeps_committed_open_and_close_restores_original_focus(
    cx: &mut TestAppContext,
) {
    let (entity, window, events, accept) = popup_harness(cx);
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(10.)), Modifiers::default());
    draw(cx, window);
    draw(cx, window);
    let opening_focus = cx
        .update_window(window, |_, window, cx| window.focused(cx).unwrap())
        .unwrap();
    accept.set(false);
    cx.simulate_keystrokes(window, "escape");
    assert!(cx.update(|cx| entity.read(cx).state().popups["overlay"].admitted()));
    cx.update_window(window, |_, window, _| {
        assert!(opening_focus.is_focused(window))
    })
    .unwrap();
    accept.set(true);
    cx.simulate_keystrokes(window, "escape");
    cx.run_until_parked();
    assert!(!cx.update(|cx| entity.read(cx).state().popups["overlay"].admitted()));
    cx.update_window(window, |_, window, cx| {
        assert!(
            entity.read(cx).state().buttons["toggle"]
                .focus
                .is_focused(window)
        )
    })
    .unwrap();
    assert_eq!(
        events.borrow().last().unwrap().1,
        json!({"kind":"dismiss","reason":"escape"})
    );
}
#[gpui::test]
fn popup_page_replacement_restores_native_focus_and_escape_after_the_focused_button_hides(
    cx: &mut TestAppContext,
) {
    popup_page_replacement(cx, true);
}
#[gpui::test]
fn popup_page_replacement_before_the_pressed_button_paints_preserves_escape(
    cx: &mut TestAppContext,
) {
    popup_page_replacement(cx, false);
}
fn popup_page_replacement(cx: &mut TestAppContext, paint_pressed_button: bool) {
    let (entity, window, events, _) = popup_harness(cx);
    let mut source = popup_source(true);
    replace(cx, window, &entity, source.clone());
    draw(cx, window);
    draw(cx, window);
    let scope = cx
        .update_window(window, |_, window, cx| window.focused(cx).unwrap())
        .unwrap();
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(window, |_, window, cx| {
            let focus = entity.read(cx).state().buttons["choice"].focus.clone();
            focus.focus(window, cx);
        })
        .unwrap();
        if paint_pressed_button { draw(cx, window); }
        source["children"][0]["children"][0]["content"] = json!({
            "kind":"stack","key":"page","direction":"column","gap":0,"padding":0,"background":0xffffff,"children":[
                {"kind":"stack","key":"first-page","direction":"column","gap":0,"padding":0,"background":0xffffff,"visible":false,"children":[{"kind":"button","key":"choice","label":"Choice"}]},
                {"kind":"button","key":"next-page","label":"Next page"}
            ]
        });
        replace(cx, window, &entity, source);
    });
    draw(cx, window);
    cx.run_until_parked();
    assert!(
        cx.update_window(window, |_, window, _| scope.is_focused(window))
            .unwrap(),
        "the still-open original popup remains a keyboard scope"
    );
    cx.simulate_keystrokes(window, "escape");
    assert_eq!(
        events.borrow().last().unwrap().1,
        json!({"kind":"dismiss","reason":"escape"})
    );
    assert!(!cx.update(|cx| entity.read(cx).state().popups["overlay"].admitted()));
}
#[gpui::test]
fn popup_content_fits_a_viewport_smaller_than_its_declared_style(cx: &mut TestAppContext) {
    let (entity, window, events, _) = popup_harness(cx);
    cx.simulate_window_resize(window, gpui_kit::size(px(220.), px(110.)));
    draw(cx, window);
    let closed_composer = ax_bounds(cx, window, "Composer");
    let mut source = popup_source(true);
    source["children"][0]["children"][0]["content"] = json!({
        "kind":"stack","key":"long-content","direction":"column","gap":0,"padding":0,"background":0xffeecc,
        "style":{"height":{"unit":"pixels","value":600},"shrink":0},
        "children":[
            {"kind":"button","key":"first-choice","label":"First choice","style":{"height":{"unit":"pixels","value":40},"shrink":0}},
            {"kind":"stack","key":"spacer","direction":"column","gap":0,"padding":0,"background":0xffeecc,"children":[],"style":{"height":{"unit":"pixels","value":520},"shrink":0}},
            {"kind":"button","key":"last-choice","label":"Last choice","style":{"height":{"unit":"pixels","value":40},"shrink":0}}
        ]
    });
    replace(cx, window, &entity, source.clone());
    draw(cx, window);
    draw(cx, window);
    let body = ax_bounds(cx, window, "Models");
    let trigger = ax_bounds(cx, window, "Choose");
    let scale = (trigger.x1 - trigger.x0) / 100.;
    assert!(body.x0 >= 8. * scale && body.x1 <= 212. * scale, "{body:?}");
    assert!(body.y0 >= 8. * scale && body.y1 <= 102. * scale, "{body:?}");
    assert!(body.y1 - body.y0 <= 94. * scale);
    assert_eq!(ax_bounds(cx, window, "Composer"), closed_composer);
    source["children"][0]["children"][0]["style"]["padding"] =
        json!({"top":128,"right":128,"bottom":128,"left":128});
    source["children"][0]["children"][0]["style"]["borderWidth"] = json!(8);
    for explicit_small_size in [false, true] {
        if explicit_small_size {
            source["children"][0]["children"][0]["style"]["width"] =
                json!({"unit":"pixels","value":80});
            source["children"][0]["children"][0]["style"]["height"] =
                json!({"unit":"pixels","value":40});
        }
        replace(cx, window, &entity, source.clone());
        draw(cx, window);
        draw(cx, window);
        let body = ax_bounds(cx, window, "Models");
        assert!(
            body.x0 >= 8. * scale
                && body.x1 <= 212. * scale
                && body.y0 >= 8. * scale
                && body.y1 <= 102. * scale,
            "valid padding/border must also fit: {body:?}"
        );
        let scroll_point = point(
            px(((body.x0 + body.x1) / (2. * scale)) as f32),
            px(((body.y0 + body.y1) / (2. * scale)) as f32),
        );
        cx.update_window(window, |_, window, cx| {
            window.dispatch_event(
                gpui_kit::PlatformInput::ScrollWheel(gpui_kit::ScrollWheelEvent {
                    position: scroll_point,
                    delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), px(1000.))),
                    modifiers: Modifiers::default(),
                    touch_phase: gpui_kit::TouchPhase::Moved,
                }),
                cx,
            );
        })
        .unwrap();
        draw(cx, window);
        let first = ax_bounds(cx, window, "First choice");
        let visible_y = first.y1.min(body.y1 - 8. * scale);
        assert!(
            visible_y > first.y0,
            "original content viewport remains visible"
        );
        let click = point(
            px(((first.x0 + first.x1) / (2. * scale)) as f32),
            px(((first.y0 + visible_y) / (2. * scale)) as f32),
        );
        VisualTestContext::from_window(window, cx).simulate_click(click, Modifiers::default());
        assert_eq!(events.borrow().last().unwrap().0, "first-choice");
        cx.update_window(window, |_, window, cx| {
            window.dispatch_event(
                gpui_kit::PlatformInput::ScrollWheel(gpui_kit::ScrollWheelEvent {
                    position: click,
                    delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), px(-1000.))),
                    modifiers: Modifiers::default(),
                    touch_phase: gpui_kit::TouchPhase::Moved,
                }),
                cx,
            );
        })
        .unwrap();
        draw(cx, window);
        let last = ax_bounds(cx, window, "Last choice");
        let visible_y = last.y0.max(body.y0 + 8. * scale);
        assert!(
            last.y1.min(body.y1 - 8. * scale) > visible_y,
            "last item scrolls into its original viewport: {last:?}, {body:?}"
        );
        let click = point(
            px(((last.x0 + last.x1) / (2. * scale)) as f32),
            px(((visible_y + last.y1.min(body.y1 - 8. * scale)) / (2. * scale)) as f32),
        );
        VisualTestContext::from_window(window, cx).simulate_click(click, Modifiers::default());
        assert_eq!(events.borrow().last().unwrap().0, "last-choice");
    }
}
#[test]
fn popup_compiler_preserves_hidden_slots_and_bounds_all_branches_and_deferred_depth() {
    let mut source = popup_source(false);
    let content = &mut source["children"][0]["children"][0]["content"];
    *content = json!({"kind":"component","key":"native-editor","binding":"editor","member":0,"props":{},"on":{}});
    let compiled = compile(&source, Instant::now() + std::time::Duration::from_secs(1)).unwrap();
    assert_eq!(compiled.slots().len(), 1);
    assert_eq!(compiled.visible_slots().count(), 0);
    for level in 0..11 {
        source = json!({"kind":"popup","key":format!("popup-{level}"),"label":"Nested","open":true,"trigger":{"kind":"text","key":format!("trigger-{level}"),"text":"Open","color":0},"content":source});
    }
    assert!(
        compile(&source, Instant::now() + std::time::Duration::from_secs(1))
            .err()
            .unwrap()
            .to_string()
            .contains("render depth")
    );
    source["open"] = json!(false);
    assert!(
        compile(&source, Instant::now() + std::time::Duration::from_secs(1)).is_ok(),
        "closed content is retained without native deferred work"
    );
    source["trigger"]["style"] = json!({"maxWidth":1e300});
    assert!(
        compile(&source, Instant::now() + std::time::Duration::from_secs(1)).is_err(),
        "even hidden/trigger branches retain source validation"
    );
}

#[gpui::test]
fn popup_nested_pointer_scope_and_inner_close_restore_the_original_outer_group(
    cx: &mut TestAppContext,
) {
    let (entity, window, events, _) = popup_harness(cx);
    let mut source = popup_source(true);
    let outer = &mut source["children"][0]["children"][0];
    outer["style"]["width"] = json!({"unit":"pixels","value":140});
    outer["content"] = json!({"kind":"popup","key":"inner","label":"Inner models","open":true,"anchor":"top_left","offset":8,
        "style":{"width":{"unit":"pixels","value":180},"maxHeight":140,"background":0xccffffffu32},
        "trigger":{"kind":"button","key":"inner-trigger","label":"Nested","style":{"width":{"unit":"pixels","value":100},"height":{"unit":"pixels","value":40}}},
        "content":{"kind":"button","key":"inner-choice","label":"Inner choice","style":{"width":{"unit":"pixels","value":100},"height":{"unit":"pixels","value":40}}}});
    replace(cx, window, &entity, source.clone());
    draw(cx, window);
    draw(cx, window);
    draw(cx, window);
    let outer_focus = cx
        .update_window(window, |_, w, cx| {
            let inner = w.focused(cx).unwrap();
            assert!(inner.tab_stop);
            entity.read(cx).state().buttons["inner-trigger"]
                .focus
                .clone()
        })
        .unwrap();
    // A deferred child is outside its parent's physical body rectangle, but is
    // still inside that parent's original native input subtree.
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(110.)), Modifiers::default());
    assert_eq!(
        events
            .borrow()
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        ["inner-choice"]
    );
    cx.simulate_keystrokes(window, "escape");
    assert_eq!(events.borrow().last().unwrap().0, "inner");
    source["children"][0]["children"][0]["content"]["open"] = json!(false);
    replace(cx, window, &entity, source);
    cx.run_until_parked();
    // The inner opened in the same batch as the outer, before a first frame.
    // Its predecessor is the original outer group, an active native focus stop.
    cx.update_window(window, |_, w, cx| {
        let current = w.focused(cx).unwrap();
        assert_ne!(current, outer_focus);
        assert!(current.tab_stop);
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_keystrokes(window, "escape");
    assert_eq!(events.borrow().last().unwrap().0, "overlay");
    assert!(!cx.update(|cx| entity.read(cx).state().popups["overlay"].admitted()));
}

#[gpui::test]
fn popup_reopen_and_hidden_retirement_reject_old_native_callbacks_before_repaint(
    cx: &mut TestAppContext,
) {
    let (entity, window, events, _) = popup_harness(cx);
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(10.)), Modifiers::default());
    draw(cx, window);
    draw(cx, window);
    cx.with_paused_automatic_drawing(|cx| {
        replace(cx, window, &entity, popup_source(false));
        replace(cx, window, &entity, popup_source(true));
        // Key dispatch synchronously redraws dirty GPUI windows. Pointer
        // capture instead exercises the actual retained old opening listener.
        VisualTestContext::from_window(window, cx)
            .simulate_click(point(px(10.), px(100.)), Modifiers::default());
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|(key, _)| key == "overlay")
                .count(),
            0,
            "old opening occurrence cannot dismiss a reopened popup"
        );
        assert!(cx.update(|cx| entity.read(cx).state().popups["overlay"].admitted()));
    });
    draw(cx, window);
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(100.)), Modifiers::default());
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|(key, _)| key == "overlay")
            .count(),
        1,
        "current opening accepts its native outside click"
    );
    let admitted_events = events.borrow().len();
    replace(cx, window, &entity, popup_source(true));
    draw(cx, window);
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(window, |_, w, cx| {
            entity.update(cx, |h, cx| {
                h.recipe.presentation(h.state.as_mut(), false, w, cx);
            })
        })
        .unwrap();
        VisualTestContext::from_window(window, cx)
            .simulate_click(point(px(10.), px(60.)), Modifiers::default());
        cx.simulate_keystrokes(window, "escape");
        assert_eq!(
            events.borrow().len(),
            admitted_events,
            "hidden content and old outside listeners are inadmissible"
        );
        cx.update_window(window, |_, w, cx| {
            entity.update(cx, |h, cx| {
                h.recipe.deactivate(h.state.as_mut(), w, cx);
            })
        })
        .unwrap();
        VisualTestContext::from_window(window, cx)
            .simulate_click(point(px(10.), px(100.)), Modifiers::default());
        assert_eq!(
            events.borrow().len(),
            admitted_events,
            "retired scene rejects its retained old native listeners"
        );
    });
}

struct CachedPointerChild {
    a: gpui_kit::FocusHandle,
    b: gpui_kit::FocusHandle,
    builds: Rc<Cell<usize>>,
    scopes: Rc<RefCell<Vec<(bool, bool)>>>,
}
impl Render for CachedPointerChild {
    fn render(&mut self, _: &mut gpui_kit::Window, _: &mut Context<Self>) -> impl IntoElement {
        self.builds.set(self.builds.get() + 1);
        let (a, b, scopes) = (self.a.clone(), self.b.clone(), self.scopes.clone());
        gpui_kit::div()
            .id("opaque-child")
            .size_full()
            .occlude()
            .on_mouse_down(gpui_kit::MouseButton::Left, move |_, window, _| {
                scopes
                    .borrow_mut()
                    .push((a.contains_hovered(window), b.contains_hovered(window)));
            })
    }
}
struct PointerScopeRoot {
    a: gpui_kit::FocusHandle,
    b: gpui_kit::FocusHandle,
    second: bool,
    child: Entity<CachedPointerChild>,
}
impl Render for PointerScopeRoot {
    fn render(&mut self, _: &mut gpui_kit::Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui_kit::div()
            .id("scope-root")
            .track_focus(if self.second { &self.b } else { &self.a })
            .child(
                self.child
                    .clone()
                    .cached(gpui_kit::StyleRefinement::default().w(px(100.)).h(px(40.))),
            )
    }
}
#[gpui::test]
fn popup_pointer_scope_follows_a_cached_opaque_child_when_its_external_parent_changes(
    cx: &mut TestAppContext,
) {
    let builds = Rc::new(Cell::new(0));
    let scopes = Rc::new(RefCell::new(vec![]));
    let (root, _) = cx.add_window_view({
        let (builds, scopes) = (builds.clone(), scopes.clone());
        move |_, cx| {
            let a = cx.focus_handle();
            let b = cx.focus_handle();
            let child = cx.new(|_| CachedPointerChild {
                a: a.clone(),
                b: b.clone(),
                builds,
                scopes,
            });
            PointerScopeRoot {
                a,
                b,
                second: false,
                child,
            }
        }
    });
    let window = cx.windows()[0];
    cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(10.)), Modifiers::default());
    let initial_builds = builds.get();
    for _ in 0..3 {
        cx.update(|cx| {
            root.update(cx, |root, cx| {
                root.second = true;
                cx.notify();
            })
        });
        cx.update_window(window, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
    }
    assert_eq!(
        builds.get(),
        initial_builds,
        "the opaque child's real prepaint/paint was retained"
    );
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(10.)), Modifiers::default());
    assert_eq!(
        *scopes.borrow(),
        [(true, false), (false, true)],
        "input ancestry comes from remapped dispatch nodes, not a cached external FocusId"
    );
}

fn command(key: &str, input: &str, enabled: bool) -> Value {
    json!({"key":key,"modifiers":["control"],"input":input,"enabled":enabled})
}
#[gpui::test]
fn shortcuts_use_native_bubbling_and_nearest_live_admission(cx: &mut TestAppContext) {
    let (entity, window, clicks) = harness(cx);
    let mut scene = source(false, false, false);
    scene["shortcuts"] = json!([command("b", "outer", true), command("n", "new", true)]);
    scene["children"][0]["shortcuts"] = json!([command("b", "inner", false)]);
    replace(cx, window, &entity, scene.clone());
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        let focus = entity.read(cx).state().buttons["action"].focus.clone();
        window.focus(&focus, cx)
    })
    .unwrap();
    cx.simulate_keystrokes(window, "ctrl-b ctrl-n");
    assert_eq!(
        clicks.get(),
        1,
        "disabled inner shadows; another chord reaches outer"
    );
    scene["children"][0]["shortcuts"][0]["enabled"] = json!(true);
    replace(cx, window, &entity, scene.clone());
    draw(cx, window);
    cx.simulate_keystrokes(window, "ctrl-b");
    assert_eq!(clicks.get(), 2, "one nearest command, never both");
    scene["children"][0]["shortcuts"][0]["enabled"] = json!(false);
    cx.update_window(window, |_, window, cx| {
        entity.update(cx, |h, cx| h.replace(scene, window, cx));
        window.dispatch_event(
            gpui_kit::PlatformInput::KeyDown(gpui_kit::KeyDownEvent {
                keystroke: gpui_kit::Keystroke::parse("ctrl-b").unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        assert_eq!(
            clicks.get(),
            2,
            "unpainted current disable revokes original listener"
        );
        entity.update(cx, |h, cx| {
            h.recipe.presentation(h.state.as_mut(), false, window, cx)
        });
        window.dispatch_event(
            gpui_kit::PlatformInput::KeyDown(gpui_kit::KeyDownEvent {
                keystroke: gpui_kit::Keystroke::parse("ctrl-n").unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        assert_eq!(clicks.get(), 2, "hidden root cannot invoke old callbacks");
    })
    .unwrap();
}
#[gpui::test]
fn unfocused_scene_root_adopts_a_non_tab_keyboard_scope(cx: &mut TestAppContext) {
    let (entity, window, clicks) = harness(cx);
    let mut scene = source(false, false, true);
    scene["shortcuts"] = json!([command("n", "new", true)]);
    replace(cx, window, &entity, scene);
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        let state = entity.read(cx).state();
        let (_, focus) = state.keyboard_focus.as_ref().unwrap();
        assert!(focus.is_focused(window));
    })
    .unwrap();
    cx.simulate_keystrokes(window, "ctrl-n");
    assert_eq!(clicks.get(), 1);
}
#[gpui::test]
fn marked_native_editor_blocks_commands_and_first_escape_only_ends_composition(
    cx: &mut TestAppContext,
) {
    use gpui_kit::{
        EntityInputHandler,
        base::input::{InputBase, TextareaState},
    };
    cx.update(gpui_kit::init);
    let (entity, window, clicks) = harness(cx);
    let editor = cx
        .update_window(window, |_, window, cx| {
            let editor = cx.new(|cx| TextareaState::new(window, cx));
            let child = editor.clone();
            entity.update(cx, |h, _| {
                h.slots = Slots::new(
                    vec!["native-editor".into()],
                    h.slots.input.clone(),
                    move |_, _, cx| {
                        let focus = child.read(cx).presentation().focus_handle().clone();
                        Ok(InputBase::new("shortcut-test-editor")
                            .size_full()
                            .track_focus(&focus)
                            .child(child.clone())
                            .into_any_element())
                    },
                );
            });
            editor
        })
        .unwrap();
    let mut scene = source(false, false, true);
    scene["shortcuts"] = json!([command("n","new",true),{"key":"escape","modifiers":[],"input":"close","enabled":true}]);
    scene["children"] = json!([{"kind":"component","key":"native-editor","binding":"editor","member":0,"props":{},"on":{}}]);
    replace(cx, window, &entity, scene);
    cx.update_window(window, |_, window, cx| {
        window.focus(editor.read(cx).presentation().focus_handle(), cx);
        editor.update(cx, |state, cx| {
            state.replace_and_mark_text_in_range(None, "ni", None, window, cx)
        });
    })
    .unwrap();
    draw(cx, window);
    cx.update_window(window, |_, window, cx| assert!(window.has_marked_text(cx)))
        .unwrap();
    cx.simulate_keystrokes(window, "ctrl-n escape");
    assert_eq!(
        clicks.get(),
        0,
        "composition owns both command suppression and first Escape"
    );
    cx.update_window(window, |_, window, cx| assert!(!window.has_marked_text(cx)))
        .unwrap();
    cx.simulate_keystrokes(window, "escape ctrl-n");
    assert_eq!(
        clicks.get(),
        2,
        "following unmarked keys reach the original scope"
    );
}
