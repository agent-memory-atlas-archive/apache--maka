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

use super::{NativeContext, NativeInput, Placement, Slots, Widget};
use maka_ui::text_file::{TextFile, TextFileObservation, TextFileProps, contract};
use maka_ui::{
    ComponentCall, ComponentOutput, Frame, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView,
};
use ratatui::buffer::Buffer;
use std::{any::Any, collections::BTreeMap, rc::Rc};
struct Recipe(TextFileProps, Option<std::path::PathBuf>);
fn observation(owner: &mut TextFile) -> Option<NativeInput> {
    owner.observe().map(|value| NativeInput {
        key: "changed".into(),
        value: serde_json::to_value(value).unwrap(),
    })
}
impl Widget for Recipe {
    fn create(&self, context: NativeContext) -> Box<dyn Any> {
        Box::new(TextFile::new(
            self.1.clone(),
            context.resources,
            context.budget,
        ))
    }
    fn commit(&self, state: &mut dyn Any) -> maka_ui::Result<Option<NativeInput>> {
        let owner = state.downcast_mut::<TextFile>().unwrap();
        owner.update(&self.0);
        Ok(observation(owner))
    }
    fn observe(&self, state: &mut dyn Any) -> Option<NativeInput> {
        observation(state.downcast_mut::<TextFile>().unwrap())
    }
    fn deactivate(&self, state: &mut dyn Any) {
        state.downcast_mut::<TextFile>().unwrap().close();
    }
    fn measure(&self, _: &Slots<'_>, _: u16) -> maka_ui::Result<(u16, u32)> {
        Ok((0, 0))
    }
    fn paint(&self, _: &mut Slots<'_>, _: Placement, _: &mut Buffer) -> maka_ui::Result<()> {
        Ok(())
    }
}
struct Component(Option<std::path::PathBuf>);
impl NativeComponent<dyn Widget> for Component {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let mut events = vec![];
        let (props, revision) = match call {
            ComponentCall::Mount { props } => (props, 0),
            ComponentCall::Update { props, state } => {
                (props, state["revision"].as_u64().unwrap() + 1)
            }
            ComponentCall::Event {
                state,
                props,
                key: "changed",
                input,
                ..
            } => {
                events.push(maka_ui::Emission {
                    name: "changed".into(),
                    value: input.clone(),
                });
                (props, state["revision"].as_u64().unwrap() + 1)
            }
            ComponentCall::Event { .. } => {
                return Err(maka_ui::Error::Invalid(
                    "text-file has no raw inputs".into(),
                ));
            }
        };
        let parsed: TextFileProps = serde_json::from_value(props.clone())
            .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
        let view = NativeView::<dyn Widget>::new(
            Rc::new(Recipe(parsed, self.0.clone())),
            BTreeMap::from([(
                "changed".into(),
                serde_json::to_value(schemars::schema_for!(TextFileObservation)).unwrap(),
            )]),
            vec![],
        )?;
        Ok(Frame {
            state: serde_json::json!({"revision":revision}),
            revision,
            surface: None,
            view,
        }
        .output(events))
    }
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register("maka.ui.text-file", |configuration, _| {
            let configuration: maka_ui::text_file::TextFileConfiguration =
                serde_json::from_value(configuration.clone())
                    .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
            if configuration
                .path
                .as_ref()
                .is_some_and(|path| !path.is_absolute() || path.file_name().is_none())
            {
                return Err(maka_ui::Error::Invalid(
                    "text file requires an absolute file path".into(),
                ));
            }
            Ok(BTreeMap::from([(
                "TextFile".into(),
                NativeExport {
                    contract: contract(),
                    value: NativeCapability::Component(Rc::new(Component(configuration.path))),
                },
            )]))
        })
        .expect("unique terminal text-file");
}
