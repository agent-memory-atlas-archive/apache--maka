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

use super::scene::{Direction, Layout};
use ratatui::layout::Rect;

pub(super) struct Flow {
    pub size: (u16, u16),
    pub children: Vec<Rect>,
}
/// Row width allocation precedes cross-axis measurement. Narrow basis/grow
/// children and wrapped rows must report their height at the actual width.
pub(super) fn measured<E>(
    direction: Direction,
    width: u16,
    height: Option<u16>,
    gap: u16,
    wrap: bool,
    layouts: &[Layout],
    mut measure: impl FnMut(usize, u16) -> Result<(u16, u16), E>,
) -> Result<Flow, E> {
    let mut children = layouts
        .iter()
        .enumerate()
        .map(|(index, &layout)| measure(index, width).map(|size| (size, layout)))
        .collect::<Result<Vec<_>, _>>()?;
    let allocated = flow(direction, width, height, gap, wrap, &children);
    if matches!(direction, Direction::Row) {
        for (index, rect) in allocated.children.iter().enumerate() {
            if rect.width != width {
                children[index].0.1 = measure(index, rect.width)?.1;
            }
        }
        Ok(flow(direction, width, height, gap, wrap, &children))
    } else {
        Ok(allocated)
    }
}
/// One cell-allocation algorithm for measurement and paint. An unbounded
/// column measures intrinsic rows; a bounded viewport distributes grow space.
pub(super) fn flow(
    direction: Direction,
    width: u16,
    height: Option<u16>,
    gap: u16,
    wrap: bool,
    children: &[((u16, u16), Layout)],
) -> Flow {
    if matches!(direction, Direction::Row) && wrap {
        let mut rows: Vec<Vec<((u16, u16), Layout)>> = Vec::new();
        let mut occupied = 0u32;
        for &(size, mut layout) in children {
            let basis = layout.basis.unwrap_or(size.0).min(width);
            if rows.last().is_some_and(|row| !row.is_empty())
                && occupied + u32::from(gap) + u32::from(basis) > u32::from(width)
            {
                occupied = 0;
                rows.push(Vec::new());
            }
            if rows.is_empty() {
                rows.push(Vec::new());
            }
            let row = rows.last_mut().unwrap();
            if !row.is_empty() {
                occupied += u32::from(gap);
            }
            occupied += u32::from(basis);
            layout.basis = Some(basis);
            row.push((size, layout));
        }
        let (mut y, mut widest) = (0u16, 0u16);
        let mut placed = Vec::with_capacity(children.len());
        for (index, row) in rows.iter().enumerate() {
            let line = flow(Direction::Row, width, None, gap, false, row);
            widest = widest.max(line.size.0);
            for mut rect in line.children {
                rect.y = y;
                placed.push(rect);
            }
            y = y.saturating_add(line.size.1);
            if index + 1 < rows.len() {
                y = y.saturating_add(gap);
            }
        }
        return Flow {
            size: (widest, y),
            children: placed,
        };
    }
    let horizontal = matches!(direction, Direction::Row);
    let available = if horizontal { Some(width) } else { height };
    let bases: Vec<u16> = children
        .iter()
        .map(|&(size, layout)| {
            layout
                .basis
                .unwrap_or(if layout.grow > 0 && available.is_some() {
                    0
                } else if horizontal {
                    size.0
                } else {
                    size.1
                })
        })
        .collect();
    let spacing = u32::from(gap) * children.len().saturating_sub(1) as u32;
    let occupied = bases.iter().map(|&n| u32::from(n)).sum::<u32>() + spacing;
    let free = available.map_or(0, |n| u32::from(n).saturating_sub(occupied));
    let weight = children
        .iter()
        .map(|(_, layout)| u32::from(layout.grow))
        .sum::<u32>();
    let (mut offset, mut cross, mut used_weight) = (0u16, 0u16, 0u32);
    let mut placed = Vec::with_capacity(children.len());
    for (&(size, layout), base) in children.iter().zip(bases) {
        let before = used_weight;
        used_weight += u32::from(layout.grow);
        let extra = if weight == 0 {
            0
        } else {
            (u64::from(free) * u64::from(used_weight) / u64::from(weight)
                - u64::from(free) * u64::from(before) / u64::from(weight)) as u32
        };
        let length = base.saturating_add(extra as u16);
        let length = if horizontal {
            length.min(width.saturating_sub(offset))
        } else {
            length
        };
        placed.push(if horizontal {
            Rect::new(offset, 0, length, height.unwrap_or(size.1))
        } else {
            Rect::new(0, offset, width, length)
        });
        cross = cross.max(if horizontal { size.1 } else { size.0 });
        offset = offset.saturating_add(length);
        if placed.len() < children.len() {
            offset = offset.saturating_add(gap);
        }
    }
    let length = offset;
    Flow {
        size: if horizontal {
            (length.min(width), cross)
        } else {
            (cross.min(width), length)
        },
        children: placed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_siblings_reserve_cells_before_long_flexible_content() {
        let children = [
            ((20, 2), Layout::default()),
            (
                (20, 8192),
                Layout {
                    grow: 1,
                    ..Layout::default()
                },
            ),
            ((20, 3), Layout::default()),
        ];
        let layout = flow(Direction::Column, 20, Some(12), 1, false, &children);
        assert_eq!(
            layout.children,
            [
                Rect::new(0, 0, 20, 2),
                Rect::new(0, 3, 20, 5),
                Rect::new(0, 9, 20, 3)
            ]
        );
        assert_eq!(
            flow(Direction::Column, 20, None, 1, false, &children)
                .size
                .1,
            8199
        );
    }
    #[test]
    fn weighted_growth_uses_every_remaining_cell_once() {
        let children = [
            (
                (1, 9),
                Layout {
                    grow: 1,
                    ..Layout::default()
                },
            ),
            (
                (1, 9),
                Layout {
                    grow: 2,
                    ..Layout::default()
                },
            ),
            ((1, 2), Layout::default()),
        ];
        let layout = flow(Direction::Column, 7, Some(10), 0, false, &children);
        assert_eq!(
            layout.children.iter().map(|r| r.height).collect::<Vec<_>>(),
            [2, 6, 2]
        );
        let layout = flow(Direction::Row, 10, Some(3), 0, false, &children);
        assert_eq!(
            layout.children.iter().map(|r| r.width).collect::<Vec<_>>(),
            [3, 6, 1]
        );
    }
    #[test]
    fn wrapping_preserves_control_order_and_reports_actual_rows() {
        let children = [
            ((8, 1), Layout::default()),
            ((9, 1), Layout::default()),
            ((6, 1), Layout::default()),
        ];
        let layout = flow(Direction::Row, 16, None, 1, true, &children);
        assert_eq!(
            layout.children,
            [
                Rect::new(0, 0, 8, 1),
                Rect::new(0, 2, 9, 1),
                Rect::new(10, 2, 6, 1)
            ]
        );
        assert_eq!(layout.size, (16, 3));
    }
}

#[cfg(test)]
mod wrap_growth_tests {
    use super::*;
    #[test]
    fn wrapped_rows_distribute_remaining_width_inside_each_row() {
        let children = [
            (
                (8, 1),
                Layout {
                    grow: 1,
                    ..Layout::default()
                },
            ),
            ((8, 1), Layout::default()),
            (
                (6, 1),
                Layout {
                    grow: 2,
                    ..Layout::default()
                },
            ),
        ];
        let layout = flow(Direction::Row, 18, None, 1, true, &children);
        assert_eq!(
            layout.children,
            [
                Rect::new(0, 0, 9, 1),
                Rect::new(10, 0, 8, 1),
                Rect::new(0, 2, 18, 1)
            ]
        );
        assert_eq!(layout.size, (18, 3));
    }
}
