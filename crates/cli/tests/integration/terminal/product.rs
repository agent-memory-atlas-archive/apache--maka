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
use alacritty_terminal::{
    Term,
    event::{Event, EventListener},
    grid::Dimensions,
    index::{Column, Line},
    term::Config,
    vte::ansi::{Color, Processor, Rgb},
};
use maka_event_log::{
    EventLog,
    root::{RootNamespaces, RootOwner},
};
use maka_plugin_core::deployment::management::DeploymentDocument;
use maka_plugins::package::Package;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path, sync::Arc};
use uuid::Uuid;

struct Size;
impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        40
    }
    fn screen_lines(&self) -> usize {
        40
    }
    fn columns(&self) -> usize {
        120
    }
}
#[derive(Clone)]
struct IgnoreEvents;
impl EventListener for IgnoreEvents {
    fn send_event(&self, _: Event) {}
}
fn cells(tui: &Pty) -> Term<IgnoreEvents> {
    // Test-only style inspection; production's public screen DTO stays textual.
    assert!(
        tui.output.len() < 1024 * 1024,
        "terminal evidence was truncated"
    );
    let mut term = Term::new(
        Config {
            scrolling_history: 0,
            ..Default::default()
        },
        &Size,
        IgnoreEvents,
    );
    let mut parser: Processor = Processor::new();
    parser.advance(&mut term, &tui.output);
    term
}
fn position(term: &Term<IgnoreEvents>, label: &str) -> Option<(usize, usize)> {
    let needle: Vec<_> = label.chars().collect();
    (0..40).find_map(|y| {
        let row: Vec<_> = (0..120)
            .map(|x| term.grid()[Line(y as i32)][Column(x)].c)
            .collect();
        row.windows(needle.len())
            .position(|part| part == needle)
            .map(|x| (x, y))
    })
}
fn click(tui: &mut Pty, label: &str, accent: Color) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        tui.read();
        if tui.frames.ready() {
            let term = cells(tui);
            if let Some((x, y)) = position(&term, label)
                && (!label.starts_with('[') || term.grid()[Line(y as i32)][Column(x)].fg == accent)
            {
                tui.send(
                    format!("\x1b[<0;{};{}M\x1b[<0;{};{}m", x + 2, y + 1, x + 2, y + 1).as_bytes(),
                );
                return;
            }
        }
        if Instant::now() >= deadline {
            let term = cells(tui);
            let actual =
                position(&term, label).map(|(x, y)| term.grid()[Line(y as i32)][Column(x)].fg);
            panic!(
                "control {label:?} not enabled: expected {accent:?}, actual {actual:?}; {}",
                tui.screen.snapshot().unwrap().screen
            );
        }
        assert!(tui.child.try_wait().unwrap().is_none());
    }
}
fn paste(tui: &mut Pty, text: &str) {
    tui.send(format!("\x1b[200~{text}\x1b[201~").as_bytes());
}
fn editable(tui: &mut Pty, accent: Color) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        tui.read();
        if tui.frames.ready() {
            let term = cells(tui);
            if let Some((x, y)) = position(&term, "[ Save composition ]") {
                // Actual light-theme enabled paint; loading disables both this
                // control and the native editor. Do not paste into a stale draft.
                if term.grid()[Line(y as i32)][Column(x)].fg == accent {
                    return;
                }
            }
        }
        assert!(
            Instant::now() < deadline,
            "source editor did not become writable"
        );
        assert!(tui.child.try_wait().unwrap().is_none());
    }
}
fn authored(
    module: &Path,
    configuration: Value,
    refs: &BTreeMap<String, maka_plugin_core::package::PackageRef>,
) -> (Value, Value, Color) {
    let output = Command::new("node").args(["--input-type=module", "-e", r#"
        const {composition,studioComposition,packageNames}=await import(process.argv[1]);
        const configuration=JSON.parse(process.argv[2]);
        const packages=JSON.parse(process.argv[3]);
        const keys=Object.fromEntries(packageNames.map((name,i)=>[name,
            '00000000-0000-0000-0000-'+String(i+20).padStart(12,'0')]));
        const common={...configuration,packages,keys};
        const {activate}=await import(process.argv[4]);
        const accent=activate({scheme:configuration.scheme}).Appearance.value.accent >>> 8;
        console.log(JSON.stringify([composition({...common,agent:{maxActiveRuns:1,maxSteps:1,tools:[]}}),
            studioComposition({...common,hostRoot:configuration.studioRoot,hostDeployment:configuration.studioDeployment}),accent]));
    "#]).arg(format!("file://{}", module.display())).arg(configuration.to_string())
        .arg(serde_json::to_string(refs).unwrap())
        .arg(format!("file://{}", module.ancestors().nth(4).unwrap().join("packages/appearance/tui.js").display()))
        .output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: Vec<Value> = serde_json::from_slice(&output.stdout).unwrap();
    let accent = result[2].as_u64().unwrap() as u32;
    (
        result[0].clone(),
        result[1].clone(),
        Color::Spec(Rgb {
            r: (accent >> 16) as u8,
            g: (accent >> 8) as u8,
            b: accent as u8,
        }),
    )
}

#[test]
#[ignore = "opt-in ordinary product build, native file input and actual foreground PTY"]
fn agent_installs_studio_and_production_launcher_replaces_the_surface_and_backend() {
    let repository = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("node")
        .arg(repository.join("packages/agent-product/scripts/build.mjs"))
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let dist = repository.join("packages/agent-product/dist");
    let names: Vec<String> =
        serde_json::from_slice(&std::fs::read(dist.join("packages.json")).unwrap()).unwrap();
    let packages: BTreeMap<_, _> = names
        .iter()
        .map(|name| {
            (
                name.clone(),
                Package::read_from(&dist.join("packages").join(name)).unwrap(),
            )
        })
        .collect();
    let refs = packages
        .iter()
        .map(|(name, package)| (name.clone(), package.application_ref()))
        .collect();
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("ui");
    let agent_root = directory.path().join("agent");
    let studio_root = directory.path().join("studio");
    let deployment = Uuid::new_v4();
    let agent_id = Uuid::new_v4();
    let studio_id = Uuid::new_v4();
    let manager_id = Uuid::new_v4();
    let (agent, mut studio, accent) = authored(
        &dist.join("authoring/agent-product/src/composition.mjs"),
        json!({
            "presentation":"tui", "locale":"en", "scheme":"light",
            "hostRoot":agent_root, "hostDeployment":agent_id, "managementDeployment":manager_id,
            "studioRoot":studio_root, "studioDeployment":studio_id,
        }),
        &refs,
    );
    for node in studio["ui"]["instances"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        for targets in node["bindings"].as_object_mut().unwrap().values_mut() {
            for target in targets.as_array_mut().unwrap() {
                if target["kind"] == "remote" && target["deployment"] == "local" {
                    target["deployment"] = json!("studio");
                }
            }
        }
    }
    for observation in studio["remoteExports"].as_array_mut().unwrap() {
        if observation["capability"]["deployment"] == "local" {
            observation["capability"]["deployment"] = json!("studio");
        }
    }
    let mut configuration = agent["connections"].clone();
    let mut secondary = studio["connections"]["hosts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|host| host["name"] == "local")
        .unwrap()
        .clone();
    secondary["name"] = json!("studio");
    configuration["hosts"]
        .as_array_mut()
        .unwrap()
        .push(secondary);
    let connections = directory.path().join("connections.json");
    std::fs::write(&connections, serde_json::to_vec(&configuration).unwrap()).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let ns = RootNamespaces::for_current_account().unwrap();
    runtime.block_on(async {
        for (path, id, source) in [
            (&agent_root, agent_id, &agent["host"]),
            (&studio_root, studio_id, &studio["host"]),
        ] {
            let owner = Arc::new(RootOwner::create(path, &ns).unwrap());
            let store = EventLog::for_root(owner).await.unwrap();
            store
                .commit_application(
                    id,
                    0,
                    serde_json::from_value(source.clone()).unwrap(),
                    vec![],
                    packages.values().cloned().collect(),
                )
                .await
                .unwrap();
            store.shutdown().await.unwrap();
        }
        let owner = Arc::new(RootOwner::create(&root, &ns).unwrap());
        let store = EventLog::for_root(owner).await.unwrap();
        store
            .commit_application(
                manager_id,
                0,
                serde_json::from_value(agent["managementHost"].clone()).unwrap(),
                vec![],
                packages.values().cloned().collect(),
            )
            .await
            .unwrap();
        // Studio's UI package is deliberately absent until native installation.
        store
            .commit_application(
                deployment,
                0,
                serde_json::from_value(agent["ui"].clone()).unwrap(),
                serde_json::from_value(agent["remoteExports"].clone()).unwrap(),
                packages
                    .iter()
                    .filter(|(name, _)| name.as_str() != "studio")
                    .map(|(_, package)| package.clone())
                    .collect(),
            )
            .await
            .unwrap();
        store.shutdown().await.unwrap();
    });
    let selected: Vec<_> = studio["ui"]["instances"]
        .as_object()
        .unwrap()
        .values()
        .map(|node| node["package"]["package"].clone())
        .collect();
    let document: DeploymentDocument = serde_json::from_value(
        json!({"input":studio["ui"], "remoteExports":studio["remoteExports"], "packages":selected}),
    )
    .unwrap();
    let bundle = directory.path().join("configuration-workbench.maka-plugin");
    packages["studio"].export_to(&bundle).unwrap();
    let mut tui = Pty::spawn(&[
        "application",
        "run",
        "--root",
        root.to_str().unwrap(),
        "--deployment",
        &deployment.to_string(),
        "--connections",
        connections.to_str().unwrap(),
    ]);
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        tui.read();
        if tui.frames.ready()
            && tui
                .screen
                .snapshot()
                .unwrap()
                .screen
                .contains("[ Plugins ]")
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "launch timed out: {:?}",
            String::from_utf8_lossy(&tui.output)
        );
        assert!(
            tui.child.try_wait().unwrap().is_none(),
            "launch exited: {:?}",
            String::from_utf8_lossy(&tui.output)
        );
    }
    drop(RootOwner::open(&studio_root, &ns).expect("idle declaration cannot acquire its Root"));
    click(&mut tui, "[ Plugins ]", accent);
    click(&mut tui, "[ Install plugin ]", accent);
    click(&mut tui, "Absolute path to a .maka-plugin file", accent);
    paste(&mut tui, bundle.to_str().unwrap());
    tui.send(b"\r");
    tui.wait_for("configuration-workbench.maka-plugin");
    click(&mut tui, "[ Install plugin ]", accent);
    click(&mut tui, "[ Confirm ]", accent);
    tui.wait_for("Plugin package saved.");
    click(&mut tui, "[ Edit product composition ]", accent);
    click(&mut tui, "[ Reload saved source ]", accent);
    click(&mut tui, "[ Confirm ]", accent);
    editable(&mut tui, accent);
    click(&mut tui, "{", accent);
    tui.send(b"\x01");
    paste(&mut tui, &serde_json::to_string(&document).unwrap());
    click(&mut tui, "[ Save composition ]", accent);
    click(&mut tui, "[ Confirm ]", accent);
    tui.wait_for("Configuration Studio");
    assert_eq!(
        RootOwner::open(&studio_root, &ns).err().unwrap().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(!tui.screen.snapshot().unwrap().screen.contains("[ Send ]"));
    click(&mut tui, "[ Connections ]", accent);
    tui.wait_for("No connections yet.");
    click(&mut tui, "[ Models ]", accent);
    click(&mut tui, "[ Choose model ]", accent);
    tui.wait_for("Add a connection to choose a model.");
    tui.send(b"\x03");
    tui.finish();
    let mut termios = unsafe { std::mem::zeroed::<libc::termios>() };
    assert_eq!(
        unsafe { libc::tcgetattr(tui.master.as_ref().unwrap().as_raw_fd(), &mut termios) },
        0
    );
    assert_ne!(termios.c_lflag & libc::ICANON, 0);
    assert_ne!(termios.c_lflag & libc::ECHO, 0);
    let final_screen = tui.screen.snapshot().unwrap();
    assert!(!final_screen.alternate_screen);
    assert!(final_screen.cursor.visible);
    for path in [&root, &agent_root, &studio_root] {
        drop(RootOwner::open(path, &ns).unwrap());
    }
    runtime.block_on(async {
        let owner = Arc::new(RootOwner::open(&root, &ns).unwrap());
        let store = EventLog::for_root(owner).await.unwrap();
        let loaded = store.load_application(deployment).await.unwrap().unwrap();
        assert_eq!(loaded.source.reference.revision, 3);
        assert_eq!(loaded.source.input, document.input);
        assert!(
            loaded
                .packages
                .iter()
                .any(|package| package.to_bundle() == packages["studio"].to_bundle())
        );
        store.shutdown().await.unwrap();
    });
}
