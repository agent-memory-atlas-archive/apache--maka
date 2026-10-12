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

//! Native range movement uses the original transcript's logical caret, sticky
//! column and visual-row rules. Committed participant geometry includes clipped
//! documents, so crossing the viewport never creates another selection owner.
use super::{DocumentRef, DocumentWeak, Endpoint, Layout, Participant, SelectionScope};
use crate::editor::layout::Layout as Literal;
use crossterm::event::KeyCode;
use std::rc::Rc;

fn layout(participant: &Participant) -> Option<Layout> {
    let document = participant.upgrade()?;
    let doc = document.borrow();
    if !doc.alive || doc.disabled {
        return None;
    }
    let width = participant
        .geometry
        .map(|(area, _)| area)
        .or_else(|| doc.glyphs.as_ref().map(|g| g.area))?
        .width;
    if width == 0 {
        return None;
    }
    if let Some(glyphs) = &doc.glyphs
        && glyphs.source == doc.source
        && glyphs.area.width == width
    {
        return Some(glyphs.layout.clone());
    }
    match &doc.markdown {
        Some(markdown) => Layout::markdown(markdown, width).ok(),
        None => Some(Literal::literal(&doc.source, width).into()),
    }
}
impl SelectionScope {
    /// Start at the selected reading child's first visible text row, using only
    /// the original documents and successful-flush geometry. Headers have no text owner.
    pub(in super::super) fn extend_child(&self, child: &str, key: KeyCode) -> bool {
        let scope = self.0.borrow();
        if !scope.enabled {
            return false;
        }
        let candidates: Vec<_> = scope
            .documents
            .iter()
            .filter(|participant| {
                participant.child.as_deref() == Some(child)
                    && participant.upgrade().is_some_and(|document| {
                        let doc = document.borrow();
                        doc.alive
                            && !doc.disabled
                            && !doc.source.is_empty()
                            && doc.binding.ptr_eq(&Rc::downgrade(&self.0))
                            && participant
                                .geometry
                                .is_some_and(|(_, revision)| revision == doc.geometry_revision)
                    })
            })
            .collect();
        let Some(participant) = candidates
            .iter()
            .copied()
            .find(|p| p.geometry.is_some_and(|(area, _)| !area.is_empty()))
            .or_else(|| candidates.first().copied())
        else {
            return false;
        };
        let Some(document) = participant.upgrade() else {
            return false;
        };
        let initial = if scope.selection.is_none() {
            let Some(layout) = layout(participant) else {
                return false;
            };
            let (area, _) = participant.geometry.unwrap();
            let row = if area.is_empty() {
                0
            } else {
                (i64::from(area.visible().y) - area.y).max(0) as usize
            };
            Some(Endpoint {
                document: participant.document.clone(),
                byte: layout.row_start(row),
                trailing: false,
            })
        } else {
            None
        };
        drop(scope);
        let created = initial.is_some();
        if let Some(cursor) = initial {
            self.0.borrow_mut().selection = Some((cursor.clone(), cursor));
        }
        let accepted = self.extend_keyboard(&document, key);
        if !accepted && created {
            self.0.borrow_mut().selection = None;
        }
        accepted
    }

    pub(super) fn extend_keyboard(&self, document: &DocumentRef, key: KeyCode) -> bool {
        if !matches!(
            key,
            KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::PageUp
                | KeyCode::PageDown
        ) {
            return false;
        }
        let scope = self.0.borrow();
        let (anchor, mut cursor) = scope.selection.clone().unwrap_or_else(|| {
            let cursor = Endpoint {
                document: Rc::downgrade(document),
                byte: 0,
                trailing: false,
            };
            (cursor.clone(), cursor)
        });
        let Some(mut at) = scope
            .documents
            .iter()
            .position(|p| p.ptr_eq(&cursor.document))
        else {
            return false;
        };
        let vertical = matches!(
            key,
            KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown
        );
        let forward = matches!(key, KeyCode::Right | KeyCode::Down | KeyCode::PageDown);
        let count = if matches!(key, KeyCode::PageUp | KeyCode::PageDown) {
            scope.area.map_or(1, |area| {
                usize::from(area.visible().height.saturating_sub(1).max(1))
            })
        } else {
            1
        };
        let mut column = if vertical { scope.column } else { None };
        // A page may visit many rows of the same document. Prepare each original
        // document at most once for this input, without retaining a second layout.
        let mut layouts: Vec<Option<Layout>> = vec![None; scope.documents.len()];
        for _ in 0..count {
            if layouts[at].is_none() {
                layouts[at] = layout(&scope.documents[at]);
            }
            let Some(current) = &layouts[at] else {
                return false;
            };
            let (row, x) = current.cursor(cursor.byte, cursor.trailing);
            if vertical {
                let x = *column.get_or_insert(x);
                let target = current.adjacent_row(row, forward);
                let target_row = if let Some(row) = target {
                    row
                } else {
                    let next = if forward {
                        at.checked_add(1).filter(|at| *at < scope.documents.len())
                    } else {
                        at.checked_sub(1)
                    };
                    let Some(next) = next else {
                        break;
                    };
                    at = next;
                    if layouts[at].is_none() {
                        layouts[at] = layout(&scope.documents[at]);
                    }
                    let Some(next) = &layouts[at] else {
                        return false;
                    };
                    next.edge_row(forward)
                };
                let current = layouts[at].as_ref().unwrap();
                cursor = Endpoint {
                    document: scope.documents[at].document.clone(),
                    byte: current.byte_at(target_row, x),
                    trailing: false,
                };
                cursor.trailing = current.cursor(cursor.byte, false).0 != target_row;
            } else {
                let Some(owner) = cursor.document.upgrade() else {
                    return false;
                };
                let len = owner.borrow().source.len();
                match key {
                    KeyCode::Left if cursor.byte == 0 && at > 0 => {
                        at -= 1;
                        let Some(previous) = scope.documents[at].upgrade() else {
                            return false;
                        };
                        cursor = Endpoint {
                            document: scope.documents[at].document.clone(),
                            byte: previous.borrow().source.len(),
                            trailing: true,
                        };
                    }
                    KeyCode::Right if cursor.byte == len && at + 1 < scope.documents.len() => {
                        at += 1;
                        cursor = Endpoint {
                            document: scope.documents[at].document.clone(),
                            byte: 0,
                            trailing: false,
                        };
                    }
                    KeyCode::Left => {
                        cursor.byte = current.previous(cursor.byte);
                        cursor.trailing = false;
                    }
                    KeyCode::Right => {
                        cursor.byte = current.next(cursor.byte);
                        cursor.trailing = false;
                    }
                    KeyCode::Home => {
                        cursor.byte = current.row_start(row);
                        cursor.trailing = false;
                    }
                    KeyCode::End => {
                        cursor.byte = current.row_end(row);
                        cursor.trailing = true;
                    }
                    _ => unreachable!(),
                }
            }
        }
        drop(scope);
        let mut scope = self.0.borrow_mut();
        scope.selection = Some((anchor, cursor));
        scope.column = column;
        scope.drag = None;
        scope.keyboard_focus_pending = true;
        scope.reveal = true;
        scope.revision += 1;
        scope.intent += 1;
        true
    }
    pub fn cancel_keyboard_reveal(&self) {
        let mut scope = self.0.borrow_mut();
        scope.keyboard_focus_pending = false;
        scope.reveal = false;
    }
    pub fn focus_revealed(&self) {
        self.0.borrow_mut().keyboard_focus_pending = false;
    }
    /// Used only while retaining keyboard focus within this original reading scope.
    pub fn keyboard_focus(&self) -> Option<DocumentWeak> {
        let scope = self.0.borrow();
        (scope.enabled && scope.keyboard_focus_pending)
            .then(|| scope.selection.as_ref().map(|(_, c)| c.document.clone()))
            .flatten()
    }
    /// Called after a successful, current-intent flush. Positions belong to that
    /// frame; failed or superseded painting never scrolls a new physical intent.
    pub fn take_reveal_row(&self) -> Option<i64> {
        let mut scope = self.0.borrow_mut();
        if !scope.enabled || !scope.reveal {
            return None;
        }
        let (_, cursor) = scope.selection.as_ref()?;
        let participant = scope
            .documents
            .iter()
            .find(|p| p.ptr_eq(&cursor.document))?;
        let (area, revision) = participant.geometry?;
        if participant.upgrade()?.borrow().geometry_revision != revision {
            return None;
        }
        let layout = layout(participant)?;
        let row = area.y + layout.cursor(cursor.byte, cursor.trailing).0 as i64;
        scope.reveal = false;
        Some(row)
    }
}
