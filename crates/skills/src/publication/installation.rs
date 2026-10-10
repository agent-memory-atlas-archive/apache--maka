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

use super::{Error, Tree};
use crate::api::InstallSource;
use maka_runtime::artifact::content_digest;

/// Update the original instruction, baseline and provenance artifacts in a captured tree.
pub fn set_installation(
    tree: &mut Tree,
    id: &str,
    source_id: &str,
    kind: InstallSource,
    content: Vec<u8>,
) -> Result<(), Error> {
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Lock<'a> {
        schema_version: u32,
        id: &'a str,
        source_type: InstallSource,
        source_name: &'static str,
        source_version: &'static str,
        content_sha256: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        source_id: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source_content_sha256: Option<&'a str>,
    }
    let hash = content_digest(&content);
    let managed = matches!(kind, InstallSource::Managed);
    let lock = serde_json::to_vec(&Lock {
        schema_version: 1,
        id,
        source_type: kind,
        source_name: if managed {
            "local-library"
        } else {
            "maka-bundled"
        },
        source_version: "1",
        content_sha256: &hash,
        source_id: managed.then_some(source_id),
        source_content_sha256: managed.then_some(hash.as_str()),
    })?;
    tree.insert("SKILL.md", content.clone())?;
    tree.insert(".maka/baseline/SKILL.md", content)?;
    tree.insert("skill.lock.json", lock)?;
    Ok(())
}
