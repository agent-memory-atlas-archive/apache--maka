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
use crossterm::event::{KeyEvent, MouseEvent};
use maka_ui::Surface;
use maka_ui::actions::ServiceCompletion;
#[allow(dead_code)]
#[path = "../../../ui/examples/counter/support.rs"]
mod support;
fn fixture(handler: &str) -> (tokio::runtime::Runtime, Input, Vec<Package>) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_time()
        .build()
        .unwrap();
    let (mut input, packages) = support::dual_fixture(handler);
    input.environment.presentation = Some(PresentationTarget::Tui);
    (runtime, input, packages)
}
fn draw(presenter: &mut Presenter, width: u16, height: u16) -> Buffer {
    presenter.sync_native().unwrap();
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    let scene = presenter.paint(area, &mut buffer).unwrap();
    presenter.present(scene);
    buffer
}
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}
#[test]
fn same_package_has_native_surfaces_and_distinct_layouts_and_paint_never_calls_guest() {
    let (runtime, input, packages) =
        fixture("return frame({count:state.count+1,revision:state.revision+1});");
    let mut desktop_input = input.clone();
    desktop_input.environment.presentation = Some(PresentationTarget::Desktop);
    let mut desktop = Application::<()>::new(
        &desktop_input,
        packages.clone(),
        runtime.handle().clone(),
        Default::default(),
        support::scene::registry(PresentationTarget::Desktop),
        Default::default(),
    )
    .unwrap();
    let mount = desktop.mount_launch(json!({})).unwrap();
    assert!(matches!(
        desktop.frame(&mount).unwrap().surface,
        Some(Surface::Window { width: 640, .. })
    ));
    let mut tui = Presenter::new(
        &input,
        packages,
        runtime.handle().clone(),
        registry(),
        json!({}),
    )
    .unwrap();
    assert!(matches!(
        tui.application.frame(&tui.root).unwrap().surface,
        Some(Surface::Terminal { .. })
    ));
    draw(&mut tui, 30, 12); // Establish mechanical focus after the first publication.
    let first = draw(&mut tui, 30, 12);
    let second = draw(&mut tui, 30, 12);
    assert_eq!(first, second);
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state,
        json!({"count":0,"revision":0})
    );
    tui.input(key(KeyCode::Enter)).unwrap();
    assert_eq!(tui.application.frame(&tui.root).unwrap().state["count"], 1);
    // Coordinates bind the last flushed scene revision. No repaint/re-resolution.
    assert!(tui.input(key(KeyCode::Enter)).is_err());
    draw(&mut tui, 30, 12);
    let hit = tui.scene[0].area;
    tui.input(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: hit.x,
        row: hit.y,
        modifiers: KeyModifiers::NONE,
    }))
    .unwrap();
    assert_eq!(tui.application.frame(&tui.root).unwrap().state["count"], 2);
    tui.surface_closed(Ok(())).unwrap();
    assert!(tui.can_shutdown());
    tui.shutdown().unwrap();
    desktop.close(&mount, Ok(())).unwrap();
    desktop.shutdown().unwrap();
}
#[test]
fn unflushed_geometry_is_not_interactive_and_failure_receipt_preserves_owner() {
    let (runtime, input, packages) =
        fixture("return frame({count:state.count+1,revision:state.revision+1});");
    let mut tui = Presenter::new(
        &input,
        packages,
        runtime.handle().clone(),
        registry(),
        json!({}),
    )
    .unwrap();
    let area = Rect::new(0, 0, 30, 12);
    let mut buffer = Buffer::empty(area);
    let _unflushed = tui.paint(area, &mut buffer).unwrap();
    tui.input(key(KeyCode::Enter)).unwrap();
    assert_eq!(tui.application.frame(&tui.root).unwrap().state["count"], 0);
    draw(&mut tui, 30, 12);
    assert!(
        tui.surface_closed(Err("native restore failed".into()))
            .is_err()
    );
    assert!(!tui.can_shutdown());
    assert!(tui.application.frame(&tui.root).is_ok());
    assert!(tui.scene.is_empty());
    tui.input(key(KeyCode::Enter)).unwrap();
    assert_eq!(tui.application.frame(&tui.root).unwrap().state["count"], 0);
    // A later real receipt may recover the same actual mount, never replace it.
    tui.surface_closed(Ok(())).unwrap();
    assert!(tui.can_shutdown());
    tui.shutdown().unwrap();
}
#[test]
fn unicode_geometry_clips_and_every_untrusted_text_sink_removes_controls() {
    let input = "中文 e\u{301} 👩‍💻\n\x1b]52;c;hostile\x07\u{009b}31m";
    let clean = sanitize(input);
    assert!(!clean.chars().any(|c| c.is_control() && c != '\n'));
    assert_eq!(sanitize("a\tb\r\n"), "a    b\n");
    let area = Rect::new(1, 1, 4, 3);
    let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 6));
    paint::text(input, area, &mut buffer, Default::default());
    for y in 0..6 {
        for x in 0..8 {
            let cell = &buffer[(x, y)];
            assert!(!cell.symbol().chars().any(char::is_control));
            if !area.contains((x, y).into()) {
                assert_eq!(cell.symbol(), " ");
            }
        }
    }
    assert_eq!(paint::text_size("中文", 3), (2, 2));
}

#[test]
fn accepted_worker_survives_surface_close_and_cleanup_failure_keeps_actual_capacity() {
    use maka_plugin_core::contract::{Contract, Interface, Method, MethodMode, Transport};
    use maka_ui::{
        NativeCapability, NativeExport,
        actions::{NativeService, ServiceFuture},
    };
    use std::{
        collections::BTreeMap,
        sync::Arc,
        time::{Duration, Instant},
    };
    struct Gate {
        permits: Arc<tokio::sync::Semaphore>,
        started: std::sync::mpsc::Sender<std::sync::Weak<()>>,
        cleanup_failed: bool,
    }
    impl NativeService for Gate {
        fn call(&self, _: maka_ui::actions::ServiceRequest) -> ServiceFuture {
            let permits = self.permits.clone();
            let started = self.started.clone();
            let failed = self.cleanup_failed;
            Box::pin(async move {
                let payload = Arc::new(());
                started.send(Arc::downgrade(&payload)).unwrap();
                let permit = permits.acquire().await.unwrap();
                permit.forget();
                drop(payload);
                ServiceCompletion {
                    result: Err("ordinary business error".into()),
                    cleanup: if failed {
                        Err("actual executor cleanup failure".into())
                    } else {
                        Ok(())
                    },
                }
            })
        }
    }
    for cleanup_failed in [false, true] {
        let contract = Contract {
            name: "test.Service".into(),
            version: 1,
            transport: Transport::Local,
            interface: Interface::Service {
                methods: BTreeMap::from([(
                    "run".into(),
                    Method {
                        // This fixture waits on a resource; it performs no business mutation.
                        mode: MethodMode::Query,
                        input: json!({"type":"object"}),
                        output: json!({}),
                        resources: BTreeMap::new(),
                    },
                )]),
            },
        };
        let (runtime, input, packages) = service_fixture(
            "const out=frame({count:state.count+1,revision:state.revision+1});if(event.kind==='raw')out.actions.push({key:'finished',binding:'service',member:0,method:'run',input:{},resources:{}});return out;",
            &contract,
        );
        let gate = Arc::new(tokio::sync::Semaphore::new(0));
        let (tx, rx) = std::sync::mpsc::channel();
        let service = Arc::new(Gate {
            permits: gate.clone(),
            started: tx,
            cleanup_failed,
        });
        let mut native = registry();
        native
            .register("test.service", move |_, _values| {
                Ok(BTreeMap::from([(
                    "Service".into(),
                    NativeExport {
                        contract: contract.clone(),
                        value: NativeCapability::Service(service.clone()),
                    },
                )]))
            })
            .unwrap();
        let mut tui = Presenter::new(
            &input,
            packages,
            runtime.handle().clone(),
            native,
            json!({}),
        )
        .unwrap();
        draw(&mut tui, 30, 12);
        tui.input(key(KeyCode::Enter)).unwrap();
        tui.pump().unwrap();
        let actual_payload = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(tui.application.accepted_action_count(), 1);
        tui.surface_closed(Ok(())).unwrap();
        assert!(tui.application.frame(&tui.root).is_err());
        assert!(!tui.can_shutdown());
        assert!(actual_payload.upgrade().is_some());
        gate.add_permits(1);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !tui.jobs.is_empty() && Instant::now() < deadline {
            let _ = tui.pump();
            std::thread::yield_now();
        }
        assert!(tui.jobs.is_empty());
        assert!(actual_payload.upgrade().is_none());
        if cleanup_failed {
            assert!(!tui.can_shutdown());
            assert_eq!(tui.application.accepted_action_count(), 1);
            let failed = tui.application.failed_action_ids();
            assert_eq!(failed.len(), 1);
            // Explicit actual-executor recovery receipt for this exact action.
            tui.application
                .complete_action_cleanup(failed[0], Ok(()))
                .unwrap();
        } else {
            assert_eq!(tui.application.accepted_action_count(), 0);
        }
        assert!(tui.can_shutdown());
        tui.shutdown().unwrap();
    }
}

#[test]
fn surface_variant_and_obsolete_window_wire_shape_are_rejected() {
    assert!(serde_json::from_value::<maka_ui::Frame>(json!({"revision":0,"state":{},"window":{"binding":"surface","width":640,"height":400,"title":"old"},"view":{"kind":"text","key":"a","text":"x","color":0}})).is_err());
    assert!(
        serde_json::from_value::<Surface>(
            json!({"kind":"terminal","binding":"surface","width":640})
        )
        .is_err()
    );
    let (runtime, input, mut packages) = fixture("");
    let original = packages.pop().unwrap();
    let app = Package::new(std::collections::BTreeMap::from([
        (
            maka_plugins::package::MANIFEST_FILE.into(),
            original
                .file(maka_plugins::package::MANIFEST_FILE)
                .unwrap()
                .to_vec(),
        ),
        (
            "index.mjs".into(),
            original.file("index.mjs").unwrap().to_vec(),
        ),
        (
            "terminal.mjs".into(),
            original.file("index.mjs").unwrap().to_vec(),
        ),
    ]))
    .unwrap();
    let mut input = input;
    input.instances.get_mut("app").unwrap().package = app.application_ref().into();
    packages.push(app);
    assert!(
        Presenter::new(
            &input,
            packages,
            runtime.handle().clone(),
            registry(),
            json!({})
        )
        .err()
        .unwrap()
        .to_string()
        .contains("surface variant mismatch")
    );
}

fn terminal_packages(mut input: Input, packages: Vec<Package>) -> (Input, Vec<Package>) {
    use maka_plugin_core::package::{EntryTarget, Implementation};
    use std::collections::BTreeMap;
    let mut adapted = Vec::new();
    for original in packages {
        let mut declaration = original.manifest().application.clone().unwrap();
        let mut entry = declaration.entries.remove(&EntryTarget::Desktop).unwrap();
        if let Implementation::Native { implementation } = &mut entry.implementation
            && implementation == "maka.ui.surface.desktop"
        {
            *implementation = "maka.ui.surface.tui".into();
            entry.exports.get_mut("Surface").unwrap().contract =
                maka_ui::surface_contract(PresentationTarget::Tui);
        }
        if let Some(surface) = entry.imports.get_mut("surface") {
            surface.contract = maka_ui::surface_contract(PresentationTarget::Tui);
        }
        if entry.scene.is_some() {
            entry.scene = Some(super::scene::language());
        }
        declaration.entries.insert(EntryTarget::Tui, entry);
        let mut files = BTreeMap::from([(
            maka_plugins::package::MANIFEST_FILE.into(),
            serde_json::to_vec(
                &json!({"schemaVersion":2,"id":original.manifest().id,"application":declaration}),
            )
            .unwrap(),
        )]);
        if let Some(source) = original.file("index.mjs") {
            let source=std::str::from_utf8(source).unwrap()
                .replace("{kind:'window',binding:'surface',title:'Composed product',width:640,height:400}","{kind:'terminal',binding:'surface'}")
                .replace("gap:8,padding:16","gap:1,padding:1")
                .replace("gap:8,padding:8","gap:1,padding:0")
                .replace("gap:16,padding:0","gap:2,padding:0");
            files.insert("index.mjs".into(), source.into_bytes());
        }
        let package = Package::new(files).unwrap();
        for node in input.instances.values_mut() {
            if node.package == original.application_ref().into() {
                node.package = package.application_ref().into();
            }
        }
        adapted.push(package);
    }
    input.environment.presentation = Some(PresentationTarget::Tui);
    (input, adapted)
}
#[test]
fn imported_native_recipe_child_state_and_focus_use_real_mount_lifetimes() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_time()
        .build()
        .unwrap();
    let (input, packages) = support::native_fixture();
    let (input, packages) = terminal_packages(input, packages);
    let mut tui = Presenter::new(
        &input,
        packages,
        runtime.handle().clone(),
        registry(),
        json!({}),
    )
    .unwrap();
    let buffer = draw(&mut tui, 80, 20);
    assert!(
        buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
            .contains("Native 0")
    );
    let right = tui
        .application
        .child_mount(&tui.root, "right")
        .unwrap()
        .clone();
    let old = tui
        .scene
        .iter()
        .find(|target| target.mount == right)
        .unwrap()
        .clone();
    tui.focus = Some(old.clone());
    tui.input(key(KeyCode::Enter)).unwrap();
    assert_eq!(tui.application.frame(&right).unwrap().state["count"], 21);
    draw(&mut tui, 80, 20);
    let toggle = tui
        .scene
        .iter()
        .find(|t| t.key == "toggle" && t.mount == tui.root)
        .unwrap()
        .clone();
    tui.application
        .dispatch(&toggle.mount, toggle.revision, &toggle.key, json!({}))
        .unwrap();
    tui.pump().unwrap();
    assert!(tui.focus.is_none());
    draw(&mut tui, 80, 20);
    let toggle = tui
        .scene
        .iter()
        .find(|t| t.key == "toggle" && t.mount == tui.root)
        .unwrap()
        .clone();
    tui.application
        .dispatch(&toggle.mount, toggle.revision, &toggle.key, json!({}))
        .unwrap();
    draw(&mut tui, 80, 20);
    let replacement = tui
        .application
        .child_mount(&tui.root, "right")
        .unwrap()
        .clone();
    assert_ne!(right, replacement);
    assert!(
        tui.application
            .dispatch(&old.mount, old.revision, &old.key, json!({}))
            .is_err()
    );
    assert_eq!(
        tui.application.frame(&replacement).unwrap().state["count"],
        20
    );
    tui.surface_closed(Ok(())).unwrap();
    assert!(tui.can_shutdown());
    tui.shutdown().unwrap();
}

#[test]
fn narrow_row_uses_the_same_remaining_width_for_measure_and_unicode_paint() {
    use std::collections::BTreeMap;
    let (runtime, mut input, mut packages) = fixture("");
    let original = packages.pop().unwrap();
    let tree = json!({"kind":"stack","key":"root","direction":"column","gap":0,"padding":0,"background":0,"children":[
        {"kind":"stack","key":"row","direction":"row","gap":0,"padding":0,"background":0,"children":[
            {"kind":"text","key":"first","text":"12345","color":16777215},
            {"kind":"text","key":"wrapped","text":"中文abcd","color":16777215}]},
        {"kind":"text","key":"after","text":"after","color":16777215}]});
    let source = format!(
        "export function activate(){{return {{App:{{contract:{},kind:'component',component:['app']}}}};}} export const app={{mount(){{return {{actions:[],events:[],frame:{{revision:0,state:{{}},surface:{{kind:'terminal',binding:'surface'}},view:{}}}}};}},update(){{throw Error('unexpected update');}},event(){{throw Error('unexpected event');}}}};",
        serde_json::to_string(
            &original.manifest().application.as_ref().unwrap().entries
                [&maka_plugin_core::package::EntryTarget::Tui]
                .exports["App"]
                .contract
        )
        .unwrap(),
        tree
    );
    let app = Package::new(BTreeMap::from([
        (
            maka_plugins::package::MANIFEST_FILE.into(),
            original
                .file(maka_plugins::package::MANIFEST_FILE)
                .unwrap()
                .to_vec(),
        ),
        (
            "index.mjs".into(),
            original.file("index.mjs").unwrap().to_vec(),
        ),
        ("terminal.mjs".into(), source.into_bytes()),
    ]))
    .unwrap();
    input.instances.get_mut("app").unwrap().package = app.application_ref().into();
    packages.push(app);
    let mut tui = Presenter::new(
        &input,
        packages,
        runtime.handle().clone(),
        registry(),
        json!({}),
    )
    .unwrap();
    let buffer = draw(&mut tui, 10, 5);
    assert_eq!(buffer[(5, 1)].symbol(), "b");
    assert_eq!(buffer[(6, 1)].symbol(), "c");
    assert_eq!(buffer[(7, 1)].symbol(), "d");
    assert_eq!(buffer[(0, 2)].symbol(), "a");
    assert_eq!(buffer[(4, 2)].symbol(), "r");
    tui.surface_closed(Ok(())).unwrap();
    tui.shutdown().unwrap();
}

fn service_fixture(
    handler: &str,
    contract: &maka_plugin_core::contract::Contract,
) -> (tokio::runtime::Runtime, Input, Vec<Package>) {
    use maka_plugin_core::{
        composition::Target,
        contract::{Availability, Cardinality, Export, Import},
        package::{Declaration, EntryDeclaration, EntryTarget, Implementation},
    };
    use std::collections::BTreeMap;
    let (runtime, mut input, mut packages) = fixture(handler);
    let service = support::package(
        "native.service",
        Declaration {
            entries: BTreeMap::from([(
                EntryTarget::Tui,
                EntryDeclaration {
                    scene: None,
                    implementation: Implementation::Native {
                        implementation: "test.service".into(),
                    },
                    configuration: json!({"type":"object"}),
                    imports: BTreeMap::new(),
                    exports: BTreeMap::from([(
                        "Service".into(),
                        Export {
                            contract: contract.clone(),
                            availability: Availability::Required,
                        },
                    )]),
                },
            )]),
            composition: None,
        },
        None,
    );
    input
        .instances
        .insert("service".into(), support::node(&service, 99));
    packages.push(service);
    let original = packages.remove(1);
    let mut declaration = original.manifest().application.clone().unwrap();
    declaration
        .entries
        .get_mut(&EntryTarget::Tui)
        .unwrap()
        .imports
        .insert(
            "service".into(),
            Import {
                contract: contract.clone(),
                cardinality: Cardinality::One,
            },
        );
    let app = Package::new(BTreeMap::from([
        (
            maka_plugins::package::MANIFEST_FILE.into(),
            serde_json::to_vec(
                &json!({"schemaVersion":2,"id":"dynamic.app","application":declaration}),
            )
            .unwrap(),
        ),
        (
            "index.mjs".into(),
            original.file("index.mjs").unwrap().to_vec(),
        ),
        (
            "terminal.mjs".into(),
            original.file("terminal.mjs").unwrap().to_vec(),
        ),
    ]))
    .unwrap();
    input.instances.get_mut("app").unwrap().package = app.application_ref().into();
    input.instances.get_mut("app").unwrap().bindings.insert(
        "service".into(),
        vec![Target::Instance {
            path: vec!["service".into()],
            export: "Service".into(),
        }],
    );
    packages.push(app);
    (runtime, input, packages)
}

#[path = "mutations.rs"]
mod mutations;

struct SlotRow {
    duplicate: bool,
}
impl Widget for SlotRow {
    fn measure(&self, slots: &Slots<'_>, width: u16) -> maka_ui::Result<(u16, u16)> {
        assert!(slots.measure("undeclared", width).is_err());
        let first = slots.measure("right", width / 2)?;
        let second = slots.measure("left", width / 2)?;
        // Repeated measurement is permitted and has no guest effects.
        assert_eq!(first, slots.measure("right", width / 2)?);
        Ok((
            first.0.saturating_add(second.0).min(width),
            first.1.max(second.1),
        ))
    }
    fn paint(&self, slots: &mut Slots<'_>, area: Rect, buffer: &mut Buffer) -> maka_ui::Result<()> {
        assert_eq!(slots.keys().collect::<Vec<_>>(), ["right", "left"]);
        let half = area.width / 2;
        slots.paint(
            "right",
            Rect::new(area.x, area.y, half, area.height),
            buffer,
        )?;
        slots.paint(
            if self.duplicate { "right" } else { "left" },
            Rect::new(area.x + half, area.y, area.width - half, area.height),
            buffer,
        )
    }
}
struct SlotRoot {
    duplicate: bool,
}
impl maka_ui::NativeComponent<dyn Widget> for SlotRoot {
    fn call(
        &self,
        call: maka_ui::ComponentCall<'_>,
        _values: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<maka_ui::ComponentOutput<maka_ui::NativeView<dyn Widget>>> {
        let revision = match call {
            maka_ui::ComponentCall::Mount { .. } => 0,
            maka_ui::ComponentCall::Update { state, .. }
            | maka_ui::ComponentCall::Event { state, .. } => {
                state["revision"].as_u64().unwrap() + 1
            }
        };
        let slots = [("right", 20), ("left", 10)]
            .into_iter()
            .map(|(key, initial)| maka_ui::ComponentSlot {
                key: key.into(),
                binding: "counter".into(),
                member: 0,
                on: Default::default(),
                props: json!({"label":format!("{key} {revision}"),"initial":initial}),
            })
            .collect();
        Ok(maka_ui::Frame {
            revision,
            state: json!({"revision":revision}),
            surface: Some(Surface::Terminal {
                binding: "surface".into(),
            }),
            view: maka_ui::NativeView::new(
                std::rc::Rc::new(SlotRow {
                    duplicate: self.duplicate,
                }) as std::rc::Rc<dyn Widget>,
                [("parent".into(), json!(true))].into(),
                slots,
            )?
            .with_hidden_slots(if revision % 2 == 1 {
                ["left".into()].into()
            } else {
                Default::default()
            })?,
        }
        .output(vec![]))
    }
}
fn native_slots_fixture(runtime: &tokio::runtime::Runtime, duplicate: bool) -> Presenter {
    use maka_plugin_core::package::{EntryTarget, Implementation};
    let (mut input, packages) =
        support::composed_fixture(support::CHILD_UPDATE, support::PARENT_EVENT);
    input.environment.presentation = Some(PresentationTarget::Tui);
    let mut converted = Vec::new();
    let mut root_contract = None;
    for old in packages {
        let mut declaration = old.manifest().application.clone().unwrap();
        let mut entry = declaration.entries.remove(&EntryTarget::Desktop).unwrap();
        let is_root = old.manifest().id == "dynamic.app";
        if is_root {
            entry.scene = None;
            entry.implementation = Implementation::Native {
                implementation: "test.terminal-slots".into(),
            };
            root_contract = Some(entry.exports["App"].contract.clone());
            entry.imports.get_mut("surface").unwrap().contract =
                maka_ui::surface_contract(PresentationTarget::Tui);
        }
        if old.manifest().id == "native.surface" {
            entry.implementation = Implementation::Native {
                implementation: "maka.ui.surface.tui".into(),
            };
            entry.exports.get_mut("Surface").unwrap().contract =
                maka_ui::surface_contract(PresentationTarget::Tui);
        }
        if entry.scene.is_some() {
            entry.scene = Some(super::scene::language());
        }
        declaration.entries.insert(EntryTarget::Tui, entry);
        let source = if is_root {
            None
        } else {
            old.file("index.mjs")
                .map(|s| std::str::from_utf8(s).unwrap().to_string())
        };
        let package = support::package(&old.manifest().id, declaration, source);
        for node in input.instances.values_mut() {
            if node.package == old.application_ref().into() {
                node.package = package.application_ref().into();
            }
        }
        converted.push(package);
    }
    let contract = root_contract.unwrap();
    let mut native = registry();
    native
        .register("test.terminal-slots", move |_, _values| {
            Ok(std::collections::BTreeMap::from([(
                "App".into(),
                maka_ui::NativeExport {
                    contract: contract.clone(),
                    value: maka_ui::NativeCapability::Component(std::rc::Rc::new(SlotRoot {
                        duplicate,
                    })),
                },
            )]))
        })
        .unwrap();
    Presenter::new(
        &input,
        converted,
        runtime.handle().clone(),
        native,
        json!({}),
    )
    .unwrap()
}
#[test]
fn native_terminal_slots_measure_and_paint_js_children_with_original_input_mounts() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_time()
        .build()
        .unwrap();
    let mut tui = native_slots_fixture(&runtime, false);
    let root = tui.root.clone();
    let left = tui.application.child_mount(&root, "left").unwrap().clone();
    let right = tui.application.child_mount(&root, "right").unwrap().clone();
    draw(&mut tui, 80, 40); // Publish initial keyboard focus.
    let first = draw(&mut tui, 80, 40);
    assert_eq!(first, draw(&mut tui, 80, 40));
    let target = tui
        .scene
        .iter()
        .find(|t| t.mount == left && t.key == "inc")
        .unwrap()
        .clone();
    tui.input(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: target.area.x,
        row: target.area.y,
        modifiers: KeyModifiers::NONE,
    }))
    .unwrap();
    assert_eq!(tui.application.frame(&left).unwrap().state["count"], 11);
    assert_eq!(tui.application.frame(&right).unwrap().state["count"], 20);
    tui.application
        .dispatch(&root, 0, "parent", Value::Null)
        .unwrap();
    draw(&mut tui, 80, 40);
    assert_eq!(tui.application.child_mount(&root, "left").unwrap(), &left);
    assert_eq!(tui.application.frame(&left).unwrap().state["count"], 11);
    assert_eq!(
        tui.application.frame(&left).unwrap().state["label"],
        "left 1"
    );
    tui.surface_closed(Ok(())).unwrap();
    tui.shutdown().unwrap();
}
#[test]
fn native_terminal_hidden_slots_override_custom_parent_measurement_and_paint() {
    let hidden = json!({"kind":"stack","key":"root","direction":"column","padding":0,"gap":0,"background":0,"visible":false,"children":(0..65).map(|n| json!({"kind":"button","key":format!("button-{n}"),"label":"Hidden"})).collect::<Vec<_>>()});
    assert!(
        super::scene::compile(
            &hidden,
            std::time::Instant::now() + std::time::Duration::from_secs(1)
        )
        .is_err(),
        "hidden controls still consume the original input quota"
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_time()
        .build()
        .unwrap();
    let mut tui = native_slots_fixture(&runtime, false);
    let root = tui.root.clone();
    let left = tui.application.child_mount(&root, "left").unwrap().clone();
    draw(&mut tui, 80, 40);
    let original = tui
        .scene
        .iter()
        .find(|target| target.mount == left && target.key == "inc")
        .unwrap()
        .clone();
    let owner = tui.resources.owners[&left.id()].identity.clone();
    tui.application
        .dispatch(&root, 0, "parent", Value::Null)
        .unwrap();
    let frame = tui.application.frame(&root).unwrap();
    assert_eq!(frame.view.slots().len(), 2);
    assert_eq!(
        frame
            .view
            .visible_slots()
            .map(|slot| slot.key.as_str())
            .collect::<Vec<_>>(),
        ["right"]
    );
    let right = tui.application.child_mount(&root, "right").unwrap();
    assert_eq!(
        tui.measure_mount(&root, 80).unwrap(),
        tui.measure_mount(right, 40).unwrap(),
        "custom parent measurement excludes hidden left slot"
    );
    let mut pending_buffer = Buffer::empty(Rect::new(0, 0, 80, 40));
    let pending = tui.paint(pending_buffer.area, &mut pending_buffer).unwrap();
    assert!(!pending.targets.iter().any(|target| target.mount == left));
    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(
        tui.resources.input(&original, &event).unwrap().is_some(),
        "unflushed projection does not change last presented facts"
    );
    tui.present(pending);
    assert!(tui.resources.input(&original, &event).unwrap().is_none());
    tui.application
        .dispatch(&root, 1, "parent", Value::Null)
        .unwrap();
    draw(&mut tui, 80, 40);
    assert!(
        tui.scene
            .iter()
            .any(|target| target.mount == left && target.key == "inc")
    );
    assert!(std::rc::Rc::ptr_eq(
        &owner,
        &tui.resources.owners[&left.id()].identity
    ));
    tui.surface_closed(Ok(())).unwrap();
    tui.shutdown().unwrap();
}
#[test]
fn duplicate_native_terminal_placement_rejects_pending_scene_before_input_publication() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_time()
        .build()
        .unwrap();
    let mut tui = native_slots_fixture(&runtime, true);
    let area = Rect::new(0, 0, 80, 40);
    let mut buffer = Buffer::empty(area);
    assert!(tui.paint(area, &mut buffer).is_err());
    assert!(tui.scene.is_empty());
    assert_eq!(tui.application.frame(&tui.root).unwrap().revision, 0);
    tui.surface_closed(Ok(())).unwrap();
    tui.shutdown().unwrap();
}

#[test]
fn raw_click_focus_is_a_default_after_successful_commit() {
    for (handler, accepts, focuses) in [
        (
            "const out=frame({count:state.count+1,revision:state.revision+1});out.decision={preventDefault:true};return out;",
            true,
            false,
        ),
        (
            "return frame({count:state.count+1,revision:state.revision+1});",
            true,
            true,
        ),
        ("throw new Error('reject input');", false, false),
    ] {
        let (runtime, input, packages) = fixture(handler);
        let mut tui = Presenter::new(
            &input,
            packages,
            runtime.handle().clone(),
            registry(),
            json!({}),
        )
        .unwrap();
        draw(&mut tui, 30, 12);
        let hit = tui.scene[0].area;
        tui.focus = None;
        let result = tui.input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: hit.x,
            row: hit.y,
            modifiers: KeyModifiers::NONE,
        }));
        assert_eq!(result.is_ok(), accepts);
        assert_eq!(tui.focus.is_some(), focuses);
        assert_eq!(
            tui.application.frame(&tui.root).unwrap().revision,
            u64::from(accepts)
        );
        tui.surface_closed(Ok(())).unwrap();
        tui.shutdown().unwrap();
    }
}

#[path = "subscriptions.rs"]
mod subscriptions;

#[path = "tests/input.rs"]
mod input;
#[path = "tests/textarea.rs"]
mod textarea;

#[test]
fn pure_native_paint_panics_drain_original_state_and_failed_drop_keeps_root() {
    use std::{
        any::Any,
        collections::BTreeMap,
        rc::Rc,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };
    struct State {
        drops: Arc<AtomicUsize>,
        failed: bool,
    }
    impl Drop for State {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
            assert!(!self.failed, "original native destruction failed");
        }
    }
    struct Panic {
        drops: Arc<AtomicUsize>,
        failed: bool,
    }
    impl Widget for Panic {
        fn create(&self) -> Box<dyn Any> {
            Box::new(State {
                drops: self.drops.clone(),
                failed: self.failed,
            })
        }
        fn measure(&self, _: &Slots<'_>, _: u16) -> maka_ui::Result<(u16, u16)> {
            panic!("pure measurement failure")
        }
        fn paint(&self, _: &mut Slots<'_>, _: Rect, _: &mut Buffer) -> maka_ui::Result<()> {
            panic!("pure painting failure")
        }
    }
    for failed in [false, true] {
        let (runtime, input, packages) = fixture("");
        let drops = Arc::new(AtomicUsize::new(0));
        let watched = drops.clone();
        let native = NativeRegistry::with_surface(PresentationTarget::Tui)
            .with_scene_compiler(scene::language(), move |_, _| {
                maka_ui::NativeView::<dyn Widget>::new(
                    Rc::new(Panic {
                        drops: watched.clone(),
                        failed,
                    }),
                    BTreeMap::new(),
                    vec![],
                )
            })
            .unwrap();
        let mut tui = Presenter::new(
            &input,
            packages,
            runtime.handle().clone(),
            native,
            json!({}),
        )
        .unwrap();
        assert!(tui.measure_mount(&tui.root, 30).is_err());
        let area = Rect::new(0, 0, 30, 12);
        assert!(tui.paint(area, &mut Buffer::empty(area)).is_err());
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        assert_eq!(tui.surface_closed(Ok(())).is_ok(), !failed);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(tui.can_shutdown(), !failed);
        if failed {
            assert!(tui.application.frame(&tui.root).is_ok());
            assert!(tui.shutdown().is_err());
        } else {
            tui.shutdown().unwrap();
        }
    }
}
