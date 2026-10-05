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

use super::NativeInput;
use crate::editor::{Editor, Geometry};
use maka_ui::editor::*;
use serde_json::{Value, json};
use std::{ops::Range, sync::Arc};
pub(super) struct Owner {
    pub editor: Editor,
    pub presented: Option<Geometry>,
    id: String,
    revision: u64,
    last_edit: u64,
    last_selection: (Range<usize>, bool),
    pub last_request: Option<EditorCommand>,
    sequence: u64,
    receipt: Option<EditorReceipt>,
    pub announced: bool,
}
impl Owner {
    pub fn invalidate_geometry(&mut self) {
        self.editor.invalidate_geometry();
        self.presented = None;
    }
    pub fn new(mut editor: Editor, initial_text: &str) -> Self {
        editor.insert(initial_text);
        editor.clear_history();
        let last_edit = editor.revision();
        let last_selection = (editor.selection_range(), editor.selection_reversed());
        Self {
            editor,
            presented: None,
            id: uuid::Uuid::new_v4().to_string(),
            revision: 0,
            last_edit,
            last_selection,
            last_request: None,
            sequence: 0,
            receipt: None,
            announced: false,
        }
    }
    pub fn refresh(&mut self) -> bool {
        let edit = self.editor.revision();
        let selection = (
            self.editor.selection_range(),
            self.editor.selection_reversed(),
        );
        if edit != self.last_edit || selection != self.last_selection {
            self.last_edit = edit;
            self.last_selection = selection;
            self.revision += 1;
            return true;
        }
        false
    }
    pub fn observation(&self) -> NativeInput {
        let range = self.editor.selection_range();
        let selection = TextRange {
            start: self.editor.text()[..range.start].encode_utf16().count(),
            end: self.editor.text()[..range.end].encode_utf16().count(),
        };
        let observation = EditorObservation {
            revision: self.revision,
            snapshot: EditorSnapshot {
                text: EditorTextRef {
                    owner: self.id.clone(),
                    revision: self.revision,
                    bytes: self.editor.text().len(),
                },
                selection,
                reversed: self.editor.selection_reversed(),
                marked: None,
            },
            receipt: self.receipt.clone(),
        };
        NativeInput {
            key: "observe".into(),
            value: serde_json::to_value(observation).expect("native editor observation"),
        }
    }
    pub fn apply(&mut self, command: &EditorCommand) {
        self.last_request = Some(command.clone());
        let outcome = if command.owner != self.id {
            EditorOutcome::Conflict {
                reason: EditorConflict::Owner,
            }
        } else if command.sequence <= self.sequence {
            EditorOutcome::Conflict {
                reason: EditorConflict::Sequence,
            }
        } else {
            self.sequence = command.sequence;
            if command.based_on != self.revision {
                EditorOutcome::Conflict {
                    reason: EditorConflict::Revision,
                }
            } else {
                let (EditorEdit::Replace { text } | EditorEdit::Reset { text }) = &command.edit;
                if !representable(text) || !self.editor.accepts_text(text) {
                    EditorOutcome::Conflict {
                        reason: EditorConflict::Representation,
                    }
                } else {
                    match &command.edit {
                        EditorEdit::Replace { text } => {
                            self.editor.select_all();
                            self.editor.insert(text);
                        }
                        EditorEdit::Reset { text } => {
                            self.editor.clear();
                            self.editor.insert(text);
                            self.editor.clear_history();
                        }
                    }
                    self.refresh();
                    EditorOutcome::Applied
                }
            }
        };
        self.receipt = Some(EditorReceipt {
            owner: command.owner.clone(),
            sequence: command.sequence,
            revision: self.revision,
            outcome,
        });
    }
}
impl Owner {
    pub fn capture(
        &self,
        target: &maka_ui::actions::CaptureTarget,
    ) -> maka_ui::Result<Arc<dyn maka_plugins::call::ReadResource>> {
        if target.method() != "snapshot" {
            return Err(maka_ui::Error::Invalid(
                "unknown terminal editor capture".into(),
            ));
        }
        let owner = self;
        let capture: EditorCapture = serde_json::from_value(target.input().clone())
            .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
        if matches!(capture,EditorCapture::AtRevision {owner:ref expected,revision} if expected!=&owner.id || revision!=owner.revision)
        {
            return Err(maka_ui::Error::Invalid("editor revision changed".into()));
        }
        Ok(Arc::new(Snapshot {
            owner: owner.id.clone(),
            text: owner.editor.text().to_owned(),
            revision: owner.revision,
        }))
    }
}

fn representable(text: &str) -> bool {
    text.len() <= 65536
        && !text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
}
pub(super) struct Snapshot {
    pub owner: String,
    pub text: String,
    pub revision: u64,
}
impl maka_plugins::call::ReadResource for Snapshot {
    fn encoding(&self) -> maka_plugins::call::ResourceEncoding {
        maka_plugins::call::ResourceEncoding::Utf8
    }
    fn resource_type(&self) -> maka_plugin_core::contract::ResourceType {
        text_resource_type()
    }
    fn bytes(&self) -> usize {
        self.text.len()
    }
    fn metadata(&self) -> Value {
        json!({"owner":self.owner,"revision":self.revision,"bytes":self.text.len()})
    }
    fn read(&self, range: Range<usize>) -> Result<Vec<u8>, String> {
        if range
            .end
            .checked_sub(range.start)
            .is_none_or(|n| n > 64 * 1024)
        {
            return Err("invalid text range or chunk exceeds 64 KiB".into());
        }
        self.text
            .get(range)
            .map(|text| text.as_bytes().to_vec())
            .ok_or_else(|| "invalid editor UTF-8 range".into())
    }
}
