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

//! Actual separately launched Application Host, not an in-process bridge fixture.
#[allow(dead_code)]
#[path = "../../application-bridge/tests/support/mod.rs"]
mod support;
use maka_application_bridge::{
    Grant,
    transport::{
        Call, Catalogue, Reply, Source, ipc,
        native::{Connector as _, Stream},
        process::{Bootstrap, Control},
    },
};
use maka_event_log::{
    EventLog,
    root::{RootNamespaces, RootOwner},
};
use maka_plugin_core::{
    composition::PresentationTarget,
    contract::{Interface, MethodMode},
    deployment::DeploymentRef,
    operation::{OperationCleanup, OperationOutcome, OperationReply, Receipt},
    package::EntryTarget,
};
use maka_plugins::call::{Attachments, ReadResource, ResourceEncoding};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap, ops::Range, path::PathBuf, process::Stdio, sync::Arc, time::Duration,
};
use tokio::{
    process::{Child, ChildStdin, ChildStdout, Command},
    runtime::Runtime,
};
use uuid::Uuid;
struct Fixture {
    timer: Runtime,
    directory: tempfile::TempDir,
    namespaces: RootNamespaces,
    caller: Arc<EventLog>,
    source: DeploymentRef,
    host: Uuid,
}
type ChannelCall = Call<tokio::io::ReadHalf<Stream>, tokio::io::WriteHalf<Stream>>;
struct HostProcess {
    child: Child,
    control: Control<ChildStdout, ChildStdin>,
    catalogue: Catalogue,
    connector: ipc::Connector,
}

impl Fixture {
    fn new() -> Self {
        let timer = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let directory = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        let namespaces = RootNamespaces::for_current_account().unwrap();
        let open = |name| {
            let owner =
                Arc::new(RootOwner::create(&directory.path().join(name), &namespaces).unwrap());
            Arc::new(timer.block_on(EventLog::for_root(owner)).unwrap())
        };
        let caller = open("caller");
        let host_store = open("host");
        let host = Uuid::new_v4();
        let (mut input, originals) = support::host();
        let mut contract = support::business();
        let Interface::Service { methods } = &mut contract.interface else {
            unreachable!()
        };
        for name in ["read", "write"] {
            methods
                .get_mut(name)
                .unwrap()
                .resources
                .insert("text".into(), maka_ui::text::text_resource_type());
        }
        let mut feed = methods["read"].clone();
        feed.mode = MethodMode::Subscription;
        feed.resources.clear();
        methods.insert("feed".into(), feed);
        let mut packages = Vec::new();
        for old in originals {
            if old.manifest().id == "local.receipts" {
                packages.push(old);
                continue;
            }
            let mut declaration = old.manifest().application.clone().unwrap();
            let entry = declaration.entries.get_mut(&EntryTarget::Host).unwrap();
            for import in entry.imports.values_mut() {
                if import.contract.name == contract.name {
                    import.contract = contract.clone();
                }
            }
            for export in entry.exports.values_mut() {
                if export.contract.name == contract.name {
                    export.contract = contract.clone();
                }
            }
            let source=(old.manifest().id=="local.business").then(||format!(r#"
                let writes=0;
                export function activate(){{return {{Counter:{{contract:{},service:['service']}}}};}}
                export const service={{
                    async read(input,observation,context){{
                        const descriptor=await context.invoke({{operation:'metadata',slot:'text'}});
                        const first=await context.invoke({{operation:'read',slot:'text',start:0,end:Math.min(5,descriptor.bytes)}});
                        return {{bytes:descriptor.bytes,owner:descriptor.metadata.owner,first,inputLength:typeof input==='string'?input.length:0,writes}};
                    }},
                    async write(input,observation,context){{
                        const first=await context.invoke({{operation:'read',slot:'text',start:0,end:5}});
                        if(first.join(',')!=='116,111,107,101,110')throw Error('original snapshot differs');
                        if(input==='hold'){{while(true){{}}}}
                        writes++;return {{accepted:true}};
                    }},
                    async feed(input,observation,context){{for(let i=0;i<35;i++)await context.invoke({{operation:'emit',value:{{i}}}});return {{done:true}};}}
                }};
            "#,serde_json::to_string(&contract).unwrap()));
            let package = support::counter::package(&old.manifest().id, declaration, source);
            for node in input.instances.values_mut() {
                if node.package == old.application_ref().into() {
                    node.package = package.application_ref().into();
                }
            }
            packages.push(package);
        }
        timer
            .block_on(host_store.commit_application(host, 0, input, vec![], packages))
            .unwrap();
        timer.block_on(host_store.shutdown()).unwrap();
        drop(host_store);
        let (input, packages, observations) = support::ui(PresentationTarget::Desktop);
        let source = timer
            .block_on(caller.commit_application(Uuid::new_v4(), 0, input, observations, packages))
            .unwrap();
        Self {
            timer,
            directory,
            namespaces,
            caller,
            source,
            host,
        }
    }
    fn host_path(&self) -> PathBuf {
        self.directory.path().join("host")
    }
    fn source(&self) -> Source {
        Source {
            deployment: self.source.clone(),
            consumer: support::identity(2),
            request: Uuid::new_v4(),
        }
    }
    fn launch(&self) -> (Child, Control<ChildStdout, ChildStdin>, Bootstrap) {
        let _entered = self.timer.enter();
        let mut child = Command::new(env!("CARGO_BIN_EXE_maka"))
            .args(["application", "serve"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let nonce = Uuid::new_v4();
        #[cfg(unix)]
        let address = self
            .directory
            .path()
            .join(format!("{}.sock", &nonce.to_string()[..8]));
        #[cfg(windows)]
        let address = PathBuf::from(format!(r"\\.\pipe\maka-application-test-{nonce}"));
        let bootstrap = Bootstrap {
            native: Vec::new(),
            root: self.host_path(),
            deployment: self.host,
            namespaces: self.namespaces.clone(),
            source: self.source.clone(),
            consumers: [support::identity(2)].into(),
            consumer: support::identity(13),
            imports: BTreeMap::from([
                (
                    "business".into(),
                    Grant {
                        methods: ["read".into(), "write".into(), "feed".into()].into(),
                        recovery: false,
                    },
                ),
                (
                    "receipts".into(),
                    Grant {
                        methods: ["read".into()].into(),
                        recovery: true,
                    },
                ),
            ]),
            address: address.clone(),
            nonce,
        };
        let control = Control::new(
            child.stdout.take().unwrap(),
            child.stdin.take().unwrap(),
            nonce,
        )
        .unwrap();
        (child, control, bootstrap)
    }
    fn start_host(&self) -> HostProcess {
        let (child, mut control, bootstrap) = self.launch();
        let catalogue = self.timer.block_on(async {
            tokio::time::timeout(Duration::from_secs(15), control.start(&bootstrap))
                .await
                .unwrap()
                .unwrap()
        });
        HostProcess {
            child,
            control,
            catalogue,
            connector: ipc::Connector::new(bootstrap.address, bootstrap.nonce).unwrap(),
        }
    }
    fn open(
        &self,
        host: &HostProcess,
        endpoint: &str,
        method: &str,
        input: Value,
        resources: Attachments,
        source: Source,
    ) -> ChannelCall {
        let stream = self.timer.block_on(async {
            tokio::time::timeout(
                Duration::from_secs(10),
                host.connector.connect(Default::default()),
            )
            .await
            .unwrap()
            .unwrap()
        });
        let (reader, writer) = tokio::io::split(stream);
        Call::new(
            reader,
            writer,
            host.catalogue.select(endpoint).unwrap(),
            source,
            method.into(),
            input,
            resources,
        )
    }
    fn spawn(
        &self,
        endpoint: &str,
        method: &str,
        input: Value,
        resources: Attachments,
        source: Source,
    ) -> (HostProcess, ChannelCall) {
        let host = self.start_host();
        let call = self.open(&host, endpoint, method, input, resources, source);
        (host, call)
    }
    fn wait(&self, mut host: HostProcess) {
        self.timer.block_on(async {
            tokio::time::timeout(Duration::from_secs(15), host.control.close())
                .await
                .unwrap()
                .unwrap();
            assert!(
                tokio::time::timeout(Duration::from_secs(15), host.child.wait())
                    .await
                    .unwrap()
                    .unwrap()
                    .success()
            );
        });
        drop(RootOwner::open(&self.host_path(), &self.namespaces).unwrap());
    }
    fn cleanup(self) {
        self.timer.block_on(self.caller.shutdown()).unwrap();
        drop(self.caller);
    }
}
struct Snapshot {
    data: String,
    metadata: Value,
}
impl ReadResource for Snapshot {
    fn resource_type(&self) -> maka_plugin_core::contract::ResourceType {
        maka_ui::text::text_resource_type()
    }
    fn encoding(&self) -> ResourceEncoding {
        ResourceEncoding::Utf8
    }
    fn bytes(&self) -> usize {
        self.data.len()
    }
    fn metadata(&self) -> Value {
        self.metadata.clone()
    }
    fn read(&self, range: Range<usize>) -> Result<Vec<u8>, String> {
        self.data
            .get(range)
            .map(|s| s.as_bytes().to_vec())
            .ok_or("UTF-8 boundary".into())
    }
}
fn text(data: String, metadata: Value) -> Attachments {
    BTreeMap::from([(
        "text".into(),
        Arc::new(Snapshot { data, metadata }) as Arc<dyn ReadResource>,
    )])
}
fn token() -> Attachments {
    text("token".into(), json!({"owner":"native-editor"}))
}
fn finished(reply: Reply) -> Value {
    match reply {
        Reply::Finished { result, cleanup } => {
            cleanup.unwrap();
            result.unwrap()
        }
        other => panic!("unexpected {other:?}"),
    }
}
#[test]
fn private_pipes_move_actual_utf8_snapshots_and_metadata() {
    let f = Fixture::new();
    let data = format!("token{}", "你🙂".repeat(20000));
    let (child, mut call) = f.spawn(
        "business",
        "read",
        json!("q".repeat(400_000)),
        text(
            data.clone(),
            json!({"owner":"actual-editor","large":"m".repeat(400_000)}),
        ),
        f.source(),
    );
    f.timer.block_on(async {
        call.start().await.unwrap();
        let value=finished(call.receive().await.unwrap());
        assert_eq!(value,json!({"bytes":data.len(),"owner":"actual-editor","first":[116,111,107,101,110],"inputLength":400_000,"writes":0}));
        assert!(call.cleanup_confirmed());
    });
    drop(call);
    f.wait(child);
    f.cleanup();
}
#[test]
fn private_mutation_requires_actual_caller_sql_and_recovers_original_receipt_after_reopen() {
    let f = Fixture::new();
    let source = f.source();
    let (child, mut call) = f.spawn("business", "write", Value::Null, token(), source.clone());
    let reply = f.timer.block_on(async {
        call.start().await.unwrap();
        assert!(matches!(
            call.receive().await.unwrap(),
            Reply::Prepared { .. }
        ));
        assert!(
            f.caller
                .application_intents(f.source.deployment, support::identity(2), 0, 10)
                .await
                .unwrap()
                .is_empty()
        );
        let recorded = f
            .caller
            .record_application_intent(
                f.source.clone(),
                support::identity(2),
                call.reference().unwrap().clone(),
            )
            .await
            .unwrap();
        let history = f
            .caller
            .application_intents(f.source.deployment, support::identity(2), 0, 10)
            .await
            .unwrap();
        assert_eq!(history[0].reference, *call.reference().unwrap());
        match call.commit(recorded).await.unwrap() {
            Reply::Mutation { reply, cleanup } => {
                cleanup.unwrap();
                *reply
            }
            other => panic!("unexpected {other:?}"),
        }
    });
    assert!(call.cleanup_confirmed());
    assert!(!call.holds_caller_intent());
    assert!(matches!(
        reply.receipt,
        Receipt::Recorded {
            cleanup: OperationCleanup::Confirmed,
            ..
        }
    ));
    drop(call);
    f.wait(child);
    let (child, mut recovery) = f.spawn(
        "receipts",
        "read",
        serde_json::to_value(&reply.reference).unwrap(),
        Attachments::new(),
        f.source(),
    );
    let observed = f.timer.block_on(async {
        recovery.start().await.unwrap();
        finished(recovery.receive().await.unwrap())
    });
    assert_eq!(
        serde_json::from_value::<OperationReply>(observed).unwrap(),
        reply
    );
    drop(recovery);
    f.wait(child);
    f.cleanup();
}
#[test]
fn private_subscription_pulls_in_order_and_cancel_drains_original_executor() {
    let f = Fixture::new();
    let (child, mut call) = f.spawn(
        "business",
        "feed",
        Value::Null,
        Attachments::new(),
        f.source(),
    );
    f.timer.block_on(async {
        call.start().await.unwrap();
        assert!(matches!(call.receive().await.unwrap(), Reply::Subscribed));
        for i in 0..3 {
            assert!(
                matches!(call.pull().await.unwrap(),Reply::Item {value} if value==json!({"i":i}))
            );
        }
        let reply = call.cancel().await.unwrap();
        assert!(reply.cleanup().unwrap().is_ok());
        assert!(call.cleanup_confirmed());
    });
    drop(call);
    f.wait(child);
    f.cleanup();
}
#[test]
fn private_preparation_abort_has_no_business_journal_effect() {
    let f = Fixture::new();
    let (child, mut call) = f.spawn("business", "write", Value::Null, token(), f.source());
    let reference = f.timer.block_on(async {
        call.start().await.unwrap();
        call.receive().await.unwrap();
        let reference = call.reference().unwrap().clone();
        assert!(matches!(
            call.cancel().await.unwrap(),
            Reply::Aborted {
                cleanup: Ok(()),
                ..
            }
        ));
        reference
    });
    drop(call);
    f.wait(child);
    let owner = Arc::new(RootOwner::open(&f.host_path(), &f.namespaces).unwrap());
    let store = f.timer.block_on(EventLog::for_root(owner)).unwrap();
    assert!(
        f.timer
            .block_on(store.operation_observation(&reference))
            .unwrap()
            .is_none()
    );
    f.timer.block_on(store.close()).unwrap();
    f.cleanup();
}
#[test]
fn private_peer_json_cannot_change_the_approved_ui_consumer() {
    let f = Fixture::new();
    let mut source = f.source();
    source.consumer = support::identity(13);
    let (child, mut call) = f.spawn("business", "write", Value::Null, token(), source);
    let error = f.timer.block_on(call.start()).unwrap_err();
    assert!(
        error.contains("caller is not approved for this exact source"),
        "{error}"
    );
    assert!(call.cleanup_confirmed());
    drop(call);
    f.wait(child);
    f.cleanup();
}

#[test]
fn private_host_loss_retains_real_caller_token_and_reopens_only_original_receipt() {
    use sqlx::{
        Connection,
        sqlite::{SqliteConnectOptions, SqliteConnection},
    };
    let f = Fixture::new();
    let source = f.source();
    let (mut child, mut call) = f.spawn("business", "write", json!("hold"), token(), source);
    let reference=f.timer.block_on(async {
        call.start().await.unwrap();assert!(matches!(call.receive().await.unwrap(),Reply::Prepared {..}));
        let reference=call.reference().unwrap().clone();
        let recorded=f.caller.record_application_intent(f.source.clone(),support::identity(2),reference.clone()).await.unwrap();
        let options=SqliteConnectOptions::new().filename(f.host_path().join(maka_event_log::root::ROOT_DATABASE)).read_only(true);
        let mut reader=SqliteConnection::connect_with(&options).await.unwrap();
        let dispatched=call.commit(recorded);tokio::pin!(dispatched);
        tokio::select! {
            result=&mut dispatched=>panic!("held operation completed before loss: {result:?}"),
            result=tokio::time::timeout(Duration::from_secs(10),async {
                loop {
                    let exists:i64=sqlx::query_scalar("SELECT count(*) FROM application_operations WHERE operation=?").bind(reference.operation.to_string()).fetch_one(&mut reader).await.unwrap();
                    if exists==1 {break;}
                    tokio::task::yield_now().await;
                }
            })=>result.unwrap(),
        }
        reader.close().await.unwrap();
        child.child.start_kill().unwrap();assert!(!tokio::time::timeout(Duration::from_secs(10),child.child.wait()).await.unwrap().unwrap().success());
        assert!(dispatched.await.is_err());reference
    });
    assert!(call.holds_caller_intent());
    assert!(!call.cleanup_confirmed());
    let (reopened, mut recovery) = f.spawn(
        "receipts",
        "read",
        serde_json::to_value(&reference).unwrap(),
        Attachments::new(),
        f.source(),
    );
    let reply = f.timer.block_on(async {
        recovery.start().await.unwrap();
        let value = finished(recovery.receive().await.unwrap());
        serde_json::from_value::<OperationReply>(value).unwrap()
    });
    assert_eq!(reply.reference, reference);
    assert!(matches!(
        reply.receipt,
        Receipt::Recorded {
            outcome: OperationOutcome::Unresolved,
            cleanup: OperationCleanup::Unconfirmed
        }
    ));
    drop(recovery);
    f.wait(reopened);
    // A successful fresh lookup never clears the lost dispatch owner's token.
    assert!(call.holds_caller_intent());
    assert!(!call.cleanup_confirmed());
    let Fixture {
        timer,
        directory,
        namespaces,
        caller,
        ..
    } = f;
    timer
        .block_on(Arc::try_unwrap(caller).ok().unwrap().close())
        .unwrap();
    assert_eq!(
        RootOwner::open(&directory.path().join("caller"), &namespaces)
            .err()
            .unwrap()
            .kind(),
        std::io::ErrorKind::WouldBlock
    );
    // Test teardown destroys this failed owner; that destruction is explicitly
    // not a cleanup receipt, replay permission or product recovery mechanism.
    drop(call);
    drop(RootOwner::open(&directory.path().join("caller"), &namespaces).unwrap());
}

#[test]
fn shared_host_keeps_actual_javascript_state_across_independent_channels() {
    let f = Fixture::new();
    let host = f.start_host();
    let original_epoch = host
        .catalogue
        .select("business")
        .unwrap()
        .description()
        .registration_epoch;
    let mut write = f.open(&host, "business", "write", Value::Null, token(), f.source());
    f.timer.block_on(async {
        write.start().await.unwrap();
        assert!(matches!(
            write.receive().await.unwrap(),
            Reply::Prepared { .. }
        ));
        let recorded = f
            .caller
            .record_application_intent(
                f.source.clone(),
                support::identity(2),
                write.reference().unwrap().clone(),
            )
            .await
            .unwrap();
        assert!(matches!(
            write.commit(recorded).await.unwrap(),
            Reply::Mutation {
                cleanup: Ok(()),
                ..
            }
        ));
    });
    drop(write);
    let mut read = f.open(&host, "business", "read", Value::Null, token(), f.source());
    let value = f.timer.block_on(async {
        read.start().await.unwrap();
        finished(read.receive().await.unwrap())
    });
    assert_eq!(value["writes"], 1);
    assert_eq!(
        host.catalogue
            .select("business")
            .unwrap()
            .description()
            .registration_epoch,
        original_epoch
    );
    drop(read);
    f.wait(host);
    f.cleanup();
}

#[test]
fn failed_start_preserves_the_same_child_actual_closed_acknowledgement() {
    let f = Fixture::new();
    for missing_import in [true, false] {
        let (mut child, mut control, mut bootstrap) = f.launch();
        if missing_import {
            bootstrap.imports.insert(
                "missing".into(),
                Grant {
                    methods: ["read".into()].into(),
                    recovery: false,
                },
            );
        } else {
            bootstrap.deployment = Uuid::new_v4();
        }
        f.timer.block_on(async {
            assert!(
                tokio::time::timeout(Duration::from_secs(15), control.start(&bootstrap))
                    .await
                    .unwrap()
                    .unwrap_err()
                    .contains("closed before publishing")
            );
            tokio::time::timeout(Duration::from_secs(2), control.close())
                .await
                .unwrap()
                .unwrap();
            control.close().await.unwrap();
            // Startup remains a failure, independently of the real cleanup Ack.
            assert!(
                !tokio::time::timeout(Duration::from_secs(15), child.wait())
                    .await
                    .unwrap()
                    .unwrap()
                    .success()
            );
        });
        drop(RootOwner::open(&f.host_path(), &f.namespaces).unwrap());
    }
    f.cleanup();
}

fn seed_abandoned_model(f: &Fixture) -> maka_runtime::event::Invocation {
    use maka_runtime::event::{EventWrite, Fact, Invocation, LogScope, RuntimeEvent};
    let invocation = Invocation {
        session_id: "abandoned-session".into(),
        turn_id: "abandoned-turn".into(),
        run_id: "abandoned-run".into(),
        invocation_id: "abandoned-invocation".into(),
    };
    f.timer.block_on(async {
        let owner = Arc::new(RootOwner::open(&f.host_path(), &f.namespaces).unwrap());
        let store = EventLog::for_root(owner).await.unwrap();
        let opening = RuntimeEvent::new(
            invocation.clone(),
            Fact::InvocationOpened {
                configuration: None,
                input: maka_runtime::input::InvocationInput::Message {
                    source_messages: Vec::new(),
                    content: "accepted before Host exit".into(),
                    request_fingerprint: Some("original-request".into()),
                },
            },
        );
        store
            .append(&EventWrite::plain(opening).unwrap())
            .await
            .unwrap();
        let prefix = store
            .scoped_prefix(
                LogScope::Session {
                    id: invocation.session_id.clone(),
                },
                10,
                128 * 1024,
            )
            .await
            .unwrap();
        let requested = RuntimeEvent::new(
            invocation.clone(),
            Fact::ModelRequested {
                item_acceptance: false,
                step_id: "original-step".into(),
                purpose: maka_runtime::context::ModelPurpose::Main,
                model_id: "original-model".into(),
                context: None,
                source_scope: prefix.scope,
                source_high_water: prefix.high_water,
                source_digest: prefix.digest,
                effective_source_digest: None,
                input_digest: "original-input".into(),
                route_identity: "original-route".into(),
                checkpoint_event_id: None,
            },
        );
        store
            .append(&EventWrite::plain(requested).unwrap())
            .await
            .unwrap();
        store.shutdown().await.unwrap();
    });
    invocation
}

#[test]
fn private_host_reconciles_abandoned_execution_before_publishing_and_never_replays_it() {
    use maka_runtime::event::{Fact, InvocationOutcome, ModelInterruption};
    let f = Fixture::new();
    let invocation = seed_abandoned_model(&f);
    for _ in 0..2 {
        let host = f.start_host();
        // Ready must already imply reconciliation. Read the canonical SQLite
        // facts while the original child still exclusively owns this Root.
        f.timer.block_on(async {
            use sqlx::Connection;
            let options = sqlx::sqlite::SqliteConnectOptions::new()
                .filename(f.host_path().join(maka_event_log::root::ROOT_DATABASE))
                .read_only(true);
            let mut reader = sqlx::SqliteConnection::connect_with(&options).await.unwrap();
            let rows: Vec<String> = sqlx::query_scalar("SELECT event_json FROM event_log WHERE invocation_id=? ORDER BY sequence")
                .bind(&invocation.invocation_id).fetch_all(&mut reader).await.unwrap();
            let events: Vec<maka_runtime::event::RuntimeEvent> = rows.iter().map(|row| serde_json::from_str(row).unwrap()).collect();
            assert_eq!(events.len(), 4, "Ready cannot expose an abandoned model as running or replay its request");
            assert!(events.iter().all(|event| event.invocation == invocation));
            assert!(matches!(&events[2].fact, Fact::ModelInterrupted {step_id,status:ModelInterruption::Failed,..} if step_id=="original-step"));
            assert!(matches!(&events[3].fact, Fact::InvocationEnded {outcome:InvocationOutcome::Failed {class,..}} if class=="host_interrupted"));
            reader.close().await.unwrap();
        });
        f.wait(host);
    }
    f.cleanup();
}

#[test]
fn private_host_recovery_failure_closes_original_sql_owner_without_publishing_ready() {
    use sqlx::Connection;
    let f = Fixture::new();
    let invocation = seed_abandoned_model(&f);
    f.timer.block_on(async {
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(f.host_path().join(maka_event_log::root::ROOT_DATABASE));
        let mut raw = sqlx::SqliteConnection::connect_with(&options).await.unwrap();
        // Valid SQLite and JSON, but unusable original execution identity.
        sqlx::query("UPDATE event_log SET event_json=json_set(event_json,'$.invocation.session_id',NULL) WHERE invocation_id=? AND kind='invocation_opened'")
            .bind(&invocation.invocation_id).execute(&mut raw).await.unwrap();
        raw.close().await.unwrap();
        let owner = Arc::new(RootOwner::open(&f.host_path(), &f.namespaces).unwrap());
        let store = EventLog::for_root(owner).await.unwrap();
        let pending = store.unfinished_invocations(10_000).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert!(store.invocation_recovery(&pending[0], 10_000, 8 * 1024 * 1024).await.is_err(), "fixture fails execution recovery, not SQL startup");
        store.shutdown().await.unwrap();
    });
    let (mut child, mut control, bootstrap) = f.launch();
    f.timer.block_on(async {
        assert!(
            tokio::time::timeout(Duration::from_secs(15), control.start(&bootstrap))
                .await
                .unwrap()
                .unwrap_err()
                .contains("closed before publishing")
        );
        tokio::time::timeout(Duration::from_secs(2), control.close())
            .await
            .unwrap()
            .unwrap();
        assert!(
            !tokio::time::timeout(Duration::from_secs(15), child.wait())
                .await
                .unwrap()
                .unwrap()
                .success()
        );
    });
    let root = RootOwner::open(&f.host_path(), &f.namespaces).unwrap();
    assert!(
        f.timer
            .block_on(maka_runtime_host::server::Host::open(root))
            .is_err()
    );
    // The shared recovery boundary also waits for legacy Host's original SQL
    // owner; a returned startup error is not merely a request to drop it later.
    drop(RootOwner::open(&f.host_path(), &f.namespaces).unwrap());
    f.cleanup();
}
