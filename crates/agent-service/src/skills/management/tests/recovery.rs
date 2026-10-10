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

use super::super::*;
use maka_event_log::{EventLog, root::RootNamespaces};
use maka_plugin_core::{
    composition::{
        Configuration as NodeConfiguration, Environment, ExecutionLocation, Input, InstanceId,
        Node, PackageUse, Target,
    },
    operation::{OperationCleanup, OperationOutcome, Receipt, RecoveryRef},
};
use maka_plugins::{
    call::{Attachments, Identity, Issuer},
    package::{MANIFEST_FILE, Package},
};
use maka_runtime_host::application::{
    Application, Binding, Invocation, Policy, RecoveryInvocation,
};
use serde_json::json;
use std::{
    path::Path,
    time::{Duration, Instant},
};
use uuid::Uuid;

const DEPLOYMENT: Uuid = Uuid::from_u128(91);
struct Grant(Issuer);
impl Policy for Grant {
    fn admit(&self, invocation: Invocation<'_>) -> Result<(), String> {
        self.0
            .owns(invocation.scope)
            .then_some(())
            .ok_or("foreign scope".into())
    }
    fn recover(&self, invocation: RecoveryInvocation<'_>) -> Result<(), String> {
        (self.0.evidence::<bool>(invocation.scope) == Some(&true))
            .then_some(())
            .ok_or("recovery denied".into())
    }
}
fn id(value: u128) -> InstanceId {
    InstanceId(vec![Uuid::from_u128(value)])
}
fn packages(source: &Path) -> (Input, Vec<Package>) {
    let provider = Package::new(BTreeMap::from([(
        MANIFEST_FILE.into(),
        serde_json::to_vec(&json!({
            "schemaVersion":2,"id":"skills","application":{"entries":{"host":{
                "implementation":{"kind":"native","implementation":IMPLEMENTATION},
                "configuration":configuration_schema(),"imports":{},
                "exports":{EXPORT:{"contract":contract(),"availability":"required"}}
            }}}
        }))
        .unwrap(),
    )]))
    .unwrap();
    let consumer = Package::new(BTreeMap::from([
        (
            MANIFEST_FILE.into(),
            serde_json::to_vec(&json!({
                "schemaVersion":2,"id":"consumer","application":{"entries":{"host":{
                    "implementation":{"kind":"javascript","module":"main.mjs","realm":"shared"},
                    "configuration":{"type":"object","additionalProperties":false},
                    "imports":{EXPORT:{"contract":contract(),"cardinality":maka_plugin_core::contract::Cardinality::One}},"exports":{}
                }}}
            }))
            .unwrap(),
        ),
        (
            "main.mjs".into(),
            b"export function activate() { return {}; }".to_vec(),
        ),
    ]))
    .unwrap();
    let node = |key, package: &Package, configuration, bindings| Node {
        key: Uuid::from_u128(key),
        package: package.application_ref().into(),
        use_package: PackageUse::Entry,
        configuration,
        bindings,
        parameters: Default::default(),
        disabled: false,
        overrides: vec![],
    };
    let input = Input {
        environment: Environment {
            location: ExecutionLocation::Host,
            presentation: None,
        },
        instances: BTreeMap::from([
            (
                "skills".into(),
                node(
                    1,
                    &provider,
                    NodeConfiguration {
                        parameters: Default::default(),
                        values: BTreeMap::from([
                            ("manage".into(), json!(true)),
                            (
                                "sources".into(),
                                json!([{"id":"local","label":"Local","path":source}]),
                            ),
                        ]),
                    },
                    BTreeMap::new(),
                ),
            ),
            (
                "caller".into(),
                node(
                    2,
                    &consumer,
                    Default::default(),
                    BTreeMap::from([(
                        EXPORT.into(),
                        vec![Target::Instance {
                            path: vec!["skills".into()],
                            export: EXPORT.into(),
                        }],
                    )]),
                ),
            ),
        ]),
        recovery: None,
        launch: None,
    };
    (input, vec![provider, consumer])
}
struct Pause {
    inner: Arc<dyn Handler>,
    ready: PathBuf,
    after: bool,
}
impl Handler for Pause {
    fn call(
        &self,
        method: String,
        input: Value,
        context: Context,
    ) -> BoxFuture<'static, Result<Value, String>> {
        let original = context.operation().cloned();
        let wait = method == "apply";
        let after = self.after;
        let ready = self.ready.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            if !wait {
                return inner.call(method, input, context).await;
            }
            if after {
                inner.call(method, input, context).await?;
            }
            let bytes = serde_json::to_vec(&original.unwrap()).unwrap();
            // The parent kills this actual Host process only after the selected
            // callback cut. No Host result/cleanup is returned or synthesized.
            std::fs::write(&ready, bytes).unwrap();
            futures_util::future::pending().await
        })
    }
}
struct Fixture {
    store: Arc<EventLog>,
    app: Application,
    issuer: Issuer,
}
impl Fixture {
    async fn open(dir: &Path, cut: Option<bool>) -> Self {
        let acquire = if cut.is_some() {
            RootOwner::create
        } else {
            RootOwner::open
        };
        let root = Arc::new(
            acquire(
                &dir.join("host"),
                &RootNamespaces {
                    ownership: dir.join("owners"),
                    control: dir.join("control"),
                },
            )
            .unwrap(),
        );
        let store = Arc::new(EventLog::for_root(root).await.unwrap());
        if store.load_application(DEPLOYMENT).await.unwrap().is_none() {
            let (input, packages) = packages(&dir.join("source"));
            store
                .commit_application(DEPLOYMENT, 0, input, vec![], packages)
                .await
                .unwrap();
        }
        let mut native = NativeRegistry::default();
        if let Some(after) = cut {
            let ready = dir.join("ready.json");
            native
                .insert(
                    IMPLEMENTATION.into(),
                    Arc::new(move |context, configuration| {
                        let ready = ready.clone();
                        Box::pin(async move {
                            let mut activation = activate(context, configuration).await;
                            let export = activation
                                .exports
                                .as_mut()
                                .unwrap()
                                .get_mut(EXPORT)
                                .unwrap();
                            export.value = Arc::new(Pause {
                                inner: export.value.clone(),
                                ready,
                                after,
                            });
                            activation
                        })
                    }),
                )
                .unwrap();
        } else {
            provide(&mut native).unwrap();
        }
        let issuer = Issuer::default();
        let app = Application::start_from_store(
            store.clone(),
            DEPLOYMENT,
            native,
            issuer.clone(),
            Arc::new(Grant(issuer.clone())),
            Default::default(),
        )
        .await
        .unwrap();
        app.ready().await.unwrap();
        Self { store, app, issuer }
    }
    fn scope(&self, recovery: bool) -> Scope {
        self.issuer
            .issue_with(
                Identity::Remote {
                    request_id: Uuid::new_v4(),
                },
                recovery,
                CancellationToken::new(),
            )
            .unwrap()
    }
    async fn binding(&self) -> Binding {
        let client = self.app.client();
        client
            .bind(client.activation(&id(2)).unwrap(), EXPORT.into())
            .await
            .unwrap()
            .remove(0)
    }
    async fn query(&self, method: &str, input: Value, recovery: bool) -> Result<Value, String> {
        self.app
            .client()
            .start(
                self.binding().await,
                method.into(),
                input,
                self.scope(recovery),
                Attachments::new().into(),
            )
            .await?
            .result()
            .await
    }
    async fn close(self) {
        self.app.shutdown().await.unwrap();
        drop(self.app);
        self.store.shutdown().await.unwrap();
    }
}
#[tokio::test]
#[ignore = "subprocess crash fixture; invoked by publication_results_survive_host_process_loss"]
async fn crash_worker() {
    let dir = PathBuf::from(std::env::var_os("MAKA_SKILL_CRASH_ROOT").expect("subprocess root"));
    let after = std::env::var("MAKA_SKILL_CRASH_CUT").unwrap() == "after";
    let fixture = Fixture::open(&dir, Some(after)).await;
    let page = fixture
        .query("list", json!({"query":"","cursor":null}), false)
        .await
        .unwrap();
    let preview = fixture
        .query(
            "preview",
            json!({"kind":"install","reference":"source:local:review","revision":page["revision"]}),
            false,
        )
        .await
        .unwrap();
    let prepared = fixture
        .app
        .client()
        .prepare_mutation(
            fixture.binding().await,
            "apply".into(),
            preview["review"].clone(),
            fixture.scope(false),
            Uuid::new_v4(),
            Attachments::new().into(),
        )
        .await
        .unwrap();
    let reference = prepared.reference().clone();
    let intent = fixture
        .store
        .record_application_intent(
            reference.source.clone(),
            prepared.consumer().instance().clone(),
            reference,
            Value::Null,
        )
        .await
        .unwrap();
    fixture
        .app
        .client()
        .mutate(prepared, intent)
        .await
        .unwrap()
        .result()
        .await;
    panic!("crash cut must not return");
}
struct Child(std::process::Child);
impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[tokio::test]
async fn publication_results_survive_host_process_loss_without_replaying_or_weakening_authority() {
    for cut in ["before", "after"] {
        let directory = tempfile::tempdir().unwrap();
        let dir = directory.path();
        std::fs::create_dir_all(dir.join("source/review")).unwrap();
        std::fs::write(
            dir.join("source/review/SKILL.md"),
            super::document("original"),
        )
        .unwrap();
        let log = std::fs::File::create(dir.join("child.log")).unwrap();
        let mut child = Child(
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "skills::management::tests::recovery::crash_worker",
                    "--nocapture",
                ])
                .env("MAKA_SKILL_CRASH_ROOT", dir)
                .env("MAKA_SKILL_CRASH_CUT", cut)
                .stdout(log.try_clone().unwrap())
                .stderr(log)
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(45);
        let reference: RecoveryRef = loop {
            if let Ok(bytes) = std::fs::read(dir.join("ready.json"))
                && let Ok(reference) = serde_json::from_slice(&bytes)
            {
                break reference;
            }
            assert!(
                child.0.try_wait().unwrap().is_none() && Instant::now() < deadline,
                "crash worker stopped or timed out: {}",
                std::fs::read_to_string(dir.join("child.log")).unwrap()
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        child.0.kill().unwrap();
        assert!(!child.0.wait().unwrap().success());
        let fixture = Fixture::open(dir, None).await;
        let observation = fixture
            .store
            .operation_observation(&reference)
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(observation.outcome, OperationOutcome::Unresolved));
        assert!(matches!(observation.cleanup, OperationCleanup::Unconfirmed));
        let namespace = crate::owner::namespace(
            &reference.source.state_domain,
            reference.source.deployment,
            &reference.namespace,
        );
        let data = dir.join("host/skill-libraries").join(namespace);
        let installed = data.join("skills/review");
        assert_eq!(installed.exists(), cut == "after");
        if installed.exists() {
            std::fs::remove_dir_all(&installed).unwrap();
        }
        // A later deployment activation can read the stable original namespace.
        let mut loaded = fixture
            .store
            .load_application(DEPLOYMENT)
            .await
            .unwrap()
            .unwrap();
        loaded
            .source
            .input
            .instances
            .get_mut("skills")
            .unwrap()
            .configuration
            .values
            .get_mut("sources")
            .unwrap()[0]["label"] = json!("Renamed collection");
        let next = fixture
            .store
            .commit_application(
                DEPLOYMENT,
                loaded.source.reference.revision,
                loaded.source.input,
                vec![],
                loaded.packages,
            )
            .await
            .unwrap();
        fixture.app.adopt_committed(next).await.unwrap();
        fixture.app.ready().await.unwrap();
        assert!(
            fixture
                .query("outcome", json!(reference), false)
                .await
                .is_err(),
            "method access is not recovery authority"
        );
        let value: contract::Outcome = serde_json::from_value(
            fixture
                .query("outcome", json!(reference), true)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(value.original.reference, reference);
        assert!(matches!(
            value.original.receipt,
            Receipt::Recorded {
                outcome: OperationOutcome::Unresolved,
                cleanup: OperationCleanup::Unconfirmed
            }
        ));
        assert_eq!(value.published.is_some(), cut == "after");
        if let Some(result) = value.published {
            assert!(
                matches!(result,ChangeResult::Applied {reference} if reference=="installed:review")
            );
        }
        assert!(
            !installed.exists(),
            "outcome lookup cannot reinstall a removed result"
        );
        if cut == "before" {
            assert_eq!(
                std::fs::read_dir(data.join("operations")).unwrap().count(),
                0,
                "query cannot reserve an unexecuted publication"
            );
        }
        for field in ["stateDomain", "deployment", "namespace", "fingerprint"] {
            let mut wrong = json!(reference);
            match field {
                "stateDomain" => wrong["source"][field] = json!("b".repeat(64)),
                "deployment" => wrong["source"][field] = json!(Uuid::new_v4()),
                "namespace" => wrong[field] = json!([Uuid::new_v4()]),
                _ => wrong[field] = json!("sha256-".to_owned() + &"b".repeat(64)),
            }
            assert!(
                fixture.query("outcome", wrong, true).await.is_err(),
                "must reject changed {field}"
            );
        }
        let after = fixture
            .store
            .operation_observation(&reference)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            after, observation,
            "domain evidence must not rewrite Host result or cleanup"
        );
        fixture.close().await;
    }
}
