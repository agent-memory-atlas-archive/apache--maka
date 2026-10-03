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

use super::{Slots, Widget, paint};
use maka_ui::{
    ComponentCall, ComponentOutput, Frame, LabelProps, NativeCapability, NativeComponent,
    NativeExport, NativeRegistry, NativeView, label_contract,
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};
use serde_json::json;
use std::{collections::BTreeMap, rc::Rc};
struct Label {
    props: LabelProps,
}
impl Widget for Label {
    fn measure(&self, _: &Slots<'_>, width: u16) -> maka_ui::Result<(u16, u16)> {
        Ok(paint::text_size(&self.props.text, width))
    }
    fn paint(&self, _: &mut Slots<'_>, area: Rect, buffer: &mut Buffer) -> maka_ui::Result<()> {
        let rgba = self.props.appearance.foreground.0;
        let mut style = Style::default().fg(Color::Rgb(
            (rgba >> 24) as u8,
            (rgba >> 16) as u8,
            (rgba >> 8) as u8,
        ));
        if self.props.appearance.font.weight >= 600 {
            style = style.add_modifier(Modifier::BOLD);
        }
        paint::text(&self.props.text, area, buffer, style);
        Ok(())
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
                state["revision"].as_u64().expect("native label revision") + 1,
            ),
            ComponentCall::Event { .. } => {
                return Err(maka_ui::Error::Invalid("label has no inputs".into()));
            }
        };
        let props = serde_json::from_value(props.clone())
            .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
        Ok(Frame {
            revision,
            state: json!({"revision":revision}),
            surface: None,
            view: NativeView::<dyn Widget>::new(Rc::new(Label { props }), BTreeMap::new(), vec![])?,
        }
        .output(Vec::new()))
    }
}
/// Available compiled implementations; a composition still chooses each instance.
/// 编译时提供实现，实例及绑定仍由产品 composition 决定。
pub fn registry() -> NativeRegistry<dyn Widget> {
    let mut registry =
        NativeRegistry::with_surface(maka_plugin_core::composition::PresentationTarget::Tui)
            .with_scene_compiler(super::scene::language(), super::scene::compile)
            .expect("valid terminal scene language");
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
        .expect("unique label factory");
    registry
}
