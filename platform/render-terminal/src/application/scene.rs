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

use maka_ui_runtime::{NativeView, PreparedScene};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::rc::Rc;

/// The nominal ABI supplied by this presenter's pure scene compiler.
pub fn language() -> maka_plugin_core::package::SceneLanguage {
    maka_plugin_core::package::SceneLanguage {
        name: "maka.tui.scene".into(),
        version: 1,
    }
}

pub use maka_ui_runtime::scene::terminal::{Direction, Layout, Node, ScrollPolicy};

#[cfg(test)]
pub(super) fn compile(source: &Value) -> maka_ui_runtime::Result<NativeView<dyn super::Widget>> {
    compile_with_inputs(source, BTreeMap::new())
}
pub(super) fn compile_with_inputs(
    source: &Value,
    inputs: BTreeMap<String, Value>,
) -> maka_ui_runtime::Result<NativeView<dyn super::Widget>> {
    finish(prepare_with_inputs(source, inputs)?)
}

pub fn prepare(source: &Value) -> maka_ui_runtime::Result<PreparedScene> {
    prepare_with_inputs(source, BTreeMap::new())
}
pub fn finish(prepared: PreparedScene) -> maka_ui_runtime::Result<NativeView<dyn super::Widget>> {
    prepared.finish::<Node, dyn super::Widget>(|node| Rc::new(node))
}
fn prepare_with_inputs(
    source: &Value,
    inputs: BTreeMap<String, Value>,
) -> maka_ui_runtime::Result<PreparedScene> {
    fn scan(source: &Value, depth: usize) -> maka_ui_runtime::Result<()> {
        if depth > 32 {
            return Err(maka_ui_runtime::Error::Invalid(
                "scene source depth limit".into(),
            ));
        }
        for field in ["background", "content"] {
            if let Some(child) = source.get(field) {
                scan(child, depth + 1)?;
            }
        }
        if let Some(children) = source.get("children").and_then(Value::as_array) {
            for child in children {
                scan(child, depth + 1)?;
            }
        }
        Ok(())
    }
    scan(source, 0)?;
    let node: Node =
        Node::deserialize(source).map_err(|e| maka_ui_runtime::Error::Invalid(e.to_string()))?;
    node.validate_root()
        .map_err(maka_ui_runtime::Error::Invalid)?;
    let maka_ui_runtime::scene::terminal::Metadata {
        slots,
        inputs,
        hidden_slots,
    } = node
        .metadata(inputs)
        .map_err(maka_ui_runtime::Error::Invalid)?;
    let visible = node.visible();
    PreparedScene::new(node, inputs, slots)?
        .with_hidden_slots(hidden_slots)
        .map(|view| view.with_root_visibility(visible))
}
impl super::Widget for Node {
    fn cancel_pointer(&self, state: &mut dyn std::any::Any) {
        let state = state.downcast_mut::<super::scroll::SceneState>().unwrap();
        state.scrollbar_drag = None;
        for pane in state.panes.values_mut() {
            pane.resizing = false;
        }
    }
    fn pointer_capture(&self, state: &dyn std::any::Any) -> bool {
        let state = state.downcast_ref::<super::scroll::SceneState>().unwrap();
        state.scrollbar_drag.is_some() || state.panes.values().any(|pane| pane.resizing)
    }
    fn scene(&self) -> Option<&Node> {
        Some(self)
    }
    fn prepare_focus(&self, state: &mut dyn std::any::Any, visible: bool, revision: u64) {
        let state = state.downcast_mut::<super::scroll::SceneState>().unwrap();
        state.prepare_focus(self, visible, revision);
    }
    fn deactivate(&self, state: &mut dyn std::any::Any) {
        let state = state.downcast_mut::<super::scroll::SceneState>().unwrap();
        state.scrollbar_drag = None;
        for pane in state.panes.values_mut() {
            pane.resizing = false;
        }
        state.dialogs.clear();
        state.focus.clear();
    }
    fn handles_tab(&self, state: &dyn std::any::Any, key: &str) -> bool {
        matches!(self.overlay(key), Some(Node::Choice(_)))
            && state
                .downcast_ref::<super::scroll::SceneState>()
                .unwrap()
                .dialogs
                .contains_key(key)
    }
    fn hover(
        &self,
        state: &mut dyn std::any::Any,
        key: &str,
        point: Option<ratatui::layout::Position>,
    ) -> bool {
        if let Some(Node::Choice(choice)) = self.overlay(key) {
            return state
                .downcast_mut::<super::scroll::SceneState>()
                .unwrap()
                .choice_hover(choice, point);
        }
        false
    }
    fn same_presentation(&self, previous: &dyn std::any::Any) -> bool {
        previous
            .downcast_ref::<Self>()
            .is_some_and(|previous| self == previous)
    }
    fn layout(&self, slots: &super::Slots<'_>) -> maka_ui_runtime::Result<Layout> {
        slots
            .presenter
            .node_layout(slots.parent, self, slots.measurements)
    }
    fn create(&self, _: super::NativeContext) -> Box<dyn std::any::Any> {
        Box::<super::scroll::SceneState>::default()
    }
    fn commit(
        &self,
        state: &mut dyn std::any::Any,
    ) -> maka_ui_runtime::Result<Option<super::NativeInput>> {
        state
            .downcast_mut::<super::scroll::SceneState>()
            .unwrap()
            .commit(self);
        Ok(None)
    }
    fn input(
        &self,
        state: &mut dyn std::any::Any,
        key: &str,
        event: &crossterm::event::Event,
    ) -> maka_ui_runtime::Result<super::WidgetInput> {
        if let Some(node @ Node::AdaptivePane { .. }) = self.overlay(key) {
            return Ok(state
                .downcast_mut::<super::scroll::SceneState>()
                .unwrap()
                .pane_input(node, event));
        }
        if let Some(Node::Choice(choice)) = self.overlay(key) {
            return Ok(state
                .downcast_mut::<super::scroll::SceneState>()
                .unwrap()
                .choice_input(choice, event));
        }
        if let crossterm::event::Event::Mouse(mouse) = event
            && matches!(
                mouse.kind,
                crossterm::event::MouseEventKind::ScrollUp
                    | crossterm::event::MouseEventKind::ScrollDown
            )
            && let Some(viewport) = state
                .downcast_ref::<super::scroll::SceneState>()
                .unwrap()
                .scrolls
                .get(key)
                .and_then(|scroll| scroll.presented.as_ref())
        {
            let current = self.wheel_commands(key);
            if viewport.wheel.as_ref() != current {
                return Ok(super::WidgetInput::Consumed(None));
            }
            if let Some(wheel) = current {
                if !viewport
                    .area
                    .visible()
                    .contains((mouse.column, mouse.row).into())
                {
                    return Ok(super::WidgetInput::Unhandled);
                }
                return Ok(super::WidgetInput::Emitted(super::NativeInput {
                    replaceable: false,
                    key: if mouse.kind == crossterm::event::MouseEventKind::ScrollUp {
                        wheel.up.clone()
                    } else {
                        wheel.down.clone()
                    },
                    value: serde_json::json!({"kind":"wheel"}),
                }));
            }
        }
        Ok(
            if state
                .downcast_mut::<super::scroll::SceneState>()
                .unwrap()
                .input(key, event)
            {
                super::WidgetInput::Consumed(None)
            } else {
                super::WidgetInput::Unhandled
            },
        )
    }
    fn present(
        &self,
        state: &mut dyn std::any::Any,
        geometry: Box<dyn std::any::Any>,
    ) -> Option<super::NativeInput> {
        state
            .downcast_mut::<super::scroll::SceneState>()
            .unwrap()
            .present(*geometry.downcast::<super::scroll::Geometry>().unwrap());
        None
    }

    fn measure(&self, slots: &super::Slots<'_>, width: u16) -> maka_ui_runtime::Result<(u16, u32)> {
        slots
            .presenter
            .measure(slots.parent, self, width, slots.measurements)
    }
    fn paint(
        &self,
        slots: &mut super::Slots<'_>,
        area: super::Placement,
        buffer: &mut ratatui::buffer::Buffer,
    ) -> maka_ui_runtime::Result<()> {
        if slots.parent == &slots.presenter.root
            && let Self::Stack {
                key,
                shortcuts,
                focus_shortcuts,
                ..
            } = self
            && !shortcuts.is_empty()
        {
            slots.scene.as_deref_mut().expect("paint scene").keyboard =
                Some(Rc::new(super::paint::Ancestor {
                    identity: super::paint::TargetIdentity {
                        mount: slots.parent.clone(),
                        key: key.clone(),
                        owner: slots.presenter.resources.owners[&slots.parent.id()]
                            .identity
                            .clone(),
                        scope: slots.scene.as_ref().unwrap().input_scope.clone(),
                    },
                    revision: slots.presenter.application.frame(slots.parent)?.revision,
                    shortcuts: shortcuts.clone(),
                    focus_group: None,
                    scroll: None,
                    focus_shortcuts: slots
                        .presenter
                        .freeze_focus_shortcuts(slots.parent, focus_shortcuts)?,
                }));
        }
        let generation = slots
            .native_state::<super::scroll::SceneState>()?
            .generation;
        let mut viewports = Vec::new();
        let mut choices = Vec::new();
        let mut panes = Vec::new();
        slots.presenter.paint_node(
            slots.parent,
            self,
            area,
            buffer,
            slots.scene.as_deref_mut().ok_or_else(|| {
                maka_ui_runtime::Error::Invalid("cannot paint while measuring".into())
            })?,
            &mut super::paint::NodePaint {
                measurements: slots.measurements,
                viewports: &mut viewports,
                choices: &mut choices,
                panes: &mut panes,
            },
        )?;
        slots.native_geometry(Box::new(super::scroll::Geometry {
            generation,
            viewports,
            choices,
            panes,
        }))
    }
}
