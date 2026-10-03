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
    area: Rect,
    top: usize,
}
impl Editor {
    pub(crate) fn mouse_presented(&mut self, mouse: MouseEvent, geometry: &Geometry) -> bool {
        if geometry.revision != self.revision {
            return false;
        }
        self.layout = geometry.layout.clone();
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
        area: Rect,
        buffer: &mut Buffer,
        focused: bool,
        normal: Style,
        selected: Style,
        caret: Style,
    ) -> Geometry {
        let layout = Layout::new(&self.text, area.width);
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
        if !area.is_empty() {
            let selection = self.selection.range();
            for (screen_row, row) in layout
                .rows
                .iter()
                .skip(top)
                .take(area.height as usize)
                .enumerate()
            {
                let y = area.y + screen_row as u16;
                for cell in &row.cells {
                    let x = area.x + cell.column;
                    let style =
                        if cell.bytes.start < selection.end && cell.bytes.end > selection.start {
                            selected
                        } else {
                            normal
                        };
                    let text = &self.text[cell.bytes.clone()];
                    if text == "\t" {
                        for column in 0..cell.width {
                            buffer[(x + column, y)].set_symbol(" ").set_style(style);
                        }
                    } else if text.width() > cell.width as usize {
                        buffer[(x, y)].set_symbol("�").set_style(style);
                    } else {
                        let symbol = if text.width() == 0 {
                            format!("◌{text}")
                        } else {
                            text.to_owned()
                        };
                        buffer.set_stringn(x, y, symbol, cell.width as usize, style);
                    }
                }
                if self.text.as_bytes().get(row.end) == Some(&b'\n') && selection.contains(&row.end)
                {
                    let column = row.cells.last().map_or(0, |cell| cell.column + cell.width);
                    buffer[(area.x + column.min(area.width - 1), y)].set_style(selected);
                }
            }
            if focused && cursor_row >= top && cursor_row < top + area.height as usize {
                buffer[(
                    area.x + cursor_column.min(area.width - 1),
                    area.y + (cursor_row - top) as u16,
                )]
                    .set_style(caret);
            }
        }
        Geometry {
            revision: self.revision,
            selection: self.selection,
            layout,
            area,
            top,
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn paint(editor: &Editor, area: Rect) -> (Geometry, Buffer) {
        let mut buffer = Buffer::empty(area);
        let geometry = editor.paint_prepared(
            area,
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
}
