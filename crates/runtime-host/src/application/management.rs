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

//! Explicitly composed management of a fixed deployment in the original Root.
use super::*;
use maka_event_log::{EventLog, StoreError, applications::validate_application};
use maka_plugin_core::deployment::management::{
    DeploymentInstances, DeploymentRead, DeploymentReplace, InstanceSetEnabled, ManagedInstance,
    ManagedInstancesPage, ManagementRejection, ManagementResult, PackageInstall, PackageInstalled,
    deployment_manager_contract, package_bundle_type,
};
use serde::Deserialize;

pub const DEPLOYMENT_MANAGER: &str = "maka.application.management";

/// Trusted launcher handle. The target and execution environment never come
/// from guest configuration or mutation input; every operation uses Host admission.
#[derive(Clone)]
pub struct ManagementTarget(Manager);
impl ManagementTarget {
    pub async fn from_source(store: Arc<EventLog>, source: DeploymentRef) -> Result<Self> {
        let metadata = store
            .read_application_metadata_at(&source)
            .await
            .map_err(|error| error.to_string())?
            .ok_or("managed source not found")?;
        let environment = metadata.source.input.environment;
        environment
            .entry_target()
            .map_err(|error| error.to_string())?;
        Ok(Self(Manager {
            source,
            store,
            environment,
            writes: Arc::new(tokio::sync::Mutex::new(())),
        }))
    }
}

#[derive(Clone)]
struct Manager {
    source: DeploymentRef,
    store: Arc<EventLog>,
    environment: maka_plugin_core::composition::Environment,
    writes: Arc<tokio::sync::Mutex<()>>,
}
impl Manager {
    async fn metadata(
        &self,
    ) -> Result<maka_plugin_core::deployment::management::ManagedDeployment> {
        self.store
            .read_application_metadata(self.source.deployment)
            .await
            .map_err(|error| error.to_string())?
            .ok_or("managed deployment not found".into())
    }
    async fn instances(&self, input: Value) -> Result<Value> {
        let request: DeploymentInstances =
            serde_json::from_value(input).map_err(|e| e.to_string())?;
        let _materialize = self.writes.lock().await;
        let (source, after) = match request {
            DeploymentInstances::Start {} => (self.metadata().await?.source.reference, None),
            DeploymentInstances::Continue { source, after } => (source, after),
        };
        if source.state_domain != self.source.state_domain
            || source.deployment != self.source.deployment
        {
            return Err("foreign managed deployment source".into());
        }
        let loaded = self
            .store
            .load_application_at(&source)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("managed source not found")?;
        let plan = desired_plan(&loaded)?;
        if after
            .as_ref()
            .is_some_and(|after| !plan.instances().contains_key(after))
        {
            return Err("managed instance cursor not found".into());
        }
        let mut remaining = plan.instances().values().filter(|instance| {
            after
                .as_ref()
                .is_none_or(|after| &instance.identity > after)
        });
        let instances: Vec<_> = remaining
            .by_ref()
            .take(8)
            .map(|instance| ManagedInstance {
                instance: instance.identity.clone(),
                path: instance.path.clone(),
                package: instance.package.clone(),
                structural: instance.entry.is_none(),
                disabled: instance.disabled,
                parent_disabled: instance
                    .parent
                    .as_ref()
                    .is_some_and(|parent| plan.instances()[parent].disabled),
            })
            .collect();
        let next = remaining.next().map(|_| {
            instances
                .last()
                .expect("full managed page")
                .instance
                .clone()
        });
        serde_json::to_value(ManagedInstancesPage {
            source,
            instances,
            next,
        })
        .map_err(|e| e.to_string())
    }
    async fn set_enabled(&self, input: Value) -> Result<Value> {
        let request: InstanceSetEnabled =
            serde_json::from_value(input).map_err(|e| e.to_string())?;
        let _write = self.writes.lock().await;
        let metadata = self.metadata().await?;
        if request.expected_revision != metadata.source.reference.revision {
            return conflict(metadata.source.reference.revision);
        }
        let mut loaded = self
            .store
            .load_application_at(&metadata.source.reference)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("managed source not found")?;
        let plan = desired_plan(&loaded)?;
        let Some(instance) = plan.instances().get(&request.instance) else {
            return rejected(ManagementRejection::InstanceUnavailable);
        };
        if request.enabled
            && instance
                .parent
                .as_ref()
                .is_some_and(|parent| plan.instances()[parent].disabled)
        {
            return rejected(ManagementRejection::ParentDisabled);
        }
        let [root, path @ ..] = instance.path.as_slice() else {
            return Err("invalid actual managed path".into());
        };
        let node = loaded
            .source
            .input
            .instances
            .get_mut(root)
            .ok_or("managed root disappeared")?;
        if path.is_empty() {
            node.disabled = !request.enabled;
        } else {
            if let Some(patch) = node.overrides.iter_mut().find(|patch| patch.path == path) {
                patch.disabled = Some(!request.enabled);
            } else {
                node.overrides
                    .push(maka_plugin_core::composition::Override {
                        path: path.to_vec(),
                        disabled: Some(!request.enabled),
                        ..Default::default()
                    });
            }
        }
        // Effective disabled may come from an ancestor. Persist the user's own
        // explicit intent even when that effective projection already matches.
        self.commit(
            request.expected_revision,
            loaded.source.input,
            loaded.source.remote_exports,
            loaded.packages,
            |source| source,
        )
        .await
    }
    async fn install(&self, input: Value, context: Context) -> Result<Value> {
        let request: PackageInstall =
            serde_json::from_value(input).map_err(|error| error.to_string())?;
        let _write = self.writes.lock().await;
        let metadata = self.metadata().await?;
        if request.expected_revision != metadata.source.reference.revision {
            return conflict(metadata.source.reference.revision);
        }
        let resource = context
            .attachments()
            .get("package")
            .ok_or("package bundle required")?
            .clone();
        let mut ticket = context
            .scope()
            .resources
            .reserve()
            .map_err(|error| error.to_string())?;
        ticket.start();
        let parsed = tokio::task::spawn_blocking(move || {
            let parsed = read_bundle(&resource);
            drop(resource);
            ticket.complete(Ok(()));
            parsed
        })
        .await
        .map_err(|error| format!("package validation worker failed: {error}"))?;
        let package = match parsed {
            Ok(package) => package,
            Err(_) => return rejected(ManagementRejection::InvalidBundle),
        };
        let reference = package.application_ref();
        let mut loaded = self
            .store
            .load_application_at(&metadata.source.reference)
            .await
            .map_err(|error| error.to_string())?
            .ok_or("managed source not found")?;
        if !loaded
            .packages
            .iter()
            .any(|old| old.application_ref() == reference)
        {
            loaded.packages.push(package);
        }
        self.commit(
            request.expected_revision,
            loaded.source.input,
            loaded.source.remote_exports,
            loaded.packages,
            move |source| PackageInstalled {
                source,
                package: reference,
            },
        )
        .await
    }
    async fn replace(&self, input: Value) -> Result<Value> {
        let request: DeploymentReplace =
            serde_json::from_value(input).map_err(|error| error.to_string())?;
        let _write = self.writes.lock().await;
        let metadata = self.metadata().await?;
        if request.expected_revision != metadata.source.reference.revision {
            return conflict(metadata.source.reference.revision);
        }
        if request.input.environment != self.environment
            || (self.environment.location == ExecutionLocation::Host
                && !request.remote_exports.is_empty())
        {
            return rejected(ManagementRejection::ExecutionEnvironment);
        }
        if request
            .packages
            .iter()
            .any(|reference| !metadata.packages.contains(reference))
        {
            return rejected(ManagementRejection::PackageUnavailable);
        }
        let loaded = self
            .store
            .load_application_at(&metadata.source.reference)
            .await
            .map_err(|error| error.to_string())?
            .ok_or("managed source not found")?;
        let available: BTreeMap<_, _> = loaded
            .packages
            .into_iter()
            .map(|package| (package.application_ref(), package))
            .collect();
        let packages = request
            .packages
            .into_iter()
            .map(|reference| {
                available
                    .get(&reference)
                    .cloned()
                    .ok_or("installed package disappeared".to_string())
            })
            .collect::<Result<Vec<_>>>()?;
        self.commit(
            request.expected_revision,
            request.input,
            request.remote_exports,
            packages,
            |source| source,
        )
        .await
    }
    async fn commit<T: serde::Serialize>(
        &self,
        expected: u64,
        input: Input,
        observations: Vec<maka_plugin_core::deployment::RemoteObservation>,
        packages: Vec<Package>,
        result: impl FnOnce(DeploymentRef) -> T,
    ) -> Result<Value> {
        if input.environment != self.environment {
            return rejected(ManagementRejection::ExecutionEnvironment);
        }
        if validate_application(&input, &observations, &packages).is_err() {
            return rejected(ManagementRejection::InvalidSource);
        }
        match self
            .store
            .commit_application(
                self.source.deployment,
                expected,
                input,
                observations,
                packages,
            )
            .await
        {
            Ok(source) => serde_json::to_value(ManagementResult::Committed {
                value: result(source),
            })
            .map_err(|error| error.to_string()),
            Err(StoreError::RevisionConflict { actual, .. }) => conflict(
                actual
                    .parse()
                    .map_err(|_| "invalid managed revision conflict")?,
            ),
            // SQL and COMMIT uncertainty must never be converted to a known refusal.
            Err(error) => Err(error.to_string()),
        }
    }
}
fn desired_plan(loaded: &maka_event_log::applications::LoadedDeployment) -> Result<Plan> {
    let mut packages = maka_plugin_core::package::Packages::default();
    for package in &loaded.packages {
        packages
            .insert(
                package.application_ref(),
                package
                    .manifest()
                    .application
                    .clone()
                    .ok_or("application declaration required")?,
            )
            .map_err(|e| e.to_string())?;
    }
    let declarations = loaded
        .source
        .remote_exports
        .iter()
        .map(|r| (r.capability.clone(), r.declaration.clone()))
        .collect();
    Plan::prepare(&loaded.source.input, &packages, &declarations).map_err(|e| e.to_string())
}
fn rejected(reason: ManagementRejection) -> Result<Value> {
    serde_json::to_value(ManagementResult::<DeploymentRef>::Rejected { reason })
        .map_err(|error| error.to_string())
}
fn conflict(current_revision: u64) -> Result<Value> {
    serde_json::to_value(ManagementResult::<DeploymentRef>::Conflict { current_revision })
        .map_err(|error| error.to_string())
}

impl Handler for Manager {
    fn call(
        &self,
        method: String,
        input: Value,
        context: Context,
    ) -> BoxFuture<'static, Result<Value>> {
        let manager = self.clone();
        Box::pin(async move {
            match method.as_str() {
                "read" => {
                    serde_json::from_value::<DeploymentRead>(input)
                        .map_err(|error| error.to_string())?;
                    serde_json::to_value(manager.metadata().await?)
                        .map_err(|error| error.to_string())
                }
                "instances" => manager.instances(input).await,
                "setEnabled" => manager.set_enabled(input).await,
                "install" => manager.install(input, context).await,
                "replace" => manager.replace(input).await,
                _ => Err("unknown deployment management method".into()),
            }
        })
    }
}
impl NativeOwner for Manager {
    fn close(&self) -> BoxFuture<'static, Result<()>> {
        // Accepted requests and their blocking/SQL work belong to original
        // Host execution Scopes; no independent activation work is started.
        Box::pin(async { Ok(()) })
    }
}
fn read_bundle(resource: &Arc<dyn maka_plugins::call::ReadResource>) -> Result<Package> {
    use maka_plugins::call::ResourceEncoding;
    let length = resource.bytes();
    if resource.resource_type() != package_bundle_type()
        || resource.encoding() != ResourceEncoding::Bytes
        || length > maka_plugins::package::MAX_PACKAGE_BYTES * 2
    {
        return Err("invalid immutable package bundle resource".into());
    }
    let mut bytes = Vec::with_capacity(length);
    for offset in (0..length).step_by(RESOURCE_CHUNK_LIMIT) {
        let end = (offset + RESOURCE_CHUNK_LIMIT).min(length);
        let chunk = resource.read(offset..end)?;
        if chunk.len() != end - offset {
            return Err("package bundle snapshot length changed".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Package::from_bundle(&bytes).map_err(|error| error.to_string())
}

pub(super) fn provide(native: &mut NativeRegistry) -> Result<()> {
    provide_manager(native, DEPLOYMENT_MANAGER.into(), None)
}

/// Makes a fixed target available to an explicitly composed native entry. This
/// does not start an entry, publish a binding, or grant a caller any permission.
pub fn provide_target(
    native: &mut NativeRegistry,
    implementation: String,
    target: ManagementTarget,
) -> Result<()> {
    provide_manager(native, implementation, Some(target))
}

fn provide_manager(
    native: &mut NativeRegistry,
    implementation: String,
    target: Option<ManagementTarget>,
) -> Result<()> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Configuration {}
    // All explicitly composed manager instances in this original Application
    // share one bounded source-materialization lane. CAS remains the SQL authority.
    let writes = Arc::new(tokio::sync::Mutex::new(()));
    native.insert(
        implementation,
        Arc::new(move |context, configuration| {
            let writes = writes.clone();
            let target = target.clone();
            Box::pin(async move {
                let manager = (|| {
                    serde_json::from_value::<Configuration>(configuration)
                        .map_err(|error| error.to_string())?;
                    let persistence = context
                        .persistence()
                        .ok_or("deployment management requires the original persisted Root")?;
                    let manager = match target {
                        Some(target) => {
                            if !Arc::ptr_eq(persistence.store(), &target.0.store) {
                                return Err(
                                    "management target requires the same original Root owner"
                                        .into(),
                                );
                            }
                            target.0
                        }
                        None => Manager {
                            source: persistence.source().clone(),
                            store: persistence.store().clone(),
                            environment: maka_plugin_core::composition::Environment {
                                location: ExecutionLocation::Host,
                                presentation: None,
                            },
                            writes,
                        },
                    };
                    Ok::<_, String>(Arc::new(manager))
                })();
                match manager {
                    Ok(manager) => NativeActivation {
                        exports: Ok(BTreeMap::from([(
                            "Management".into(),
                            Published {
                                contract: deployment_manager_contract(),
                                value: manager.clone() as Arc<dyn Handler>,
                            },
                        )])),
                        owner: manager,
                    },
                    Err(error) => NativeActivation {
                        exports: Err(error),
                        owner: Arc::new(Uninitialized),
                    },
                }
            })
        }),
    )
}

struct Uninitialized;
impl NativeOwner for Uninitialized {
    fn close(&self) -> BoxFuture<'static, Result<()>> {
        Box::pin(async { Ok(()) })
    }
}
