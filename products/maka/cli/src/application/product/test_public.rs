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

//! Real public transport fixture; management operations retain their production implementation.
use maka_event_log::EventLog;
use maka_host_runtime::application::{
    host::{Host, LifecycleMode, Options, discovery::Registration},
    management::{ManagementTarget, provide_target},
    transport::HostEndpoint,
};
use maka_packages::Package;
use maka_plugin_core::{composition::Input, deployment::management::deployment_manager_contract};
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(super) struct PublishedHost {
    registration: Registration,
    cancel: CancellationToken,
    _cleanup: tokio_util::sync::DropGuard,
    server: tokio::task::JoinHandle<Result<(), String>>,
}
impl PublishedHost {
    /// A separate controller uses the trusted fixed-target manager capability,
    /// so tests can retain deliberately empty or otherwise unlaunchable sources.
    pub(super) async fn start(store: Arc<EventLog>, deployment: Uuid) -> Self {
        let mut source = store
            .read_application_metadata(deployment)
            .await
            .unwrap()
            .unwrap()
            .source
            .reference;
        source.revision = 1;
        let target = ManagementTarget::from_source(store.clone(), source)
            .await
            .unwrap();
        let contract = deployment_manager_contract();
        let implementation = "maka.test.product-manager";
        let package = |id: &str, implementation: &str, imports, exports| {
            Package::new(BTreeMap::from([(
                "maka.extension.json".into(),
                serde_json::to_vec(&json!({
                    "schemaVersion":1,"id":id,"application":{"entries":{"host":{
                        "implementation":{"kind":"native","implementation":implementation},
                        "configuration":{"type":"object","additionalProperties":false},
                        "imports":imports,"exports":exports
                    }}}
                }))
                .unwrap(),
            )]))
            .unwrap()
        };
        let manager = package(
            "maka.test.manager",
            implementation,
            json!({}),
            json!({"Management":{"contract":contract,"availability":"required"}}),
        );
        let gateway = package(
            "maka.test.gateway",
            "maka.application.gateway",
            json!({"management":{"contract":contract,"cardinality":{"kind":"one"}}}),
            json!({}),
        );
        let consumer = Uuid::new_v4();
        let input: Input = serde_json::from_value(json!({
            "environment":{"location":"host"},"instances":{
                "manager":{"key":Uuid::new_v4(),"package":{"source":"pinned","package":manager.application_ref()},"usePackage":"entry"},
                "gateway":{"key":consumer,"package":{"source":"pinned","package":gateway.application_ref()},"usePackage":"entry","bindings":{"management":[{"kind":"instance","path":["manager"],"export":"Management"}]}}
            }
        })).unwrap();
        let controller = Uuid::new_v4();
        store
            .commit_application(controller, 0, input, vec![], vec![manager, gateway])
            .await
            .unwrap();
        let mut native = crate::application::serve::native().unwrap();
        provide_target(&mut native, implementation.into(), target).unwrap();
        let endpoints: BTreeMap<String, HostEndpoint> = serde_json::from_value(json!({"management":{
            "consumer":[consumer],"import":"management","grant":{"methods":["read","readDocument","exportPackage","replace"],"recovery":false}
        }})).unwrap();
        let host = Host::open_with_options(
            store,
            controller,
            native,
            None,
            endpoints,
            vec![],
            Options {
                lifecycle_mode: LifecycleMode::Service,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        Self::publish(host)
    }
    pub(super) async fn start_product(
        store: Arc<EventLog>,
        deployment: Uuid,
        endpoints: BTreeMap<String, HostEndpoint>,
        native: Vec<maka_host_runtime::application::transport::NativeGrant>,
    ) -> Self {
        let original = store.load_application(deployment).await.unwrap().unwrap();
        super::headless::save_product(
            &store,
            deployment,
            maka_event_log::applications::PreparedApplication::new(
                original.source().input.clone(),
                original.source().remote_exports.clone(),
                original.packages().to_vec(),
            )
            .unwrap(),
            endpoints.clone(),
            native.clone(),
        )
        .await
        .unwrap();
        let host = Host::open_with_options(
            store,
            deployment,
            crate::application::serve::native().unwrap(),
            Some(maka_host_js::factory(Default::default())),
            endpoints,
            native,
            Options {
                lifecycle_mode: LifecycleMode::Service,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        Self::publish(host)
    }
    fn publish(host: Arc<Host>) -> Self {
        let endpoint = crate::endpoint::LocalEndpoint::bind().unwrap();
        let registration = host.publish_registration(&endpoint.path, None).unwrap();
        let cancel = CancellationToken::new();
        let server = tokio::spawn(serve(endpoint, host, cancel.clone()));
        Self {
            registration,
            _cleanup: cancel.clone().drop_guard(),
            cancel,
            server,
        }
    }
    pub(super) async fn stop(self) {
        self.cancel.cancel();
        self.server.await.unwrap().unwrap();
        self.registration.remove().unwrap();
    }
}

// Owning the whole endpoint keeps its Unix socket directory alive while serving.
async fn serve(
    endpoint: crate::endpoint::LocalEndpoint,
    host: Arc<Host>,
    cancel: CancellationToken,
) -> Result<(), String> {
    host.serve_local(endpoint.listener, cancel).await
}
