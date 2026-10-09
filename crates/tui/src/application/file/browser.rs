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

//! Original attachment browser bounds and ordering, owned by the local picker.
use super::super::{CellRect, Placement, paint};
use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use maka_ui::{
    budget::{Kind, NativeBudget, Permit},
    file::{FileError, FilePickerProps, FileSnapshot, read_selected},
};
use ratatui::{buffer::Buffer, layout::Rect, style::Style};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

const MAX_ENTRIES: usize = 2048;

pub(super) struct Request {
    pub path: PathBuf,
    pub directory: bool,
}
pub(super) enum Selected {
    File {
        path: PathBuf,
        snapshot: Arc<FileSnapshot>,
    },
    Directory(Directory),
}
struct Entry {
    path: PathBuf,
    directory: bool,
    bytes: u64,
    _memory: Permit,
}
pub(super) struct Directory {
    path: PathBuf,
    entries: Vec<Entry>,
    truncated: bool,
    _memory: Permit,
}
/// Only an admitted physical gesture can call this, in its original worker.
pub(super) fn select(request: Request, budget: &NativeBudget) -> Result<Selected, FileError> {
    let mut path = if request.path.as_os_str().is_empty() {
        std::env::current_dir().map_err(|_| FileError::ReadFailed)?
    } else {
        request.path
    };
    if !super::paths::valid(&path) {
        return Err(FileError::InvalidFile);
    }
    let metadata = std::fs::symlink_metadata(&path).map_err(|_| FileError::ReadFailed)?;
    if metadata.is_file() {
        if !request.directory {
            let snapshot = read_selected(&path, budget)?;
            return Ok(Selected::File { path, snapshot });
        }
        path = path.parent().ok_or(FileError::InvalidFile)?.to_path_buf();
    } else if !metadata.is_dir() {
        return Err(FileError::InvalidFile);
    }
    // Charge retained path/entry allocations to the original shared native budget.
    let memory = budget.reserve(
        Kind::Encoded,
        MAX_ENTRIES * std::mem::size_of::<Entry>() + path.capacity(),
    )?;
    let mut entries = Vec::with_capacity(MAX_ENTRIES);
    let mut truncated = false;
    for (index, entry) in std::fs::read_dir(&path)
        .map_err(|_| FileError::ReadFailed)?
        .enumerate()
    {
        if index == MAX_ENTRIES {
            truncated = true;
            break;
        }
        let entry = entry.map_err(|_| FileError::ReadFailed)?;
        let kind = entry.file_type().map_err(|_| FileError::ReadFailed)?;
        let path = entry.path();
        if !super::paths::valid(&path) || !(kind.is_dir() || kind.is_file()) {
            continue;
        }
        let metadata = entry.metadata().map_err(|_| FileError::ReadFailed)?;
        let memory = budget.reserve(Kind::Encoded, path.capacity())?;
        entries.push(Entry {
            path,
            directory: kind.is_dir(),
            bytes: metadata.len(),
            _memory: memory,
        });
    }
    entries.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then_with(|| a.path.file_name().cmp(&b.path.file_name()))
    });
    Ok(Selected::Directory(Directory {
        path,
        entries,
        truncated,
        _memory: memory,
    }))
}

pub(super) struct Browser {
    directory: Directory,
    revision: u64,
    selected: usize,
    offset: usize,
    reveal: bool,
    geometry: Option<Geometry>,
    drag: Option<Rect>,
}
pub(super) struct Geometry {
    revision: u64,
    pub area: Placement,
    hits: Vec<(usize, Placement)>,
    offset: usize,
    rows: usize,
    bar: Option<Rect>,
    intent: (usize, usize, bool),
}
pub(super) enum Input {
    Select(PathBuf),
    Handled,
    PreserveFocus,
    Unhandled,
}
impl Browser {
    pub fn new(directory: Directory, revision: u64) -> Self {
        Self {
            directory,
            revision,
            selected: 0,
            offset: 0,
            reveal: true,
            geometry: None,
            drag: None,
        }
    }
    pub fn path(&self) -> &Path {
        &self.directory.path
    }
    pub fn parent(&self) -> Option<PathBuf> {
        self.geometry
            .as_ref()
            .filter(|geometry| geometry.revision == self.revision)?;
        self.directory.path.parent().map(Path::to_path_buf)
    }
    pub fn has_parent(&self) -> bool {
        self.directory.path.parent().is_some()
    }
    pub fn height(&self, rows: usize) -> u32 {
        self.directory.entries.len().min(rows).max(1) as u32 + 1
    }
    pub fn suspend(&mut self) {
        self.geometry = None;
        self.drag = None;
    }
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }
    pub fn cancel_pointer(&mut self) {
        self.drag = None;
    }
    pub fn input(&mut self, event: &Event) -> Input {
        if matches!(event, Event::Mouse(mouse) if mouse.kind == MouseEventKind::Up(MouseButton::Left))
            && self.drag.take().is_some()
        {
            return Input::PreserveFocus;
        }
        let Some(geometry) = self
            .geometry
            .as_ref()
            .filter(|geometry| geometry.revision == self.revision)
        else {
            return Input::Handled;
        };
        if let Event::Mouse(mouse) = event {
            let bar = match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => geometry
                    .bar
                    .filter(|bar| bar.contains((mouse.column, mouse.row).into())),
                MouseEventKind::Drag(MouseButton::Left) => {
                    self.drag.filter(|bar| Some(*bar) == geometry.bar)
                }
                _ => None,
            };
            if let Some(bar) = bar {
                // Original Surface/Rows track mapping, including captured drags outside the pane.
                let maximum = self.directory.entries.len().saturating_sub(geometry.rows);
                let row = mouse
                    .row
                    .saturating_sub(bar.y)
                    .min(bar.height.saturating_sub(1));
                self.offset =
                    usize::from(row) * maximum / usize::from(bar.height.saturating_sub(1).max(1));
                self.reveal = false;
                self.drag = Some(bar);
                return Input::PreserveFocus;
            }
        }
        let last = self.directory.entries.len().saturating_sub(1);
        match event {
            Event::Mouse(mouse) if geometry.area.contains((mouse.column, mouse.row).into()) => {
                match mouse.kind {
                    MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                        self.offset = if mouse.kind == MouseEventKind::ScrollDown {
                            self.offset
                                .saturating_add(3)
                                .min(self.directory.entries.len().saturating_sub(geometry.rows))
                        } else {
                            self.offset.saturating_sub(3)
                        };
                        self.reveal = false;
                        Input::Handled
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        if let Some((index, _)) = geometry
                            .hits
                            .iter()
                            .find(|(_, area)| area.contains((mouse.column, mouse.row).into()))
                        {
                            self.selected = *index;
                            Input::Select(self.directory.entries[*index].path.clone())
                        } else {
                            Input::Handled
                        }
                    }
                    _ => Input::Unhandled,
                }
            }
            Event::Key(key) if key.kind != KeyEventKind::Release && key.modifiers.is_empty() => {
                self.drag = None;
                match key.code {
                    KeyCode::Enter => {
                        return geometry
                            .hits
                            .iter()
                            .find(|(index, _)| *index == self.selected)
                            .map_or(Input::Handled, |(index, _)| {
                                Input::Select(self.directory.entries[*index].path.clone())
                            });
                    }
                    KeyCode::Backspace => {
                        return self.parent().map_or(Input::Handled, Input::Select);
                    }
                    KeyCode::Home => self.selected = 0,
                    KeyCode::End => self.selected = last,
                    KeyCode::Up => self.selected = self.selected.saturating_sub(1),
                    KeyCode::Down => self.selected = self.selected.saturating_add(1).min(last),
                    KeyCode::PageUp => self.selected = self.selected.saturating_sub(geometry.rows),
                    KeyCode::PageDown => {
                        self.selected = self.selected.saturating_add(geometry.rows).min(last)
                    }
                    _ => return Input::Unhandled,
                }
                self.reveal = true;
                Input::Handled
            }
            _ => Input::Unhandled,
        }
    }
    pub fn paint(
        &self,
        props: &FilePickerProps,
        area: Placement,
        rows: usize,
        buffer: &mut Buffer,
        style: Style,
        focused: bool,
    ) -> Geometry {
        let viewport = area.clipped(buffer.area).visible();
        let rows = self
            .directory
            .entries
            .len()
            .min(rows)
            .max(1)
            .min(usize::from(viewport.height.saturating_sub(1).max(1)))
            .min(usize::from(viewport.height));
        let mut offset = self
            .offset
            .min(self.directory.entries.len().saturating_sub(rows));
        let resized = focused && self.geometry.as_ref().is_some_and(|g| g.rows != rows);
        if self.reveal || resized {
            offset = offset
                .min(self.selected)
                .max((self.selected + 1).saturating_sub(rows));
        }
        let mut hits = Vec::new();
        let mut list = Placement::from(viewport);
        list.bounds.height = rows as u32;
        let viewport = list.visible();
        let bar =
            (self.directory.entries.len() > rows && viewport.height > 1 && viewport.width > 0)
                .then(|| Rect::new(viewport.right() - 1, viewport.y, 1, viewport.height));
        for (line, (index, entry)) in self
            .directory
            .entries
            .iter()
            .enumerate()
            .skip(offset)
            .take(rows)
            .enumerate()
        {
            let row = Placement::new(
                CellRect::new(
                    list.x,
                    list.y + line as i64,
                    list.width.saturating_sub(1),
                    1,
                ),
                list.clip,
            )
            .clipped(buffer.area);
            let style = if focused && index == self.selected {
                style.add_modifier(ratatui::style::Modifier::REVERSED)
            } else {
                style
            };
            buffer.set_style(row.visible(), style);
            let name = entry.path.file_name().unwrap().to_str().unwrap();
            let label = if entry.directory {
                format!("/ {name}")
            } else {
                format!("  {name} · {} B", entry.bytes)
            };
            paint::text(
                &paint::truncated_line(&label, row.width),
                row,
                buffer,
                style,
            );
            if !row.is_empty() {
                hits.push((index, row));
            }
        }
        if let Some(bar) = bar {
            let maximum = self.directory.entries.len() - rows;
            let visible = usize::from(bar.height);
            let thumb = (visible * visible / (maximum + visible)).max(1);
            let top = offset * (visible - thumb) / maximum;
            for line in 0..visible {
                buffer[(bar.x, bar.y + line as u16)]
                    .set_char(if (top..top + thumb).contains(&line) {
                        '█'
                    } else {
                        '│'
                    })
                    .set_style(style);
            }
        }
        if self.directory.entries.is_empty() {
            paint::text(&props.empty_label, list, buffer, style);
        }
        let status = if self.directory.truncated {
            props.truncated_label.clone()
        } else {
            format!(
                "{}–{} / {}",
                if self.directory.entries.is_empty() {
                    0
                } else {
                    offset + 1
                },
                (offset + rows).min(self.directory.entries.len()),
                self.directory.entries.len()
            )
        };
        paint::text(
            &paint::truncated_line(&status, list.width),
            Placement::new(
                CellRect::new(list.x, list.y + rows as i64, list.width, 1),
                list.clip,
            ),
            buffer,
            style,
        );
        Geometry {
            revision: self.revision,
            area: list,
            hits,
            offset,
            rows,
            bar,
            intent: (self.offset, self.selected, self.reveal),
        }
    }
    pub fn present(&mut self, geometry: Geometry) {
        if geometry.revision != self.revision {
            return;
        }
        if self.drag.is_some_and(|bar| Some(bar) != geometry.bar) {
            self.drag = None;
        }
        if geometry.intent == (self.offset, self.selected, self.reveal) {
            self.offset = geometry.offset;
            self.reveal = false;
        }
        self.geometry = Some(geometry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers, MouseEvent};
    use ratatui::layout::Rect;

    fn listing(path: &Path, budget: &NativeBudget) -> Directory {
        let Selected::Directory(directory) = select(
            Request {
                path: path.into(),
                directory: true,
            },
            budget,
        )
        .unwrap() else {
            panic!("expected directory");
        };
        directory
    }
    #[test]
    fn original_ordering_limits_and_native_allocations_are_bounded() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("z-folder")).unwrap();
        for name in ["z.txt", "a.txt"] {
            std::fs::write(root.path().join(name), b"abc").unwrap();
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.path().join("a.txt"), root.path().join("symlink"))
                .unwrap();
            assert!(matches!(
                select(
                    Request {
                        path: root.path().join("symlink"),
                        directory: false
                    },
                    &NativeBudget::default()
                ),
                Err(FileError::InvalidFile)
            ));
        }
        let budget = NativeBudget::default();
        let directory = listing(root.path(), &budget);
        assert_eq!(
            directory
                .entries
                .iter()
                .map(|entry| entry.path.file_name().unwrap().to_str().unwrap())
                .collect::<Vec<_>>(),
            ["z-folder", "a.txt", "z.txt"]
        );
        assert!(!directory.truncated);
        assert!(budget.usage().0 > 0);
        drop(directory);
        assert_eq!(budget.usage(), (0, 0, 0));
        let tiny = NativeBudget::new(maka_ui::budget::BudgetLimits {
            encoded_bytes: 1,
            ..Default::default()
        });
        assert!(matches!(
            select(
                Request {
                    path: root.path().into(),
                    directory: true
                },
                &tiny
            ),
            Err(FileError::BudgetExhausted)
        ));
        assert_eq!(tiny.usage(), (0, 0, 0));
        assert!(matches!(
            select(
                Request {
                    path: "relative".into(),
                    directory: true
                },
                &budget
            ),
            Err(FileError::InvalidFile)
        ));
        for index in 0..MAX_ENTRIES {
            std::fs::write(root.path().join(format!("entry-{index:04}")), b"").unwrap();
        }
        let directory = listing(root.path(), &budget);
        assert!(directory.truncated);
        assert!(directory.entries.len() <= MAX_ENTRIES);
        drop(directory);
        assert_eq!(budget.usage(), (0, 0, 0));
    }
    #[test]
    fn browser_uses_only_presented_rows_and_suspends_navigation_on_retirement() {
        let root = tempfile::tempdir().unwrap();
        for index in 0..20 {
            std::fs::write(root.path().join(format!("{index:02}.txt")), b"chosen").unwrap();
        }
        let budget = NativeBudget::default();
        let props = super::super::tests::props();
        let mut browser = Browser::new(listing(root.path(), &budget), 7);
        let area = Rect::new(0, 0, 30, 6);
        let paint = |browser: &Browser| {
            browser.paint(
                &props,
                Placement::new(CellRect::new(0, 0, 30, 6), area),
                5,
                &mut Buffer::empty(area),
                Style::default(),
                true,
            )
        };
        let key = |code| Event::Key(KeyEvent::new(code, KeyModifiers::NONE));
        assert!(matches!(
            browser.input(&key(KeyCode::Enter)),
            Input::Handled
        ));
        browser.present(paint(&browser));
        browser.input(&key(KeyCode::End));
        assert!(
            matches!(browser.input(&key(KeyCode::Enter)), Input::Handled),
            "an unpainted offscreen row cannot activate"
        );
        browser.present(paint(&browser));
        let Input::Select(path) = browser.input(&key(KeyCode::Enter)) else {
            panic!("visible last file");
        };
        assert_eq!(path, root.path().join("19.txt"));
        // A parent can clip the list to a single visible row on terminal resize.
        let clipped = browser.paint(
            &props,
            Placement::new(CellRect::new(0, 0, 30, 6), Rect::new(0, 0, 30, 1)),
            5,
            &mut Buffer::empty(area),
            Style::default(),
            true,
        );
        assert_eq!(clipped.rows, 1);
        browser.present(clipped);
        let Input::Select(path) = browser.input(&key(KeyCode::Enter)) else {
            panic!("resized selected row must be visible and actionable");
        };
        assert_eq!(path, root.path().join("19.txt"));
        browser.present(paint(&browser));
        browser.input(&Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 1,
            row: 2,
            modifiers: KeyModifiers::NONE,
        }));
        browser.present(paint(&browser));
        let Input::Select(path) = browser.input(&Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 1,
            row: 0,
            modifiers: KeyModifiers::NONE,
        })) else {
            panic!("painted pointer row");
        };
        assert_eq!(path, root.path().join("12.txt"));
        assert!(matches!(
            browser.input(&Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 29,
                row: 0,
                modifiers: KeyModifiers::NONE
            })),
            Input::PreserveFocus
        ));
        assert!(browser.dragging());
        browser.input(&Event::Mouse(MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 40,
            row: 30,
            modifiers: KeyModifiers::NONE,
        }));
        browser.present(paint(&browser));
        assert_eq!(
            browser.offset, 15,
            "captured outside drag reaches the final viewport"
        );
        browser.input(&Event::Mouse(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 40,
            row: 30,
            modifiers: KeyModifiers::NONE,
        }));
        assert!(!browser.dragging());
        assert!(browser.parent().is_some());
        browser.suspend();
        assert!(browser.parent().is_none());
        assert!(matches!(
            browser.input(&key(KeyCode::Enter)),
            Input::Handled
        ));
        let mut stale = paint(&browser);
        stale.revision = 6;
        browser.present(stale);
        assert!(matches!(
            browser.input(&key(KeyCode::Enter)),
            Input::Handled
        ));
        drop(browser);
        assert_eq!(budget.usage(), (0, 0, 0));
    }
}
