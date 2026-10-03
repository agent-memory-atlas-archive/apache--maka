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

//! Read-only plain-text accessibility data from the current native prepaint.
use super::{InputBaseState, InputModeKind, RopeExt as _, layout::LastLayout};
use gpui::{Bounds, Pixels, TextAlign, point, px, size};
use std::{cell::RefCell, ops::Range, rc::Rc};
use unicode_segmentation::UnicodeSegmentation;

/// A read-only view of the original input's current prepaint. It owns no text,
/// selection, layout, or mutation authority.
#[derive(Clone, Default)]
pub struct InputTextAccessibility(Rc<RefCell<Option<Rc<InputTextAccessibilitySnapshot>>>>);
impl InputTextAccessibility {
    /// None until measured, inactive, masked, or using atomic inline tokens.
    pub fn snapshot(&self) -> Option<Rc<InputTextAccessibilitySnapshot>> {
        self.0.borrow().clone()
    }
    pub(super) fn publish(&self, snapshot: Option<InputTextAccessibilitySnapshot>) {
        *self.0.borrow_mut() = snapshot.map(Rc::new);
    }
}
/// Immutable derived data. Byte offsets refer to the original native document.
pub struct InputTextAccessibilitySnapshot {
    pub revision: u64,
    /// The scale of this exact native prepaint, for platform accessibility coordinates.
    pub scale_factor: f32,
    pub anchor: usize,
    pub focus: usize,
    pub focus_line_end_affinity: bool,
    pub runs: Vec<InputTextAccessibilityRun>,
}
/// One bounded fragment of an actual visual line, with native selectable units.
pub struct InputTextAccessibilityRun {
    pub range: Range<usize>,
    pub value: String,
    pub character_lengths: Vec<u8>,
    pub word_starts: Vec<u8>,
    pub visual_line: usize,
    /// Geometry exists only for native shaped rows in the current viewport.
    pub geometry: Option<InputTextAccessibilityGeometry>,
}
pub struct InputTextAccessibilityGeometry {
    pub bounds: Bounds<Pixels>,
    pub character_positions: Vec<f32>,
    pub character_widths: Vec<f32>,
}

pub(super) fn snapshot<M: InputModeKind>(
    state: &InputBaseState<M>,
    layout: &LastLayout,
    bounds: Bounds<Pixels>,
    viewport: Bounds<Pixels>,
    scale_factor: f32,
) -> Option<InputTextAccessibilitySnapshot> {
    // A scalar projection would misrepresent masked text and atomic tokens.
    if state.masked
        || state.inline_tokens.is_some()
        || !state.display_map.folded_ranges().is_empty()
    {
        return None;
    }
    let text = state.text.to_string();
    let native_word_starts: Vec<_> = text
        .split_word_bound_indices()
        .filter(|(_, word)| !word.trim_start().is_empty())
        .map(|(byte, _)| byte)
        .collect();
    let selection = state.active_selection();
    let (anchor, focus) = if selection.reversed {
        (selection.end, selection.start)
    } else {
        (selection.start, selection.end)
    };
    let mut runs = Vec::new();
    for row in 0..state.display_map.buffer_line_count() {
        let line_start = state.text.line_start_offset(row);
        let line_end = if row + 1 < state.display_map.buffer_line_count() {
            state.text.line_start_offset(row + 1)
        } else {
            text.len()
        };
        let item = state.display_map.line(row)?;
        for (subrow, wrapped) in item.wrapped_lines.iter().enumerate() {
            let start = line_start + wrapped.start;
            // Attach the real hard newline to the last visual row. Do not
            // publish a CRLF midpoint as a selectable or reflow boundary.
            if state.cursor_boundary(start, sum_tree::Bias::Left) != start {
                continue;
            }
            let mut end = if subrow + 1 == item.wrapped_lines.len() {
                line_end
            } else {
                line_start + wrapped.end
            };
            if state.cursor_boundary(end, sum_tree::Bias::Left) != end {
                end = state.next_boundary(end.saturating_sub(1));
            }
            let visual_line = state.display_map.buffer_line_to_display_row(row) + subrow;
            let mut fragment = start;
            loop {
                let mut offset = fragment;
                let mut lengths = Vec::new();
                let mut word_starts = Vec::new();
                while offset < end && lengths.len() < u8::MAX as usize {
                    if native_word_starts.binary_search(&offset).is_ok() {
                        word_starts.push(lengths.len() as u8);
                    }
                    let next = state.next_boundary(offset).min(end);
                    lengths.push((next - offset) as u8);
                    offset = next;
                }
                let range = fragment..offset;
                let geometry = layout.line(row).and_then(|line| {
                    // Native alignment differs per wrapped row; never infer another alignment
                    // from the logical-line caret helper. The facade uses Left.
                    if layout.text_align != TextAlign::Left {
                        return None;
                    }
                    let row_y =
                        layout.line_height * state.display_map.buffer_line_to_display_row(row);
                    let native_position = |byte, affinity| {
                        line.position_for_index(byte - line_start, layout, affinity)
                            .map(|p| bounds.origin + point(layout.line_number_width, row_y) + p)
                    };
                    let origin = native_position(fragment, false)?;
                    let mut positions = Vec::new();
                    let mut widths = Vec::new();
                    let mut byte = fragment;
                    for length in &lengths {
                        let start = native_position(byte, false)?;
                        let next = byte + *length as usize;
                        let end = if text[byte..next].ends_with('\n') {
                            start + point(layout.space_width, px(0.))
                        } else {
                            native_position(next, true)?
                        };
                        if start.y != origin.y || end.y != origin.y {
                            return None;
                        }
                        positions.push(f32::from(start.x - origin.x));
                        if end.x < start.x {
                            return None;
                        }
                        widths.push(f32::from(end.x - start.x));
                        byte = next;
                    }
                    let width = positions
                        .last()
                        .zip(widths.last())
                        .map_or(0., |(x, w)| x + w);
                    let rect = Bounds::new(origin, size(px(width), layout.line_height));
                    if rect.bottom() <= viewport.top() || rect.top() >= viewport.bottom() {
                        return None;
                    }
                    Some(InputTextAccessibilityGeometry {
                        bounds: rect,
                        character_positions: positions,
                        character_widths: widths,
                    })
                });
                runs.push(InputTextAccessibilityRun {
                    range,
                    value: text[fragment..offset].to_owned(),
                    character_lengths: lengths,
                    word_starts,
                    visual_line,
                    geometry,
                });
                if offset == end {
                    break;
                }
                fragment = offset;
            }
        }
    }
    Some(InputTextAccessibilitySnapshot {
        revision: state.document_revision(),
        scale_factor,
        anchor,
        focus,
        focus_line_end_affinity: state.cursor_line_end_affinity,
        runs,
    })
}
