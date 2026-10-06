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

use super::scene::Node;
use super::{CellRect, Placement, Presenter};
use maka_plugin_core::runtime::Mount;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Remove terminal controls before measuring, painting or emitting diagnostics.
/// 换行保留为布局数据；其他终端控制字符不会进入输出流。
pub fn sanitize(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\t' => output.push_str("    "),
            '\n' => output.push(c),
            c if !c.is_control() => output.push(c),
            _ => {}
        }
    }
    output
}
fn lines(text: &str, width: u16) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let text = sanitize(text);
    let mut result = Vec::new();
    for source in text.split('\n') {
        let mut line = String::new();
        let mut cells = 0;
        for grapheme in source.graphemes(true) {
            let size = grapheme.width();
            if size > usize::from(width) {
                continue;
            }
            if cells + size > usize::from(width) {
                result.push(std::mem::take(&mut line));
                cells = 0;
            }
            line.push_str(grapheme);
            cells += size;
        }
        result.push(line);
    }
    result
}
pub(super) fn text_size(text: &str, width: u16) -> (u16, u32) {
    let lines = lines(text, width);
    (
        lines
            .iter()
            .map(|s| s.width().min(usize::from(width)) as u16)
            .max()
            .unwrap_or(0),
        lines.len().min(u32::MAX as usize) as u32,
    )
}
pub(super) fn text(text: &str, area: Placement, buffer: &mut Buffer, style: Style) {
    let area = area.clipped(buffer.area);
    let visible = area.visible();
    let first = (i64::from(visible.y) - area.y).max(0) as usize;
    for (row, line) in lines(text, area.width)
        .into_iter()
        .take(area.height as usize)
        .enumerate()
        .skip(first)
        .take(usize::from(visible.height))
    {
        let mut column = 0;
        for grapheme in line.graphemes(true) {
            let width = grapheme.width();
            if let Some(point) = area.point(column, row)
                && (width == 0 || area.point(column + width - 1, row).is_some())
            {
                buffer.set_stringn(point.x, point.y, grapheme, width.max(1), style);
            }
            column += width;
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TargetKind {
    Action,
    Native,
    Scroll,
}
#[derive(Clone)]
pub(super) struct TargetIdentity {
    pub mount: Mount,
    pub key: String,
    pub owner: std::rc::Rc<()>,
}
#[derive(Clone)]
pub(super) struct Target {
    pub identity: TargetIdentity,
    pub revision: u64,
    pub area: Placement,
    pub tab_index: i16,
    pub kind: TargetKind,
    pub ancestors: Vec<TargetIdentity>,
}
impl std::ops::Deref for Target {
    type Target = TargetIdentity;
    fn deref(&self) -> &TargetIdentity {
        &self.identity
    }
}
impl Target {
    pub fn same(&self, other: &Self) -> bool {
        self.mount == other.mount
            && self.key == other.key
            && std::rc::Rc::ptr_eq(&self.owner, &other.owner)
    }
}
/// A pending scene is not input geometry until the embedding confirms a flush.
/// 绘制成功并完成实际 flush 后，才发布对应的命中区域。
pub struct PaintedScene {
    pub(super) targets: Vec<Target>,
    placed: std::collections::BTreeSet<uuid::Uuid>,
    laid_out: std::collections::BTreeSet<uuid::Uuid>,
    pub(super) scope: Option<super::selection::ScopePaint>,
    geometries: Vec<NativeGeometry>,
    ancestry: Vec<TargetIdentity>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MeasurementKey {
    Mount,
    Node(usize),
}
type MeasurementCache = std::collections::BTreeMap<(uuid::Uuid, MeasurementKey, u16), (u16, u32)>;
/// Derived, paint-local memoization. No entry survives the immutable render pass
/// or participates in input/continuation/geometry admission.
#[derive(Default)]
pub(super) struct Measurements {
    cache: std::cell::RefCell<MeasurementCache>,
    #[cfg(test)]
    requests: std::cell::Cell<usize>,
    #[cfg(test)]
    limit: std::cell::Cell<usize>,
}
impl Measurements {
    fn get_or(
        &self,
        mount: uuid::Uuid,
        key: MeasurementKey,
        width: u16,
        measure: impl FnOnce() -> maka_ui::Result<(u16, u32)>,
    ) -> maka_ui::Result<(u16, u32)> {
        #[cfg(test)]
        {
            let requests = self.requests.get() + 1;
            self.requests.set(requests);
            assert!(
                self.limit.get() == 0 || requests <= self.limit.get(),
                "measurement work exceeded fixture bound"
            );
        }
        let key = (mount, key, width);
        if let Some(size) = self.cache.borrow().get(&key).copied() {
            return Ok(size);
        }
        let size = measure()?;
        self.cache.borrow_mut().insert(key, size);
        Ok(size)
    }
    #[cfg(test)]
    pub(super) fn guarded(limit: usize) -> Self {
        let value = Self::default();
        value.limit.set(limit);
        value
    }
    #[cfg(test)]
    pub(super) fn requests(&self) -> usize {
        self.requests.get()
    }
}

/// Derived data for one immutable scene recipe's paint pass.
pub(super) struct NodePaint<'a> {
    pub measurements: &'a Measurements,
    pub viewports: &'a mut Vec<(String, super::scroll::Viewport)>,
}

/// Synchronous terminal geometry over one parent's committed component slots.
/// Measuring is read-only; painting publishes hit geometry only after flush.
pub struct Slots<'a> {
    pub(super) presenter: &'a Presenter,
    pub(super) parent: &'a Mount,
    declarations: &'a [maka_ui::ComponentSlot],
    pub(super) scene: Option<&'a mut PaintedScene>,
    pub(super) measurements: &'a Measurements,
    placement: Option<Placement>,
}
struct NativeGeometry {
    mount: Mount,
    owner: std::rc::Rc<()>,
    geometry: Box<dyn std::any::Any>,
}
impl Slots<'_> {
    pub fn native_state<T: std::any::Any>(&self) -> maka_ui::Result<&T> {
        self.presenter
            .resources
            .owners
            .get(&self.parent.id())
            .and_then(|owner| owner.state.downcast_ref())
            .ok_or_else(|| maka_ui::Error::Invalid("terminal native state unavailable".into()))
    }
    pub fn native_geometry(&mut self, geometry: Box<dyn std::any::Any>) -> maka_ui::Result<()> {
        let owner = self
            .presenter
            .resources
            .owners
            .get(&self.parent.id())
            .ok_or_else(|| maka_ui::Error::Invalid("terminal native owner unavailable".into()))?;
        self.scene
            .as_deref_mut()
            .ok_or_else(|| {
                maka_ui::Error::Invalid("cannot publish geometry while measuring".into())
            })?
            .geometries
            .push(NativeGeometry {
                mount: self.parent.clone(),
                owner: owner.identity.clone(),
                geometry,
            });
        Ok(())
    }
    pub fn native_target(
        &mut self,
        key: &str,
        area: Placement,
        tab_index: i16,
    ) -> maka_ui::Result<()> {
        let owner = self
            .presenter
            .resources
            .owners
            .get(&self.parent.id())
            .ok_or_else(|| maka_ui::Error::Invalid("terminal native owner unavailable".into()))?;
        let area = area.clipped(
            self.placement
                .ok_or_else(|| {
                    maka_ui::Error::Invalid("cannot publish hits while measuring".into())
                })?
                .visible(),
        );
        if area.is_empty() {
            return Ok(());
        }
        let target = Target {
            identity: TargetIdentity {
                mount: self.parent.clone(),
                key: key.into(),
                owner: owner.identity.clone(),
            },
            revision: self.presenter.application.frame(self.parent)?.revision,
            area,
            tab_index,
            kind: TargetKind::Native,
            ancestors: self.scene.as_ref().unwrap().ancestry.clone(),
        };
        self.scene
            .as_deref_mut()
            .ok_or_else(|| maka_ui::Error::Invalid("cannot publish hits while measuring".into()))?
            .targets
            .push(target);
        Ok(())
    }
    pub(super) fn participant(
        &mut self,
        document: &super::selection::DocumentRef,
    ) -> Option<super::selection::SelectionScope> {
        self.scene
            .as_mut()
            .and_then(|scene| scene.scope.as_mut())
            .map(|scope| {
                scope.documents.push(std::rc::Rc::downgrade(document));
                scope.scope.clone()
            })
    }
    pub fn focused(&self) -> bool {
        self.presenter.focus.as_ref().is_some_and(|target| {
            target.mount == *self.parent
                && self
                    .presenter
                    .resources
                    .owners
                    .get(&self.parent.id())
                    .is_some_and(|owner| std::rc::Rc::ptr_eq(&owner.identity, &target.owner))
        })
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.declarations.iter().map(|slot| slot.key.as_str())
    }
    pub fn measure(&self, key: &str, width: u16) -> maka_ui::Result<(u16, u32)> {
        let child = self.presenter.application.child_mount(self.parent, key)?;
        if !self
            .presenter
            .application
            .frame(self.parent)?
            .view
            .visible_slots()
            .any(|slot| slot.key == key)
        {
            return Ok((0, 0));
        }
        self.presenter
            .measure_mount(child, width, self.measurements)
    }
    pub fn paint(
        &mut self,
        key: &str,
        area: Placement,
        buffer: &mut Buffer,
    ) -> maka_ui::Result<()> {
        let child = self.presenter.application.child_mount(self.parent, key)?;
        if !self
            .presenter
            .application
            .frame(self.parent)?
            .view
            .visible_slots()
            .any(|slot| slot.key == key)
        {
            return Ok(());
        }
        let scene = self
            .scene
            .as_deref_mut()
            .ok_or_else(|| maka_ui::Error::Invalid("cannot paint during measurement".into()))?;
        self.presenter.paint_mount(
            child,
            area.clipped(buffer.area)
                .clipped(self.placement.unwrap().visible()),
            buffer,
            scene,
            self.measurements,
        )
    }
}

fn color(rgb: u32) -> Color {
    Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}
impl Presenter {
    pub fn paint(&self, area: Rect, buffer: &mut Buffer) -> maka_ui::Result<PaintedScene> {
        if self.closing {
            return Err(maka_ui::Error::Invalid("terminal surface retired".into()));
        }
        let area = area.intersection(buffer.area);
        buffer.set_style(
            area,
            Style::default()
                .bg(Color::Rgb(23, 23, 26))
                .fg(Color::Rgb(235, 235, 240)),
        );
        let mut scene = PaintedScene {
            targets: Vec::new(),
            placed: Default::default(),
            laid_out: Default::default(),
            scope: None,
            geometries: Vec::new(),
            ancestry: Vec::new(),
        };
        let measurements = Measurements::default();
        self.paint_mount(&self.root, area.into(), buffer, &mut scene, &measurements)?;
        Ok(scene)
    }
    pub fn present(&mut self, scene: PaintedScene) {
        if self.closing {
            return;
        }
        self.resources.presentation(&scene.placed);
        for pending in scene.geometries {
            self.resources
                .present(&pending.mount, &pending.owner, pending.geometry);
        }
        self.drag = self
            .drag
            .take()
            .filter(|target| self.resources.pointer_capture(target).unwrap_or(false))
            .and_then(|old| scene.targets.iter().find(|new| new.same(&old)).cloned());
        self.focus = self
            .focus
            .take()
            .and_then(|old| scene.targets.iter().find(|new| new.same(&old)).cloned())
            .or_else(|| {
                scene
                    .targets
                    .iter()
                    .filter(|target| target.tab_index >= 0)
                    .min_by_key(|target| (target.tab_index == 0, target.tab_index))
                    .cloned()
            });
        self.scene = scene.targets;
    }
    pub(super) fn measure_mount(
        &self,
        mount: &Mount,
        width: u16,
        measurements: &Measurements,
    ) -> maka_ui::Result<(u16, u32)> {
        measurements.get_or(mount.id(), MeasurementKey::Mount, width, || {
            let native = &self.application.frame(mount)?.view;
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                native.view.measure(
                    &Slots {
                        presenter: self,
                        parent: mount,
                        declarations: native.slots(),
                        scene: None,
                        measurements,
                        placement: None,
                    },
                    width,
                )
            }))
            .map_err(|_| maka_ui::Error::Invalid("terminal native measurement panicked".into()))?
        })
    }
    fn visible_children<'a>(
        &self,
        mount: &Mount,
        children: &'a [Node],
    ) -> maka_ui::Result<Vec<&'a Node>> {
        let mut visible = Vec::with_capacity(children.len());
        for child in children {
            let shown = match child {
                Node::Component(slot) => self
                    .application
                    .frame(self.application.child_mount(mount, &slot.key)?)?
                    .view
                    .root_visible(),
                _ => child.visible(),
            };
            if shown {
                visible.push(child);
            }
        }
        Ok(visible)
    }
    fn node_layout(&self, mount: &Mount, node: &Node) -> maka_ui::Result<super::scene::Layout> {
        match node {
            Node::Component(slot) => Ok(self
                .application
                .frame(self.application.child_mount(mount, &slot.key)?)?
                .view
                .view
                .layout()),
            _ => Ok(node.layout()),
        }
    }
    pub(super) fn measure(
        &self,
        mount: &Mount,
        node: &Node,
        width: u16,
        measurements: &Measurements,
    ) -> maka_ui::Result<(u16, u32)> {
        measurements.get_or(
            mount.id(),
            MeasurementKey::Node(node as *const Node as usize),
            width,
            || {
                if !node.visible() {
                    return Ok((0, 0));
                }
                Ok(match node {
                    Node::Text { text, .. } => text_size(text, width),
                    Node::Button { label, .. } => (
                        sanitize(label)
                            .replace('\n', " ")
                            .width()
                            .saturating_add(4)
                            .min(usize::from(width)) as u16,
                        1,
                    ),
                    Node::Component(slot) => {
                        return self.measure_mount(
                            self.application.child_mount(mount, &slot.key)?,
                            width,
                            measurements,
                        );
                    }
                    Node::Stack {
                        direction,
                        gap,
                        padding,
                        children,
                        layout,
                        ..
                    } => {
                        let inset = padding.saturating_mul(2);
                        let available = width.saturating_sub(inset);
                        let children = self.visible_children(mount, children)?;
                        let layouts: Vec<_> = children
                            .iter()
                            .map(|child| self.node_layout(mount, child))
                            .collect::<maka_ui::Result<Vec<_>>>()?;
                        let flow = super::layout::measured(
                            *direction,
                            available,
                            None,
                            *gap,
                            layout.wrap,
                            &layouts,
                            |index, width| {
                                self.measure(mount, children[index], width, measurements)
                            },
                        )?;
                        (
                            flow.size.0.saturating_add(inset).min(width),
                            flow.size.1.saturating_add(u32::from(inset)),
                        )
                    }
                })
            },
        )
    }
    fn paint_mount(
        &self,
        mount: &Mount,
        area: Placement,
        buffer: &mut Buffer,
        scene: &mut PaintedScene,
        measurements: &Measurements,
    ) -> maka_ui::Result<()> {
        if area.width == 0
            || area.height == 0
            || !self.application.frame(mount)?.view.root_visible()
        {
            return Ok(());
        }
        if !scene.laid_out.insert(mount.id()) {
            return Err(maka_ui::Error::Invalid(
                "component placed twice in terminal scene".into(),
            ));
        }
        if !area.is_empty() {
            scene.placed.insert(mount.id());
        }
        let native = &self.application.frame(mount)?.view;
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            native.view.paint(
                &mut Slots {
                    presenter: self,
                    parent: mount,
                    declarations: native.slots(),
                    scene: Some(scene),
                    measurements,
                    placement: Some(area),
                },
                area,
                buffer,
            )
        }))
        .map_err(|_| maka_ui::Error::Invalid("terminal native painting panicked".into()))?
    }
    pub(super) fn paint_node(
        &self,
        mount: &Mount,
        node: &Node,
        area: Placement,
        buffer: &mut Buffer,
        scene: &mut PaintedScene,
        pass: &mut NodePaint<'_>,
    ) -> maka_ui::Result<()> {
        let measurements = pass.measurements;
        if !node.visible() || area.width == 0 || area.height == 0 {
            return Ok(());
        }
        match node {
            Node::Text {
                text: content,
                color: rgb,
                ..
            } => text(content, area, buffer, Style::default().fg(color(*rgb))),
            Node::Button { key, label } => {
                let label = format!("[ {} ]", sanitize(label).replace('\n', " "));
                let target = Target {
                    kind: TargetKind::Action,
                    ancestors: scene.ancestry.clone(),
                    revision: self.application.frame(mount)?.revision,
                    area: Placement::new(CellRect::new(area.x, area.y, area.width, 1), area.clip),
                    tab_index: 0,
                    identity: TargetIdentity {
                        mount: mount.clone(),
                        key: key.clone(),
                        owner: self
                            .resources
                            .owners
                            .get(&mount.id())
                            .ok_or_else(|| {
                                maka_ui::Error::Invalid("terminal native owner unavailable".into())
                            })?
                            .identity
                            .clone(),
                    },
                };
                if target.area.is_empty() {
                    return Ok(());
                }
                let mut style = Style::default().fg(Color::Rgb(156, 194, 230));
                if self.focus.as_ref().is_some_and(|old| old.same(&target)) {
                    style = style.add_modifier(Modifier::REVERSED);
                }
                // Buttons are single-line, clipped rather than wrapped to hidden hit cells.
                text(&label, target.area, buffer, style);
                scene.targets.push(target);
            }
            Node::Component(slot) => self.paint_mount(
                self.application.child_mount(mount, &slot.key)?,
                area,
                buffer,
                scene,
                measurements,
            )?,
            Node::Stack {
                key,
                scroll,
                direction,
                gap,
                padding,
                background,
                children,
                ..
            } => {
                buffer.set_style(area.visible(), Style::default().bg(color(*background)));
                let inner = area.inner(ratatui::layout::Margin::new(*padding, *padding));
                let children = self.visible_children(mount, children)?;
                let layouts: Vec<_> = children
                    .iter()
                    .map(|child| self.node_layout(mount, child))
                    .collect::<maka_ui::Result<Vec<_>>>()?;
                let flow = super::layout::measured(
                    *direction,
                    inner.width,
                    if *scroll == super::scene::ScrollPolicy::None {
                        Some(inner.height)
                    } else {
                        None
                    },
                    *gap,
                    node.layout().wrap,
                    &layouts,
                    |index, width| self.measure(mount, children[index], width, measurements),
                )?;
                let scrolling = *scroll != super::scene::ScrollPolicy::None;
                let offset = if scrolling {
                    let identities: Vec<_> = children
                        .iter()
                        .zip(&flow.children)
                        .map(|(child, rect)| (child.key().to_owned(), rect.y as u32, rect.height))
                        .collect();
                    let state = self.resources.owners[&mount.id()]
                        .state
                        .downcast_ref::<super::scroll::SceneState>()
                        .unwrap();
                    let viewport = state.candidate(key, *scroll, inner, flow.size.1, &identities);
                    let offset = viewport.offset;
                    pass.viewports.push((key.clone(), viewport));
                    let identity = TargetIdentity {
                        mount: mount.clone(),
                        key: key.clone(),
                        owner: self.resources.owners[&mount.id()].identity.clone(),
                    };
                    if !inner.is_empty() {
                        scene.targets.push(Target {
                            identity: identity.clone(),
                            revision: self.application.frame(mount)?.revision,
                            area: inner,
                            tab_index: -1,
                            kind: TargetKind::Scroll,
                            ancestors: scene.ancestry.clone(),
                        });
                    }
                    scene.ancestry.push(identity);
                    offset
                } else {
                    0
                };
                for (child, relative) in children.into_iter().zip(flow.children) {
                    let area = Placement::new(
                        relative.translated(inner.x, inner.y - i64::from(offset)),
                        inner.visible(),
                    );
                    self.paint_node(mount, child, area, buffer, scene, pass)?;
                }
                if scrolling {
                    scene.ancestry.pop();
                }
            }
        }
        Ok(())
    }
}
