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

//! Process logging belongs to executable entry, before runtime/store startup.

pub(super) fn initialize() -> Result<(), maka_host_runtime::HostError> {
    tracing_subscriber::fmt()
        .with_max_level(tracing_subscriber::filter::LevelFilter::WARN)
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .try_init()
}

#[cfg(test)]
mod tests {
    use sqlx::{ConnectOptions, Connection};
    use std::{process::Command, time::Duration};

    #[test]
    fn sqlx_warnings_reach_stderr_without_debug_queries() {
        for (mode, expected) in [("slow", true), ("ordinary", false)] {
            let output = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "logging::tests::sqlx_worker", "--nocapture"])
                .env("MAKA_LOGGING_SQLX_PROBE", mode)
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
            let stdout = String::from_utf8(output.stdout).unwrap();
            assert!(stdout.contains("query completed"));
            assert!(!stdout.contains("WARN") && !stdout.contains("host-log-sqlx-probe"));
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert_eq!(
                stderr.contains("host-log-sqlx-probe"),
                expected,
                "{stderr:?}"
            );
            if expected {
                assert!(
                    stderr.contains("WARN") && stderr.contains("sqlx::query"),
                    "{stderr}"
                );
            }
            assert!(
                !stderr.contains('\u{1b}'),
                "log output must not contain ANSI styling"
            );
        }
    }

    #[test]
    fn sqlx_worker() {
        let Ok(mode) = std::env::var("MAKA_LOGGING_SQLX_PROBE") else {
            return;
        };
        super::initialize().unwrap();
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let options = sqlx::sqlite::SqliteConnectOptions::new()
                    .in_memory(true)
                    .log_statements(log::LevelFilter::Debug)
                    .log_slow_statements(
                        log::LevelFilter::Warn,
                        if mode == "slow" {
                            Duration::ZERO
                        } else {
                            Duration::MAX
                        },
                    );
                let mut connection = sqlx::SqliteConnection::connect_with(&options)
                    .await
                    .unwrap();
                let value: String = sqlx::query_scalar("SELECT 'host-log-sqlx-probe'")
                    .fetch_one(&mut connection)
                    .await
                    .unwrap();
                assert_eq!(value, "host-log-sqlx-probe");
                connection.close().await.unwrap();
            });
        println!("query completed");
    }
}
