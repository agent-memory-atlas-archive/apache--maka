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

//! Small plugin-owned UI preferences using the existing instance checkpoint store.
//! Accepted writes drain before native retirement; paint never performs storage I/O.
use super::Scope;
use crate::budget::{Kind, NativeBudget};
use maka_capabilities::call::Resources;
use maka_plugin_core::contract::{Contract, Interface, Transport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::{mpsc, watch};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValueCheckpointProps {
    #[schemars(length(min = 1, max = 512))]
    pub key: String,
    #[schemars(length(min = 1, max = 128))]
    pub format: String,
    pub command: Option<ValueCheckpointCommand>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ValueCheckpointCommand {
    Read {
        sequence: u64,
    },
    Write {
        sequence: u64,
        #[serde(rename = "expectedRevision")]
        expected_revision: Option<u64>,
        value: Value,
    },
}
impl ValueCheckpointCommand {
    fn sequence(&self) -> u64 {
        match self {
            Self::Read { sequence } | Self::Write { sequence, .. } => *sequence,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValueCheckpointObservation {
    pub sequence: u64,
    pub state: ValueCheckpointState,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ValueCheckpointState {
    Loading,
    Saving,
    Ready {
        revision: Option<u64>,
        value: Option<Value>,
    },
    Failed {
        diagnostic: String,
    },
}
pub fn contract() -> Contract {
    Contract {
        name: "maka.ui.ValueCheckpoint".into(),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Component {
            props: serde_json::to_value(schemars::schema_for!(ValueCheckpointProps)).unwrap(),
            events: BTreeMap::from([(
                "changed".into(),
                serde_json::to_value(schemars::schema_for!(ValueCheckpointObservation)).unwrap(),
            )]),
            captures: BTreeMap::new(),
        },
    }
}
pub struct ValueCheckpoint {
    key: String,
    format: String,
    command: Option<ValueCheckpointCommand>,
    sender: Option<mpsc::Sender<ValueCheckpointCommand>>,
    receiver: watch::Receiver<ValueCheckpointObservation>,
    current: ValueCheckpointObservation,
    announced: Option<ValueCheckpointObservation>,
}
fn failure(sequence: u64, error: impl std::fmt::Display) -> ValueCheckpointObservation {
    ValueCheckpointObservation {
        sequence,
        state: ValueCheckpointState::Failed {
            diagnostic: error.to_string().chars().take(1024).collect(),
        },
    }
}

impl ValueCheckpoint {
    pub fn new(
        scope: Option<Scope>,
        props: &ValueCheckpointProps,
        resources: &Arc<Resources>,
        budget: NativeBudget,
    ) -> Self {
        let initial = ValueCheckpointObservation {
            sequence: 0,
            state: ValueCheckpointState::Loading,
        };
        let (reply, receiver) = watch::channel(initial.clone());
        let (sender, requests) = mpsc::channel(1);
        let started = (|| {
            let scope = scope.ok_or("UI preferences require an owned UI Root")?;
            let store = scope
                .store
                .upgrade()
                .ok_or("original UI Root is unavailable")?;
            let mut ticket = resources.reserve().map_err(|e| e.to_string())?;
            let key = props.key.clone();
            let format = props.format.clone();
            let wake = resources.clone();
            ticket.start();
            scope.scheduler.clone().spawn(async move {
                let mut requests = requests;
                let mut checkpoints = None;
                let mut command = ValueCheckpointCommand::Read { sequence: 0 };
                loop {
                    let sequence = command.sequence();
                    let result: Result<ValueCheckpointState, String> = async {
                        let _bytes = budget
                            .reserve(Kind::Encoded, super::MAX_CONTENT + 32 * 1024)
                            .map_err(|e| e.to_string())?;
                        if checkpoints.is_none() {
                            checkpoints = Some(scope.open(&store)?);
                        }
                        let checkpoints = checkpoints.as_ref().unwrap();
                        match command {
                            ValueCheckpointCommand::Read { .. } => {
                                let saved =
                                    checkpoints.read(&key).await.map_err(|e| e.to_string())?;
                                match saved {
                                    None => Ok(ValueCheckpointState::Ready {
                                        revision: None,
                                        value: None,
                                    }),
                                    Some(saved) => {
                                        if saved.format != format
                                            || saved.data_source != Value::Null
                                            || saved.content.len() > 16384
                                        {
                                            return Err(
                                                "UI preference format is incompatible".into()
                                            );
                                        }
                                        let value = serde_json::from_slice(&saved.content)
                                            .map_err(|e| e.to_string())?;
                                        Ok(ValueCheckpointState::Ready {
                                            revision: Some(saved.revision),
                                            value: Some(value),
                                        })
                                    }
                                }
                            }
                            ValueCheckpointCommand::Write {
                                expected_revision,
                                value,
                                ..
                            } => {
                                let content =
                                    serde_json::to_vec(&value).map_err(|e| e.to_string())?;
                                if content.len() > 16384 {
                                    return Err("UI preferences exceed 16 KiB".into());
                                }
                                let revision = checkpoints
                                    .write(
                                        &key,
                                        expected_revision,
                                        &format,
                                        &Value::Null,
                                        &Value::Null,
                                        content,
                                    )
                                    .await
                                    .map_err(|e| e.to_string())?;
                                Ok(ValueCheckpointState::Ready {
                                    revision: Some(revision),
                                    value: Some(value),
                                })
                            }
                        }
                    }
                    .await;
                    reply.send_replace(match result {
                        Ok(state) => ValueCheckpointObservation { sequence, state },
                        Err(error) => failure(sequence, error),
                    });
                    wake.notify();
                    match requests.recv().await {
                        Some(next) => command = next,
                        None => break,
                    }
                }
                ticket.complete(Ok(()));
            });
            Ok::<_, String>(())
        })();
        Self {
            key: props.key.clone(),
            format: props.format.clone(),
            command: None,
            sender: started.is_ok().then_some(sender),
            receiver,
            current: started.err().map_or(initial, |error| failure(0, error)),
            announced: None,
        }
    }
    pub fn update(&mut self, props: &ValueCheckpointProps) {
        self.poll();
        if props.key != self.key || props.format != self.format {
            self.current = failure(
                self.current.sequence,
                "UI preference identity cannot change on a live mount",
            );
            return;
        }
        let Some(command) = props
            .command
            .as_ref()
            .filter(|command| Some(*command) != self.command.as_ref())
        else {
            return;
        };
        if self
            .command
            .as_ref()
            .is_some_and(|prior| command.sequence() <= prior.sequence())
        {
            return;
        }
        if let ValueCheckpointCommand::Write { value, .. } = command
            && serde_json::to_vec(value).map_or(true, |bytes| bytes.len() > 16384)
        {
            self.current = failure(command.sequence(), "UI preferences exceed 16 KiB");
            self.command = Some(command.clone());
            return;
        }
        self.command = Some(command.clone());
        self.current = match self
            .sender
            .as_ref()
            .ok_or("UI preferences are unavailable")
            .and_then(|sender| {
                sender
                    .try_send(command.clone())
                    .map_err(|_| "UI preferences are busy")
            }) {
            Ok(()) => ValueCheckpointObservation {
                sequence: command.sequence(),
                state: if matches!(command, ValueCheckpointCommand::Write { .. }) {
                    ValueCheckpointState::Saving
                } else {
                    ValueCheckpointState::Loading
                },
            },
            Err(error) => failure(command.sequence(), error),
        };
    }
    fn poll(&mut self) {
        if self.receiver.has_changed().unwrap_or(false) {
            let reply = self.receiver.borrow_and_update().clone();
            if reply.sequence >= self.current.sequence {
                self.current = reply;
            }
        }
    }
    pub fn observe(&mut self) -> Option<ValueCheckpointObservation> {
        self.poll();
        if self.announced.as_ref() == Some(&self.current) {
            return None;
        }
        self.announced = Some(self.current.clone());
        Some(self.current.clone())
    }
    pub fn close(&mut self) {
        self.sender.take();
    }
}

pub const IMPLEMENTATION: &str = "maka.ui.checkpoint";

pub fn native() -> maka_plugin_core::package::NativeDefinition {
    maka_plugin_core::package::NativeDefinition::new(
        IMPLEMENTATION,
        serde_json::json!({"type":"object","properties":{},"required":[],"additionalProperties":false}),
        [("Checkpoint", contract())],
    )
}
