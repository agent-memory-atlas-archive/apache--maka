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

//! JSON values keep JSON Number semantics instead of serde_v8's automatic BigInt conversion.
use deno_core::{FromV8, ToV8, v8};
use deno_error::JsErrorBox;
use serde_json::Value;

pub(super) struct Json(String);
impl Json {
    pub(super) fn encode(value: &Value) -> Result<Self, JsErrorBox> {
        exact_numbers(value)?;
        serde_json::to_string(value)
            .map(Self)
            .map_err(|e| JsErrorBox::generic(e.to_string()))
    }
}
pub(super) fn check_delivery(value: &Value, limit: usize) -> Result<usize, JsErrorBox> {
    let size = super::bridge::encoded_size(value, limit)
        .map_err(|e| JsErrorBox::generic(e.to_string()))?;
    exact_numbers(value)?;
    Ok(size)
}

fn exact_numbers(value: &Value) -> Result<(), JsErrorBox> {
    match value {
        Value::Number(n) => {
            if inexact_integer(n) {
                return Err(JsErrorBox::generic(
                    "JSON integer cannot be represented exactly as a JavaScript Number",
                ));
            }
        }
        Value::Array(items) => {
            for value in items {
                exact_numbers(value)?;
            }
        }
        Value::Object(items) => {
            for value in items.values() {
                exact_numbers(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn inexact_integer(n: &serde_json::Number) -> bool {
    if let Some(n) = n.as_i64() {
        n as f64 as i128 != n as i128
    } else if let Some(n) = n.as_u64() {
        n as f64 as u128 != n as u128
    } else {
        false
    }
}
// V8's shortest decimal may spell a different mathematical integer while still
// identifying the same binary Number. Keep that Number, rather than minting an
// exact Rust integer from its approximate decimal spelling.
fn js_numbers(value: &mut Value) {
    match value {
        Value::Number(n) if inexact_integer(n) => {
            *n = serde_json::Number::from_f64(n.as_f64().expect("finite JSON number"))
                .expect("finite JSON number");
        }
        Value::Array(items) => items.iter_mut().for_each(js_numbers),
        Value::Object(items) => items.values_mut().for_each(js_numbers),
        _ => {}
    }
}
impl<'a> ToV8<'a> for Json {
    type Error = JsErrorBox;
    fn to_v8<'i>(
        self,
        scope: &mut v8::PinScope<'a, 'i>,
    ) -> Result<v8::Local<'a, v8::Value>, Self::Error> {
        let text = v8::String::new(scope, &self.0)
            .ok_or_else(|| JsErrorBox::generic("JSON string allocation failed"))?;
        v8::json::parse(scope, text).ok_or_else(|| JsErrorBox::generic("JSON parse terminated"))
    }
}
pub(super) fn to_v8<'a, 'i>(
    scope: &mut v8::PinScope<'a, 'i>,
    value: &Value,
) -> Result<v8::Local<'a, v8::Value>, JsErrorBox> {
    Json::encode(value)?.to_v8(scope)
}

/// Op conversion checks the VM-owned JSON string before allocating a Rust Value.
pub(super) struct JsonInput(pub Value);
impl<'a> FromV8<'a> for JsonInput {
    type Error = JsErrorBox;
    fn from_v8<'i>(
        scope: &mut v8::PinScope<'a, 'i>,
        value: v8::Local<'a, v8::Value>,
    ) -> Result<Self, Self::Error> {
        decode(scope, value, 32 * 1024 * 1024).map(Self)
    }
}
pub(super) fn decode<'a, 'i>(
    scope: &mut v8::PinScope<'a, 'i>,
    value: v8::Local<'a, v8::Value>,
    limit: usize,
) -> Result<Value, JsErrorBox> {
    v8::tc_scope!(scope, scope);
    let value = if value.is_undefined() {
        v8::null(scope).into()
    } else {
        value
    };
    let encoded = v8::json::stringify(scope, value).ok_or_else(|| {
        JsErrorBox::generic(
            scope
                .exception()
                .map(|e| e.to_rust_string_lossy(scope))
                .unwrap_or_else(|| "value is not JSON".into()),
        )
    })?;
    if encoded.utf8_length(scope) > limit {
        return Err(JsErrorBox::generic(format!(
            "JSON value exceeds {} MiB",
            limit / (1024 * 1024)
        )));
    }
    let mut value = serde_json::from_str(&encoded.to_rust_string_lossy(scope))
        .map_err(|e| JsErrorBox::generic(e.to_string()))?;
    js_numbers(&mut value);
    Ok(value)
}
