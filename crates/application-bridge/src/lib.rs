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

//! Trusted local connections between separate UI and Host deployment owners.
mod connections;
mod gateway;
mod service;
pub mod stdio;
pub mod transport;
pub use connections::{Configuration, Connections, Grant, Host, HostExecutable};
pub use gateway::{GATEWAY, with_gateway};
pub use service::{LocalService, Scopes};
pub type Result<T> = std::result::Result<T, String>;

/// Product startup must not exit after losing confirmation of an acquired SQL
/// owner's close. That original worker retains the Root and writer authority.
pub async fn open_store(
    owner: std::sync::Arc<maka_event_log::root::RootOwner>,
) -> std::result::Result<maka_event_log::EventLog, maka_event_log::StoreError> {
    match maka_event_log::EventLog::for_root(owner).await {
        Err(error @ maka_event_log::StoreError::StartupCleanupFailed { .. }) => {
            eprintln!("SQL startup cleanup unconfirmed: {error}");
            std::future::pending().await
        }
        result => result,
    }
}

/// Wait for a SQL owner's real close acknowledgement. On failure the
/// launcher must retain this store and its Root, rather than imply clean exit.
pub async fn drain_store(store: &maka_event_log::EventLog) {
    let mut reported = std::collections::BTreeSet::new();
    loop {
        match store.shutdown().await {
            Ok(()) => return,
            Err(error) => {
                let error = error.to_string();
                if reported.insert(error.clone()) {
                    eprintln!("SQL cleanup unconfirmed: {error:?}");
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
    }
}
