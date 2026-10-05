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

use gpui_kit::base::input::Rope;
use maka_ui::editor::*;
use serde_json::json;
use std::ops::Range;
/// Kit alone owns the buffer, selections and composition. This owner records
/// observations and causality, not a second writable copy of the text.
#[derive(PartialEq)]
pub(super) struct NativeSnapshot {
    pub text: Rope,
    pub selection: TextRange,
    pub reversed: bool,
    pub marked: Option<TextRange>,
}

pub(super) struct TextSnapshot {
    pub owner: String,
    pub text: Rope,
    pub revision: u64,
}
impl maka_plugins::call::ReadResource for TextSnapshot {
    fn encoding(&self) -> maka_plugins::call::ResourceEncoding {
        maka_plugins::call::ResourceEncoding::Utf8
    }
    fn resource_type(&self) -> maka_plugin_core::contract::ResourceType {
        text_resource_type()
    }
    fn bytes(&self) -> usize {
        self.text.len()
    }
    fn metadata(&self) -> serde_json::Value {
        json!({"owner":self.owner,"revision":self.revision,"bytes":self.text.len()})
    }
    fn read(&self, range: Range<usize>) -> Result<Vec<u8>, String> {
        if range
            .end
            .checked_sub(range.start)
            .is_none_or(|len| len > 64 * 1024)
        {
            return Err("invalid text range or chunk exceeds 64 KiB".into());
        }
        self.text
            .try_slice(range)
            .map(|s| s.to_string().into_bytes())
            .map_err(|_| "invalid UTF-8 range".into())
    }
}
