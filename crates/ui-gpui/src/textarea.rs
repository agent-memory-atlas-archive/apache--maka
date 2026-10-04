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

use super::editing::{NativeSnapshot, TextSnapshot};
use super::{LiveInput, Slots, Widget};
use gpui_kit::base::input::{InputBase, RopeExt, TextareaState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use maka_ui::{
    ComponentCall, ComponentOutput, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView,
};
use maka_ui::{editor::*, textarea::*};
use std::{any::Any, collections::BTreeMap, ops::Range, rc::Rc};
mod accessibility;
mod menu;

pub struct TextareaOwner {
    id: String,
    pub state: Entity<TextareaState>,
    input: LiveInput,
    pub revision: u64,
    snapshot: NativeSnapshot,
    last_request: Option<EditorCommand>,
    sequence: u64,
    pub receipt: Option<EditorReceipt>,
    pending: bool,
    needs_publication: bool,
    visible: bool,
    _observation: Subscription,
    projection: super::appearance::TextareaProjection,
    label: String,
    menu: Option<TextareaMenu>,
    open_menu: Option<menu::OpenMenu>,
    _menu_input: Option<Subscription>,
}
impl TextareaOwner {
    /// Original editor correlation, including across application processes.
    /// The identifier conveys no lookup or read authority.
    pub fn owner_id(&self) -> &str {
        &self.id
    }

    fn new(
        props: &TextareaProps,
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
        let projection = super::appearance::TextareaProjection::new(&props.appearance);
        let state = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx);
            projection.apply(&mut state);
            state.set_readonly(props.read_only, cx);
            if props.disabled && state.presentation().focus_handle().is_focused(window) {
                window.blur(cx);
            }
            state.set_disabled(props.disabled, cx);
            state.set_soft_wrap(props.soft_wrap, window, cx);
            state.set_tab_index(
                if props.disabled {
                    -1
                } else {
                    props.tab_index.into()
                },
                cx,
            );
            state.set_value(props.initial_text.clone(), window, cx);
            state.set_placeholder(props.placeholder.clone(), window, cx);
            state.set_context_menu_enabled(props.menu.is_some());
            // Entity references release before GPUI reclaims their values.
            // Keep the resource lease until Kit's actual release processing.
            cx.on_release(move |_, _| drop(keep)).detach();
            state
        });
        let owner = cx.new(|cx: &mut Context<Self>| {
            let snapshot = Self::read_snapshot(&state, window, cx);
            let observation = cx.observe_in(&state, window, |owner, _, window, cx| {
                owner.refresh(false, window, cx);
                // Native focus/editability also change the facade and AX tree,
                // even when buffer/selection metadata is unchanged.
                cx.notify();
            });
            Self {
                id: uuid::Uuid::new_v4().to_string(),
                state,
                input,
                revision: 0,
                snapshot,
                last_request: None,
                sequence: 0,
                receipt: None,
                pending: false,
                needs_publication: true,
                visible: true,
                _observation: observation,
                projection,
                label: props.accessibility_label.clone(),
                menu: props.menu.clone(),
                open_menu: None,
                _menu_input: None,
            }
        });
        let subscription = Self::install_menu_input(&owner, cx);
        owner.update(cx, |owner, _| owner._menu_input = Some(subscription));
        owner.update(cx, |owner, cx| owner.schedule(window, cx));
        owner
    }
    fn read_snapshot(
        state: &Entity<TextareaState>,
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
            self.needs_publication = true;
            self.revision = self.revision.checked_add(1).expect("editor revision");
            self.schedule(window, cx);
        }
    }
    fn schedule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending || !self.needs_publication || self.input.0.borrow().is_none() {
            return;
        }
        self.pending = true;
        let owner = cx.weak_entity();
        // Leave the owner/entity borrow before component dispatch can sync it.
        window.defer(cx, move |window, cx| {
            let Ok(Some((input, observation))) = owner.update(cx, |owner, cx| {
                owner.refresh(false, window, cx);
                owner.pending = false;
                if !owner.needs_publication {
                    return None;
                }
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
            if let Err(error) = (input.0)(
                "observe",
                serde_json::to_value(&observation).expect("editor observation"),
                window,
                cx,
            ) {
                // This observation follows native input. It cannot cancel an
                // already completed event or affect a later event's defaults.
                eprintln!("editor observation rejected: {error}");
            } else {
                let _ = owner.update(cx, |owner, _| {
                    if owner.revision == observation.revision
                        && owner.receipt == observation.receipt
                    {
                        owner.needs_publication = false;
                    }
                });
            }
        });
    }
    fn apply(&mut self, props: &TextareaProps, window: &mut Window, cx: &mut Context<Self>) {
        // Notify observers may still be queued after keyboard/selection input.
        // This synchronous barrier is authoritative for command admission.
        self.refresh(false, window, cx);
        let appearance_changed = self.projection.data != props.appearance;
        if appearance_changed {
            self.projection = super::appearance::TextareaProjection::new(&props.appearance);
            cx.notify();
        }
        if self.label != props.accessibility_label {
            self.label = props.accessibility_label.clone();
            cx.notify();
        }
        if self.menu != props.menu {
            self.menu = props.menu.clone();
            cx.notify();
        }
        if props.disabled || !self.visible || props.menu.is_none() {
            self.close_menu(true, window, cx);
        }
        self.state.update(cx, |state, cx| {
            if appearance_changed {
                self.projection.apply(state);
                cx.notify();
            }
            state.set_readonly(props.read_only, cx);
            let disabled = props.disabled || !self.visible;
            if disabled && state.presentation().focus_handle().is_focused(window) {
                window.blur(cx);
            }
            state.set_disabled(disabled, cx);
            state.set_context_menu_enabled(self.visible && props.menu.is_some());
            state.set_soft_wrap(props.soft_wrap, window, cx);
            state.set_tab_index(if disabled { -1 } else { props.tab_index.into() }, cx);
            state.set_placeholder(props.placeholder.clone(), window, cx);
        });
        let Some(command) = &props.command else {
            return;
        };
        if self.last_request.as_ref() == Some(command) {
            return;
        }
        self.last_request = Some(command.clone());
        let outcome = if command.owner != self.id {
            EditorOutcome::Conflict {
                reason: EditorConflict::Owner,
            }
        } else if command.sequence <= self.sequence {
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
        self.needs_publication = true;
        self.receipt = Some(EditorReceipt {
            owner: command.owner.clone(),
            sequence: command.sequence,
            revision: self.revision,
            outcome,
        });
        self.schedule(window, cx);
    }
}
impl Render for TextareaOwner {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().hidden().into_any_element();
        }
        let menu = self.render_menu(window, cx);
        let state = self.state.read(cx);
        let presentation = state.presentation();
        let focused = presentation.focus_handle().is_focused(window);
        let focus = presentation.focus_handle().clone();
        let disabled = presentation.is_disabled();
        let readonly = presentation.is_readonly();
        let placeholder = presentation.placeholder().clone();
        let native_text = state.text_accessibility();
        let selection_owner = cx.weak_entity();
        let value = (window.is_a11y_active() || cfg!(test)).then(|| state.text().to_string());
        let appearance = &self.projection.data;
        let focus_state = self.state.clone();
        let edit_owner = cx.weak_entity();
        let focus_input = self.input.clone();
        let mouse_state = self.state.clone();
        let mouse_input = self.input.clone();
        let mouse_up_state = self.state.clone();
        let mouse_up_input = self.input.clone();
        let content = InputBase::new(("textarea", self.state.entity_id()))
            .size_full()
            .flex()
            .track_focus(&focus)
            .focused(focused)
            .disabled(disabled)
            .capture_any_mouse_down(move |event, window, cx| {
                // A commit can disable/remove this owner before the next paint.
                // Admit against live native facts, never a prior frame's flags.
                if mouse_input.0.borrow().is_none()
                    || mouse_state.read(cx).presentation().is_disabled()
                    || (event.button == MouseButton::Right
                        && mouse_state
                            .update(cx, |s, cx| s.marked_text_range(window, cx))
                            .is_some())
                {
                    cx.stop_propagation();
                }
            })
            .capture_any_mouse_up(move |_, _, cx| {
                if mouse_up_input.0.borrow().is_none()
                    || mouse_up_state.read(cx).presentation().is_disabled()
                {
                    cx.stop_propagation();
                }
            })
            .role(Role::MultilineTextInput)
            .aria_label(self.label.clone())
            .aria_placeholder(placeholder)
            .aria_disabled(disabled)
            .aria_readonly(readonly)
            .when_some(value, |this, value| this.aria_value(value))
            .on_a11y_action(AccessibleAction::Focus, move |_, window, cx| {
                if focus_input.0.borrow().is_some() {
                    focus_state.update(cx, |state, cx| {
                        if !state.presentation().is_disabled() {
                            state.focus(window, cx);
                        }
                    });
                }
            })
            .when(!disabled && !readonly, |this| {
                this.on_a11y_action(AccessibleAction::SetValue, move |data, window, cx| {
                    if let Some(accesskit::ActionData::Value(value)) = data {
                        let _ = edit_owner.update(cx, |owner, cx| {
                            if owner.input.0.borrow().is_none()
                                || owner.open_menu.is_some()
                                || !owner.state.read(cx).is_editable()
                                || owner
                                    .state
                                    .update(cx, |state, cx| state.marked_text_range(window, cx))
                                    .is_some()
                            {
                                return;
                            }
                            owner.state.update(cx, |state, cx| {
                                state.replace_all(value.clone(), window, cx);
                            });
                            owner.refresh(false, window, cx);
                        });
                    }
                })
            })
            .font(self.projection.font.clone())
            .text_size(px(appearance.text.font.size))
            .line_height(relative(appearance.text.font.line_height))
            .text_color(super::appearance::color(appearance.text.foreground))
            .bg(super::appearance::color(appearance.background))
            .border(px(appearance.border_width))
            .border_color(super::appearance::color(if focused {
                appearance.focused_border
            } else {
                appearance.border
            }))
            .rounded(px(appearance.radius))
            .child(self.state.clone());
        let content = accessibility::attach(content, native_text, selection_owner);
        Proxy {
            content: div()
                .size_full()
                .relative()
                .child(content)
                .children(menu)
                .into_any_element(),
            owner: cx.entity(),
        }
        .into_any_element()
    }
}

/// Register the proxy after Kit's handler, retaining the same editing engine.
/// The proxy observes composition and fences new platform input against the
/// live owner, actual native focus, editability and local menu navigation.
struct Proxy {
    content: AnyElement,
    owner: Entity<TextareaOwner>,
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
impl EntityInputHandler for TextareaOwner {
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
        if !self.accepts_text_input(window, cx) {
            return;
        }
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
        if !self.accepts_text_input(window, cx) {
            return;
        }
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
        if self.input.0.borrow().is_none()
            || self.open_menu.is_some()
            || !self
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
            || self.state.read(cx).presentation().is_disabled()
        {
            return;
        }
        self.state
            .update(cx, |s, cx| s.set_selected_text_range(r, window, cx));
        self.refresh(false, window, cx);
    }
    fn text_length_utf16(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<usize> {
        self.state
            .update(cx, |s, cx| s.text_length_utf16(window, cx))
    }
    fn accepts_text_input(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.input.0.borrow().is_some()
            && self.open_menu.is_none()
            && self.state.read(cx).is_editable()
            && self
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
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

struct TextareaRecipe {
    props: TextareaProps,
}
impl Widget for TextareaRecipe {
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
            .downcast_ref::<Entity<TextareaOwner>>()
            .expect("editor owner")
            .update(cx, |owner, cx| {
                if owner.input.0.borrow().is_none() {
                    return Err(maka_ui::Error::Invalid("editor closed".into()));
                }
                owner.refresh(false, window, cx);
                if let EditorCapture::AtRevision {
                    owner: ref expected,
                    revision,
                } = input
                    && (expected != &owner.id || revision != owner.revision)
                {
                    return Err(maka_ui::Error::Invalid("editor revision changed".into()));
                }
                Ok(std::sync::Arc::new(TextSnapshot {
                    owner: owner.id.clone(),
                    text: owner.snapshot.text.clone(),
                    revision: owner.revision,
                })
                    as std::sync::Arc<dyn maka_plugins::call::ReadResource>)
            })
    }

    fn create(&self, input: LiveInput, window: &mut Window, cx: &mut App) -> Box<dyn Any> {
        Box::new(TextareaOwner::new(&self.props, input, window, cx))
    }
    fn commit(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<TextareaOwner>>()
            .expect("editor native owner")
            .update(cx, |owner, cx| owner.apply(&self.props, window, cx));
    }
    fn publish_pending(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<TextareaOwner>>()
            .expect("editor owner")
            .update(cx, |owner, cx| owner.schedule(window, cx));
    }
    fn presentation(&self, state: &mut dyn Any, visible: bool, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<TextareaOwner>>()
            .expect("editor owner")
            .update(cx, |owner, cx| {
                owner.visible = visible;
                cx.notify();
                owner.state.update(cx, |state, cx| {
                    let disabled = self.props.disabled || !visible;
                    if disabled && state.presentation().focus_handle().is_focused(window) {
                        window.blur(cx);
                    }
                    state.set_disabled(disabled, cx);
                    state.set_tab_index(
                        if disabled {
                            -1
                        } else {
                            self.props.tab_index.into()
                        },
                        cx,
                    );
                    state.set_context_menu_enabled(visible && self.props.menu.is_some());
                });
                if !visible {
                    owner.close_menu(true, window, cx);
                }
            });
    }
    fn deactivate(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        let owner = state
            .downcast_ref::<Entity<TextareaOwner>>()
            .expect("editor owner");
        owner.update(cx, |owner, cx| {
            // The original LiveInput has already been revoked. Closing the
            // menu clears its own focus without restoring the retired editor.
            owner.close_menu(true, window, cx);
            if owner
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
            {
                window.blur(cx);
            }
        });
    }
    fn render(
        &self,
        state: &dyn Any,
        _: &Slots,
        _: &mut Window,
        _: &mut App,
    ) -> maka_ui::Result<AnyElement> {
        Ok(state
            .downcast_ref::<Entity<TextareaOwner>>()
            .expect("editor native owner")
            .clone()
            .into_any_element())
    }
}
struct TextareaComponent;
impl NativeComponent<dyn Widget> for TextareaComponent {
    fn capture(
        &self,
        _: maka_ui::actions::CaptureCall<'_>,
    ) -> maka_ui::Result<maka_ui::actions::CapturePlan> {
        Ok(maka_ui::actions::CapturePlan::Native)
    }

    fn call(
        &self,
        call: ComponentCall<'_>,
        _values: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let output = maka_ui::editor::reduce::<TextareaProps>(call)?;
        let view = NativeView::<dyn Widget>::new(
            Rc::new(TextareaRecipe {
                props: output.frame.view.clone(),
            }),
            BTreeMap::from([("observe".into(), observation_schema())]),
            vec![],
        )?;
        Ok(output.map_view(|_| view))
    }
}

pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register("maka.ui.textarea", |_, _values| {
            Ok(BTreeMap::from([(
                "Textarea".into(),
                NativeExport {
                    contract: textarea_contract(),
                    value: NativeCapability::Component(Rc::new(TextareaComponent)),
                },
            )]))
        })
        .expect("unique native editor");
}
