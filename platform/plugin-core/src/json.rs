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

//! Bounded JSON envelopes shared by native and JavaScript plugin boundaries.
use serde_json::Value;

/// Reject integer rounding before a value crosses an application boundary.
pub fn check_delivery(value: &Value, limit: usize) -> Result<usize, String> {
    let size = encoded_size(value, limit).map_err(|error| error.to_string())?;
    exact_numbers(value)?;
    Ok(size)
}

pub fn exact_numbers(value: &Value) -> Result<(), String> {
    match value {
        Value::Number(n) => {
            if inexact_integer(n) {
                return Err(
                    "JSON integer cannot be represented exactly as a JavaScript Number".into(),
                );
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
pub fn inexact_integer(n: &serde_json::Number) -> bool {
    if let Some(n) = n.as_i64() {
        n as f64 as i128 != n as i128
    } else if let Some(n) = n.as_u64() {
        n as f64 as u128 != n as u128
    } else {
        false
    }
}
// Count encoded bytes without allocating a second copy of a large Host result.
pub fn encoded_size<T: serde::Serialize + ?Sized>(
    value: &T,
    limit: usize,
) -> serde_json::Result<usize> {
    struct Counter {
        size: usize,
        limit: usize,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.limit.saturating_sub(self.size) {
                return Err(std::io::Error::other("Host payload byte limit"));
            }
            self.size += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter { size: 0, limit };
    serde_json::to_writer(&mut counter, value)?;
    Ok(counter.size)
}
