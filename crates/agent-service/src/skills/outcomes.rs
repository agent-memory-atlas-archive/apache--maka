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

//! Original publication receipts remain readable without a live skill library.
use super::*;
use cap_fs_ext::DirExt;
use maka_event_log::root::RootOwner;
use maka_plugin_core::{
    contract::{Interface, Method, MethodMode, Transport},
    operation::{OperationReply, Receipt, RecoveryRef},
};

pub const IMPLEMENTATION: &str = "maka.skills.outcomes";
pub const EXPORT: &str = "Outcomes";

pub fn contract() -> maka_plugin_core::contract::Contract {
    maka_plugin_core::contract::Contract {
        name: "maka.skills.Outcomes".into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service {
            methods: BTreeMap::from([(
                "read".into(),
                Method {
                    mode: MethodMode::Query,
                    input: schemars::schema_for!(RecoveryRef).into(),
                    output: schemars::schema_for!(super::contract::Outcome).into(),
                    resources: BTreeMap::new(),
                },
            )]),
        },
    }
}
pub(super) fn provide(registry: &mut NativeRegistry) -> Result<(), String> {
    registry.insert(
        IMPLEMENTATION.into(),
        Arc::new(|context, configuration| {
            Box::pin(async move {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Configuration {}
                let root = (|| {
                    serde_json::from_value::<Configuration>(configuration)
                        .map_err(|_| "skills_configuration_invalid")?;
                    context
                        .persistence()
                        .ok_or("skills_require_persistence")?
                        .store()
                        .owned_root()
                        .map_err(|_| "skills_root_unavailable")
                })();
                match root {
                    Ok(root) => {
                        let owner = Arc::new(Reader { root });
                        NativeActivation {
                            exports: Ok(BTreeMap::from([(
                                EXPORT.into(),
                                maka_plugin_core::runtime::Published {
                                    contract: contract(),
                                    value: owner.clone() as Arc<dyn Handler>,
                                },
                            )])),
                            owner,
                        }
                    }
                    Err(error) => NativeActivation {
                        exports: Err(error.into()),
                        owner: Arc::new(crate::Uninitialized),
                    },
                }
            })
        }),
    )
}
#[derive(Clone)]
struct Reader {
    root: Arc<RootOwner>,
}
impl NativeOwner for Reader {
    fn close(&self) -> BoxFuture<'static, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }
}
impl Handler for Reader {
    fn call(
        &self,
        method: String,
        input: Value,
        context: Context,
    ) -> BoxFuture<'static, Result<Value, String>> {
        let reader = self.clone();
        Box::pin(async move {
            if method != "read" {
                return Err("skills_outcome_method_invalid".into());
            }
            let reference: RecoveryRef =
                serde_json::from_value(input).map_err(|_| "skills_recovery_invalid")?;
            reference
                .validate()
                .map_err(|_| "skills_recovery_invalid")?;
            let current = super::contract();
            let compatible = match (&reference.contract.interface, &current.interface) {
                (
                    Interface::Service { methods: original },
                    Interface::Service { methods: current },
                ) => original.get("apply") == current.get("apply"),
                _ => false,
            };
            if reference.export != super::EXPORT
                || reference.method != "apply"
                || reference.contract.name != current.name
                || reference.contract.version != current.version
                || reference.contract.transport != current.transport
                || !compatible
            {
                return Err("skills_recovery_contract_unsupported".into());
            }
            // Only the actual Host's independent historical grant can release
            // original records. A reference is a locator, never a file grant.
            let observation = context.recover(reference.clone())?.await?;
            let (receipt, published) = match observation {
                None => (Receipt::NotRecorded, None),
                Some(observation) => {
                    let namespace = crate::owner::namespace(
                        &reference.source.state_domain,
                        reference.source.deployment,
                        &reference.namespace,
                    );
                    let published = match reader
                        .capture(namespace, reference.operation, context.scope())
                        .await?
                    {
                        None => None,
                        Some(root) => maka_skills::publication::receipt::read_view(
                            &root.bind_scope(context.scope()),
                            &management::operation(&reference)?,
                        )
                        .await
                        .map_err(|_| "skills_receipt_unavailable")?
                        .map(management::recorded)
                        .transpose()?,
                    };
                    (
                        Receipt::Recorded {
                            outcome: observation.outcome,
                            cleanup: observation.cleanup,
                        },
                        published,
                    )
                }
            };
            // Use the Host's original receipt delivery ceiling, not the smaller
            // catalog-page budget. Preserve original diagnostics and cleanup.
            serde_json::to_value(super::contract::Outcome {
                original: OperationReply { reference, receipt },
                published,
            })
            .map_err(|_| "skills_encoding_failed".into())
        })
    }
}
impl Reader {
    async fn capture(
        &self,
        namespace: String,
        operation: uuid::Uuid,
        scope: &Scope,
    ) -> Result<Option<ReadRoot>, String> {
        let root = self.root.clone();
        let cancellation = scope.cancellation.clone();
        let mut ticket = scope.resources.reserve().map_err(|_| "skills_cancelled")?;
        tokio::task::spawn_blocking(move || {
            ticket.start();
            let result = (|| {
                if cancellation.is_cancelled() {
                    return Err(std::io::Error::other("cancelled"));
                }
                root.validate_current().map_err(std::io::Error::other)?;
                let directory = cap_std::fs::Dir::open_ambient_dir(
                    root.canonical_path(),
                    cap_std::ambient_authority(),
                )
                .map_err(std::io::Error::other)?;
                root.validate_directory_handle(&directory.try_clone()?.into_std_file())
                    .map_err(std::io::Error::other)?;
                let directory = directory
                    .open_dir_nofollow("skill-libraries")?
                    .open_dir_nofollow(&namespace)?
                    .open_dir_nofollow("receipts")?;
                let view = ReadRoot::from_handle(
                    directory,
                    root.canonical_path()
                        .join("skill-libraries")
                        .join(namespace)
                        .join("receipts"),
                )
                .select(BTreeSet::from([format!("{operation}.json")]))
                .map_err(std::io::Error::other)?;
                Ok(view)
            })();
            drop(root);
            ticket.complete(Ok(()));
            match result {
                Ok(view) => Ok(Some(view)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(_) => Err("skills_receipt_unavailable".to_owned()),
            }
        })
        .await
        .map_err(|_| "skills_receipt_unavailable")?
    }
}
