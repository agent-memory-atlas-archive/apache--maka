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

//! Pure presentation protocols shared by schema generation and native renderers.
pub mod desktop;
pub mod terminal;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{Duration, Instant};

    #[test]
    fn copy_button_text_has_a_byte_budget_and_no_conflicting_native_default() {
        for text in ["".to_owned(), "界".repeat(24 * 1024)] {
            let value = json!({"kind":"button","key":"copy","label":"Copy","copy_text":text});
            let deadline = Instant::now() + Duration::from_secs(1);
            assert!(
                serde_json::from_value::<terminal::Node>(value.clone())
                    .unwrap()
                    .validate_root(deadline)
                    .is_err()
            );
            assert!(
                serde_json::from_value::<desktop::Node>(value)
                    .unwrap()
                    .validate_root(deadline)
                    .is_err()
            );
        }
        let value =
            json!({"kind":"button","key":"copy","label":"Copy","copy_text":"x".repeat(64 * 1024)});
        let deadline = Instant::now() + Duration::from_secs(1);
        serde_json::from_value::<terminal::Node>(value.clone())
            .unwrap()
            .validate_root(deadline)
            .unwrap();
        serde_json::from_value::<desktop::Node>(value.clone())
            .unwrap()
            .validate_root(deadline)
            .unwrap();
        let mut conflict = value;
        conflict["focus_child"] = json!("editor");
        assert!(
            serde_json::from_value::<terminal::Node>(conflict)
                .unwrap()
                .validate_root(deadline)
                .is_err()
        );
    }
}
