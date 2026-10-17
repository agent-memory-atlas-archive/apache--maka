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

//! Safe evidence at the boundary where private errors become public rejection codes.

use maka_agent_ports::provider::Error as ProviderError;
use maka_config::ConfigError;
use maka_model::ModelError;

pub(super) fn provider(
    stage: &'static str,
    error: &ProviderError,
    connection_id: &str,
    provider: &str,
) {
    let cause = match error {
        ProviderError::Cancelled => return,
        ProviderError::Unavailable => "provider_unavailable",
        ProviderError::AuthenticationRequired => "provider_authentication_required",
        ProviderError::Rejected(_) => "provider_rejected",
        ProviderError::Invalid(_) => "provider_invalid",
        ProviderError::OutcomeUnknown => "provider_outcome_unknown",
        ProviderError::Transport(_) => "provider_transport",
        ProviderError::Http(_) => "provider_http",
    };
    let status = match error {
        ProviderError::Http(status) => Some(*status),
        _ => None,
    };
    tracing::warn!(
        stage,
        cause,
        connection_id,
        provider,
        status,
        "Model preparation failed"
    );
}

pub(super) fn configuration(stage: &'static str, error: &ConfigError, connection_id: Option<&str>) {
    let cause = crate::diagnostics::config_cause(error);
    match error {
        ConfigError::Io(error) => tracing::warn!(
            stage, cause, connection_id, error_kind = ?error.kind(), os_code = error.raw_os_error(),
            "Model preparation failed"
        ),
        ConfigError::Sqlx(error) => {
            // Only a numeric SQLite code, never the database's free-form message.
            let sql_code = error
                .as_database_error()
                .and_then(|error| error.code())
                .and_then(|code| code.parse::<i32>().ok());
            tracing::warn!(
                stage,
                cause,
                connection_id,
                sql_code,
                "Model preparation failed"
            );
        }
        ConfigError::Json(error) => tracing::warn!(
            stage, cause, connection_id, category = ?error.classify(),
            line = error.line(), column = error.column(), "Model preparation failed"
        ),
        _ => tracing::warn!(stage, cause, connection_id, "Model preparation failed"),
    }
}

pub(super) fn model(stage: &'static str, error: &ModelError, connection_id: &str) {
    match error {
        ModelError::Cancelled => {}
        ModelError::Http { status } => tracing::warn!(
            stage,
            cause = "model_http",
            connection_id,
            status,
            "Model preparation failed"
        ),
        ModelError::TimedOut => tracing::warn!(
            stage,
            cause = "model_timed_out",
            connection_id,
            "Model preparation failed"
        ),
        ModelError::ContextOverflow { observed_output } => tracing::warn!(
            stage,
            cause = "model_context_overflow",
            connection_id,
            observed_output,
            "Model preparation failed"
        ),
        ModelError::Provider(error) => {
            tracing::warn!(stage, cause = "model_provider", connection_id, reason = ?error.reason(), "Model preparation failed")
        }
        ModelError::Adapter(_) => tracing::warn!(
            stage,
            cause = "model_adapter",
            connection_id,
            "Model preparation failed"
        ),
    }
}

pub(super) fn network(stage: &'static str, error: &maka_network::Error, connection_id: &str) {
    let cause = match error {
        maka_network::Error::InvalidProxy => "proxy_invalid",
        maka_network::Error::MissingCredentials => "proxy_credentials_missing",
        maka_network::Error::InvalidUpgrade => "network_invalid_upgrade",
        maka_network::Error::Transport(_) => "network_transport",
    };
    tracing::warn!(stage, cause, connection_id, "Model preparation failed");
}
