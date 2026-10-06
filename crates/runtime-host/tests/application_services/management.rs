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
use maka_event_log::{
    EventLog,
    root::{RootNamespaces, RootOwner},
};
use maka_plugin_core::{
    deployment::{DeploymentRef, management::*},
    operation::{OperationCleanup, OperationOutcome, OperationReply, Receipt},
};
use maka_runtime_host::application::{Binding, management::DEPLOYMENT_MANAGER};

fn sources() -> (Input, Vec<Package>) {
    let contract = deployment_manager_contract();
    let provider = package(
        "management",
        EntryDeclaration {
            scene: None,
            implementation: Implementation::Native {
                implementation: DEPLOYMENT_MANAGER.into(),
            },
            configuration: json!({"type":"object","additionalProperties":false}),
            imports: BTreeMap::new(),
            exports: BTreeMap::from([(
                "Management".into(),
                Export {
                    contract: contract.clone(),
                    availability: Availability::Required,
                },
            )]),
        },
        None,
    );
    let consumer = package(
        "manager.client",
        EntryDeclaration {
            scene: None,
            implementation: Implementation::JavaScript {
                module: "host.mjs".into(),
                realm: Realm::Dedicated,
            },
            configuration: json!({"type":"object"}),
            exports: BTreeMap::new(),
            imports: BTreeMap::from([(
                "management".into(),
                Import {
                    contract,
                    cardinality: Cardinality::One,
                },
            )]),
        },
        Some("export function activate(){return {}; }".into()),
    );
    let mut consumer_node = node(&consumer, 2, false);
    consumer_node.bindings = BTreeMap::from([(
        "management".into(),
        vec![Target::Instance {
            path: vec!["provider".into()],
            export: "Management".into(),
        }],
    )]);
    (
        Input {
            environment: Environment {
                location: ExecutionLocation::Host,
                presentation: None,
            },
            instances: BTreeMap::from([
                ("provider".into(), node(&provider, 1, true)),
                ("a".into(), consumer_node),
            ]),
            launch: None,
        },
        vec![provider, consumer],
    )
}
struct Fixture {
    _directory: tempfile::TempDir,
    root: std::path::PathBuf,
    namespaces: RootNamespaces,
    store: Arc<EventLog>,
    app: Application,
    issuer: Issuer,
    source: DeploymentRef,
    binding: Binding,
}
impl Fixture {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("root");
        let namespaces = RootNamespaces {
            ownership: directory.path().join("owners"),
            control: directory.path().join("control"),
        };
        let store = Arc::new(
            EventLog::for_root(Arc::new(RootOwner::create(&root, &namespaces).unwrap()))
                .await
                .unwrap(),
        );
        let (input, packages) = sources();
        let source = store
            .commit_application(Uuid::new_v4(), 0, input, vec![], packages)
            .await
            .unwrap();
        let issuer = Issuer::default();
        let app = Application::start_from_store(
            store.clone(),
            source.deployment,
            NativeRegistry::default(),
            issuer.clone(),
            Arc::new(Grant(issuer.clone())),
            Limits::default(),
        )
        .await
        .unwrap();
        app.ready().await.unwrap();
        let client = app.client();
        let binding = client
            .bind(client.activation(&id(2)).unwrap(), "management".into())
            .await
            .unwrap()
            .remove(0);
        Self {
            _directory: directory,
            root,
            namespaces,
            store,
            app,
            issuer,
            source,
            binding,
        }
    }
    async fn mutate(&self, method: &str, input: Value, attachments: Attachments) -> OperationReply {
        let client = self.app.client();
        let prepared = client
            .prepare_mutation(
                self.binding.clone(),
                method.into(),
                input,
                scope(&self.issuer, true),
                Uuid::new_v4(),
                attachments.into(),
            )
            .await
            .unwrap();
        let intent = self
            .store
            .record_application_intent(
                prepared.reference().source.clone(),
                id(2),
                prepared.reference().clone(),
            )
            .await
            .unwrap();
        let completed = client
            .mutate(prepared, intent)
            .await
            .unwrap()
            .completion()
            .await;
        completed.cleanup.unwrap();
        completed.reply
    }
    async fn read(&self) -> ManagedDeployment {
        let result = self
            .app
            .client()
            .start(
                self.binding.clone(),
                "read".into(),
                json!({}),
                scope(&self.issuer, true),
                Default::default(),
            )
            .await
            .unwrap()
            .completion()
            .await;
        result.cleanup.unwrap();
        serde_json::from_value(result.result.unwrap()).unwrap()
    }
    async fn effective(&self, source: &DeploymentRef) {
        tokio::time::timeout(Duration::from_secs(10), async {
            while self.app.client().snapshot().effective.as_ref() != Some(source) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
    async fn close(self) {
        self.app.shutdown().await.unwrap();
        drop(self.app);
        drop(self.binding);
        Arc::try_unwrap(self.store)
            .ok()
            .unwrap()
            .close()
            .await
            .unwrap();
        drop(RootOwner::open(&self.root, &self.namespaces).unwrap());
    }
}
struct Bundle(Vec<u8>);
impl ReadResource for Bundle {
    fn resource_type(&self) -> ResourceType {
        package_bundle_type()
    }
    fn bytes(&self) -> usize {
        self.0.len()
    }
    fn metadata(&self) -> Value {
        json!({})
    }
    fn read(&self, range: std::ops::Range<usize>) -> Result<Vec<u8>> {
        Ok(self.0[range].to_vec())
    }
}
fn resource(bytes: Vec<u8>) -> Attachments {
    BTreeMap::from([(
        "package".into(),
        Arc::new(Bundle(bytes)) as Arc<dyn ReadResource>,
    )])
}
fn returned(reply: &OperationReply) -> Value {
    let Receipt::Recorded {
        outcome: OperationOutcome::Returned { value },
        cleanup: OperationCleanup::Confirmed,
    } = &reply.receipt
    else {
        panic!("{reply:?}")
    };
    value.clone()
}
fn extension() -> Package {
    package(
        "installed.echo",
        entry(
            true,
            Implementation::JavaScript {
                module: "host.mjs".into(),
                realm: Realm::Dedicated,
            },
            MethodMode::Query,
        ),
        Some(format!(
            "export function activate(){{return {{echo:{{contract:{},service:['service']}}}};}}; export const service={{echo(input){{return 'installed:'+input;}}}};",
            serde_json::to_string(&contract(MethodMode::Query)).unwrap(),
        )),
    )
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn installed_bytes_do_not_activate_until_explicit_cas_replace_and_original_receipt_survives()
{
    let f = Fixture::new().await;
    let original_epoch = f.binding.registration_epoch();
    let extension = extension();
    let install = f
        .mutate(
            "install",
            json!({"expectedRevision":1}),
            resource(extension.to_bundle()),
        )
        .await;
    let installed: PackageInstalled =
        serde_json::from_value(returned(&install)["value"].clone()).unwrap();
    assert_eq!(installed.package, extension.application_ref());
    assert_eq!(installed.source.revision, 2);
    f.effective(&installed.source).await;
    assert!(f.app.client().activation(&id(7)).is_err());
    assert_eq!(f.binding.registration_epoch(), original_epoch);
    let current = f.read().await;
    assert!(current.packages.contains(&extension.application_ref()));
    let mut input = current.source.input;
    input
        .instances
        .insert("installed".into(), node(&extension, 7, true));
    let client_node = input.instances.get_mut("a").unwrap();
    // The existing manager client intentionally keeps its old imports. A new
    // consumer gets the installed export through an explicit package/binding.
    assert_eq!(client_node.key, Uuid::from_u128(2));
    let consumer = consumer();
    let mut extra = node(&consumer, 8, false);
    extra.bindings.get_mut("echo").unwrap()[0] = Target::Instance {
        path: vec!["installed".into()],
        export: "echo".into(),
    };
    // Install that consumer's immutable bytes independently as well.
    let install_consumer = f
        .mutate(
            "install",
            json!({"expectedRevision":2}),
            resource(consumer.to_bundle()),
        )
        .await;
    let installed_consumer: PackageInstalled =
        serde_json::from_value(returned(&install_consumer)["value"].clone()).unwrap();
    f.effective(&installed_consumer.source).await;
    let current = f.read().await;
    input.instances.insert("new.client".into(), extra);
    let replacement = f
        .mutate(
            "replace",
            serde_json::to_value(DeploymentReplace {
                expected_revision: 3,
                input,
                remote_exports: vec![],
                packages: current.packages,
            })
            .unwrap(),
            Attachments::new(),
        )
        .await;
    let source: DeploymentRef =
        serde_json::from_value(returned(&replacement)["value"].clone()).unwrap();
    f.effective(&source).await;
    let client = f.app.client();
    let binding = client
        .bind(client.activation(&id(8)).unwrap(), "echo".into())
        .await
        .unwrap()
        .remove(0);
    assert_eq!(
        client
            .start(
                binding,
                "echo".into(),
                json!("value"),
                scope(&f.issuer, true),
                Default::default()
            )
            .await
            .unwrap()
            .result()
            .await
            .unwrap(),
        json!("installed:value")
    );
    assert_eq!(f.read().await.source.reference, source);
    assert_eq!(
        f.store
            .operation_observation(&install.reference)
            .await
            .unwrap()
            .unwrap()
            .outcome,
        OperationOutcome::Returned {
            value: returned(&install)
        }
    );
    assert_eq!(
        RootOwner::open(&f.root, &f.namespaces)
            .err()
            .unwrap()
            .kind(),
        std::io::ErrorKind::WouldBlock
    );
    drop(client);
    f.close().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejected_bundle_cas_role_and_unknown_revision_leave_original_plan_healthy() {
    let f = Fixture::new().await;
    let invalid = f
        .mutate(
            "install",
            json!({"expectedRevision":1}),
            resource(b"{}".to_vec()),
        )
        .await;
    assert_eq!(
        returned(&invalid),
        json!({"status":"rejected","reason":"invalid_bundle"})
    );
    assert_eq!(f.read().await.source.reference, f.source);
    let current = f.read().await;
    let mut input = current.source.input;
    input.environment = Environment {
        location: ExecutionLocation::Ui,
        presentation: Some(maka_plugin_core::composition::PresentationTarget::Desktop),
    };
    let refused = f
        .mutate(
            "replace",
            serde_json::to_value(DeploymentReplace {
                expected_revision: 1,
                input,
                remote_exports: vec![],
                packages: current.packages,
            })
            .unwrap(),
            Attachments::new(),
        )
        .await;
    assert_eq!(
        returned(&refused),
        json!({"status":"rejected","reason":"execution_environment"})
    );
    assert_eq!(f.read().await.source.reference, f.source);
    let installed = f
        .mutate(
            "install",
            json!({"expectedRevision":1}),
            resource(extension().to_bundle()),
        )
        .await;
    let newer: PackageInstalled =
        serde_json::from_value(returned(&installed)["value"].clone()).unwrap();
    f.effective(&newer.source).await;
    let conflict = f
        .mutate(
            "install",
            json!({"expectedRevision":1}),
            resource(extension().to_bundle()),
        )
        .await;
    assert_eq!(
        returned(&conflict),
        json!({"status":"conflict","currentRevision":2})
    );
    assert_eq!(f.read().await.source.reference, newer.source);
    let mut read = f.read().await;
    read.packages.push(extension().application_ref());
    read.packages.last_mut().unwrap().revision = format!("sha256-{}", "0".repeat(64));
    let refused = f
        .mutate(
            "replace",
            serde_json::to_value(DeploymentReplace {
                expected_revision: 2,
                input: read.source.input,
                remote_exports: vec![],
                packages: read.packages,
            })
            .unwrap(),
            Attachments::new(),
        )
        .await;
    assert_eq!(
        returned(&refused),
        json!({"status":"rejected","reason":"package_unavailable"})
    );
    assert_eq!(f.read().await.source.reference, newer.source);
    f.close().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manager_availability_and_schema_do_not_grant_authority_or_accept_guest_root() {
    let f = Fixture::new().await;
    for input in [
        json!({"expectedRevision":1,"root":"/tmp/foreign"}),
        json!({"expectedRevision":1,"deployment":Uuid::new_v4()}),
    ] {
        assert!(
            f.app
                .client()
                .prepare_mutation(
                    f.binding.clone(),
                    "install".into(),
                    input,
                    scope(&f.issuer, true),
                    Uuid::new_v4(),
                    resource(extension().to_bundle()).into()
                )
                .await
                .is_err()
        );
    }
    assert!(
        f.app
            .client()
            .prepare_mutation(
                f.binding.clone(),
                "install".into(),
                json!({"expectedRevision":1}),
                scope(&f.issuer, false),
                Uuid::new_v4(),
                resource(extension().to_bundle()).into()
            )
            .await
            .is_err()
    );
    assert_eq!(f.read().await.source.reference, f.source);
    let (input, packages) = sources();
    assert!(
        Application::start(
            input,
            packages,
            NativeRegistry::default(),
            f.issuer.clone(),
            Arc::new(Grant(f.issuer.clone())),
            Limits::default()
        )
        .is_err()
    );
    f.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_installation_commits_once_and_records_the_other_cas_as_conflict() {
    let f = Fixture::new().await;
    let extension = extension();
    let (a, b) = tokio::join!(
        f.mutate(
            "install",
            json!({"expectedRevision":1}),
            resource(extension.to_bundle())
        ),
        f.mutate(
            "install",
            json!({"expectedRevision":1}),
            resource(extension.to_bundle())
        ),
    );
    let a = returned(&a);
    let b = returned(&b);
    assert!(
        (a["status"] == "committed" && b == json!({"status":"conflict","currentRevision":2}))
            || (b["status"] == "committed"
                && a == json!({"status":"conflict","currentRevision":2}))
    );
    assert_eq!(f.read().await.source.reference.revision, 2);
    f.close().await;
}
