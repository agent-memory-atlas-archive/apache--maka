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
pub use maka_ui_runtime::budget::{BudgetLimits, NativeBudget};
mod editing;
mod file;
mod image;
pub mod input;
mod markdown;
mod reading;
pub mod scene;
mod selection_menu;
mod source;
mod terminal;
pub mod textarea;

use gpui_kit::{
    AnyElement, App, IntoElement, ParentElement, SharedString, Styled, Window, div, px,
};
use maka_ui_runtime::{
    ComponentCall, ComponentOutput, Frame, LabelProps, NativeCapability, NativeComponent,
    NativeExport, NativeRegistry, NativeView, label_contract,
};
use serde_json::{Value, json};
use std::{
    any::Any,
    cell::{OnceCell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

/// The presenter supplies its native view ABI; the plugin kernel never depends
/// on GPUI. Only native drawing happens here. Input bindings retain their mount
/// lease and invoke the component adapter later, during native event delivery.
/// A Widget is immutable recipe data: do not cache Entity/Input/subscription
/// handles here. Native state belongs to renderer owners and GPUI retained state;
/// its actual destruction releases leases independently of the retired frame.
pub trait Widget: Any {
    /// Allocate custody without taking focus. The presenter applies actual
    /// visibility, including initially hidden owners, before the first commit.
    /// Trusted bounds for reliable outcomes introduced by the candidate.
    fn publication_capacity(&self, _: &dyn Any, _: &App) -> Vec<(&'static str, usize)> {
        Vec::new()
    }
    fn create(&self, _: LiveInput, _: &mut Window, _: &mut App) -> Box<dyn Any> {
        Box::new(())
    }
    fn capture(
        &self,
        _: &mut dyn Any,
        _: &maka_ui_runtime::actions::CaptureTarget,
        _: &mut Window,
        _: &mut App,
    ) -> maka_ui_runtime::Result<std::sync::Arc<dyn maka_capabilities::call::ReadResource>> {
        Err(maka_ui_runtime::Error::Invalid(
            "widget capture unavailable".into(),
        ))
    }
    fn commit(&self, _: &mut dyn Any, _: &mut Window, _: &mut App) {}
    fn capture_continuation(
        &self,
        _: &dyn Any,
        _: &maka_ui_runtime::actions::CaptureTarget,
        _: &mut Window,
        _: &mut App,
    ) -> maka_ui_runtime::Result<maka_ui_runtime::actions::ContinuationWitness> {
        Err(maka_ui_runtime::Error::Invalid(
            "widget continuation unavailable".into(),
        ))
    }
    /// Read native evidence only. This runs before the feedback transaction;
    /// it must not dispatch input or publish observations from inside the check.
    fn validate_continuation(
        &self,
        _: &dyn Any,
        _: &maka_ui_runtime::actions::CaptureTarget,
        _: &maka_ui_runtime::actions::ContinuationWitness,
        _: &mut Window,
        _: &mut App,
    ) -> maka_ui_runtime::Result<bool> {
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
    ) -> maka_ui_runtime::Result<AnyElement>;
}
/// Owner-originated observations resolve the current dispatcher at delivery;
/// they are not button events against a retained old scene. Removal clears it
/// before dropping the owner, so delayed observations cannot retarget a new one.
#[derive(Clone)]
pub struct LiveInput {
    pub current: Rc<RefCell<Option<Input>>>,
    pub budget: NativeBudget,
    pub resources: std::sync::Arc<maka_capabilities::call::Resources>,
    pub checkpoint: Option<maka_ui_runtime::checkpoint::Scope>,
    pub observations: maka_ui_runtime::publication::Observations,
    // A mechanical weak callback, bound once by this mount's native producer.
    // It clears matching flags in the original logical ACK turn, before paint.
    publisher: Rc<OnceCell<Box<Published>>>,
}
type Published = dyn Fn(&str, &Value, &mut Window, &mut App);
impl LiveInput {
    pub fn with_checkpoint(mut self, scope: Option<maka_ui_runtime::checkpoint::Scope>) -> Self {
        self.checkpoint = scope;
        self
    }
    pub fn new(
        current: Rc<RefCell<Option<Input>>>,
        budget: NativeBudget,
        resources: std::sync::Arc<maka_capabilities::call::Resources>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        let observations = maka_ui_runtime::publication::Observations::new(budget.clone(), wake);
        Self {
            current,
            budget,
            resources,
            checkpoint: None,
            observations,
            publisher: Rc::new(OnceCell::new()),
        }
    }
    pub(crate) fn bind_publisher(
        &self,
        published: impl Fn(&str, &Value, &mut Window, &mut App) + 'static,
    ) {
        assert!(
            self.publisher.set(Box::new(published)).is_ok(),
            "original native producer already bound"
        );
    }
    pub fn advance(
        &self,
        input: &Input,
        window: &mut Window,
        cx: &mut App,
    ) -> Vec<maka_ui_runtime::Error> {
        self.observations.advance(
            input.revision,
            &mut (window, cx),
            |key, value, (window, cx)| (input.observe)(key, value, window, cx),
            |key, value, (window, cx)| {
                if self.current.borrow().is_some()
                    && let Some(published) = self.publisher.get()
                {
                    published(key, value, window, cx);
                }
            },
        )
    }
    /// Capture while holding the native owner, then dispatch after leaving its
    /// entity borrow. Only the original logical commit resolves the returned ACK.
    pub fn capture(
        &self,
        key: &str,
        value: Value,
        replaceable: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> maka_ui_runtime::Result<Option<maka_ui_runtime::backend::Job<()>>> {
        if self.current.borrow().is_none() {
            return Ok(None);
        }
        let acknowledgement = self.observations.capture(key, value, replaceable)?;
        if acknowledgement.is_some() {
            let live = self.clone();
            window.defer(cx, move |window, cx| {
                let Some(input) = live.current.borrow().clone() else {
                    return;
                };
                for error in live.advance(&input, window, cx) {
                    eprintln!("native observation candidate rejected: {error}");
                }
            });
        }
        Ok(acknowledgement)
    }
}
type Dispatch = dyn Fn(&str, Value, &mut Window, &mut App) -> maka_ui_runtime::Result<()>;
type Observe = dyn Fn(
    &str,
    Value,
    &mut Window,
    &mut App,
) -> maka_ui_runtime::Result<maka_ui_runtime::backend::Job<()>>;
#[derive(Clone)]
pub struct Input {
    pub dispatch: Rc<Dispatch>,
    pub observe: Rc<Observe>,
    pub revision: (u64, u64),
}
impl Input {
    pub fn new(revision: (u64, u64), dispatch: Rc<Dispatch>, observe: Rc<Observe>) -> Self {
        Self {
            revision,
            dispatch,
            observe,
        }
    }
    #[cfg(test)]
    pub(crate) fn testing(dispatch: Rc<Dispatch>) -> Self {
        let observation = dispatch.clone();
        Self::new(
            (0, 0),
            dispatch,
            Rc::new(move |key, value, window, cx| {
                Ok(maka_ui_runtime::backend::Job::ready(observation(
                    key, value, window, cx,
                )))
            }),
        )
    }
    pub fn bind<E: 'static>(
        &self,
        key: String,
        encode: impl Fn(&E) -> Value + 'static,
    ) -> impl Fn(&E, &mut Window, &mut App) + 'static {
        let dispatch = self.dispatch.clone();
        move |event, window, cx| {
            if let Err(error) = dispatch(&key, encode(event), window, cx) {
                eprintln!("component input rejected: {error}");
                window.prevent_default();
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
type SlotRenderer = dyn Fn(&str, &mut Window, &mut App) -> maka_ui_runtime::Result<AnyElement>;
impl Slots {
    /// Constructed by the trusted surface adapter, with exact mount custody.
    pub fn new(
        keys: Vec<String>,
        input: Input,
        render: impl Fn(&str, &mut Window, &mut App) -> maka_ui_runtime::Result<AnyElement> + 'static,
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
    ) -> maka_ui_runtime::Result<AnyElement> {
        (self.render)(key, window, cx)
    }
}

struct Label {
    props: LabelProps,
}
struct LabelOwner {
    props: LabelProps,
    visible: bool,
    tick: Option<gpui_kit::Task<()>>,
}
impl LabelOwner {
    fn sync(&mut self, window: &mut Window, cx: &mut gpui_kit::Context<Self>) {
        if !self.visible
            || !self
                .props
                .elapsed
                .as_ref()
                .is_some_and(|time| time.to.is_none())
        {
            self.tick = None;
        } else if self.tick.is_none() {
            // The task owns only weak entity/window handles. Hiding, retirement
            // and native destruction cancel this presentation-only timer.
            self.tick = Some(cx.spawn_in(window, async move |this, cx| {
                loop {
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(1))
                        .await;
                    if !this
                        .update_in(cx, |this, _, cx| {
                            if !this.visible {
                                return false;
                            }
                            cx.notify();
                            true
                        })
                        .unwrap_or(false)
                    {
                        break;
                    }
                }
            }));
        }
    }
}
impl gpui_kit::Render for LabelOwner {
    fn render(&mut self, _: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
        let appearance = &self.props.appearance;
        div()
            .font(appearance::font(&appearance.font))
            .text_color(appearance::color(appearance.foreground))
            .text_size(px(appearance.font.size))
            .line_height(gpui_kit::relative(appearance.font.line_height))
            .child(SharedString::from(self.props.rendered().into_owned()))
    }
}
impl Widget for Label {
    fn create(&self, _: LiveInput, _: &mut Window, cx: &mut App) -> Box<dyn Any> {
        use gpui_kit::AppContext as _;
        Box::new(cx.new(|_| LabelOwner {
            props: self.props.clone(),
            visible: false,
            tick: None,
        }))
    }
    fn commit(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<gpui_kit::Entity<LabelOwner>>()
            .unwrap()
            .update(cx, |owner, cx| {
                if owner.props != self.props {
                    owner.props = self.props.clone();
                    owner.sync(window, cx);
                    cx.notify();
                }
            });
    }
    fn presentation(&self, state: &mut dyn Any, visible: bool, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<gpui_kit::Entity<LabelOwner>>()
            .unwrap()
            .update(cx, |owner, cx| {
                if owner.visible != visible {
                    owner.visible = visible;
                    owner.sync(window, cx);
                    cx.notify();
                }
            });
    }
    fn deactivate(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        self.presentation(state, false, window, cx);
    }
    fn render(
        &self,
        state: &dyn Any,
        _: &Slots,
        _: &mut Window,
        _: &mut App,
    ) -> maka_ui_runtime::Result<AnyElement> {
        Ok(state
            .downcast_ref::<gpui_kit::Entity<LabelOwner>>()
            .unwrap()
            .clone()
            .into_any_element())
    }
}
struct LabelComponent;
impl NativeComponent<dyn Widget> for LabelComponent {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _values: &maka_ui_runtime::abi::ValueBindings,
    ) -> maka_ui_runtime::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let (props, revision) = match call {
            ComponentCall::Mount { props } => (props, 0),
            ComponentCall::Update { state, props } => (
                props,
                state["revision"].as_u64().expect("label revision") + 1,
            ),
            ComponentCall::Event { .. } => {
                return Err(maka_ui_runtime::Error::Invalid(
                    "label has no inputs".into(),
                ));
            }
        };
        let props: LabelProps = serde_json::from_value(props.clone())
            .map_err(|error| maka_ui_runtime::Error::Invalid(error.to_string()))?;
        props.validate()?;
        Ok(Frame {
            revision,
            state: json!({"revision":revision}),
            surface: None,
            view: NativeView::<dyn Widget>::new(Rc::new(Label { props }), BTreeMap::new(), vec![])?,
        }
        .output(Vec::new()))
    }
}
pub fn registry() -> NativeRegistry<dyn Widget> {
    let mut registry =
        NativeRegistry::with_surface(maka_plugin_core::composition::PresentationTarget::Desktop)
            .with_render_depth_limit(gpui_kit::MAX_DEFERRED_DEPTH)
            .with_scene_compiler(scene::language(), scene::prepare, scene::finish)
            .expect("valid desktop scene language");
    registry
        .register(maka_ui_runtime::LABEL_IMPLEMENTATION, |_, _values| {
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
    terminal::register(&mut registry);
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

    #[gpui::test]
    fn elapsed_label_ticks_only_while_visible_and_releases_its_native_owner(
        cx: &mut TestAppContext,
    ) {
        let props: LabelProps = serde_json::from_value(json!({
            "text":"· ","elapsed":{"from":1000,"to":null},
            "appearance":{"font":{"family":"Menlo","fallbacks":[],"size":14,"lineHeight":1,"weight":400},"foreground":4294967295u32}
        })).unwrap();
        let initial = props.clone();
        let (owner, _) = cx.add_window_view(move |_, _| LabelOwner {
            props: initial,
            visible: false,
            tick: None,
        });
        let handle = cx.windows()[0];
        let mut state: Box<dyn Any> = Box::new(owner.clone());
        let recipe = Label { props };
        let seen = Rc::new(Cell::new(0));
        let count = seen.clone();
        let _observation =
            cx.update(|cx| cx.observe(&owner, move |_, _| count.set(count.get() + 1)));
        cx.update_window(handle, |_, window, cx| {
            recipe.presentation(state.as_mut(), true, window, cx);
            recipe.commit(state.as_mut(), window, cx);
            assert!(owner.read(cx).tick.is_some());
        })
        .unwrap();
        cx.run_until_parked();
        let before = seen.get();
        cx.update_window(handle, |_, window, cx| {
            recipe.presentation(state.as_mut(), true, window, cx);
            recipe.commit(state.as_mut(), window, cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(
            seen.get(),
            before,
            "unchanged paint must not schedule another paint"
        );
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        assert_eq!(seen.get(), before + 1);
        cx.update_window(handle, |_, window, cx| {
            recipe.presentation(state.as_mut(), false, window, cx);
            assert!(owner.read(cx).tick.is_none());
        })
        .unwrap();
        cx.run_until_parked();
        let hidden = seen.get();
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(5));
        cx.run_until_parked();
        assert_eq!(seen.get(), hidden);
        let mut finished = recipe.props.clone();
        finished.elapsed.as_mut().unwrap().to = Some(69000);
        let finished = Label { props: finished };
        cx.update_window(handle, |_, window, cx| {
            finished.commit(state.as_mut(), window, cx);
            finished.presentation(state.as_mut(), true, window, cx);
            assert!(owner.read(cx).tick.is_none());
            assert_eq!(owner.read(cx).props.rendered(), "· 1m 08s");
            finished.deactivate(state.as_mut(), window, cx);
            recipe.commit(state.as_mut(), window, cx);
            recipe.presentation(state.as_mut(), true, window, cx);
            assert!(owner.read(cx).tick.is_some());
        })
        .unwrap();
        let weak = owner.downgrade();
        drop(state);
        drop(owner);
        cx.update_window(handle, |_, window, _| window.remove_window())
            .unwrap();
        cx.run_until_parked();
        cx.update(|_| {
            assert!(
                weak.upgrade().is_none(),
                "timer must not retain the native owner"
            )
        });
    }

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
    fn input_admission_failure_consumes_the_native_default_and_bubbling(cx: &mut TestAppContext) {
        let mode = Rc::new(Cell::new(0_u8));
        let delivered = Rc::new(Cell::new(0_usize));
        let parent = Rc::new(Cell::new(0_usize));
        let decision = mode.clone();
        let seen = delivered.clone();
        let input = Input::testing(Rc::new(move |_, _, _, _| {
            seen.set(seen.get() + 1);
            if decision.get() == 1 {
                return Err(maka_ui_runtime::Error::Invalid("uncommitted batch".into()));
            }
            Ok(())
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
        for current in 0..=1 {
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
            assert_eq!(result.default_prevented, current == 1);
            assert_eq!(parent.get() - before, usize::from(current == 0));
        }
    }
}
