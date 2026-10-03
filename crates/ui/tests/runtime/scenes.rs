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

use super::{support, timer};
use maka_plugin_core::composition::PresentationTarget;
use maka_ui::{Application, NativeRegistry, NativeView};
use serde_json::{Value, json};
use std::{cell::Cell, collections::BTreeMap, rc::Rc};
fn fixture(
    payload: usize,
) -> (
    maka_plugin_core::composition::Input,
    Vec<maka_plugins::package::Package>,
) {
    let (mut input, mut packages) =
        support::fixture("return frame({count:state.count+1,revision:state.revision+1});");
    let index = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let old = packages.remove(index);
    let mut declaration = old.manifest().application.clone().unwrap();
    declaration
        .entries
        .get_mut(&maka_plugin_core::package::EntryTarget::Desktop)
        .unwrap()
        .scene = Some(language());
    let mut source = std::str::from_utf8(old.file("index.mjs").unwrap())
        .unwrap()
        .to_owned();
    source.push_str(&format!("const previous=frame;frame=(state)=>{{const output=previous(state);output.frame.view={{language:'private.fixture',blob:'x'.repeat({payload})}};return output;}};"));
    let package = support::package("dynamic.app", declaration, Some(source));
    input.instances.get_mut("app").unwrap().package = package.application_ref().into();
    packages.push(package);
    (input, packages)
}
#[test]
fn opaque_guest_scenes_cannot_be_laundered_out_of_retained_budget() {
    let timer = timer();
    let (input, packages) = fixture(950_000);
    let native = NativeRegistry::with_surface(PresentationTarget::Desktop)
        .with_scene_compiler(language(), |source, _| {
            assert_eq!(source["language"], "private.fixture");
            // Deliberately tries to give the kernel a smaller declaration. The
            // original source must still be retained and charged by the kernel.
            NativeView::new(
                Rc::new(()),
                BTreeMap::from([("inc".into(), json!(true))]),
                vec![],
            )?
            .with_declaration(json!({"tiny":true}))
        })
        .unwrap();
    let mut app = Application::new(
        &input,
        packages,
        timer.handle().clone(),
        Default::default(),
        native,
        Default::default(),
    )
    .unwrap();
    let mut mounts = vec![];
    for _ in 0..35 {
        mounts.push(app.mount_launch(json!({})).unwrap());
    }
    for _ in 0..3 {
        let error = app.mount_launch(json!({})).err().unwrap().to_string();
        assert!(error.contains("UI state limit"), "{error}");
    }
    app.dispatch(&mounts[0], 0, "inc", Value::Null).unwrap();
    assert_eq!(app.frame(&mounts[0]).unwrap().state["count"], 1);
    app.close(&mounts.remove(0), Ok(())).unwrap();
    mounts.push(app.mount_launch(json!({})).unwrap());
    for mount in mounts {
        app.close(&mount, Ok(())).unwrap();
    }
    app.shutdown().unwrap();
}
#[test]
fn guest_output_admission_precedes_scene_compilation() {
    let timer = timer();
    let (input, packages) = fixture(1_100_000);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let native = NativeRegistry::with_surface(PresentationTarget::Desktop)
        .with_scene_compiler(language(), move |_, _| {
            observed.set(observed.get() + 1);
            NativeView::new(Rc::new(()), BTreeMap::new(), vec![])
        })
        .unwrap();
    let mut app = Application::new(
        &input,
        packages,
        timer.handle().clone(),
        Default::default(),
        native,
        Default::default(),
    )
    .unwrap();
    assert!(app.mount_launch(json!({})).is_err());
    assert_eq!(calls.get(), 0);
    app.shutdown().unwrap();
}
#[test]
fn missing_scene_compiler_is_rejected_before_native_activation() {
    let timer = timer();
    let (input, packages) = fixture(0);
    let native = NativeRegistry::<()>::with_surface(PresentationTarget::Desktop);
    let error = Application::new(
        &input,
        packages,
        timer.handle().clone(),
        Default::default(),
        native,
        Default::default(),
    )
    .err()
    .unwrap()
    .to_string();
    assert!(error.contains("scene compiler required"));
}
#[test]
fn scene_compile_failure_preserves_last_committed_frame_and_input_custody() {
    let timer = timer();
    let (input, packages) = fixture(0);
    let fail = Rc::new(Cell::new(false));
    let observed = fail.clone();
    let native = NativeRegistry::with_surface(PresentationTarget::Desktop)
        .with_scene_compiler(language(), move |_, _| {
            if observed.get() {
                return Err(maka_ui::Error::Invalid("scene compile failure".into()));
            }
            NativeView::new(
                Rc::new(()),
                BTreeMap::from([("inc".into(), json!(true))]),
                vec![],
            )
        })
        .unwrap();
    let mut app = Application::new(
        &input,
        packages,
        timer.handle().clone(),
        Default::default(),
        native,
        Default::default(),
    )
    .unwrap();
    let mount = app.mount_launch(json!({"count":4})).unwrap();
    fail.set(true);
    assert!(app.dispatch(&mount, 0, "inc", Value::Null).is_err());
    assert_eq!(app.frame(&mount).unwrap().revision, 0);
    assert_eq!(app.frame(&mount).unwrap().state["count"], 4);
    assert!(!app.is_retiring(&mount));
    fail.set(false);
    app.dispatch(&mount, 0, "inc", Value::Null).unwrap();
    assert_eq!(app.frame(&mount).unwrap().state["count"], 5);
    app.close(&mount, Ok(())).unwrap();
    app.shutdown().unwrap();
}

fn language() -> maka_plugin_core::package::SceneLanguage {
    maka_plugin_core::package::SceneLanguage {
        name: "private.fixture.scene".into(),
        version: 1,
    }
}

#[test]
fn selected_languages_are_checked_before_any_factory_or_guest_evaluates() {
    use maka_plugin_core::{
        contract::Availability,
        package::{Declaration, EntryTarget},
    };
    use maka_ui::{NativeCapability, NativeExport};
    let timer = timer();
    for (name, version, disabled) in [
        ("unknown.scene", 1, false),
        ("private.fixture.scene", 2, false),
        ("unknown.scene", 1, true),
    ] {
        let (mut input, mut packages) = fixture(0);
        let index = packages
            .iter()
            .position(|p| p.manifest().id == "dynamic.app")
            .unwrap();
        let old = packages.remove(index);
        let declaration = old.manifest().application.clone().unwrap();
        let mut other = declaration.entries[&EntryTarget::Desktop].clone();
        other.imports.clear();
        other.exports.get_mut("App").unwrap().availability = Availability::Optional;
        other.scene = Some(maka_plugin_core::package::SceneLanguage {
            name: name.into(),
            version,
        });
        let extra = support::package(
            "later.component",
            Declaration {
                entries: BTreeMap::from([(EntryTarget::Desktop, other)]),
                composition: None,
            },
            Some("throw new Error('later guest must not evaluate');".into()),
        );
        let mut node = support::node(&extra, 3);
        node.disabled = disabled;
        input.instances.insert("later".into(), node);
        packages.push(extra);
        let source = std::str::from_utf8(old.file("index.mjs").unwrap()).unwrap();
        let source = if disabled {
            source.to_owned()
        } else {
            format!("throw new Error('earlier guest must not evaluate');\n{source}")
        };
        let root = support::package("dynamic.app", declaration, Some(source));
        input.instances.get_mut("app").unwrap().package = root.application_ref().into();
        packages.push(root);
        let factory_calls = Rc::new(Cell::new(0));
        let observed = factory_calls.clone();
        let mut native = NativeRegistry::default()
            .with_scene_compiler(language(), |_, _| {
                NativeView::new(
                    Rc::new(()),
                    BTreeMap::from([("inc".into(), json!(true))]),
                    vec![],
                )
            })
            .unwrap();
        native
            .register("maka.ui.surface.desktop", move |_, _| {
                observed.set(observed.get() + 1);
                Ok(BTreeMap::from([(
                    "Surface".into(),
                    NativeExport {
                        contract: maka_ui::surface_contract(PresentationTarget::Desktop),
                        value: NativeCapability::Surface(PresentationTarget::Desktop),
                    },
                )]))
            })
            .unwrap();
        let result = Application::new(
            &input,
            packages,
            timer.handle().clone(),
            Default::default(),
            native,
            Default::default(),
        );
        if disabled {
            let mut app = result.unwrap();
            assert_eq!(factory_calls.get(), 1);
            let mount = app.mount_launch(json!({"count":7})).unwrap();
            app.dispatch(&mount, 0, "inc", json!(null)).unwrap();
            assert_eq!(app.frame(&mount).unwrap().state["count"], 8);
            app.close(&mount, Ok(())).unwrap();
            app.shutdown().unwrap();
        } else {
            let error = result.err().unwrap().to_string();
            assert!(
                error.contains(&format!("unsupported scene language: {name} v{version}")),
                "{error}"
            );
            assert_eq!(factory_calls.get(), 0);
        }
    }
    assert!(
        NativeRegistry::<()>::default()
            .with_scene_compiler(
                maka_plugin_core::package::SceneLanguage {
                    name: "invalid/name".into(),
                    version: 1
                },
                |_, _| NativeView::new(Rc::new(()), BTreeMap::new(), vec![]),
            )
            .is_err()
    );
}

#[test]
fn value_only_guest_needs_no_scene_compiler_and_cannot_publish_components() {
    use maka_plugin_core::{
        composition::{Environment, ExecutionLocation, Input},
        contract::Interface,
        package::{Declaration, EntryTarget},
    };
    let timer = timer();
    let (_, packages) = fixture(0);
    let root = packages
        .iter()
        .find(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let mut entry =
        root.manifest().application.as_ref().unwrap().entries[&EntryTarget::Desktop].clone();
    entry.imports.clear();
    entry.scene = None;
    entry.exports.get_mut("App").unwrap().contract.interface = Interface::Value {
        schema: json!({"type":"integer"}),
    };
    let contract = entry.exports["App"].contract.clone();
    for undeclared in [false, true] {
        let source = if undeclared {
            // Actual-export checking must reject this before it can ever mount.
            let component = root.manifest().application.as_ref().unwrap().entries
                [&EntryTarget::Desktop]
                .exports["App"]
                .contract
                .clone();
            format!(
                "export function activate(){{return {{App:{{kind:'component',contract:{},component:['app']}}}};}}export const app={{mount(){{}},update(){{}},event(){{}}}};",
                serde_json::to_string(&component).unwrap()
            )
        } else {
            format!(
                "export function activate(){{return {{App:{{kind:'value',contract:{},value:42}}}};}}",
                serde_json::to_string(&contract).unwrap()
            )
        };
        let package = support::package(
            "plain.value",
            Declaration {
                entries: BTreeMap::from([(EntryTarget::Desktop, entry.clone())]),
                composition: None,
            },
            Some(source),
        );
        let input = Input {
            environment: Environment {
                location: ExecutionLocation::Ui,
                presentation: Some(PresentationTarget::Desktop),
            },
            instances: BTreeMap::from([("value".into(), support::node(&package, 1))]),
            launch: None,
        };
        let result = Application::<()>::new(
            &input,
            vec![package],
            timer.handle().clone(),
            Default::default(),
            NativeRegistry::default(),
            Default::default(),
        );
        if undeclared {
            let error = result.err().unwrap().to_string();
            assert!(error.contains("undeclared or changed export"), "{error}");
        } else {
            result.unwrap().shutdown().unwrap();
        }
    }
}
