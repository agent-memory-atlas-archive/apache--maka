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

use super::TextareaOwner;
use gpui_kit::base::input::{InputBase, InputTextAccessibility, InputTextAccessibilitySnapshot};
use gpui_kit::{
    AccessibleAction, EntityInputHandler, Role, StatefulInteractiveElement, WeakEntity, accesskit,
};
use std::{cell::RefCell, rc::Rc};

struct Published {
    snapshot: Rc<InputTextAccessibilitySnapshot>,
    ids: Vec<accesskit::NodeId>,
}
impl Published {
    fn byte(&self, position: accesskit::TextPosition) -> Option<usize> {
        let index = self.ids.iter().position(|id| *id == position.node)?;
        let run = &self.snapshot.runs[index];
        if position.character_index > run.character_lengths.len() {
            return None;
        }
        Some(
            run.range.start
                + run.character_lengths[..position.character_index]
                    .iter()
                    .map(|length| *length as usize)
                    .sum::<usize>(),
        )
    }
    fn line_end(&self, position: accesskit::TextPosition) -> bool {
        self.ids
            .iter()
            .position(|id| *id == position.node)
            .is_some_and(|index| {
                let run = &self.snapshot.runs[index];
                position.character_index == run.character_lengths.len()
                    && !run.value.ends_with('\n')
                    && self.snapshot.runs.get(index + 1).is_some_and(|next| {
                        next.range.start == run.range.end && next.visual_line != run.visual_line
                    })
            })
    }
    fn position(&self, byte: usize, line_end_affinity: bool) -> Option<accesskit::TextPosition> {
        let at_line_end = if line_end_affinity {
            self.snapshot
                .runs
                .iter()
                .enumerate()
                .find_map(|(index, run)| {
                    (run.range.end == byte
                        && self.line_end(accesskit::TextPosition {
                            node: self.ids[index],
                            character_index: run.character_lengths.len(),
                        }))
                    .then_some(index)
                })
        } else {
            None
        };
        let index = at_line_end.or_else(|| {
            self.snapshot
                .runs
                .iter()
                .position(|run| {
                    run.range.start <= byte && (byte < run.range.end || run.range.is_empty())
                })
                .or_else(|| self.snapshot.runs.len().checked_sub(1))
        })?;
        let run = &self.snapshot.runs[index];
        let mut offset = run.range.start;
        for character_index in 0..=run.character_lengths.len() {
            if offset == byte {
                return Some(accesskit::TextPosition {
                    node: self.ids[index],
                    character_index,
                });
            }
            if let Some(length) = run.character_lengths.get(character_index) {
                offset += *length as usize;
            }
        }
        None
    }
}

pub(super) fn attach(
    input: InputBase,
    native: InputTextAccessibility,
    owner: WeakEntity<TextareaOwner>,
) -> InputBase {
    // This projection is shared only by the same painted node and its action
    // listener. It grants no buffer, selection, layout or lifecycle authority.
    let published: Rc<RefCell<Option<Published>>> = Rc::new(RefCell::new(None));
    let action_projection = published.clone();
    input
        .a11y_synthetic_children(move |builder| {
            let Some(snapshot) = native.snapshot() else {
                *published.borrow_mut() = None;
                return;
            };
            let ids: Vec<_> = snapshot
                .runs
                .iter()
                .map(|run| {
                    // Both edits and pure reflow must not rebind an old run id to a
                    // different native byte range. The builder also hashes the parent.
                    builder.synthetic_node_id((snapshot.revision, run.range.start, run.range.end))
                })
                .collect();
            for (index, run) in snapshot.runs.iter().enumerate() {
                let mut node = accesskit::Node::new(Role::TextRun);
                node.set_value(run.value.clone());
                node.set_character_lengths(run.character_lengths.clone());
                node.set_word_starts(run.word_starts.clone());
                if index > 0 && snapshot.runs[index - 1].visual_line == run.visual_line {
                    node.set_previous_on_line(ids[index - 1]);
                }
                if index + 1 < ids.len() && snapshot.runs[index + 1].visual_line == run.visual_line
                {
                    node.set_next_on_line(ids[index + 1]);
                }
                if let Some(geometry) = &run.geometry {
                    let rect = geometry.bounds;
                    let scale = snapshot.scale_factor;
                    node.set_bounds(accesskit::Rect::new(
                        (f32::from(rect.left()) * scale) as f64,
                        (f32::from(rect.top()) * scale) as f64,
                        (f32::from(rect.right()) * scale) as f64,
                        (f32::from(rect.bottom()) * scale) as f64,
                    ));
                    node.set_text_direction(accesskit::TextDirection::LeftToRight);
                    node.set_character_positions(
                        geometry
                            .character_positions
                            .iter()
                            .map(|x| x * scale)
                            .collect::<Vec<_>>(),
                    );
                    node.set_character_widths(
                        geometry
                            .character_widths
                            .iter()
                            .map(|x| x * scale)
                            .collect::<Vec<_>>(),
                    );
                }
                builder.push_child(ids[index], node);
            }
            let projection = Published { snapshot, ids };
            if let (Some(anchor), Some(focus)) = (
                projection.position(
                    projection.snapshot.anchor,
                    projection.snapshot.anchor == projection.snapshot.focus
                        && projection.snapshot.focus_line_end_affinity,
                ),
                projection.position(
                    projection.snapshot.focus,
                    projection.snapshot.focus_line_end_affinity,
                ),
            ) {
                builder
                    .parent_node()
                    .set_text_selection(accesskit::TextSelection { anchor, focus });
            }
            *published.borrow_mut() = Some(projection);
        })
        .on_a11y_action(
            AccessibleAction::SetTextSelection,
            move |data, window, cx| {
                let Some(accesskit::ActionData::SetTextSelection(selection)) = data else {
                    return;
                };
                let projection = action_projection.borrow();
                let Some(projection) = projection.as_ref() else {
                    return;
                };
                let (Some(anchor), Some(focus)) = (
                    projection.byte(selection.anchor),
                    projection.byte(selection.focus),
                ) else {
                    return;
                };
                let _ = owner.update(cx, |owner, cx| {
                    if owner.input.current.borrow().is_none()
                        || owner.open_menu.is_some()
                        || owner.state.read(cx).document_revision() != projection.snapshot.revision
                    {
                        return;
                    }
                    let applied = owner.state.update(cx, |state, cx| {
                        if state.marked_text_range(window, cx).is_some() {
                            return false;
                        }
                        state.set_selection_anchor_focus(
                            anchor,
                            focus,
                            projection.line_end(selection.focus),
                            cx,
                        )
                    });
                    if applied {
                        owner.refresh(false, window, cx);
                    }
                });
            },
        )
}
