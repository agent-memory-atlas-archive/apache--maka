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

pub mod editor;

use gpui_kit::{
    AnyElement, App, IntoElement, ParentElement, SharedString, Styled, Window, div, px, rgb,
};
use maka_ui::{
    ComponentCall, ComponentOutput, Frame, LabelProps, NativeCapability, NativeComponent,
    NativeExport, NativeRegistry, NativeView, Presentation, label_contract,
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
    fn render(
        &self,
        state: &dyn Any,
        slots: &Slots,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement;
}
/// Owner-originated observations resolve the current dispatcher at delivery;
/// they are not button events against a retained old scene. Removal clears it
/// before dropping the owner, so delayed observations cannot retarget a new one.
#[derive(Clone)]
pub struct LiveInput(pub Rc<RefCell<Option<Input>>>);
type Dispatch = dyn Fn(&str, Value, &mut Window, &mut App);
#[derive(Clone)]
pub struct Input(pub Rc<Dispatch>);
impl Input {
    pub fn bind<E: 'static>(
        &self,
        key: String,
        encode: impl Fn(&E) -> Value + 'static,
    ) -> impl Fn(&E, &mut Window, &mut App) + 'static {
        let dispatch = self.0.clone();
        move |event, window, cx| dispatch(&key, encode(event), window, cx)
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
}
type SlotRenderer = dyn Fn(&str, &mut Window, &mut App) -> maka_ui::Result<AnyElement>;
impl Slots {
    /// Constructed by the trusted surface adapter, with exact mount custody.
    pub fn new(
        keys: Vec<String>,
        render: impl Fn(&str, &mut Window, &mut App) -> maka_ui::Result<AnyElement> + 'static,
    ) -> Self {
        Self {
            keys: Rc::new(keys),
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
    emphasis: bool,
}
impl Widget for Label {
    fn render(&self, _: &dyn Any, _: &Slots, _: &mut Window, _: &mut App) -> AnyElement {
        // Opaque native payload: GPUI styling is not constrained to the JSON
        // Node primitives. Product placement remains in the importing component.
        let label = div()
            .px_3()
            .py_1()
            .rounded_full()
            .border_1()
            .border_color(rgb(0x5d82aa))
            .bg(rgb(0x172034))
            .text_color(rgb(0xe2efff))
            .text_size(px(18.))
            .child(SharedString::from(self.text.clone()));
        if self.emphasis {
            label
                .font_weight(gpui_kit::FontWeight::BOLD)
                .into_any_element()
        } else {
            label.into_any_element()
        }
    }
}
struct LabelComponent;
impl NativeComponent<dyn Widget> for LabelComponent {
    fn call(
        &self,
        call: ComponentCall<'_>,
    ) -> maka_ui::Result<ComponentOutput<Presentation<dyn Widget>>> {
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
            view: Presentation::Native(NativeView::<dyn Widget>::new(
                Rc::new(Label {
                    text: props.text,
                    emphasis: props.emphasis,
                }),
                BTreeMap::new(),
                vec![],
            )?),
        }
        .output(Vec::new()))
    }
}
pub fn registry() -> NativeRegistry<dyn Widget> {
    let mut registry =
        NativeRegistry::with_surface(maka_plugin_core::composition::PresentationTarget::Desktop);
    registry
        .register("maka.ui.label", |_| {
            Ok(BTreeMap::from([(
                "Label".into(),
                NativeExport {
                    contract: label_contract(),
                    value: NativeCapability::Component(Rc::new(LabelComponent)),
                },
            )]))
        })
        .expect("unique native label implementation");
    editor::register(&mut registry);
    registry
}
