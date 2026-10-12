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

//! Public IPC executes the same persisted application and owns caller intent.
#[allow(dead_code)]
mod support;
use maka_event_log::{
    EventLog,
    root::{RootNamespaces, RootOwner},
};
use maka_host_runtime::{
    application::{
        NativeRegistry,
        host::{Client, Host, discovery},
        transport::{Grant, HostEndpoint, Reply},
    },
    ipc::LocalListener,
};
use maka_plugin_core::operation::{OperationCleanup, OperationOutcome, Receipt};
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn public_ipc_persists_original_gateway_intent_and_retires_original_owner() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let directory = tempfile::tempdir().unwrap();
        maka_event_log::root::private_directory(directory.path()).unwrap();
        let namespaces = RootNamespaces { ownership: directory.path().join("owners"), control: directory.path().join("control") };
        let root = directory.path().join("root");
        let store = Arc::new(EventLog::for_root(Arc::new(RootOwner::create(&root, &namespaces).unwrap())).await.unwrap());
        let deployment = Uuid::new_v4();
        let (input, packages) = support::host();
        let source = store.commit_application(deployment, 0, input, vec![], packages).await.unwrap();
        let host = Host::open(store.clone(), deployment, NativeRegistry::default(), Some(maka_host_js::factory(Default::default())), BTreeMap::from([
            ("business".into(), HostEndpoint { consumer: support::identity(13), import: "business".into(), grant: Grant { methods: ["read".into(), "write".into()].into(), recovery: false } }),
            ("receipts".into(), HostEndpoint { consumer: support::identity(13), import: "receipts".into(), grant: Grant { methods: ["read".into()].into(), recovery: true } }),
        ]), vec![]).await.unwrap();
        #[cfg(unix)]
        let address = directory.path().join("host.sock");
        #[cfg(windows)]
        let address = std::path::PathBuf::from(format!(r"\\.\pipe\maka-public-test-{}", Uuid::new_v4()));
        let listener = LocalListener::bind(&address).unwrap();
        let registration = host.publish_registration(&address, None).unwrap();
        let discovery = discovery::read_with_namespaces(&root, &namespaces).unwrap();
        let serving = tokio::spawn({ let host = host.clone(); async move { host.serve_local(listener, CancellationToken::new()).await } });
        let mut live = Client::status(&address, Some(host.epoch())).await.unwrap();
        discovery.verify(&live).unwrap();
        live.source.revision += 1;
        assert!(discovery.verify(&live).is_err());
        live.source.revision -= 1;
        live.generation = Some("another deployment".into());
        assert!(discovery.verify(&live).is_err());
        live.generation = None;
        live.pid = std::num::NonZeroU32::new(live.pid.get() + 1).unwrap();
        assert!(discovery.verify(&live).is_err());
        assert!(Client::connect(&address, Some(Uuid::new_v4())).await.is_err());
        let mut client = Client::connect(&address, Some(host.epoch())).await.unwrap();
        let mut other = Client::connect(&address, Some(host.epoch())).await.unwrap();
        assert_eq!(Client::status(&address, Some(host.epoch())).await.unwrap().sessions, 2);
        assert!(!Client::retire_with_options(&address, host.epoch(), false, false).await.unwrap());
        let mut write = client.call("business", "write".into(), Value::Null, Default::default()).await.unwrap();
        write.start().await.unwrap();
        assert!(matches!(write.receive().await.unwrap(), Reply::Prepared { .. }));
        let reference = write.reference().unwrap().clone();
        assert!(store.application_intent(deployment, support::identity(13), reference.operation).await.unwrap().is_none());
        assert!(matches!(write.confirm().await.unwrap(), Reply::Mutation { ref reply, cleanup: Ok(()) } if matches!(reply.receipt, Receipt::Recorded { outcome: OperationOutcome::Returned { .. }, cleanup: OperationCleanup::Confirmed })));
        assert!(write.cleanup_confirmed());
        let intent = store.application_intent(deployment, support::identity(13), reference.operation).await.unwrap().unwrap();
        assert_eq!(intent.reference, reference);
        assert_eq!(intent.caller, source);
        assert_eq!(intent.consumer, support::identity(13));
        // An already committed peer attestation cannot choose the public mode.
        let mut rejected = client.call("business", "write".into(), Value::Null, Default::default()).await.unwrap();
        rejected.start().await.unwrap();
        assert!(matches!(rejected.receive().await.unwrap(), Reply::Prepared { .. }));
        let recorded = store.record_application_intent(source.clone(), support::identity(13), rejected.reference().unwrap().clone(), Value::Null).await.unwrap();
        assert!(matches!(rejected.commit(recorded).await.unwrap(), Reply::Aborted { cleanup: Ok(()), .. }));
        assert!(rejected.cleanup_confirmed());
        client.close().await.unwrap();
        assert!(client.call("business", "read".into(), Value::Null, Default::default()).await.is_err());
        // Closing one physical session cannot revoke a different session.
        let mut read = other.call("business", "read".into(), Value::Null, Default::default()).await.unwrap();
        read.start().await.unwrap();
        assert!(matches!(read.receive().await.unwrap(), Reply::Finished { result: Ok(ref value), cleanup: Ok(()) } if value["effects"] == 1));
        assert!(read.cleanup_confirmed());
        other.close().await.unwrap();
        assert!(Client::retire(&address, host.epoch()).await.unwrap());
        serving.await.unwrap().unwrap();
        assert!(host.status().closed);
        assert!(discovery.verify(&host.status()).is_err());
        registration.remove().unwrap();
        assert_eq!(discovery::read_with_namespaces(&root, &namespaces).unwrap_err().kind(), std::io::ErrorKind::NotFound);
        drop((client, other, write, rejected, read, host, store));
        drop(RootOwner::open(&root, &namespaces).unwrap());
    }).await.unwrap();
}

struct Blocked {
    entered: Arc<tokio::sync::Semaphore>,
    cancelled: Arc<tokio::sync::Semaphore>,
    release: Arc<tokio::sync::Semaphore>,
}
impl maka_host_runtime::application::Handler for Blocked {
    fn call(
        &self,
        _: String,
        _: Value,
        context: maka_host_runtime::application::Context,
    ) -> futures_util::future::BoxFuture<'static, Result<Value, String>> {
        let entered = self.entered.clone();
        let cancelled = self.cancelled.clone();
        let release = self.release.clone();
        Box::pin(async move {
            entered.add_permits(1);
            context.scope().cancellation.cancelled().await;
            cancelled.add_permits(1);
            release.acquire().await.unwrap().forget();
            Ok(Value::Null)
        })
    }
}
impl maka_host_runtime::application::NativeOwner for Blocked {
    fn prepare_retirement(
        &self,
    ) -> Option<Box<dyn maka_host_runtime::application::NativeRetirement>> {
        Some(Box::new(()))
    }
    fn close(&self) -> futures_util::future::BoxFuture<'static, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lost_session_close_observer_cannot_release_accepted_work_or_root_early() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let directory = tempfile::tempdir().unwrap();
        maka_event_log::root::private_directory(directory.path()).unwrap();
        let namespaces = RootNamespaces { ownership: directory.path().join("owners"), control: directory.path().join("control") };
        let root = directory.path().join("root");
        let store = Arc::new(EventLog::for_root(Arc::new(RootOwner::create(&root, &namespaces).unwrap())).await.unwrap());
        let deployment = Uuid::new_v4();
        let (mut input, mut packages) = support::host();
        let package = packages.iter_mut().find(|package| package.manifest().id == "local.business").unwrap();
        let mut declaration = package.manifest().application.clone().unwrap();
        declaration.entries.get_mut(&maka_plugin_core::package::EntryTarget::Host).unwrap().implementation = maka_plugin_core::package::Implementation::Native { implementation: "public.blocked".into() };
        *package = support::counter::package("local.business", declaration, None);
        input.instances.get_mut("business").unwrap().package = package.application_ref().into();
        let blocked = Arc::new(Blocked { entered: Arc::new(tokio::sync::Semaphore::new(0)), cancelled: Arc::new(tokio::sync::Semaphore::new(0)), release: Arc::new(tokio::sync::Semaphore::new(0)) });
        let mut native = NativeRegistry::default();
        native.insert("public.blocked".into(), Arc::new({ let blocked = blocked.clone(); move |_, _| { let blocked = blocked.clone(); Box::pin(async move { maka_host_runtime::application::NativeActivation {
            exports: Ok(BTreeMap::from([("Counter".into(), maka_plugin_core::runtime::Published { contract: support::business(), value: blocked.clone() as Arc<dyn maka_host_runtime::application::Handler> })])), owner: blocked,
        } }) } })).unwrap();
        store.commit_application(deployment, 0, input, vec![], packages).await.unwrap();
        let host = Host::open(store.clone(), deployment, native, Some(maka_host_js::factory(Default::default())), BTreeMap::from([
            ("business".into(), HostEndpoint { consumer: support::identity(13), import: "business".into(), grant: Grant { methods: ["write".into()].into(), recovery: false } }),
        ]), vec![]).await.unwrap();
        #[cfg(unix)]
        let address = directory.path().join("host.sock");
        #[cfg(windows)]
        let address = std::path::PathBuf::from(format!(r"\\.\pipe\maka-public-test-{}", Uuid::new_v4()));
        let listener = LocalListener::bind(&address).unwrap();
        let serving = tokio::spawn({ let host = host.clone(); async move { host.serve_local(listener, CancellationToken::new()).await } });
        let mut client = Client::connect(&address, Some(host.epoch())).await.unwrap();
        let mut call = client.call("business", "write".into(), Value::Null, Default::default()).await.unwrap();
        call.start().await.unwrap();
        assert!(matches!(call.receive().await.unwrap(), Reply::Prepared { .. }));
        let mut confirming = Box::pin(call.confirm());
        tokio::select! {
            permit = blocked.entered.acquire() => permit.unwrap().forget(),
            result = &mut confirming => panic!("accepted work ended before release: {result:?}"),
        }
        drop(confirming);
        assert!(!Client::retire(&address, host.epoch()).await.unwrap());
        let mut closing = Box::pin(client.close());
        tokio::select! {
            permit = blocked.cancelled.acquire() => permit.unwrap().forget(),
            result = &mut closing => panic!("session acknowledged before physical cleanup: {result:?}"),
        }
        drop(closing);
        drop(client);
        assert!(!Client::retire(&address, host.epoch()).await.unwrap());
        assert_eq!(RootOwner::open(&root, &namespaces).err().unwrap().kind(), std::io::ErrorKind::WouldBlock);
        assert!(Client::retire_with_options(&address, host.epoch(), true, true).await.unwrap());
        host.wait_for_drain().await;
        assert!(!host.status().closed);
        assert!(!serving.is_finished());
        assert_eq!(RootOwner::open(&root, &namespaces).err().unwrap().kind(), std::io::ErrorKind::WouldBlock);
        blocked.release.add_permits(1);
        serving.await.unwrap().unwrap();
        // The disconnected invocation still has no same-channel cleanup receipt.
        call.disconnect();
        assert!(!call.cleanup_confirmed());
        drop((call, host, store));
        drop(RootOwner::open(&root, &namespaces).unwrap());
    }).await.unwrap();
}
