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
use gpui_kit::{
    AnyWindowHandle, Entity, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    PlatformInput,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
struct Scene {
    window: AnyWindowHandle,
    surface: Entity<Surface>,
    app: Rc<RefCell<DesktopApplication>>,
    resources: Rc<RefCell<resources::NativeResources>>,
    root: Mount,
}
impl Scene {
    fn new(cx: &mut TestAppContext, timer: &tokio::runtime::Runtime, mode: &str) -> Self {
        Self::from_packages(cx, timer, mode, support::canvas_fixture())
    }
    fn from_packages(
        cx: &mut TestAppContext,
        timer: &tokio::runtime::Runtime,
        mode: &str,
        (input, packages): (
            maka_plugin_core::composition::Input,
            Vec<maka_packages::Package>,
        ),
    ) -> Self {
        Self::with_registry(cx, timer, mode, (input, packages), widgets::registry())
    }
    fn with_registry(
        cx: &mut TestAppContext,
        timer: &tokio::runtime::Runtime,
        mode: &str,
        (input, packages): (
            maka_plugin_core::composition::Input,
            Vec<maka_packages::Package>,
        ),
        registry: maka_ui_runtime::NativeRegistry<dyn widgets::Widget>,
    ) -> Self {
        let mut application = Application::new(
            &input,
            packages,
            timer.handle().clone(),
            maka_ui_js::execution(Default::default()),
            registry,
            Default::default(),
        )
        .unwrap();
        let root = application.mount_launch(json!({"mode":mode})).unwrap();
        let app = Rc::new(RefCell::new(application));
        let (wake, receipts) = tokio::sync::mpsc::unbounded_channel();
        let resources = Rc::new(RefCell::new(resources::NativeResources::new(
            root.clone(),
            wake,
        )));
        let (window, surface) = cx.update(|cx| {
            gpui_kit::init(cx);
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(0.), px(0.)),
                        size(px(640.), px(420.)),
                    ))),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    window_owner::start(
                        window,
                        cx,
                        resources.clone(),
                        app.clone(),
                        receipts,
                        |outcome, _| outcome.unwrap(),
                    )
                    .unwrap()
                    .detach();
                    let surface = cx.new(|_| Surface {
                        application: app.clone(),
                        resources: resources.clone(),
                        mount: root.clone(),
                    });
                    surface.update(cx, |s, cx| s.sync_native(window, cx).unwrap());
                    surface
                },
            )
            .unwrap()
        });
        let scene = Self {
            window,
            surface,
            app,
            resources,
            root,
        };
        scene.draw(cx);
        scene
    }
    fn draw(&self, cx: &mut TestAppContext) {
        cx.update_window(self.window, |_, w, cx| {
            self.surface
                .update(cx, |s, cx| s.sync_native(w, cx).unwrap());
            w.refresh();
            w.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    fn state(&self) -> Value {
        self.app.borrow().frame(&self.root).unwrap().state.clone()
    }
    fn down(&self, cx: &mut TestAppContext, x: f32, y: f32) {
        cx.update_window(self.window, |_, w, cx| {
            w.dispatch_event(
                PlatformInput::MouseDown(MouseDownEvent {
                    button: MouseButton::Left,
                    position: point(px(x), px(y)),
                    modifiers: Modifiers::default(),
                    click_count: 1,
                    first_mouse: false,
                }),
                cx,
            );
        })
        .unwrap();
    }
    fn move_to(&self, cx: &mut TestAppContext, x: f32, y: f32) {
        cx.update_window(self.window, |_, w, cx| {
            w.dispatch_event(
                PlatformInput::MouseMove(MouseMoveEvent {
                    position: point(px(x), px(y)),
                    pressed_button: Some(MouseButton::Left),
                    modifiers: Modifiers::default(),
                }),
                cx,
            );
        })
        .unwrap();
    }
    fn up(&self, cx: &mut TestAppContext, x: f32, y: f32) {
        cx.update_window(self.window, |_, w, cx| {
            w.dispatch_event(
                PlatformInput::MouseUp(MouseUpEvent {
                    button: MouseButton::Left,
                    position: point(px(x), px(y)),
                    modifiers: Modifiers::default(),
                    click_count: 1,
                }),
                cx,
            );
        })
        .unwrap();
    }
    fn close(self, cx: &mut TestAppContext) {
        cx.update_window(self.window, |_, w, _| w.remove_window())
            .unwrap();
        cx.run_until_parked();
    }
}
fn timer() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_time()
        .build()
        .unwrap()
}
#[gpui::test]
fn ordinary_dynamic_canvas_rules_commit_real_native_drag_keyboard_and_accessibility(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::new(cx, &timer, "ok");
    let original = cx
        .update_window(scene.window, |_, w, cx| {
            scene.surface.update(cx, |s, cx| s.scene(w, cx))
        })
        .unwrap();
    // Capture uses the real presenter custody before any component updates.
    let original = original.unwrap();
    scene.down(cx, 60., 180.);
    assert_eq!(scene.state()["calls"], 1);
    cx.update_window(scene.window, |_, w, cx| assert!(w.focused(cx).is_some()))
        .unwrap();
    scene.draw(cx);
    scene.move_to(cx, 380., 200.);
    scene.draw(cx);
    assert_eq!(
        scene.state()["calls"],
        1,
        "native high frequency dragging executes no guest move rule"
    );
    scene.up(cx, 380., 200.);
    assert_eq!(scene.state()["calls"], 2);
    assert_eq!(scene.state()["x"], 360);
    assert_eq!(scene.state()["y"], 180);
    assert_eq!(scene.state()["placed"], true);
    cx.update_window(scene.window, |_, w, cx| {
        assert!(
            original.render(w, cx).is_err(),
            "retained old recipe cannot resolve new region state"
        )
    })
    .unwrap();
    scene.draw(cx);
    cx.simulate_keystrokes(scene.window, "left");
    assert_eq!(scene.state()["x"], 340);
    scene.draw(cx);
    scene.down(cx, 350., 200.);
    scene.draw(cx);
    scene.move_to(cx, 50., 200.);
    scene.up(cx, 50., 200.);
    assert_eq!(
        scene.state()["x"],
        340,
        "invalid drop leaves business geometry committed at its prior location"
    );
    scene.draw(cx);
    cx.activate_accessibility(scene.window);
    scene.draw(cx);
    let target = cx
        .accessibility_update(scene.window)
        .unwrap()
        .nodes
        .iter()
        .find(|(_, n)| {
            n.label() == Some("Refine the experience") && n.role() == gpui_kit::Role::Button
        })
        .unwrap()
        .0;
    cx.simulate_accessibility_action(
        scene.window,
        gpui_kit::accesskit::ActionRequest {
            action: gpui_kit::AccessibleAction::Click,
            target_tree: gpui_kit::accesskit::TreeId::ROOT,
            target_node: target,
            data: None,
        },
    );
    assert_eq!(scene.state()["last"]["kind"], "activate");
    drop(original);
    scene.close(cx);
}
#[gpui::test]
fn canvas_callback_can_reconcile_original_owner_and_revoke_pending_defaults(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    for mode in ["disable", "remove"] {
        let scene = Scene::new(cx, &timer, mode);
        scene.down(cx, 60., 180.);
        assert_eq!(
            scene.state()["calls"],
            1,
            "rule commits while original native callback is active"
        );
        cx.update_window(scene.window, |_, w, cx| {
            assert!(
                w.focused(cx).is_none(),
                "postcommit revocation prevents pending native focus"
            )
        })
        .unwrap();
        scene.move_to(cx, 380., 200.);
        scene.up(cx, 380., 200.);
        assert_eq!(
            scene.state()["calls"],
            1,
            "pending gesture was never admitted"
        );
        scene.close(cx);
    }
}
#[gpui::test]
fn canvas_failure_preserves_paint_and_only_retirement_closes_native_admission(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    for mode in ["invalid", "throw"] {
        let scene = Scene::new(cx, &timer, mode);
        let before = cx
            .update_window(scene.window, |_, w, _| {
                w.painted_quads()
                    .into_iter()
                    .filter(|q| q.bounds.size.width > gpui_kit::ScaledPixels(200.))
                    .map(|q| (q.bounds, q.background))
                    .collect::<Vec<_>>()
            })
            .unwrap();
        scene.down(cx, 60., 180.);
        assert_eq!(scene.state()["revision"], 0);
        assert_eq!(scene.state()["calls"], 0);
        let retiring = mode == "throw";
        assert_eq!(scene.app.borrow().is_retiring(&scene.root), retiring);
        assert_eq!(
            scene.resources.borrow().owners[&scene.root.id().to_string()]
                .input
                .0
                .borrow()
                .is_none(),
            retiring
        );
        cx.update_window(scene.window, |_, w, cx| assert!(w.focused(cx).is_none()))
            .unwrap();
        scene.draw(cx);
        let after = cx
            .update_window(scene.window, |_, w, _| {
                w.painted_quads()
                    .into_iter()
                    .filter(|q| q.bounds.size.width > gpui_kit::ScaledPixels(200.))
                    .map(|q| (q.bounds, q.background))
                    .collect::<Vec<_>>()
            })
            .unwrap();
        assert!(!before.is_empty());
        assert_eq!(
            after, before,
            "last committed commands remain actually painted after rejected rule"
        );
        assert_eq!(
            scene.resources.borrow().owners[&scene.root.id().to_string()]
                .input
                .0
                .borrow()
                .is_none(),
            retiring,
            "later synchronization cannot reactivate a retiring owner"
        );
        scene.move_to(cx, 380., 200.);
        scene.up(cx, 380., 200.);
        assert_eq!(scene.state()["revision"], 0);
        scene.close(cx);
    }
}

#[gpui::test]
fn canvas_tab_and_copy_defaults_obey_committed_plugin_veto(cx: &mut TestAppContext) {
    let timer = timer();
    for (mode, reading) in [
        ("tab-veto", false),
        ("tab-allow", false),
        ("tab-veto", true),
        ("tab-allow", true),
    ] {
        let (mut input, mut packages) = support::canvas_fixture();
        let index = packages
            .iter()
            .position(|p| p.manifest().id == "dynamic.canvas")
            .unwrap();
        let old = packages.remove(index);
        let mut source = std::str::from_utf8(old.file("index.mjs").unwrap()).unwrap()
            .replace("label: 'Review lane', tabIndex: -1", "label: 'Review lane', tabIndex: 0")
            .replace("const decision = {};", "const decision = {};if(event.kind==='key_down'&&((event.key==='tab'&&state.mode==='tab-veto')||event.key==='c'))decision.preventDefault=true;");
        let mut declaration = old.manifest().application.clone().unwrap();
        if reading {
            use maka_plugin_core::{
                contract::{Availability, Cardinality, Export, Import},
                package::{EntryTarget, Implementation},
            };
            let contract = maka_ui_runtime::reading::reading_contract();
            declaration
                .entries
                .get_mut(&EntryTarget::Desktop)
                .unwrap()
                .imports
                .insert(
                    "reading".into(),
                    Import {
                        contract: contract.clone(),
                        cardinality: Cardinality::One,
                    },
                );
            let mut native = packages[0].manifest().application.clone().unwrap();
            let entry = native.entries.get_mut(&EntryTarget::Desktop).unwrap();
            entry.implementation = Implementation::Native {
                implementation: "maka.ui.reading".into(),
            };
            entry.exports = BTreeMap::from([(
                "ReadingArea".into(),
                Export {
                    contract,
                    availability: Availability::Required,
                },
            )]);
            let native = support::package("native.reading", native, None);
            input
                .instances
                .insert("reading".into(), support::node(&native, 3));
            input.instances.get_mut("app").unwrap().bindings.insert(
                "reading".into(),
                vec![maka_plugin_core::composition::Target::Instance {
                    path: vec!["reading".into()],
                    export: "ReadingArea".into(),
                }],
            );
            packages.push(native);
            source.push_str(r#"
const contentFrame = frame;
frame = (state) => {
    const output = contentFrame(state);
    output.frame.view = {kind:'component',key:'reading',binding:'reading',member:0,on:{input:'reading-input',observed:'reading-observed'},props:{command:null,content:{kind:'stack',key:'reading-content',direction:'column',padding:0,gap:0,background:0,scroll:'manual',style:{width:{unit:'pixels',value:640},height:{unit:'pixels',value:400}},children:[output.frame.view]}}};
    return output;
};
const contentEvent = app.event;
app.event = (state, input) => {
    if (input.kind === 'component' && input.child === 'reading') {
        if(input.name === 'input') return contentEvent(state, input.event);
        return frame({...state,revision:state.revision+1});
    }
    return contentEvent(state,input);
};
"#);
        }
        let root = support::package("dynamic.canvas", declaration, Some(source));
        input.instances.get_mut("app").unwrap().package = root.application_ref().into();
        packages.push(root);
        let scene = Scene::from_packages(cx, &timer, mode, (input, packages));
        scene.down(cx, 60., 180.);
        scene.draw(cx);
        let original = cx
            .update_window(scene.window, |_, window, cx| window.focused(cx).unwrap())
            .unwrap();
        let before = scene.state()["calls"].as_u64().unwrap();
        cx.simulate_keystrokes(scene.window, "tab");
        assert_eq!(
            scene.state()["calls"],
            before + 1,
            "the actual plugin admits the key first"
        );
        assert_eq!(scene.state()["last"]["key"], "tab");
        assert_eq!(scene.state()["mode"], mode);
        cx.update_window(scene.window, |_, window, _| {
            assert_eq!(original.is_focused(window), mode == "tab-veto")
        })
        .unwrap();
        if mode == "tab-allow" {
            scene.draw(cx);
            cx.simulate_keystrokes(scene.window, "shift-tab");
            cx.update_window(scene.window, |_, window, _| {
                assert!(original.is_focused(window))
            })
            .unwrap();
        }
        scene.draw(cx);
        cx.update(|cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                "Original clipboard".into(),
            ))
        });
        let before = scene.state()["calls"].as_u64().unwrap();
        cx.simulate_keystrokes(scene.window, "cmd-c");
        assert_eq!(scene.state()["calls"], before + 1);
        assert_eq!(scene.state()["last"]["key"], "c");
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("Original clipboard")
        );
        scene.close(cx);
    }
}

mod image;
