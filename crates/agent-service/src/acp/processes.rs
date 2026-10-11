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
use crate::processes::protocol;
use maka_plugin_core::runtime::Activation;
use maka_plugins::{
    call::{Attachments, Resources, Scope},
    process::{self, Chunk, Command, Error, Exit, Handle, Id, Process, Stream},
};
use maka_runtime_host::application::NativeRequest;
use std::sync::Mutex;
use tokio::sync::{mpsc, oneshot, watch};

pub(super) struct Processes {
    calls: Arc<calls::Calls>,
    configuration: Configuration,
    entries: Mutex<BTreeMap<String, Arc<Entry>>>,
    drain: CancellationToken,
    workers: Arc<Resources>,
}
struct Entry {
    id: Uuid,
    consumer: Activation,
    session: String,
    running: Arc<Running>,
}
struct Running {
    request: Arc<NativeRequest>,
    chunks: tokio::sync::Mutex<mpsc::Receiver<Chunk>>,
    done: watch::Receiver<Option<Arc<Done>>>,
}
struct Done {
    exit: Result<Exit, String>,
    cleanup: Result<(), String>,
}
impl Processes {
    pub fn new(calls: Arc<calls::Calls>, configuration: Configuration) -> Self {
        Self {
            calls,
            configuration,
            entries: Mutex::default(),
            drain: CancellationToken::new(),
            workers: Arc::default(),
        }
    }
    pub async fn close_all(&self) -> Result<(), String> {
        self.drain.cancel();
        let entries: Vec<_> = self.entries.lock().unwrap().values().cloned().collect();
        let mut result = Ok(());
        for entry in entries {
            if let Err(error) = entry.running.close().await {
                result = Err(error);
            }
        }
        if let Err(error) = self.workers.finish().await {
            result = Err(error.to_string());
        }
        if result.is_ok() {
            self.entries.lock().unwrap().clear();
        }
        result
    }
    fn bind(&self, scope: Scope, entry: Arc<Entry>) -> Result<Handle, Error> {
        let origin = self.calls.origin(&scope).map_err(Error::Invalid)?;
        if origin.requester() != &entry.consumer
            || origin.scope().identity.agent().map(|i| &i.session_id) != Some(&entry.session)
        {
            return Err(Error::Denied);
        }
        Ok(Handle {
            id: Id(entry.id.to_string()),
            io: Arc::new(Bound {
                calls: self.calls.clone(),
                configuration: self.configuration.clone(),
                entry,
                scope,
            }),
        })
    }
}
fn identity(scope: &Scope) -> String {
    format!("acp-process-{}", calls::digest(&scope.identity.agent()))
}
impl process::Processes for Processes {
    fn spawn(&self, scope: Scope, command: Command) -> BoxFuture<'_, Result<Handle, Error>> {
        Box::pin(async move {
            let origin = self.calls.origin(&scope).map_err(Error::Invalid)?;
            command.validate()?;
            if self.drain.is_cancelled() {
                return Err(Error::Denied);
            }
            {
                let mut entries = self.entries.lock().unwrap();
                entries.retain(|_, entry| {
                    entry
                        .running
                        .done
                        .borrow()
                        .as_ref()
                        .is_none_or(|d| d.cleanup.is_err())
                });
                if entries.len() >= 32 {
                    return Err(Error::Invalid("ACP process capacity reached".into()));
                }
            }
            let consumer = origin.requester().clone();
            let session = origin
                .scope()
                .identity
                .agent()
                .ok_or(Error::Denied)?
                .session_id
                .clone();
            let request = Arc::new(
                self.calls
                    .request(&scope, identity(&scope), self.drain.clone())
                    .map_err(Error::Invalid)?,
            );
            let input = protocol::Run {
                executable: command.executable,
                args: command.args,
                env: command.env,
                additional_permissions: self.configuration.permissions.clone(),
                justification: self
                    .configuration
                    .permissions
                    .as_ref()
                    .map(|_| format!("Run {}", self.configuration.agent.display_name)),
            };
            let (chunks, received) = mpsc::channel(8);
            let (done, completed) = watch::channel(None);
            let (started, waiting) = oneshot::channel();
            let running = Arc::new(Running {
                request: request.clone(),
                chunks: tokio::sync::Mutex::new(received),
                done: completed,
            });
            let mut ticket = self
                .workers
                .reserve()
                .map_err(|e| Error::Failed(e.to_string()))?;
            tokio::spawn(async move {
                ticket.start();
                let exit = pump(&request, input, chunks, started).await;
                let cleanup = request.finish().await;
                ticket.complete(cleanup.clone());
                done.send_replace(Some(Arc::new(Done { exit, cleanup })));
            });
            let id = tokio::select! {
                biased;
                () = scope.cancellation.cancelled() => None,
                id = waiting => id.ok(),
            };
            let Some(id) = id else {
                running.close().await.map_err(Error::CleanupUnconfirmed)?;
                return Err(Error::Failed("ACP process did not start".into()));
            };
            let entry = Arc::new(Entry {
                id,
                consumer,
                session,
                running,
            });
            let handle = match self.bind(scope, entry.clone()) {
                Ok(handle) => handle,
                Err(error) => {
                    entry
                        .running
                        .close()
                        .await
                        .map_err(Error::CleanupUnconfirmed)?;
                    return Err(error);
                }
            };
            self.entries.lock().unwrap().insert(id.to_string(), entry);
            Ok(handle)
        })
    }
    fn open(&self, scope: Scope, id: Id) -> Result<Handle, Error> {
        let entry = self
            .entries
            .lock()
            .unwrap()
            .get(&id.0)
            .cloned()
            .ok_or(Error::Denied)?;
        self.bind(scope, entry)
    }
    fn close(&self, id: Id) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(async move {
            let entry = self.entries.lock().unwrap().get(&id.0).cloned();
            if let Some(entry) = entry {
                entry
                    .running
                    .close()
                    .await
                    .map_err(Error::CleanupUnconfirmed)?;
                self.entries.lock().unwrap().remove(&id.0);
            }
            Ok(())
        })
    }
}
async fn pump(
    request: &NativeRequest,
    input: protocol::Run,
    chunks: mpsc::Sender<Chunk>,
    started: oneshot::Sender<Uuid>,
) -> Result<Exit, String> {
    let mut mutation = request
        .begin_mutation(
            "processes",
            0,
            "run".into(),
            serde_json::to_value(input).map_err(|e| e.to_string())?,
            Uuid::new_v4(),
            Attachments::default(),
        )?
        .await?;
    let mut started = Some(started);
    while let Some(value) = mutation.recv().await {
        let progress: protocol::Progress =
            serde_json::from_value(value).map_err(|e| e.to_string())?;
        match progress {
            protocol::Progress::Started { id } => {
                started
                    .take()
                    .ok_or("ACP process started twice")?
                    .send(id)
                    .map_err(|_| "ACP start observer closed")?;
            }
            protocol::Progress::Output { stream, bytes } => {
                if started.is_some() {
                    return Err("ACP output preceded process start".into());
                }
                let chunk = Chunk {
                    stream: match stream {
                        protocol::Stream::Stdout => Stream::Stdout,
                        protocol::Stream::Stderr => Stream::Stderr,
                    },
                    bytes,
                };
                tokio::select! {
                    biased;
                    () = request.scope().cancellation.cancelled() => {},
                    result = chunks.send(chunk) => if result.is_err() { request.scope().cancellation.cancel(); },
                }
            }
        }
    }
    let completion = mutation.completion().await;
    completion.cleanup?;
    match serde_json::from_value(calls::returned(completion.reply.receipt)?)
        .map_err(|e| e.to_string())?
    {
        protocol::Completion::Exited {
            code,
            success,
            stopped,
            error,
        } => Ok(Exit {
            code,
            success,
            stopped,
            error,
        }),
        protocol::Completion::Failed { message } => Err(message),
    }
}
impl Running {
    async fn completion(&self) -> Result<Arc<Done>, String> {
        let mut done = self.done.clone();
        loop {
            if let Some(result) = done.borrow_and_update().clone() {
                return Ok(result);
            }
            done.changed()
                .await
                .map_err(|_| "ACP process observer disappeared")?;
        }
    }
    async fn close(&self) -> Result<(), String> {
        self.request.finish().await?;
        self.completion().await?.cleanup.clone()
    }
}
struct Bound {
    calls: Arc<calls::Calls>,
    configuration: Configuration,
    entry: Arc<Entry>,
    scope: Scope,
}
impl Bound {
    async fn input(&self, bytes: Option<Vec<u8>>) -> Result<(), Error> {
        let value = serde_json::to_value(protocol::Input {
            id: self.entry.id,
            bytes,
            additional_permissions: self.configuration.permissions.clone(),
            justification: self
                .configuration
                .permissions
                .as_ref()
                .map(|_| format!("Continue {}", self.configuration.agent.display_name)),
        })
        .map_err(|e| Error::Invalid(e.to_string()))?;
        // Individual writes have unique durable mutation IDs while permission
        // decisions remain scoped to this original prompt's process operation.
        let request = self
            .calls
            .request(
                &self.scope,
                identity(&self.scope),
                self.scope.cancellation.clone(),
            )
            .map_err(Error::Invalid)?;
        let mut ticket = self
            .scope
            .resources
            .reserve()
            .map_err(|e| Error::Failed(e.to_string()))?;
        // The SDK may stop polling its sink as soon as it receives the peer's
        // reply. Accepted writes and their cleanup must finish independently.
        tokio::spawn(async move {
            ticket.start();
            let result = async {
                let reply = request
                    .mutate(
                        "processes",
                        0,
                        "input".into(),
                        value,
                        Uuid::new_v4(),
                        Attachments::default(),
                    )?
                    .await?;
                calls::returned(reply.receipt).map(|_| ())
            }
            .await;
            let cleanup = request.finish().await;
            ticket.complete(cleanup.clone());
            cleanup.map_err(Error::CleanupUnconfirmed)?;
            result.map_err(Error::Failed)
        })
        .await
        .map_err(|_| Error::CleanupUnconfirmed("ACP stdin worker disappeared".into()))?
    }
}
impl Process for Bound {
    fn write(&self, bytes: Vec<u8>) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(async move {
            if bytes.is_empty() {
                return self.input(Some(bytes)).await;
            }
            for bytes in bytes.chunks(8192) {
                self.input(Some(bytes.to_vec())).await?;
            }
            Ok(())
        })
    }
    fn end_input(&self) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(self.input(None))
    }
    fn next(&self) -> BoxFuture<'_, Result<Option<Chunk>, Error>> {
        Box::pin(async move {
            self.calls.origin(&self.scope).map_err(Error::Invalid)?;
            let chunk = self.entry.running.chunks.lock().await.recv().await;
            if chunk.is_none() {
                let done = self
                    .entry
                    .running
                    .completion()
                    .await
                    .map_err(Error::Failed)?;
                done.cleanup.clone().map_err(Error::CleanupUnconfirmed)?;
                if let Err(error) = &done.exit {
                    return Err(Error::Failed(error.clone()));
                }
            }
            Ok(chunk)
        })
    }
    fn wait(&self) -> BoxFuture<'_, Result<Exit, Error>> {
        Box::pin(async move {
            self.calls.origin(&self.scope).map_err(Error::Invalid)?;
            let done = self
                .entry
                .running
                .completion()
                .await
                .map_err(Error::Failed)?;
            done.cleanup.clone().map_err(Error::CleanupUnconfirmed)?;
            let exit = done.exit.as_ref().map_err(|e| Error::Failed(e.clone()))?;
            Ok(Exit {
                code: exit.code,
                success: exit.success,
                stopped: exit.stopped,
                error: exit.error.clone(),
            })
        })
    }
    fn close(&self) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(async move {
            self.entry
                .running
                .close()
                .await
                .map_err(Error::CleanupUnconfirmed)
        })
    }
}
