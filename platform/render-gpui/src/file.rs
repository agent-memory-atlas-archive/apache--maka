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

//! Original OS file selection; opaque component props cannot name filesystem paths.
use crate::{
    LiveInput, Slots, Widget,
    appearance::{color, font},
};
use gpui_kit::base::TestSupportExt;
mod press;
use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, IntoElement, ParentElement,
    PathPromptOptions, Render, Styled, Window, div, px,
};
use gpui_kit::{InteractiveElement, StatefulInteractiveElement};
use maka_ui_runtime::{
    ComponentCall, ComponentOutput, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView, budget::Kind, file::*,
};
use press::Press;
use std::{any::Any, collections::BTreeMap, rc::Rc, sync::Arc};

struct Owner {
    input: LiveInput,
    props: FilePickerProps,
    focus: FocusHandle,
    click: Option<gpui_kit::ClickHandle>,
    selection: FileSelection,
    visible: bool,
    active: bool,
    pending: bool,
    needs_publication: bool,
    #[cfg(test)]
    read_failure: bool,
}
impl Owner {
    fn new(props: &FilePickerProps, input: LiveInput, cx: &mut App) -> Entity<Self> {
        let keep = input.0.borrow().as_ref().expect("live file picker").clone();
        cx.new(|cx| {
            cx.on_release(move |_, _| drop(keep)).detach();
            Self {
                input,
                props: props.clone(),
                focus: cx.focus_handle(),
                click: Default::default(),
                selection: FileSelection::default(),
                visible: true,
                active: true,
                pending: false,
                needs_publication: true,
                #[cfg(test)]
                read_failure: false,
            }
        })
    }
    fn enabled(&self) -> bool {
        self.active
            && self.visible
            && !self.props.disabled
            && !self.selection.busy()
            && self.input.0.borrow().is_some()
    }
    fn presentation(&mut self, visible: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.visible = visible;
        let enabled = self.enabled();
        self.focus = self
            .focus
            .clone()
            .tab_stop(enabled && self.props.tab_index >= 0)
            .tab_index(if enabled {
                self.props.tab_index.into()
            } else {
                -1
            });
        if !enabled {
            if let Some(click) = self.click.as_ref() {
                click.cancel();
            }
            self.selection.cancel_intent();
            if self.focus.contains_focused(window, cx) {
                window.blur(cx);
            }
        }
        cx.notify();
    }
    fn apply(&mut self, view: &FileView, window: &mut Window, cx: &mut Context<Self>) {
        self.props = view.props.clone();
        self.presentation(self.visible, window, cx);
        if self.selection.accept(view.request.as_deref()) {
            self.start(window, cx);
            self.presentation(self.visible, window, cx);
        }
        self.schedule(window, cx);
    }
    fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let budget = self.input.1.clone();
        let job = match budget.reserve(Kind::Job, 1) {
            Ok(job) => job,
            Err(error) => {
                self.selection.settle(Err(error.into()));
                self.needs_publication = true;
                return;
            }
        };
        let mut ticket = match self.input.2.reserve() {
            Ok(ticket) => ticket,
            Err(_) => {
                self.selection.settle(Err(FileError::WorkerFailed));
                self.needs_publication = true;
                return;
            }
        };
        let keep = self
            .input
            .0
            .borrow()
            .as_ref()
            .expect("admitted file picker")
            .clone();
        // Custody and aggregate job admission precede creation of the real OS panel.
        ticket.start();
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(self.props.label.clone().into()),
        });
        let executor = cx.background_executor().clone();
        let weak = cx.weak_entity();
        self.needs_publication = true;
        #[cfg(test)]
        let read_failure = self.read_failure;
        window
            .spawn(cx, async move |cx| {
                let response = prompt.await;
                let lost_dialog = response.is_err();
                let (result, cleanup) = match response {
                    Ok(Ok(None)) => (Ok(None), Ok(())),
                    Ok(Ok(Some(paths))) if paths.len() == 1 => {
                        let path = paths.into_iter().next().expect("one selected file");
                        executor
                            .spawn(async move {
                                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    #[cfg(test)]
                                    assert!(!read_failure, "original file worker panic");
                                    read_selected(&path, &budget)
                                })) {
                                    Ok(result) => (result.map(Some), Ok(())),
                                    Err(_) => (
                                        Err(FileError::WorkerFailed),
                                        Err("native file worker panicked".into()),
                                    ),
                                }
                            })
                            .await
                    }
                    Err(_) => (
                        Err(FileError::ReadFailed),
                        Err("native file dialog disappeared".into()),
                    ),
                    _ => (Err(FileError::ReadFailed), Ok(())),
                };
                // Window/observer retirement does not cancel the original dialog or read.
                if lost_dialog {
                    ticket.unconfirmed("native file dialog disappeared".into(), job);
                } else {
                    drop(job);
                    ticket.complete(cleanup);
                }
                let _ = cx.update(move |window, cx| {
                    let _ = weak.update(cx, |owner, cx| {
                        if !owner.active || owner.input.0.borrow().is_none() {
                            return;
                        }
                        owner.selection.settle(result);
                        owner.presentation(owner.visible, window, cx);
                        owner.needs_publication = true;
                        owner.schedule(window, cx);
                        cx.notify();
                    });
                    drop(keep);
                });
            })
            .detach();
    }
    fn schedule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending
            || !self.needs_publication
            || !self.active
            || self.input.0.borrow().is_none()
        {
            return;
        }
        self.pending = true;
        let weak = cx.weak_entity();
        window.defer(cx, move |window, cx| {
            let Ok(Some((input, value))) = weak.update(cx, |owner, _| {
                owner.pending = false;
                if !owner.active || !owner.needs_publication {
                    return None;
                }
                Some((
                    owner.input.0.borrow().clone()?,
                    owner.selection.observation(),
                ))
            }) else {
                return;
            };
            match (input.0)(
                "observe",
                serde_json::to_value(&value).expect("file observation"),
                window,
                cx,
            ) {
                Ok(_) => {
                    let _ = weak.update(cx, |owner, _| {
                        if owner.selection.observation() == value {
                            owner.needs_publication = false;
                        }
                    });
                }
                Err(error) => eprintln!("file observation rejected: {error}"),
            }
        });
    }
}
impl Render for Owner {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let enabled = self.enabled();
        let a = &self.props.appearance;
        let focus = self
            .focus
            .clone()
            .tab_stop(enabled && self.props.tab_index >= 0)
            .tab_index(if enabled {
                self.props.tab_index.into()
            } else {
                -1
            });
        self.focus = focus.clone();
        let weak = cx.weak_entity();
        let down = weak.clone();
        let up = weak.clone();
        let key_down = weak.clone();
        let key_up = weak.clone();
        let a11y = weak.clone();
        let admitted = |weak: &gpui_kit::WeakEntity<Owner>, cx: &App| {
            weak.upgrade().is_some_and(|owner| owner.read(cx).enabled())
        };
        let button = div()
            .id("selected-file")
            .test_support()
            .flex()
            .items_center()
            .justify_center()
            .line_height(gpui_kit::relative(1.))
            .role(gpui_kit::accesskit::Role::Button)
            .aria_label(self.props.label.clone())
            .child(self.props.label.clone())
            .aria_disabled(!enabled)
            .capture_any_mouse_down(move |_, window, cx| {
                if !admitted(&down, cx) {
                    window.prevent_default();
                    cx.stop_propagation();
                }
            })
            .capture_any_mouse_up(move |_, window, cx| {
                if !admitted(&up, cx) {
                    window.prevent_default();
                    cx.stop_propagation();
                }
            })
            .capture_key_down(move |_, window, cx| {
                if !admitted(&key_down, cx) {
                    window.prevent_default();
                    cx.stop_propagation();
                }
            })
            .capture_key_up(move |_, window, cx| {
                if !admitted(&key_up, cx) {
                    window.prevent_default();
                    cx.stop_propagation();
                }
            })
            .on_a11y_action(gpui_kit::AccessibleAction::Focus, move |_, window, cx| {
                if let Some(owner) = a11y.upgrade()
                    && owner.read(cx).enabled()
                {
                    let focus = owner.read(cx).focus.clone();
                    focus.focus(window, cx);
                }
            })
            .track_focus(&focus)
            .tab_index(focus.tab_index)
            .tab_stop(focus.tab_stop)
            .font(font(&a.text.font))
            .text_size(px(a.text.font.size))
            .text_color(color(a.text.foreground))
            .bg(color(a.background))
            .border(px(a.border_width))
            .border_color(color(a.border))
            .rounded(px(a.radius))
            .pt(px(a.padding.top))
            .pr(px(a.padding.right))
            .pb(px(a.padding.bottom))
            .pl(px(a.padding.left))
            .focus(|s| s.border_color(color(a.focused_border)))
            .on_click(move |_, window, cx| {
                let Ok(Some((input, choose))) = weak.update(cx, |owner, _| {
                    if !owner.enabled() {
                        return None;
                    }
                    owner.selection.cancel_intent();
                    Some((owner.input.0.borrow().clone()?, owner.selection.stage()?))
                }) else {
                    return;
                };
                if (input.0)("choose", serde_json::to_value(choose).unwrap(), window, cx).is_err() {
                    let _ = weak.update(cx, |owner, _| owner.selection.cancel_intent());
                }
            });
        div().w_full().child(Press {
            inner: button,
            owner: cx.weak_entity(),
        })
    }
}
struct Recipe(FileView);
impl Widget for Recipe {
    fn create(&self, input: LiveInput, _: &mut Window, cx: &mut App) -> Box<dyn Any> {
        Box::new(Owner::new(&self.0.props, input, cx))
    }
    fn commit(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<Owner>>()
            .unwrap()
            .update(cx, |owner, cx| owner.apply(&self.0, window, cx));
    }
    fn publish_pending(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<Owner>>()
            .unwrap()
            .update(cx, |owner, cx| owner.schedule(window, cx));
    }
    fn presentation(&self, state: &mut dyn Any, visible: bool, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<Owner>>()
            .unwrap()
            .update(cx, |owner, cx| owner.presentation(visible, window, cx));
    }
    fn deactivate(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<Owner>>()
            .unwrap()
            .update(cx, |owner, cx| {
                owner.active = false;
                owner.presentation(false, window, cx);
            });
    }
    fn capture(
        &self,
        state: &mut dyn Any,
        target: &maka_ui_runtime::actions::CaptureTarget,
        _: &mut Window,
        cx: &mut App,
    ) -> maka_ui_runtime::Result<Arc<dyn maka_capabilities::call::ReadResource>> {
        let owner = state.downcast_ref::<Entity<Owner>>().unwrap().read(cx);
        if !owner.active || owner.input.0.borrow().is_none() {
            return Err(maka_ui_runtime::Error::Invalid(
                "file picker retired".into(),
            ));
        }
        owner.selection.capture(
            &serde_json::from_value(target.input().clone())
                .map_err(|e| maka_ui_runtime::Error::Invalid(e.to_string()))?,
        )
    }
    fn render(
        &self,
        state: &dyn Any,
        _: &Slots,
        _: &mut Window,
        _: &mut App,
    ) -> maka_ui_runtime::Result<AnyElement> {
        Ok(state
            .downcast_ref::<Entity<Owner>>()
            .unwrap()
            .clone()
            .into_any_element())
    }
}
struct Component;
impl NativeComponent<dyn Widget> for Component {
    fn capture(
        &self,
        _: maka_ui_runtime::actions::CaptureCall<'_>,
    ) -> maka_ui_runtime::Result<maka_ui_runtime::actions::CapturePlan> {
        Ok(maka_ui_runtime::actions::CapturePlan::Native)
    }
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui_runtime::abi::ValueBindings,
    ) -> maka_ui_runtime::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let output = reduce(call)?;
        let view = NativeView::<dyn Widget>::new(
            Rc::new(Recipe(output.frame.view.clone())),
            BTreeMap::from([
                ("choose".into(), choose_schema()),
                ("observe".into(), observation_schema()),
            ]),
            vec![],
        )?;
        Ok(output.map_view(|_| view))
    }
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register(maka_ui_runtime::file::IMPLEMENTATION, |_, _| {
            Ok(BTreeMap::from([(
                "FilePicker".into(),
                NativeExport {
                    contract: file_picker_contract(),
                    value: NativeCapability::Component(Rc::new(Component)),
                },
            )]))
        })
        .expect("unique native file picker");
}

#[cfg(test)]
mod tests;
