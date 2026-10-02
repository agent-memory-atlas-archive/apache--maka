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

//! Request-local resources and accepted asynchronous service calls.
mod mutations;
use super::*;
use maka_event_log::applications::intents::RecordedIntent;
use maka_plugin_core::{
    contract::{MethodMode, ResourceType},
    operation::{OperationReply, RecoveryRef},
    runtime::{Capability, Execution},
};
use maka_plugins::call::{Attachments, ReadResource};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, future::Future, pin::Pin, sync::Arc};

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CaptureSource {
    pub child: String,
    pub method: String,
    pub input: Value,
}
/// An inert intent. Resource slots name capture requests, never bearer handles.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub key: String,
    pub binding: String,
    pub member: u32,
    pub method: String,
    pub input: Value,
    pub resources: BTreeMap<String, CaptureSource>,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CapturePlan {
    Forward { source: CaptureSource },
    Native,
}
pub type ServiceFuture = Pin<Box<dyn Future<Output = ServiceCompletion> + Send + 'static>>;
/// Native query execution and effect-free mutation ownership. Implementations
/// receive checked requests, never guest-created authorization evidence.
pub trait NativeService: Send + Sync + 'static {
    /// Construct a background future only. Polling performs the query after
    /// final admission; the executor retains it independently of any UI waiter.
    fn call(&self, request: ServiceRequest) -> ServiceFuture;
    /// Construct an inert owner before polling preparation. A returned error
    /// must leave no cleanup resource; panics are unconfirmed cleanup.
    fn mutation(
        &self,
        _request: ServiceRequest,
    ) -> std::result::Result<Arc<dyn NativeMutation>, String> {
        Err("service has no mutation executor".into())
    }
}

pub type ReferenceFuture =
    Pin<Box<dyn Future<Output = std::result::Result<RecoveryRef, String>> + Send>>;
pub type MutationFuture = Pin<Box<dyn Future<Output = MutationCompletion> + Send>>;
pub type CleanupFuture = Pin<Box<dyn Future<Output = std::result::Result<(), String>> + Send>>;
/// One actual native operation owner. Preparation has no business effects.
/// Dispatch rechecks native authority; close acknowledges actual cleanup, not
/// cancellation. The kernel retains this owner across every asynchronous stage.
pub trait NativeMutation: Send + Sync + 'static {
    fn prepare(self: Arc<Self>) -> ReferenceFuture;
    fn dispatch(self: Arc<Self>, recorded: RecordedIntent) -> MutationFuture;
    fn close(self: Arc<Self>) -> CleanupFuture;
}
pub struct MutationCompletion {
    pub reply: OperationReply,
    pub cleanup: std::result::Result<(), String>,
}

/// Immutable facts from the checked request. Final admission remains required.
/// These observations do not grant Host access.
/// Guest JSON cannot supply or replace this source.
#[derive(Clone, Debug)]
pub struct ServiceSource {
    deployment: Option<maka_plugin_core::deployment::DeploymentRef>,
    consumer: maka_plugin_core::runtime::Activation,
    mount: Mount,
    request: Uuid,
}
impl ServiceSource {
    pub fn deployment(&self) -> Option<&maka_plugin_core::deployment::DeploymentRef> {
        self.deployment.as_ref()
    }
    pub fn consumer(&self) -> &maka_plugin_core::runtime::Activation {
        &self.consumer
    }
    pub fn mount(&self) -> &Mount {
        &self.mount
    }
    pub fn request_id(&self) -> Uuid {
        self.request
    }
}
pub struct ServiceRequest {
    pub source: ServiceSource,
    pub method: String,
    pub input: Value,
    pub resources: Attachments,
}

/// Static source observations and actual endpoint values are separate facts.
#[derive(Default)]
pub struct RemoteServices {
    pub declarations: maka_plugin_core::plan::RemoteExports,
    pub registrations: Vec<RemoteService>,
}
pub struct RemoteService {
    pub target: maka_plugin_core::plan::RemoteCapability,
    pub registration_epoch: Uuid,
    pub contract: maka_plugin_core::contract::Contract,
    pub implementation: Arc<dyn NativeService>,
}
/// Business outcome and actual executor cleanup are separate facts.
pub struct ServiceCompletion {
    pub result: std::result::Result<Value, String>,
    pub cleanup: std::result::Result<(), String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActionOutcome {
    NotAccepted { reason: String },
    Succeeded { value: Value },
    Failed { reason: String },
    Operation { reply: Box<OperationReply> },
}
/// A presenter can resolve only this exact already-checked native leaf.
/// Construction is private: plugin JSON cannot supply a Mount or resource grant.
pub struct CaptureTarget {
    pub(crate) mount: Mount,
    pub(crate) method: String,
    pub(crate) input: Value,
    pub(crate) resource: ResourceType,
    edges: Vec<Edge>,
}
impl CaptureTarget {
    pub fn mount(&self) -> &Mount {
        &self.mount
    }
    pub fn method(&self) -> &str {
        &self.method
    }
    pub fn input(&self) -> &Value {
        &self.input
    }
    pub fn resource_type(&self) -> &ResourceType {
        &self.resource
    }
}
pub struct CaptureCall<'a> {
    pub state: &'a Value,
    pub props: &'a Value,
    pub method: &'a str,
    pub input: &'a Value,
}
pub type TaskFuture = Pin<Box<dyn Future<Output = TaskCompletion> + Send>>;
/// Execute independently of presentation lifetime, then return to the original
/// foreground owner. Completion can produce another task, including close.
pub struct ActionTask {
    pub id: Uuid,
    pub future: TaskFuture,
}
pub enum TaskCompletion {
    Query(ServiceCompletion),
    Prepared(std::result::Result<RecordedIntent, String>),
    Mutation(MutationCompletion),
    Closed(std::result::Result<(), String>),
    Interrupted(String),
}
#[derive(Clone)]
pub(super) struct Edge {
    pub parent: Mount,
    pub child: Mount,
    pub slot: String,
}
pub(super) struct Prepared<V: ?Sized> {
    id: Uuid,
    pub consumer: Mount,
    pub key: String,
    pub method: String,
    pub input: Value,
    pub capability: Capability<Export<V>>,
    pub captures: BTreeMap<String, CaptureTarget>,
    pub invalidated: bool,
    resources: Option<Attachments>,
}
struct Running<V: ?Sized> {
    consumer: Mount,
    key: String,
    method: String,
    completed: bool,
    execution: Execution<Export<V>>,
    resources: Attachments,
    mutation: Option<mutations::Session>,
}
pub(super) struct Outcome {
    pub consumer: Mount,
    pub key: String,
    pub outcome: ActionOutcome,
}
pub(super) struct Actions<V: ?Sized> {
    pub queue: VecDeque<Prepared<V>>,
    running: BTreeMap<Uuid, Running<V>>,
    preparing: BTreeMap<Uuid, mutations::Preparing<V>>,
    pub outcomes: VecDeque<Outcome>,
}
impl<V: ?Sized> Default for Actions<V> {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
            running: BTreeMap::new(),
            preparing: BTreeMap::new(),
            outcomes: VecDeque::new(),
        }
    }
}
impl<V: ?Sized> Actions<V> {
    pub fn accepted_count(&self) -> usize {
        self.running.len() + self.preparing.len()
    }
    pub fn len(&self) -> usize {
        self.queue.len() + self.running.len() + self.preparing.len() + self.outcomes.len()
    }
    fn bytes(&self) -> usize {
        self.queue
            .iter()
            .filter_map(|q| q.resources.as_ref())
            .chain(self.running.values().map(|r| &r.resources))
            .flat_map(|r| r.values())
            .map(|r| r.bytes())
            .sum()
    }
}
impl<V: ?Sized> Prepared<V> {
    pub(super) fn invalidate(&mut self, mut exists: impl FnMut(&Edge) -> bool) {
        if self
            .captures
            .values()
            .flat_map(|t| &t.edges)
            .any(|edge| !exists(edge))
        {
            self.invalidated = true;
        }
    }
}
impl<V: ?Sized + 'static> Application<V> {
    pub(super) fn prepare_action(
        &mut self,
        consumer: &Mount,
        action: Action,
        update: &mut tree::Update<V>,
    ) -> Result<Prepared<V>> {
        for name in [&action.key, &action.binding, &action.method] {
            if name.is_empty() || name.len() > 128 {
                return Err(Error::Invalid("invalid action name".into()));
            }
        }
        if action.resources.len() > 16 {
            return Err(Error::Invalid("action resource limit".into()));
        }
        let provider = self.provider(consumer)?;
        let caps = self.runtime.bind(&provider, &action.binding)?;
        let capability = caps
            .get(action.member as usize)
            .ok_or_else(|| Error::Invalid("action binding member unavailable".into()))?
            .clone();
        let declaration = match self.runtime.inspect(&capability)? {
            Export::Service { methods, .. } => methods.get(&action.method).map(|m| {
                (
                    m.mode,
                    m.resources.clone(),
                    m.input
                        .validate(&action.input)
                        .map_err(|e| Error::Invalid(e.to_string())),
                )
            }),
            _ => None,
        };
        let (mode, resources, valid) =
            declaration.ok_or_else(|| Error::Invalid("undeclared action method".into()))?;
        valid?;
        if mode == MethodMode::Mutation && self.source.is_none() {
            return Err(Error::Invalid(
                "mutation requires a persisted UI source".into(),
            ));
        }
        if resources.keys().ne(action.resources.keys()) {
            return Err(Error::Invalid("action resource slots mismatch".into()));
        }
        let mut captures = BTreeMap::new();
        for (slot, source) in action.resources {
            captures.insert(
                slot.clone(),
                self.prepare_capture(consumer, source, &resources[&slot], Vec::new(), update)?,
            );
        }
        Ok(Prepared {
            id: Uuid::new_v4(),
            consumer: consumer.clone(),
            key: action.key,
            method: action.method,
            input: action.input,
            capability,
            captures,
            invalidated: false,
            resources: None,
        })
    }
    fn prepare_capture(
        &mut self,
        parent: &Mount,
        source: CaptureSource,
        resource: &ResourceType,
        mut edges: Vec<Edge>,
        update: &mut tree::Update<V>,
    ) -> Result<CaptureTarget> {
        if source.child.is_empty()
            || source.child.len() > 128
            || source.method.is_empty()
            || source.method.len() > 128
            || edges.len() >= 32
        {
            return Err(Error::Invalid("invalid capture source/depth".into()));
        }
        tree::check_call_bytes(&source.input)?;
        update.charge_intent(&source)?;
        let view = self.current(parent, update)?;
        let child = view
            .children
            .get(&source.child)
            .ok_or_else(|| Error::Invalid("capture child unavailable".into()))?
            .mount
            .clone();
        let view = self.current(&child, update)?;
        tree::check_call_bytes(&(
            &view.frame.state,
            &view.props,
            &source.method,
            &source.input,
        ))?;
        let state = view.frame.state.clone();
        let props = view.props.clone();
        edges.push(Edge {
            parent: parent.clone(),
            child: child.clone(),
            slot: source.child,
        });
        update.step(self.mount_depth(&child)?)?;
        let execution = self.runtime.admit_mount(&child)?;
        let Export::Component {
            implementation,
            captures,
            ..
        } = execution.value()
        else {
            unreachable!()
        };
        let result = (|| {
            let definition = captures
                .get(&source.method)
                .ok_or_else(|| Error::Invalid("undeclared capture method".into()))?;
            if &definition.resource != resource {
                return Err(Error::Invalid("capture resource type mismatch".into()));
            }
            definition
                .input
                .validate(&source.input)
                .map_err(|e| Error::Invalid(e.to_string()))?;
            match implementation {
                Component::JavaScript { realm, callbacks } => {
                    let value = realm.borrow_mut().call_bound_until(
                        callbacks,
                        "capture",
                        &[state, props, json!(source.method), source.input.clone()],
                        update.deadline,
                    )?;
                    serde_json::from_value(value).map_err(|e| Error::Invalid(e.to_string()))
                }
                Component::Native(component) => component.capture(CaptureCall {
                    state: &state,
                    props: &props,
                    method: &source.method,
                    input: &source.input,
                }),
            }
        })();
        self.runtime.settle(&execution, Ok(()))?;
        if matches!(result, Err(Error::Script(_))) {
            self.retire_failed_realm(&child)?;
        }
        let plan = result?;
        update.charge_intent(&plan)?;
        match plan {
            CapturePlan::Forward { source } => {
                self.prepare_capture(&child, source, resource, edges, update)
            }
            CapturePlan::Native => {
                if !matches!(
                    execution.value(),
                    Export::Component {
                        implementation: Component::Native(_),
                        ..
                    }
                ) {
                    return Err(Error::Invalid(
                        "JavaScript cannot forge a native capture".into(),
                    ));
                }
                Ok(CaptureTarget {
                    mount: child,
                    method: source.method,
                    input: source.input,
                    resource: resource.clone(),
                    edges,
                })
            }
        }
    }
    fn target_valid(&self, target: &CaptureTarget) -> bool {
        target.edges.iter().all(|e| {
            self.views
                .get(&e.parent.id())
                .and_then(|v| v.children.get(&e.slot))
                .is_some_and(|c| c.mount == e.child && !self.is_retiring(&e.child))
        })
    }
    fn reject_action(&mut self, action: Prepared<V>, reason: String) {
        let reason = bounded_reason(reason);
        self.actions.outcomes.push_back(Outcome {
            consumer: action.consumer,
            key: action.key,
            outcome: ActionOutcome::NotAccepted { reason },
        });
    }
    /// Run after native owners are prepared, before native commands/cleanup.
    /// Capture failure rejects this operation, not the already-published scene.
    pub fn capture_actions(
        &mut self,
        root: &Mount,
        mut capture: impl FnMut(&CaptureTarget) -> Result<Arc<dyn ReadResource>>,
    ) -> Result<usize> {
        let mut captured = 0;
        for _ in 0..self.actions.queue.len() {
            let mut action = self.actions.queue.pop_front().expect("bounded queue");
            if self
                .mount_root(&action.consumer)
                .as_ref()
                .is_ok_and(|r| r != root)
                || action.resources.is_some()
            {
                self.actions.queue.push_back(action);
                continue;
            }
            if action.invalidated
                || self.is_retiring(&action.consumer)
                || action.captures.values().any(|t| !self.target_valid(t))
            {
                self.reject_action(action, "capture source or consumer retired".into());
                continue;
            }
            let result = (|| {
                let mut resources = Attachments::new();
                let mut bytes = self.actions.bytes();
                for (slot, target) in &action.captures {
                    let resource = capture(target)?;
                    if resource.resource_type() != target.resource {
                        return Err(Error::Invalid("native resource type mismatch".into()));
                    }
                    bytes = bytes
                        .checked_add(resource.bytes())
                        .ok_or_else(|| Error::Invalid("resource byte overflow".into()))?;
                    if bytes > 64 * 1024 * 1024 {
                        return Err(Error::Invalid("action resources exceed 64 MiB".into()));
                    }
                    tree::check_call_bytes(&resource.metadata())?;
                    resources.insert(slot.clone(), resource);
                }
                Ok(resources)
            })();
            match result {
                Ok(resources) => {
                    action.resources = Some(resources);
                    self.actions.queue.push_back(action);
                    captured += 1;
                }
                Err(error) => self.reject_action(action, error.to_string()),
            }
        }
        Ok(captured)
    }
    /// Final Mount and captured registration check. A queue is not accepted Work.
    pub fn start_task(&mut self, root: &Mount) -> Result<Option<ActionTask>> {
        loop {
            let Some(index) = self.actions.queue.iter().position(|a| {
                a.resources.is_some()
                    && (self.is_retiring(&a.consumer)
                        || self
                            .mount_root(&a.consumer)
                            .as_ref()
                            .is_ok_and(|r| r == root))
            }) else {
                return Ok(None);
            };
            let action = self.actions.queue.remove(index).expect("queue position");
            if action.invalidated
                || self.is_retiring(&action.consumer)
                || action.captures.values().any(|t| !self.target_valid(t))
            {
                self.reject_action(action, "consumer retired before admission".into());
                continue;
            }
            let mode = match self.runtime.inspect(&action.capability) {
                Ok(Export::Service { methods, .. }) => methods[&action.method].mode,
                Ok(_) => unreachable!(),
                Err(error) => {
                    self.reject_action(action, error.to_string());
                    continue;
                }
            };
            if mode == MethodMode::Mutation {
                if let Some(task) = self.start_mutation(action)? {
                    return Ok(Some(task));
                }
                continue;
            }
            let execution = match self.runtime.admit(&action.capability) {
                Ok(execution) => execution,
                Err(error) => {
                    self.reject_action(action, error.to_string());
                    continue;
                }
            };
            let Export::Service { implementation, .. } = execution.value() else {
                unreachable!()
            };
            let implementation = implementation.clone();
            let resources = action.resources.expect("captured");
            let method = action.method;
            let id = action.id;
            let request = ServiceRequest {
                source: ServiceSource {
                    deployment: self.source.clone(),
                    consumer: action.capability.consumer().clone(),
                    mount: action.consumer.clone(),
                    request: id,
                },
                method: method.clone(),
                input: action.input,
                resources: resources.clone(),
            };
            self.actions.running.insert(
                id,
                Running {
                    consumer: action.consumer,
                    key: action.key,
                    method,
                    completed: false,
                    execution,
                    resources,
                    mutation: None,
                },
            );
            return Ok(Some(ActionTask {
                id,
                future: Box::pin(async move {
                    TaskCompletion::Query(implementation.call(request).await)
                }),
            }));
        }
    }
    /// Executor completion is independent of result delivery. Failed/unknown
    /// cleanup preserves Work and resource ownership; it is never an implicit Ok.
    fn finish_query(&mut self, id: Uuid, completion: ServiceCompletion) -> Result<()> {
        let running = self
            .actions
            .running
            .get(&id)
            .ok_or_else(|| Error::Invalid("unknown accepted action".into()))?;
        if running.completed {
            return Err(Error::Invalid("action outcome already completed".into()));
        }
        if running.mutation.is_some() {
            return Err(Error::Invalid(
                "mutation requires its original receipt".into(),
            ));
        }
        let output = match completion.result {
            Ok(value) => {
                let Export::Service { methods, .. } = running.execution.value() else {
                    unreachable!()
                };
                let valid = tree::check_call_bytes(&value).and_then(|()| {
                    methods[&running.method]
                        .output
                        .validate(&value)
                        .map_err(|e| Error::Invalid(e.to_string()))
                });
                match valid {
                    Ok(()) => ActionOutcome::Succeeded { value },
                    Err(e) => ActionOutcome::Failed {
                        reason: bounded_reason(e.to_string()),
                    },
                }
            }
            Err(reason) => ActionOutcome::Failed {
                reason: if reason.len() > 64 * 1024 {
                    "service error exceeds 64 KiB".into()
                } else {
                    reason
                },
            },
        };
        let consumer = running.consumer.clone();
        let key = running.key.clone();
        let cleanup_result = completion.cleanup.map_err(bounded_reason);
        let failure = cleanup_result.as_ref().err().cloned();
        let cleanup = self.runtime.settle(&running.execution, cleanup_result);
        self.actions
            .running
            .get_mut(&id)
            .expect("running")
            .completed = true;
        self.actions.outcomes.push_back(Outcome {
            consumer,
            key,
            outcome: output,
        });
        cleanup?;
        if let Some(reason) = failure {
            return Err(Error::Core(maka_plugin_core::Error::CleanupFailed(vec![
                reason,
            ])));
        }
        self.actions.running.remove(&id);
        Ok(())
    }
    /// Recovery may acknowledge only actual resource cleanup after an unknown
    /// result. It cannot rewrite or redeliver the previously completed outcome.
    pub fn complete_action_cleanup(
        &mut self,
        id: Uuid,
        cleanup: std::result::Result<(), String>,
    ) -> Result<()> {
        if self.actions.preparing.contains_key(&id) {
            return self.confirm_preparation_cleanup(id, cleanup);
        }
        let running = self
            .actions
            .running
            .get(&id)
            .ok_or_else(|| Error::Invalid("unknown accepted action".into()))?;
        if !running.completed {
            return Err(Error::Invalid("executor has not completed".into()));
        }
        let cleanup = cleanup.map_err(bounded_reason);
        let failure = cleanup.as_ref().err().cloned();
        self.runtime.settle(&running.execution, cleanup)?;
        if let Some(reason) = failure {
            return Err(Error::Core(maka_plugin_core::Error::CleanupFailed(vec![
                reason,
            ])));
        }
        self.actions.running.remove(&id);
        Ok(())
    }
    pub fn pending_action_count(&self) -> usize {
        self.actions.len()
    }
    pub fn failed_action_ids(&self) -> Vec<Uuid> {
        self.actions
            .running
            .iter()
            .filter(|(_, r)| r.completed)
            .map(|(id, _)| *id)
            .chain(
                self.actions
                    .preparing
                    .iter()
                    .filter(|(_, p)| p.failed())
                    .map(|(id, _)| *id),
            )
            .collect()
    }

    pub fn accepted_action_count(&self) -> usize {
        self.actions.running.len()
    }
    /// No raw-input authority: only outcomes produced by owned requests can
    /// reach these handler keys. Each feedback is a new bounded UI transaction.
    pub fn deliver_action_results(&mut self, root: &Mount) -> Result<usize> {
        let mut count = 0;
        let mut first_error = None;
        for _ in 0..self.actions.outcomes.len() {
            let outcome = self.actions.outcomes.pop_front().expect("bounded outcomes");
            if self.is_retiring(&outcome.consumer) {
                continue;
            }
            if self
                .mount_root(&outcome.consumer)
                .as_ref()
                .is_ok_and(|r| r != root)
            {
                self.actions.outcomes.push_back(outcome);
                continue;
            }
            let feedback =
                self.operation_feedback(&outcome.consumer, &outcome.key, outcome.outcome);
            match self.dispatch_action_result(
                &outcome.consumer,
                &outcome.key,
                serde_json::to_value(feedback).expect("typed outcome"),
            ) {
                Ok(()) => count += 1,
                Err(error) => {
                    // No healthy consumer may remain waiting on an outcome
                    // its handler rejected. Work has already settled separately.
                    self.runtime.retire_mount(&outcome.consumer)?;
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        Ok(count)
    }
    pub(super) fn retire_failed_realm(&mut self, mount: &Mount) -> Result<()> {
        let execution = self.runtime.admit_mount(mount)?;
        let realm = match execution.value() {
            Export::Component {
                implementation: Component::JavaScript { realm, .. },
                ..
            } => Some(realm.clone()),
            _ => None,
        };
        self.runtime.settle(&execution, Ok(()))?;
        if let Some(realm) = realm {
            for (id, loaded) in &self.modules {
                if Rc::ptr_eq(&realm, &loaded.realm) {
                    self.runtime.retire(&self.activations[id])?;
                }
            }
        }
        Ok(())
    }
}

fn bounded_reason(reason: String) -> String {
    if reason.len() > 64 * 1024 {
        "action diagnostic exceeds 64 KiB".into()
    } else {
        reason
    }
}
