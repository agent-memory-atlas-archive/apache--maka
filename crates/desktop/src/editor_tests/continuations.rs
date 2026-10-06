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
use gpui_kit::{AnyElement, IntoElement, div};
use maka_ui::actions::{CaptureCall, CapturePlan, CaptureTarget, ContinuationWitness};
use maka_ui::{
    ComponentCall, ComponentOutput, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView,
};
use serde_json::{Value, json};
use std::{any::Any, cell::Cell, sync::Arc};
struct Probe;
struct Replacement;
impl widgets::Widget for Probe {
    fn create(&self, _: widgets::LiveInput, _: &mut gpui_kit::Window, _: &mut App) -> Box<dyn Any> {
        Box::new(Cell::new(7u64))
    }
    fn capture(
        &self,
        _: &mut dyn Any,
        _: &CaptureTarget,
        _: &mut gpui_kit::Window,
        _: &mut App,
    ) -> maka_ui::Result<Arc<dyn maka_plugins::call::ReadResource>> {
        Ok(Arc::new(Text))
    }
    fn capture_continuation(
        &self,
        state: &dyn Any,
        _: &CaptureTarget,
        _: &mut gpui_kit::Window,
        _: &mut App,
    ) -> maka_ui::Result<ContinuationWitness> {
        Ok(ContinuationWitness::new(
            state.downcast_ref::<Cell<u64>>().unwrap().get(),
            std::mem::size_of::<u64>(),
        ))
    }
    fn validate_continuation(
        &self,
        state: &dyn Any,
        _: &CaptureTarget,
        witness: &ContinuationWitness,
        _: &mut gpui_kit::Window,
        _: &mut App,
    ) -> maka_ui::Result<bool> {
        Ok(
            witness.downcast_ref::<u64>()
                == Some(&state.downcast_ref::<Cell<u64>>().unwrap().get()),
        )
    }
    fn render(
        &self,
        _: &dyn Any,
        _: &widgets::Slots,
        _: &mut gpui_kit::Window,
        _: &mut App,
    ) -> maka_ui::Result<AnyElement> {
        Ok(div().into_any_element())
    }
}
impl widgets::Widget for Replacement {
    fn render(
        &self,
        _: &dyn Any,
        _: &widgets::Slots,
        _: &mut gpui_kit::Window,
        _: &mut App,
    ) -> maka_ui::Result<AnyElement> {
        Ok(div().into_any_element())
    }
    fn validate_continuation(
        &self,
        _: &dyn Any,
        _: &CaptureTarget,
        _: &ContinuationWitness,
        _: &mut gpui_kit::Window,
        _: &mut App,
    ) -> maka_ui::Result<bool> {
        panic!("original owner kind must reject replacement before widget validation")
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
impl NativeComponent<dyn widgets::Widget> for Component {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<dyn widgets::Widget>>> {
        let (revision, props) = match call {
            ComponentCall::Mount { props } => (0, props),
            ComponentCall::Update { state, props } | ComponentCall::Event { state, props, .. } => {
                (state["revision"].as_u64().unwrap() + 1, props)
            }
        };
        let recipe: Rc<dyn widgets::Widget> = if props["command"].is_null() {
            Rc::new(Probe)
        } else {
            Rc::new(Replacement)
        };
        Ok(maka_ui::Frame {
            revision,
            state: json!({"revision":revision}),
            surface: None,
            view: NativeView::new(recipe, BTreeMap::new(), vec![])?,
        }
        .output(vec![]))
    }
    fn capture(&self, _: CaptureCall<'_>) -> maka_ui::Result<CapturePlan> {
        Ok(CapturePlan::Native)
    }
}
#[gpui::test]
fn original_native_continuation_is_checked_before_scene_publication(cx: &mut TestAppContext) {
    let timer = timer();
    for change in ["none", "reading", "owner", "retained_owner", "recipe"] {
        let (input, packages, contract) = support::continuation_fixture();
        let mut registry = NativeRegistry::with_surface(PresentationTarget::Desktop)
            .with_scene_compiler(widgets::scene::language(), widgets::scene::compile)
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
        let scene = Scene::with_registry(
            cx,
            &timer,
            false,
            (input, packages),
            registry,
            Default::default(),
        );
        scene
            .app
            .borrow_mut()
            .dispatch(&scene.root, 0, "read", json!({"key":"A"}))
            .unwrap();
        cx.update_window(scene.window, |_, window, cx| {
            scene
                .app
                .borrow_mut()
                .capture_actions(&scene.root, |target| {
                    scene.resources.borrow_mut().capture(target, window, cx)
                })
                .unwrap()
        })
        .unwrap();
        let task = scene
            .app
            .borrow_mut()
            .start_task(&scene.root)
            .unwrap()
            .unwrap();
        let completion = timer.block_on(task.future);
        let maka_ui::actions::TaskCompletion::Query(ref result) = completion else {
            panic!("query")
        };
        assert_eq!(result.result.as_ref().unwrap()["text"], "Start");
        scene
            .app
            .borrow_mut()
            .finish_task(task.id, completion)
            .unwrap();
        let retained_owner = (change == "retained_owner").then(|| {
            scene.resources.borrow().owners[&scene.editor.id().to_string()]
                .identity
                .clone()
        });
        match change {
            "reading" => scene.resources.borrow().owners[&scene.editor.id().to_string()]
                .state
                .downcast_ref::<Cell<u64>>()
                .unwrap()
                .set(8),
            "owner" | "retained_owner" => {
                let mut resources = scene.resources.borrow_mut();
                let owner = resources
                    .owners
                    .get_mut(&scene.editor.id().to_string())
                    .unwrap();
                let original = Rc::downgrade(&owner.identity);
                owner.identity = Rc::new(());
                assert_eq!(
                    original.upgrade().is_some(),
                    retained_owner.is_some(),
                    "witness must not retain the native owner"
                );
            }
            "recipe" => {
                scene.app.borrow_mut().dispatch(&scene.root, 1, "read", json!({"key":"B", "command":{"owner":"probe","sequence":1,"basedOn":0,"edit":{"kind":"reset","text":""}}})).unwrap();
                assert!(
                    scene.resources.borrow().owners[&scene.editor.id().to_string()]
                        .recipe
                        .as_ref()
                        .type_id()
                        == std::any::TypeId::of::<Probe>()
                );
                assert!(
                    scene
                        .app
                        .borrow()
                        .frame(&scene.editor)
                        .unwrap()
                        .view
                        .view
                        .as_ref()
                        .type_id()
                        == std::any::TypeId::of::<Replacement>()
                );
            }
            _ => {}
        }
        assert!(
            scene
                .app
                .borrow()
                .frame(&scene.root)
                .unwrap()
                .state
                .get("outcome")
                .is_none()
        );
        cx.update_window(scene.window, |_, window, cx| {
            assert_eq!(
                scene
                    .surface
                    .update(cx, |surface, cx| surface.deliver_action_results(window, cx))
                    .unwrap(),
                1
            )
        })
        .unwrap();
        assert_eq!(
            scene.app.borrow().frame(&scene.root).unwrap().state["outcome"]["status"],
            if change == "none" {
                "succeeded"
            } else {
                "stale"
            }
        );
        assert!(!scene.app.borrow().is_retiring(&scene.root));
        assert_eq!(scene.app.borrow().accepted_action_count(), 0);
        drop(retained_owner);
        scene.close(cx);
        scene.app.borrow_mut().shutdown().unwrap();
    }
}
