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

//! The original bounded image policy shared by file reads and image consumers.
use crate::failed;
use maka_agent_facts::tools::ToolError;

pub const MAX_BYTES: usize = 5 * 1024 * 1024;
const MAX_DIMENSION: usize = 2000;

/// Validate owned bytes only. This grants no filesystem or resource access.
pub fn validate(bytes: &[u8]) -> Result<&'static str, ToolError> {
    if bytes.len() > MAX_BYTES {
        return Err(failed("Read source exceeds byte limit"));
    }
    let mime = match maka_agent_facts::attachment::sniff_binary_mime(bytes) {
        Some(mime @ ("image/png" | "image/jpeg" | "image/gif" | "image/webp")) => mime,
        _ => {
            return Err(failed(
                "Read image requires PNG, JPEG, GIF, or WebP content",
            ));
        }
    };
    let dimensions = imagesize::blob_size(bytes)
        .map_err(|error| failed(format!("Read image dimensions unavailable: {error}")))?;
    if dimensions.width == 0
        || dimensions.height == 0
        || dimensions.width > MAX_DIMENSION
        || dimensions.height > MAX_DIMENSION
    {
        return Err(failed(
            "Read image dimensions must be between 1 and 2000 pixels",
        ));
    }
    Ok(mime)
}
