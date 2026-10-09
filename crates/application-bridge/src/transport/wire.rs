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

//! Bounded frames; payload chunks are raw bytes, never expanded JSON arrays.
use crate::Result;
use maka_event_log::applications::intents::CallerIntent;
use maka_plugin_core::{
    composition::InstanceId,
    deployment::DeploymentRef,
    operation::{OperationReply, RecoveryRef},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::io::Write;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use uuid::Uuid;

pub(super) const FRAME_LIMIT: usize = 2 * 1024 * 1024;
/// Observations supplied by the caller adapter, never transport authentication.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub deployment: DeploymentRef,
    pub consumer: InstanceId,
    /// Original mount, captured by the trusted UI adapter rather than action JSON.
    pub mount: Uuid,
    pub request: Uuid,
}
impl TryFrom<&maka_ui::actions::ServiceSource> for Source {
    type Error = String;
    fn try_from(value: &maka_ui::actions::ServiceSource) -> Result<Self> {
        Ok(Self {
            deployment: value
                .deployment()
                .ok_or("persisted UI source required")?
                .clone(),
            consumer: value.consumer().instance().clone(),
            mount: value.mount().id(),
            request: value.request_id(),
        })
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Open {
    pub endpoint: String,
    pub registration_epoch: Uuid,
    pub source: Source,
    pub method: String,
    pub input: Value,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Control {
    Pull,
    Cancel,
    Commit { intent: Box<CallerIntent> },
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Reply {
    Ready,
    Subscribed,
    Prepared {
        reference: Box<RecoveryRef>,
    },
    Item {
        value: Value,
    },
    Finished {
        result: Result<Value>,
        cleanup: Result<()>,
    },
    Mutation {
        reply: Box<OperationReply>,
        cleanup: Result<()>,
    },
    Aborted {
        reason: String,
        cleanup: Result<()>,
    },
}
impl Reply {
    pub fn cleanup(&self) -> Option<&Result<()>> {
        match self {
            Self::Finished { cleanup, .. }
            | Self::Mutation { cleanup, .. }
            | Self::Aborted { cleanup, .. } => Some(cleanup),
            _ => None,
        }
    }
}
struct Bounded(Vec<u8>, usize);
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.1 - self.0.len() {
            return Err(std::io::Error::other(
                "Application frame exceeds byte limit",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub(super) async fn write<W: AsyncWrite + Unpin, T: Serialize>(
    writer: &mut W,
    value: &T,
) -> Result<()> {
    write_bounded(writer, value, FRAME_LIMIT).await
}
pub(super) async fn write_bounded<W: AsyncWrite + Unpin, T: Serialize>(
    writer: &mut W,
    value: &T,
    limit: usize,
) -> Result<()> {
    let buffer = encode_bounded(value, limit)?;
    writer
        .write_u32(buffer.len() as u32)
        .await
        .map_err(|e| e.to_string())?;
    writer.write_all(&buffer).await.map_err(|e| e.to_string())?;
    writer.flush().await.map_err(|e| e.to_string())
}
pub(super) fn encode_bounded<T: Serialize>(value: &T, limit: usize) -> Result<Vec<u8>> {
    let mut buffer = Bounded(Vec::new(), limit.min(FRAME_LIMIT));
    serde_json::to_writer(&mut buffer, value).map_err(|e| e.to_string())?;
    Ok(buffer.0)
}
/// Keeps partial prefix/body custody across select cancellation.
pub(super) struct Reader<R> {
    pub(super) inner: BufReader<R>,
    prefix: [u8; 4],
    prefix_read: usize,
    body: Vec<u8>,
    body_read: usize,
}
impl<R: AsyncRead + Unpin> Reader<R> {
    pub(super) fn new(inner: R) -> Self {
        Self {
            inner: BufReader::new(inner),
            prefix: [0; 4],
            prefix_read: 0,
            body: Vec::new(),
            body_read: 0,
        }
    }
    pub(super) async fn read<T: DeserializeOwned>(&mut self) -> Result<T> {
        self.read_bounded(FRAME_LIMIT).await
    }
    pub(super) async fn read_bounded<T: DeserializeOwned>(&mut self, limit: usize) -> Result<T> {
        while self.prefix_read < 4 {
            let n = self
                .inner
                .read(&mut self.prefix[self.prefix_read..])
                .await
                .map_err(|e| e.to_string())?;
            if n == 0 {
                return Err("Application channel ended before a complete frame".into());
            }
            self.prefix_read += n;
        }
        let length = u32::from_be_bytes(self.prefix) as usize;
        if !(1..=limit.min(FRAME_LIMIT)).contains(&length) {
            return Err("invalid Application frame length".into());
        }
        if self.body.is_empty() {
            self.body.resize(length, 0);
        }
        while self.body_read < length {
            let n = self
                .inner
                .read(&mut self.body[self.body_read..])
                .await
                .map_err(|e| e.to_string())?;
            if n == 0 {
                return Err("Application channel ended before a complete frame".into());
            }
            self.body_read += n;
        }
        let body = std::mem::take(&mut self.body);
        self.prefix_read = 0;
        self.body_read = 0;
        serde_json::from_slice(&body).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn selected_reads_preserve_partial_prefix_and_body() {
        let (input, mut output) = tokio::io::duplex(64);
        let mut reader = Reader::new(input);
        let body = br#"{"done":true}"#;
        let prefix = (body.len() as u32).to_be_bytes();
        output.write_all(&prefix[..1]).await.unwrap();
        {
            let mut future = Box::pin(reader.read::<Value>());
            assert!(futures_util::poll!(&mut future).is_pending());
        }
        output.write_all(&prefix[1..]).await.unwrap();
        output.write_all(&body[..3]).await.unwrap();
        {
            let mut future = Box::pin(reader.read::<Value>());
            assert!(futures_util::poll!(&mut future).is_pending());
        }
        output.write_all(&body[3..]).await.unwrap();
        assert_eq!(
            reader.read::<Value>().await.unwrap(),
            serde_json::json!({"done":true})
        );
    }
    #[tokio::test]
    async fn oversized_serialization_writes_no_frame_prefix() {
        let mut output = Vec::new();
        assert!(write(&mut output, &"x".repeat(FRAME_LIMIT)).await.is_err());
        assert!(output.is_empty());
    }
    #[tokio::test]
    async fn near_limit_input_and_descriptors_use_separate_real_frames() {
        use maka_plugin_core::{
            composition::InstanceId, contract::ResourceType, deployment::DeploymentRef,
        };
        use maka_plugins::call::ResourceEncoding;
        use maka_runtime_host::application::ResourceDescriptor;
        use std::collections::BTreeMap;
        let open = Open {
            endpoint: "business".into(),
            registration_epoch: Uuid::new_v4(),
            source: Source {
                deployment: DeploymentRef {
                    state_domain: "original-root-domain".into(),
                    deployment: Uuid::new_v4(),
                    revision: 1,
                },
                consumer: InstanceId(vec![Uuid::new_v4()]),
                mount: uuid::Uuid::new_v4(),
                request: Uuid::new_v4(),
            },
            method: "read".into(),
            input: Value::String("i".repeat(1_048_574)),
        };
        let mut descriptors = BTreeMap::from([(
            "text".to_string(),
            ResourceDescriptor {
                encoding: ResourceEncoding::Utf8,
                resource: ResourceType {
                    name: "Text".into(),
                    version: 1,
                },
                length: 0,
                metadata: Value::String(String::new()),
            },
        )]);
        let overhead = serde_json::to_vec(&descriptors).unwrap().len();
        descriptors.get_mut("text").unwrap().metadata =
            Value::String("m".repeat(1_048_576 - overhead));
        assert_eq!(serde_json::to_vec(&open.input).unwrap().len(), 1_048_576);
        assert_eq!(serde_json::to_vec(&descriptors).unwrap().len(), 1_048_576);
        let combined = serde_json::json!({"endpoint":open.endpoint,"source":open.source,"method":open.method,"input":open.input,"resources":descriptors});
        assert!(serde_json::to_vec(&combined).unwrap().len() > FRAME_LIMIT);
        let (output, input) = tokio::io::duplex(64 * 1024);
        let mut reader = Reader::new(input);
        let writer = tokio::spawn(async move {
            let mut output = output;
            write(&mut output, &open).await.unwrap();
            write(&mut output, &descriptors).await.unwrap();
        });
        let decoded: Open = reader.read().await.unwrap();
        assert_eq!(decoded.input.as_str().unwrap().len(), 1_048_574);
        let decoded: BTreeMap<String, ResourceDescriptor> = reader.read().await.unwrap();
        assert_eq!(serde_json::to_vec(&decoded).unwrap().len(), 1_048_576);
        writer.await.unwrap();
    }
}
