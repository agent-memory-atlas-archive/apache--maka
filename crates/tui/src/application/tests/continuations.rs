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
use maka_plugin_core::package::EntryTarget;
use maka_ui::actions::{CaptureCall, CapturePlan, CaptureTarget, ContinuationWitness};
use maka_ui::{
    ComponentCall, ComponentOutput, NativeCapability, NativeComponent, NativeExport, NativeView,
};
use std::{any::Any, cell::Cell, collections::BTreeMap, rc::Rc, sync::Arc};
struct Probe;
impl Widget for Probe {
    fn create(&self) -> Box<dyn Any> {
        Box::new(Cell::new(7u64))
    }
    fn capture(
        &self,
        _: &dyn Any,
        _: &CaptureTarget,
    ) -> maka_ui::Result<Arc<dyn maka_plugins::call::ReadResource>> {
        Ok(Arc::new(Text))
    }
    fn capture_continuation(
        &self,
        state: &dyn Any,
        _: &CaptureTarget,
    ) -> maka_ui::Result<ContinuationWitness> {
        Ok(ContinuationWitness::new(
            state.downcast_ref::<Cell<u64>>().unwrap().get(),
            8,
        ))
    }
    fn validate_continuation(
        &self,
        state: &dyn Any,
        _: &CaptureTarget,
        witness: &ContinuationWitness,
    ) -> maka_ui::Result<bool> {
        Ok(
            witness.downcast_ref::<u64>()
                == Some(&state.downcast_ref::<Cell<u64>>().unwrap().get()),
        )
    }
    fn measure(&self, _: &Slots<'_>, _: u16) -> maka_ui::Result<(u16, u16)> {
        Ok((1, 1))
    }
    fn paint(&self, _: &mut Slots<'_>, _: Rect, _: &mut Buffer) -> maka_ui::Result<()> {
        Ok(())
    }
}
struct Text;
impl maka_plugins::call::ReadResource for Text {
    fn resource_type(&self) -> maka_plugin_core::contract::ResourceType {
        maka_ui::text::text_resource_type()
    }
    fn bytes(&self) -> usize {
        5
    }
    fn metadata(&self) -> Value {
        json!({})
    }
    fn read(&self, range: std::ops::Range<usize>) -> Result<Vec<u8>, String> {
        b"Start"
            .get(range)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| "range".into())
    }
}
struct Component;
impl NativeComponent<dyn Widget> for Component {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let revision = match call {
            ComponentCall::Mount { .. } => 0,
            ComponentCall::Update { state, .. } | ComponentCall::Event { state, .. } => {
                state["revision"].as_u64().unwrap() + 1
            }
        };
        Ok(maka_ui::Frame {
            revision,
            state: json!({"revision":revision}),
            surface: None,
            view: NativeView::<dyn Widget>::new(Rc::new(Probe), BTreeMap::new(), vec![])?,
        }
        .output(vec![]))
    }
    fn capture(&self, _: CaptureCall<'_>) -> maka_ui::Result<CapturePlan> {
        Ok(CapturePlan::Native)
    }
}
fn terminal_fixture() -> (Input, Vec<Package>, maka_plugin_core::contract::Contract) {
    let (mut input, mut packages, contract) = support::continuation_fixture();
    input.environment.presentation = Some(PresentationTarget::Tui);
    for original in &mut packages {
        let mut declaration = original.manifest().application.clone().unwrap();
        let mut entry = declaration.entries.remove(&EntryTarget::Desktop).unwrap();
        if entry.scene.is_some() {
            entry.scene = Some(scene::language());
        }
        if let Some(surface) = entry.imports.get_mut("surface") {
            surface.contract = maka_ui::surface_contract(PresentationTarget::Tui);
        }
        if let Some(surface) = entry.exports.get_mut("Surface") {
            surface.contract = maka_ui::surface_contract(PresentationTarget::Tui);
            entry.implementation = maka_plugin_core::package::Implementation::Native {
                implementation: "maka.ui.surface.tui".into(),
            };
        }
        declaration.entries.insert(EntryTarget::Tui, entry);
        let source = original.file("index.mjs").map(|bytes| {
            std::str::from_utf8(bytes).unwrap().replace(
                "{kind:'window',binding:'surface',title:'Immutable draft',width:640,height:480}",
                "{kind:'terminal',binding:'surface'}",
            )
        });
        let replacement = support::package(&original.manifest().id, declaration, source);
        for instance in input.instances.values_mut() {
            if instance.package == original.application_ref().into() {
                instance.package = replacement.application_ref().into();
            }
        }
        *original = replacement;
    }
    (input, packages, contract)
}
#[test]
fn terminal_pump_uses_original_native_continuation_before_feedback_publication() {
    let timer = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    for change in ["none", "reading", "owner", "retained_owner"] {
        let (input, packages, contract) = terminal_fixture();
        let mut registry = NativeRegistry::with_surface(PresentationTarget::Tui)
            .with_scene_compiler(scene::language(), scene::compile)
            .unwrap();
        registry
            .register("maka.ui.textarea", move |_, _| {
                Ok(BTreeMap::from([(
                    "Textarea".into(),
                    NativeExport {
                        contract: contract.clone(),
                        value: NativeCapability::Component(Rc::new(Component)),
                    },
                )]))
            })
            .unwrap();
        let app = Application::new(
            &input,
            packages,
            timer.handle().clone(),
            Default::default(),
            registry,
            Default::default(),
        )
        .unwrap();
        let mut presenter = Presenter::mount(app, json!({})).unwrap();
        let outer = presenter
            .application
            .child_mount(&presenter.root, "editor")
            .unwrap()
            .clone();
        let leaf = presenter
            .application
            .child_mount(&outer, "draft")
            .unwrap()
            .clone();
        presenter
            .application
            .dispatch(&presenter.root, 0, "read", json!({"key":"A"}))
            .unwrap();
        presenter.sync_native().unwrap();
        let task = presenter
            .application
            .start_task(&presenter.root)
            .unwrap()
            .unwrap();
        let completion = timer.block_on(task.future);
        let TaskCompletion::Query(ref result) = completion else {
            panic!("query")
        };
        assert_eq!(result.result.as_ref().unwrap()["text"], "Start");
        presenter
            .application
            .finish_task(task.id, completion)
            .unwrap();
        let retained = (change == "retained_owner")
            .then(|| presenter.resources.owners[&leaf.id()].identity.clone());
        match change {
            "reading" => presenter.resources.owners[&leaf.id()]
                .state
                .downcast_ref::<Cell<u64>>()
                .unwrap()
                .set(8),
            "owner" | "retained_owner" => {
                let owner = presenter.resources.owners.get_mut(&leaf.id()).unwrap();
                let original = Rc::downgrade(&owner.identity);
                owner.identity = Rc::new(());
                assert_eq!(original.upgrade().is_some(), retained.is_some());
            }
            _ => {}
        }
        assert!(
            presenter
                .application
                .frame(&presenter.root)
                .unwrap()
                .state
                .get("outcome")
                .is_none()
        );
        presenter.pump().unwrap();
        assert_eq!(
            presenter.application.frame(&presenter.root).unwrap().state["outcome"]["status"],
            if change == "none" {
                "succeeded"
            } else {
                "stale"
            }
        );
        assert!(!presenter.application.is_retiring(&presenter.root));
        assert_eq!(presenter.application.accepted_action_count(), 0);
        drop(retained);
        presenter.surface_closed(Ok(())).unwrap();
        presenter.pump().unwrap();
        presenter.shutdown().unwrap();
    }
}
