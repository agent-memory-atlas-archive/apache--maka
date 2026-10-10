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

//! Native elapsed-time formatting. Boundaries are supplied by the product;
//! this has no execution, scheduling or recovery authority.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Elapsed {
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub from: u64,
    #[schemars(with = "maka_runtime::schema::RequiredNullable<u64>")]
    pub to: Option<u64>,
}
impl Elapsed {
    pub fn validate(&self) -> crate::Result<()> {
        const MAX: u64 = (1 << 53) - 1;
        if self.from > MAX || self.to.is_some_and(|to| to < self.from || to > MAX) {
            return Err(crate::Error::Invalid(
                "invalid elapsed-time boundaries".into(),
            ));
        }
        Ok(())
    }
    pub fn text(&self, now: u64) -> Option<String> {
        let seconds = self.to.unwrap_or(now).checked_sub(self.from)? / 1000;
        Some(if seconds < 60 {
            format!("{seconds}s")
        } else if seconds < 3600 {
            format!("{}m {:02}s", seconds / 60, seconds % 60)
        } else {
            format!(
                "{}h {:02}m {:02}s",
                seconds / 3600,
                seconds / 60 % 60,
                seconds % 60
            )
        })
    }
}
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |time| time.as_millis().min(u64::MAX as u128) as u64)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recorded_end_freezes_elapsed_and_invalid_clocks_never_become_zero() {
        let mut elapsed = Elapsed {
            from: 1000,
            to: None,
        };
        assert_eq!(elapsed.text(13000).as_deref(), Some("12s"));
        assert_eq!(elapsed.text(69000).as_deref(), Some("1m 08s"));
        assert_eq!(elapsed.text(3662000).as_deref(), Some("1h 01m 01s"));
        assert_eq!(elapsed.text(999), None);
        elapsed.to = Some(69000);
        assert_eq!(elapsed.text(999999).as_deref(), Some("1m 08s"));
        elapsed.to = Some(999);
        assert!(elapsed.validate().is_err());
        elapsed.to = Some(1 << 53);
        assert!(elapsed.validate().is_err());
    }
}
