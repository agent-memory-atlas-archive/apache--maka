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

use gpui_kit::{base, *};
use maka_ui::appearance as data;
use std::{rc::Rc, time::Duration};

pub(super) fn color(value: data::Color) -> Hsla {
    rgba(value.0).into()
}
pub(super) fn font(value: &data::FontAppearance) -> Font {
    Font {
        family: value.family.0.clone().into(),
        features: FontFeatures::default(),
        fallbacks: Some(FontFallbacks::from_fonts(
            value.fallbacks.iter().map(|f| f.0.clone()).collect(),
        )),
        weight: FontWeight(value.weight as f32),
        style: FontStyle::Normal,
    }
}

/// Derived from committed immutable data; never consults an App Global.
pub(super) struct TextareaProjection {
    pub data: data::TextareaAppearance,
    pub font: Font,
    pub editor: base::input::InputEditorStyle,
    pub scrollbar: Rc<base::Theme>,
}
impl TextareaProjection {
    pub fn new(value: &data::TextareaAppearance) -> Self {
        let scroll = &value.scrollbar;
        let entrance = match scroll.entrance {
            data::ScrollbarEntrance::Fade => base::ScrollbarEntrance::Fade,
            data::ScrollbarEntrance::SlideAndFade => base::ScrollbarEntrance::SlideAndFade,
        };
        let motion = base::ScrollbarMotion::default()
            .with_idle(Duration::from_millis(scroll.idle_millis.into()))
            .with_enter(Duration::from_millis(scroll.enter_millis.into()))
            .with_exit(Duration::from_millis(scroll.exit_millis.into()))
            .with_expand(Duration::from_millis(scroll.expand_millis.into()))
            .with_entrance(entrance)
            .with_thumb_hover_entrance(entrance);
        let styles = base::ScrollbarStyles::default()
            .track(|s| s.bg(color(scroll.track)).width(px(scroll.track_width)))
            .track_hover(|s| s.bg(color(scroll.track_hover)))
            .track_active(|s| s.bg(color(scroll.track_active)))
            .thumb(|s| thumb(s, &scroll.thumb))
            .thumb_hover(|s| thumb(s, &scroll.thumb_hover))
            .thumb_active(|s| thumb(s, &scroll.thumb_active));
        let mut theme = base::Theme::default();
        theme.tokens.colors.foreground = color(value.text.foreground);
        theme.tokens.colors.muted_foreground = color(value.muted_foreground);
        theme.tokens.radius.md = px(scroll
            .thumb
            .radius
            .max(scroll.thumb_hover.radius)
            .max(scroll.thumb_active.radius));
        theme.scrollbar = base::ScrollbarTheme::new()
            .with_mode(match scroll.mode {
                data::ScrollbarMode::Scrolling => base::ScrollbarMode::Scrolling,
                data::ScrollbarMode::Hover => base::ScrollbarMode::Hover,
                data::ScrollbarMode::Always => base::ScrollbarMode::Always,
            })
            .with_motion(motion)
            .with_styles(styles);
        Self {
            data: value.clone(),
            font: font(&value.text.font),
            editor: base::input::InputEditorStyle {
                foreground: color(value.text.foreground),
                muted_foreground: color(value.muted_foreground),
                background: color(value.background),
                border: color(value.border),
                selection: color(value.selection),
                caret: color(value.caret),
                ..Default::default()
            },
            scrollbar: Rc::new(theme),
        }
    }
    pub fn apply(&self, state: &mut base::input::TextareaState) {
        state.set_complete_editor_style(self.editor.clone());
        state.set_scrollbar_theme(Some(self.scrollbar.clone()));
        let p = self.data.padding;
        state.set_editor_paddings(Edges {
            top: px(p.top),
            right: px(p.right),
            bottom: px(p.bottom),
            left: px(p.left),
        });
    }
}
fn thumb(
    style: base::ScrollbarThumbStyle,
    value: &data::ScrollbarThumb,
) -> base::ScrollbarThumbStyle {
    style
        .bg(color(value.color))
        .width(px(value.width))
        .inset(px(value.inset))
        .radius(px(value.radius))
        .min_length(px(value.min_length))
}
