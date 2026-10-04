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
use maka_plugin_core::{
    contract::{Contract, Interface},
    package::EntryTarget,
};
use maka_plugins::call::ReadResource;
use maka_ui::actions::*;
use std::{
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    task::Poll,
    time::Duration,
};

fn contract() -> Contract {
    let mut contract = support::business();
    let Interface::Service { methods } = &mut contract.interface else {
        unreachable!()
    };
    methods
        .get_mut("write")
        .unwrap()
        .resources
        .insert("text".into(), maka_ui::text::text_resource_type());
    contract
}
fn fixture() -> Fixture {
    fixture_with_ui(false)
}
fn fixture_with_ui(resources: bool) -> Fixture {
    let declared = contract();
    let (mut input, originals) = support::host();
    let mut packages = Vec::new();
    for old in originals {
        if old.manifest().id == "local.receipts" {
            packages.push(old);
            continue;
        }
        let mut declaration = old.manifest().application.clone().unwrap();
        let entry = declaration.entries.get_mut(&EntryTarget::Host).unwrap();
        for import in entry.imports.values_mut() {
            if import.contract.name == declared.name {
                import.contract = declared.clone();
            }
        }
        for export in entry.exports.values_mut() {
            if export.contract.name == declared.name {
                export.contract = declared.clone();
            }
        }
        let source=(old.manifest().id=="local.business").then(||format!(r#"
            let effects=0;
            export function activate(){{return {{Counter:{{contract:{},service:['service']}}}};}}
            export const service={{read(){{return {{effects}};}},async write(input,_observation,context){{
                const value=await context.invoke({{operation:'read',slot:'text',start:0,end:5}});
                if(String.fromCharCode(...value)!=='token')throw Error('wrong original snapshot');
                return {{effects:++effects}};
            }}}};
        "#,serde_json::to_string(&declared).unwrap()));
        let new = support::counter::package(&old.manifest().id, declaration, source);
        for node in input.instances.values_mut() {
            if node.package == old.application_ref().into() {
                node.package = new.application_ref().into();
            }
        }
        packages.push(new);
    }
    let (mut ui_input, mut ui_packages, mut observations) =
        support::ui(PresentationTarget::Desktop);
    for observation in &mut observations {
        if observation.declaration.contract.name == declared.name {
            observation.declaration.contract = declared.clone();
        }
    }
    let old = ui_packages.pop().unwrap();
    let mut files = old.files().clone();
    let mut declaration = old.manifest().application.clone().unwrap();
    for entry in declaration.entries.values_mut() {
        entry.imports.get_mut("business").unwrap().contract = declared.clone();
    }
    files.insert(
        maka_plugins::package::MANIFEST_FILE.into(),
        serde_json::to_vec(
            &json!({"schemaVersion":2,"id":old.manifest().id,"application":declaration}),
        )
        .unwrap(),
    );
    let new = maka_plugins::package::Package::new(files).unwrap();
    ui_input.instances.get_mut("app").unwrap().package = new.application_ref().into();
    ui_packages.push(new);
    let ui = if resources {
        resource_ui(&declared)
    } else {
        (ui_input, ui_packages, observations)
    };
    Fixture::from_sources((input, packages), ui)
}
struct CaptureSource {
    inner: Arc<dyn NativeService>,
    source: Arc<Mutex<Option<ServiceSource>>>,
}
impl NativeService for CaptureSource {
    fn call(&self, request: ServiceRequest) -> ServiceFuture {
        *self.source.lock().unwrap() = Some(request.source.clone());
        self.inner.call(request)
    }
}
struct Snapshot {
    data: String,
    descriptors: AtomicUsize,
    entered: Arc<tokio::sync::Semaphore>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
    pause: bool,
}
impl ReadResource for Snapshot {
    fn resource_type(&self) -> maka_plugin_core::contract::ResourceType {
        maka_ui::text::text_resource_type()
    }
    fn bytes(&self) -> usize {
        self.data.len()
    }
    fn metadata(&self) -> Value {
        if self.pause && self.descriptors.fetch_add(1, Ordering::SeqCst) == 1 {
            self.entered.add_permits(1);
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(3))
                .unwrap();
        }
        json!({"private":"never-log-this-metadata"})
    }
    fn read(&self, range: std::ops::Range<usize>) -> Result<Vec<u8>, String> {
        self.data
            .get(range)
            .map(|s| s.as_bytes().to_vec())
            .ok_or("range".into())
    }
}

#[test]
fn actual_local_session_close_waits_for_preparing_custody_and_drops_its_own_resource_copy() {
    let f = fixture();
    let mut connections = f.connect(f.configuration(true, true));
    let source = Arc::new(Mutex::new(None));
    let mut registrations = connections.take_registrations();
    let business = registrations
        .iter_mut()
        .find(|r| r.target.export == "Counter")
        .unwrap();
    let actual = business.implementation.clone();
    business.implementation = Arc::new(CaptureSource {
        inner: actual.clone(),
        source: source.clone(),
    });
    let mut app = f
        .timer
        .block_on(Application::start_from_store(
            f.caller.clone(),
            connections.source().clone(),
            PresentationTarget::Desktop,
            f.timer.handle().clone(),
            Default::default(),
            support::counter::scene::registry(PresentationTarget::Desktop),
            registrations,
        ))
        .unwrap();
    let root = app.mount_launch(json!({})).unwrap();
    assert_eq!(action(&f, &mut app, &root, "read")["status"], "succeeded");
    let source = source.lock().unwrap().take().unwrap();
    for phase in 0..3 {
        let entered = Arc::new(tokio::sync::Semaphore::new(0));
        let (release, receiver) = std::sync::mpsc::channel();
        let snapshot = Arc::new(Snapshot {
            data: "token-private-credential".into(),
            descriptors: AtomicUsize::new(0),
            entered: entered.clone(),
            release: Mutex::new(receiver),
            pause: phase == 1,
        });
        let weak = Arc::downgrade(&snapshot);
        let session = actual
            .mutation(ServiceRequest {
                source: source.clone(),
                method: "write".into(),
                input: json!({"public":true}),
                resources: BTreeMap::from([("text".into(), snapshot as Arc<dyn ReadResource>)]),
            })
            .unwrap();
        f.timer.block_on(async {
            let mut preparation = if phase == 0 {
                None
            } else {
                Some(session.clone().prepare())
            };
            if phase == 1 {
                assert!(matches!(
                    futures_util::poll!(preparation.as_mut().unwrap()),
                    Poll::Pending
                ));
                tokio::time::timeout(Duration::from_secs(3), entered.acquire())
                    .await
                    .unwrap()
                    .unwrap()
                    .forget();
                let mut close = session.clone().close();
                assert!(matches!(futures_util::poll!(&mut close), Poll::Pending));
                assert!(weak.upgrade().is_some());
                release.send(()).unwrap();
                assert!(preparation.take().unwrap().await.is_err());
                close.await.unwrap();
            } else {
                if let Some(preparation) = preparation {
                    preparation.await.unwrap();
                }
                session.clone().close().await.unwrap();
            }
            assert!(
                weak.upgrade().is_none(),
                "a still-live closed Session must release its actual snapshot copy"
            );
            session.clone().close().await.unwrap();
        });
    }
    // No preparation or close performed the secret-consuming business effect.
    assert_eq!(
        action(&f, &mut app, &root, "read")["value"],
        json!({"effects":0})
    );
    stop(&f, app, &root, &mut connections);
    f.timer.block_on(f.caller.shutdown()).unwrap();
}

fn resource_ui(
    declared: &Contract,
) -> (
    maka_plugin_core::composition::Input,
    Vec<maka_plugins::package::Package>,
    Vec<maka_plugin_core::deployment::RemoteObservation>,
) {
    let (mut input, originals) = support::counter::resource_fixture();
    input.instances.remove("reader");
    let mut packages = Vec::new();
    for old in originals {
        if old.manifest().id == "native.reader" {
            continue;
        }
        if old.manifest().id != "dynamic.app" {
            packages.push(old);
            continue;
        }
        let mut declaration = old.manifest().application.clone().unwrap();
        declaration
            .entries
            .get_mut(&EntryTarget::Desktop)
            .unwrap()
            .imports
            .get_mut("reader")
            .unwrap()
            .contract = declared.clone();
        let source=std::str::from_utf8(old.file("index.mjs").unwrap()).unwrap()
            .replace("method:'read'","method:'write'")
            .replace("else if(event.key==='remove')next.show=false;", "else if(event.key==='remove')next.show=false;else if(event.key==='restore')next.show=true;")
            .replace("{kind:'button',key:'remove',label:'Remove composer'}", "{kind:'button',key:'remove',label:'Remove composer'},{kind:'button',key:'restore',label:'Restore composer'}");
        let new = support::counter::package(&old.manifest().id, declaration, Some(source));
        input.instances.get_mut("app").unwrap().package = new.application_ref().into();
        packages.push(new);
    }
    let target = maka_plugin_core::plan::RemoteCapability {
        deployment: "local".into(),
        instance: support::identity(11),
        export: "Counter".into(),
    };
    input.instances.get_mut("app").unwrap().bindings.insert(
        "reader".into(),
        vec![maka_plugin_core::composition::Target::Remote {
            deployment: target.deployment.clone(),
            instance: target.instance.clone(),
            export: target.export.clone(),
        }],
    );
    let observation = maka_plugin_core::deployment::RemoteObservation {
        capability: target,
        declaration: maka_plugin_core::contract::Export {
            contract: declared.clone(),
            availability: maka_plugin_core::contract::Availability::Required,
        },
    };
    (input, packages, vec![observation])
}
struct Leaf;
impl maka_ui::NativeComponent<()> for Leaf {
    fn call(
        &self,
        call: maka_ui::ComponentCall<'_>,
        _: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<maka_ui::ComponentOutput<maka_ui::NativeView<()>>> {
        let revision = match call {
            maka_ui::ComponentCall::Mount { .. } => 0,
            maka_ui::ComponentCall::Event { state, .. }
            | maka_ui::ComponentCall::Update { state, .. } => {
                state["revision"].as_u64().unwrap() + 1
            }
        };
        Ok(maka_ui::Frame {
            revision,
            state: json!({"revision":revision}),
            surface: None,
            view: maka_ui::NativeView::new(std::rc::Rc::new(()), BTreeMap::new(), vec![])?,
        }
        .output(vec![]))
    }
    fn capture(&self, _: CaptureCall<'_>) -> maka_ui::Result<CapturePlan> {
        Ok(CapturePlan::Native)
    }
}
fn snapshot(size: usize) -> (Arc<Snapshot>, std::sync::Weak<Snapshot>) {
    let (_, receiver) = std::sync::mpsc::channel();
    let mut data = "x".repeat(size);
    if size >= 40 {
        let secret = "token-never-persist-this-secret-🙂";
        data.replace_range(0..secret.len(), secret);
    }
    let snapshot = Arc::new(Snapshot {
        data,
        descriptors: AtomicUsize::new(0),
        entered: Arc::new(tokio::sync::Semaphore::new(0)),
        release: Mutex::new(receiver),
        pause: false,
    });
    let weak = Arc::downgrade(&snapshot);
    (snapshot, weak)
}
fn dispatch(app: &mut Application, root: &maka_plugin_core::runtime::Mount, key: &str) {
    app.dispatch(root, app.frame(root).unwrap().revision, key, Value::Null)
        .unwrap();
}
#[test]
fn actual_resource_mutation_counts_preparing_running_and_closing_and_rechecks_original_edges_after_sql()
 {
    let f = fixture_with_ui(true);
    let mut configuration = f.configuration(true, false);
    configuration.hosts[0].imports.remove("receipts");
    let mut connections = f.connect(configuration);
    let mut native = support::counter::scene::registry(PresentationTarget::Desktop);
    native
        .register("maka.ui.textarea", |_, _| {
            Ok(BTreeMap::from([(
                "Textarea".into(),
                maka_ui::NativeExport {
                    contract: maka_ui::textarea::textarea_contract(),
                    value: maka_ui::NativeCapability::Component(std::rc::Rc::new(Leaf)),
                },
            )]))
        })
        .unwrap();
    let mut app = f
        .timer
        .block_on(Application::start_from_store(
            f.caller.clone(),
            connections.source().clone(),
            PresentationTarget::Desktop,
            f.timer.handle().clone(),
            Default::default(),
            native,
            connections.take_registrations(),
        ))
        .unwrap();
    let root = app.mount_launch(json!({})).unwrap();
    let old = app.child_mount(&root, "editor").unwrap().clone();
    dispatch(&mut app, &root, "read");
    let (resource, weak) = snapshot(64 * 1024 * 1024);
    assert_eq!(
        app.capture_actions(&root, |_| Ok(resource.clone()))
            .unwrap(),
        1
    );
    drop(resource);
    let original = app.start_task(&root).unwrap().unwrap();
    for _ in 0..2 {
        dispatch(&mut app, &root, "read");
        let (resource, rejected) = snapshot(1);
        assert_eq!(
            app.capture_actions(&root, |_| Ok(resource.clone()))
                .unwrap(),
            0
        );
        drop(resource);
        assert!(rejected.upgrade().is_none());
        assert!(app.start_task(&root).unwrap().is_none());
    }
    let prepared = f.timer.block_on(original.future);
    assert!(matches!(prepared, TaskCompletion::Prepared(Ok(_))));
    dispatch(&mut app, &root, "remove");
    dispatch(&mut app, &root, "restore");
    assert_ne!(app.child_mount(&root, "editor").unwrap(), &old);
    let closing = app.finish_task(original.id, prepared).unwrap().unwrap();
    assert!(
        weak.upgrade().is_some(),
        "closing still owns the original snapshot and budget"
    );
    dispatch(&mut app, &root, "read");
    let (resource, rejected) = snapshot(1);
    assert_eq!(
        app.capture_actions(&root, |_| Ok(resource.clone()))
            .unwrap(),
        0
    );
    drop(resource);
    assert!(rejected.upgrade().is_none());
    app.finish_task(closing.id, f.timer.block_on(closing.future))
        .unwrap();
    assert!(weak.upgrade().is_none());
    // With actual close confirmed, the next full-size captured operation is usable.
    dispatch(&mut app, &root, "read");
    let (resource, weak) = snapshot(64 * 1024 * 1024);
    assert_eq!(
        app.capture_actions(&root, |_| Ok(resource.clone()))
            .unwrap(),
        1
    );
    drop(resource);
    let preparation = app.start_task(&root).unwrap().unwrap();
    let completed = f.timer.block_on(preparation.future);
    let running = app.finish_task(preparation.id, completed).unwrap().unwrap();
    dispatch(&mut app, &root, "read");
    let (resource, rejected) = snapshot(1);
    assert_eq!(
        app.capture_actions(&root, |_| Ok(resource.clone()))
            .unwrap(),
        0
    );
    drop(resource);
    assert!(rejected.upgrade().is_none());
    app.finish_task(running.id, f.timer.block_on(running.future))
        .unwrap();
    assert!(weak.upgrade().is_none());
    assert!(app.deliver_action_results(&root).unwrap() > 0);
    let history = f
        .timer
        .block_on(
            f.caller
                .application_intents(f.source.deployment, support::identity(2), 0, 10),
        )
        .unwrap();
    assert_eq!(history.len(), 2);
    app.retire(&root).unwrap();
    for mount in app.retiring_mounts() {
        app.close(&mount, Ok(())).unwrap();
    }
    app.shutdown().unwrap();
    drop(app);
    f.timer.block_on(connections.close()).unwrap();
    let owner = Arc::new(RootOwner::open(&f.directory.path().join("host"), &f.namespaces).unwrap());
    let journal = f.timer.block_on(EventLog::for_root(owner)).unwrap();
    assert!(
        f.timer
            .block_on(journal.operation_observation(&history[0].reference))
            .unwrap()
            .is_none(),
        "retired original capture did not produce an effect after SQL"
    );
    let effect = f
        .timer
        .block_on(journal.operation_observation(&history[1].reference))
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(effect.outcome).unwrap()["value"],
        json!({"effects":1})
    );
    f.timer.block_on(journal.close()).unwrap();
    f.timer.block_on(f.caller.shutdown()).unwrap();
    for name in ["host", "caller"] {
        let bytes = std::fs::read(
            f.directory
                .path()
                .join(name)
                .join(maka_event_log::root::ROOT_DATABASE),
        )
        .unwrap();
        for private in [
            "token-never-persist-this-secret-🙂",
            "never-log-this-metadata",
        ] {
            assert!(
                !bytes
                    .windows(private.len())
                    .any(|window| window == private.as_bytes()),
                "actual durable records must not store snapshot bytes or metadata"
            );
        }
    }
}
