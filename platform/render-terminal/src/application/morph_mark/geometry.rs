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

//! Surface sampling and spatial pairing copied from the original welcome mark.
use maka_ui_runtime::morph_mark::MorphShape;
#[derive(Clone, Copy)]
pub(super) struct Point {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}
pub(super) struct Drone {
    pub logo: Point,
    pub word: Point,
}
const STEP: f32 = 2.2 / 143.0;
const DEPTH: f32 = 0.085;
const BEVEL: f32 = 0.035;
pub(super) fn formations(shapes: &[MorphShape; 2]) -> Vec<Drone> {
    let prepare = |shape: &MorphShape| {
        let paths: Vec<_> = shape.paths.iter().map(Vec::as_slice).collect();
        let miters: Vec<_> = shape
            .miters
            .iter()
            .map(|i| miter(paths[*i], shape.radius))
            .collect();
        surface(&paths, shape.radius, &miters, shape.columns)
    };
    let mut logo = prepare(&shapes[0]);
    let mut word = prepare(&shapes[1]);
    if logo.is_empty() || word.is_empty() {
        return vec![];
    }
    // Pair nearby groups in spatial order, so neighboring dots travel together
    // rather than randomly crossing the entire formation during the transition.
    let order = |points: &mut Vec<Point>, extent: f32| {
        let spread = |mut value: u32| {
            value = (value | value << 8) & 0x00ff00ff;
            value = (value | value << 4) & 0x0f0f0f0f;
            value = (value | value << 2) & 0x33333333;
            (value | value << 1) & 0x55555555
        };
        points.sort_by_key(|point| {
            let x = ((point.position[0] / extent + 1.0) * 32767.0).clamp(0.0, 65535.0) as u32;
            let y = ((point.position[1] + 1.1) / 2.2 * 65535.0).clamp(0.0, 65535.0) as u32;
            spread(x) | spread(y) << 1
        });
    };
    order(&mut logo, shapes[0].columns as f32 * STEP / 2.0);
    order(&mut word, shapes[1].columns as f32 * STEP / 2.0);
    let count = logo.len().max(word.len());
    (0..count)
        .map(|i| Drone {
            logo: logo[i * (logo.len() - 1) / (count - 1)],
            word: word[i * (word.len() - 1) / (count - 1)],
        })
        .collect()
}
fn segment(point: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let [dx, dy] = [b[0] - a[0], b[1] - a[1]];
    let t =
        (((point[0] - a[0]) * dx + (point[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (point[0] - a[0] - dx * t).hypot(point[1] - a[1] - dy * t)
}

fn miter(path: &[[f32; 2]], radius: f32) -> [[f32; 2]; 3] {
    let [a, b, c] = [path[0], path[1], path[2]];
    let normal = |from: [f32; 2], to: [f32; 2]| {
        let [x, y] = [to[0] - from[0], to[1] - from[1]];
        let length = x.hypot(y);
        [y / length, -x / length]
    };
    let mut n = [normal(a, b), normal(b, c)];
    if n[0][1] + n[1][1] > 0.0 {
        n = n.map(|v| v.map(|v| -v));
    }
    let sum = [n[0][0] + n[1][0], n[0][1] + n[1][1]];
    let denominator = sum[0] * n[0][0] + sum[1] * n[0][1];
    if denominator.abs() < 0.001 {
        return [b; 3];
    }
    let scale = radius / denominator;
    [
        [b[0] + n[0][0] * radius, b[1] + n[0][1] * radius],
        [b[0] + sum[0] * scale, b[1] + sum[1] * scale],
        [b[0] + n[1][0] * radius, b[1] + n[1][1] * radius],
    ]
}

fn triangle(point: [f32; 2], vertices: [[f32; 2]; 3]) -> f32 {
    let mut distance = f32::INFINITY;
    let mut signs = [0.0; 3];
    for i in 0..3 {
        let [a, b] = [vertices[i], vertices[(i + 1) % 3]];
        distance = distance.min(segment(point, a, b));
        signs[i] = (b[0] - a[0]) * (point[1] - a[1]) - (b[1] - a[1]) * (point[0] - a[0]);
    }
    if signs.iter().all(|v| *v >= 0.0) || signs.iter().all(|v| *v <= 0.0) {
        distance
    } else {
        -distance
    }
}

fn surface(
    paths: &[&[[f32; 2]]],
    radius: f32,
    miters: &[[[f32; 2]; 3]],
    columns: usize,
) -> Vec<Point> {
    let x0 = (columns - 1) as f32 * STEP / 2.0;
    let mut field = vec![-1.0_f32; 144 * columns];
    // Only each stroke's small neighborhood can contribute to the surface.
    // This avoids scanning every curve for every pixel on the first UI frame.
    let mut paint = |vertices: &[[f32; 2]], padding: f32, distance: &dyn Fn([f32; 2]) -> f32| {
        let bounds = vertices.iter().fold(
            [
                f32::INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
            ],
            |[l, t, r, b], [x, y]| [l.min(*x), t.min(*y), r.max(*x), b.max(*y)],
        );
        let left = ((bounds[0] - padding + x0) / STEP).floor().max(0.0) as usize;
        let right = ((bounds[2] + padding + x0) / STEP)
            .ceil()
            .min((columns - 1) as f32) as usize;
        let top = ((bounds[1] - padding + 1.1) / STEP).floor().max(0.0) as usize;
        let bottom = ((bounds[3] + padding + 1.1) / STEP).ceil().min(143.0) as usize;
        for row in top..=bottom {
            for column in left..=right {
                let i = row * columns + column;
                field[i] = field[i].max(distance([
                    column as f32 * STEP - x0,
                    row as f32 * STEP - 1.1,
                ]));
            }
        }
    };
    for path in paths {
        paint(path, radius + STEP * 3.0, &|point| {
            path.windows(2)
                .map(|pair| radius - segment(point, pair[0], pair[1]))
                .fold(f32::NEG_INFINITY, f32::max)
        });
    }
    for vertices in miters {
        paint(vertices, STEP * 3.0, &|point| triangle(point, *vertices));
    }
    let mut points = Vec::new();
    for row in 1..143 {
        for column in 1..columns - 1 {
            let [x, y] = [column as f32 * STEP - x0, row as f32 * STEP - 1.1];
            let i = row * columns + column;
            let d = field[i];
            if d < -STEP {
                continue;
            }
            let [dx, dy] = [
                field[i + 1] - field[i - 1],
                field[i + columns] - field[i - columns],
            ];
            let length = dx.hypot(dy).max(f32::EPSILON);
            let [nx, ny] = [-dx / length, -dy / length];
            if d >= 0.0 {
                let edge = (1.0 - d / BEVEL).clamp(0.0, 1.0);
                let nz = (1.0 - edge * edge).sqrt();
                let z = DEPTH - BEVEL + BEVEL * nz;
                for sign in [-1.0, 1.0] {
                    points.push(Point {
                        position: [x, y, sign * z],
                        normal: [nx * edge, ny * edge, sign * nz],
                    });
                }
            }
            if d.abs() < STEP * 0.6 {
                for layer in 0..=8 {
                    points.push(Point {
                        position: [
                            x + nx * d,
                            y + ny * d,
                            (DEPTH - BEVEL) * (layer as f32 / 4.0 - 1.0),
                        ],
                        normal: [nx, ny, 0.0],
                    });
                }
            }
        }
    }
    points
}
