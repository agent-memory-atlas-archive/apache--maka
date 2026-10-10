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

use maka_ui::NativeView;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::{rc::Rc, time::Instant};

/// The nominal ABI supplied by this presenter's pure scene compiler.
pub fn language() -> maka_plugin_core::package::SceneLanguage {
    maka_plugin_core::package::SceneLanguage {
        name: "maka.tui.scene".into(),
        version: 1,
    }
}

pub use maka_ui::scene::terminal::{Direction, Layout, Node, ScrollPolicy};

pub(super) fn compile(
    source: &Value,
    deadline: Instant,
) -> maka_ui::Result<NativeView<dyn super::Widget>> {
    compile_with_inputs(source, deadline, BTreeMap::new())
}
pub(super) fn compile_with_inputs(
    source: &Value,
    deadline: Instant,
    mut inputs: BTreeMap<String, Value>,
) -> maka_ui::Result<NativeView<dyn super::Widget>> {
    if Instant::now() >= deadline {
        return Err(maka_ui::Error::Invalid("terminal scene deadline".into()));
    }
    fn scan(
        source: &Value,
        depth: usize,
        count: &mut usize,
        deadline: Instant,
    ) -> maka_ui::Result<()> {
        *count += 1;
        if Instant::now() >= deadline || depth > 32 || *count > 4096 {
            return Err(maka_ui::Error::Invalid(
                "scene source depth/node/deadline limit".into(),
            ));
        }
        for field in ["background", "content"] {
            if let Some(child) = source.get(field) {
                scan(child, depth + 1, count, deadline)?;
            }
        }
        if let Some(children) = source.get("children").and_then(Value::as_array) {
            for child in children {
                scan(child, depth + 1, count, deadline)?;
            }
        }
        Ok(())
    }
    scan(source, 0, &mut 0, deadline)?;
    let node: Node = serde_json::from_value(source.clone())
        .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
    node.validate_root(deadline)
        .map_err(maka_ui::Error::Invalid)?;
    let mut slots = vec![];
    let mut hidden_slots = BTreeSet::new();
    node.metadata(
        &mut slots,
        &mut inputs,
        &mut hidden_slots,
        &mut 0,
        true,
        deadline,
    )
    .map_err(maka_ui::Error::Invalid)?;
    node.validate_focus_targets(&slots)
        .map_err(maka_ui::Error::Invalid)?;
    node.shortcut_inputs(&mut inputs, true)
        .map_err(maka_ui::Error::Invalid)?;
    let visible = node.visible();
    NativeView::new(Rc::new(node) as Rc<dyn super::Widget>, inputs, slots)?
        .with_hidden_slots(hidden_slots)
        .map(|view| view.with_root_visibility(visible))
}
impl super::Widget for Node {
    fn scene(&self) -> Option<&Node> {
        Some(self)
    }
    fn prepare_focus(&self, state: &mut dyn std::any::Any, visible: bool) {
        let state = state.downcast_mut::<super::scroll::SceneState>().unwrap();
        state.prepare_dialogs(self, visible);
        state.prepare_focus_scopes(self, visible);
    }
    fn deactivate(&self, state: &mut dyn std::any::Any) {
        let state = state.downcast_mut::<super::scroll::SceneState>().unwrap();
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
    ) {
        if let Some(Node::Choice(choice)) = self.overlay(key) {
            state
                .downcast_mut::<super::scroll::SceneState>()
                .unwrap()
                .choice_hover(choice, point);
        }
    }
    fn same_presentation(&self, previous: &dyn std::any::Any) -> bool {
        previous
            .downcast_ref::<Self>()
            .is_some_and(|previous| self == previous)
    }
    fn layout(&self, slots: &super::Slots<'_>) -> maka_ui::Result<Layout> {
        slots
            .presenter
            .node_layout(slots.parent, self, slots.measurements)
    }
    fn create(&self, _: super::NativeContext) -> Box<dyn std::any::Any> {
        Box::<super::scroll::SceneState>::default()
    }
    fn commit(&self, state: &mut dyn std::any::Any) -> maka_ui::Result<Option<super::NativeInput>> {
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
    ) -> maka_ui::Result<super::WidgetInput> {
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

    fn measure(&self, slots: &super::Slots<'_>, width: u16) -> maka_ui::Result<(u16, u32)> {
        slots
            .presenter
            .measure(slots.parent, self, width, slots.measurements)
    }
    fn paint(
        &self,
        slots: &mut super::Slots<'_>,
        area: super::Placement,
        buffer: &mut ratatui::buffer::Buffer,
    ) -> maka_ui::Result<()> {
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
            slots
                .scene
                .as_deref_mut()
                .ok_or_else(|| maka_ui::Error::Invalid("cannot paint while measuring".into()))?,
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
