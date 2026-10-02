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

use std::{
    sync::OnceLock,
    time::{Duration, Instant},
};

fn scheduler() -> tokio::runtime::Handle {
    static TIMER: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    TIMER
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_time()
                .build()
                .unwrap()
        })
        .handle()
        .clone()
}

use maka_js_runtime::plugin::{Foreground, ForegroundModule, Limits};
use serde_json::{Value, json};

fn realm() -> Foreground {
    Foreground::new(
        Limits {
            synchronous_slice: Duration::from_secs(1),
            ..Limits::default()
        },
        scheduler(),
    )
    .unwrap()
}

fn call(
    realm: &mut Foreground,
    module: &ForegroundModule,
    name: &str,
    args: &[Value],
) -> Result<Value, maka_js_runtime::plugin::Error> {
    realm.call(module, &[name.into()], args)
}

#[test]
fn foreground_runs_dynamic_interaction_without_ambient_io_or_tokio_context() {
    assert!(tokio::runtime::Handle::try_current().is_err());
    let mut realm = realm();
    let module = realm.load("graph.mjs", r#"
        export function capabilities() {
            return [typeof Deno, typeof fetch, typeof setTimeout, typeof TextEncoder, typeof URL, typeof __bootstrap, typeof console, typeof queueMicrotask];
        }
        export function drag(state, event) {
            const point = {x: Math.round(event.x / state.grid) * state.grid, y: Math.round(event.y / state.grid) * state.grid};
            const accepted = !state.occupied.some(node => node.x === point.x && node.y === point.y);
            return {state: {...state, preview: point}, commands: accepted ? [{kind:'move', node:event.node, point}] : [], consumed: accepted};
        }
    "#).unwrap();
    assert_eq!(
        call(&mut realm, &module, "capabilities", &[]).unwrap(),
        json!([
            "undefined",
            "undefined",
            "undefined",
            "function",
            "function",
            "undefined",
            "undefined",
            "undefined"
        ])
    );
    let state = json!({"grid":8,"occupied":[{"x":16,"y":24}]});
    let accepted = call(
        &mut realm,
        &module,
        "drag",
        &[state.clone(), json!({"node":"a","x":18,"y":35})],
    )
    .unwrap();
    assert_eq!(accepted["state"]["preview"], json!({"x":16,"y":32}));
    assert_eq!(accepted["commands"].as_array().unwrap().len(), 1);
    assert_eq!(accepted["consumed"], true);
    let rejected = call(
        &mut realm,
        &module,
        "drag",
        &[state, json!({"node":"a","x":18,"y":25})],
    )
    .unwrap();
    assert_eq!(rejected["commands"], json!([]));
    assert_eq!(rejected["consumed"], false);
    assert!(tokio::runtime::Handle::try_current().is_err());
}

#[test]
fn failed_handler_cannot_return_partial_intent_and_shared_realm_is_quarantined() {
    let mut realm = realm();
    let module = realm
        .load(
            "editor.mjs",
            r#"
        export function good(value) { return {commands:[{kind:'reset',value}],consumed:true}; }
        export function bad(value) {
            const commands = [{kind:'reset',value}];
            globalThis.uncommitted = commands;
            throw new Error('failure after staged reset');
        }
    "#,
        )
        .unwrap();
    let sibling = realm
        .load(
            "sibling.mjs",
            "export function read() { return globalThis.uncommitted; }",
        )
        .unwrap();
    let mut committed = call(
        &mut realm,
        &module,
        "good",
        &[json!("accepted native text")],
    )
    .unwrap();
    let before = committed.clone();
    let outcome = call(&mut realm, &module, "bad", &[json!("late stale reset")]);
    if let Ok(batch) = &outcome {
        committed = batch.clone();
    }
    assert_eq!(committed, before);
    assert!(
        outcome
            .unwrap_err()
            .0
            .contains("failure after staged reset")
    );
    assert!(realm.failure().is_some());
    assert!(call(&mut realm, &sibling, "read", &[]).is_err());
    realm.close(&module).unwrap();
    realm.close(&sibling).unwrap();
    let mut recovery = self::realm();
    let replacement = recovery
        .load("recovery.mjs", "export function read(){return 'recovery';}")
        .unwrap();
    assert_eq!(
        call(&mut recovery, &replacement, "read", &[]).unwrap(),
        json!("recovery")
    );
}

#[test]
fn synchronous_callbacks_reject_promises_and_budget_json_serialization() {
    let mut asynchronous = realm();
    let module = asynchronous
        .load(
            "async.mjs",
            "export async function handler(){return {consumed:true};}",
        )
        .unwrap();
    assert!(
        call(&mut asynchronous, &module, "handler", &[])
            .unwrap_err()
            .0
            .contains("synchronously")
    );
    assert!(asynchronous.failure().is_some());

    let mut serializing = Foreground::new(
        Limits {
            synchronous_slice: Duration::from_millis(100),
            ..Limits::default()
        },
        scheduler(),
    )
    .unwrap();
    let module = serializing
        .load(
            "getter.mjs",
            "export function handler(){return {toJSON(){while(true){}}};}",
        )
        .unwrap();
    let started = Instant::now();
    assert!(call(&mut serializing, &module, "handler", &[]).is_err());
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(serializing.failure().unwrap().0.contains("budget exceeded"));
}

#[test]
fn stalled_module_evaluation_and_handler_retire_the_entire_execution_domain() {
    for (source, handler) in [
        ("while(true){}; export function handler(){}", false),
        ("export function handler(){while(true){}}", true),
    ] {
        let mut realm = Foreground::new(
            Limits {
                synchronous_slice: Duration::from_millis(100),
                ..Limits::default()
            },
            scheduler(),
        )
        .unwrap();
        let started = Instant::now();
        let outcome = realm.load("stall.mjs", source);
        if handler {
            assert!(call(&mut realm, &outcome.unwrap(), "handler", &[]).is_err());
        } else {
            assert!(outcome.is_err());
        }
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(realm.failure().unwrap().0.contains("budget exceeded"));
        assert!(
            realm
                .load("later.mjs", "export function read(){return true;}")
                .is_err()
        );
    }
}

#[test]
fn module_handles_reject_cross_realm_and_post_close_dispatch_without_harming_siblings() {
    let mut first = realm();
    let mut second = realm();
    let module = first
        .load("first.mjs", "export function read(){return 'first';}")
        .unwrap();
    let sibling = first
        .load("sibling.mjs", "export function read(){return 'sibling';}")
        .unwrap();
    let other = second
        .load("second.mjs", "export function read(){return 'second';}")
        .unwrap();
    assert!(call(&mut second, &module, "read", &[]).is_err());
    assert!(second.failure().is_none());
    first.close(&module).unwrap();
    assert!(call(&mut first, &module, "read", &[]).is_err());
    assert_eq!(
        call(&mut first, &sibling, "read", &[]).unwrap(),
        json!("sibling")
    );
    assert_eq!(
        call(&mut second, &other, "read", &[]).unwrap(),
        json!("second")
    );
    // Opaque handles may outlive the realm; only the realm owns V8 Globals.
    drop(first);
    assert_eq!(
        call(&mut second, &other, "read", &[]).unwrap(),
        json!("second")
    );
    drop(module);
    drop(sibling);
}

#[test]
fn foreground_bounds_wire_results_and_rejects_non_json_before_copying() {
    for body in [
        "return 'x'.repeat(1024*1024+1);",
        "const x={};x.self=x;return x;",
        "return 1n;",
    ] {
        let mut realm = realm();
        let module = realm
            .load("wire.mjs", &format!("export function read(){{{body}}}"))
            .unwrap();
        assert!(call(&mut realm, &module, "read", &[]).is_err());
        assert!(realm.failure().is_some());
    }
    let mut realm = realm();
    let module = realm
        .load(
            "utf8.mjs",
            "export function read(){return '界'.repeat(400000);}",
        )
        .unwrap();
    assert!(
        call(&mut realm, &module, "read", &[])
            .unwrap_err()
            .0
            .contains("1 MiB")
    );
}

#[test]
fn oversized_input_never_dispatches_and_does_not_poison_the_realm() {
    let mut realm = realm();
    let module = realm
        .load(
            "counter.mjs",
            "let calls=0; export function read(){return ++calls;}",
        )
        .unwrap();
    assert!(
        call(
            &mut realm,
            &module,
            "read",
            &[json!("x".repeat(2 * 1024 * 1024))]
        )
        .is_err()
    );
    assert!(realm.failure().is_none());
    assert_eq!(call(&mut realm, &module, "read", &[]).unwrap(), json!(1));
}

#[test]
fn ready_jobs_fail_or_stall_before_the_current_callback_batch_can_commit() {
    for job in ["throw new Error('deferred failure');", "while(true){}"] {
        let mut realm = Foreground::new(
            Limits {
                synchronous_slice: Duration::from_millis(100),
                ..Limits::default()
            },
            scheduler(),
        )
        .unwrap();
        let source = format!(
            "export function handler(){{Promise.resolve().then(()=>{{{job}}});return {{commands:[{{kind:'reset',value:'stale'}}],consumed:true}};}}"
        );
        let module = realm.load("jobs.mjs", &source).unwrap();
        let started = Instant::now();
        assert!(call(&mut realm, &module, "handler", &[]).is_err());
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(realm.failure().is_some());
    }
}
