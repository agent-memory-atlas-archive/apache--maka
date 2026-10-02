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

use super::Presenter;
use maka_plugin_core::runtime::Mount;
use maka_ui::{Direction, Node, Presentation};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Paragraph, Widget as _},
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
pub(super) fn text_size(text: &str, width: u16) -> (u16, u16) {
    let lines = lines(text, width);
    (
        lines
            .iter()
            .map(|s| s.width().min(usize::from(width)) as u16)
            .max()
            .unwrap_or(0),
        lines.len().min(usize::from(u16::MAX)) as u16,
    )
}
pub(super) fn text(text: &str, area: Rect, buffer: &mut Buffer, style: Style) {
    let lines = lines(text, area.width);
    let content = lines
        .into_iter()
        .take(usize::from(area.height))
        .collect::<Vec<_>>()
        .join("\n");
    Paragraph::new(content).style(style).render(area, buffer);
}
#[derive(Clone)]
pub(super) struct Target {
    pub mount: Mount,
    pub key: String,
    pub revision: u64,
    pub area: Rect,
}
impl Target {
    pub fn same(&self, other: &Self) -> bool {
        self.mount == other.mount && self.key == other.key
    }
}
/// A pending scene is not input geometry until the embedding confirms a flush.
/// 绘制成功并完成实际 flush 后，才发布对应的命中区域。
pub struct PaintedScene {
    pub(super) targets: Vec<Target>,
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
        };
        self.paint_mount(&self.root, area, buffer, &mut scene)?;
        Ok(scene)
    }
    pub fn present(&mut self, scene: PaintedScene) {
        if self.closing {
            return;
        }
        self.focus = self
            .focus
            .take()
            .and_then(|old| scene.targets.iter().find(|new| new.same(&old)).cloned())
            .or_else(|| scene.targets.first().cloned());
        self.scene = scene.targets;
    }
    fn measure_mount(&self, mount: &Mount, width: u16) -> maka_ui::Result<(u16, u16)> {
        match &self.application.frame(mount)?.view {
            Presentation::Tree(node) => self.measure(mount, node, width),
            Presentation::Native(native) => Ok(native.view.measure(width)),
        }
    }
    fn measure(&self, mount: &Mount, node: &Node, width: u16) -> maka_ui::Result<(u16, u16)> {
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
            Node::Component { key, .. } => {
                return self.measure_mount(self.application.child_mount(mount, key)?, width);
            }
            Node::Stack {
                direction,
                gap,
                padding,
                children,
                ..
            } => {
                let inset = padding.saturating_mul(2);
                let available = width.saturating_sub(inset);
                let (mut w, mut h) = (0u16, 0u16);
                for (index, child) in children.iter().enumerate() {
                    let remaining = match direction {
                        Direction::Row => available.saturating_sub(w),
                        Direction::Column => available,
                    };
                    if matches!(direction, Direction::Row) && remaining == 0 {
                        break;
                    }
                    let size = self.measure(mount, child, remaining)?;
                    match direction {
                        Direction::Row => {
                            w = w.saturating_add(size.0);
                            h = h.max(size.1);
                        }
                        Direction::Column => {
                            w = w.max(size.0);
                            h = h.saturating_add(size.1);
                        }
                    }
                    if index + 1 < children.len() {
                        match direction {
                            Direction::Row => w = w.saturating_add(*gap).min(available),
                            Direction::Column => h = h.saturating_add(*gap),
                        }
                    }
                }
                (w.saturating_add(inset).min(width), h.saturating_add(inset))
            }
        })
    }
    fn paint_mount(
        &self,
        mount: &Mount,
        area: Rect,
        buffer: &mut Buffer,
        scene: &mut PaintedScene,
    ) -> maka_ui::Result<()> {
        match &self.application.frame(mount)?.view {
            Presentation::Tree(node) => self.paint_node(mount, node, area, buffer, scene),
            Presentation::Native(native) => {
                native.view.paint(area, buffer);
                Ok(())
            }
        }
    }
    fn paint_node(
        &self,
        mount: &Mount,
        node: &Node,
        area: Rect,
        buffer: &mut Buffer,
        scene: &mut PaintedScene,
    ) -> maka_ui::Result<()> {
        if area.width == 0 || area.height == 0 {
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
                    mount: mount.clone(),
                    key: key.clone(),
                    revision: self.application.frame(mount)?.revision,
                    area: Rect::new(area.x, area.y, area.width, 1),
                };
                let mut style = Style::default().fg(Color::Rgb(156, 194, 230));
                if self.focus.as_ref().is_some_and(|old| old.same(&target)) {
                    style = style.add_modifier(Modifier::REVERSED);
                }
                // Buttons are single-line, clipped rather than wrapped to hidden hit cells.
                Paragraph::new(label)
                    .style(style)
                    .render(target.area, buffer);
                scene.targets.push(target);
            }
            Node::Component { key, .. } => self.paint_mount(
                self.application.child_mount(mount, key)?,
                area,
                buffer,
                scene,
            )?,
            Node::Stack {
                direction,
                gap,
                padding,
                background,
                children,
                ..
            } => {
                buffer.set_style(area, Style::default().bg(color(*background)));
                let px = (*padding).min(area.width / 2);
                let py = (*padding).min(area.height / 2);
                let inner = Rect::new(
                    area.x + px,
                    area.y + py,
                    area.width - px * 2,
                    area.height - py * 2,
                );
                let mut offset = 0u16;
                for child in children {
                    let width = match direction {
                        Direction::Row => inner.width.saturating_sub(offset),
                        Direction::Column => inner.width,
                    };
                    let size = self.measure(mount, child, width)?;
                    let child_area = match direction {
                        Direction::Row => {
                            Rect::new(inner.x + offset, inner.y, size.0.min(width), inner.height)
                        }
                        Direction::Column => Rect::new(
                            inner.x,
                            inner.y + offset,
                            inner.width,
                            size.1.min(inner.height.saturating_sub(offset)),
                        ),
                    };
                    self.paint_node(mount, child, child_area.intersection(inner), buffer, scene)?;
                    offset = offset
                        .saturating_add(match direction {
                            Direction::Row => size.0,
                            Direction::Column => size.1,
                        })
                        .saturating_add(*gap);
                    if offset
                        >= match direction {
                            Direction::Row => inner.width,
                            Direction::Column => inner.height,
                        }
                    {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}
