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

//! Live macOS probe of the production launcher, native quit selector and Host close.
//! Run with a fresh evidence directory and optionally `fail`; create `release`
//! after `closing` appears. Failed close intentionally remains fenced until killed.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
#[path = "../src/main.rs"]
mod desktop;
#[cfg(target_os = "macos")]
#[allow(dead_code)]
#[path = "../../application-bridge/tests/support/mod.rs"]
mod fixture;
#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    probe::run()
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This live native quit probe requires macOS.");
}
#[cfg(target_os = "macos")]
mod probe {
    use super::*;
    use futures_util::future::BoxFuture;
    use maka_event_log::{
        EventLog,
        root::{RootNamespaces, RootOwner},
    };
    use maka_plugin_core::{composition::PresentationTarget, runtime::Published};
    use maka_runtime_host::application::{
        Context, Handler, NativeActivation, NativeOwner, NativeRegistry,
    };
    use objc2::{class, msg_send};
    use serde_json::{Value, json};
    use std::{
        collections::BTreeMap,
        path::PathBuf,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };
    struct Resource {
        directory: PathBuf,
        file: Mutex<Option<std::fs::File>>,
        effects: AtomicUsize,
        fail: bool,
    }
    impl Handler for Resource {
        fn call(
            &self,
            method: String,
            _: Value,
            _: Context,
        ) -> BoxFuture<'static, Result<Value, String>> {
            let effects = if method == "write" {
                self.effects.fetch_add(1, Ordering::SeqCst) + 1
            } else {
                self.effects.load(Ordering::SeqCst)
            };
            Box::pin(async move { Ok(json!({"effects":effects})) })
        }
    }
    struct Close(Arc<Resource>);
    impl NativeOwner for Close {
        fn close(&self) -> BoxFuture<'static, Result<(), String>> {
            let resource = self.0.clone();
            Box::pin(async move {
                std::fs::write(
                    resource.directory.join("closing"),
                    b"actual Host activation close started",
                )
                .map_err(|e| e.to_string())?;
                while !resource.directory.join("release").is_file() {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                if resource.fail {
                    std::fs::write(
                        resource.directory.join("failed"),
                        b"real native File remains held",
                    )
                    .map_err(|e| e.to_string())?;
                    return Err("probe native File cleanup failed; custody retained".into());
                }
                resource.file.lock().unwrap().take();
                std::fs::write(
                    resource.directory.join("closed"),
                    b"actual native File released before successful close receipt",
                )
                .map_err(|e| e.to_string())?;
                Ok(())
            })
        }
    }
    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let directory = PathBuf::from(
            std::env::args_os()
                .nth(1)
                .ok_or("fresh probe directory required")?,
        );
        std::fs::create_dir(&directory)?;
        let mode = std::env::args().nth(2);
        let fail = mode.as_deref() == Some("fail");
        let sql = mode.as_deref() == Some("sql");
        let timer = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_time()
            .build()?;
        let namespaces = RootNamespaces::for_current_account()?;
        let host = uuid::Uuid::new_v4();
        let caller = uuid::Uuid::new_v4();
        timer.block_on(async {
            for (name, id, ui) in [("host", host, false), ("caller", caller, true)] {
                if sql && !ui {
                    continue;
                }
                let owner = Arc::new(RootOwner::create(&directory.join(name), &namespaces)?);
                let store = EventLog::for_root(owner).await?;
                let (input, packages, observations) = if sql {
                    let (input, packages) = fixture::counter::dual_fixture("");
                    (input, packages, vec![])
                } else if ui {
                    fixture::ui(PresentationTarget::Desktop)
                } else {
                    let (input, packages) = fixture::native_host("probe.File");
                    (input, packages, vec![])
                };
                store
                    .commit_application(id, 0, input, observations, packages)
                    .await?;
                store.close().await?;
            }
            Result::<(), Box<dyn std::error::Error>>::Ok(())
        })?;
        let path = directory.join("connections.json");
        let mut native = NativeRegistry::default();
        if sql {
            std::fs::write(&path, b"{\"hosts\":[]}")?;
        } else {
            std::fs::write(
                &path,
                serde_json::to_vec(
                    &json!({"hosts":[{"name":"local","root":directory.join("host"),"deployment":host,"consumer":fixture::identity(13),"imports":{"business":{"methods":["read","write"],"recovery":false},"receipts":{"methods":["read"],"recovery":true}}}]}),
                )?,
            )?;
            let resource = Arc::new(Resource {
                file: Mutex::new(Some(std::fs::File::create(
                    directory.join("native-resource"),
                )?)),
                directory: directory.clone(),
                effects: AtomicUsize::new(0),
                fail,
            });
            native.insert(
                "probe.File".into(),
                Arc::new(move |_| {
                    let resource = resource.clone();
                    Box::pin(async move {
                        NativeActivation {
                            exports: Ok(BTreeMap::from([(
                                "Counter".into(),
                                Published {
                                    contract: fixture::business(),
                                    value: resource.clone() as Arc<dyn Handler>,
                                },
                            )])),
                            owner: Arc::new(Close(resource)),
                        }
                    })
                }),
            )?;
        }
        let launcher = desktop::Launcher::start(
            desktop::Args {
                root: directory.join("caller"),
                deployment: caller,
                connections: Some(path),
            },
            native,
            maka_ui_gpui::registry(),
        )?;
        if sql {
            // This is the launcher's same actual EventLog, not another Root owner.
            // Lose the accepted writer's waiter while a real SQLite lock holds it.
            let blocker = rusqlite::Connection::open(
                directory
                    .join("caller")
                    .join(maka_event_log::root::ROOT_DATABASE),
            )?;
            blocker.execute_batch("BEGIN IMMEDIATE;")?;
            let pending = uuid::Uuid::new_v4();
            let (input, packages) = fixture::counter::dual_fixture("");
            let waited = launcher.timer.block_on(async {
                tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    launcher
                        .store
                        .commit_application(pending, 0, input, vec![], packages),
                )
                .await
            });
            if waited.is_ok() {
                return Err("real SQL writer did not remain blocked".into());
            }
            std::fs::write(
                directory.join("sql_pending"),
                serde_json::to_vec(&json!({"caller":caller,"pending":pending}))?,
            )?;
            let release = directory.clone();
            std::thread::spawn(move || {
                while !release.join("release_sql").is_file() {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                blocker.execute_batch("COMMIT;").unwrap();
            });
        }
        // Request actual NSApplication terminate on its main queue after native
        // window creation. No App::quit test shortcut or synthetic receipt.
        let signal = directory.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let signal = signal.clone();
                dispatch2::DispatchQueue::main().exec_async(move || unsafe {
                    let app: *mut objc2::runtime::AnyObject =
                        msg_send![class!(NSApplication), sharedApplication];
                    let windows: *mut objc2::runtime::AnyObject = msg_send![app, windows];
                    let count: usize = msg_send![windows, count];
                    if count > 0 && !signal.join("requested").exists() {
                        std::fs::write(
                            signal.join("requested"),
                            b"native terminate requested with a real window",
                        )
                        .unwrap();
                        let _: () =
                            msg_send![app,terminate:std::ptr::null_mut::<objc2::runtime::AnyObject>()];
                    }
                    if signal.join("request_again").is_file() {
                        std::fs::remove_file(signal.join("request_again")).unwrap();
                        let _: () =
                            msg_send![app,terminate:std::ptr::null_mut::<objc2::runtime::AnyObject>()];
                    }
                });
            }
        });
        launcher.run()
    }
}
