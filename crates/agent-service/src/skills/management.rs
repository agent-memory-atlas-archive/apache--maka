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

use super::*;
use cap_fs_ext::DirExt;
use maka_event_log::root::RootOwner;
use maka_skills::publication::{
    self, Publisher, Tree,
    receipt::{Destination, Operation, Outcome},
};
use tokio_util::sync::CancellationToken;

pub(super) struct Managed {
    root: Arc<RootOwner>,
    namespace: String,
    data: cap_std::fs::Dir,
    pub read: ReadRoot,
    serial: tokio::sync::Mutex<()>,
}
impl Managed {
    pub async fn open(root: Arc<RootOwner>, namespace: String) -> Result<Arc<Self>, String> {
        let (managed, publisher) = tokio::task::spawn_blocking(move || {
            root.validate_current().map_err(store_error)?;
            let directory = cap_std::fs::Dir::open_ambient_dir(
                root.canonical_path(),
                cap_std::ambient_authority(),
            )
            .map_err(store_error)?;
            root.validate_directory_handle(
                &directory.try_clone().map_err(store_error)?.into_std_file(),
            )
            .map_err(store_error)?;
            let parent = child(&directory, "skill-libraries").map_err(store_error)?;
            let data = child(&parent, &namespace).map_err(store_error)?;
            let publisher =
                Publisher::open(&data, &CancellationToken::new()).map_err(store_error)?;
            let read = ReadRoot::from_handle(
                data.try_clone().map_err(store_error)?,
                root.canonical_path()
                    .join("skill-libraries")
                    .join(&namespace),
            )
            .select(BTreeSet::from(["skills/".into()]))
            .map_err(store_error)?;
            Ok::<_, String>((
                Arc::new(Self {
                    root,
                    namespace,
                    data,
                    read,
                    serial: Default::default(),
                }),
                publisher,
            ))
        })
        .await
        .map_err(store_error)??;
        publisher.recover().await.map_err(store_error)?;
        Ok(managed)
    }
    async fn publisher(self: &Arc<Self>, scope: &Scope) -> Result<Publisher, String> {
        let managed = self.clone();
        let scope = scope.clone();
        let mut ticket = scope.resources.reserve().map_err(store_error)?;
        tokio::task::spawn_blocking(move || {
            ticket.start();
            let result = managed
                .root
                .validate_current()
                .map_err(store_error)
                .and_then(|()| Publisher::open_scoped(&managed.data, &scope).map_err(store_error));
            drop(managed);
            ticket.complete(Ok(()));
            result
        })
        .await
        .map_err(store_error)?
    }
}
fn child(parent: &cap_std::fs::Dir, name: &str) -> std::io::Result<cap_std::fs::Dir> {
    match parent.create_dir(name) {
        Ok(()) => {
            #[cfg(unix)]
            parent.open(".")?.sync_all()?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    parent.open_dir_nofollow(name)
}
fn store_error(_: impl std::fmt::Display) -> String {
    "skills_store_unavailable".into()
}

struct Plan {
    review: Review,
    id: String,
    expected: Option<Tree>,
    next: Option<Tree>,
    locally_modified: bool,
}
impl Plan {
    fn preview(self) -> Preview {
        let complete = self
            .next
            .as_ref()
            .or(self.expected.as_ref())
            .map_or_else(Vec::new, Tree::paths);
        let entries = complete.len();
        let mut paths = Vec::new();
        for path in complete {
            paths.push(path);
            if serde_json::to_vec(&paths)
                .expect("path strings serialize")
                .len()
                > 24 * 1024
            {
                paths.pop();
                break;
            }
        }
        Preview::Ready {
            target: format!("installed:{}", self.id),
            entries,
            paths_truncated: paths.len() != entries,
            paths,
            locally_modified: self.locally_modified,
            review: self.review,
        }
    }
}
enum Prepared {
    Ready(Box<Plan>),
    Changed,
    Rejected(&'static str),
}
impl Library {
    async fn prepare(
        &self,
        change: Change,
        publisher: &Publisher,
        scope: &Scope,
    ) -> Result<Prepared, String> {
        let rejected = |reason| Ok(Prepared::Rejected(reason));
        let snapshot = self.snapshot(scope).await?;
        if snapshot.revision != change.revision {
            return Ok(Prepared::Changed);
        }
        let Some((entry, digest, _)) = snapshot
            .entries
            .iter()
            .find(|(entry, _, _)| entry.reference == change.reference)
        else {
            return rejected("not_found");
        };
        let (id, source) = match change.kind {
            ChangeKind::Delete => {
                let Some(id) = change
                    .reference
                    .strip_prefix("installed:")
                    .filter(|id| maka_skills::safe_source_id(id))
                else {
                    return rejected("read_only_source");
                };
                (id.to_owned(), None)
            }
            ChangeKind::Install | ChangeKind::Update => {
                if !entry.valid {
                    return rejected("invalid_skill");
                }
                let Some((id, root)) = self.sources.iter().find_map(|(source, root)| {
                    change
                        .reference
                        .strip_prefix(&format!("source:{}:", source.id))
                        .filter(|id| maka_skills::safe_source_id(id))
                        .map(|id| (id, root))
                }) else {
                    return rejected("read_only_source");
                };
                let tree = Tree::read_view(&root.bind_scope(scope), id)
                    .await
                    .map_err(store_error)?;
                if tree
                    .get("SKILL.md")
                    .map(maka_runtime::artifact::content_digest)
                    .as_ref()
                    != Some(digest)
                {
                    return Ok(Prepared::Changed);
                }
                (id.to_owned(), Some(tree))
            }
        };
        // Match the original installation catalog's case-insensitive occupancy
        // rule on both case-sensitive and case-insensitive filesystems.
        if matches!(change.kind, ChangeKind::Install)
            && snapshot.entries.iter().any(|(entry, _, _)| {
                entry
                    .reference
                    .strip_prefix("installed:")
                    .is_some_and(|installed| installed.eq_ignore_ascii_case(&id))
            })
        {
            return rejected("already_installed");
        }
        let expected = publisher
            .capture(&id, &scope.cancellation)
            .await
            .map_err(store_error)?;
        if let Some(tree) = &expected {
            let current = snapshot
                .entries
                .iter()
                .find(|(entry, _, _)| entry.reference == format!("installed:{id}"));
            if current.is_none_or(|(_, digest, _)| {
                tree.get("SKILL.md")
                    .map(maka_runtime::artifact::content_digest)
                    .as_ref()
                    != Some(digest)
            }) {
                return Ok(Prepared::Changed);
            }
        }
        match change.kind {
            ChangeKind::Install if expected.is_some() => return rejected("already_installed"),
            ChangeKind::Update | ChangeKind::Delete if expected.is_none() => {
                return rejected("not_installed");
            }
            _ => {}
        }
        let locally_modified = expected
            .as_ref()
            .is_some_and(|tree| tree.get("SKILL.md") != tree.get(".maka/baseline/SKILL.md"));
        let digest = maka_runtime::artifact::content_digest(
            &serde_json::to_vec(&(
                &change,
                expected
                    .as_ref()
                    .map(Tree::digest)
                    .transpose()
                    .map_err(store_error)?,
                source
                    .as_ref()
                    .map(Tree::digest)
                    .transpose()
                    .map_err(store_error)?,
            ))
            .map_err(store_error)?,
        );
        let next = match (&change.kind, source) {
            (ChangeKind::Delete, _) => None,
            (_, Some(source)) => {
                let content = source
                    .get("SKILL.md")
                    .ok_or("skills_source_unavailable")?
                    .to_vec();
                let mut tree = if matches!(change.kind, ChangeKind::Update) {
                    expected.as_ref().unwrap().clone()
                } else {
                    source
                };
                publication::set_installation(
                    &mut tree,
                    &id,
                    &id,
                    maka_skills::api::InstallSource::Managed,
                    content,
                )
                .map_err(store_error)?;
                Some(tree)
            }
            _ => unreachable!("installation source was captured"),
        };
        Ok(Prepared::Ready(Box::new(Plan {
            review: Review { change, digest },
            id,
            expected,
            next,
            locally_modified,
        })))
    }
    pub(super) async fn preview(&self, input: Change, scope: &Scope) -> Result<Preview, String> {
        let Some(managed) = &self.managed else {
            return Ok(Preview::Rejected {
                reason: "management_disabled".into(),
            });
        };
        let _serial = tokio::select! {
            biased;
            _ = scope.cancellation.cancelled() => return Err("skills_cancelled".into()),
            guard = managed.serial.lock() => guard,
        };
        let publisher = managed.publisher(scope).await?;
        Ok(match self.prepare(input, &publisher, scope).await? {
            Prepared::Ready(plan) => plan.preview(),
            Prepared::Changed => Preview::Changed,
            Prepared::Rejected(reason) => Preview::Rejected {
                reason: reason.into(),
            },
        })
    }
    pub(super) async fn apply(
        &self,
        input: Review,
        reference: &maka_plugin_core::operation::RecoveryRef,
        scope: &Scope,
    ) -> Result<ChangeResult, String> {
        let operation = operation(reference)?;
        self.apply_operation(input, &operation, scope).await
    }
    pub(super) async fn outcome(
        &self,
        reference: maka_plugin_core::operation::RecoveryRef,
        context: &Context,
    ) -> Result<contract::Outcome, String> {
        use maka_plugin_core::operation::{OperationReply, Receipt};
        reference.validate().map_err(store_error)?;
        let managed = self.managed.as_ref().ok_or("management_disabled")?;
        let current = contract();
        let compatible = match (&reference.contract.interface, &current.interface) {
            (
                maka_plugin_core::contract::Interface::Service { methods: original },
                maka_plugin_core::contract::Interface::Service { methods: current },
            ) => original.get("apply") == current.get("apply"),
            _ => false,
        };
        if reference.export != EXPORT
            || reference.method != "apply"
            || reference.contract.name != current.name
            || reference.contract.version != current.version
            || reference.contract.transport != current.transport
            || !compatible
            || crate::owner::namespace(
                &reference.source.state_domain,
                reference.source.deployment,
                &reference.namespace,
            ) != managed.namespace
        {
            return Err("skills_original_owner_mismatch".into());
        }
        // The reference and current method grant do not authorize historical
        // reads. Preserve the actual caller Scope and independent recovery policy.
        let observation = context.recover(reference.clone())?.await?;
        let (receipt, published) = match observation {
            None => (Receipt::NotRecorded, None),
            Some(observation) => {
                let scope = context.scope();
                let _serial = tokio::select! {
                    biased;
                    _ = scope.cancellation.cancelled() => return Err("skills_cancelled".into()),
                    guard = managed.serial.lock() => guard,
                };
                let publisher = managed.publisher(scope).await?;
                let published = publisher
                    .outcome(&operation(&reference)?)
                    .await
                    .map_err(store_error)?
                    .map(recorded)
                    .transpose()?;
                (
                    Receipt::Recorded {
                        outcome: observation.outcome,
                        cleanup: observation.cleanup,
                    },
                    published,
                )
            }
        };
        Ok(contract::Outcome {
            original: OperationReply { reference, receipt },
            published,
        })
    }
    async fn apply_operation(
        &self,
        input: Review,
        operation: &Operation,
        scope: &Scope,
    ) -> Result<ChangeResult, String> {
        let Some(managed) = &self.managed else {
            return Ok(ChangeResult::Rejected {
                reason: "management_disabled".into(),
            });
        };
        let _serial = tokio::select! {
            biased;
            _ = scope.cancellation.cancelled() => return Err("skills_cancelled".into()),
            guard = managed.serial.lock() => guard,
        };
        let publisher = managed.publisher(scope).await?;
        if let Some(outcome) = publisher.reserve(operation).await.map_err(store_error)? {
            return recorded(outcome);
        }
        publisher.recover().await.map_err(store_error)?;
        if let Some(outcome) = publisher.reserve(operation).await.map_err(store_error)? {
            return recorded(outcome);
        }
        let prepared = self.prepare(input.change, &publisher, scope).await?;
        let outcome = match prepared {
            Prepared::Ready(plan) if plan.review.digest == input.digest => {
                let destination = Destination::Skill {
                    reference: format!("installed:{}", plan.id),
                };
                match publisher
                    .publish_operation(
                        &plan.id,
                        plan.expected.as_ref(),
                        plan.next.as_ref(),
                        Some((operation, destination.clone())),
                        &scope.cancellation,
                    )
                    .await
                {
                    Ok(()) => Outcome::Applied { destination },
                    Err(publication::Error::Conflict) => Outcome::Conflict,
                    Err(error) => return Err(store_error(error)),
                }
            }
            Prepared::Changed | Prepared::Ready(_) => Outcome::Conflict,
            Prepared::Rejected(reason) => Outcome::Rejected {
                reason: reason.into(),
            },
        };
        publisher
            .complete(operation, outcome.clone())
            .await
            .map_err(store_error)?;
        recorded(outcome)
    }
}
fn operation(reference: &maka_plugin_core::operation::RecoveryRef) -> Result<Operation, String> {
    Ok(Operation {
        id: reference.operation,
        // Preserve the exact Host fingerprint in the publication domain’s digest format.
        fingerprint: maka_runtime::artifact::content_digest(reference.fingerprint.as_bytes()),
        binding: maka_runtime::artifact::content_digest(
            &serde_json::to_vec(reference).map_err(store_error)?,
        ),
    })
}
fn recorded(outcome: Outcome) -> Result<ChangeResult, String> {
    match outcome {
        Outcome::Applied {
            destination: Destination::Skill { reference },
        } => Ok(ChangeResult::Applied { reference }),
        Outcome::Conflict => Ok(ChangeResult::Changed),
        Outcome::Rejected { reason } => Ok(ChangeResult::Rejected { reason }),
        _ => Err("skills_receipt_invalid".into()),
    }
}

#[cfg(test)]
mod tests;
