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

use super::{LiveInput, Widget};
use gpui_kit::component::input::{Editor as KitEditor, EditorState, Rope, RopeExt};
use gpui_kit::*;
use maka_ui::editor::*;
use maka_ui::{
    ComponentCall, ComponentOutput, Emission, Frame, NativeCapability, NativeComponent,
    NativeExport, NativeRegistry, NativeView, Presentation,
};
use serde_json::json;
use std::{any::Any, collections::BTreeMap, ops::Range, rc::Rc};

/// Kit alone owns the buffer, selections and composition. This owner records
/// observations and causality, not a second writable copy of the text.
#[derive(PartialEq)]
struct NativeSnapshot {
    text: Rope,
    selection: TextRange,
    reversed: bool,
    marked: Option<TextRange>,
}

pub(crate) struct EditorOwner {
    id: String,
    pub state: Entity<EditorState>,
    input: LiveInput,
    pub revision: u64,
    snapshot: NativeSnapshot,
    last_request: Option<EditorCommand>,
    sequence: u64,
    pub receipt: Option<EditorReceipt>,
    pending: bool,
    _observation: Subscription,
}
impl EditorOwner {
    fn new(
        props: &EditorProps,
        input: LiveInput,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let keep = input
            .0
            .borrow()
            .as_ref()
            .expect("live editor mount")
            .clone();
        let state = cx.new(|cx| {
            let mut state = EditorState::new(window, cx);
            state.set_value(props.initial_text.clone(), window, cx);
            state.set_placeholder(props.placeholder.clone(), window, cx);
            // Entity references release before GPUI reclaims their values.
            // Keep the resource lease until Kit's actual release processing.
            cx.on_release(move |_, _| drop(keep)).detach();
            state
        });
        cx.new(|cx: &mut Context<Self>| {
            let snapshot = Self::read_snapshot(&state, window, cx);
            let observation = cx.observe_in(&state, window, |owner, _, window, cx| {
                owner.refresh(false, window, cx)
            });
            Self {
                id: cx.entity_id().to_string(),
                state,
                input,
                revision: 0,
                snapshot,
                last_request: None,
                sequence: 0,
                receipt: None,
                pending: false,
                _observation: observation,
            }
        })
    }
    fn read_snapshot(
        state: &Entity<EditorState>,
        window: &mut Window,
        cx: &mut App,
    ) -> NativeSnapshot {
        state.update(cx, |state, cx| {
            let selected = state
                .selected_text_range(true, window, cx)
                .expect("native selection");
            NativeSnapshot {
                text: state.text().clone(),
                selection: TextRange {
                    start: selected.range.start,
                    end: selected.range.end,
                },
                reversed: selected.reversed,
                marked: state.marked_text_range(window, cx).map(|r| TextRange {
                    start: r.start,
                    end: r.end,
                }),
            }
        })
    }
    fn observation_snapshot(&self) -> EditorSnapshot {
        EditorSnapshot {
            text: EditorTextRef {
                owner: self.id.clone(),
                revision: self.revision,
                bytes: self.snapshot.text.len(),
            },
            selection: self.snapshot.selection.clone(),
            reversed: self.snapshot.reversed,
            marked: self.snapshot.marked.clone(),
        }
    }
    /// This reference is meaningful only against this already-bound native
    /// owner. There is no global resource lookup or authority in its string id.
    pub fn read_text(
        &mut self,
        reference: &EditorTextRef,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<String, EditorReadError> {
        if self.input.0.borrow().is_none() || reference.owner != self.id {
            return Err(EditorReadError::Owner);
        }
        self.refresh(false, window, cx);
        if reference.revision != self.revision {
            return Err(EditorReadError::Revision);
        }
        if range
            .end
            .checked_sub(range.start)
            .is_none_or(|len| len > 64 * 1024)
        {
            return Err(EditorReadError::Range);
        }
        self.snapshot
            .text
            .try_slice(range)
            .map(|slice| slice.to_string())
            .map_err(|_| EditorReadError::Range)
    }
    fn refresh(&mut self, force: bool, window: &mut Window, cx: &mut Context<Self>) {
        let snapshot = Self::read_snapshot(&self.state, window, cx);
        if force || snapshot != self.snapshot {
            self.snapshot = snapshot;
            self.revision = self.revision.checked_add(1).expect("editor revision");
            self.schedule(window, cx);
        }
    }
    fn schedule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending {
            return;
        }
        self.pending = true;
        let owner = cx.weak_entity();
        // Leave the owner/entity borrow before component dispatch can sync it.
        window.defer(cx, move |window, cx| {
            let Ok(Some((input, observation))) = owner.update(cx, |owner, cx| {
                owner.refresh(false, window, cx);
                owner.pending = false;
                let input = owner.input.0.borrow().clone()?;
                let value = EditorObservation {
                    revision: owner.revision,
                    snapshot: owner.observation_snapshot(),
                    receipt: owner.receipt.clone(),
                };
                Some((input, value))
            }) else {
                return;
            };
            (input.0)(
                "observe",
                serde_json::to_value(observation).expect("editor observation"),
                window,
                cx,
            );
        });
    }
    fn apply(&mut self, props: &EditorProps, window: &mut Window, cx: &mut Context<Self>) {
        // Notify observers may still be queued after keyboard/selection input.
        // This synchronous barrier is authoritative for command admission.
        self.refresh(false, window, cx);
        self.state.update(cx, |state, cx| {
            state.set_placeholder(props.placeholder.clone(), window, cx)
        });
        let Some(command) = &props.command else {
            return;
        };
        if self.last_request.as_ref() == Some(command) {
            return;
        }
        self.last_request = Some(command.clone());
        let outcome = if command.sequence <= self.sequence {
            EditorOutcome::Conflict {
                reason: EditorConflict::Sequence,
            }
        } else {
            self.sequence = command.sequence;
            if command.based_on != self.revision {
                EditorOutcome::Conflict {
                    reason: EditorConflict::Revision,
                }
            } else if self.snapshot.marked.is_some() {
                EditorOutcome::Conflict {
                    reason: EditorConflict::Composition,
                }
            } else {
                self.state.update(cx, |state, cx| match &command.edit {
                    EditorEdit::Replace { text } => state.replace_all(text.clone(), window, cx),
                    EditorEdit::Reset { text } => state.set_value(text.clone(), window, cx),
                });
                self.refresh(true, window, cx);
                EditorOutcome::Applied
            }
        };
        self.receipt = Some(EditorReceipt {
            sequence: command.sequence,
            revision: self.revision,
            outcome,
        });
        self.schedule(window, cx);
    }
}
impl Render for EditorOwner {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Proxy {
            content: KitEditor::new(&self.state).h(px(120.)).into_any_element(),
            owner: cx.entity(),
        }
    }
}

/// Register the proxy after Kit's handler, retaining the same editing engine.
/// Only the proxy adds the missing mark/unmark observation boundary.
struct Proxy {
    content: AnyElement,
    owner: Entity<EditorOwner>,
}
impl IntoElement for Proxy {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Proxy {
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
        (self.content.request_layout(window, cx), ())
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
        self.content.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.content.paint(window, cx);
        let focus = self.owner.read(cx).state.read(cx).focus_handle(cx);
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.owner.clone()),
            cx,
        );
    }
}
impl EntityInputHandler for EditorOwner {
    fn text_for_range(
        &mut self,
        r: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        self.refresh(false, window, cx);
        let range = self.snapshot.text.offset_utf16_to_offset(r.start)
            ..self.snapshot.text.offset_utf16_to_offset(r.end);
        let reference = self.observation_snapshot().text;
        let text = self.read_text(&reference, range.clone(), window, cx).ok()?;
        *adjusted = Some(
            self.snapshot.text.offset_to_offset_utf16(range.start)
                ..self.snapshot.text.offset_to_offset_utf16(range.end),
        );
        Some(text)
    }
    fn selected_text_range(
        &mut self,
        ignore: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        self.state
            .update(cx, |s, cx| s.selected_text_range(ignore, window, cx))
    }
    fn marked_text_range(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.state
            .update(cx, |s, cx| s.marked_text_range(window, cx))
    }
    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| s.unmark_text(window, cx));
        self.refresh(false, window, cx);
    }
    fn replace_text_in_range(
        &mut self,
        r: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state
            .update(cx, |s, cx| s.replace_text_in_range(r, text, window, cx));
        self.refresh(false, window, cx);
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        r: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.update(cx, |s, cx| {
            s.replace_and_mark_text_in_range(r, text, selected, window, cx)
        });
        self.refresh(false, window, cx);
    }
    fn bounds_for_range(
        &mut self,
        r: Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        self.state
            .update(cx, |s, cx| s.bounds_for_range(r, bounds, window, cx))
    }
    fn character_index_for_point(
        &mut self,
        p: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<usize> {
        self.state
            .update(cx, |s, cx| s.character_index_for_point(p, window, cx))
    }
    fn set_selected_text_range(
        &mut self,
        r: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state
            .update(cx, |s, cx| s.set_selected_text_range(r, window, cx));
        self.refresh(false, window, cx);
    }
    fn text_length_utf16(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<usize> {
        self.state
            .update(cx, |s, cx| s.text_length_utf16(window, cx))
    }
    fn accepts_text_input(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.state
            .update(cx, |s, cx| s.accepts_text_input(window, cx))
    }
    fn text_input_configuration(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> TextInputConfiguration {
        self.state
            .update(cx, |s, cx| s.text_input_configuration(window, cx))
    }
    fn text_input_editable_range(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.state
            .update(cx, |s, cx| s.text_input_editable_range(window, cx))
    }
}

struct EditorRecipe {
    props: EditorProps,
}
struct TextSnapshot {
    text: Rope,
    revision: u64,
}
impl maka_plugins::call::ReadResource for TextSnapshot {
    fn resource_type(&self) -> maka_plugin_core::contract::ResourceType {
        text_resource_type()
    }
    fn bytes(&self) -> usize {
        self.text.len()
    }
    fn metadata(&self) -> serde_json::Value {
        json!({"revision":self.revision,"bytes":self.text.len()})
    }
    fn read(&self, range: Range<usize>) -> Result<Vec<u8>, String> {
        if range
            .end
            .checked_sub(range.start)
            .is_none_or(|len| len > 64 * 1024)
        {
            return Err("invalid text range or chunk exceeds 64 KiB".into());
        }
        self.text
            .try_slice(range)
            .map(|s| s.to_string().into_bytes())
            .map_err(|_| "invalid UTF-8 range".into())
    }
}
impl Widget for EditorRecipe {
    fn capture(
        &self,
        state: &mut dyn Any,
        target: &maka_ui::actions::CaptureTarget,
        window: &mut Window,
        cx: &mut App,
    ) -> maka_ui::Result<std::sync::Arc<dyn maka_plugins::call::ReadResource>> {
        if target.method() != "snapshot" {
            return Err(maka_ui::Error::Invalid("unknown editor capture".into()));
        }
        let input: EditorCapture = serde_json::from_value(target.input().clone())
            .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
        state
            .downcast_ref::<Entity<EditorOwner>>()
            .expect("editor owner")
            .update(cx, |owner, cx| {
                if owner.input.0.borrow().is_none() {
                    return Err(maka_ui::Error::Invalid("editor closed".into()));
                }
                owner.refresh(false, window, cx);
                if let EditorCapture::AtRevision { revision } = input
                    && revision != owner.revision
                {
                    return Err(maka_ui::Error::Invalid("editor revision changed".into()));
                }
                Ok(std::sync::Arc::new(TextSnapshot {
                    text: owner.snapshot.text.clone(),
                    revision: owner.revision,
                })
                    as std::sync::Arc<dyn maka_plugins::call::ReadResource>)
            })
    }

    fn create(&self, input: LiveInput, window: &mut Window, cx: &mut App) -> Box<dyn Any> {
        Box::new(EditorOwner::new(&self.props, input, window, cx))
    }
    fn commit(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<EditorOwner>>()
            .expect("editor native owner")
            .update(cx, |owner, cx| owner.apply(&self.props, window, cx));
    }
    fn render(&self, state: &dyn Any, _: &mut Window, _: &mut App) -> AnyElement {
        state
            .downcast_ref::<Entity<EditorOwner>>()
            .expect("editor native owner")
            .clone()
            .into_any_element()
    }
}
struct EditorComponent;
impl NativeComponent<dyn Widget> for EditorComponent {
    fn capture(
        &self,
        _: maka_ui::actions::CaptureCall<'_>,
    ) -> maka_ui::Result<maka_ui::actions::CapturePlan> {
        Ok(maka_ui::actions::CapturePlan::Native)
    }

    fn call(
        &self,
        call: ComponentCall<'_>,
    ) -> maka_ui::Result<ComponentOutput<Presentation<dyn Widget>>> {
        let (props, revision, observation, mut request, mut pending) = match call {
            ComponentCall::Mount { props } => {
                (props.clone(), 0, serde_json::Value::Null, None, false)
            }
            ComponentCall::Update { state, props } => (
                props.clone(),
                state["revision"].as_u64().expect("editor UI revision") + 1,
                state["observation"].clone(),
                serde_json::from_value::<Option<EditorCommand>>(state["request"].clone())
                    .expect("editor requested command"),
                state["pending"].as_bool().expect("editor pending command"),
            ),
            ComponentCall::Event {
                state,
                props,
                input,
                ..
            } => {
                let request =
                    serde_json::from_value::<Option<EditorCommand>>(state["request"].clone())
                        .expect("editor requested command");
                let observation: EditorObservation =
                    serde_json::from_value(input.clone()).expect("validated editor observation");
                let settled = request
                    .as_ref()
                    .zip(observation.receipt.as_ref())
                    .is_some_and(|(command, receipt)| command.sequence == receipt.sequence);
                return Self::frame(
                    props.clone(),
                    state["revision"].as_u64().expect("editor UI revision") + 1,
                    input.clone(),
                    request,
                    state["pending"].as_bool().expect("editor pending command") && !settled,
                )
                .map(|mut output| {
                    output.events.push(Emission {
                        name: "changed".into(),
                        value: input.clone(),
                    });
                    output
                });
            }
        };
        let typed: EditorProps = serde_json::from_value(props.clone())
            .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
        if let Some(command) = &typed.command
            && request.as_ref() != Some(command)
        {
            if pending {
                return Err(maka_ui::Error::Invalid(
                    "editor prior command has not settled".into(),
                ));
            }
            if request
                .as_ref()
                .is_some_and(|prior| command.sequence <= prior.sequence)
            {
                return Err(maka_ui::Error::Invalid(
                    "editor command sequence must advance".into(),
                ));
            }
            request = Some(command.clone());
            pending = true;
        }
        Self::frame(props, revision, observation, request, pending)
    }
}
impl EditorComponent {
    fn frame(
        props: serde_json::Value,
        revision: u64,
        observation: serde_json::Value,
        request: Option<EditorCommand>,
        pending: bool,
    ) -> maka_ui::Result<ComponentOutput<Presentation<dyn Widget>>> {
        let mut typed: EditorProps =
            serde_json::from_value(props).map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
        // Props propose new commands; the accepted request remains authoritative
        // until its receipt commits. None does not cancel a staged request.
        typed.command = request.clone().filter(|_| pending);
        Ok(Frame {
            revision,
            state: json!({"revision":revision,"request":request,"pending":pending,"observation":observation}),
            window: None,
            view: Presentation::Native(NativeView::<dyn Widget>::new(
                Rc::new(EditorRecipe { props: typed }),
                BTreeMap::from([("observe".into(), maka_ui::editor::observation_schema())]),
            )?),
        }.output(Vec::new()))
    }
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register("maka.ui.editor", |_| {
            Ok(BTreeMap::from([(
                "Editor".into(),
                NativeExport {
                    contract: editor_contract(),
                    value: NativeCapability::Component(Rc::new(EditorComponent)),
                },
            )]))
        })
        .expect("unique native editor");
}
