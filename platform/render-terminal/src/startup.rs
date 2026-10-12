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

//! Trusted first-install UI, before any product modules or workspace tools run.
use crate::terminal::{self, Guard, Screen};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph, Wrap},
};
use std::{io, path::Path};

/// The same alternate screen and terminal guard pass into the product presenter.
pub struct Startup {
    pub(crate) screen: Screen,
    pub(crate) guard: Guard,
}
impl Startup {
    pub fn confirm(workspace: &Path, locale: &str, light: bool) -> io::Result<Option<Self>> {
        let mut guard = Guard::new();
        let screen = guard.setup("an interactive terminal is required")?;
        let mut startup = Self { screen, guard };
        let copy = Copy::for_locale(locale);
        let path = crate::application::sanitize(&workspace.display().to_string());
        let mut accept = false;
        loop {
            terminal::draw(&mut startup.screen, |frame| {
                paint(frame, &copy, &path, light, accept, false);
            })?;
            match choice(event::read()?, &mut accept) {
                Some(true) => {
                    terminal::draw(&mut startup.screen, |frame| {
                        paint(frame, &copy, &path, light, accept, true);
                    })?;
                    return Ok(Some(startup));
                }
                Some(false) => {
                    startup.guard.finish()?;
                    return Ok(None);
                }
                None => {}
            }
        }
    }
}
/// Keep startup failures visible even when a recovery component can mount.
/// This owns no plugin work; the caller retains and drains the original presenter.
pub(crate) fn issues(screen: &mut Screen, message: &str) -> io::Result<bool> {
    let message = crate::application::sanitize(message);
    let mut scroll = 0u16;
    loop {
        terminal::draw(screen, |frame| {
            let area = frame.area();
            frame.render_widget(
                Block::default().style(
                    Style::default()
                        .bg(Color::Rgb(23, 23, 26))
                        .fg(Color::Rgb(235, 235, 240)),
                ),
                area,
            );
            let body = Block::default().padding(Padding::uniform(2)).inner(area);
            let chunks = Layout::vertical([
                Constraint::Length(2),
                Constraint::Min(1),
                Constraint::Length(2),
            ])
            .split(body);
            frame.render_widget(
                Paragraph::new("Maka · Startup failed").style(
                    Style::default()
                        .fg(Color::Rgb(255, 160, 140))
                        .add_modifier(Modifier::BOLD),
                ),
                chunks[0],
            );
            frame.render_widget(
                Paragraph::new(message.as_str())
                    .wrap(Wrap { trim: false })
                    .scroll((scroll, 0)),
                chunks[1],
            );
            frame.render_widget(
                Paragraph::new("Enter: open recovery interface · Ctrl-Q / Esc: exit · ↑ ↓: scroll")
                    .wrap(Wrap { trim: false }),
                chunks[2],
            );
        })?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Enter => return Ok(true),
            KeyCode::Esc => return Ok(false),
            KeyCode::Char('c' | 'q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(false);
            }
            KeyCode::Down => {
                scroll = scroll
                    .saturating_add(1)
                    .min(message.chars().count().min(u16::MAX as usize) as u16)
            }
            KeyCode::Up => scroll = scroll.saturating_sub(1),
            _ => {}
        }
    }
}

fn choice(event: Event, accept: &mut bool) -> Option<bool> {
    let Event::Key(key) = event else { return None };
    if key.kind != KeyEventKind::Press {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c' | 'q'))
    {
        return Some(false);
    }
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
    {
        return None;
    }
    match key.code {
        KeyCode::Tab
        | KeyCode::BackTab
        | KeyCode::Left
        | KeyCode::Right
        | KeyCode::Up
        | KeyCode::Down => {
            *accept = !*accept;
            None
        }
        KeyCode::Char('y' | 'Y') => Some(true),
        KeyCode::Char('n' | 'N') | KeyCode::Esc => Some(false),
        KeyCode::Enter => Some(*accept),
        _ => None,
    }
}
struct Copy {
    title: &'static str,
    body: &'static str,
    accept: &'static str,
    cancel: &'static str,
    keys: &'static str,
    opening: &'static str,
}
impl Copy {
    fn for_locale(locale: &str) -> Self {
        let language = locale.to_ascii_lowercase();
        if language.starts_with("zh") {
            if ["hant", "tw", "hk", "mo"]
                .iter()
                .any(|part| language.contains(part))
            {
                Self {
                    title: "信任此工作目錄？",
                    body: "允許修改此目錄中的檔案並執行沙箱命令。命令可以讀取目錄外的檔案，但網路存取已停用。專案和使用者技能目錄將啟用。",
                    accept: "信任並繼續",
                    cancel: "退出",
                    keys: "← → / Tab 選擇 · Enter 確認 · Esc 退出",
                    opening: "正在初始化產品…",
                }
            } else {
                Self {
                    title: "信任此工作目录？",
                    body: "允许修改此目录中的文件并执行沙箱命令。命令可以读取目录外的文件，但网络访问已禁用。项目和用户技能目录将启用。",
                    accept: "信任并继续",
                    cancel: "退出",
                    keys: "← → / Tab 选择 · Enter 确认 · Esc 退出",
                    opening: "正在初始化产品…",
                }
            }
        } else {
            Self {
                title: "Trust this workspace?",
                body: "Allow file changes in this directory and sandboxed commands. Commands may read files outside it; their network access is disabled. Project and user skill directories will be enabled.",
                accept: "Trust and continue",
                cancel: "Exit",
                keys: "← → / Tab select · Enter confirm · Esc exit",
                opening: "Initializing product…",
            }
        }
    }
}
fn paint(
    frame: &mut ratatui::Frame<'_>,
    copy: &Copy,
    path: &str,
    light: bool,
    accept: bool,
    opening: bool,
) {
    let (background, foreground, muted, accent) = if light {
        (
            Color::Rgb(250, 250, 250),
            Color::Rgb(25, 25, 28),
            Color::Rgb(90, 90, 100),
            Color::Rgb(30, 95, 170),
        )
    } else {
        (
            Color::Rgb(23, 23, 26),
            Color::Rgb(235, 235, 240),
            Color::Rgb(160, 160, 170),
            Color::Rgb(110, 180, 250),
        )
    };
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(background).fg(foreground)),
        area,
    );
    let width = area.width.min(84);
    let body = ratatui::layout::Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y,
        width,
        height: area.height,
    };
    let body = Block::default().padding(Padding::uniform(2)).inner(body);
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(body);
    frame.render_widget(
        Paragraph::new("Maka").style(Style::default().fg(accent).add_modifier(Modifier::BOLD)),
        chunks[0],
    );
    let text = vec![
        Line::from(Span::styled(
            copy.title,
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(path),
        Line::from(""),
        Line::from(Span::styled(copy.body, Style::default().fg(muted))),
    ];
    frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), chunks[1]);
    let selected = Style::default()
        .bg(accent)
        .fg(background)
        .add_modifier(Modifier::BOLD);
    let actions = if opening {
        vec![Line::from(Span::styled(
            copy.opening,
            Style::default().fg(accent),
        ))]
    } else {
        vec![
            Line::from(vec![
                Span::styled(
                    format!(" {} ", copy.accept),
                    if accept { selected } else { Style::default() },
                ),
                Span::raw("   "),
                Span::styled(
                    format!(" {} ", copy.cancel),
                    if !accept { selected } else { Style::default() },
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(copy.keys, Style::default().fg(muted))),
        ]
    };
    frame.render_widget(Paragraph::new(actions), chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;
    #[test]
    fn trust_requires_explicit_selection_and_ignores_paste_and_releases() {
        let mut accept = false;
        let key = |code| Event::Key(KeyEvent::new(code, KeyModifiers::NONE));
        assert_eq!(choice(Event::Paste("y\n".into()), &mut accept), None);
        assert_eq!(choice(key(KeyCode::Enter), &mut accept), Some(false));
        assert_eq!(choice(key(KeyCode::Tab), &mut accept), None);
        assert_eq!(choice(key(KeyCode::Enter), &mut accept), Some(true));
        assert_eq!(
            choice(
                Event::Key(KeyEvent::new_with_kind(
                    KeyCode::Char('y'),
                    KeyModifiers::NONE,
                    KeyEventKind::Release
                )),
                &mut accept
            ),
            None
        );
        assert_eq!(choice(key(KeyCode::Esc), &mut accept), Some(false));
    }
}
