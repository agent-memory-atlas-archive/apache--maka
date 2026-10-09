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

//! Original scene/scroll/selection custody, with weak post-paint observation.
use crate::{
    LiveInput, Slots, Widget,
    scene::{Node, NodePresentation},
};
use gpui_kit::base::{ElementExt as _, TextSelection, TextSelectionGroup, TextSelectionScopeId};
use gpui_kit::{
    AnyElement, App, AppContext, Bounds, Element, ElementId, Entity, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, ScrollCommand, ScrollHandle,
    ScrollReadingWitness, ScrollRequestId, ScrollRequestStatus, SharedString, WeakEntity, Window,
};
use maka_ui::{
    ComponentCall, ComponentOutput, Emission, EventOrigin, Frame, NativeCapability,
    NativeComponent, NativeExport, NativeRegistry, NativeView,
    actions::{CaptureCall, CapturePlan, CaptureTarget, ContinuationWitness},
    reading::*,
};
use serde_json::json;
use std::{
    any::Any,
    collections::BTreeMap,
    rc::Rc,
    time::{Duration, Instant},
};
const OBSERVED: &str = "__reading_observation";

// Identity only: preserves the child's exact LayoutId and all original geometry.
struct Mark {
    key: ElementId,
    child: AnyElement,
}
pub(crate) fn mark(key: &str, child: AnyElement) -> AnyElement {
    Mark {
        key: SharedString::from(key.to_owned()).into(),
        child,
    }
    .into_any_element()
}
impl IntoElement for Mark {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Mark {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some(self.key.clone())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}
struct Painted {
    child: AnyElement,
    owner: WeakEntity<Owner>,
}
impl IntoElement for Painted {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Painted {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
        if let Some(owner) = self.owner.upgrade() {
            owner.query(cx, |state, cx| {
                state.selection = TextSelection::group_for_element(window, cx)
            });
            Owner::schedule(&owner, window, cx);
        }
    }
}
struct Pending {
    sequence: u64,
    id: ScrollRequestId,
}
struct Owner {
    scene: Box<dyn Any>,
    root_key: String,
    handle: ScrollHandle,
    scope: TextSelectionScopeId,
    selection: Option<TextSelectionGroup>,
    input: LiveInput,
    alive: bool,
    last_command: Option<ReadingCommand>,
    pending: Option<Pending>,
    receipt: Option<ReadingReceipt>,
    published: Option<ReadingObservation>,
    scheduled: bool,
}
impl Owner {
    fn poll(&mut self) {
        if let Some(pending) = &self.pending {
            let status = match self.handle.request_status(pending.id) {
                Some(ScrollRequestStatus::Applied) => ReadingStatus::Applied,
                Some(ScrollRequestStatus::Cancelled) => ReadingStatus::Cancelled,
                Some(ScrollRequestStatus::Rejected) => ReadingStatus::Rejected,
                _ => return,
            };
            self.receipt = Some(ReadingReceipt {
                sequence: pending.sequence,
                status,
            });
            self.pending = None;
        }
    }
    fn observation(&mut self) -> ReadingObservation {
        self.poll();
        ReadingObservation {
            reading_revision: self.handle.reading_revision(),
            viewport: self
                .handle
                .painted_observation()
                .map(|painted| ReadingViewport {
                    reading_revision: painted.reading_revision,
                    at_end: painted.at_end,
                    visible_child_keys: painted
                        .visible_child_keys
                        .into_iter()
                        .map(|key| match key {
                            ElementId::Name(name) => name.to_string(),
                            _ => unreachable!("reading markers use name IDs"),
                        })
                        .collect(),
                }),
            pending_command: self.pending.as_ref().map(|pending| pending.sequence),
            receipt: self.receipt.clone(),
            find: None,
        }
    }
    fn schedule(owner: &Entity<Self>, window: &mut Window, cx: &mut App) {
        let schedule = owner.query(cx, |state, _| {
            if !state.alive || state.scheduled || state.input.0.borrow().is_none() {
                return false;
            }
            let observation = state.observation();
            if state.published.as_ref() == Some(&observation) {
                return false;
            }
            state.scheduled = true;
            true
        });
        if !schedule {
            return;
        }
        let weak = owner.downgrade();
        window.on_next_frame(move |window, cx| {
            let Some(owner) = weak.upgrade() else {
                return;
            };
            let pending = owner.query(cx, |state, _| {
                state.scheduled = false;
                if !state.alive {
                    return None;
                }
                let observation = state.observation();
                if state.published.as_ref() == Some(&observation) {
                    return None;
                }
                let input = state.input.0.borrow().clone()?;
                Some((input, observation))
            });
            let Some((input, observation)) = pending else {
                return;
            };
            // Plugin work is outside paint and the original native owner lease.
            let result = (input.0)(
                OBSERVED,
                serde_json::to_value(&observation).expect("reading observation"),
                window,
                cx,
            );
            if result.is_ok() {
                owner.query(cx, |state, _| state.published = Some(observation));
            }
        });
    }
    fn cancel(&mut self) {
        if self.pending.is_some() {
            self.handle.set_offset(self.handle.offset());
            self.poll();
        }
    }
    fn eligible(&self, _window: &Window, cx: &App) -> bool {
        self.handle.painted_observation().is_some_and(|painted| {
            painted.reading_revision == self.handle.reading_revision() && painted.at_end
        }) && self
            .selection
            .as_ref()
            .and_then(|group| group.has_selection_in_scope(self.scope, cx))
            == Some(false)
    }
}
struct Reading {
    node: Node,
    command: Option<ReadingCommand>,
}
struct Tail {
    selection: Option<TextSelectionGroup>,
    scroll: ScrollReadingWitness,
    scope: TextSelectionScopeId,
    eligible: bool,
}
fn owner(state: &dyn Any) -> &Entity<Owner> {
    state
        .downcast_ref::<Entity<Owner>>()
        .expect("reading native owner")
}
impl Widget for Reading {
    fn create(&self, input: LiveInput, window: &mut Window, cx: &mut App) -> Box<dyn Any> {
        let scene = self.node.create(input.clone(), window, cx);
        let handle = self.node.reading_handle(scene.as_ref());
        Box::new(cx.new(|_| Owner {
            scene,
            handle,
            root_key: self.node.reading_key().unwrap().into(),
            scope: TextSelectionScopeId::new(),
            selection: None,
            input,
            alive: true,
            last_command: None,
            pending: None,
            receipt: None,
            published: None,
            scheduled: false,
        }))
    }
    fn commit(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        owner(state).update(cx, |state, cx| {
            self.node.commit(state.scene.as_mut(), window, cx);
            let handle = self.node.reading_handle(state.scene.as_ref());
            if !handle.same_owner(&state.handle) {
                state.cancel();
            }
            state.handle = handle;
            state.root_key = self.node.reading_key().unwrap().into();
            if let Some(command) = &self.command
                && state.last_command.as_ref() != Some(command)
            {
                state.cancel();
                state.last_command = Some(command.clone());
                let target = match &command.target {
                    _ if !self.node.reading_visible(state.scene.as_ref()) => None,
                    ReadingTarget::Start => Some(ScrollCommand::Start),
                    ReadingTarget::End => Some(ScrollCommand::End),
                    ReadingTarget::RevealChild { key } if self.node.reading_target(key) => Some(
                        ScrollCommand::RevealChild(SharedString::from(key.clone()).into()),
                    ),
                    ReadingTarget::RevealChild { .. }
                    | ReadingTarget::Find { .. }
                    | ReadingTarget::ClearFind
                    | ReadingTarget::RevealSource { .. } => None,
                };
                if let Some(target) = target {
                    let id = state.handle.request_scroll(target);
                    state.pending = Some(Pending {
                        sequence: command.sequence,
                        id,
                    });
                } else {
                    state.handle.set_offset(state.handle.offset());
                    state.receipt = Some(ReadingReceipt {
                        sequence: command.sequence,
                        status: ReadingStatus::Rejected,
                    });
                }
            }
        });
    }
    fn presentation(&self, state: &mut dyn Any, visible: bool, window: &mut Window, cx: &mut App) {
        owner(state).update(cx, |state, cx| {
            self.node
                .presentation(state.scene.as_mut(), visible, window, cx)
        });
    }
    fn publish_pending(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        Owner::schedule(owner(state), window, cx);
    }
    fn deactivate(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        owner(state).update(cx, |state, cx| {
            state.alive = false;
            state.cancel();
            self.node.deactivate(state.scene.as_mut(), window, cx);
        });
    }
    fn capture_continuation(
        &self,
        state: &dyn Any,
        _: &CaptureTarget,
        window: &mut Window,
        cx: &mut App,
    ) -> maka_ui::Result<ContinuationWitness> {
        let state = owner(state).read(cx);
        Ok(ContinuationWitness::new(
            Tail {
                selection: state.selection.clone(),
                scroll: state.handle.reading_witness(),
                scope: state.scope,
                eligible: state.alive
                    && self.node.reading_key() == Some(state.root_key.as_str())
                    && self.node.reading_visible(state.scene.as_ref())
                    && state.eligible(window, cx),
            },
            std::mem::size_of::<Tail>(),
        ))
    }
    fn validate_continuation(
        &self,
        state: &dyn Any,
        _: &CaptureTarget,
        witness: &ContinuationWitness,
        window: &mut Window,
        cx: &mut App,
    ) -> maka_ui::Result<bool> {
        let state = owner(state).read(cx);
        Ok(witness.downcast_ref::<Tail>().is_some_and(|proof| {
            proof.eligible
                && state.alive
                && proof.scope == state.scope
                && proof
                    .selection
                    .as_ref()
                    .zip(state.selection.as_ref())
                    .is_some_and(|(old, current)| old.same_owner(current))
                && self.node.reading_key() == Some(state.root_key.as_str())
                && self.node.reading_visible(state.scene.as_ref())
                && proof.scroll.is_current(&state.handle)
                && state.eligible(window, cx)
        }))
    }
    fn render(
        &self,
        state: &dyn Any,
        slots: &Slots,
        window: &mut Window,
        cx: &mut App,
    ) -> maka_ui::Result<AnyElement> {
        let owner = owner(state);
        let (child, scope) = owner.query(cx, |state, cx| {
            let child =
                self.node
                    .render_reading(state.scene.as_ref(), slots, state.scope, window, cx)?;
            maka_ui::Result::Ok((child, state.scope))
        })?;
        Ok(Painted {
            child,
            owner: owner.downgrade(),
        }
        .text_selection_scope(scope)
        .into_any_element())
    }
}
struct Component;
impl NativeComponent<dyn Widget> for Component {
    fn capture(&self, call: CaptureCall<'_>) -> maka_ui::Result<CapturePlan> {
        if call.method != "tail" {
            return Err(maka_ui::Error::Invalid("unknown reading capture".into()));
        }
        Ok(CapturePlan::Native)
    }
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let (props, revision, previous, event) = match call {
            ComponentCall::Mount { props } => (props, 0, None, None),
            ComponentCall::Update { state, props } => (
                props,
                state["revision"].as_u64().unwrap() + 1,
                state.get("command"),
                None,
            ),
            ComponentCall::Event {
                state,
                props,
                key,
                input,
                origin,
            } => (
                props,
                state["revision"].as_u64().unwrap() + 1,
                state.get("command"),
                Some((key, input, origin)),
            ),
        };
        let props: ReadingProps = serde_json::from_value(props.clone())
            .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
        let previous: Option<ReadingCommand> = previous
            .filter(|value| !value.is_null())
            .map(|value| serde_json::from_value(value.clone()).expect("reading command state"));
        if let Some(command) = &props.command
            && (!command.valid_target()
                || command.sequence == 0
                || previous.as_ref().is_some_and(|last| {
                    command.sequence < last.sequence
                        || (command.sequence == last.sequence && command != last)
                }))
        {
            return Err(maka_ui::Error::Invalid(
                "reading command sequence changed or regressed".into(),
            ));
        }
        let mut view = crate::scene::compile_with_inputs(
            &props.content,
            Instant::now() + Duration::from_millis(25),
            BTreeMap::from([(OBSERVED.into(), observation_schema())]),
        )?;
        let node = (view.view.as_ref() as &dyn Any)
            .downcast_ref::<Node>()
            .expect("compiled scene node")
            .clone();
        if node.reading_key().is_none() {
            return Err(maka_ui::Error::Invalid(
                "reading content requires a scrolling column Stack".into(),
            ));
        }
        view.view = Rc::new(Reading {
            node,
            command: props.command.clone(),
        });
        view = view.with_declaration(serde_json::to_value(&props).expect("reading props"))?;
        let events = event
            .map(|(key, input, origin)| match origin {
                EventOrigin::Raw if key == OBSERVED => Emission {
                    name: "observed".into(),
                    value: input.clone(),
                },
                origin => {
                    let input = match origin {
                        EventOrigin::Raw => maka_ui::abi::ComponentInput::Raw {
                            key: key.into(),
                            event: input.clone(),
                        },
                        EventOrigin::Action => maka_ui::abi::ComponentInput::Action {
                            key: key.into(),
                            event: input.clone(),
                        },
                        EventOrigin::Component { child, name } => {
                            maka_ui::abi::ComponentInput::Component {
                                key: key.into(),
                                child: child.into(),
                                name: name.into(),
                                event: input.clone(),
                            }
                        }
                    };
                    Emission {
                        name: "input".into(),
                        value: serde_json::to_value(input).expect("reading child input"),
                    }
                }
            })
            .into_iter()
            .collect();
        Ok(Frame {
            revision,
            state: json!({"revision":revision,"command":props.command.or(previous)}),
            surface: None,
            view,
        }
        .output(events))
    }
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register("maka.ui.reading", |_, _| {
            Ok(BTreeMap::from([(
                "ReadingArea".into(),
                NativeExport {
                    contract: reading_contract(),
                    value: NativeCapability::Component(Rc::new(Component)),
                },
            )]))
        })
        .expect("unique native reading implementation");
}

#[cfg(test)]
mod tests;
