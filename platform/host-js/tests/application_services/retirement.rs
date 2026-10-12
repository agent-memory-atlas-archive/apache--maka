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
use maka_host_runtime::application::NativeRetirement;
use tokio::sync::{Mutex, OwnedMutexGuard};

#[derive(Default)]
struct State {
    busy: bool,
    retired: bool,
}
struct Native {
    state: Arc<Mutex<State>>,
    entered: Arc<Semaphore>,
    release: Arc<Semaphore>,
    closing: Arc<Semaphore>,
    closed: Arc<Semaphore>,
}
impl Native {
    fn new(busy: bool) -> Arc<Self> {
        Arc::new(Self {
            state: Arc::new(Mutex::new(State {
                busy,
                retired: false,
            })),
            entered: Arc::new(Semaphore::new(0)),
            release: Arc::new(Semaphore::new(0)),
            closing: Arc::new(Semaphore::new(0)),
            closed: Arc::new(Semaphore::new(0)),
        })
    }
}
struct Idle(OwnedMutexGuard<State>);
impl NativeRetirement for Idle {
    fn commit(mut self: Box<Self>) {
        self.0.retired = true;
    }
}
impl NativeOwner for Native {
    fn prepare_retirement(&self) -> Option<Box<dyn NativeRetirement>> {
        let state = self.state.clone().try_lock_owned().ok()?;
        if state.busy {
            return None;
        }
        Some(Box::new(Idle(state)))
    }
    fn close(&self) -> BoxFuture<'static, Result<()>> {
        let closing = self.closing.clone();
        let closed = self.closed.clone();
        Box::pin(async move {
            closing.add_permits(1);
            permit(&closed).await;
            Ok(())
        })
    }
}
impl Handler for Native {
    fn call(&self, _: String, input: Value, _: Context) -> BoxFuture<'static, Result<Value>> {
        let entered = self.entered.clone();
        let release = self.release.clone();
        Box::pin(async move {
            entered.add_permits(1);
            permit(&release).await;
            Ok(input)
        })
    }
}

#[tokio::test]
async fn idle_retirement_fences_background_work_rolls_back_busy_and_owns_cleanup() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let provider = native_package();
        let consumer = consumer();
        let mut source = input(&provider, &consumer);
        let mut second = source.instances["provider"].clone();
        second.key = Uuid::from_u128(4);
        source.instances.insert("second".into(), second);
        let first = Native::new(false);
        let second = Native::new(true);
        let mut native = NativeRegistry::default();
        native
            .insert(
                "native.echo".into(),
                Arc::new({
                    let first = first.clone();
                    let second = second.clone();
                    move |context, _| {
                        let owner = if context.activation().instance() == &id(1) {
                            first.clone()
                        } else {
                            second.clone()
                        };
                        Box::pin(async move {
                            NativeActivation {
                                exports: Ok(BTreeMap::from([(
                                    "echo".into(),
                                    Published {
                                        contract: contract(MethodMode::Query),
                                        value: owner.clone() as Arc<dyn Handler>,
                                    },
                                )])),
                                owner,
                            }
                        })
                    }
                }),
            )
            .unwrap();
        let issuer = Issuer::default();
        let app = Application::start(
            source,
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
            .remove(0);
        // No RPC remains, but this owner still has accepted background work.
        assert!(!app.shutdown_if_idle().await.unwrap());
        assert!(!first.state.try_lock().unwrap().retired);
        assert!(!client.snapshot().closing);
        let call = client
            .start(
                binding.clone(),
                "echo".into(),
                json!("accepted"),
                scope(&issuer, true),
                Attachments::new().into(),
            )
            .await
            .unwrap();
        permit(&first.entered).await;
        second.state.lock().await.busy = false;
        assert!(!app.shutdown_if_idle().await.unwrap());
        first.release.add_permits(1);
        assert_eq!(call.result().await.unwrap(), json!("accepted"));
        // Once committed, losing the public observer cannot release Root early.
        {
            let retirement = app.shutdown_if_idle();
            tokio::pin!(retirement);
            tokio::select! {
                result = &mut retirement => panic!("cleanup was not settled: {result:?}"),
                () = permit(&first.closing) => {}
            }
        }
        assert!(first.state.lock().await.retired);
        assert!(second.state.lock().await.retired);
        assert!(!client.snapshot().closed);
        assert!(
            client
                .start(
                    binding,
                    "echo".into(),
                    json!("late"),
                    scope(&issuer, true),
                    Attachments::new().into()
                )
                .await
                .is_err()
        );
        first.closed.add_permits(1);
        second.closed.add_permits(1);
        assert!(app.shutdown_if_idle().await.unwrap());
        assert!(client.snapshot().closed);
        assert!(app.shutdown_if_idle().await.unwrap());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn unsupported_native_owner_is_busy_without_closing_admission() {
    let provider = native_package();
    let consumer = consumer();
    let issuer = Issuer::default();
    let app = Application::start(
        input(&provider, &consumer),
        vec![provider, consumer],
        registry(Arc::new(Echo {
            calls: Arc::new(AtomicUsize::new(0)),
        })),
        issuer.clone(),
        Arc::new(Grant(issuer)),
        Some(maka_host_js::factory(Limits::default())),
    )
    .unwrap();
    app.ready().await.unwrap();
    assert!(!app.shutdown_if_idle().await.unwrap());
    assert!(!app.client().snapshot().closing);
    app.shutdown().await.unwrap();
}
