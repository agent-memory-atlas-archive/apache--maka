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
use maka_plugin_core::composition::PresentationTarget;
use maka_ui_runtime::Application;
use serde_json::json;
use std::time::{Duration, Instant};

fn ready(
    handler: &str,
) -> (
    tokio::runtime::Runtime,
    Application,
    maka_plugin_core::runtime::Mount,
) {
    let scheduler = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_time()
        .build()
        .unwrap();
    let (input, packages) = support::fixture(handler);
    let mut app = Application::new(
        &input,
        packages,
        scheduler.handle().clone(),
        maka_ui_js::execution(maka_js_runtime::plugin::Limits {
            synchronous_slice: Duration::from_millis(500),
            ..Default::default()
        }),
        support::scene::registry::<()>(PresentationTarget::Desktop),
        Default::default(),
    )
    .unwrap();
    scheduler.block_on(app.wait_ready()).unwrap();
    let mount = app.mount_launch(json!({"count":3})).unwrap();
    scheduler.block_on(app.wait_idle()).unwrap();
    (scheduler, app, mount)
}
#[test]
fn slow_guest_keeps_owner_available_and_only_prepared_commit_publishes_state() {
    let (scheduler, mut app, mount) = ready(
        "const until=Date.now()+100;while(Date.now()<until){};return frame({count:state.count+1,revision:state.revision+1});",
    );
    app.defer_native_commit();
    let at = Instant::now();
    app.dispatch(&mount, 0, "inc", json!({})).unwrap();
    assert!(
        at.elapsed() < Duration::from_millis(30),
        "foreground waited for JavaScript: {:?}",
        at.elapsed()
    );
    assert_eq!(app.frame(&mount).unwrap().state["count"], 3);
    assert!(app.logical_pending());
    scheduler.block_on(app.wait_prepared(&mount)).unwrap();
    assert_eq!(app.frame(&mount).unwrap().state["count"], 3);
    assert_eq!(app.prepared_frame(&mount).unwrap().state["count"], 4);
    app.commit_ui(|_, candidate| {
        assert_eq!(candidate.changed().count(), 1);
        Ok(())
    })
    .unwrap();
    assert_eq!(app.frame(&mount).unwrap().state["count"], 4);
    app.close(&mount, Ok(())).unwrap();
    scheduler.block_on(async {
        loop {
            match app.shutdown() {
                Ok(()) => break,
                Err(maka_ui_runtime::Error::Core(maka_plugin_core::Error::CleanupPending)) => {
                    tokio::task::yield_now().await
                }
                Err(error) => panic!("{error}"),
            }
        }
    });
}
#[test]
fn retired_mount_waits_for_actual_guest_completion_and_rejects_late_candidate() {
    let (scheduler, mut app, mount) = ready(
        "const until=Date.now()+100;while(Date.now()<until){};return frame({count:999,revision:state.revision+1});",
    );
    app.dispatch(&mount, 0, "inc", json!({})).unwrap();
    app.retire(&mount).unwrap();
    // Native close and accepted execution settle independently. The original
    // provider/Job remains retained after its physical surface is actually gone.
    app.close(&mount, Ok(())).unwrap();
    assert!(!app.can_shutdown());
    assert!(scheduler.block_on(app.wait_idle()).is_err());
    assert!(app.frame(&mount).is_err());
    scheduler.block_on(async {
        loop {
            match app.shutdown() {
                Ok(()) => break,
                Err(maka_ui_runtime::Error::Core(maka_plugin_core::Error::CleanupPending)) => {
                    tokio::task::yield_now().await
                }
                Err(error) => panic!("{error}"),
            }
        }
    });
}
