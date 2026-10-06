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

//! Reading-position retention in the original scroll owner. Geometry is
//! acknowledged only when the container paints; retry targets are logical.

use super::*;

/// How a tracked container responds to changes in its children's layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollRetention {
    /// Preserve the absolute native offset, subject to the new scroll limits.
    #[default]
    None,
    /// Preserve the first surviving visible identified child's viewport top.
    Anchor,
    /// Follow the end while there, otherwise preserve the visible child.
    FollowEnd,
}

/// An explicit command for a tracked Div's original reading owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScrollCommand {
    /// Reveal the beginning of the container.
    Start,
    /// Reveal the end of the container.
    End,
    /// Reveal a currently committed direct child's stable ID.
    RevealChild(ElementId),
}

/// Owner-local command identity. It carries no authority across handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollRequestId(pub(super) u64);

/// A command settles only at actual paint, or when superseded by new intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollRequestStatus {
    /// Awaiting actual presentation.
    Pending,
    /// The resolved target was actually presented.
    Applied,
    /// A later explicit reading intent superseded the command.
    Cancelled,
    /// The target did not exist in the painted layout.
    Rejected,
}

/// Immutable facts from the last actually painted tracked Div viewport.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScrollObservation {
    /// Explicit reading intent used when laying out the displayed children.
    pub reading_revision: u64,
    /// Device-pixel snapped offset actually used for the children.
    pub offset: Point<Pixels>,
    /// Scroll limits of that same painted layout.
    pub max_offset: Point<Pixels>,
    /// Size of that same viewport.
    pub viewport: Size<Pixels>,
    /// Whether that same painted offset reaches the vertical end.
    pub at_end: bool,
    /// Identified direct children intersecting the vertical viewport.
    pub visible_child_keys: Vec<ElementId>,
}

#[derive(Debug)]
pub(crate) struct ScrollPresentation {
    pub layout: ScrollLayout,
    pub observation: ScrollObservation,
    pub request: Option<(ScrollRequestId, ScrollRequestStatus)>,
}

#[derive(Debug)]
pub(crate) struct ScrollLayout {
    pub max_y: Pixels,
    pub height: Pixels,
    pub children: Vec<(Option<ElementId>, Bounds<Pixels>)>,
}

#[derive(Debug)]
enum Target {
    Offset(Pixels),
    End,
    Child { id: ElementId, top: Pixels },
}

/// One unpainted attempt. A reading origin is interpreted only against the
/// immutable last-painted geometry; it is not another live offset. Repeated
/// candidates reselect surviving stable children from the same reading frame.
#[derive(Debug)]
pub(crate) struct Attempt {
    applied_y: Pixels,
    origin: Origin,
}
#[derive(Debug)]
enum Origin {
    Reading(Pixels),
    Explicit(Pixels),
}
impl ScrollHandleState {
    pub(super) fn note_reading_intent(&mut self) {
        self.reading_revision = self
            .reading_revision
            .checked_add(1)
            .expect("reading revision exhausted");
        if let Some(request) = self.scroll_request.take() {
            self.last_receipt = Some((request.id, ScrollRequestStatus::Cancelled));
        }
    }

    pub(super) fn explicit_scroll(&mut self, y: Pixels) {
        if let Some(attempt) = &mut self.retention_attempt {
            attempt.origin = Origin::Explicit(y);
            attempt.applied_y = y;
        }
    }

    pub(super) fn retain_position(
        &mut self,
        policy: ScrollRetention,
        next: &ScrollLayout,
        tolerance: Pixels,
    ) {
        let offset = self.offset.borrow().y;
        let attempt = self.retention_attempt.get_or_insert_with(|| Attempt {
            applied_y: offset,
            origin: Origin::Reading(offset),
        });
        if attempt.applied_y != offset {
            // Real input and focus reveal supersede the automatic plan until
            // paint. Repeated layout must preserve their absolute target.
            attempt.origin = Origin::Explicit(offset);
        }
        let y = match attempt.origin {
            Origin::Explicit(y) => y,
            Origin::Reading(y) => {
                match target(policy, self.painted_layout.as_ref(), next, y, tolerance) {
                    Target::Offset(y) => y,
                    Target::End => -next.max_y,
                    Target::Child { id, top } => next
                        .children
                        .iter()
                        .find(|(child, _)| child.as_ref() == Some(&id))
                        .map_or(y, |(_, bounds)| top - bounds.top()),
                }
            }
        }
        .clamp(-next.max_y, px(0.));
        self.offset.borrow_mut().y = y;
        attempt.applied_y = y;
    }
}

fn target(
    policy: ScrollRetention,
    previous: Option<&ScrollLayout>,
    next: &ScrollLayout,
    offset: Pixels,
    tolerance: Pixels,
) -> Target {
    if policy == ScrollRetention::None {
        return Target::Offset(offset);
    }
    let Some(previous) = previous else {
        return if policy == ScrollRetention::FollowEnd {
            Target::End
        } else {
            Target::Offset(offset)
        };
    };
    let offset = offset.clamp(-previous.max_y, px(0.));
    if policy == ScrollRetention::FollowEnd && (offset + previous.max_y).abs() <= tolerance {
        return Target::End;
    }
    let survivor = previous
        .children
        .iter()
        .filter_map(|(id, bounds)| {
            let id = id.as_ref()?;
            next.children
                .iter()
                .any(|(next, _)| next.as_ref() == Some(id))
                .then_some((id, bounds))
        })
        .min_by(|(_, a), (_, b)| {
            let rank = |bounds: &Bounds<Pixels>| {
                let top = bounds.top() + offset;
                let visible = bounds.bottom() + offset > px(0.) && top < previous.height;
                (!visible, if visible { top } else { top.abs() })
            };
            rank(a).partial_cmp(&rank(b)).unwrap_or(Ordering::Equal)
        });
    match survivor {
        Some((id, bounds)) => Target::Child {
            id: id.clone(),
            top: bounds.top() + offset,
        },
        None => Target::Offset(offset),
    }
}
