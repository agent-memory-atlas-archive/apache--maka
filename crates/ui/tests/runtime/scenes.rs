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
    let mut source = std::str::from_utf8(old.file("index.mjs").unwrap())
        .unwrap()
        .to_owned();
    source.push_str(&format!("const previous=frame;frame=(state)=>{{const output=previous(state);output.frame.view={{language:'private.fixture',blob:'x'.repeat({payload})}};return output;}};"));
    let package = support::package(
        "dynamic.app",
        old.manifest().application.clone().unwrap(),
        Some(source),
    );
    input.instances.get_mut("app").unwrap().package = package.application_ref().into();
    packages.push(package);
    (input, packages)
}
#[test]
fn opaque_guest_scenes_cannot_be_laundered_out_of_retained_budget() {
    let timer = timer();
    let (input, packages) = fixture(950_000);
    let native = NativeRegistry::with_surface(PresentationTarget::Desktop).with_scene_compiler(
        |source, _| {
            assert_eq!(source["language"], "private.fixture");
            // Deliberately tries to give the kernel a smaller declaration. The
            // original source must still be retained and charged by the kernel.
            NativeView::new(
                Rc::new(()),
                BTreeMap::from([("inc".into(), json!(true))]),
                vec![],
            )?
            .with_declaration(json!({"tiny":true}))
        },
    );
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
    let native = NativeRegistry::with_surface(PresentationTarget::Desktop).with_scene_compiler(
        move |_, _| {
            observed.set(observed.get() + 1);
            NativeView::new(Rc::new(()), BTreeMap::new(), vec![])
        },
    );
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
    let native = NativeRegistry::with_surface(PresentationTarget::Desktop).with_scene_compiler(
        move |_, _| {
            if observed.get() {
                return Err(maka_ui::Error::Invalid("scene compile failure".into()));
            }
            NativeView::new(
                Rc::new(()),
                BTreeMap::from([("inc".into(), json!(true))]),
                vec![],
            )
        },
    );
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
