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
            NativeRegistry::<()>::with_surface(target),
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
