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

#[allow(dead_code)]
mod support;
use futures_util::future::BoxFuture;
use maka_application_bridge::Configuration;
use maka_event_log::{
    EventLog,
    root::{RootNamespaces, RootOwner},
};
use maka_plugin_core::runtime::Published;
use maka_runtime_host::application::{
    Application, Context, Handler, Invocation, NativeActivation, NativeOwner, NativeRegistry,
    Policy,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

struct Permit;
impl Policy for Permit {
    fn admit(&self, _: Invocation<'_>) -> Result<(), String> {
        Ok(())
    }
}
struct FileOwner {
    file: Arc<Mutex<Option<std::fs::File>>>,
    closing: Arc<tokio::sync::Semaphore>,
    release: Arc<tokio::sync::Semaphore>,
}
impl Handler for FileOwner {
    fn call(&self, _: String, _: Value, _: Context) -> BoxFuture<'static, Result<Value, String>> {
        Box::pin(async { Ok(json!({"effects":0})) })
    }
}
impl NativeOwner for FileOwner {
    fn close(&self) -> BoxFuture<'static, Result<(), String>> {
        let file = self.file.clone();
        let closing = self.closing.clone();
        let release = self.release.clone();
        Box::pin(async move {
            closing.add_permits(1);
            release.acquire_owned().await.unwrap().forget();
            file.lock().unwrap().take();
            Ok(())
        })
    }
}
#[tokio::test]
async fn compiled_native_file_and_root_wait_for_the_original_application_and_sql_close() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("host");
    let namespaces = RootNamespaces {
        ownership: directory.path().join("owners"),
        control: directory.path().join("control"),
    };
    let original = Arc::new(RootOwner::create(&root, &namespaces).unwrap());
    let store = Arc::new(EventLog::for_root(original).await.unwrap());
    let deployment = uuid::Uuid::new_v4();
    let (input, packages) = support::native_host("native.File");
    store
        .commit_application(deployment, 0, input, vec![], packages)
        .await
        .unwrap();
    let file = Arc::new(Mutex::new(None));
    let closing = Arc::new(tokio::sync::Semaphore::new(0));
    let release = Arc::new(tokio::sync::Semaphore::new(0));
    let resource = Arc::new(FileOwner {
        file: file.clone(),
        closing: closing.clone(),
        release: release.clone(),
    });
    let path = directory.path().join("native-file");
    let mut native = NativeRegistry::default();
    native
        .insert(
            "native.File".into(),
            Arc::new(move |_, _| {
                let resource = resource.clone();
                let path = path.clone();
                Box::pin(async move {
                    // Compiled availability itself has no native resource effect.
                    *resource.file.lock().unwrap() = Some(std::fs::File::create(path).unwrap());
                    NativeActivation {
                        exports: Ok(BTreeMap::from([(
                            "Counter".into(),
                            Published {
                                contract: support::business(),
                                value: resource.clone() as Arc<dyn Handler>,
                            },
                        )])),
                        owner: resource,
                    }
                })
            }),
        )
        .unwrap();
    assert!(file.lock().unwrap().is_none());
    let application = Application::start_from_store(
        store.clone(),
        deployment,
        maka_application_bridge::with_gateway(native).unwrap(),
        Default::default(),
        Arc::new(Permit),
        Default::default(),
    )
    .await
    .unwrap();
    application.ready().await.unwrap();
    file.lock().unwrap().as_ref().unwrap().metadata().unwrap();
    let close = tokio::spawn(async move {
        application.shutdown().await.unwrap();
        application
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), closing.acquire_owned())
        .await
        .unwrap()
        .unwrap()
        .forget();
    assert!(!close.is_finished());
    file.lock().unwrap().as_ref().unwrap().metadata().unwrap();
    assert_eq!(
        RootOwner::open(&root, &namespaces).err().unwrap().kind(),
        std::io::ErrorKind::WouldBlock
    );
    release.add_permits(1);
    let application = close.await.unwrap();
    assert!(file.lock().unwrap().is_none());
    // Application close is not the SQL owner's independent close receipt.
    assert_eq!(
        RootOwner::open(&root, &namespaces).err().unwrap().kind(),
        std::io::ErrorKind::WouldBlock
    );
    drop(application);
    store.shutdown().await.unwrap();
    drop(store);
    drop(RootOwner::open(&root, &namespaces).unwrap());
}

#[test]
fn startup_configuration_is_closed_and_bounded_before_opening_any_host() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("connections.json");
    for data in [
        br#"{"hosts":[],"approved":true}"#.as_slice(),
        br#"{"hosts":[{"name":"local","credential":"fake"}]}"#.as_slice(),
    ] {
        std::fs::write(&path, data).unwrap();
        assert!(Configuration::read(&path).is_err());
    }
    std::fs::write(&path, vec![b' '; 1_048_577]).unwrap();
    assert!(Configuration::read(&path).err().unwrap().contains("1 MiB"));
    std::fs::write(&path, br#"{"hosts":[]}"#).unwrap();
    assert!(Configuration::read(&path).unwrap().hosts.is_empty());
}
