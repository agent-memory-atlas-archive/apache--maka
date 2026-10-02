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
use maka_js_runtime::plugin::Captured;
use std::sync::Arc;

async fn capture(module: &Module, name: &str, method: &str) -> Captured {
    module
        .capture(vec![name.into()], vec![method.into()])
        .await
        .unwrap()
}

#[tokio::test]
async fn worker_registration_freezes_async_functions_receiver_and_module_identity() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let vm = Vm::new(Limits::default()).unwrap();
        let first = vm.load("first.mjs".into(), r#"
            export let service = { label: 'first', async read(input) { return this.label + ':' + input; } };
            export function replace() { service.read = () => 'changed function'; service = {read: () => 'changed namespace'}; }
        "#.into()).unwrap();
        let second = vm.load("second.mjs".into(), "export const service = { label:'second', async read(input) { return this.label + ':' + input; } };".into()).unwrap();
        let old = capture(&first, "service", "read").await;
        let other = capture(&second, "service", "read").await;
        first.call(vec!["replace".into()], vec![]).await.unwrap();
        assert_eq!(old.call("read", vec![json!("draft")]).await.unwrap(), json!("first:draft"));
        assert_eq!(other.call("read", vec![json!("draft")]).await.unwrap(), json!("second:draft"));
        assert!(old.call("replace", vec![]).await.is_err());
        assert_eq!(vm.statistics(false).await.unwrap().captured_callbacks, 2);
        drop(old);
        assert_eq!(vm.statistics(false).await.unwrap().captured_callbacks, 1);
        first.close().await.unwrap();
        assert!(capture_result(&first).await.is_err());
        assert_eq!(other.call("read", vec![json!("live")]).await.unwrap(), json!("second:live"));
        second.close().await.unwrap();
        assert!(other.call("read", vec![]).await.is_err());
        assert_eq!(vm.statistics(false).await.unwrap().captured_callbacks, 0);
        vm.shutdown().await;
    }).await.unwrap();
}
async fn capture_result(module: &Module) -> Result<Captured, Error> {
    module
        .capture(vec!["service".into()], vec!["read".into()])
        .await
}

#[tokio::test]
async fn unclaimed_worker_registration_payload_releases_globals_and_capacity() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let vm = Vm::new(Limits::default()).unwrap();
        let module = vm
            .load(
                "capture-drop.mjs".into(),
                "export const service = {read:()=>1};".into(),
            )
            .unwrap();
        module.ready().await.unwrap();
        let mut future = Box::pin(capture_result(&module));
        assert!(futures_util::poll!(future.as_mut()).is_pending());
        // Observe successful worker capture while the awaiting future is never polled again.
        assert_eq!(vm.statistics(false).await.unwrap().captured_callbacks, 1);
        drop(future);
        assert_eq!(vm.statistics(false).await.unwrap().captured_callbacks, 0);
        let mut handles = Vec::new();
        for _ in 0..1024 {
            handles.push(capture_result(&module).await.unwrap());
        }
        assert!(
            capture_result(&module)
                .await
                .err()
                .unwrap()
                .to_string()
                .contains("registration limit")
        );
        handles.pop();
        handles.push(capture_result(&module).await.unwrap());
        assert_eq!(vm.statistics(false).await.unwrap().captured_callbacks, 1024);
        drop(handles);
        assert_eq!(vm.statistics(false).await.unwrap().captured_callbacks, 0);
        module.close().await.unwrap();
        vm.shutdown().await;
    })
    .await
    .unwrap();
}

#[derive(Clone)]
struct Gate {
    started: Arc<tokio::sync::Semaphore>,
    release: Arc<tokio::sync::Semaphore>,
    finished: Arc<tokio::sync::Semaphore>,
}
impl Gate {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            started: Arc::new(tokio::sync::Semaphore::new(0)),
            release: Arc::new(tokio::sync::Semaphore::new(0)),
            finished: Arc::new(tokio::sync::Semaphore::new(0)),
        })
    }
}
impl Bridge for Gate {
    fn call(
        &self,
        method: String,
        _: Value,
    ) -> futures_util::future::BoxFuture<'static, Result<Value, Error>> {
        let gate = self.clone();
        Box::pin(async move {
            match method.as_str() {
                "wait" => {
                    gate.started.add_permits(1);
                    let _permit = gate.release.acquire_owned().await.unwrap();
                    Ok(json!("captured"))
                }
                "finished" => {
                    gate.finished.add_permits(1);
                    Ok(Value::Null)
                }
                _ => panic!("unexpected bridge method"),
            }
        })
    }
}
const GATED: &str = r#"
    export const service = { async read(key) {
        const value = await Deno.core.ops.op_maka_plugin(key, 'wait', null);
        await Deno.core.ops.op_maka_plugin(key, 'finished', null);
        return value;
    }};
"#;

#[tokio::test]
async fn accepted_promises_survive_all_external_vm_module_handles_and_waiters() {
    tokio::time::timeout(Duration::from_secs(10), async {
        for bound in [false, true] {
            let gate = Gate::new();
            let vm = Vm::new(Limits::default()).unwrap();
            let module = vm
                .load_with_bridge("last-owner.mjs".into(), GATED.into(), Some(gate.clone()))
                .unwrap();
            let callbacks = capture(&module, "service", "read").await;
            let weak = module.downgrade();
            let args = vec![json!(module.host_key())];
            let mut pending = if bound {
                Box::pin(callbacks.call("read", args))
                    as std::pin::Pin<Box<dyn Future<Output = Result<Value, Error>>>>
            } else {
                Box::pin(module.call(vec!["service".into(), "read".into()], args))
                    as std::pin::Pin<Box<dyn Future<Output = Result<Value, Error>>>>
            };
            assert!(futures_util::poll!(pending.as_mut()).is_pending());
            let _started = gate.started.acquire().await.unwrap();
            assert_eq!(vm.statistics(false).await.unwrap().pending_calls, 1);
            drop(pending);
            drop(callbacks);
            drop(module);
            drop(vm);
            assert!(
                weak.upgrade().is_some(),
                "accepted Request is now the sole module/VM owner"
            );
            gate.release.add_permits(1);
            let _finished = gate.finished.acquire().await.unwrap();
            while weak.upgrade().is_some() {
                tokio::task::yield_now().await;
            }
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn explicit_close_waits_for_captured_work_after_reply_observer_is_abandoned() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let gate = Gate::new();
        let vm = Vm::new(Limits::default()).unwrap();
        let module = vm
            .load_with_bridge("close-owned.mjs".into(), GATED.into(), Some(gate.clone()))
            .unwrap();
        let callbacks = capture(&module, "service", "read").await;
        let mut pending = Box::pin(callbacks.call("read", vec![json!(module.host_key())]));
        assert!(futures_util::poll!(pending.as_mut()).is_pending());
        let _started = gate.started.acquire().await.unwrap();
        drop(pending);
        let closing = tokio::spawn({
            let module = module.clone();
            async move { module.close().await }
        });
        while module.ready().await.is_ok() {
            tokio::task::yield_now().await;
        }
        assert!(!closing.is_finished());
        assert_eq!(vm.statistics(false).await.unwrap().pending_calls, 1);
        assert!(callbacks.call("read", vec![]).await.is_err());
        gate.release.add_permits(1);
        closing.await.unwrap().unwrap();
        let _finished = gate.finished.acquire().await.unwrap();
        assert_eq!(vm.statistics(false).await.unwrap().pending_calls, 0);
        assert_eq!(vm.statistics(false).await.unwrap().captured_callbacks, 0);
        vm.shutdown().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn worker_registration_checks_input_result_json_and_registration_shapes() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let vm = Vm::new(Limits::default()).unwrap();
        let module = vm.load("json-boundary.mjs".into(), r#"
            let calls = 0;
            export const service = {read(value) { calls++; return value; }, large() {return '界'.repeat(400000);}, json() {return {toJSON(){return {calls};}};}, circular() { const value={}; value.self=value; return value; }};
        "#.into()).unwrap();
        for methods in [vec![], vec!["read".into(),"read".into()], vec!["x".repeat(129)]] {
            assert!(module.capture(vec!["service".into()], methods).await.is_err());
        }
        assert!(module.capture(vec!["service".into();33], vec!["read".into()]).await.is_err());
        let callbacks = module.capture(vec!["service".into()], vec!["read".into(),"large".into(),"json".into(),"circular".into()]).await.unwrap();
        assert!(callbacks.call("read", vec![json!("x".repeat(1024*1024))]).await.is_err());
        for value in [json!(u64::MAX), json!(i64::MAX), json!(9_007_199_254_740_993_u64)] {
            assert!(callbacks.call("read", vec![json!({"nested":[value]})]).await
                .unwrap_err().to_string().contains("represented exactly"));
        }
        assert_eq!(callbacks.call("json", vec![]).await.unwrap(), json!({"calls":0}));
        assert!(callbacks.call("large", vec![]).await.unwrap_err().to_string().contains("exceeds 1 MiB"));
        assert!(callbacks.call("circular", vec![]).await.is_err());
        assert_eq!(callbacks.call("read", vec![json!("valid")]).await.unwrap(), json!("valid"));
        assert!(!vm.is_terminated(), "ordinary invalid values do not terminate the whole worker");
        module.close().await.unwrap();
        vm.shutdown().await;
    }).await.unwrap();
}

#[tokio::test]
async fn capture_getters_and_result_to_json_stalls_terminate_the_actual_worker_domain() {
    tokio::time::timeout(Duration::from_secs(10), async {
        for capture_stall in [true, false] {
            let vm = Vm::new(Limits {
                synchronous_slice: Duration::from_millis(200),
                ..Limits::default()
            })
            .unwrap();
            let source = if capture_stall {
                "export const service = {get read(){while(true){}}};"
            } else {
                "export const service = {read(){return {toJSON(){while(true){}}};}};"
            };
            let module = vm.load("stall.mjs".into(), source.into()).unwrap();
            if capture_stall {
                assert!(capture_result(&module).await.is_err());
            } else {
                let callbacks = capture_result(&module).await.unwrap();
                assert!(callbacks.call("read", vec![]).await.is_err());
            }
            assert!(vm.is_terminated());
            module.close().await.unwrap();
            vm.shutdown().await;
        }
    })
    .await
    .unwrap();
}

struct Inexact;
impl Bridge for Inexact {
    fn call(
        &self,
        _: String,
        _: Value,
    ) -> futures_util::future::BoxFuture<'static, Result<Value, Error>> {
        Box::pin(async { Ok(json!({"nested":[u64::MAX]})) })
    }
}
#[tokio::test]
async fn native_bridge_rejects_inexact_integer_results_without_bigint_or_rounding() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let vm = Vm::new(Limits::default()).unwrap();
        let module = vm
            .load_with_bridge(
                "inexact-output.mjs".into(),
                r#"
            export const service = {async read(key) {
                try { await Deno.core.ops.op_maka_plugin(key, 'read', null); return false; }
                catch(error) { return error.message.includes('represented exactly'); }
            }};
        "#
                .into(),
                Some(Arc::new(Inexact)),
            )
            .unwrap();
        let callbacks = capture_result(&module).await.unwrap();
        assert_eq!(
            callbacks
                .call("read", vec![json!(module.host_key())])
                .await
                .unwrap(),
            json!(true)
        );
        assert!(!vm.is_terminated());
        module.close().await.unwrap();
        vm.shutdown().await;
    })
    .await
    .unwrap();
}

struct CountInput(Arc<std::sync::atomic::AtomicUsize>);
impl Bridge for CountInput {
    fn call(
        &self,
        _: String,
        input: Value,
    ) -> futures_util::future::BoxFuture<'static, Result<Value, Error>> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async move { Ok(input) })
    }
}
#[tokio::test]
async fn native_bridge_checks_vm_json_input_before_copy_and_before_dispatch() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let vm = Vm::new(Limits::default()).unwrap();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let module = vm.load_with_bridge("input-json.mjs".into(), r#"
            export const service = { async read(key, mode) {
                const input = mode==='bigint' ? {nested:[9007199254740993n]} : {text:'x'.repeat(32*1024*1024)};
                try { await Deno.core.ops.op_maka_plugin(key, 'echo', input); return false; }
                catch(error) { return error.message.includes(mode==='bigint'?'BigInt':'32 MiB'); }
            }};
        "#.into(), Some(Arc::new(CountInput(calls.clone())))).unwrap();
        let callbacks = capture_result(&module).await.unwrap();
        for mode in ["bigint", "large"] {
            assert_eq!(callbacks.call("read", vec![json!(module.host_key()),json!(mode)]).await.unwrap(), json!(true));
        }
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert!(!vm.is_terminated());
        module.close().await.unwrap();
        vm.shutdown().await;
    }).await.unwrap();
}
