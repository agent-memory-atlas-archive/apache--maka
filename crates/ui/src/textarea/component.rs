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

use super::*;
use crate::{ComponentCall, ComponentOutput, Emission, Frame};
use serde_json::json;

/// Pure native textarea protocol shared by presentation adapters. The renderer
/// remains the sole buffer, selection and composition owner.
pub fn reduce(call: ComponentCall<'_>) -> crate::Result<ComponentOutput<TextareaProps>> {
    let (props, revision, observation, mut request, mut pending) = match call {
        ComponentCall::Mount { props } => (props.clone(), 0, serde_json::Value::Null, None, false),
        ComponentCall::Update { state, props } => (
            props.clone(),
            state["revision"].as_u64().expect("editor UI revision") + 1,
            state["observation"].clone(),
            serde_json::from_value::<Option<TextareaCommand>>(state["request"].clone())
                .expect("editor requested command"),
            state["pending"].as_bool().expect("editor pending command"),
        ),
        ComponentCall::Event {
            state,
            props,
            input,
            ..
        } => {
            let request =
                serde_json::from_value::<Option<TextareaCommand>>(state["request"].clone())
                    .expect("editor requested command");
            let observation: TextareaObservation =
                serde_json::from_value(input.clone()).expect("validated editor observation");
            let settled = request
                .as_ref()
                .zip(observation.receipt.as_ref())
                .is_some_and(|(command, receipt)| {
                    command.owner == receipt.owner && command.sequence == receipt.sequence
                });
            return frame(
                props.clone(),
                state["revision"].as_u64().expect("editor UI revision") + 1,
                input.clone(),
                request,
                state["pending"].as_bool().expect("editor pending command") && !settled,
            )
            .map(|mut output| {
                output.events.push(Emission {
                    name: "changed".into(),
                    value: input.clone(),
                });
                output
            });
        }
    };
    let typed: TextareaProps =
        serde_json::from_value(props.clone()).map_err(|e| crate::Error::Invalid(e.to_string()))?;
    if let Some(command) = &typed.command
        && request.as_ref() != Some(command)
    {
        if pending {
            return Err(crate::Error::Invalid(
                "editor prior command has not settled".into(),
            ));
        }
        if request
            .as_ref()
            .is_some_and(|prior| command.owner == prior.owner && command.sequence <= prior.sequence)
        {
            return Err(crate::Error::Invalid(
                "editor command sequence must advance".into(),
            ));
        }
        request = Some(command.clone());
        pending = true;
    }
    frame(props, revision, observation, request, pending)
}

fn frame(
    props: serde_json::Value,
    revision: u64,
    observation: serde_json::Value,
    request: Option<TextareaCommand>,
    pending: bool,
) -> crate::Result<ComponentOutput<TextareaProps>> {
    let mut typed: TextareaProps =
        serde_json::from_value(props).map_err(|e| crate::Error::Invalid(e.to_string()))?;
    // Props propose new commands; the accepted request remains authoritative
    // until its receipt commits. None does not cancel a staged request.
    typed.command = request.clone().filter(|_| pending);
    Ok(Frame {
            revision,
            state: json!({"revision":revision,"request":request,"pending":pending,"observation":observation}),
            surface: None,
            view: typed,
        }.output(Vec::new()))
}
