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
use maka_plugin_core::composition::Target;
use std::time::{Duration, Instant};

fn secondary(f: &Fixture, configuration: &mut Configuration) {
    let owner =
        Arc::new(RootOwner::create(&f.directory.path().join("secondary"), &f.namespaces).unwrap());
    let store = f.timer.block_on(EventLog::for_root(owner)).unwrap();
    let (input, packages) = support::host();
    f.timer
        .block_on(store.commit_application(f.host, 0, input, vec![], packages))
        .unwrap();
    f.timer.block_on(store.shutdown()).unwrap();
    drop(store);
    let mut host = configuration.hosts[0].clone();
    host.name = "secondary".into();
    host.location = HostLocation::Process {
        root: f.directory.path().join("secondary"),
    };
    configuration.hosts.push(host);
}

fn commit(f: &Fixture, expected: u64, host: &str) -> DeploymentRef {
    let (mut input, packages, mut observations) = support::ui(f.target);
    for node in input.instances.values_mut() {
        for target in node.bindings.values_mut().flatten() {
            if let Target::Remote { deployment, .. } = target {
                *deployment = host.into();
            }
        }
    }
    for observation in &mut observations {
        observation.capability.deployment = host.into();
    }
    f.timer
        .block_on(f.caller.commit_application(
            f.source.deployment,
            expected,
            input,
            observations,
            packages,
        ))
        .unwrap()
}

fn next(
    updates: &mut maka_application_bridge::Updates,
    app: &Application,
) -> Result<maka_application_bridge::ApprovedUpdate, String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match updates.poll(app)? {
            Some(update) => return Ok(update),
            None => {
                assert!(
                    Instant::now() < deadline,
                    "committed source did not converge"
                );
                std::thread::yield_now();
            }
        }
    }
}

#[test]
fn unselected_host_opens_only_when_the_original_updates_pipeline_selects_it() {
    let f = Fixture::new();
    let mut configuration = f.configuration(true, true);
    secondary(&f, &mut configuration);
    let mut connections = f.connect(configuration);
    f.reopened("secondary");
    let mut app = f.ui(&mut connections);
    let mount = app.mount_launch(json!({})).unwrap();
    let connections = Arc::new(tokio::sync::Mutex::new(connections));
    let mut updates = maka_application_bridge::Updates::new(
        f.caller.clone(),
        f.source.clone(),
        connections.clone(),
        f.timer.handle().clone(),
    )
    .unwrap();
    let selected = commit(&f, 1, "secondary");
    let update = next(&mut updates, &app).unwrap();
    assert_eq!(update.prepared.source(), &selected);
    assert!(
        update
            .registrations
            .iter()
            .all(|r| r.target.deployment == "secondary")
    );
    f.blocked("secondary");
    app.adopt_prepared(update.prepared, update.registrations)
        .unwrap();
    assert_eq!(app.source(), Some(&selected));
    updates.stop();
    f.timer.block_on(updates.drain());
    app.retire(&mount).unwrap();
    app.close(&mount, Ok(())).unwrap();
    app.shutdown().unwrap();
    drop(app);
    f.timer
        .block_on(async { connections.lock().await.close().await })
        .unwrap();
    f.reopened("host");
    f.reopened("secondary");
    drop(connections);
    f.timer.block_on(f.caller.shutdown()).unwrap();
}

#[test]
fn failed_deferred_owner_preserves_old_ui_and_allows_a_corrected_source() {
    for lost_capture in [false, true] {
        if lost_capture && !cfg!(unix) {
            continue;
        }
        let f = Fixture::new();
        let mut configuration = f.configuration(true, true);
        secondary(&f, &mut configuration);
        if !lost_capture {
            configuration.hosts[1]
                .imports
                .get_mut("business")
                .unwrap()
                .methods
                .insert("undeclared".into());
        }
        let mut connections = if lost_capture {
            // The proxy runs the actual Host and corrupts only secondary's first
            // capture acknowledgement. EOF sends Close on the intact inner channel.
            let script = f.directory.path().join("control-proxy.py");
            std::fs::write(&script, CAPTURE_PROXY).unwrap();
            f.timer
                .block_on(Connections::prepare(
                    f.caller.clone(),
                    f.source.deployment,
                    f.target,
                    configuration,
                    f.namespaces.clone(),
                    HostExecutable::new(
                        "/usr/bin/env".into(),
                        vec![
                            "python3".into(),
                            script.into_os_string(),
                            env!("CARGO_BIN_EXE_maka").into(),
                        ],
                    )
                    .unwrap(),
                    Default::default(),
                ))
                .unwrap()
        } else {
            f.prepare(configuration).unwrap()
        };
        f.timer.block_on(connections.start()).unwrap();
        f.reopened("secondary");
        let mut app = f.ui(&mut connections);
        let mount = app.mount_launch(json!({})).unwrap();
        let connections = Arc::new(tokio::sync::Mutex::new(connections));
        let mut updates = maka_application_bridge::Updates::new(
            f.caller.clone(),
            f.source.clone(),
            connections.clone(),
            f.timer.handle().clone(),
        )
        .unwrap();
        commit(&f, 1, "secondary");
        assert!(next(&mut updates, &app).is_err());
        assert_eq!(app.source(), Some(&f.source));
        assert_eq!(action(&f, &mut app, &mount, "read")["status"], "succeeded");
        let corrected = commit(&f, 2, "local");
        let update = next(&mut updates, &app).unwrap();
        assert_eq!(update.prepared.source(), &corrected);
        app.adopt_prepared(update.prepared, update.registrations)
            .unwrap();
        assert_eq!(app.source(), Some(&corrected));
        commit(&f, 3, "secondary");
        let error = next(&mut updates, &app).err().unwrap();
        assert!(
            error.contains(if lost_capture {
                "not ready for capture"
            } else {
                "startup is unconfirmed"
            }),
            "{error}"
        );
        assert!(app.frame(&mount).is_ok());
        updates.stop();
        f.timer.block_on(updates.drain());
        app.retire(&mount).unwrap();
        app.close(&mount, Ok(())).unwrap();
        app.shutdown().unwrap();
        drop(app);
        f.timer
            .block_on(async { connections.lock().await.close().await })
            .unwrap();
        f.reopened("host");
        f.reopened("secondary");
        drop(connections);
        f.timer.block_on(f.caller.shutdown()).unwrap();
    }
}

#[test]
fn spawn_failure_keeps_the_unstarted_trusted_declaration() {
    let f = Fixture::new();
    let mut configuration = f.configuration(true, true);
    secondary(&f, &mut configuration);
    let loaded = f
        .timer
        .block_on(f.caller.load_application_at(&f.source))
        .unwrap()
        .unwrap();
    let (input, packages) = support::host();
    f.timer
        .block_on(
            f.caller
                .commit_application(f.host, 0, input, vec![], packages),
        )
        .unwrap();
    configuration.hosts[0].location = HostLocation::Cohost {};
    let mut connections = f
        .timer
        .block_on(Connections::prepare(
            f.caller.clone(),
            f.source.deployment,
            f.target,
            configuration,
            f.namespaces.clone(),
            HostExecutable::new(f.directory.path().join("absent-executable"), vec![]).unwrap(),
            Default::default(),
        ))
        .unwrap();
    f.timer.block_on(connections.start()).unwrap();
    let selected = commit(&f, 1, "secondary");
    for _ in 0..2 {
        let error = f
            .timer
            .block_on(connections.approve_source(&f.caller, selected.clone(), BTreeSet::from([1])))
            .err()
            .unwrap();
        assert!(!error.contains("unconfigured Host"), "{error}");
        let spawn_error = std::process::Command::new(f.directory.path().join("absent-executable"))
            .spawn()
            .unwrap_err()
            .to_string();
        assert_eq!(error, spawn_error);
    }
    f.reopened("secondary");
    // The original source remains approvable after the failed spawn.
    f.timer
        .block_on(connections.approve_source(
            &f.caller,
            loaded.source.reference,
            BTreeSet::from([1]),
        ))
        .unwrap();
    f.timer.block_on(connections.close()).unwrap();
    drop(connections);
    f.timer.block_on(f.caller.shutdown()).unwrap();
}

// Framed control fault at the actual process boundary; no endpoint, receipt,
// owner cleanup or foreground adoption is mocked.
const CAPTURE_PROXY: &str = r#"
import json, struct, subprocess, sys

def read(stream):
    prefix = stream.read(4)
    if not prefix:
        return None
    if len(prefix) != 4:
        raise RuntimeError('partial prefix')
    size = struct.unpack('>I', prefix)[0]
    body = stream.read(size)
    if len(body) != size:
        raise RuntimeError('partial body')
    return json.loads(body)

def write(stream, value):
    body = json.dumps(value).encode()
    stream.write(struct.pack('>I', len(body)) + body)
    stream.flush()

bootstrap = read(sys.stdin.buffer)
child = subprocess.Popen([sys.argv[1], 'application', 'serve'],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE)
write(child.stdin, bootstrap)
write(sys.stdout.buffer, read(child.stdout))
while True:
    command = read(sys.stdin.buffer)
    if command is None:
        write(child.stdin, {'kind': 'close', 'nonce': bootstrap['nonce']})
        child.stdin.close()
        while True:
            event = read(child.stdout)
            if event is None:
                break
            write(sys.stdout.buffer, event)
        break
    write(child.stdin, command)
    event = read(child.stdout)
    if command['kind'] == 'capture' and bootstrap['root'].endswith('/secondary'):
        event['nonce'] = '00000000-0000-0000-0000-000000000099'
    write(sys.stdout.buffer, event)
    if event['kind'] == 'closed':
        break
sys.exit(child.wait())
"#;
