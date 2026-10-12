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
use maka_host_runtime::application::Caller;
use std::sync::Mutex;

struct Evidence {
    allowed: bool,
    caller: Option<Caller>,
}
struct CallerPolicy(Issuer);
impl Policy for CallerPolicy {
    fn admit(&self, invocation: Invocation<'_>) -> Result<()> {
        let evidence = self
            .0
            .evidence::<Evidence>(invocation.scope)
            .ok_or("foreign source")?;
        if evidence.allowed || invocation.provider.instance() == &id(4) {
            Ok(())
        } else {
            Err("leaf denied".into())
        }
    }
    fn caller(&self, scope: &Scope) -> Option<Caller> {
        self.0.evidence::<Evidence>(scope)?.caller.clone()
    }
}
struct Observe {
    forward: bool,
    observed: Arc<Mutex<Vec<Option<Caller>>>>,
}
impl Handler for Observe {
    fn call(
        &self,
        method: String,
        input: Value,
        context: Context,
    ) -> BoxFuture<'static, Result<Value>> {
        self.observed
            .lock()
            .unwrap()
            .push(context.caller().cloned());
        let forward = self.forward;
        Box::pin(async move {
            if forward {
                context
                    .call("echo", 0, method, input, Attachments::default())?
                    .await
            } else {
                Ok(input)
            }
        })
    }
}

#[tokio::test]
async fn caller_observation_survives_forwarding_without_granting_the_leaf() {
    let provider = native_package();
    let consumer = consumer();
    let implementation = Implementation::Native {
        implementation: "native.wrapper".into(),
    };
    let mut declaration = entry(true, implementation.clone(), MethodMode::Query);
    declaration.imports = entry(false, implementation, MethodMode::Query).imports;
    let wrapper = package("wrapper", declaration, None);
    let mut composition = input(&provider, &consumer);
    composition
        .instances
        .insert("wrapper".into(), node(&wrapper, 4, false));
    composition
        .instances
        .get_mut("a")
        .unwrap()
        .bindings
        .get_mut("echo")
        .unwrap()[0] = Target::Instance {
        path: vec!["wrapper".into()],
        export: "echo".into(),
    };
    let observed = Arc::new(Mutex::new(Vec::new()));
    let mut native = NativeRegistry::default();
    for (name, forward) in [("native.echo", false), ("native.wrapper", true)] {
        let observed = observed.clone();
        native
            .insert(
                name.into(),
                Arc::new(move |_, _| {
                    let observed = observed.clone();
                    Box::pin(async move {
                        NativeActivation {
                            exports: Ok(BTreeMap::from([(
                                "echo".into(),
                                Published {
                                    contract: contract(MethodMode::Query),
                                    value: Arc::new(Observe { forward, observed })
                                        as Arc<dyn Handler>,
                                },
                            )])),
                            owner: native_owner(|| Box::pin(async { Ok(()) })),
                        }
                    })
                }),
            )
            .unwrap();
    }
    let issuer = Issuer::default();
    let app = Application::start(
        composition,
        vec![provider, consumer, wrapper],
        native,
        issuer.clone(),
        Arc::new(CallerPolicy(issuer.clone())),
        Some(maka_host_js::factory(Limits::default())),
    )
    .unwrap();
    app.ready().await.unwrap();
    let client = app.client();
    let binding = client
        .bind(client.activation(&id(2)).unwrap(), "echo".into())
        .await
        .unwrap()
        .pop()
        .unwrap();
    let caller = Caller::new(
        Arc::new(()),
        maka_plugin_core::deployment::DeploymentRef {
            state_domain: "original-ui-root".into(),
            deployment: Uuid::new_v4(),
            revision: 1,
        },
        id(2),
        Uuid::new_v4(),
    );
    for (allowed, origin) in [
        (true, Some(caller.clone())),
        (false, Some(caller.clone())),
        (true, None),
    ] {
        observed.lock().unwrap().clear();
        let scope = issuer
            .issue_with(
                Identity::Remote {
                    request_id: Uuid::new_v4(),
                },
                Evidence {
                    allowed,
                    caller: origin.clone(),
                },
                CancellationToken::new(),
            )
            .unwrap();
        let result = client
            .start(
                binding.clone(),
                "echo".into(),
                json!("original"),
                scope.clone(),
                Attachments::default().into(),
            )
            .await
            .unwrap()
            .result()
            .await;
        if allowed {
            assert_eq!(result.unwrap(), json!("original"));
        } else {
            assert!(result.unwrap_err().contains("leaf denied"));
        }
        scope.finish().await.unwrap();
        let observations = observed.lock().unwrap();
        assert_eq!(observations.len(), if allowed { 2 } else { 1 });
        assert!(observations.iter().all(|value| value == &origin));
    }
    app.shutdown().await.unwrap();
}

#[tokio::test]
async fn ordinary_admission_does_not_create_an_authenticated_caller() {
    let provider = native_package();
    let consumer = consumer();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let native = registry(Arc::new(Observe {
        forward: false,
        observed: observed.clone(),
    }));
    let issuer = Issuer::default();
    let app = Application::start(
        input(&provider, &consumer),
        vec![provider, consumer],
        native,
        issuer.clone(),
        Arc::new(Grant(issuer.clone())),
        Some(maka_host_js::factory(Limits::default())),
    )
    .unwrap();
    app.ready().await.unwrap();
    let client = app.client();
    let binding = client
        .bind(client.activation(&id(2)).unwrap(), "echo".into())
        .await
        .unwrap()
        .pop()
        .unwrap();
    let scope = scope(&issuer, true);
    assert_eq!(
        client
            .start(
                binding,
                "echo".into(),
                json!("allowed"),
                scope.clone(),
                Attachments::default().into()
            )
            .await
            .unwrap()
            .result()
            .await
            .unwrap(),
        json!("allowed")
    );
    scope.finish().await.unwrap();
    assert_eq!(*observed.lock().unwrap(), vec![None]);
    app.shutdown().await.unwrap();
}
