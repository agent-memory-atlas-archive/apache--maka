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

//! A bounded paint-only reveal: source and layout are always complete.
use crate::markdown::Palette;
use crate::markdown::VisualLine;
use ratatui::{
    style::Color,
    text::{Line, Span},
};
use std::{
    ops::Range,
    time::{Duration, Instant},
};
use unicode_segmentation::UnicodeSegmentation;

const DURATION: Duration = Duration::from_millis(220);
const FRAME: Duration = Duration::from_millis(33);
const MAX_SPANS: usize = 8;

#[derive(Clone, Default)]
pub(super) struct Reveal {
    spans: Vec<(Range<usize>, Instant)>,
}
impl Reveal {
    pub fn clear(&mut self) {
        self.spans = Vec::new();
    }
    pub fn expire(&mut self, now: Option<Instant>) {
        self.spans.retain(|(_, start)| {
            now.is_some_and(|now| now.saturating_duration_since(*start) < DURATION)
        });
        if self.spans.is_empty() {
            self.clear();
        }
    }
    pub fn append(&mut self, source: Range<usize>, now: Option<Instant>) {
        self.expire(now);
        let Some(now) = now.filter(|_| !source.is_empty()) else {
            return;
        };
        // A transport burst never creates a character backlog or resets older text.
        if self.spans.len() == MAX_SPANS {
            self.spans.remove(0);
        }
        self.spans.push((source, now));
    }
    pub fn paint(
        &self,
        visual: &VisualLine,
        line: &mut Line<'static>,
        now: Option<Instant>,
        colors: Palette,
    ) -> Option<Duration> {
        let now = now.filter(|_| !colors.terminal)?;
        let mut ranges = Vec::new();
        for (source, start) in &self.spans {
            let elapsed = now.saturating_duration_since(*start);
            if elapsed >= DURATION {
                continue;
            }
            let t = elapsed.as_secs_f32() / DURATION.as_secs_f32();
            let strength = 1.0 - 0.18 * (1.0 - t).powi(2);
            for mapping in &visual.mapping {
                let start = source.start.max(mapping.logical.start);
                let end = source.end.min(mapping.logical.end);
                let display = (start < end).then(|| {
                    if mapping.display.len() == mapping.logical.len() {
                        mapping.display.start + start - mapping.logical.start
                            ..mapping.display.start + end - mapping.logical.start
                    } else {
                        mapping.display.clone()
                    }
                });
                if let Some(display) = display {
                    ranges.push((display, strength, FRAME.min(DURATION - elapsed)));
                }
            }
        }
        if ranges.is_empty() {
            return None;
        }
        let mut output: Vec<Span<'static>> = Vec::new();
        let mut offset = 0;
        let mut wake = None;
        for span in std::mem::take(&mut line.spans) {
            for (index, grapheme) in span.content.grapheme_indices(true) {
                let start = offset + index;
                let end = start + grapheme.len();
                let mut style = span.style;
                // An appended combining mark or ZWJ colors the whole grapheme.
                if let Some((_, strength, wait)) = ranges
                    .iter()
                    .rev()
                    .find(|(range, _, _)| start < range.end && end > range.start)
                {
                    let foreground = style.fg.or(line.style.fg).unwrap_or(colors.foreground);
                    let background = style.bg.or(line.style.bg).unwrap_or(colors.background);
                    if let Some(color) = blend(foreground, background, *strength)
                        && color != foreground
                    {
                        style.fg = Some(color);
                        wake = Some(wake.map_or(*wait, |old: Duration| old.min(*wait)));
                    }
                }
                if let Some(previous) = output.last_mut().filter(|last| last.style == style) {
                    previous.content.to_mut().push_str(grapheme);
                } else {
                    output.push(Span::styled(grapheme.to_owned(), style));
                }
            }
            offset += span.content.len();
        }
        line.spans = output;
        wake
    }
}

pub(crate) fn blend(foreground: Color, background: Color, strength: f32) -> Option<Color> {
    let (Color::Rgb(r, g, b), Color::Rgb(br, bg, bb)) = (foreground, background) else {
        // Terminal-owned and ANSI colors have no known RGB background to blend against.
        return None;
    };
    let channel =
        |fg: u8, bg: u8| (f32::from(bg) + (f32::from(fg) - f32::from(bg)) * strength).round() as u8;
    Some(Color::Rgb(channel(r, br), channel(g, bg), channel(b, bb)))
}
