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

#[cfg(test)]
extern crate gpui_kit as gpui;

mod appearance;
pub use maka_ui::budget::{BudgetLimits, NativeBudget};
mod editing;
mod file;
mod image;
pub mod input;
mod markdown;
mod reading;
pub mod scene;
mod selection_menu;
mod source;
pub mod textarea;

use gpui_kit::{
    AnyElement, App, IntoElement, ParentElement, SharedString, Styled, Window, div, px,
};
use maka_ui::{
    ComponentCall, ComponentOutput, Frame, LabelProps, NativeCapability, NativeComponent,
    NativeExport, NativeRegistry, NativeView, label_contract,
};
use serde_json::{Value, json};
use std::{any::Any, cell::RefCell, collections::BTreeMap, rc::Rc};

/// The presenter supplies its native view ABI; the plugin kernel never depends
/// on GPUI. Only native drawing happens here. Input bindings retain their mount
/// lease and invoke the component adapter later, during native event delivery.
/// A Widget is immutable recipe data: do not cache Entity/Input/subscription
/// handles here. Native state belongs to renderer owners and GPUI retained state;
/// its actual destruction releases leases independently of the retired frame.
pub trait Widget: Any {
    /// Allocate custody without taking focus. The presenter applies actual
    /// visibility, including initially hidden owners, before the first commit.
    fn create(&self, _: LiveInput, _: &mut Window, _: &mut App) -> Box<dyn Any> {
        Box::new(())
    }
    fn capture(
        &self,
        _: &mut dyn Any,
        _: &maka_ui::actions::CaptureTarget,
        _: &mut Window,
        _: &mut App,
    ) -> maka_ui::Result<std::sync::Arc<dyn maka_plugins::call::ReadResource>> {
        Err(maka_ui::Error::Invalid("widget capture unavailable".into()))
    }
    fn commit(&self, _: &mut dyn Any, _: &mut Window, _: &mut App) {}
    fn capture_continuation(
        &self,
        _: &dyn Any,
        _: &maka_ui::actions::CaptureTarget,
        _: &mut Window,
        _: &mut App,
    ) -> maka_ui::Result<maka_ui::actions::ContinuationWitness> {
        Err(maka_ui::Error::Invalid(
            "widget continuation unavailable".into(),
        ))
    }
    /// Read native evidence only. This runs before the feedback transaction;
    /// it must not dispatch input or publish observations from inside the check.
    fn validate_continuation(
        &self,
        _: &dyn Any,
        _: &maka_ui::actions::CaptureTarget,
        _: &maka_ui::actions::ContinuationWitness,
        _: &mut Window,
        _: &mut App,
    ) -> maka_ui::Result<bool> {
        Ok(false)
    }
    /// Reversible mechanical visibility. This does not retire the owner, revoke
    /// capture, or stop native observation and command receipt settlement.
    fn presentation(&self, _: &mut dyn Any, _: bool, _: &mut Window, _: &mut App) {}
    /// Attempt retained native observations after a real synchronization,
    /// including when the committed child frame is unchanged. No effects replay.
    fn publish_pending(&self, _: &mut dyn Any, _: &mut Window, _: &mut App) {}
    /// Close mechanical input/focus on the original window before resource
    /// release. Cached listeners may still retain the old owner during cleanup.
    fn deactivate(&self, _: &mut dyn Any, _: &mut Window, _: &mut App) {}
    fn render(
        &self,
        state: &dyn Any,
        slots: &Slots,
        window: &mut Window,
        cx: &mut App,
    ) -> maka_ui::Result<AnyElement>;
}
/// Owner-originated observations resolve the current dispatcher at delivery;
/// they are not button events against a retained old scene. Removal clears it
/// before dropping the owner, so delayed observations cannot retarget a new one.
#[derive(Clone)]
pub struct LiveInput(
    pub Rc<RefCell<Option<Input>>>,
    pub NativeBudget,
    pub std::sync::Arc<maka_plugins::call::Resources>,
);
impl LiveInput {
    pub fn new(
        input: Rc<RefCell<Option<Input>>>,
        budget: NativeBudget,
        resources: std::sync::Arc<maka_plugins::call::Resources>,
    ) -> Self {
        Self(input, budget, resources)
    }
}
type Dispatch =
    dyn Fn(&str, Value, &mut Window, &mut App) -> maka_ui::Result<maka_ui::InputDecision>;
#[derive(Clone)]
pub struct Input(pub Rc<Dispatch>);
impl Input {
    pub fn bind<E: 'static>(
        &self,
        key: String,
        encode: impl Fn(&E) -> Value + 'static,
    ) -> impl Fn(&E, &mut Window, &mut App) + 'static {
        let dispatch = self.0.clone();
        move |event, window, cx| {
            let decision = match dispatch(&key, encode(event), window, cx) {
                Ok(decision) => decision,
                Err(error) => {
                    eprintln!("component input rejected: {error}");
                    maka_ui::InputDecision {
                        prevent_default: true,
                        stop_propagation: true,
                    }
                }
            };
            if decision.prevent_default {
                window.prevent_default();
            }
            if decision.stop_propagation {
                cx.stop_propagation();
            }
        }
    }
}

/// A read-only renderer handle for the exact committed children of one parent.
/// This may be retained by virtual-list callbacks. Rendering does not execute
/// plugins or resolve a new binding. Removed or recreated owners reject it.
/// It belongs to the original platform window; another surface needs its own mount.
/// Multiple measurements are allowed; one display may place each mount once.
#[derive(Clone)]
pub struct Slots {
    keys: Rc<Vec<String>>,
    render: Rc<SlotRenderer>,
    pub(crate) input: Input,
}
type SlotRenderer = dyn Fn(&str, &mut Window, &mut App) -> maka_ui::Result<AnyElement>;
impl Slots {
    /// Constructed by the trusted surface adapter, with exact mount custody.
    pub fn new(
        keys: Vec<String>,
        input: Input,
        render: impl Fn(&str, &mut Window, &mut App) -> maka_ui::Result<AnyElement> + 'static,
    ) -> Self {
        Self {
            keys: Rc::new(keys),
            input,
            render: Rc::new(render),
        }
    }
    pub fn keys(&self) -> &[String] {
        &self.keys
    }
    pub fn render(
        &self,
        key: &str,
        window: &mut Window,
        cx: &mut App,
    ) -> maka_ui::Result<AnyElement> {
        (self.render)(key, window, cx)
    }
}

struct Label {
    text: String,
    appearance: maka_ui::appearance::TextAppearance,
}
impl Widget for Label {
    fn render(
        &self,
        _: &dyn Any,
        _: &Slots,
        _: &mut Window,
        _: &mut App,
    ) -> maka_ui::Result<AnyElement> {
        // Opaque native payload: GPUI styling is not constrained to the JSON
        // Node primitives. Product placement remains in the importing component.
        Ok(div()
            .font(appearance::font(&self.appearance.font))
            .text_color(appearance::color(self.appearance.foreground))
            .text_size(px(self.appearance.font.size))
            .line_height(gpui_kit::relative(self.appearance.font.line_height))
            .child(SharedString::from(self.text.clone()))
            .into_any_element())
    }
}
struct LabelComponent;
impl NativeComponent<dyn Widget> for LabelComponent {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _values: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let (props, revision) = match call {
            ComponentCall::Mount { props } => (props, 0),
            ComponentCall::Update { state, props } => (
                props,
                state["revision"].as_u64().expect("label revision") + 1,
            ),
            ComponentCall::Event { .. } => {
                return Err(maka_ui::Error::Invalid("label has no inputs".into()));
            }
        };
        let props: LabelProps = serde_json::from_value(props.clone())
            .map_err(|error| maka_ui::Error::Invalid(error.to_string()))?;
        Ok(Frame {
            revision,
            state: json!({"revision":revision}),
            surface: None,
            view: NativeView::<dyn Widget>::new(
                Rc::new(Label {
                    text: props.text,
                    appearance: props.appearance,
                }),
                BTreeMap::new(),
                vec![],
            )?,
        }
        .output(Vec::new()))
    }
}
pub fn registry() -> NativeRegistry<dyn Widget> {
    let mut registry =
        NativeRegistry::with_surface(maka_plugin_core::composition::PresentationTarget::Desktop)
            .with_render_depth_limit(gpui_kit::MAX_DEFERRED_DEPTH)
            .with_scene_compiler(scene::language(), scene::compile)
            .expect("valid desktop scene language");
    registry
        .register("maka.ui.label", |_, _values| {
            Ok(BTreeMap::from([(
                "Label".into(),
                NativeExport {
                    contract: label_contract(),
                    value: NativeCapability::Component(Rc::new(LabelComponent)),
                },
            )]))
        })
        .expect("unique native label implementation");
    textarea::register(&mut registry);
    markdown::register(&mut registry);
    image::register(&mut registry);
    reading::register(&mut registry);
    source::register(&mut registry);
    selection_menu::register(&mut registry);
    input::register(&mut registry);
    file::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{
        AppContext, Context, InteractiveElement, Modifiers, MouseButton, MouseDownEvent, Render,
        TestAppContext, point,
    };
    use std::cell::Cell;

    struct DecisionSurface {
        input: Input,
        parent: Rc<Cell<usize>>,
    }
    impl Render for DecisionSurface {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let parent = self.parent.clone();
            div()
                .size(px(200.))
                .on_mouse_down(MouseButton::Left, move |_, _, _| {
                    parent.set(parent.get() + 1);
                })
                .child(
                    div().size(px(100.)).on_mouse_down(
                        MouseButton::Left,
                        self.input
                            .bind("rule".into(), |_: &MouseDownEvent| Value::Null),
                    ),
                )
        }
    }

    #[gpui::test]
    fn committed_decisions_control_native_defaults_and_bubbling_and_failure_rejects(
        cx: &mut TestAppContext,
    ) {
        let mode = Rc::new(Cell::new(0_u8));
        let delivered = Rc::new(Cell::new(0_usize));
        let parent = Rc::new(Cell::new(0_usize));
        let decision = mode.clone();
        let seen = delivered.clone();
        let input = Input(Rc::new(move |_, _, _, _| {
            seen.set(seen.get() + 1);
            if decision.get() == 4 {
                return Err(maka_ui::Error::Invalid("uncommitted batch".into()));
            }
            Ok(maka_ui::InputDecision {
                prevent_default: decision.get() & 1 != 0,
                stop_propagation: decision.get() & 2 != 0,
            })
        }));
        let observed = parent.clone();
        let (_, _) = cx.add_window_view(move |_, _| DecisionSurface {
            input,
            parent: observed,
        });
        let handle = cx.windows()[0];
        cx.update_window(handle, |_, w, cx| {
            w.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
        for current in 0..=4 {
            mode.set(current);
            let before = parent.get();
            let result = cx
                .update_window(handle, |_, w, cx| {
                    w.dispatch_event(
                        gpui_kit::PlatformInput::MouseDown(MouseDownEvent {
                            button: MouseButton::Left,
                            position: point(px(10.), px(10.)),
                            modifiers: Modifiers::default(),
                            click_count: 1,
                            first_mouse: false,
                        }),
                        cx,
                    )
                })
                .unwrap();
            assert_eq!(delivered.get(), usize::from(current) + 1);
            assert_eq!(result.default_prevented, current & 1 != 0 || current == 4);
            assert_eq!(parent.get() - before, usize::from(current < 2));
        }
    }
}
