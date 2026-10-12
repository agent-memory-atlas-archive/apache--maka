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

//! Border geometry and painting extracted from the original TUI boundary.
use super::{CellRect, Placement};
use maka_ui_runtime::scene::terminal::BorderStyle;
use ratatui::{
    buffer::Buffer,
    style::{Color, Style},
};

pub(super) fn inner(area: Placement) -> Placement {
    Placement::new(
        CellRect::new(
            area.x + 1,
            area.y + 1,
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        ),
        area.visible(),
    )
}

pub(super) fn paint(buffer: &mut Buffer, area: Placement, color: Color, style: BorderStyle) {
    let visible = area.visible();
    if visible.is_empty() {
        return;
    }
    let right = area.right() - 1;
    let bottom = area.bottom() - 1;
    let horizontal = [area.y, bottom]
        .iter()
        .any(|y| (i64::from(visible.top())..i64::from(visible.bottom())).contains(y));
    let vertical = [area.x, right]
        .iter()
        .any(|x| (i64::from(visible.left())..i64::from(visible.right())).contains(x));
    // A viewport may expose only the body. Never add a border at a clip edge.
    if !horizontal && !vertical {
        return;
    }
    let (corners, horizontal, vertical) = match style {
        BorderStyle::Rounded => (["╭", "╮", "╰", "╯"], "─", "│"),
        BorderStyle::Square => (["┌", "┐", "└", "┘"], "─", "│"),
        BorderStyle::Ascii => (["+"; 4], "-", "|"),
    };
    let style = Style::default().fg(color);
    for y in visible.top()..visible.bottom() {
        for x in visible.left()..visible.right() {
            let symbol = match (
                i64::from(y) == area.y,
                i64::from(y) == bottom,
                i64::from(x) == area.x,
                i64::from(x) == right,
            ) {
                (true, _, true, _) => corners[0],
                (true, _, _, true) => corners[1],
                (_, true, true, _) => corners[2],
                (_, true, _, true) => corners[3],
                (true, _, _, _) | (_, true, _, _) => horizontal,
                (_, _, true, _) | (_, _, _, true) => vertical,
                _ => continue,
            };
            buffer[(x, y)].set_symbol(symbol).set_style(style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;
    #[test]
    fn clipped_frames_keep_original_edges_and_leave_the_body_untouched() {
        let clip = Rect::new(0, 0, 6, 3);
        for (style, corner) in [
            (BorderStyle::Rounded, "╰"),
            (BorderStyle::Square, "└"),
            (BorderStyle::Ascii, "+"),
        ] {
            let mut buffer = Buffer::empty(clip);
            paint(
                &mut buffer,
                Placement::new(CellRect::new(1, -2, 7, 5), clip),
                Color::Red,
                style,
            );
            assert_eq!(buffer[(1, 2)].symbol(), corner);
            assert_eq!(buffer[(3, 0)].symbol(), " ");
            assert_eq!(buffer[(5, 1)].symbol(), " ");
            assert_eq!(buffer[(0, 2)].symbol(), " ");
        }
        for (width, height) in [(1, 1), (1, 3), (3, 1), (2, 2)] {
            let area = Placement::from(Rect::new(0, 0, width, height));
            assert!(inner(area).is_empty());
            let mut buffer = Buffer::empty(clip);
            paint(&mut buffer, area, Color::Red, BorderStyle::Rounded);
            assert_eq!(buffer[(0, 0)].symbol(), "╭");
        }
        let mut buffer = Buffer::empty(clip);
        paint(
            &mut buffer,
            Placement::new(CellRect::new(-1, -1, 8, 5), clip),
            Color::Red,
            BorderStyle::Rounded,
        );
        assert!(buffer.content.iter().all(|cell| cell.symbol() == " "));
    }
}
