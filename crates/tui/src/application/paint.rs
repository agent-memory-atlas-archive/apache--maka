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
    layout::{Rect, Size},
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
/// Original session::fit cell/grapheme elision, over one sanitized line.
pub(super) fn truncated_line(text: &str, width: u16) -> String {
    if width == 0 {
        return String::new();
    }
    let text = sanitize(text).replace('\n', " ");
    let width = usize::from(width);
    if text.width() <= width {
        return text;
    }
    let mut result = String::new();
    let mut cells = 0;
    for grapheme in text.graphemes(true) {
        cells += grapheme.width();
        if cells + 1 > width {
            break;
        }
        result.push_str(grapheme);
    }
    result.push('…');
    result
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
fn paint_button(
    label: &str,
    area: Placement,
    buffer: &mut Buffer,
    style: Style,
    align: Option<maka_ui::scene::terminal::Align>,
) {
    let area = area.clipped(buffer.area);
    buffer.set_style(area.visible(), style.remove_modifier(Modifier::UNDERLINED));
    let spare = area.width.saturating_sub(label.width() as u16);
    let offset = match align.unwrap_or(maka_ui::scene::terminal::Align::Center) {
        maka_ui::scene::terminal::Align::Start => 0,
        maka_ui::scene::terminal::Align::Center => spare / 2,
        maka_ui::scene::terminal::Align::End => spare,
    };
    text(
        label,
        Placement::new(
            CellRect::new(area.x + i64::from(offset), area.y, area.width - offset, 1),
            area.clip,
        ),
        buffer,
        style,
    );
}
#[derive(Clone)]
pub(super) enum TargetKind {
    SelectionCopy {
        text: std::rc::Rc<str>,
    },
    ContextMenu {
        button: bool,
    },
    Action {
        copy_text: Option<std::rc::Rc<str>>,
    },
    Item {
        current: bool,
        follow_focus: bool,
        submit: bool,
        submit_space: bool,
        reading_child: Option<String>,
    },
    Focus(FocusChild),
    Native,
    Scroll,
    Backdrop {
        body: Placement,
    },
}
#[derive(Clone)]
pub(super) struct TargetIdentity {
    pub mount: Mount,
    pub key: String,
    pub owner: std::rc::Rc<()>,
    pub scope: Option<super::dialog::Scope>,
}
/// Original successfully painted ancestry, including across component slots.
#[derive(Clone)]
pub(super) struct Ancestor {
    pub identity: TargetIdentity,
    pub revision: u64,
    pub shortcuts: Vec<maka_ui::shortcut::Shortcut>,
    pub focus_group: Option<maka_ui::scene::terminal::Direction>,
    pub scroll: Option<super::scroll::Viewport>,
    pub focus_shortcuts: Vec<FocusBinding>,
}
#[derive(Clone)]
pub(super) struct FocusBinding {
    pub declaration: maka_ui::shortcut::FocusShortcut,
    pub target: FocusChild,
}
#[derive(Clone)]
pub(super) struct FocusChild {
    pub child: Mount,
    pub owner: std::rc::Weak<()>,
}
impl std::ops::Deref for Ancestor {
    type Target = TargetIdentity;
    fn deref(&self) -> &TargetIdentity {
        &self.identity
    }
}
#[derive(Clone)]
pub(super) struct Target {
    pub identity: TargetIdentity,
    pub revision: u64,
    pub area: Placement,
    pub tab_index: i16,
    pub autofocus: bool,
    pub kind: TargetKind,
    pub ancestors: Vec<std::rc::Rc<Ancestor>>,
}
impl std::ops::Deref for Target {
    type Target = TargetIdentity;
    fn deref(&self) -> &TargetIdentity {
        &self.identity
    }
}
impl Target {
    pub fn role(&self) -> (std::mem::Discriminant<TargetKind>, bool) {
        (
            std::mem::discriminant(&self.kind),
            matches!(self.kind, TargetKind::ContextMenu { button: true }),
        )
    }
    pub fn same(&self, other: &Self) -> bool {
        self.mount == other.mount
            && self.key == other.key
            && std::rc::Rc::ptr_eq(&self.owner, &other.owner)
            && self.role() == other.role()
    }
}
/// A pending scene is not input geometry until the embedding confirms a flush.
/// 绘制成功并完成实际 flush 后，才发布对应的命中区域。
pub struct PaintedScene {
    tooltip: Option<(Option<super::dialog::Scope>, Placement, String, Style)>,
    pub(super) viewport: Rect,
    pub(super) targets: Vec<Target>,
    pub(super) keyboard: Option<std::rc::Rc<Ancestor>>,
    focus_offers: Vec<(TargetIdentity, Target)>,
    placed: std::collections::BTreeSet<uuid::Uuid>,
    laid_out: std::collections::BTreeMap<uuid::Uuid, PaintedMount>,
    dependencies: std::collections::BTreeMap<uuid::Uuid, PaintedMount>,
    pub(super) scope: Option<super::selection::ScopePaint>,
    pub(super) geometries: Vec<NativeGeometry>,
    pub(super) ancestry: Vec<std::rc::Rc<Ancestor>>,
    pub(super) input_scope: Option<super::dialog::Scope>,
    pub(super) dialogs: std::collections::VecDeque<super::dialog::Deferred>,
    openings: Vec<super::dialog::Scope>,
    pub(super) input_path: Vec<super::dialog::Scope>,
    pub(super) foreground_path: Vec<super::dialog::Scope>,
}

struct PaintedMount {
    mount: Mount,
    revision: u64,
    view: maka_ui::NativeView<dyn super::Widget>,
    owner: std::rc::Weak<()>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MeasurementKey {
    Mount,
    Node(usize),
}
type MeasurementCache =
    std::collections::BTreeMap<(uuid::Uuid, MeasurementKey, u16, (u16, u16)), (u16, u32)>;
/// Derived, paint-local memoization. No entry survives the immutable render pass
/// or participates in input/continuation/geometry admission.
pub(super) struct Measurements {
    time: std::time::Instant,
    cache: std::cell::RefCell<MeasurementCache>,
    #[cfg(test)]
    requests: std::cell::Cell<usize>,
    #[cfg(test)]
    limit: std::cell::Cell<usize>,
}
impl Default for Measurements {
    fn default() -> Self {
        Self {
            time: std::time::Instant::now(),
            cache: Default::default(),
            #[cfg(test)]
            requests: Default::default(),
            #[cfg(test)]
            limit: Default::default(),
        }
    }
}
impl Measurements {
    pub(super) fn context(&self, viewport: Size) -> MeasureContext<'_> {
        MeasureContext {
            cache: self,
            viewport,
        }
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
/// Immutable layout context borrowing the current paint's only measurement cache.
#[derive(Clone, Copy)]
pub(super) struct MeasureContext<'a> {
    cache: &'a Measurements,
    viewport: Size,
}
impl MeasureContext<'_> {
    pub(super) fn time(self) -> std::time::Instant {
        self.cache.time
    }
    fn within(self, viewport: Size) -> Self {
        Self { viewport, ..self }
    }
    fn get_or(
        &self,
        mount: uuid::Uuid,
        key: MeasurementKey,
        width: u16,
        measure: impl FnOnce() -> maka_ui::Result<(u16, u32)>,
    ) -> maka_ui::Result<(u16, u32)> {
        #[cfg(test)]
        {
            let requests = self.cache.requests.get() + 1;
            self.cache.requests.set(requests);
            assert!(
                self.cache.limit.get() == 0 || requests <= self.cache.limit.get(),
                "measurement work exceeded fixture bound"
            );
        }
        let key = (
            mount,
            key,
            width,
            (self.viewport.width, self.viewport.height),
        );
        if let Some(size) = self.cache.cache.borrow().get(&key).copied() {
            return Ok(size);
        }
        let size = measure()?;
        self.cache.cache.borrow_mut().insert(key, size);
        Ok(size)
    }
}

/// Derived data for one immutable scene recipe's paint pass.
pub(super) struct NodePaint<'a> {
    pub measurements: MeasureContext<'a>,
    pub viewports: &'a mut Vec<(String, super::scroll::Viewport)>,
    pub choices: &'a mut Vec<super::choice::Geometry>,
    pub panes: &'a mut Vec<super::adaptive::Geometry>,
}

/// Synchronous terminal geometry over one parent's committed component slots.
/// Measuring is read-only; painting publishes hit geometry only after flush.
pub struct Slots<'a> {
    pub(super) presenter: &'a Presenter,
    pub(super) parent: &'a Mount,
    declarations: &'a [maka_ui::ComponentSlot],
    pub(super) scene: Option<&'a mut PaintedScene>,
    pub(super) measurements: MeasureContext<'a>,
    placement: Option<Placement>,
}
pub(super) struct NativeGeometry {
    pub mount: Mount,
    pub owner: std::rc::Rc<()>,
    pub geometry: Box<dyn std::any::Any>,
}
impl Slots<'_> {
    /// Nearest declared layout viewport, or the surface by default. Ordinary
    /// wrappers and component slots preserve it; clipping does not resize it.
    pub fn viewport(&self) -> Size {
        self.measurements.viewport
    }
    pub fn native_state<T: std::any::Any>(&self) -> maka_ui::Result<&T> {
        self.presenter
            .resources
            .owners
            .get(&self.parent.id())
            .and_then(|owner| owner.state.downcast_ref())
            .ok_or_else(|| maka_ui::Error::Invalid("terminal native state unavailable".into()))
    }
    /// Include an unpainted recipe in this flush's existing owner/recipe proof.
    /// Reading find inspects folded descendants without placing or focusing them.
    pub(super) fn depend_on(&mut self, mount: &Mount) -> maka_ui::Result<()> {
        let frame = self.presenter.application.frame(mount)?;
        let owner = self
            .presenter
            .resources
            .owners
            .get(&mount.id())
            .ok_or_else(|| maka_ui::Error::Invalid("terminal native owner unavailable".into()))?;
        self.scene
            .as_mut()
            .ok_or_else(|| {
                maka_ui::Error::Invalid("cannot collect paint dependencies while measuring".into())
            })?
            .dependencies
            .insert(
                mount.id(),
                PaintedMount {
                    mount: mount.clone(),
                    revision: frame.revision,
                    view: frame.view.clone(),
                    owner: std::rc::Rc::downgrade(&owner.identity),
                },
            );
        Ok(())
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
        autofocus: bool,
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
                scope: self.scene.as_ref().unwrap().input_scope.clone(),
            },
            revision: self.presenter.application.frame(self.parent)?.revision,
            area,
            tab_index,
            autofocus,
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
        area: Placement,
    ) -> Option<super::selection::SelectionScope> {
        self.scene
            .as_mut()
            .and_then(|scene| scene.scope.as_mut())
            .map(|scope| {
                scope.documents.push(super::selection::Participant {
                    document: std::rc::Rc::downgrade(document),
                    child: scope.child.clone(),
                    geometry: Some((area, document.borrow().geometry_revision)),
                });
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

    pub(super) fn animation_phase(&self, period: u32) -> Option<f32> {
        self.presenter.window_focused.then(|| {
            (self
                .measurements
                .cache
                .time
                .saturating_duration_since(self.presenter.animation_origin)
                .as_millis()
                % u128::from(period)) as f32
                / period as f32
        })
    }

    /// The original focused native input within this mounted widget, if any.
    pub fn focused_key(&self) -> Option<&str> {
        self.presenter
            .focus
            .as_ref()
            .filter(|_| self.focused())
            .map(|target| target.key.as_str())
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

pub(super) fn color(value: impl Into<maka_ui::scene::terminal::Color>) -> Color {
    use maka_ui::scene::terminal::{Color as Input, NamedColor as N};
    match value.into() {
        Input::Rgb(rgb) => Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8),
        Input::Named(named) => match named {
            N::Reset => Color::Reset,
            N::Black => Color::Black,
            N::Red => Color::Red,
            N::Green => Color::Green,
            N::Yellow => Color::Yellow,
            N::Blue => Color::Blue,
            N::Magenta => Color::Magenta,
            N::Cyan => Color::Cyan,
            N::Gray => Color::Gray,
            N::DarkGray => Color::DarkGray,
            N::LightRed => Color::LightRed,
            N::LightGreen => Color::LightGreen,
            N::LightYellow => Color::LightYellow,
            N::LightBlue => Color::LightBlue,
            N::LightMagenta => Color::LightMagenta,
            N::LightCyan => Color::LightCyan,
            N::White => Color::White,
        },
    }
}
impl Presenter {
    pub fn paint(&self, area: Rect, buffer: &mut Buffer) -> maka_ui::Result<PaintedScene> {
        if !self.surface.is_live() {
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
            tooltip: None,
            viewport: area,
            targets: Vec::new(),
            focus_offers: Vec::new(),
            placed: Default::default(),
            laid_out: Default::default(),
            dependencies: Default::default(),
            scope: None,
            geometries: Vec::new(),
            ancestry: Vec::new(),
            keyboard: None,
            input_scope: None,
            dialogs: Default::default(),
            openings: self.opening_witnesses()?,
            input_path: Vec::new(),
            foreground_path: Vec::new(),
        };
        if !self.resources.ready(&self.application, &self.root)? {
            return Ok(scene);
        }
        let cache = Measurements::default();
        let measurements = cache.context(Size {
            width: area.width,
            height: area.height,
        });
        self.paint_mount(&self.root, area.into(), buffer, &mut scene, measurements)?;
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.paint_dialogs(buffer, &mut scene, measurements)
        }))
        .map_err(|_| maka_ui::Error::Invalid("terminal native painting panicked".into()))??;
        if let Some((scope, anchor, hint, style)) = &scene.tooltip
            && super::dialog::same_scope(scope, &scene.foreground_path.last().cloned())
            && area.height > 1
        {
            let width = (sanitize(hint).width() as u16)
                .saturating_add(2)
                .min(area.width);
            let x = anchor.visible().x.min(area.right().saturating_sub(width));
            let y = if anchor.visible().bottom() < area.bottom() {
                anchor.visible().bottom()
            } else {
                anchor.visible().y.saturating_sub(1).max(area.y)
            };
            let tooltip: Placement = Rect::new(x, y, width, 1).into();
            buffer.set_stringn(
                x,
                y,
                " ".repeat(usize::from(width)),
                usize::from(width),
                *style,
            );
            paint_button(
                &truncated_line(hint, width.saturating_sub(2)),
                tooltip,
                buffer,
                *style,
                None,
            );
        }
        Ok(scene)
    }
    pub fn present(&mut self, mut scene: PaintedScene) {
        if !self.surface.is_live() {
            return;
        }
        // A delayed flush cannot withdraw newer presentation or replace its
        // geometry. Native observations can revise business state while the
        // complete immutable recipe remains the same (e.g. selection during a
        // resize). Validate that distinction and the original owner as a batch.
        if !self.scene_openings_current(&scene.openings)
            || !scene.laid_out.contains_key(&self.root.id())
        {
            return;
        }
        let mut frames_current = true;
        for painted in scene.laid_out.values().chain(scene.dependencies.values()) {
            let Ok(frame) = self.application.frame(&painted.mount) else {
                return;
            };
            let frame_current = frame.revision == painted.revision;
            if self.application.is_retiring(&painted.mount)
                || !frame.view.same_presentation_metadata(&painted.view)
                || !self
                    .resources
                    .owners
                    .get(&painted.mount.id())
                    .is_some_and(|owner| {
                        owner.live
                            && painted
                                .owner
                                .ptr_eq(&std::rc::Rc::downgrade(&owner.identity))
                    })
                || !(frame_current
                    || std::rc::Rc::ptr_eq(&frame.view.view, &painted.view.view)
                    || frame
                        .view
                        .view
                        .same_presentation(painted.view.view.as_ref()))
            {
                return;
            }
            frames_current &= frame_current;
        }
        self.reconcile_choices(&scene);
        let scope = scene.foreground_path.last().cloned();
        scene
            .targets
            .retain(|target| super::dialog::same_scope(&target.scope, &scope));
        if !scene
            .keyboard
            .as_ref()
            .is_some_and(|keyboard| super::dialog::same_scope(&keyboard.scope, &scope))
        {
            scene.keyboard = None;
        }
        if !super::dialog::same_path(&self.physical_path, &scene.foreground_path) {
            if let Err(error) = self.resources.cancel_pointer() {
                self.report(&error);
                return;
            }
            if let Err(error) = self.clear_hover() {
                self.report(&error);
                return;
            }
            self.drag = None;
            if self.focus_before_flush.is_none() {
                self.focus_before_flush = self.focus.as_ref().map(super::dialog::WeakTarget::new);
            }
            self.focus = None;
        }
        if let Err(error) = self.reconcile_hover(&scene.targets) {
            self.report(&error);
            return;
        }
        for opening in &scene.openings {
            if let Some(opening) = opening.upgrade() {
                opening.pending_flush.set(false);
            }
        }
        self.publish_modal_focus(&scene.foreground_path, &scene.targets);
        self.physical_path = scene.foreground_path;
        self.resources.presentation(&scene.placed);
        for pending in scene.geometries {
            self.resources
                .present(&pending.mount, &pending.owner, pending.geometry);
        }
        let mut autofocus = None;
        for target in &scene.targets {
            if frames_current
                && target.autofocus
                && self.application.mount_visible(&target.mount)
                && self.resources.present_focus(target)
            {
                autofocus.get_or_insert_with(|| target.clone());
            }
        }
        for (container, candidate) in scene.focus_offers {
            if !frames_current
                || !super::dialog::same_scope(&container.scope, &scope)
                || !self.application.mount_visible(&container.mount)
                || !scene.targets.iter().any(|target| {
                    target.same(&candidate)
                        && super::dialog::same_scope(&target.scope, &candidate.scope)
                })
            {
                continue;
            }
            let Some(owner) = self.resources.owners.get_mut(&container.mount.id()) else {
                continue;
            };
            if !owner.live || !std::rc::Rc::ptr_eq(&owner.identity, &container.owner) {
                continue;
            }
            if owner
                .state
                .downcast_mut::<super::scroll::SceneState>()
                .and_then(|state| state.focus.get_mut(&container.key))
                .is_some_and(maka_ui::focus::FocusOnReveal::presented)
            {
                autofocus.get_or_insert(candidate);
            }
        }
        self.drag = self
            .drag
            .take()
            .filter(|target| self.resources.pointer_capture(target).unwrap_or(false))
            .and_then(|old| scene.targets.iter().find(|new| new.same(&old)).cloned());
        // A newly revealed control wins this presentation. Consume the
        // modal return either way; it must not steal focus on a later frame.
        let restored = self
            .restore_focus
            .take()
            .and_then(|target| target.resolve(&scene.targets));
        let reading_focus = frames_current
            .then(|| self.reading_selection_focus(&scene.targets))
            .flatten();
        self.focus = autofocus
            .or(restored)
            .or(reading_focus)
            .or_else(|| {
                self.focus
                    .take()
                    .and_then(|old| scene.targets.iter().find(|new| new.same(&old)).cloned())
            })
            .or_else(|| {
                scene
                    .targets
                    .iter()
                    .filter(|target| {
                        target.tab_index >= 0
                            && !matches!(self.native_return, Some(super::focus::Return::Exited))
                    })
                    .min_by_key(|target| (target.tab_index == 0, target.tab_index))
                    .cloned()
            });
        self.keyboard = scene.keyboard;
        self.scene = scene.targets;
        if frames_current {
            let mounts: Vec<_> = scene
                .laid_out
                .values()
                .map(|painted| painted.mount.clone())
                .collect();
            if let Err(error) = self.apply_reading_focus(&mounts) {
                self.report(&error);
                return;
            }
        }
        if let Err(error) = self.track_focus_return() {
            self.report(&error);
        }
    }
    pub(super) fn measure_mount(
        &self,
        mount: &Mount,
        width: u16,
        measurements: MeasureContext<'_>,
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
    pub(super) fn node_layout(
        &self,
        mount: &Mount,
        node: &Node,
        measurements: MeasureContext<'_>,
    ) -> maka_ui::Result<super::scene::Layout> {
        match node {
            Node::Dialog { background, .. }
            | Node::ContextMenu { background, .. }
            | Node::AdaptivePane { background, .. } => {
                self.node_layout(mount, background, measurements)
            }
            Node::Item { content, .. } => self.node_layout(mount, content, measurements),
            Node::Component(slot) => {
                let child = self.application.child_mount(mount, &slot.key)?;
                let native = &self.application.frame(child)?.view;
                native.view.layout(&Slots {
                    presenter: self,
                    parent: child,
                    declarations: native.slots(),
                    scene: None,
                    measurements,
                    placement: None,
                })
            }
            _ => Ok(node.layout()),
        }
    }
    pub(super) fn measure(
        &self,
        mount: &Mount,
        node: &Node,
        width: u16,
        measurements: MeasureContext<'_>,
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
                    Node::Dialog { background, .. } => {
                        self.measure(mount, background, width, measurements)?
                    }
                    Node::AdaptivePane {
                        key,
                        background,
                        content,
                        ..
                    } => {
                        let state = self.resources.owners[&mount.id()]
                            .state
                            .downcast_ref::<super::scroll::SceneState>()
                            .unwrap();
                        let inline = if state.dialogs.contains_key(key) {
                            0
                        } else {
                            state.panes[key].width(node, width, measurements.cache.time)
                        };
                        let (body_width, body_height) = self.measure(
                            mount,
                            background,
                            width.saturating_sub(inline),
                            measurements,
                        )?;
                        let (pane_width, pane_height) = if inline > 0 {
                            self.measure(mount, content, inline, measurements)?
                        } else {
                            (0, 0)
                        };
                        (
                            body_width.saturating_add(pane_width),
                            body_height.max(pane_height),
                        )
                    }
                    Node::ContextMenu { background, .. } => {
                        let (w, h) =
                            self.measure(mount, background, width.saturating_sub(3), measurements)?;
                        (w.saturating_add(3).min(width), h.max(1))
                    }
                    Node::Choice(choice) => super::choice::size(choice, width),
                    Node::Item { content, .. } => {
                        self.measure(mount, content, width, measurements)?
                    }
                    Node::Text {
                        text,
                        truncate,
                        animation,
                        ..
                    } => std::iter::once(text.as_str())
                        .chain(animation.iter().map(|frame| frame.text.as_str()))
                        .map(|text| {
                            if *truncate {
                                (
                                    truncated_line(text, width).width() as u16,
                                    u32::from(width > 0),
                                )
                            } else {
                                text_size(text, width)
                            }
                        })
                        .fold((0, 0), |a, b| (a.0.max(b.0), a.1.max(b.1))),
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
                        wide,
                        gap,
                        padding,
                        border,
                        children,
                        layout,
                        viewport,
                        ..
                    } => {
                        let inset = (padding + u16::from(border.is_some())).saturating_mul(2);
                        if *viewport {
                            return Ok((inset.min(width), u32::from(inset)));
                        }
                        let available = width.saturating_sub(inset);
                        let wide = wide.as_ref().filter(|wide| available >= wide.min_width);
                        let children = self.visible_children(mount, children)?;
                        let layouts: Vec<_> = children
                            .iter()
                            .map(|child| {
                                self.node_layout(mount, child, measurements).map(|layout| {
                                    wide.and_then(|wide| wide.children.get(child.key()))
                                        .copied()
                                        .unwrap_or(layout)
                                })
                            })
                            .collect::<maka_ui::Result<Vec<_>>>()?;
                        let flow = super::layout::measured(
                            wide.map_or(*direction, |wide| wide.direction),
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
        measurements: MeasureContext<'_>,
    ) -> maka_ui::Result<()> {
        let native_frame = self.application.frame(mount)?;
        let owner = &self
            .resources
            .owners
            .get(&mount.id())
            .ok_or_else(|| maka_ui::Error::Invalid("terminal native owner unavailable".into()))?
            .identity;
        if scene
            .laid_out
            .insert(
                mount.id(),
                PaintedMount {
                    mount: mount.clone(),
                    revision: native_frame.revision,
                    view: native_frame.view.clone(),
                    owner: std::rc::Rc::downgrade(owner),
                },
            )
            .is_some()
        {
            return Err(maka_ui::Error::Invalid(
                "component placed twice in terminal scene".into(),
            ));
        }
        if area.width == 0 || area.height == 0 || !native_frame.view.root_visible() {
            return Ok(());
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
            Node::Item {
                key,
                content,
                disabled,
                current,
                selected_color,
                selected_foreground,
                follow_focus,
                submit,
                submit_space,
            } => {
                let target = Target {
                    identity: TargetIdentity {
                        mount: mount.clone(),
                        key: key.clone(),
                        owner: self.resources.owners[&mount.id()].identity.clone(),
                        scope: scene.input_scope.clone(),
                    },
                    kind: TargetKind::Item {
                        current: *current,
                        follow_focus: *follow_focus,
                        submit: *submit,
                        submit_space: *submit_space,
                        reading_child: scene.scope.as_ref().and_then(|scope| scope.child.clone()),
                    },
                    ancestors: scene.ancestry.clone(),
                    revision: self.application.frame(mount)?.revision,
                    area,
                    tab_index: 0,
                    autofocus: false,
                };
                // Retain the logical row even outside the viewport. Keyboard
                // navigation reveals it; pointer hit testing still uses clip.
                if !disabled {
                    scene.targets.push(target.clone());
                }
                self.paint_node(mount, content, area, buffer, scene, pass)?;
                // Original Surface: current fills the row, focus underlines,
                // and hover adds emphasis without changing selection.
                let style = if *disabled {
                    Style::default().add_modifier(Modifier::DIM)
                } else {
                    let mut style = if *current {
                        selected_color.map_or_else(
                            || Style::default().add_modifier(Modifier::REVERSED),
                            |selected| Style::default().bg(color(selected)),
                        )
                    } else {
                        Style::default()
                    };
                    if *current {
                        style = style.add_modifier(Modifier::BOLD);
                        if let Some(foreground) = selected_foreground {
                            style = style.fg(color(*foreground));
                        }
                    }
                    if self.focus.as_ref().is_some_and(|old| old.same(&target)) {
                        style = style.add_modifier(Modifier::UNDERLINED);
                    } else if self.hover.as_ref().is_some_and(|old| old.same(&target)) {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    style
                };
                buffer.set_style(area.visible(), style);
            }
            Node::AdaptivePane {
                key,
                background,
                content,
                overlay,
                ..
            } => {
                let state = self.resources.owners[&mount.id()]
                    .state
                    .downcast_ref::<super::scroll::SceneState>()
                    .unwrap();
                let opening = state.dialogs.get(key);
                let inline = if opening.is_some() {
                    0
                } else {
                    state.panes[key].width(node, area.width, measurements.cache.time)
                };
                pass.panes.push(super::adaptive::Geometry {
                    key: key.clone(),
                    width: area.width,
                    inline_width: inline,
                    target: state.panes[key].target(node, area.width),
                    time: measurements.cache.time,
                });
                if inline > 0 {
                    let pane = Placement::new(
                        CellRect::new(area.x, area.y, inline.saturating_sub(1), area.height),
                        area.clip,
                    );
                    self.paint_node(mount, content, pane, buffer, scene, pass)?;
                    let divider = Placement::new(
                        CellRect::new(area.x + i64::from(inline) - 1, area.y, 1, area.height),
                        area.clip,
                    )
                    .visible();
                    for y in divider.y..divider.bottom() {
                        buffer.set_stringn(
                            divider.x,
                            y,
                            if overlay.ascii { "|" } else { "│" },
                            1,
                            Style::default().fg(color(overlay.border_color)),
                        );
                    }
                }
                let body = Placement::new(
                    CellRect::new(
                        area.x + i64::from(inline),
                        area.y,
                        area.width.saturating_sub(inline),
                        area.height,
                    ),
                    area.clip,
                );
                self.paint_node(mount, background, body, buffer, scene, pass)?;
                if let Some(opening) = opening {
                    scene.dialogs.push_back(super::dialog::Deferred {
                        mount: mount.clone(),
                        key: key.clone(),
                        opening: std::rc::Rc::downgrade(opening),
                        parent_path: scene.input_path.clone(),
                        anchor: None,
                    });
                }
            }
            Node::ContextMenu { .. } => {
                self.paint_context_menu(mount, node, area, buffer, scene, pass)?;
            }
            Node::Choice(choice) => {
                self.paint_choice(mount, choice, area, buffer, scene, pass.choices)?
            }
            Node::Dialog {
                key,
                background,
                open,
                ..
            } => {
                self.paint_node(mount, background, area, buffer, scene, pass)?;
                if *open {
                    let state = self.resources.owners[&mount.id()]
                        .state
                        .downcast_ref::<super::scroll::SceneState>()
                        .unwrap();
                    if let Some(opening) = state.dialogs.get(key) {
                        scene.dialogs.push_back(super::dialog::Deferred {
                            mount: mount.clone(),
                            key: key.clone(),
                            opening: std::rc::Rc::downgrade(opening),
                            parent_path: scene.input_path.clone(),
                            anchor: None,
                        });
                    }
                }
            }
            Node::Text {
                text: content,
                color: foreground,
                truncate,
                align,
                bold,
                animation,
                ..
            } => {
                let total: u128 = animation.iter().map(|frame| u128::from(frame.millis)).sum();
                let mut offset = if self.window_focused && total > 0 {
                    measurements
                        .cache
                        .time
                        .saturating_duration_since(self.animation_origin)
                        .as_millis()
                        % total
                } else {
                    0
                };
                let content = animation
                    .iter()
                    .find_map(|frame| {
                        if offset < u128::from(frame.millis) {
                            Some(frame.text.as_str())
                        } else {
                            offset -= u128::from(frame.millis);
                            None
                        }
                    })
                    .unwrap_or(content);
                let mut style = Style::default().fg(color(*foreground));
                if *bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                let rows = if *truncate {
                    vec![truncated_line(content, area.width)]
                } else {
                    lines(content, area.width)
                };
                for (row, content) in rows.iter().enumerate() {
                    let remaining = area.width.saturating_sub(content.width() as u16);
                    let offset = match align {
                        maka_ui::scene::terminal::Align::Start => 0,
                        maka_ui::scene::terminal::Align::Center => remaining / 2,
                        maka_ui::scene::terminal::Align::End => remaining,
                    };
                    text(
                        content,
                        Placement::new(
                            CellRect::new(
                                area.x + i64::from(offset),
                                area.y + row as i64,
                                area.width.saturating_sub(offset),
                                1,
                            ),
                            area.clip,
                        ),
                        buffer,
                        style,
                    );
                }
            }
            Node::Button {
                key,
                label,
                align,
                hint,
                focus_child,
                copy_text,
                disabled,
                color: foreground,
                background,
                hover_background,
                bold,
            } => {
                // Original Surface buttons: a centered, elided label in a
                // four-cell-padded hit area, without punctuation as chrome.
                let label = truncated_line(label, area.width);
                let button_area =
                    Placement::new(CellRect::new(area.x, area.y, area.width, 1), area.clip);
                if button_area.is_empty() {
                    return Ok(());
                }
                let mut style = Style::default().fg(Color::Reset);
                if let Some(foreground) = foreground {
                    style = style.fg(color(*foreground));
                }
                if *bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if let Some(background) = background {
                    style = style.bg(color(*background));
                }
                if *disabled {
                    style = style.add_modifier(Modifier::DIM);
                    paint_button(&label, button_area, buffer, style, *align);
                    return Ok(());
                }
                let target = Target {
                    kind: match focus_child {
                        Some(child) => TargetKind::Focus(self.freeze_focus_child(mount, child)?),
                        None => TargetKind::Action {
                            copy_text: copy_text.as_deref().map(Into::into),
                        },
                    },
                    ancestors: scene.ancestry.clone(),
                    revision: self.application.frame(mount)?.revision,
                    area: button_area,
                    tab_index: 0,
                    autofocus: false,
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
                        scope: scene.input_scope.clone(),
                    },
                };
                if self.focus.as_ref().is_some_and(|old| old.same(&target)) {
                    style = style.add_modifier(Modifier::UNDERLINED);
                    if scene
                        .input_scope
                        .as_ref()
                        .and_then(|scope| scope.upgrade())
                        .is_some_and(|opening| opening.anchor.is_some())
                        && let Some(background) = hover_background
                    {
                        style = style.bg(color(*background));
                    }
                } else if self.hover.as_ref().is_some_and(|old| old.same(&target)) {
                    style = match hover_background {
                        Some(background) => style.bg(color(*background)),
                        None => style.add_modifier(Modifier::BOLD),
                    };
                }
                // Buttons are single-line, clipped rather than wrapped to hidden hit cells.
                paint_button(&label, target.area, buffer, style, *align);
                if !hint.is_empty() && self.hover.as_ref().is_some_and(|old| old.same(&target)) {
                    scene.tooltip = Some((
                        scene.input_scope.clone(),
                        button_area,
                        hint.clone(),
                        style.remove_modifier(Modifier::UNDERLINED),
                    ));
                }
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
                wide,
                gap,
                padding,
                border,
                background,
                children,
                shortcuts,
                focus_shortcuts,
                focus_group,
                auto_focus,
                wheel,
                viewport,
                ..
            } => {
                if let Some(background) = background {
                    buffer.set_style(area.visible(), Style::default().bg(color(*background)));
                }
                let target_start = scene.targets.len();
                let inner = if border.is_some() {
                    super::border::inner(area)
                } else {
                    area
                };
                let inner = inner.inner(ratatui::layout::Margin::new(*padding, *padding));
                let measurements = if *viewport {
                    measurements.within(Size {
                        width: inner.width,
                        height: inner.height.min(u32::from(u16::MAX)) as u16,
                    })
                } else {
                    measurements
                };
                let mut pass = NodePaint {
                    measurements,
                    viewports: pass.viewports,
                    choices: pass.choices,
                    panes: pass.panes,
                };
                let children = self.visible_children(mount, children)?;
                let wide = wide.as_ref().filter(|wide| inner.width >= wide.min_width);
                let layouts: Vec<_> = children
                    .iter()
                    .map(|child| {
                        self.node_layout(mount, child, measurements).map(|layout| {
                            wide.and_then(|wide| wide.children.get(child.key()))
                                .copied()
                                .unwrap_or(layout)
                        })
                    })
                    .collect::<maka_ui::Result<Vec<_>>>()?;
                let flow = super::layout::measured(
                    wide.map_or(*direction, |wide| wide.direction),
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
                let viewport = if scrolling {
                    let identities: Vec<_> = children
                        .iter()
                        .zip(&flow.children)
                        .map(|(child, rect)| (child.key().to_owned(), rect.y as u32, rect.height))
                        .collect();
                    let state = self.resources.owners[&mount.id()]
                        .state
                        .downcast_ref::<super::scroll::SceneState>()
                        .unwrap();
                    let viewport = state.candidate(
                        key,
                        *scroll,
                        wheel.as_ref(),
                        inner,
                        flow.size.1,
                        &identities,
                    );
                    pass.viewports.push((key.clone(), viewport.clone()));
                    let identity = TargetIdentity {
                        mount: mount.clone(),
                        key: key.clone(),
                        owner: self.resources.owners[&mount.id()].identity.clone(),
                        scope: scene.input_scope.clone(),
                    };
                    if !inner.is_empty() {
                        scene.targets.push(Target {
                            identity: identity.clone(),
                            revision: self.application.frame(mount)?.revision,
                            area: inner,
                            tab_index: -1,
                            autofocus: false,
                            kind: TargetKind::Scroll,
                            ancestors: scene.ancestry.clone(),
                        });
                    }
                    Some(viewport)
                } else {
                    None
                };
                let offset = viewport.as_ref().map_or(0, |viewport| viewport.offset);
                let focus_shortcuts = self.freeze_focus_shortcuts(mount, focus_shortcuts)?;
                let scoped = *focus_group || !shortcuts.is_empty() || !focus_shortcuts.is_empty();
                if !shortcuts.is_empty() && scene.input_scope.is_some() && scene.ancestry.is_empty()
                {
                    scene.keyboard = Some(std::rc::Rc::new(Ancestor {
                        identity: TargetIdentity {
                            mount: mount.clone(),
                            key: key.clone(),
                            owner: self.resources.owners[&mount.id()].identity.clone(),
                            scope: scene.input_scope.clone(),
                        },
                        revision: self.application.frame(mount)?.revision,
                        shortcuts: shortcuts.clone(),
                        focus_group: focus_group.then_some(*direction),
                        scroll: viewport.clone(),
                        focus_shortcuts: focus_shortcuts.clone(),
                    }));
                }
                if scrolling || scoped {
                    scene.ancestry.push(std::rc::Rc::new(Ancestor {
                        identity: TargetIdentity {
                            mount: mount.clone(),
                            key: key.clone(),
                            owner: self.resources.owners[&mount.id()].identity.clone(),
                            scope: scene.input_scope.clone(),
                        },
                        revision: self.application.frame(mount)?.revision,
                        shortcuts: shortcuts.clone(),
                        focus_group: focus_group.then_some(*direction),
                        scroll: viewport.clone(),
                        focus_shortcuts: focus_shortcuts.clone(),
                    }));
                }
                let reading_children = scene.scope.as_ref().is_some_and(|scope| {
                    scope
                        .root
                        .as_ref()
                        .is_some_and(|(id, root)| *id == mount.id() && root == key)
                });
                for (child, relative) in children.into_iter().zip(flow.children) {
                    let previous_child = if reading_children {
                        scene
                            .scope
                            .as_mut()
                            .unwrap()
                            .child
                            .replace(child.key().to_owned())
                    } else {
                        None
                    };
                    let area = Placement::new(
                        relative.translated(inner.x, inner.y - i64::from(offset)),
                        inner.visible(),
                    );
                    self.paint_node(mount, child, area, buffer, scene, &mut pass)?;
                    if reading_children {
                        scene.scope.as_mut().unwrap().child = previous_child;
                    }
                }
                if scrolling || scoped {
                    scene.ancestry.pop();
                }
                if *auto_focus
                    && self.resources.owners[&mount.id()]
                        .state
                        .downcast_ref::<super::scroll::SceneState>()
                        .unwrap()
                        .focus
                        .get(key)
                        .is_some_and(maka_ui::focus::FocusOnReveal::pending)
                {
                    let targets = &scene.targets[target_start..];
                    let candidate = targets
                        .iter()
                        .filter(|target| target.tab_index >= 0 && !target.area.is_empty())
                        .min_by_key(|target| (target.tab_index == 0, target.tab_index))
                        .or_else(|| {
                            targets.iter().find(|target| {
                                target.mount == *mount
                                    && target.key == *key
                                    && matches!(target.kind, TargetKind::Scroll)
                            })
                        });
                    if let Some(candidate) = candidate {
                        scene.focus_offers.push((
                            TargetIdentity {
                                mount: mount.clone(),
                                key: key.clone(),
                                owner: self.resources.owners[&mount.id()].identity.clone(),
                                scope: scene.input_scope.clone(),
                            },
                            candidate.clone(),
                        ));
                    }
                }
                if let Some(border) = border {
                    let focused = self.focus.as_ref().is_some_and(|focused| {
                        scene.targets[target_start..]
                            .iter()
                            .any(|target| target.same(focused))
                    });
                    let background = background.map(color).unwrap_or_else(|| {
                        let visible = area.visible();
                        buffer
                            .cell((visible.x, visible.y))
                            .map_or(Color::Reset, |cell| cell.bg)
                    });
                    super::border::paint(
                        buffer,
                        area,
                        {
                            let foreground = color(if focused {
                                border.focused_color
                            } else {
                                border.color
                            });
                            if border.breathing && self.window_focused {
                                let millis = measurements
                                    .time()
                                    .saturating_duration_since(self.animation_origin)
                                    .as_millis();
                                let phase = (millis % 2800) as f32 / 2800.0;
                                let breath = (1.0 - (phase * std::f32::consts::TAU).cos()) * 0.5;
                                super::source::reveal::blend(
                                    foreground,
                                    background,
                                    0.36 + 0.64 * breath,
                                )
                                .unwrap_or(foreground)
                            } else {
                                foreground
                            }
                        },
                        border.style,
                    );
                }
            }
        }
        Ok(())
    }
}
