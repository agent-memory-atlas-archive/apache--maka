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

//! Literal source, prepared Markdown and Diff share the original native reading owner.
use super::{NativeInput, Placement, Slots, Widget, WidgetInput};
use crate::editor::layout::Layout;
use crossterm::event::Event;
use maka_ui::{
    ComponentCall, ComponentOutput, Frame, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView, source::*,
};
use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier, Style},
};
use serde_json::json;
use std::{any::Any, collections::BTreeMap, rc::Rc, sync::Arc};
use unicode_width::UnicodeWidthStr;

use super::selection::{Document, DocumentRef, Glyphs, SelectionScope};
mod diff;
struct Geometry {
    glyphs: Glyphs,
    parent: Option<SelectionScope>,
}
pub(super) struct Owner {
    pub document: DocumentRef,
}
impl Owner {
    fn apply(&mut self, recipe: &Recipe) {
        let props = &recipe.0;
        let format_changed = match (&self.document.borrow().markdown, &recipe.1) {
            (Some(old), Some(new)) => !Arc::ptr_eq(old, new),
            (None, None) => false,
            _ => true,
        };
        let changed = {
            let doc = self.document.borrow();
            !props.source.starts_with(doc.source.as_ref()) || props.disabled || !recipe.2
        };
        if let Some(scope) = SelectionScope::active(&self.document) {
            if self.document.borrow().source.as_ref() != props.source
                || props.disabled
                || !recipe.2
                || format_changed
            {
                scope.release_pointer(&self.document);
            }
            if changed {
                scope.withdraw(&self.document);
            }
        }
        let mut doc = self.document.borrow_mut();
        if doc.source.as_ref() != props.source || format_changed {
            doc.geometry_revision += 1;
        }
        if changed || format_changed {
            doc.glyphs = None;
        }
        if doc.source.as_ref() != props.source {
            doc.source = Arc::from(props.source.as_str());
        }
        doc.disabled = props.disabled;
        doc.markdown = recipe.1.clone();
    }
}
struct Recipe(
    SourceTextProps,
    Option<Arc<crate::markdown::prepared::Document>>,
    bool,
);
fn color(rgba: u32) -> Color {
    Color::Rgb((rgba >> 24) as u8, (rgba >> 16) as u8, (rgba >> 8) as u8)
}
impl Widget for Recipe {
    fn create(&self, _: super::NativeContext) -> Box<dyn Any> {
        Box::new(Owner {
            document: Rc::new(std::cell::RefCell::new(Document {
                source: Arc::from(self.0.source.as_str()),
                geometry_revision: 0,
                markdown: self.1.clone(),
                disabled: self.0.disabled,
                alive: true,
                physical: false,
                glyphs: None,
                binding: std::rc::Weak::new(),
                standalone: None,
            })),
        })
    }
    fn commit(&self, state: &mut dyn Any) -> maka_ui::Result<Option<NativeInput>> {
        state.downcast_mut::<Owner>().unwrap().apply(self);
        Ok(None)
    }
    fn input(&self, state: &mut dyn Any, _: &str, event: &Event) -> maka_ui::Result<WidgetInput> {
        let owner = state.downcast_mut::<Owner>().unwrap();
        let enabled =
            self.2 && !self.0.disabled && self.0.source == owner.document.borrow().source.as_ref();
        let handled = enabled
            && SelectionScope::active(&owner.document)
                .is_some_and(|scope| scope.input(&owner.document, event));
        Ok(
            if handled && !self.0.focus_on_pointer && matches!(event, Event::Mouse(_)) {
                WidgetInput::PreserveFocus
            } else if handled {
                WidgetInput::Consumed(None)
            } else {
                WidgetInput::Unhandled
            },
        )
    }
    fn cancel_pointer(&self, state: &mut dyn Any) {
        let owner = state.downcast_mut::<Owner>().unwrap();
        if let Some(scope) = SelectionScope::active(&owner.document) {
            scope.release_pointer(&owner.document);
        }
    }
    fn pointer_capture(&self, state: &dyn Any) -> bool {
        let owner = state.downcast_ref::<Owner>().unwrap();
        let doc = owner.document.borrow();
        self.2
            && !self.0.disabled
            && doc.alive
            && doc.physical
            && self.0.source == doc.source.as_ref()
            && doc
                .glyphs
                .as_ref()
                .is_some_and(|glyphs| glyphs.source.as_ref() == doc.source.as_ref())
            && SelectionScope::active(&owner.document)
                .is_some_and(|scope| scope.pointer_capture(&owner.document))
    }
    fn copy(&self, state: &dyn Any) -> maka_ui::Result<Option<String>> {
        let owner = state.downcast_ref::<Owner>().unwrap();
        let doc = owner.document.borrow();
        if !self.2
            || self.0.disabled
            || doc.disabled
            || !doc.alive
            || !self.0.source.starts_with(doc.source.as_ref())
        {
            return Ok(None);
        }
        match SelectionScope::active(&owner.document) {
            Some(scope) => scope.copy(),
            None => Ok(None),
        }
    }
    fn presentation(&self, state: &mut dyn Any, visible: bool) {
        let owner = state.downcast_mut::<Owner>().unwrap();
        if !visible && let Some(scope) = SelectionScope::active(&owner.document) {
            scope.release_pointer(&owner.document);
        }
        let mut doc = owner.document.borrow_mut();
        doc.physical = visible;
        if !visible {
            doc.glyphs = None;
        }
    }
    fn deactivate(&self, state: &mut dyn Any) {
        let owner = state.downcast_mut::<Owner>().unwrap();
        if let Some(scope) = SelectionScope::active(&owner.document) {
            scope.withdraw(&owner.document);
        }
        let mut doc = owner.document.borrow_mut();
        doc.alive = false;
        doc.physical = false;
        doc.glyphs = None;
        doc.standalone = None;
    }
    fn measure(&self, _: &Slots<'_>, width: u16) -> maka_ui::Result<(u16, u32)> {
        if let Some(document) = &self.1 {
            let super::selection::Layout::Markdown { lines, .. } =
                super::selection::Layout::markdown(document, width)?
            else {
                unreachable!()
            };
            return Ok((
                lines
                    .iter()
                    .map(|line| line.line.width())
                    .max()
                    .unwrap_or(0)
                    .min(usize::from(width)) as u16,
                lines.len() as u32,
            ));
        }
        let layout = Layout::literal(&self.0.source, width);
        Ok((
            layout
                .rows
                .iter()
                .map(|row| row.cells.last().map_or(0, |cell| cell.column + cell.width))
                .max()
                .unwrap_or(0)
                .min(usize::from(width)) as u16,
            layout.rows.len() as u32,
        ))
    }
    fn paint(
        &self,
        slots: &mut Slots<'_>,
        area: Placement,
        buffer: &mut Buffer,
    ) -> maka_ui::Result<()> {
        if let Some(markdown) = &self.1 {
            return self.paint_markdown(markdown, slots, area, buffer);
        }
        let document = slots.native_state::<Owner>()?.document.clone();
        let parent = if self.2 && !self.0.disabled {
            slots.participant(&document, area)
        } else {
            None
        };
        let reveal = slots
            .scene
            .as_ref()
            .and_then(|scene| scene.scope.as_ref())
            .and_then(|scope| scope.reveal.as_ref())
            .filter(|reveal| reveal.targets(&document));
        if area.width == 0 || (area.is_empty() && reveal.is_none()) {
            return Ok(());
        }
        let doc = document.borrow();
        let scope = parent.clone().or_else(|| SelectionScope::active(&document));
        let layout = Layout::literal(&doc.source, area.width);
        if let Some(offset) = reveal.map(|reveal| reveal.offset) {
            let row = area.y + layout.cursor(offset, false).0 as i64;
            let visible = area.visible();
            let reveal = slots
                .scene
                .as_mut()
                .unwrap()
                .scope
                .as_mut()
                .unwrap()
                .reveal
                .as_mut()
                .unwrap();
            reveal.row = Some(row);
            reveal.visible = row >= i64::from(visible.y) && row < i64::from(visible.bottom());
        }
        if area.is_empty() {
            return Ok(());
        }
        let highlights = slots
            .scene
            .as_ref()
            .and_then(|scene| scene.scope.as_ref())
            .and_then(|scope| scope.find.as_ref())
            .map(|find| find.highlights(&document))
            .unwrap_or_default();
        let range = scope.as_ref().map_or(0..0, |scope| scope.range(&document));
        let mut normal = Style::default().fg(color(self.0.appearance.foreground.0));
        if self.0.appearance.font.weight >= 600 {
            normal = normal.add_modifier(Modifier::BOLD);
        }
        let selected = normal.bg(color(self.0.selection.0));
        let visible = area.visible();
        let first = (i64::from(visible.y) - area.y).max(0) as usize;
        for (row_index, row) in layout
            .rows
            .iter()
            .enumerate()
            .skip(first)
            .take(usize::from(visible.height))
        {
            for cell in &row.cells {
                let Some(point) = area.point(cell.column, row_index) else {
                    continue;
                };
                if area
                    .point(cell.column + cell.width - 1, row_index)
                    .is_none()
                {
                    continue;
                }
                let literal = &doc.source[cell.bytes.clone()];
                let style = if cell.bytes.start < range.end && cell.bytes.end > range.start {
                    selected
                } else if let Some((_, active)) = highlights
                    .get(highlights.partition_point(|(range, _)| range.end <= cell.bytes.start))
                    .filter(|(range, _)| range.start < cell.bytes.end)
                {
                    let style = normal.add_modifier(Modifier::UNDERLINED);
                    if *active {
                        style.add_modifier(Modifier::REVERSED | Modifier::BOLD)
                    } else {
                        style
                    }
                } else {
                    normal
                };
                if literal == "\t" {
                    for column in 0..cell.width {
                        buffer[(point.x + column as u16, point.y)]
                            .set_symbol(" ")
                            .set_style(style);
                    }
                } else {
                    let symbol =
                        if literal.chars().any(char::is_control) || literal.width() > cell.width {
                            "�".to_owned()
                        } else if literal.width() == 0 {
                            format!("◌{literal}")
                        } else {
                            literal.to_owned()
                        };
                    buffer.set_stringn(point.x, point.y, symbol, cell.width, style);
                }
            }
            let break_end = layout
                .rows
                .get(row_index + 1)
                .map_or(row.end, |next| next.start);
            if row.end < range.end && break_end > range.start && break_end > row.end {
                let column = row.cells.last().map_or(0, |cell| cell.column + cell.width);
                if let Some(point) = area.point(
                    column.min(usize::from(area.width.saturating_sub(1))),
                    row_index,
                ) {
                    buffer[(point.x, point.y)].set_style(selected);
                }
            }
        }
        if slots.focused() && range.is_empty() {
            let (byte, trailing) = scope
                .as_ref()
                .map_or((0, false), |scope| scope.cursor(&document));
            let (row, column) = layout.cursor(byte, trailing);
            if let Some(point) = area.point(column, row) {
                buffer[(point.x, point.y)].set_style(normal.add_modifier(Modifier::UNDERLINED));
            }
        }
        let geometry = Geometry {
            glyphs: Glyphs {
                source: doc.source.clone(),
                layout: layout.into(),
                area,
            },
            parent,
        };
        slots.native_geometry(Box::new(geometry))?;
        if self.2 && !self.0.disabled {
            slots.native_target("source", area, 0, false)?;
        }
        Ok(())
    }
    fn present(&self, state: &mut dyn Any, geometry: Box<dyn Any>) -> Option<NativeInput> {
        let owner = state.downcast_mut::<Owner>().unwrap();
        let geometry = *geometry.downcast::<Geometry>().unwrap();
        if self.2
            && !owner.document.borrow().disabled
            && owner
                .document
                .borrow()
                .source
                .starts_with(geometry.glyphs.source.as_ref())
        {
            let area = geometry.glyphs.area;
            owner.document.borrow_mut().glyphs = Some(geometry.glyphs);
            if geometry.parent.is_none() {
                SelectionScope::standalone(&owner.document, area);
            }
        }
        None
    }
}
impl Recipe {
    fn paint_markdown(
        &self,
        markdown: &crate::markdown::prepared::Document,
        slots: &mut Slots<'_>,
        area: Placement,
        buffer: &mut Buffer,
    ) -> maka_ui::Result<()> {
        use unicode_segmentation::UnicodeSegmentation;
        let document = slots.native_state::<Owner>()?.document.clone();
        let parent = if self.2 && !self.0.disabled {
            slots.participant(&document, area)
        } else {
            None
        };
        let reveal = slots
            .scene
            .as_ref()
            .and_then(|scene| scene.scope.as_ref())
            .and_then(|scope| scope.reveal.as_ref())
            .filter(|reveal| reveal.targets(&document))
            .map(|reveal| reveal.offset);
        if area.width == 0 || area.is_empty() && reveal.is_none() {
            return Ok(());
        }
        let layout = super::selection::Layout::markdown(markdown, area.width)?;
        if let Some(offset) = reveal {
            let row = area.y + layout.cursor(offset, false).0 as i64;
            let visible = area.visible();
            let reveal = slots
                .scene
                .as_mut()
                .unwrap()
                .scope
                .as_mut()
                .unwrap()
                .reveal
                .as_mut()
                .unwrap();
            reveal.row = Some(row);
            reveal.visible = row >= i64::from(visible.y) && row < i64::from(visible.bottom());
        }
        if area.is_empty() {
            return Ok(());
        }
        let highlights = slots
            .scene
            .as_ref()
            .and_then(|scene| scene.scope.as_ref())
            .and_then(|scope| scope.find.as_ref())
            .map(|find| find.highlights(&document))
            .unwrap_or_default();
        let scope = parent.clone().or_else(|| SelectionScope::active(&document));
        let range = scope.as_ref().map_or(0..0, |scope| scope.range(&document));
        let mut normal = Style::default().fg(color(self.0.appearance.foreground.0));
        if self.0.appearance.font.weight >= 600 {
            normal = normal.add_modifier(Modifier::BOLD);
        }
        let super::selection::Layout::Markdown { lines, .. } = &layout else {
            unreachable!()
        };
        let visible = area.visible();
        let first = (i64::from(visible.y) - area.y).max(0) as usize;
        for (row, visual) in lines
            .iter()
            .enumerate()
            .skip(first)
            .take(usize::from(visible.height))
        {
            let text = visual.line.to_string();
            let mut column = 0;
            let mut styles = Vec::with_capacity(visual.line.spans.len());
            let mut start = 0;
            for span in &visual.line.spans {
                styles.push((start..start + span.content.len(), span.style));
                start += span.content.len();
            }
            for (byte, glyph) in text.grapheme_indices(true) {
                let width = glyph.width().max(1);
                let mut logical: Option<std::ops::Range<usize>> = None;
                for span in &visual.mapping {
                    let start = byte.max(span.display.start);
                    let end = (byte + glyph.len()).min(span.display.end);
                    if start >= end {
                        continue;
                    }
                    let part = if span.display.len() == span.logical.len() {
                        span.logical.start + start - span.display.start
                            ..span.logical.start + end - span.display.start
                    } else {
                        span.logical.clone()
                    };
                    logical = Some(logical.map_or(part.clone(), |old| {
                        old.start.min(part.start)..old.end.max(part.end)
                    }));
                }
                let mut style = normal.patch(visual.line.style);
                for (span, part) in &styles {
                    if span.start < byte + glyph.len() && span.end > byte {
                        style = style.patch(*part);
                    }
                }
                if let Some(logical) = logical {
                    if logical.start < range.end && logical.end > range.start {
                        style = style.bg(color(self.0.selection.0));
                    } else if let Some((_, active)) = highlights
                        .get(highlights.partition_point(|(hit, _)| hit.end <= logical.start))
                        .filter(|(hit, _)| hit.start < logical.end)
                    {
                        style = style.add_modifier(Modifier::UNDERLINED);
                        if *active {
                            style = style.add_modifier(Modifier::REVERSED | Modifier::BOLD);
                        }
                    }
                }
                if let Some(point) = area.point(column, row)
                    && area.point(column + width - 1, row).is_some()
                {
                    if glyph.width() == 0 {
                        buffer.set_stringn(point.x, point.y, format!("◌{glyph}"), width, style);
                    } else {
                        buffer.set_stringn(point.x, point.y, glyph, width, style);
                    }
                }
                column += width;
            }
        }
        if slots.focused() && range.is_empty() {
            let (byte, trailing) = scope
                .as_ref()
                .map_or((0, false), |scope| scope.cursor(&document));
            let (row, column) = layout.cursor(byte, trailing);
            if let Some(point) = area.point(column, row) {
                buffer[(point.x, point.y)].set_style(normal.add_modifier(Modifier::UNDERLINED));
            }
        }
        slots.native_geometry(Box::new(Geometry {
            glyphs: Glyphs {
                source: document.borrow().source.clone(),
                layout,
                area,
            },
            parent,
        }))?;
        if self.2 && !self.0.disabled {
            slots.native_target("source", area, 0, false)?;
        }
        Ok(())
    }
}
fn markdown_palette(a: &maka_ui::markdown::MarkdownAppearance) -> crate::markdown::Palette {
    let mut colors = crate::markdown::Palette::default();
    if !a.dark {
        colors.syntax = [
            0x804caf, 0x246c48, 0x5e6c82, 0x985812, 0x205fc1, 0x147078, 0x4c5c72,
        ]
        .map(|rgb| color((rgb << 8) | 255));
    }
    colors.foreground = color(a.code.foreground.0);
    colors.background = color(a.code_background.0);
    colors.surface = colors.background;
    colors.accent = color(a.link.0);
    colors.muted = color(a.muted.0);
    colors.subtle = colors.muted;
    colors.border = color(a.border.0);
    colors.warning = color(a.code.foreground.0);
    colors.headings = a.headings.clone().map(|heading| {
        let style = Style::default().fg(color(heading.foreground.0));
        if heading.font.weight >= 600 {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        }
    });
    colors
}
pub(super) fn compile_markdown(
    props: &serde_json::Value,
) -> maka_ui::Result<NativeView<dyn Widget>> {
    let declaration = props.clone();
    let props: maka_ui::markdown::MarkdownProps = serde_json::from_value(props.clone())
        .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
    if props.source.len() > 65536 {
        return Err(maka_ui::Error::Invalid(
            "Markdown source byte budget".into(),
        ));
    }
    let document = Arc::new(
        crate::markdown::prepared::Document::markdown(
            &props.source,
            false,
            markdown_palette(&props.appearance),
        )
        .map_err(|e| maka_ui::Error::Invalid(e.into()))?,
    );
    if document.text().len() > 65536 {
        return Err(maka_ui::Error::Invalid(
            "Markdown readable text byte budget".into(),
        ));
    }
    let source = SourceTextProps {
        source: document.text().to_owned(),
        appearance: props.appearance.body,
        selection: props.appearance.selection,
        disabled: props.disabled,
        focus_on_pointer: props.focus_on_pointer,
    };
    NativeView::<dyn Widget>::new(
        Rc::new(Recipe(source, Some(document), props.selectable)),
        BTreeMap::new(),
        vec![],
    )?
    .with_declaration(declaration)
}

pub(super) fn compile_diff(props: &serde_json::Value) -> maka_ui::Result<NativeView<dyn Widget>> {
    use maka_ui::diff::DiffProps;
    let declaration = props.clone();
    let props: DiffProps = serde_json::from_value(props.clone())
        .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
    let (text, rows) = diff::prepare(&props.content)?;
    let a = &props.appearance;
    let mut colors = crate::markdown::Palette::default();
    if !a.dark {
        colors.syntax = [
            0x804caf, 0x246c48, 0x5e6c82, 0x985812, 0x205fc1, 0x147078, 0x4c5c72,
        ]
        .map(|rgb| color((rgb << 8) | 255));
    }
    colors.foreground = color(a.text.foreground.0);
    colors.background = color(a.background.0);
    colors.surface = colors.background;
    colors.success = color(a.added.0);
    colors.error = color(a.removed.0);
    let document = Arc::new(
        crate::markdown::prepared::Document::diff(&text, &rows, props.ascii, colors)
            .map_err(|e| maka_ui::Error::Invalid(e.into()))?,
    );
    if document.text().len() > 65536 {
        return Err(maka_ui::Error::Invalid(
            "Diff readable text byte budget".into(),
        ));
    }
    let source = SourceTextProps {
        source: document.text().to_owned(),
        appearance: props.appearance.text,
        selection: props.appearance.selection,
        disabled: props.disabled,
        focus_on_pointer: props.focus_on_pointer,
    };
    NativeView::<dyn Widget>::new(
        Rc::new(Recipe(source, Some(document), true)),
        BTreeMap::new(),
        vec![],
    )?
    .with_declaration(declaration)
}

type Compile = fn(&serde_json::Value) -> maka_ui::Result<NativeView<dyn Widget>>;
struct Component(Compile);
impl NativeComponent<dyn Widget> for Component {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let (props, revision) = match call {
            ComponentCall::Mount { props } => (props, 0),
            ComponentCall::Update { props, state } => {
                (props, state["revision"].as_u64().unwrap() + 1)
            }
            ComponentCall::Event { .. } => {
                return Err(maka_ui::Error::Invalid(
                    "SourceText has no component events".into(),
                ));
            }
        };
        Ok(Frame {
            revision,
            state: json!({"revision":revision}),
            surface: None,
            view: (self.0)(props)?,
        }
        .output(vec![]))
    }
}
pub(super) fn compile(props: &serde_json::Value) -> maka_ui::Result<NativeView<dyn Widget>> {
    let declaration = props.clone();
    let props: SourceTextProps = serde_json::from_value(props.clone())
        .map_err(|error| maka_ui::Error::Invalid(error.to_string()))?;
    if props.source.len() > 65536 {
        return Err(maka_ui::Error::Invalid("source byte budget".into()));
    }
    NativeView::<dyn Widget>::new(Rc::new(Recipe(props, None, true)), BTreeMap::new(), vec![])?
        .with_declaration(declaration)
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register("maka.ui.diff", |_, _| {
            Ok(BTreeMap::from([(
                "Diff".into(),
                NativeExport {
                    contract: maka_ui::diff::diff_contract(),
                    value: NativeCapability::Component(Rc::new(Component(compile_diff))),
                },
            )]))
        })
        .expect("unique Diff factory");
    registry
        .register("maka.ui.markdown", |_, _| {
            Ok(BTreeMap::from([(
                "Markdown".into(),
                NativeExport {
                    contract: maka_ui::markdown::markdown_contract(),
                    value: NativeCapability::Component(Rc::new(Component(compile_markdown))),
                },
            )]))
        })
        .expect("unique Markdown factory");
    registry
        .register("maka.ui.source", |_, _| {
            Ok(BTreeMap::from([(
                "SourceText".into(),
                NativeExport {
                    contract: source_contract(),
                    value: NativeCapability::Component(Rc::new(Component(compile))),
                },
            )]))
        })
        .expect("unique source implementation");
}

#[cfg(test)]
pub(super) fn painted_area(state: &dyn Any) -> Option<Placement> {
    state
        .downcast_ref::<Owner>()?
        .document
        .borrow()
        .glyphs
        .as_ref()
        .map(|glyphs| glyphs.area)
}
