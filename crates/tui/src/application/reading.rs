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

//! A reading boundary shares the original scene's scroll and document owners.
use super::{
    NativeInput, Placement, Slots, Widget, WidgetInput,
    scene::Node,
    scroll::{SceneState, Viewport},
    selection::{ScopePaint, SelectionScope},
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
pub(super) mod find;

const OBSERVED: &str = "__reading_observation";
pub(super) struct Pending {
    pub command: ReadingCommand,
    pub intent: u64,
}
pub(super) struct ReadingState {
    pub root_key: String,
    pub scope: SelectionScope,
    pub pending: Option<Pending>,
    last_command: Option<ReadingCommand>,
    receipt: Option<ReadingReceipt>,
    viewport: Option<ReadingViewport>,
    published: Option<ReadingObservation>,
    alive: bool,
    find: find::State,
    find_intent: Option<u64>,
}
impl ReadingState {
    fn new(root_key: String) -> Self {
        Self {
            root_key,
            scope: SelectionScope::default(),
            pending: None,
            last_command: None,
            receipt: None,
            viewport: None,
            published: None,
            alive: true,
            find: find::State::default(),
            find_intent: None,
        }
    }
    pub fn cancel(&mut self) {
        if let Some(command) = self.pending.take() {
            self.receipt = Some(ReadingReceipt {
                sequence: command.command.sequence,
                status: ReadingStatus::Cancelled,
            });
        }
    }
    pub fn accept(&mut self, viewport: &Viewport) {
        let revision = viewport
            .reading_revision
            .expect("reading viewport revision");
        self.viewport = Some(ReadingViewport {
            reading_revision: revision,
            at_end: viewport.at_end(),
            visible_child_keys: viewport.visible_child_keys.clone(),
        });
        if revision == self.scope.revision()
            && let Some((sequence, applied)) = viewport.command
            && self.pending.as_ref().is_some_and(|pending| {
                pending.command.sequence == sequence
                    && pending.intent == self.scope.intent()
                    && viewport.reading_intent == Some(pending.intent)
            })
        {
            self.pending = None;
            self.receipt = Some(ReadingReceipt {
                sequence,
                status: if applied {
                    ReadingStatus::Applied
                } else {
                    ReadingStatus::Rejected
                },
            });
        }
    }
    pub(super) fn eligible(&self) -> bool {
        self.alive
            && self.scope.0.borrow().enabled
            && self.pending.is_none()
            && !self.find.active()
            && !self.scope.has_selection()
            && !self.scope.has_pointer_capture()
            && self.viewport.as_ref().is_some_and(|viewport| {
                viewport.reading_revision == self.scope.revision() && viewport.at_end
            })
    }
    fn observe(&mut self) -> Option<NativeInput> {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.intent != self.scope.intent())
        {
            self.cancel();
        }
        let observation = ReadingObservation {
            reading_revision: self.scope.revision(),
            selected_child_key: self.scope.selected_child(),
            viewport: self.viewport.clone(),
            pending_command: self
                .pending
                .as_ref()
                .map(|pending| pending.command.sequence),
            receipt: self.receipt.clone(),
            find: self.find.observation.clone(),
        };
        if self.published.as_ref() == Some(&observation) {
            return None;
        }
        self.published = Some(observation.clone());
        Some(NativeInput {
            key: OBSERVED.into(),
            value: serde_json::to_value(observation).expect("reading observation"),
        })
    }
}
struct Reading {
    node: Node,
    command: Option<ReadingCommand>,
}
struct ScopeGeometry {
    generation: u64,
    intent: u64,
    command: Option<u64>,
    paint: ScopePaint,
}
struct Tail {
    revision: u64,
    root_key: String,
    eligible: bool,
}
fn reading_key(node: &Node) -> Option<&str> {
    match node {
        Node::Stack {
            key,
            direction: super::scene::Direction::Column,
            scroll,
            ..
        } if *scroll != super::scene::ScrollPolicy::None => Some(key),
        _ => None,
    }
}
fn owner_state(state: &dyn Any) -> &SceneState {
    state
        .downcast_ref::<SceneState>()
        .expect("reading scene owner")
}
fn state_mut(state: &mut dyn Any) -> &mut SceneState {
    state
        .downcast_mut::<SceneState>()
        .expect("reading scene owner")
}
impl Widget for Reading {
    fn scene(&self) -> Option<&Node> {
        Some(&self.node)
    }
    fn prepare_focus(&self, state: &mut dyn Any, visible: bool) {
        state_mut(state).prepare_dialogs(&self.node, visible);
    }
    fn same_presentation(&self, previous: &dyn Any) -> bool {
        previous
            .downcast_ref::<Self>()
            .is_some_and(|previous| self.node == previous.node && self.command == previous.command)
    }
    fn layout(&self, slots: &Slots<'_>) -> maka_ui::Result<super::TerminalLayout> {
        Widget::layout(&self.node, slots)
    }
    fn create(&self, _: super::NativeContext) -> Box<dyn Any> {
        Box::new(SceneState {
            reading: Some(ReadingState::new(reading_key(&self.node).unwrap().into())),
            ..Default::default()
        })
    }
    fn commit(&self, state: &mut dyn Any) -> maka_ui::Result<Option<NativeInput>> {
        let state = state_mut(state);
        state.commit(&self.node);
        let reading = state.reading.as_mut().unwrap();
        let key = reading_key(&self.node).unwrap();
        if reading.root_key != key {
            reading.cancel();
            reading.scope.close();
            reading.scope.touch();
            reading.viewport = None;
            reading.root_key = key.into();
            reading.find = find::State::default();
            reading.find_intent = None;
        }
        if !self.node.visible() {
            reading.scope.suspend();
            reading.viewport = None;
        }
        if let Some(command) = &self.command
            && reading.last_command.as_ref() != Some(command)
        {
            reading.cancel();
            let stale_find = match command.target {
                ReadingTarget::RevealSource {
                    find_sequence: Some(sequence),
                    ..
                } => {
                    reading
                        .find
                        .observation
                        .as_ref()
                        .is_none_or(|find| find.sequence != sequence)
                        || reading.find_intent != Some(reading.scope.intent())
                }
                _ => false,
            };
            reading.last_command = Some(command.clone());
            if stale_find {
                reading.receipt = Some(ReadingReceipt {
                    sequence: command.sequence,
                    status: ReadingStatus::Cancelled,
                });
                return Ok(None);
            }
            reading.scope.touch();
            if self.node.visible() {
                reading.pending = Some(Pending {
                    command: command.clone(),
                    intent: reading.scope.intent(),
                });
            } else {
                reading.receipt = Some(ReadingReceipt {
                    sequence: command.sequence,
                    status: ReadingStatus::Rejected,
                });
            }
        }
        Ok(None)
    }
    fn observe(&self, state: &mut dyn Any) -> Option<NativeInput> {
        state_mut(state).reading.as_mut().unwrap().observe()
    }
    fn input(
        &self,
        state: &mut dyn Any,
        key: &str,
        event: &crossterm::event::Event,
    ) -> maka_ui::Result<WidgetInput> {
        if !self.node.visible()
            || !owner_state(state)
                .reading
                .as_ref()
                .unwrap()
                .scope
                .0
                .borrow()
                .enabled
        {
            return Ok(WidgetInput::Unhandled);
        }
        self.node.input(state, key, event)
    }
    fn handles_tab(&self, state: &dyn Any, key: &str) -> bool {
        self.node.handles_tab(state, key)
    }
    fn hover(&self, state: &mut dyn Any, key: &str, point: Option<ratatui::layout::Position>) {
        self.node.hover(state, key, point);
    }
    fn present(&self, state: &mut dyn Any, geometry: Box<dyn Any>) -> Option<NativeInput> {
        let state = state_mut(state);
        let geometry = match geometry.downcast::<super::scroll::Geometry>() {
            Ok(geometry) => {
                state.present(*geometry);
                return None;
            }
            Err(geometry) => geometry,
        };
        let geometry = *geometry
            .downcast::<ScopeGeometry>()
            .expect("reading scope geometry");
        if geometry.generation != state.generation {
            return None;
        }
        let reading = state.reading.as_mut().unwrap();
        if !self.node.visible() {
            return None;
        }
        let current = geometry.intent == reading.scope.intent();
        let paint = geometry.paint;
        let command = reading
            .pending
            .as_ref()
            .filter(|pending| {
                current
                    && Some(pending.command.sequence) == geometry.command
                    && pending.intent == geometry.intent
            })
            .map(|pending| pending.command.clone());
        if current
            && let Some(find) = paint.find
            && find.valid()
        {
            reading.find = find.accept();
            if let Some(command) = &command
                && matches!(
                    command.target,
                    ReadingTarget::Find { .. } | ReadingTarget::ClearFind
                )
            {
                reading.pending = None;
                reading.find_intent =
                    matches!(command.target, ReadingTarget::Find { .. }).then_some(geometry.intent);
                reading.receipt = Some(ReadingReceipt {
                    sequence: command.sequence,
                    status: ReadingStatus::Applied,
                });
            }
        }
        if let Some(command) = command
            && matches!(command.target, ReadingTarget::RevealSource { .. })
        {
            let row = paint.reveal.and_then(|reveal| {
                let document = reveal.document.upgrade()?;
                let doc = document.borrow();
                (doc.alive
                    && !doc.disabled
                    && std::sync::Arc::ptr_eq(&doc.source, &reveal.source)
                    && paint
                        .documents
                        .iter()
                        .any(|weak| weak.ptr_eq(&reveal.document)))
                .then_some(reveal.row.map(|row| (row, reveal.visible)))
                .flatten()
            });
            let scroll = state.scrolls.get_mut(&reading.root_key).unwrap();
            if let Some(((row, visible), viewport)) = row
                .zip(scroll.presented.as_ref())
                .filter(|(_, viewport)| viewport.reading_intent == Some(geometry.intent))
            {
                let offset = (i64::from(viewport.offset) + row
                    - viewport.area.y
                    - i64::from(viewport.area.height / 3))
                .clamp(0, i64::from(viewport.end())) as u32;
                if viewport.offset == offset {
                    reading.pending = None;
                    reading.receipt = Some(ReadingReceipt {
                        sequence: command.sequence,
                        status: if visible {
                            ReadingStatus::Applied
                        } else {
                            ReadingStatus::Rejected
                        },
                    });
                } else {
                    scroll.offset = offset;
                    scroll.intent += 1;
                    reading.scope.0.borrow_mut().revision += 1;
                }
            } else {
                reading.pending = None;
                reading.receipt = Some(ReadingReceipt {
                    sequence: command.sequence,
                    status: ReadingStatus::Rejected,
                });
            }
        }
        reading.scope.present(paint.documents, paint.area);
        if current
            && let Some(row) = reading.scope.take_reveal_row()
            && let Some(scroll) = state.scrolls.get_mut(&reading.root_key)
            && let Some(viewport) = &scroll.presented
        {
            let visible = viewport.area.visible();
            let delta = if row < i64::from(visible.y) {
                row - i64::from(visible.y)
            } else {
                (row - i64::from(visible.bottom().saturating_sub(1))).max(0)
            };
            let offset =
                (i64::from(viewport.offset) + delta).clamp(0, i64::from(viewport.end())) as u32;
            if offset != viewport.offset {
                scroll.offset = offset;
                scroll.intent += 1;
                reading.scope.0.borrow_mut().revision += 1;
            }
        }
        reading.observe()
    }
    fn presentation(&self, state: &mut dyn Any, visible: bool) {
        if !visible {
            let reading = state_mut(state).reading.as_mut().unwrap();
            reading.scope.suspend();
            reading.viewport = None;
        }
    }
    fn deactivate(&self, state: &mut dyn Any) {
        state_mut(state).dialogs.clear();
        let reading = state_mut(state).reading.as_mut().unwrap();
        reading.alive = false;
        reading.cancel();
        reading.scope.close();
    }
    fn capture_continuation(
        &self,
        state: &dyn Any,
        _: &CaptureTarget,
    ) -> maka_ui::Result<ContinuationWitness> {
        let reading = owner_state(state).reading.as_ref().unwrap();
        let proof = Tail {
            revision: reading.scope.revision(),
            root_key: reading.root_key.clone(),
            eligible: reading.eligible()
                && self
                    .command
                    .as_ref()
                    .is_none_or(|command| reading.last_command.as_ref() == Some(command))
                && self.node.visible()
                && reading_key(&self.node) == Some(reading.root_key.as_str()),
        };
        let bytes = std::mem::size_of::<Tail>() + proof.root_key.len();
        Ok(ContinuationWitness::new(proof, bytes))
    }
    fn validate_continuation(
        &self,
        state: &dyn Any,
        _: &CaptureTarget,
        witness: &ContinuationWitness,
    ) -> maka_ui::Result<bool> {
        let reading = owner_state(state).reading.as_ref().unwrap();
        Ok(witness.downcast_ref::<Tail>().is_some_and(|proof| {
            proof.eligible
                && reading.eligible()
                && proof.revision == reading.scope.revision()
                && proof.root_key == reading.root_key
                && self
                    .command
                    .as_ref()
                    .is_none_or(|command| reading.last_command.as_ref() == Some(command))
                && self.node.visible()
                && reading_key(&self.node) == Some(reading.root_key.as_str())
        }))
    }
    fn measure(&self, slots: &Slots<'_>, width: u16) -> maka_ui::Result<(u16, u32)> {
        self.node.measure(slots, width)
    }
    fn paint(
        &self,
        slots: &mut Slots<'_>,
        area: Placement,
        buffer: &mut ratatui::buffer::Buffer,
    ) -> maka_ui::Result<()> {
        let state = slots.native_state::<SceneState>()?;
        let generation = state.generation;
        let reading = state.reading.as_ref().unwrap();
        let intent = reading.scope.intent();
        let root = Some((slots.parent.id(), reading.root_key.clone()));
        let pending = reading
            .pending
            .as_ref()
            .filter(|pending| pending.intent == intent);
        let command = pending.map(|pending| pending.command.sequence);
        let (entries, dependencies) = if reading.find.active()
            || pending.is_some_and(|pending| {
                matches!(
                    pending.command.target,
                    ReadingTarget::Find { .. } | ReadingTarget::RevealSource { .. }
                )
            }) {
            find::documents(slots)?
        } else {
            (Vec::new(), Vec::new())
        };
        let reveal = pending.and_then(|pending| match &pending.command.target {
            ReadingTarget::RevealSource { path, offset, .. } => {
                find::Reveal::new(&entries, path, *offset)
            }
            _ => None,
        });
        let find = reading
            .find
            .paint(entries, pending.map(|pending| &pending.command));
        let scope = slots
            .native_state::<SceneState>()?
            .reading
            .as_ref()
            .unwrap()
            .scope
            .clone();
        let padding = match self.node {
            Node::Stack { padding, .. } => padding,
            _ => unreachable!(),
        };
        for mount in dependencies {
            slots.depend_on(&mount)?;
        }
        let scene = slots
            .scene
            .as_deref_mut()
            .ok_or_else(|| maka_ui::Error::Invalid("cannot paint while measuring".into()))?;
        let previous = scene.scope.replace(ScopePaint {
            root,
            child: None,
            scope,
            documents: Vec::new(),
            area: area.inner(padding.into()),
            find: Some(find),
            reveal,
        });
        let result = self.node.paint(slots, area, buffer);
        let scene = slots.scene.as_deref_mut().unwrap();
        let geometry = scene.scope.take().expect("reading paint scope");
        scene.scope = previous;
        result?;
        slots.native_geometry(Box::new(ScopeGeometry {
            generation,
            intent,
            command,
            paint: geometry,
        }))
    }
}
pub(super) struct Component;
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
        let mut view = super::scene::compile_with_inputs(
            &props.content,
            Instant::now() + Duration::from_millis(25),
            BTreeMap::from([(OBSERVED.into(), observation_schema())]),
        )?;
        let node = (view.view.as_ref() as &dyn Any)
            .downcast_ref::<Node>()
            .expect("compiled scene node")
            .clone();
        if reading_key(&node).is_none() {
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
