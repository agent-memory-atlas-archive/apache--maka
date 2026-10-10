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

//! Original welcome projection, with product-supplied geometry and appearance.
use super::{NativeContext, Placement, Slots, TerminalLayout, Widget};
use maka_ui::{
    ComponentCall, ComponentOutput, Frame, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView, morph_mark::*,
};
use ratatui::{buffer::Buffer, layout::Rect};
use serde_json::json;
use std::{any::Any, cell::RefCell, collections::BTreeMap, rc::Rc};
mod geometry;
mod render;
struct Data {
    formations: Vec<geometry::Drone>,
    color: maka_ui::scene::terminal::Color,
    period: u32,
}
struct Mark {
    data: Rc<Data>,
    props: MorphMarkProps,
}
impl Widget for Mark {
    fn create(&self, _: NativeContext) -> Box<dyn Any> {
        Box::new(RefCell::new(render::Renderer::default()))
    }
    fn same_presentation(&self, other: &dyn Any) -> bool {
        other
            .downcast_ref::<Self>()
            .is_some_and(|other| Rc::ptr_eq(&self.data, &other.data) && self.props == other.props)
    }
    fn layout(&self, _: &Slots<'_>) -> maka_ui::Result<TerminalLayout> {
        Ok(TerminalLayout {
            grow: 1,
            ..Default::default()
        })
    }
    fn measure(&self, _: &Slots<'_>, width: u16) -> maka_ui::Result<(u16, u32)> {
        Ok((width, 0))
    }
    fn paint(
        &self,
        slots: &mut Slots<'_>,
        area: Placement,
        buffer: &mut Buffer,
    ) -> maka_ui::Result<()> {
        let visible = area.visible();
        let rows = visible
            .height
            .saturating_sub(4)
            .min(22)
            .min(visible.width.saturating_sub(4) / 3);
        if rows < 12 {
            return Ok(());
        }
        let stage = Rect::new(
            visible.x + (visible.width - rows * 3) / 2,
            visible.y + (visible.height - rows) / 2,
            rows * 3,
            rows,
        );
        let phase = self
            .props
            .motion
            .then(|| slots.animation_phase(self.data.period))
            .flatten();
        slots
            .native_state::<RefCell<render::Renderer>>()?
            .borrow_mut()
            .draw(
                stage,
                buffer,
                [
                    super::paint::color(self.props.background),
                    super::paint::color(self.props.foreground),
                    super::paint::color(self.data.color),
                ],
                &self.data.formations,
                self.props.ascii,
                phase,
            );
        Ok(())
    }
}
struct Component {
    data: Rc<Data>,
}
impl NativeComponent<dyn Widget> for Component {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let (props, revision) = match call {
            ComponentCall::Mount { props } => (props, 0),
            ComponentCall::Update { state, props } => {
                (props, state["revision"].as_u64().unwrap() + 1)
            }
            ComponentCall::Event { .. } => {
                return Err(maka_ui::Error::Invalid(
                    "morphing mark has no inputs".into(),
                ));
            }
        };
        let props: MorphMarkProps = serde_json::from_value(props.clone())
            .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
        Ok(Frame {
            revision,
            state: json!({"revision": revision}),
            surface: None,
            view: NativeView::<dyn Widget>::new(
                Rc::new(Mark {
                    data: self.data.clone(),
                    props,
                }),
                BTreeMap::new(),
                vec![],
            )?,
        }
        .output(vec![]))
    }
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register("maka.ui.morph-mark", |configuration, _| {
            let configuration: MorphMarkConfiguration =
                serde_json::from_value(configuration.clone())
                    .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
            configuration.validate()?;
            let data = Rc::new(Data {
                formations: geometry::formations(&configuration.shapes),
                color: configuration.color,
                period: configuration.period,
            });
            Ok(BTreeMap::from([(
                "MorphMark".into(),
                NativeExport {
                    contract: contract(),
                    value: NativeCapability::Component(Rc::new(Component { data })),
                },
            )]))
        })
        .expect("unique morphing mark factory");
}
