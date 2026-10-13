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

//! File paths in a reply that open on this machine: which spans name one,
//! the reply's Markdown with each of them made a link, and where a click
//! on one goes.
//!
//! The kit's text view draws inline code as text and offers no hook for a
//! click on it; its Markdown plugins replace a node with an element of
//! their own, which a find cannot paint inside and a selection takes
//! whole. So once a reply has finished streaming, the view hands its text
//! view a copy of the reply's Markdown ([`PathLinks`]) in which each
//! qualifying span is a link with this client's own scheme ([`SCHEME`]):
//! an inline code span `` `~/a.md` `` becomes
//! `` [`~/a.md`](maka-file:/Users/me/a.md) ``, still a code chip, drawn in
//! the link colour with the link's pointer and underline; a link to a
//! `file:` URL or to a plain path keeps its text and takes the scheme for
//! its address. The text drawn is the reply's own, so a selection copies
//! the path as shown and Copy reply copies the reply as written; a find
//! searches the reply's own Markdown and maps each match through
//! [`PathLinks::map_range`] before the text view paints it.
//!
//! Only for a Host on this machine ([`LocalPaths`], from the shell; a Host
//! elsewhere gets none and nothing is decorated), and only when the path
//! exists, file or folder:
//! - an inline code span whose whole content, trimmed, is one path:
//!   absolute (`/…`), in the home folder (`~/…`), or relative to the
//!   task's working folder; a trailing `:line` or `:line:col` may be
//!   dropped. A command such as `ls ~/Downloads` names no path that
//!   exists;
//! - a link whose address is a `file:` URL or a plain path, resolved the
//!   same way.
//!
//! Not code blocks, not bare paths in prose, not code inside a link's
//! text, not user messages. Existence is checked once per reply, when it
//! has finished streaming, off the main thread ([`check`]).
//!
//! A click decides nothing from what the reply wrote: the view reads the
//! path back out of the link, resolves it again and hands it to the shell
//! ([`crate::ConversationViewEvent::OpenPath`]), which looks at the file
//! as it is then. A link the agent wrote with this scheme goes the same
//! way.

use std::ops::Range;
use std::path::{Path, PathBuf};

use gpui_kit::SharedString;
use markdown::mdast::Node;

use crate::corpus::parse_options;

/// The scheme of a link to a path on this machine: this client's own,
/// never handed to the platform.
pub(crate) const SCHEME: &str = "maka-file:";

/// The most spans of one reply that are looked at.
const MAX_SPANS: usize = 64;
/// The longest path looked at, in bytes (macOS's `PATH_MAX`).
const MAX_PATH_BYTES: usize = 1024;

/// Where the paths a reply names lie on this machine: the home folder
/// (`~/…`) and the task's working folder (a relative path), as the shell
/// hands them to the view for a Host on this machine
/// ([`crate::ConversationView::set_local_paths`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalPaths {
    home: Option<PathBuf>,
    folder: Option<PathBuf>,
}

impl LocalPaths {
    pub fn new(home: Option<PathBuf>, folder: Option<PathBuf>) -> Self {
        Self { home, folder }
    }

    /// The path `written` names, without looking at the disk: as it is
    /// when absolute, in the home folder for `~/…`, else in the working
    /// folder. `None` for an empty text, a URL (`scheme://…`), or a folder
    /// this does not know.
    pub(crate) fn resolve(&self, written: &str) -> Option<PathBuf> {
        if written.is_empty()
            || written.len() > MAX_PATH_BYTES
            || written.contains(['\n', '\r', '\0'])
            || url_scheme(written).is_some_and(|_| written.contains("://"))
        {
            return None;
        }
        if let Some(rest) = written.strip_prefix("~/") {
            return self.home.as_ref().map(|home| home.join(rest));
        }
        let path = Path::new(written);
        if path.is_absolute() {
            return Some(path.to_path_buf());
        }
        self.folder.as_ref().filter(|folder| folder.is_absolute()).map(|folder| folder.join(path))
    }
}

/// A settled reply's Markdown with each span that names an existing path
/// on this machine made a link with [`SCHEME`], and the map from the
/// reply's own Markdown to it.
#[derive(Debug)]
pub(crate) struct PathLinks {
    text: SharedString,
    /// What changed, in source order.
    edits: Vec<Edit>,
}

/// `removed` bytes of the reply at `at` became `inserted` bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Edit {
    at: usize,
    removed: usize,
    inserted: usize,
}

impl PathLinks {
    /// `source` with each of `changes` (a range of it and what replaces
    /// it; in order, none overlapping) made.
    fn new(source: &str, changes: Vec<(Range<usize>, String)>) -> Self {
        let mut text = String::with_capacity(source.len() + changes.len() * 32);
        let mut edits = Vec::with_capacity(changes.len());
        let mut copied = 0;
        for (range, inserted) in changes {
            text.push_str(&source[copied..range.start]);
            text.push_str(&inserted);
            copied = range.end;
            edits.push(Edit { at: range.start, removed: range.len(), inserted: inserted.len() });
        }
        text.push_str(&source[copied..]);
        Self { text: text.into(), edits }
    }

    /// The Markdown the text view draws.
    pub(crate) fn text(&self) -> &SharedString {
        &self.text
    }

    /// `range` of the reply's own Markdown as a range of [`Self::text`]:
    /// what was inserted where it starts comes before it, what was
    /// inserted where it ends after it, so it never takes in a link's
    /// markup on its edges.
    pub(crate) fn map_range(&self, range: Range<usize>) -> Range<usize> {
        let start = self.map(range.start, true);
        let end = self.map(range.end, false).max(start);
        start..end
    }

    fn map(&self, at: usize, start: bool) -> usize {
        let mut mapped = at;
        for edit in &self.edits {
            let before = if edit.removed == 0 && edit.at == at {
                start
            } else {
                edit.at + edit.removed <= at
            };
            if !before {
                break;
            }
            mapped = (mapped + edit.inserted).saturating_sub(edit.removed);
        }
        mapped
    }
}

/// The links of `source`, a settled reply, to the paths it names that
/// exist on this machine under `paths`; `None` when it names none. Looks
/// at the disk: run it off the main thread.
pub(crate) fn check(source: &str, paths: &LocalPaths) -> Option<PathLinks> {
    let mut changes = Vec::new();
    for span in spans(source).into_iter().take(MAX_SPANS) {
        match span {
            Span::Code { range, content } => {
                let Some(path) = existing(&spellings(content.trim()), paths) else { continue };
                let Some(href) = href(&path) else { continue };
                changes.push((range.start..range.start, "[".to_owned()));
                changes.push((range.end..range.end, format!("]({href})")));
            }
            Span::Link { tail, address } => {
                let Some(path) = existing(&link_spellings(&address), paths) else { continue };
                let Some(href) = href(&path) else { continue };
                changes.push((tail, format!("]({href})")));
            }
        }
    }
    changes.sort_by_key(|(range, _)| range.start);
    (!changes.is_empty()).then(|| PathLinks::new(source, changes))
}

/// Whether `href` is a link to a path on this machine ([`SCHEME`]), in
/// any case.
pub(crate) fn is_path_link(href: &str) -> bool {
    url_scheme(href).is_some_and(|scheme| format!("{scheme}:").eq_ignore_ascii_case(SCHEME))
}

/// The path a link with [`SCHEME`] names, decoded, as it was written: the
/// view resolves it again (an agent may have written `~/…` or a relative
/// one).
pub(crate) fn link_path(href: &str) -> Option<String> {
    if !is_path_link(href) {
        return None;
    }
    decode(&href[SCHEME.len()..])
}

/// The first of `spellings` that names a path that exists.
fn existing(spellings: &[String], paths: &LocalPaths) -> Option<PathBuf> {
    spellings.iter().filter_map(|spelling| paths.resolve(spelling)).find(|path| path.exists())
}

/// The address of a link to `path`.
fn href(path: &Path) -> Option<String> {
    Some(format!("{SCHEME}{}", encode(path.to_str()?)))
}

/// A span of a reply that may name a path.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Span {
    /// An inline code span: its source, backticks included, and its
    /// content.
    Code { range: Range<usize>, content: String },
    /// A link: the source from the end of its text to its end (`](…)`),
    /// and its address.
    Link { tail: Range<usize>, address: String },
}

/// The spans of `source` that may name a path, in source order: inline
/// code and inline links in the text the view draws.
fn spans(source: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    if let Ok(Node::Root(root)) = markdown::to_mdast(source, &parse_options()) {
        blocks(source, &root.children, &mut spans);
    }
    spans
}

fn blocks(source: &str, nodes: &[Node], spans: &mut Vec<Span>) {
    for node in nodes {
        match node {
            Node::Paragraph(_) | Node::Heading(_) => {
                inlines(source, node.children().map_or(&[][..], Vec::as_slice), spans);
            }
            Node::Blockquote(_)
            | Node::List(_)
            | Node::ListItem(_)
            | Node::FootnoteDefinition(_) => {
                blocks(source, node.children().map_or(&[][..], Vec::as_slice), spans);
            }
            Node::Table(table) => {
                for row in &table.children {
                    for cell in row.children().into_iter().flatten() {
                        inlines(source, cell.children().map_or(&[][..], Vec::as_slice), spans);
                    }
                }
            }
            // Code blocks, math, HTML, rules and definitions.
            _ => {}
        }
    }
}

fn inlines(source: &str, nodes: &[Node], spans: &mut Vec<Span>) {
    for node in nodes {
        let range = node.position().map(|position| position.start.offset..position.end.offset);
        let Some(range) = range.filter(|range| range.end <= source.len()) else { continue };
        match node {
            // After a `!` the link it becomes would be an image.
            Node::InlineCode(code) if !source[..range.start].ends_with('!') => {
                spans.push(Span::Code { range, content: code.value.clone() });
            }
            // An inline link with text; not an autolink (`<…>`), whose
            // text is its address. Code in its text is its own.
            Node::Link(link) if source[range.clone()].starts_with('[') => {
                let text_end = link.children.last().and_then(Node::position);
                if let Some(text_end) = text_end.map(|position| position.end.offset)
                    && source[text_end..range.end].starts_with(']')
                {
                    spans.push(Span::Link { tail: text_end..range.end, address: link.url.clone() });
                }
            }
            Node::Emphasis(_) | Node::Strong(_) | Node::Delete(_) => {
                inlines(source, node.children().map_or(&[][..], Vec::as_slice), spans);
            }
            // Reference links, images, inline math and HTML.
            _ => {}
        }
    }
}

/// How a code span's path may be written: as it is, then without a
/// trailing `:line` or `:line:col`.
fn spellings(written: &str) -> Vec<String> {
    let mut spellings = vec![written.to_owned()];
    spellings.extend(without_line(written).map(str::to_owned));
    spellings
}

/// `text` without a trailing `:line` or `:line:col`, when it ends so.
fn without_line(text: &str) -> Option<&str> {
    let number = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    let (rest, last) = text.rsplit_once(':')?;
    if !number(last) {
        return None;
    }
    let head = match rest.rsplit_once(':') {
        Some((head, line)) if number(line) => head,
        _ => rest,
    };
    (!head.is_empty()).then_some(head)
}

/// How a link's address may name a path: a `file:` URL's path; a plain
/// path as written, decoded (`%20`), and without a `#fragment`, each also
/// without a trailing `:line`. Any other URL names none.
fn link_spellings(address: &str) -> Vec<String> {
    if let Some(scheme) = url_scheme(address) {
        if scheme.eq_ignore_ascii_case("file") {
            return file_url_path(address).into_iter().collect();
        }
        return Vec::new();
    }
    if address.is_empty() || address.starts_with(['#', '?']) || address.starts_with("//") {
        return Vec::new();
    }
    let mut written = vec![address.to_owned()];
    written.extend(decode(address).filter(|decoded| decoded != address));
    if let Some((path, _)) = address.split_once('#') {
        written.push(path.to_owned());
        written.extend(decode(path));
    }
    let mut spellings: Vec<String> = Vec::new();
    for text in written {
        for spelling in self::spellings(&text) {
            if !spellings.contains(&spelling) {
                spellings.push(spelling);
            }
        }
    }
    spellings
}

/// The path of a `file:` URL on this machine (no host, or `localhost`),
/// decoded.
fn file_url_path(url: &str) -> Option<String> {
    let rest = &url["file:".len()..];
    let rest = rest.split(['?', '#']).next().unwrap_or_default();
    let path = match rest.strip_prefix("//") {
        Some(after) => {
            let (host, path) = after.split_at(after.find('/')?);
            (host.is_empty() || host.eq_ignore_ascii_case("localhost")).then_some(path)?
        }
        None => rest,
    };
    path.starts_with('/').then(|| decode(path)).flatten()
}

/// The scheme `text` starts with (`https` of `https:…`), as RFC 3986 spells
/// one: a letter, then letters, digits, `+`, `-` or `.`, then a colon.
fn url_scheme(text: &str) -> Option<&str> {
    let (scheme, _) = text.split_once(':')?;
    let mut bytes = scheme.bytes();
    let first = bytes.next()?;
    (first.is_ascii_alphabetic()
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.')))
    .then_some(scheme)
}

/// `path` percent-encoded for a link's address: letters, digits, `-._~`
/// and `/` stay, every other byte is `%XX`, so the address holds nothing
/// Markdown or a table reads as markup.
fn encode(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// `text` with each `%XX` decoded; `None` when an escape is broken or the
/// bytes are not UTF-8.
fn decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let hex = text.get(at + 1..at + 3)?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            at += 3;
        } else {
            decoded.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)] // Tests write their scratch files.
mod tests {
    use super::*;

    /// A scratch home with `Downloads/报告 final.docx`, and a task folder
    /// with `notes.md` and `src/main.rs`.
    struct Scratch {
        root: PathBuf,
        paths: LocalPaths,
    }

    impl Scratch {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("maka-paths-{}", uuid::Uuid::new_v4().simple()));
            let (home, folder) = (root.join("home"), root.join("task"));
            std::fs::create_dir_all(home.join("Downloads")).expect("home");
            std::fs::create_dir_all(folder.join("src")).expect("folder");
            std::fs::write(home.join("Downloads/报告 final.docx"), b"doc").expect("docx");
            std::fs::write(folder.join("notes.md"), b"# Notes").expect("notes");
            std::fs::write(folder.join("src/main.rs"), b"fn main() {}").expect("main");
            Self { paths: LocalPaths::new(Some(home), Some(folder)), root }
        }

        fn task(&self, rest: &str) -> String {
            self.root.join("task").join(rest).to_str().expect("utf-8").to_owned()
        }

        fn home(&self, rest: &str) -> String {
            self.root.join("home").join(rest).to_str().expect("utf-8").to_owned()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.root).ok();
        }
    }

    /// The path each link of `text` names, in order.
    fn linked(text: &str) -> Vec<String> {
        let Ok(Node::Root(root)) = markdown::to_mdast(text, &parse_options()) else {
            panic!("parse");
        };
        let mut found = Vec::new();
        collect_links(&Node::Root(root), &mut found);
        found
    }

    fn collect_links(node: &Node, found: &mut Vec<String>) {
        if let Node::Link(link) = node {
            found.push(link_path(&link.url).unwrap_or_else(|| format!("other: {}", link.url)));
        }
        for child in node.children().into_iter().flatten() {
            collect_links(child, found);
        }
    }

    #[test]
    fn paths_resolve_absolute_in_the_home_or_in_the_task_folder() {
        let paths = LocalPaths::new(Some("/Users/me".into()), Some("/work/task".into()));
        assert_eq!(paths.resolve("/etc/hosts"), Some("/etc/hosts".into()));
        assert_eq!(paths.resolve("~/Downloads/a.md"), Some("/Users/me/Downloads/a.md".into()));
        assert_eq!(paths.resolve("src/main.rs"), Some("/work/task/src/main.rs".into()));
        assert_eq!(paths.resolve("ls ~/Downloads"), Some("/work/task/ls ~/Downloads".into()));
        for none in ["", "https://example.com/a.md", "file:///etc/hosts", "a\nb"] {
            assert_eq!(paths.resolve(none), None, "{none:?}");
        }
        let elsewhere = LocalPaths::new(None, None);
        assert_eq!(elsewhere.resolve("~/a.md"), None);
        assert_eq!(elsewhere.resolve("a.md"), None);
    }

    #[test]
    fn spans_that_name_existing_paths_become_links() {
        let scratch = Scratch::new();
        let absolute = scratch.task("notes.md");
        let reply = format!(
            "**交付文件**：`~/Downloads/报告 final.docx`\n\n\
             See `{absolute}`, ` src/main.rs:12:3 `, and [the notes](notes.md \"Notes\").\n\n\
             | file | |\n|---|---|\n| `src` | [plan](file://{absolute}) |\n"
        );
        let links = check(&reply, &scratch.paths).expect("links");
        assert_eq!(
            linked(links.text()),
            [
                scratch.home("Downloads/报告 final.docx"),
                absolute.clone(),
                scratch.task("src/main.rs"),
                scratch.task("notes.md"),
                scratch.task("src"),
                absolute.clone(),
            ]
        );
        assert!(
            links.text().contains("[`~/Downloads/报告 final.docx`](maka-file:/"),
            "the span stays a code chip: {}",
            links.text()
        );
    }

    #[test]
    fn commands_missing_paths_urls_and_code_blocks_stay_text() {
        let scratch = Scratch::new();
        let reply = format!(
            "Run `ls ~/Downloads`, open `missing/report.md` or `https://example.com/notes.md`.\n\n\
             ```\n{}\n```\n\n\
             A bare {} in prose, [a page](https://example.com), \
             [`notes.md`](https://example.com/notes) and Done!`notes.md`\n",
            scratch.task("notes.md"),
            scratch.task("notes.md"),
        );
        assert!(check(&reply, &scratch.paths).is_none());
        // A Host elsewhere: nothing resolves.
        let relative = "Open `notes.md`.";
        assert!(check(relative, &scratch.paths).is_some());
        assert!(check(relative, &LocalPaths::default()).is_none());
    }

    #[test]
    fn a_match_in_the_reply_maps_into_its_links() {
        let scratch = Scratch::new();
        let reply = "Read `notes.md` now, then [this](notes.md).";
        let links = check(reply, &scratch.paths).expect("links");
        let text = links.text().to_string();
        let source_of = |word: &str| {
            let start = reply.find(word).expect("word");
            start..start + word.len()
        };
        for (word, expected) in [("notes", "notes"), ("Read", "Read"), ("now, then", "now, then")] {
            let mapped = links.map_range(source_of(word));
            assert_eq!(&text[mapped], expected, "{word}");
        }
        // A match that ends at the span's backtick takes in none of the
        // link's markup on its edges.
        let edge = source_of("Read ");
        assert_eq!(&text[links.map_range(edge)], "Read ");
        let this = reply.rfind("this").expect("this");
        assert_eq!(&text[links.map_range(this..this + 4)], "this");
        assert_eq!(&text[links.map_range(reply.len() - 1..reply.len())], ".");
    }

    #[test]
    fn link_addresses_round_trip_and_other_schemes_are_not_paths() {
        let path = "/Users/me/Downloads/麦当劳 (MCD) 100%|a#b.docx";
        let href = href(Path::new(path)).expect("href");
        assert!(href.bytes().all(|byte| byte.is_ascii_graphic()), "{href}");
        assert!(!href.contains(['|', '(', ')', ' ', '#']));
        assert_eq!(link_path(&href).as_deref(), Some(path));
        assert_eq!(link_path("MAKA-FILE:~/a.md").as_deref(), Some("~/a.md"));
        assert_eq!(link_path("maka-file:%zz"), None);
        for other in ["https://example.com", "file:///etc/hosts", "maka-files:/x", "notes.md"] {
            assert!(!is_path_link(other), "{other}");
        }
        assert_eq!(file_url_path("file:///a%20b/c.md#x").as_deref(), Some("/a b/c.md"));
        assert_eq!(file_url_path("file://localhost/a.md").as_deref(), Some("/a.md"));
        assert_eq!(file_url_path("file://server/a.md"), None);
        assert_eq!(without_line("a.rs:12:3"), Some("a.rs"));
        assert_eq!(without_line("a.rs:12"), Some("a.rs"));
        assert_eq!(without_line("a.rs"), None);
        assert_eq!(without_line(":12"), None);
    }
}
