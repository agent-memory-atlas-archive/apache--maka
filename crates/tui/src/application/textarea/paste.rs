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

//! One physically admitted batch, read serially through the original file custody.
use super::super::{NativeContext, NativeInput, file::paths};
use maka_ui::{
    budget::{Kind, Permit},
    file::{FileCapture, FileError, FilePhase, FileSelection, FileSnapshot, read_selected},
    textarea::{FilePasteAction, FilePasteCommand, FilePasteObservation, FilePastePolicy},
};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, mpsc},
};

struct Batch {
    context: String,
    paths: VecDeque<PathBuf>,
    staged: bool,
    _memory: Permit,
}
struct Read {
    revision: u64,
    receiver: mpsc::Receiver<Result<Arc<FileSnapshot>, FileError>>,
}
pub(super) struct Paste {
    context: NativeContext,
    selection: FileSelection,
    batch: Option<Batch>,
    read: Option<Read>,
    dirty: bool,
}
impl Paste {
    pub fn pending(&self) -> bool {
        self.batch.is_some() || self.read.is_some()
    }
    pub fn new(context: NativeContext) -> Self {
        Self {
            context,
            selection: FileSelection::default(),
            batch: None,
            read: None,
            dirty: false,
        }
    }
    /// The whole paste must classify as local paths. Otherwise native text keeps it.
    pub fn stage(&mut self, text: &str, policy: &FilePastePolicy) -> Option<NativeInput> {
        if self.batch.is_some() || self.read.is_some() {
            return None;
        }
        let paths = paths::parse(text)?;
        let mut unique = VecDeque::new();
        for path in paths {
            if !unique.contains(&path) {
                if unique.len() == policy.max_files {
                    return None;
                }
                unique.push_back(path);
            }
        }
        if unique.is_empty() || unique.len() > policy.max_files {
            return None;
        }
        let memory = self
            .context
            .budget
            .reserve(
                Kind::Encoded,
                unique.capacity() * std::mem::size_of::<PathBuf>()
                    + unique.iter().map(PathBuf::capacity).sum::<usize>()
                    + policy.context.len(),
            )
            .ok()?;
        let choose = self.selection.stage()?;
        self.batch = Some(Batch {
            context: policy.context.clone(),
            paths: unique,
            staged: true,
            _memory: memory,
        });
        Some(NativeInput {
            key: "file_choose".into(),
            value: serde_json::to_value(choose).unwrap(),
        })
    }
    pub fn suspend(&mut self) {
        self.selection.cancel_intent();
        if self.batch.as_ref().is_some_and(|batch| batch.staged) {
            self.batch = None;
        }
    }
    pub fn commit(
        &mut self,
        request: Option<&str>,
        command: Option<&FilePasteCommand>,
        enabled: bool,
    ) {
        if !enabled {
            self.suspend();
        }
        if self.selection.accept(request) {
            self.batch
                .as_mut()
                .expect("admitted original file paste")
                .staged = false;
            self.start();
            self.dirty = true;
        }
        let Some(command) = command else { return };
        let observation = self.selection.observation();
        if self.batch.is_none()
            || command.selection.owner != observation.owner
            || command.selection.revision != observation.revision
        {
            return;
        }
        match command.action {
            FilePasteAction::Cancel => {
                self.batch.as_mut().unwrap().paths.clear();
                self.selection.clear();
            }
            FilePasteAction::Next
                if matches!(observation.phase, FilePhase::Ready | FilePhase::Failed) =>
            {
                self.batch.as_mut().unwrap().paths.pop_front();
                self.selection.clear();
                if !self.batch.as_ref().unwrap().paths.is_empty() {
                    self.advance();
                }
            }
            FilePasteAction::Retry if observation.phase == FilePhase::Failed => self.advance(),
            _ => return,
        }
        self.dirty = true;
    }
    fn advance(&mut self) {
        // A committed disposition can only continue paths from the original
        // physical batch. No guest command can supply or substitute a path.
        let choose = self.selection.stage().expect("settled file selection");
        assert!(self.selection.accept(Some(&choose.request)));
        self.start();
    }
    fn start(&mut self) {
        let path = self.batch.as_ref().unwrap().paths.front().unwrap().clone();
        let job = match self.context.budget.reserve(Kind::Job, 1) {
            Ok(job) => job,
            Err(error) => {
                self.selection.settle(Err(error.into()));
                return;
            }
        };
        let mut ticket = match self.context.resources.reserve() {
            Ok(ticket) => ticket,
            Err(_) => {
                self.selection.settle(Err(FileError::WorkerFailed));
                return;
            }
        };
        let budget = self.context.budget.clone();
        let revision = self.selection.observation().revision;
        let (sender, receiver) = mpsc::sync_channel(1);
        match std::thread::Builder::new()
            .name("native-file-paste".into())
            .spawn(move || {
                ticket.start();
                let (result, cleanup) =
                    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        read_selected(&path, &budget)
                    })) {
                        Ok(result) => (result, Ok(())),
                        Err(_) => (
                            Err(FileError::WorkerFailed),
                            Err("native file paste worker panicked".into()),
                        ),
                    };
                drop(job);
                let _ = sender.send(result);
                ticket.complete(cleanup);
            }) {
            Ok(_) => self.read = Some(Read { revision, receiver }),
            Err(_) => self.selection.settle(Err(FileError::WorkerFailed)),
        }
    }
    pub fn observe(&mut self) -> Option<NativeInput> {
        if let Some(read) = &self.read {
            let result = match read.receiver.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(FileError::WorkerFailed)),
                Err(mpsc::TryRecvError::Empty) => None,
            };
            if let Some(result) = result {
                let revision = read.revision;
                self.read = None;
                if self.selection.observation().revision == revision {
                    self.selection.settle(result.map(Some));
                    self.dirty = true;
                }
            }
        }
        let batch = self.batch.as_ref()?;
        if !self.dirty || batch.staged {
            return None;
        }
        let observation = FilePasteObservation {
            context: batch.context.clone(),
            selection: self.selection.observation(),
            remaining: batch.paths.len(),
        };
        self.dirty = false;
        if batch.paths.is_empty() {
            self.batch = None;
        }
        Some(NativeInput {
            key: "file_observe".into(),
            value: serde_json::to_value(observation).unwrap(),
        })
    }
    pub fn capture(
        &self,
        input: &serde_json::Value,
    ) -> maka_ui::Result<Arc<dyn maka_plugins::call::ReadResource>> {
        let capture: FileCapture = serde_json::from_value(input.clone())
            .map_err(|error| maka_ui::Error::Invalid(error.to_string()))?;
        self.selection.capture(&capture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maka_ui::budget::{BudgetLimits, NativeBudget};
    fn settled(paste: &mut Paste) -> FilePasteObservation {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(input) = paste.observe() {
                let observed: FilePasteObservation = serde_json::from_value(input.value).unwrap();
                if observed.selection.phase != FilePhase::Choosing {
                    return observed;
                }
            }
            assert!(
                std::time::Instant::now() < deadline,
                "original worker did not settle"
            );
            std::thread::yield_now();
        }
    }
    fn command(observed: &FilePasteObservation, action: FilePasteAction) -> FilePasteCommand {
        FilePasteCommand {
            selection: FileCapture {
                owner: observed.selection.owner.clone(),
                revision: observed.selection.revision,
            },
            action,
        }
    }
    #[test]
    fn original_batch_advances_only_by_revision_and_retains_capture_budget_until_release() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first.txt");
        let second = directory.path().join("second.txt");
        let bytes = vec![b'a'; 65537];
        std::fs::write(&first, &bytes).unwrap();
        let resources = Arc::new(maka_plugins::call::Resources::default());
        let budget = NativeBudget::new(BudgetLimits {
            encoded_bytes: 2 * 65536 + 16384,
            ..Default::default()
        });
        let mut paste = Paste::new(NativeContext {
            resources: resources.clone(),
            budget: budget.clone(),
        });
        let policy = FilePastePolicy {
            context: "original-session".into(),
            max_files: 2,
        };
        let staged = paste
            .stage(
                &format!(
                    "\"{}\" \"{}\" \"{}\"",
                    first.display(),
                    first.display(),
                    second.display()
                ),
                &policy,
            )
            .unwrap();
        assert!(
            !staged
                .value
                .to_string()
                .contains(directory.path().to_str().unwrap())
        );
        assert!(paste.read.is_none());
        paste.commit(Some("forged"), None, true);
        assert!(paste.read.is_none());
        paste.commit(staged.value["request"].as_str(), None, true);
        let observed = settled(&mut paste);
        assert_eq!(observed.remaining, 2, "duplicate native paths use one slot");
        assert_eq!(observed.context, "original-session");
        assert_eq!(observed.selection.phase, FilePhase::Ready);
        let capture = paste
            .capture(
                &serde_json::to_value(&command(&observed, FilePasteAction::Next).selection)
                    .unwrap(),
            )
            .unwrap();
        std::fs::remove_file(&first).unwrap();
        assert_eq!(capture.read(0..32).unwrap(), bytes[..32]);
        let mut invalid = command(&observed, FilePasteAction::Next);
        invalid.selection.owner = "another-owner".into();
        paste.commit(None, Some(&invalid), true);
        assert_eq!(paste.selection.observation(), observed.selection);
        // The later file did not exist when this batch was first admitted.
        std::fs::write(&second, vec![b'b'; 65537]).unwrap();
        let next = command(&observed, FilePasteAction::Next);
        paste.commit(None, Some(&next), true);
        let failed = settled(&mut paste);
        assert_eq!(failed.selection.error, Some(FileError::BudgetExhausted));
        assert_eq!(failed.remaining, 1);
        assert!(
            paste
                .capture(&serde_json::to_value(&next.selection).unwrap())
                .is_err()
        );
        drop(capture);
        paste.commit(None, Some(&command(&failed, FilePasteAction::Retry)), true);
        let second = settled(&mut paste);
        assert_eq!(second.selection.phase, FilePhase::Ready);
        let held = paste
            .capture(
                &serde_json::to_value(&command(&second, FilePasteAction::Next).selection).unwrap(),
            )
            .unwrap();
        assert_eq!(held.read(0..4).unwrap(), b"bbbb");
        paste.commit(None, Some(&next), true);
        assert_eq!(
            paste.selection.observation(),
            second.selection,
            "old disposition cannot consume the next item"
        );
        paste.commit(None, Some(&command(&second, FilePasteAction::Next)), true);
        assert_eq!(settled(&mut paste).remaining, 0);
        assert_eq!(budget.usage().0, 2 * 65536);
        drop(held);
        assert_eq!(budget.usage(), (0, 0, 0));
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(resources.finish())
            .unwrap();
    }
    #[test]
    fn uncommitted_or_cancelled_paste_cannot_read_or_publish_a_late_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("file.txt");
        std::fs::write(&path, b"private").unwrap();
        let resources = Arc::new(maka_plugins::call::Resources::default());
        let budget = NativeBudget::default();
        let mut paste = Paste::new(NativeContext {
            resources: resources.clone(),
            budget: budget.clone(),
        });
        let policy = FilePastePolicy {
            context: "session".into(),
            max_files: 1,
        };
        let staged = paste.stage(path.to_str().unwrap(), &policy).unwrap();
        paste.suspend();
        paste.commit(staged.value["request"].as_str(), None, true);
        assert!(paste.read.is_none());
        assert_eq!(budget.usage(), (0, 0, 0));
        let staged = paste.stage(path.to_str().unwrap(), &policy).unwrap();
        paste.commit(staged.value["request"].as_str(), None, true);
        let choosing = FilePasteObservation {
            context: "session".into(),
            selection: paste.selection.observation(),
            remaining: 1,
        };
        paste.commit(
            None,
            Some(&command(&choosing, FilePasteAction::Cancel)),
            false,
        );
        assert_eq!(settled(&mut paste).remaining, 0);
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(resources.finish())
            .unwrap();
        assert!(
            paste.observe().is_none(),
            "late worker cannot restore a cancelled batch"
        );
        assert_eq!(budget.usage(), (0, 0, 0));
    }
}
