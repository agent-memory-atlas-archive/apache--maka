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

//! Immutable renderer colors; application themes remain explicit component inputs.
use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub background: Color,
    pub foreground: Color,
    pub surface: Color,
    pub accent: Color,
    pub warning: Color,
    pub muted: Color,
    pub subtle: Color,
    pub border: Color,
    pub success: Color,
    pub error: Color,
    pub syntax: [Color; 7],
    pub terminal: bool,
    pub headings: [Style; 6],
}
impl Default for Palette {
    fn default() -> Self {
        let syntax = [
            0xbe9df7, 0x8ecca7, 0x8591a2, 0xeab982, 0x71a8fd, 0x71ccd1, 0xc0c9d9,
        ]
        .map(rgb);
        let accent = rgb(0x71a8fd);
        let muted = rgb(0xa0afc3);
        let subtle = rgb(0x8695ab);
        Self {
            background: rgb(0x131720),
            foreground: rgb(0xe1e7f0),
            surface: rgb(0x202837),
            accent,
            warning: rgb(0xe7bd7f),
            muted,
            subtle,
            border: rgb(0x344258),
            success: rgb(0x7ecfa4),
            error: rgb(0xec939b),
            syntax,
            terminal: false,
            headings: [syntax[5], accent, syntax[0], muted, muted, subtle]
                .map(|color| Style::default().fg(color).add_modifier(Modifier::BOLD)),
        }
        .with_subtle_heading()
    }
}
impl Palette {
    fn with_subtle_heading(mut self) -> Self {
        self.headings[5] = Style::default().fg(self.subtle);
        self
    }
    pub fn base(self) -> Style {
        Style::default().fg(self.foreground).bg(self.background)
    }
    pub fn panel(self) -> Color {
        match (self.background, self.surface) {
            (Color::Rgb(r, g, b), Color::Rgb(sr, sg, sb)) => {
                let mix = |a: u8, b: u8| ((u16::from(a) + u16::from(b)) / 2) as u8;
                Color::Rgb(mix(r, sr), mix(g, sg), mix(b, sb))
            }
            _ => self.background,
        }
    }
}
const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}
