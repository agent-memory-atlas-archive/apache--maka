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

use super::*;
use ratatui::buffer::Buffer;

/// Pending read-only geometry. No buffer/selection/history is copied or mutated.
#[derive(Clone)]
pub(crate) struct Geometry {
    revision: u64,
    selection: Selection,
    layout: Layout,
    area: Placement,
    top: usize,
    left: usize,
}
impl Editor {
    pub(crate) fn mouse_presented(&mut self, mouse: MouseEvent, geometry: &Geometry) -> bool {
        if geometry.revision != self.revision {
            return false;
        }
        self.layout = geometry.layout.clone();
        self.left = geometry.left;
        self.area = (!geometry.area.is_empty()).then_some(geometry.area);
        if !matches!(
            mouse.kind,
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
        ) {
            self.top = geometry.top;
        }
        self.mouse(mouse)
    }
    pub(crate) fn paint_prepared(
        &self,
        area: Placement,
        buffer: &mut Buffer,
        focused: bool,
        normal: Style,
        selected: Style,
        caret: Style,
    ) -> Geometry {
        let layout = self.make_layout(area.width);
        let mut left = self.left;
        let (cursor_row, cursor_column) =
            layout.cursor(self.selection.cursor, self.selection.upstream);
        let mut top = self
            .top
            .min(layout.rows.len().saturating_sub(area.height as usize));
        if self.area != Some(area) || self.layout.width != area.width {
            if cursor_row < top {
                top = cursor_row;
            }
            if cursor_row >= top + area.height as usize {
                top = cursor_row.saturating_sub(area.height.saturating_sub(1) as usize);
            }
        }
        if self.single_line {
            top = 0;
            left = left.min(
                layout.rows[0]
                    .cells
                    .last()
                    .map_or(0, |cell| cell.column + cell.width),
            );
            if cursor_column < left {
                left = cursor_column;
            }
            if cursor_column >= left + area.width as usize {
                left = cursor_column
                    .saturating_add(1)
                    .saturating_sub(area.width as usize);
            }
        } else {
            left = 0;
        }
        if !area.is_empty() {
            let selection = self.selection.range();
            for (screen_row, row) in layout
                .rows
                .iter()
                .skip(top)
                .take(area.height as usize)
                .enumerate()
            {
                let y = area.y + screen_row as i64;
                for cell in &row.cells {
                    if cell.column < left || cell.column + cell.width > left + area.width as usize {
                        continue;
                    }
                    let Some(point) = area.point(cell.column - left, screen_row) else {
                        continue;
                    };
                    if area
                        .point(
                            cell.column - left + cell.width.saturating_sub(1),
                            screen_row,
                        )
                        .is_none()
                    {
                        continue;
                    }
                    let (x, y) = (point.x, point.y);
                    let style =
                        if cell.bytes.start < selection.end && cell.bytes.end > selection.start {
                            selected
                        } else {
                            normal
                        };
                    let text = if self.single_line && self.masked {
                        "•"
                    } else {
                        &self.text[cell.bytes.clone()]
                    };
                    if text == "\t" {
                        for column in 0..cell.width {
                            buffer[(x + column as u16, y)]
                                .set_symbol(" ")
                                .set_style(style);
                        }
                    } else if text.width() > cell.width {
                        buffer[(x, y)].set_symbol("�").set_style(style);
                    } else {
                        let symbol = if text.width() == 0 {
                            format!("◌{text}")
                        } else {
                            text.to_owned()
                        };
                        buffer.set_stringn(x, y, symbol, cell.width, style);
                    }
                }
                if self.text.as_bytes().get(row.end) == Some(&b'\n') && selection.contains(&row.end)
                {
                    let column = row.cells.last().map_or(0, |cell| cell.column + cell.width);
                    if let Some(point) = area.point(
                        column.min(area.width.saturating_sub(1) as usize),
                        (y - area.y) as usize,
                    ) {
                        buffer[(point.x, point.y)].set_style(selected);
                    }
                }
            }
            if focused
                && cursor_row >= top
                && cursor_row < top + area.height as usize
                && let Some(point) = area.point(
                    cursor_column
                        .saturating_sub(left)
                        .min(area.width.saturating_sub(1) as usize),
                    cursor_row - top,
                )
            {
                buffer[(point.x, point.y)].set_style(caret);
            }
        }
        Geometry {
            revision: self.revision,
            selection: self.selection,
            layout,
            area,
            top,
            left,
        }
    }
    pub(crate) fn present_geometry(&mut self, geometry: Geometry) {
        // A successful flush cannot overwrite geometry after a newer native edit.
        if geometry.revision != self.revision || geometry.selection != self.selection {
            return;
        }
        self.layout = geometry.layout;
        self.area = (!geometry.area.is_empty()).then_some(geometry.area);
        self.top = geometry.top;
        self.left = geometry.left;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn paint(editor: &Editor, area: Rect) -> (Geometry, Buffer) {
        let mut buffer = Buffer::empty(area);
        let geometry = editor.paint_prepared(
            area.into(),
            &mut buffer,
            true,
            Style::default(),
            Style::default(),
            Style::default(),
        );
        (geometry, buffer)
    }
    #[test]
    fn wheel_viewport_survives_repaint_and_unrelated_selection_observation() {
        let mut editor = Editor::default();
        editor.insert(
            &(0..20)
                .map(|i| format!("row {i:02}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let area = Rect::new(0, 0, 20, 4);
        let (visible, _) = paint(&editor, area);
        editor.present_geometry(visible.clone());
        let wheel = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        assert!(editor.mouse_presented(wheel, &visible));
        assert!(editor.mouse_presented(wheel, &visible));
        let (pending, first) = paint(&editor, area);
        let line = (0..6).map(|x| first[(x, 0)].symbol()).collect::<String>();
        assert_eq!(line, "row 10");
        editor.present_geometry(pending);
        let (_, second) = paint(&editor, area);
        assert_eq!(first, second);
        // Native keyboard navigation requests reveal, unlike passive paint.
        editor.key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        let (_, after_key) = paint(&editor, area);
        assert_ne!(first, after_key);
    }
    #[test]
    fn masked_single_line_full_capacity_and_unicode_hit_use_original_bytes() {
        let mut editor = Editor::single_line(65536, true);
        editor.insert(&"x".repeat(65536));
        let area = Rect::new(3, 2, 20, 1);
        let (visible, buffer) = paint(&editor, area);
        assert_eq!(visible.layout.rows.len(), 1);
        assert_eq!(visible.layout.rows[0].cells.last().unwrap().column, 65535);
        assert_eq!(visible.left, 65517);
        assert!(!buffer.content().iter().any(|cell| cell.symbol() == "x"));
        editor.present_geometry(visible.clone());
        assert_eq!(editor.cursor_position(), Some(Position::new(22, 2)));
        assert!(editor.mouse_presented(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 3,
                row: 2,
                modifiers: KeyModifiers::NONE
            },
            &visible
        ));
        assert_eq!(editor.selection.cursor, 65517);
        let old = visible;
        editor.clear();
        editor.insert("A👩🏽‍💻e\u{301}中");
        assert!(!editor.mouse_presented(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 3,
                row: 2,
                modifiers: KeyModifiers::NONE
            },
            &old
        ));
        let (visible, _) = paint(&editor, area);
        editor.present_geometry(visible.clone());
        assert!(editor.mouse_presented(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 5,
                row: 2,
                modifiers: KeyModifiers::NONE
            },
            &visible
        ));
        assert_eq!(editor.selection.cursor, "A👩🏽‍💻".len());
        editor.key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT));
        assert_eq!(editor.selected_text(), "e\u{301}");
    }
    #[test]
    fn clipped_editor_paints_and_hits_original_unicode_row_without_rebasing_source() {
        let mut editor = Editor::default();
        editor.insert("00\n11\n中文😀\n44\n55");
        editor.selection.cursor = 0;
        let clip = Rect::new(0, 0, 6, 3);
        let area = Placement::new(crate::application::CellRect::new(0, -2, 12, 6), clip);
        let mut buffer = Buffer::empty(clip);
        let geometry = editor.paint_prepared(
            area,
            &mut buffer,
            false,
            Style::default(),
            Style::default(),
            Style::default(),
        );
        assert_eq!(buffer[(0, 0)].symbol(), "中");
        assert_eq!(buffer[(2, 0)].symbol(), "文");
        assert_eq!(buffer[(4, 0)].symbol(), "😀");
        assert!(editor.mouse_presented(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 4,
                row: 0,
                modifiers: KeyModifiers::NONE
            },
            &geometry
        ));
        assert_eq!(editor.selection.cursor, "00\n11\n中文".len());
    }
}
