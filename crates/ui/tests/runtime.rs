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

#[path = "../examples/counter/support.rs"]
mod support;
use maka_plugin_core::{
    composition::{PackageUse, PresentationTarget, Target},
    contract::{Cardinality, Import},
    package::Declaration,
};
use maka_ui::{Application, surface_contract};
use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};
use support::{fixture, node, package};

fn app(handler: &str, timer: &tokio::runtime::Runtime) -> Application {
    let (input, packages) = fixture(handler);
    Application::new(
        &input,
        packages,
        timer.handle().clone(),
        maka_js_runtime::plugin::Limits {
            synchronous_slice: Duration::from_millis(100),
            ..Default::default()
        },
    )
    .unwrap()
}
fn timer() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_time()
        .build()
        .unwrap()
}

#[test]
fn dynamic_components_publish_then_mount_independent_state_and_commit_input() {
    let timer = timer();
    let mut app = app(
        "return frame({count:state.count+1,revision:state.revision+1});",
        &timer,
    );
    let a = app.mount_launch(json!({"count":10})).unwrap();
    let b = app.mount_launch(json!({"count":20})).unwrap();
    app.dispatch(&a, 0, "inc", json!({})).unwrap();
    assert_eq!(app.frame(&a).unwrap().state["count"], 11);
    assert_eq!(app.frame(&b).unwrap().state["count"], 20);
    assert!(app.dispatch(&a, 0, "inc", json!({})).is_err());
    assert_eq!(app.frame(&a).unwrap().revision, 1);
    app.close(&a, Ok(())).unwrap();
    assert!(app.dispatch(&a, 1, "inc", json!({})).is_err());
    app.dispatch(&b, 0, "inc", json!({})).unwrap();
    assert_eq!(app.frame(&b).unwrap().state["count"], 21);
    app.close(&b, Ok(())).unwrap();
    app.shutdown().unwrap();
}

#[test]
fn invalid_batch_and_script_failure_keep_the_last_committed_view() {
    let timer = timer();
    for handler in [
        "const value=frame({count:999,revision:state.revision+1});value.view.children.push(value.view.children[0]);return value;",
        "const value=frame({count:999,revision:state.revision+1});throw new Error('failed batch');",
    ] {
        let mut app = app(handler, &timer);
        let mount = app.mount_launch(json!({"count":3})).unwrap();
        assert!(app.dispatch(&mount, 0, "inc", Value::Null).is_err());
        assert_eq!(app.frame(&mount).unwrap().state["count"], 3);
        assert_eq!(app.frame(&mount).unwrap().revision, 0);
        app.close(&mount, Ok(())).unwrap();
    }
}

#[test]
fn realm_failure_retires_all_its_mounts_and_unknown_cleanup_keeps_the_owner() {
    let timer = timer();
    let mut app = app("while(true){}", &timer);
    let a = app.mount_launch(json!({})).unwrap();
    let b = app.mount_launch(json!({})).unwrap();
    assert!(app.dispatch(&a, 0, "inc", Value::Null).is_err());
    assert!(app.is_retiring(&a));
    assert!(app.is_retiring(&b));
    assert!(
        app.close(&a, Err("native window close unknown".into()))
            .is_err()
    );
    assert!(app.frame(&a).is_ok());
    app.close(&a, Ok(())).unwrap();
    app.close(&b, Ok(())).unwrap();
    app.shutdown().unwrap();
}

#[test]
fn structural_imports_route_to_owned_providers_without_startup_deadlock() {
    use maka_plugin_core::composition::Template;
    let (mut input, mut packages) =
        fixture("return frame({count:state.count+1,revision:state.revision+1});");
    let mut children = std::mem::take(&mut input.instances);
    children.get_mut("app").unwrap().bindings.insert(
        "surface".into(),
        vec![Target::Import {
            name: "surface".into(),
        }],
    );
    let product = package(
        "product",
        Declaration {
            entries: BTreeMap::new(),
            composition: Some(Template {
                parameters: BTreeMap::new(),
                imports: BTreeMap::from([(
                    "surface".into(),
                    Import {
                        contract: surface_contract(PresentationTarget::Desktop),
                        cardinality: Cardinality::One,
                    },
                )]),
                exports: BTreeMap::from([(
                    "App".into(),
                    Target::Instance {
                        path: vec!["app".into()],
                        export: "App".into(),
                    },
                )]),
                children,
            }),
        },
        None,
    );
    let mut root = node(&product, 100);
    root.use_package = PackageUse::Composition;
    root.bindings.insert(
        "surface".into(),
        vec![Target::Instance {
            path: vec!["product".into(), "surface".into()],
            export: "Surface".into(),
        }],
    );
    input.instances.insert("product".into(), root);
    input.launch = Some(Target::Instance {
        path: vec!["product".into()],
        export: "App".into(),
    });
    packages.push(product);
    let timer = timer();
    let mut app =
        Application::new(&input, packages, timer.handle().clone(), Default::default()).unwrap();
    let mount = app.mount_launch(json!({})).unwrap();
    app.dispatch(&mount, 0, "inc", Value::Null).unwrap();
    assert_eq!(app.frame(&mount).unwrap().state["count"], 1);
    assert!(app.shutdown().is_err());
    assert!(app.is_retiring(&mount));
    assert!(app.frame(&mount).is_ok());
    app.close(&mount, Ok(())).unwrap();
    app.shutdown().unwrap();
}

#[test]
fn unimplemented_window_mutation_is_rejected_without_scene_commit() {
    let timer = timer();
    let mut app = app(
        "const next=frame({count:state.count+1,revision:state.revision+1});next.window.width=900;return next;",
        &timer,
    );
    let mount = app.mount_launch(json!({})).unwrap();
    assert!(app.dispatch(&mount, 0, "inc", Value::Null).is_err());
    assert_eq!(app.frame(&mount).unwrap().revision, 0);
    assert_eq!(app.frame(&mount).unwrap().window.width, 640);
    app.close(&mount, Ok(())).unwrap();
    app.shutdown().unwrap();
}
