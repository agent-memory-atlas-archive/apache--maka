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

use ratatui::layout::{Position, Rect};
use std::ops::Deref;

/// Logical terminal cells. Signed origins retain content above/left of the
/// viewport; full height never narrows to the physical terminal's u16 extent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellRect {
    pub x: i64,
    pub y: i64,
    pub width: u16,
    pub height: u32,
}
impl CellRect {
    pub fn new(x: i64, y: i64, width: u16, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn right(self) -> i64 {
        self.x + i64::from(self.width)
    }
    pub fn bottom(self) -> i64 {
        self.y + i64::from(self.height)
    }
    pub fn translated(self, x: i64, y: i64) -> Self {
        Self {
            x: self.x + x,
            y: self.y + y,
            ..self
        }
    }
}
/// One logical placement plus its inherited physical clip. Clipping never
/// changes wrapping width, source origin, full height or pointer byte mapping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Placement {
    pub bounds: CellRect,
    pub clip: Rect,
}
impl Deref for Placement {
    type Target = CellRect;
    fn deref(&self) -> &CellRect {
        &self.bounds
    }
}
impl From<Rect> for Placement {
    fn from(area: Rect) -> Self {
        Self::new(
            CellRect::new(
                i64::from(area.x),
                i64::from(area.y),
                area.width,
                u32::from(area.height),
            ),
            area,
        )
    }
}
impl Placement {
    pub fn new(bounds: CellRect, clip: Rect) -> Self {
        Self { bounds, clip }
    }
    pub fn visible(self) -> Rect {
        let x = self.x.max(i64::from(self.clip.x));
        let y = self.y.max(i64::from(self.clip.y));
        let right = self.right().min(i64::from(self.clip.right()));
        let bottom = self.bottom().min(i64::from(self.clip.bottom()));
        if right <= x || bottom <= y {
            return Rect::default();
        }
        Rect::new(x as u16, y as u16, (right - x) as u16, (bottom - y) as u16)
    }
    pub fn is_empty(self) -> bool {
        self.visible().is_empty()
    }
    pub fn contains(self, point: Position) -> bool {
        self.visible().contains(point)
    }
    pub fn clipped(self, clip: Rect) -> Self {
        Self {
            clip: self.clip.intersection(clip),
            ..self
        }
    }
    pub fn inner(self, margin: ratatui::layout::Margin) -> Self {
        let x = margin.horizontal.min(self.width / 2);
        let y = u32::from(margin.vertical).min(self.height / 2);
        Self::new(
            CellRect::new(
                self.x + i64::from(x),
                self.y + i64::from(y),
                self.width - x * 2,
                self.height - y * 2,
            ),
            self.visible(),
        )
    }
    pub fn point(self, x: usize, y: usize) -> Option<Position> {
        let x = self.x + i64::try_from(x).ok()?;
        let y = self.y + i64::try_from(y).ok()?;
        let point = Position::new(u16::try_from(x).ok()?, u16::try_from(y).ok()?);
        self.contains(point).then_some(point)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clipping_preserves_negative_origin_and_full_height() {
        let placement = Placement::new(
            CellRect::new(-3, -70000, 20, 100000),
            Rect::new(0, 0, 10, 8),
        );
        assert_eq!(placement.visible(), Rect::new(0, 0, 10, 8));
        assert_eq!(placement.y, -70000);
        assert_eq!(placement.height, 100000);
        assert_eq!(placement.point(3, 70000), Some(Position::new(0, 0)));
        assert_eq!(placement.point(2, 70000), None);
    }
}
