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

use super::{support, timer};
use maka_plugin_core::{
    composition::{Input, PresentationTarget},
    contract::ResourceType,
    runtime::Mount,
};
use maka_plugins::call::{Attachments, ReadResource};
use maka_ui::actions::*;
use maka_ui::{
    Application, ComponentCall, ComponentOutput, Frame, NativeCapability, NativeComponent,
    NativeExport, NativeRegistry, NativeView, Presentation,
};
use serde_json::{Value, json};
use std::{cell::Cell, collections::BTreeMap, rc::Rc, sync::Arc};
struct Leaf {
    captures: Rc<Cell<usize>>,
    large: bool,
}
impl NativeComponent<()> for Leaf {
    fn call(&self, call: ComponentCall<'_>) -> maka_ui::Result<ComponentOutput<Presentation<()>>> {
        let revision = match call {
            ComponentCall::Mount { .. } => 0,
            ComponentCall::Update { state, .. } | ComponentCall::Event { state, .. } => {
                state["revision"].as_u64().unwrap() + 1
            }
        };
        Ok(Frame {revision,state:json!({"revision":revision,"blob":if self.large {"x".repeat(1_048_570)} else {String::new()}}),window:None,view:Presentation::Native(NativeView::new(Rc::new(()),BTreeMap::new())?) }.output(vec![]))
    }
    fn capture(&self, _: CaptureCall<'_>) -> maka_ui::Result<CapturePlan> {
        self.captures.set(self.captures.get() + 1);
        Ok(CapturePlan::Native)
    }
}
struct Text(String);
impl ReadResource for Text {
    fn resource_type(&self) -> ResourceType {
        maka_ui::text::text_resource_type()
    }
    fn bytes(&self) -> usize {
        self.0.len()
    }
    fn metadata(&self) -> Value {
        json!({"revision":3,"bytes":self.0.len()})
    }
    fn read(&self, r: std::ops::Range<usize>) -> Result<Vec<u8>, String> {
        self.0
            .get(r)
            .map(|s| s.as_bytes().to_vec())
            .ok_or("range".into())
    }
}
fn fixture(
    large: bool,
) -> (
    Input,
    Vec<maka_plugins::package::Package>,
    NativeRegistry<()>,
    Rc<Cell<usize>>,
) {
    let (input, packages) = support::resource_fixture();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let mut native = NativeRegistry::with_surface(PresentationTarget::Desktop);
    native
        .register("maka.ui.editor", move |_| {
            Ok(BTreeMap::from([(
                "Editor".into(),
                NativeExport {
                    contract: maka_ui::editor::editor_contract(),
                    value: NativeCapability::Component(Rc::new(Leaf {
                        captures: observed.clone(),
                        large,
                    })),
                },
            )]))
        })
        .unwrap();
    (input, packages, native, calls)
}
fn application(
    large: bool,
    timer: &tokio::runtime::Runtime,
) -> (Application, Mount, Rc<Cell<usize>>) {
    let (input, packages, native, calls) = fixture(large);
    let mut app = Application::new(
        &input,
        packages,
        timer.handle().clone(),
        Default::default(),
        native,
    )
    .unwrap();
    let root = app.mount_launch(json!({})).unwrap();
    (app, root, calls)
}
fn capture(app: &mut Application, root: &Mount, text: &str) -> Arc<dyn ReadResource> {
    let snapshot: Arc<dyn ReadResource> = Arc::new(Text(text.into()));
    let result = snapshot.clone();
    app.capture_actions(root, |target| {
        assert_eq!(target.method(), "snapshot");
        assert_eq!(target.resource_type(), &maka_ui::text::text_resource_type());
        Ok(snapshot.clone())
    })
    .unwrap();
    result
}
fn close_all(app: &mut Application, roots: &[Mount]) {
    for r in roots {
        app.retire(r).unwrap();
    }
    for m in app.retiring_mounts() {
        app.close(&m, Ok(())).unwrap();
    }
    app.shutdown().unwrap();
}
#[test]
fn mounted_wrapper_capture_and_typed_resource_action_keep_exact_root_and_internal_result_authority()
{
    let timer = timer();
    let (mut app, a, calls) = application(false, &timer);
    let b = app.mount_launch(json!({})).unwrap();
    let inner = app
        .child_mount(app.child_mount(&a, "editor").unwrap(), "draft")
        .unwrap()
        .clone();
    app.dispatch(&a, 0, "read", json!({})).unwrap();
    assert_eq!(calls.get(), 1);
    app.capture_actions(&a, |target| {
        assert_eq!(target.mount(), &inner);
        Ok(Arc::new(Text("Start".into())))
    })
    .unwrap();
    assert!(app.start_action(&b).unwrap().is_none());
    let accepted = app.start_action(&a).unwrap().unwrap();
    let done = timer.block_on(accepted.future);
    assert_eq!(done.result.as_ref().unwrap()["text"], "Start");
    app.finish_action(accepted.id, done).unwrap();
    assert_eq!(app.deliver_action_results(&b).unwrap(), 0);
    assert_eq!(app.frame(&b).unwrap().revision, 0);
    assert_eq!(app.deliver_action_results(&a).unwrap(), 1);
    assert_eq!(
        app.frame(&a).unwrap().state["outcome"]["value"]["text"],
        "Start"
    );
    assert!(
        app.dispatch(
            &a,
            2,
            "read_done",
            json!({"status":"succeeded","value":{"text":"forged"}})
        )
        .is_err()
    );
    close_all(&mut app, &[a, b]);
}
#[test]
fn untrusted_metadata_or_native_plan_cannot_manufacture_resource_authority() {
    let timer = timer();
    let (mut app, root, calls) = application(false, &timer);
    assert!(
        app.dispatch(
            &root,
            0,
            "read",
            json!({"capture":{"kind":"current","owner":"forged-id"}})
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(app.pending_action_count(), 0);
    assert_eq!(app.frame(&root).unwrap().revision, 0);
    close_all(&mut app, &[root]);
    let (input, mut packages, native, _) = fixture(false);
    let i = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.composer")
        .unwrap();
    let old = packages.remove(i);
    let source = std::str::from_utf8(old.file("index.mjs").unwrap())
        .unwrap()
        .replace(
            "return {kind:'forward',source:{child:'draft',method,input}};",
            "return {kind:'native'};",
        );
    let composer = support::package(
        "dynamic.composer",
        old.manifest().application.clone().unwrap(),
        Some(source),
    );
    let mut input = input;
    input.instances.get_mut("composer").unwrap().package = composer.application_ref().into();
    packages.push(composer);
    let mut app = Application::new(
        &input,
        packages,
        timer.handle().clone(),
        Default::default(),
        native,
    )
    .unwrap();
    let root = app.mount_launch(json!({})).unwrap();
    assert!(app.dispatch(&root, 0, "read", Value::Null).is_err());
    assert_eq!(app.pending_action_count(), 0);
    close_all(&mut app, &[root]);
}
#[test]
fn capture_failure_is_an_owned_not_accepted_outcome_after_scene_commit() {
    let timer = timer();
    let (mut app, root, _) = application(false, &timer);
    app.dispatch(&root, 0, "read", json!({})).unwrap();
    assert_eq!(app.frame(&root).unwrap().revision, 1);
    app.capture_actions(&root, |_| {
        Err(maka_ui::Error::Invalid("revision conflict".into()))
    })
    .unwrap();
    assert!(app.start_action(&root).unwrap().is_none());
    app.deliver_action_results(&root).unwrap();
    assert_eq!(
        app.frame(&root).unwrap().state["outcome"]["status"],
        "not_accepted"
    );
    assert!(!app.is_retiring(&root));
    close_all(&mut app, &[root]);
}
#[test]
fn source_removal_before_admission_cancels_but_accepted_snapshot_outlives_removed_subtree() {
    let timer = timer();
    for accepted_first in [false, true] {
        let (mut app, root, _) = application(false, &timer);
        app.dispatch(&root, 0, "read", Value::Null).unwrap();
        let snapshot = capture(&mut app, &root, "Start");
        let weak = Arc::downgrade(&snapshot);
        drop(snapshot);
        let accepted = if accepted_first {
            app.start_action(&root).unwrap()
        } else {
            None
        };
        app.dispatch(&root, 1, "remove", Value::Null).unwrap();
        for m in app.retiring_mounts() {
            app.close(&m, Ok(())).unwrap();
        }
        if let Some(work) = accepted {
            let done = timer.block_on(work.future);
            assert_eq!(done.result.as_ref().unwrap()["text"], "Start");
            app.finish_action(work.id, done).unwrap();
        } else {
            assert!(app.start_action(&root).unwrap().is_none());
        }
        app.deliver_action_results(&root).unwrap();
        assert_eq!(
            app.frame(&root).unwrap().state["outcome"]["status"],
            if accepted_first {
                "succeeded"
            } else {
                "not_accepted"
            }
        );
        assert!(weak.upgrade().is_none());
        close_all(&mut app, &[root]);
    }
}
#[test]
fn specific_consumer_mount_is_checked_even_when_same_provider_has_another_live_root() {
    let timer = timer();
    let (mut app, a, _) = application(false, &timer);
    let b = app.mount_launch(json!({})).unwrap();
    app.dispatch(&a, 0, "read", Value::Null).unwrap();
    capture(&mut app, &a, "Start");
    app.retire(&a).unwrap();
    assert!(app.start_action(&a).unwrap().is_none());
    assert_eq!(app.accepted_action_count(), 0);
    app.dispatch(&b, 0, "read", Value::Null).unwrap();
    capture(&mut app, &b, "Start");
    let work = app.start_action(&b).unwrap().unwrap();
    let done = timer.block_on(work.future);
    app.finish_action(work.id, done).unwrap();
    app.deliver_action_results(&b).unwrap();
    close_all(&mut app, &[a, b]);
}
#[test]
fn native_capture_uses_the_complete_call_limit_before_native_callback() {
    let timer = timer();
    let (mut app, root, calls) = application(true, &timer);
    assert!(app.dispatch(&root, 0, "read", Value::Null).is_err());
    assert_eq!(calls.get(), 0);
    assert_eq!(app.frame(&root).unwrap().revision, 0);
    assert_eq!(app.pending_action_count(), 0);
    close_all(&mut app, &[root]);
}
#[test]
fn failed_or_lost_result_handlers_do_not_hold_accepted_work_or_healthy_pending_mounts() {
    let timer = timer();
    let (input, mut packages, native, _) = fixture(false);
    let i = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let old = packages.remove(i);
    let source = std::str::from_utf8(old.file("index.mjs").unwrap())
        .unwrap()
        .replace(
            "if(event.kind==='action')next.outcome=event.event;",
            "if(event.kind==='action')throw new Error('result handler failed');",
        );
    let root_pkg = support::package(
        "dynamic.app",
        old.manifest().application.clone().unwrap(),
        Some(source),
    );
    let mut input = input;
    input.instances.get_mut("app").unwrap().package = root_pkg.application_ref().into();
    packages.push(root_pkg);
    let mut app = Application::new(
        &input,
        packages,
        timer.handle().clone(),
        Default::default(),
        native,
    )
    .unwrap();
    let root = app.mount_launch(json!({})).unwrap();
    app.dispatch(&root, 0, "read", Value::Null).unwrap();
    capture(&mut app, &root, "Start");
    let work = app.start_action(&root).unwrap().unwrap();
    let done = timer.block_on(work.future);
    app.finish_action(work.id, done).unwrap();
    assert_eq!(app.accepted_action_count(), 0);
    assert!(app.deliver_action_results(&root).is_err());
    assert!(app.is_retiring(&root));
    close_all(&mut app, &[root]);
}

struct CompletionService {
    value: Value,
    cleanup_error: bool,
}
impl NativeService for CompletionService {
    fn call(&self, _: String, _: Value, _: Attachments) -> ServiceFuture {
        let value = self.value.clone();
        let failed = self.cleanup_error;
        Box::pin(async move {
            ServiceCompletion {
                result: Ok(value),
                cleanup: if failed {
                    Err("cleanup unknown".into())
                } else {
                    Ok(())
                },
            }
        })
    }
}
fn service_fixture(
    timer: &tokio::runtime::Runtime,
    value: Value,
    cleanup_error: bool,
    padding: usize,
) -> (Application, Mount) {
    let (mut input, mut packages, mut native, _) = fixture(false);
    let mut contract = maka_ui::text::reader_contract();
    let maka_plugin_core::contract::Interface::Service { methods } = &mut contract.interface else {
        unreachable!()
    };
    methods.get_mut("read").unwrap().output = json!({"type":"string"});
    for name in ["dynamic.app", "native.reader"] {
        let i = packages
            .iter()
            .position(|p| p.manifest().id == name)
            .unwrap();
        let old = packages.remove(i);
        let mut declaration = old.manifest().application.clone().unwrap();
        let entry = declaration
            .entries
            .get_mut(&maka_plugin_core::package::EntryTarget::Desktop)
            .unwrap();
        let source = if name == "dynamic.app" {
            entry.imports.get_mut("reader").unwrap().contract = contract.clone();
            Some(
                std::str::from_utf8(old.file("index.mjs").unwrap())
                    .unwrap()
                    .replace(
                        "revision:0,show:true",
                        &format!("revision:0,show:true,padding:'x'.repeat({padding})"),
                    ),
            )
        } else {
            entry.exports.get_mut("Reader").unwrap().contract = contract.clone();
            entry.implementation = maka_plugin_core::package::Implementation::Native {
                implementation: "test.service".into(),
            };
            None
        };
        let package = support::package(name, declaration, source);
        input
            .instances
            .get_mut(if name == "dynamic.app" {
                "app"
            } else {
                "reader"
            })
            .unwrap()
            .package = package.application_ref().into();
        packages.push(package);
    }
    native
        .register("test.service", move |_| {
            Ok(BTreeMap::from([(
                "Reader".into(),
                NativeExport {
                    contract: contract.clone(),
                    value: NativeCapability::Service(Arc::new(CompletionService {
                        value: value.clone(),
                        cleanup_error,
                    })),
                },
            )]))
        })
        .unwrap();
    let mut app = Application::new(
        &input,
        packages,
        timer.handle().clone(),
        Default::default(),
        native,
    )
    .unwrap();
    let root = app.mount_launch(json!({})).unwrap();
    (app, root)
}
#[test]
fn cleanup_failure_preserves_work_and_input_until_explicit_recovery_without_replaying_outcome() {
    let timer = timer();
    let (mut app, root) = service_fixture(&timer, json!("ok"), true, 0);
    app.dispatch(&root, 0, "read", Value::Null).unwrap();
    let snapshot = capture(&mut app, &root, "Start");
    let weak = Arc::downgrade(&snapshot);
    drop(snapshot);
    let work = app.start_action(&root).unwrap().unwrap();
    let done = timer.block_on(work.future);
    assert!(app.finish_action(work.id, done).is_err());
    assert_eq!(app.accepted_action_count(), 1);
    assert!(weak.upgrade().is_some());
    assert_eq!(app.deliver_action_results(&root).unwrap(), 0);
    assert!(
        app.is_retiring(&root),
        "cleanup failure retires service consumers"
    );
    assert!(
        app.complete_action_cleanup(work.id, Err("still unknown".into()))
            .is_err()
    );
    assert!(weak.upgrade().is_some());
    app.retire(&root).unwrap();
    for m in app.retiring_mounts() {
        app.close(&m, Ok(())).unwrap();
    }
    assert!(!app.can_shutdown());
    assert!(app.shutdown().is_err());
    app.complete_action_cleanup(work.id, Ok(())).unwrap();
    assert!(weak.upgrade().is_none());
    assert!(app.can_shutdown());
    assert_eq!(app.pending_action_count(), 0);
    app.shutdown().unwrap();
}
#[test]
fn oversized_feedback_envelope_retires_only_its_consumer_after_independent_settlement() {
    let timer = timer();
    let (mut app, a) = service_fixture(&timer, json!("y".repeat(600_000)), false, 500_000);
    let b = app.mount_launch(json!({})).unwrap();
    app.dispatch(&a, 0, "read", Value::Null).unwrap();
    capture(&mut app, &a, "Start");
    let work = app.start_action(&a).unwrap().unwrap();
    let done = timer.block_on(work.future);
    app.finish_action(work.id, done).unwrap();
    assert_eq!(app.accepted_action_count(), 0);
    assert!(app.deliver_action_results(&a).is_err());
    assert!(app.is_retiring(&a));
    assert!(!app.is_retiring(&b));
    assert_eq!(app.pending_action_count(), 0);
    close_all(&mut app, &[a, b]);
}
#[test]
fn actual_snapshot_byte_budget_rejects_before_business_admission() {
    let timer = timer();
    let (mut app, root, _) = application(false, &timer);
    app.dispatch(&root, 0, "read", Value::Null).unwrap();
    let bytes = "x".repeat(64 * 1024 * 1024 + 1);
    app.capture_actions(&root, |_| Ok(Arc::new(Text(bytes.clone()))))
        .unwrap();
    assert!(app.start_action(&root).unwrap().is_none());
    assert_eq!(app.accepted_action_count(), 0);
    app.deliver_action_results(&root).unwrap();
    assert_eq!(
        app.frame(&root).unwrap().state["outcome"]["status"],
        "not_accepted"
    );
    close_all(&mut app, &[root]);
}
