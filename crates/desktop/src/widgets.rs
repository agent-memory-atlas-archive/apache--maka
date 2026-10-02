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

use gpui_kit::{
    AnyElement, App, IntoElement, ParentElement, SharedString, Styled, Window, div, px, rgb,
};
use maka_ui::{
    ComponentCall, Frame, LabelProps, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView, Presentation, label_contract,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, rc::Rc};

/// The presenter supplies its native view ABI; the plugin kernel never depends
/// on GPUI. Only native drawing happens here. Input bindings retain their mount
/// lease and invoke the component adapter later, during native event delivery.
/// A Widget is immutable recipe data: do not cache Entity/Input/subscription
/// handles here. Native state belongs to the GPUI retained element state/arena;
/// its actual destruction releases leases independently of the retired frame.
pub(super) trait Widget {
    fn render(&self, input: Input, window: &mut Window, cx: &mut App) -> AnyElement;
}
type Dispatch = dyn Fn(&str, Value, &mut App);
#[derive(Clone)]
pub(super) struct Input(pub Rc<Dispatch>);
impl Input {
    pub fn bind<E: 'static>(
        &self,
        key: String,
        encode: impl Fn(&E) -> Value + 'static,
    ) -> impl Fn(&E, &mut Window, &mut App) + 'static {
        let dispatch = self.0.clone();
        move |event, _, cx| dispatch(&key, encode(event), cx)
    }
}

struct Label {
    text: String,
    emphasis: bool,
}
impl Widget for Label {
    fn render(&self, _: Input, _: &mut Window, _: &mut App) -> AnyElement {
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
    fn call(&self, call: ComponentCall<'_>) -> maka_ui::Result<Frame<Presentation<dyn Widget>>> {
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
            window: None,
            view: Presentation::Native(NativeView::<dyn Widget>::new(
                Rc::new(Label {
                    text: props.text,
                    emphasis: props.emphasis,
                }),
                BTreeMap::new(),
            )?),
        })
    }
}
pub(super) fn registry() -> NativeRegistry<dyn Widget> {
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
    registry
}
