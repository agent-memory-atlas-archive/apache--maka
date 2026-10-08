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
use maka_plugin_core::package::EntryTarget;
use maka_plugins::package::Package;
use maka_ui::Surface;
use std::{path::Path, process::Command};

#[test]
#[ignore = "opt-in packaged product build and actual native schema compiler"]
fn packaged_product_component_schemas_compile_independently_for_both_presentations() {
    use maka_plugin_core::{
        contract::Interface,
        package::{Declaration, Implementation},
    };
    let timer = timer();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let build = Command::new("node")
        .arg("packages/agent-product/scripts/build.mjs")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let dist = root.join("packages/agent-product/dist");
    let names: Vec<String> =
        serde_json::from_slice(&std::fs::read(dist.join("packages.json")).unwrap()).unwrap();
    let mut checked_palette = 0;
    for name in names {
        let manifest: Value = serde_json::from_slice(
            &std::fs::read(
                dist.join("packages")
                    .join(&name)
                    .join("maka.extension.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let declaration: Declaration =
            serde_json::from_value(manifest["application"].clone()).unwrap();
        for (target, entry) in declaration.entries {
            if name == "command-palette" {
                let Implementation::JavaScript { module, .. } = &entry.implementation else {
                    panic!("ordinary palette requires JavaScript");
                };
                let mut realm = maka_js_runtime::plugin::Foreground::new(
                    Default::default(),
                    timer.handle().clone(),
                )
                .unwrap();
                let palette = realm
                    .load(
                        "palette",
                        &std::fs::read_to_string(
                            dist.join("packages/command-palette").join(module),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                let actual: BTreeMap<String, maka_ui::abi::UiExport> = serde_json::from_value(
                    realm
                        .call(&palette, &["activate".into()], &[json!({}), json!({})])
                        .unwrap(),
                )
                .unwrap();
                let maka_ui::abi::UiExport::Component {
                    contract,
                    component,
                } = &actual["CommandPalette"]
                else {
                    panic!("palette must publish its component");
                };
                assert_eq!(contract, &entry.exports["CommandPalette"].contract);
                let callbacks = realm
                    .capture(&palette, component, &["mount", "event", "update"])
                    .unwrap();
                let appearance = realm
                    .load(
                        "appearance",
                        &std::fs::read_to_string(dist.join("packages/appearance").join(module))
                            .unwrap(),
                    )
                    .unwrap();
                let theme = realm
                    .call(
                        &appearance,
                        &["activate".into()],
                        &[json!({"scheme":"light"})],
                    )
                    .unwrap()["Appearance"]["value"]
                    .clone();
                let props = json!({"theme":theme, "locale":"en", "open":true, "busy":false,
                    "commands":[{"id":"settings", "label":"Settings", "hint":"", "disabled":false}]});
                let mounted: maka_ui::ComponentOutput = serde_json::from_value(
                    realm
                        .call_bound(&callbacks, "mount", &[props.clone(), json!({})])
                        .unwrap(),
                )
                .unwrap();
                assert!(mounted.actions.is_empty());
                assert!(mounted.events.is_empty());
                let closed: maka_ui::ComponentOutput = serde_json::from_value(
                    realm
                        .call_bound(
                            &callbacks,
                            "event",
                            &[
                                mounted.frame.state,
                                json!({"kind":"raw", "key":"close", "event":null}),
                                props,
                                json!({}),
                            ],
                        )
                        .unwrap(),
                )
                .unwrap();
                assert_eq!(closed.events[0].name, "closed");
                assert_eq!(
                    closed.frame.state["open"], true,
                    "parent still owns controlled open"
                );
                realm.close(&palette).unwrap();
                realm.close(&appearance).unwrap();
            }
            for (export, contract) in entry.exports {
                let Interface::Component {
                    props,
                    events,
                    captures,
                } = contract.contract.interface
                else {
                    continue;
                };
                let check = |schema: &Value| {
                    jsonschema::validator_for(schema)
                        .unwrap_or_else(|error| panic!("{name}/{target:?}/{export}: {error}"))
                };
                check(&props);
                for schema in events.values() {
                    check(schema);
                }
                for capture in captures.values() {
                    check(&capture.input);
                }
                if export == "CommandPalette" {
                    checked_palette += 1;
                    assert!(check(&events["selected"]).is_valid(&json!({
                        "id":"session", "title":"Native schema checked", "revision":1,
                        "updatedAt":1, "archived":false, "activity":"idle", "labels":[],
                        "workspace":{"path":"/workspace", "directoryIdentity":format!("sha256:{}", "a".repeat(64)), "identity":"workspace:v1:00000000-0000-0000-0000-000000000001"},
                    })));
                    assert!(!check(&events["selected"]).is_valid(&json!({"id":"session"})));
                }
            }
        }
    }
    assert_eq!(checked_palette, 2);
}

#[test]
fn sdk_typescript_bundles_execute_the_same_package_in_both_presentations() {
    let timer = timer();
    let work = tempfile::tempdir().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sdk = root.join("packages/plugin-sdk");
    for (variant, output) in [("desktop", "index.mjs"), ("terminal", "terminal.mjs")] {
        let status = Command::new("node")
            .arg(sdk.join("scripts/bundle.mjs"))
            .arg(sdk.join(format!("examples/counter/{variant}.ts")))
            .arg(work.path().join(output))
            .current_dir(&root)
            .status()
            .unwrap();
        assert!(status.success(), "SDK bundle failed for {variant}");
    }
    let (mut input, mut packages) = support::dual_fixture("");
    let old = packages.pop().unwrap();
    let mut files = BTreeMap::from([(
        maka_plugins::package::MANIFEST_FILE.into(),
        serde_json::to_vec(&json!({"schemaVersion":2,"id":old.manifest().id,
            "application":old.manifest().application}))
        .unwrap(),
    )]);
    for name in ["index.mjs", "terminal.mjs"] {
        files.insert(name.into(), std::fs::read(work.path().join(name)).unwrap());
    }
    let package = Package::new(files).unwrap();
    assert_eq!(
        package
            .manifest()
            .application
            .as_ref()
            .unwrap()
            .entries
            .len(),
        2
    );
    assert!(
        package
            .manifest()
            .application
            .as_ref()
            .unwrap()
            .entries
            .contains_key(&EntryTarget::Tui)
    );
    input.instances.get_mut("app").unwrap().package = package.application_ref().into();
    packages.push(package);
    for target in [PresentationTarget::Desktop, PresentationTarget::Tui] {
        input.environment.presentation = Some(target);
        let mut app = Application::new(
            &input,
            packages.clone(),
            timer.handle().clone(),
            maka_js_runtime::plugin::Limits {
                synchronous_slice: Duration::from_millis(100),
                ..Default::default()
            },
            crate::support::scene::registry::<()>(target),
            Default::default(),
        )
        .unwrap();
        let first = app.mount_launch(json!({"count":10})).unwrap();
        let second = app.mount_launch(json!({"count":20})).unwrap();
        let surface = app.frame(&first).unwrap().surface.as_ref().unwrap();
        match (target, surface) {
            (PresentationTarget::Tui, Surface::Terminal { binding }) => {
                assert_eq!(binding, "surface")
            }
            (PresentationTarget::Desktop, Surface::Window { title, width, .. }) => {
                assert_eq!(title, "SDK counter");
                assert_eq!(*width, 640);
            }
            _ => panic!("incorrect presentation variant"),
        }
        app.dispatch(&first, 0, "inc", json!({})).unwrap();
        assert_eq!(app.frame(&first).unwrap().state["count"], 11);
        assert_eq!(app.frame(&second).unwrap().state["count"], 20);
        assert!(app.dispatch(&first, 0, "inc", json!({})).is_err());
        app.close(&first, Ok(())).unwrap();
        app.close(&second, Ok(())).unwrap();
        app.shutdown().unwrap();
    }
}
