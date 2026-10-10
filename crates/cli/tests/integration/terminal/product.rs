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
fn position(term: &Term<IgnoreEvents>, label: &str, last: bool) -> Option<(usize, usize)> {
    let needle: Vec<_> = label.chars().collect();
    (0..40).find_map(|y| {
        let y = if last { 39 - y } else { y };
        let row: Vec<_> = (0..120)
            .map(|x| term.grid()[Line(y as i32)][Column(x)].c)
            .collect();
        row.windows(needle.len())
            .position(|part| part == needle)
            .map(|x| (x, y))
    })
}
fn click(tui: &mut Pty, label: &str, enabled: [Color; 2]) {
    let button = label.starts_with("[ ");
    let label = label
        .strip_prefix("[ ")
        .and_then(|s| s.strip_suffix(" ]"))
        .unwrap_or(label);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        tui.read();
        if tui.frames.ready() {
            let term = cells(tui);
            if let Some((x, y)) = position(&term, label, button)
                && (!button || enabled.contains(&term.grid()[Line(y as i32)][Column(x)].fg))
            {
                tui.send(
                    format!("\x1b[<0;{};{}M\x1b[<0;{};{}m", x + 2, y + 1, x + 2, y + 1).as_bytes(),
                );
                return;
            }
        }
        if Instant::now() >= deadline {
            let term = cells(tui);
            let actual = position(&term, label, button)
                .map(|(x, y)| term.grid()[Line(y as i32)][Column(x)].fg);
            panic!(
                "control {label:?} not enabled: expected {enabled:?}, actual {actual:?}; {}",
                tui.screen.snapshot().unwrap().screen
            );
        }
        assert!(tui.child.try_wait().unwrap().is_none());
    }
}
fn paste(tui: &mut Pty, text: &str) {
    // A complete composition can fill both PTY directions. Keep draining the
    // renderer while the input side applies backpressure.
    let fd = tui.master.as_ref().unwrap().as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    assert!(flags >= 0);
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) },
        0
    );
    let packet = format!("\x1b[200~{text}\x1b[201~");
    let mut pending = packet.as_bytes();
    let deadline = Instant::now() + Duration::from_secs(60);
    while !pending.is_empty() {
        match tui.master.as_mut().unwrap().write(pending) {
            Ok(0) => panic!("terminal closed during paste"),
            Ok(written) => pending = &pending[written..],
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => panic!("terminal paste failed: {error}"),
        }
        tui.read();
        assert!(Instant::now() < deadline, "terminal paste timed out");
    }
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETFL, flags) }, 0);
}

fn editable(tui: &mut Pty, enabled: [Color; 2]) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        tui.read();
        if tui.frames.ready() {
            let term = cells(tui);
            if let Some((x, y)) = position(&term, "Save composition", true) {
                // Actual light-theme enabled paint; loading disables both this
                // control and the native editor. Do not paste into a stale draft.
                if enabled.contains(&term.grid()[Line(y as i32)][Column(x)].fg) {
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
) -> (Value, Value, [Color; 2]) {
    let output = Command::new("node").args(["--input-type=module", "-e", r#"
        const {composition,studioComposition,instanceNames}=await import(process.argv[1]);
        const configuration=JSON.parse(process.argv[2]);
        const packages=JSON.parse(process.argv[3]);
        const keys=Object.fromEntries(instanceNames.map((name,i)=>[name,
            '00000000-0000-0000-0000-'+String(i+20).padStart(12,'0')]));
        const common={...configuration,packages,keys};
        const {activate}=await import(process.argv[4]);
        const theme=activate({scheme:configuration.scheme}).Appearance.value;
        const enabled=[theme.accent >>> 8,theme.text >>> 8];
        console.log(JSON.stringify([composition({...common,agent:{workspaceRoots:configuration.workspaceRoots,maxActiveRuns:1, permissions: { access: 'workspace_write', requestPermissions: true },maxSteps:1,tools:[]}}),
            studioComposition({...common,hostRoot:configuration.studioRoot,hostDeployment:configuration.studioDeployment}),enabled]));
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
    let enabled = [0, 1].map(|i| {
        let color = result[2][i].as_u64().unwrap() as u32;
        Color::Spec(Rgb {
            r: (color >> 16) as u8,
            g: (color >> 8) as u8,
            b: color as u8,
        })
    });
    (result[0].clone(), result[1].clone(), enabled)
}

fn built_packages() -> (std::path::PathBuf, BTreeMap<String, Package>) {
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
                Package::read_from(&dist.join("packages").join(name)).unwrap_or_else(|error| {
                    panic!(
                        "package {}: {error}",
                        dist.join("packages").join(name).display()
                    )
                }),
            )
        })
        .collect();
    (dist, packages)
}

#[test]
#[ignore = "opt-in ordinary product build, native file input and actual foreground PTY"]
fn agent_installs_studio_and_production_launcher_replaces_the_surface_and_backend() {
    let (dist, packages) = built_packages();
    let refs = packages
        .iter()
        .map(|(name, package)| (name.clone(), package.application_ref()))
        .collect();
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("ui");
    let agent_root = directory.path().join("agent");
    let studio_root = directory.path().join("studio");
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
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
            "workspaceRoots":[{"label":"Workspace", "path":workspace}],
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
        .map(|node| {
            serde_json::from_value::<maka_plugin_core::package::PackageRef>(
                node["package"]["package"].clone(),
            )
            .unwrap()
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
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
        if tui.frames.ready() && tui.screen.snapshot().unwrap().screen.contains("Plugins") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "launch timed out: {:?}",
            tui.screen.snapshot().unwrap().screen
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
    tui.wait_until_with_timeout(
        |screen| screen.contains("Plugin package saved."),
        Duration::from_secs(60),
    );
    click(&mut tui, "[ Edit product composition ]", accent);
    click(&mut tui, "[ Reload saved source ]", accent);
    click(&mut tui, "[ Confirm ]", accent);
    tui.wait_until(|screen| !screen.contains("Discard this draft and reload?"));
    editable(&mut tui, accent);
    click(&mut tui, "{", accent);
    tui.send(b"\x01");
    paste(&mut tui, &serde_json::to_string(&document).unwrap());
    click(&mut tui, "[ Save composition ]", accent);
    click(&mut tui, "[ Confirm ]", accent);
    tui.wait_until_with_timeout(
        |screen| screen.contains("Configuration Studio"),
        Duration::from_secs(60),
    );
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

#[test]
#[ignore = "opt-in ordinary skill library, actual Host and foreground PTY"]
fn skill_library_reads_selected_sources_and_rejects_changed_pages_in_the_real_product() {
    let (dist, packages) = built_packages();
    let refs = packages
        .iter()
        .map(|(name, package)| (name.clone(), package.application_ref()))
        .collect();
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("ui");
    let host_root = directory.path().join("host");
    let workspace = directory.path().join("workspace");
    let source = directory.path().join("skills");
    std::fs::create_dir(&workspace).unwrap();
    for index in 0..40 {
        let skill = source.join(format!("skill-{index:02}"));
        std::fs::create_dir_all(&skill).unwrap();
        let body = if index == 0 {
            format!(
                "FIRST PAGE\n{}\nLAST PAGE",
                "A skill instruction line.\n".repeat(164)
            )
        } else {
            format!("Instructions for skill-{index:02}")
        };
        std::fs::write(
            skill.join("SKILL.md"),
            format!(
                "---\nname: skill-{index:02}\ndescription: Library entry {index:02}\n---\n{body}"
            ),
        )
        .unwrap();
    }
    std::fs::create_dir_all(source.join("skill-39/scripts")).unwrap();
    std::fs::write(source.join("skill-39/scripts/check"), "original resource").unwrap();
    let deployment = Uuid::new_v4();
    let host_id = Uuid::new_v4();
    let manager_id = Uuid::new_v4();
    let (agent, _, accent) = authored(
        &dist.join("authoring/agent-product/src/composition.mjs"),
        json!({
            "presentation":"tui", "locale":"en", "scheme":"light",
            "hostRoot":host_root, "hostDeployment":host_id, "managementDeployment":manager_id,
            "studioRoot":directory.path().join("unused"), "studioDeployment":Uuid::new_v4(),
            "workspaceRoots":[{"label":"Workspace", "path":workspace}],
            "skills":{"manage":true,"sources":[{"id":"local", "label":"Test collection", "path":source}]},
        }),
        &refs,
    );
    let connections = directory.path().join("connections.json");
    std::fs::write(
        &connections,
        serde_json::to_vec(&agent["connections"]).unwrap(),
    )
    .unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let ns = RootNamespaces::for_current_account().unwrap();
    runtime.block_on(async {
        for (path, instances) in [
            (&host_root, vec![(host_id, "host", false)]),
            (
                &root,
                vec![
                    (manager_id, "managementHost", false),
                    (deployment, "ui", true),
                ],
            ),
        ] {
            let owner = Arc::new(RootOwner::create(path, &ns).unwrap());
            let store = EventLog::for_root(owner).await.unwrap();
            for (id, name, remote) in instances {
                store
                    .commit_application(
                        id,
                        0,
                        serde_json::from_value(agent[name].clone()).unwrap(),
                        if remote {
                            serde_json::from_value(agent["remoteExports"].clone()).unwrap()
                        } else {
                            vec![]
                        },
                        packages.values().cloned().collect(),
                    )
                    .await
                    .unwrap();
            }
            store.shutdown().await.unwrap();
        }
    });
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
    tui.wait_until_with_timeout(|screen| screen.contains("Plugins"), Duration::from_secs(30));
    tui.send(b"\x0b");
    tui.wait_for("Commands");
    paste(&mut tui, "Skills");
    click(&mut tui, "[ Skills ]", accent);
    tui.wait_for("Library entry 00");
    // First page and next page are real bounded Host queries.
    click(&mut tui, "[ Next page ]", accent);
    tui.wait_for("Library entry 32");
    click(&mut tui, "[ Previous page ]", accent);
    tui.wait_for("Library entry 00");
    click(&mut tui, "skill-00", accent);
    tui.wait_for("FIRST PAGE");
    click(&mut tui, "[ Next excerpt ]", accent);
    tui.wait_for("LAST PAGE");
    click(&mut tui, "[ Previous excerpt ]", accent);
    tui.wait_for("FIRST PAGE");
    // A changed file cannot be joined to the retained body page.
    std::fs::write(
        source.join("skill-00/SKILL.md"),
        "---\nname: skill-00\ndescription: Updated description\n---\nUPDATED BODY",
    )
    .unwrap();
    click(&mut tui, "[ Next excerpt ]", accent);
    tui.wait_for("Skills changed.");
    assert!(tui.screen.snapshot().unwrap().screen.contains("FIRST PAGE"));
    click(&mut tui, "[ Refresh ]", accent);
    tui.wait_for("Updated description");
    // Search uses a captured native editor revision, then the complete Host catalog.
    click(&mut tui, "Search skills", accent);
    paste(&mut tui, "skill-39");
    tui.send(b"\r");
    tui.wait_until(|screen| {
        screen.contains("Library entry 39") && !screen.contains("Updated description")
    });
    // Native list entry accepts keyboard navigation without opening on focus.
    tui.send(b"\t\t");
    tui.wait_for("Library entry 39");
    assert!(
        !tui.screen
            .snapshot()
            .unwrap()
            .screen
            .contains("Instructions for skill-39")
    );
    tui.send(b"\r");
    tui.wait_for("Instructions for skill-39");
    click(&mut tui, "[ Install ]", accent);
    tui.wait_for("Review skill change");
    click(&mut tui, "[ Cancel ]", accent);
    tui.wait_for("Instructions for skill-39");
    click(&mut tui, "[ Install ]", accent);
    tui.wait_for("Review skill change");
    click(&mut tui, "[ Confirm change ]", accent);
    tui.wait_for("Installed · Library entry 39");
    let store = std::fs::read_dir(host_root.join("skill-libraries"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let installed = store.join("skills/skill-39");
    assert_eq!(
        std::fs::read(installed.join("scripts/check")).unwrap(),
        b"original resource"
    );
    // Update review includes the complete installed tree. An intervening local
    // resource edit refuses publication, and the next explicit review preserves it.
    std::fs::write(
        installed.join("SKILL.md"),
        "---\nname: skill-39\ndescription: Local skill 39\n---\nLOCAL CHANGES",
    )
    .unwrap();
    std::fs::write(
        source.join("skill-39/SKILL.md"),
        "---\nname: skill-39\ndescription: Updated skill 39 source\n---\nUPDATED SKILL 39",
    )
    .unwrap();
    click(&mut tui, "[ Refresh ]", accent);
    tui.wait_until(|screen| {
        screen.contains("Test collection · Updated skill 39 source")
            && !screen.contains("Loading skills")
    });
    click(
        &mut tui,
        "Test collection · Updated skill 39 source",
        accent,
    );
    tui.wait_for("UPDATED SKILL 39");
    click(&mut tui, "[ Update installed copy ]", accent);
    tui.wait_for("The installed instructions have local changes.");
    std::fs::write(installed.join("scripts/check"), "edited after preview").unwrap();
    click(&mut tui, "[ Confirm change ]", accent);
    tui.wait_for("Skills changed.");
    assert!(
        std::fs::read_to_string(installed.join("SKILL.md"))
            .unwrap()
            .contains("LOCAL CHANGES")
    );
    click(&mut tui, "[ Refresh ]", accent);
    tui.wait_until(|screen| {
        screen.contains("Test collection · Updated skill 39 source")
            && !screen.contains("Loading skills")
    });
    click(
        &mut tui,
        "Test collection · Updated skill 39 source",
        accent,
    );
    tui.wait_for("UPDATED SKILL 39");
    click(&mut tui, "[ Update installed copy ]", accent);
    tui.wait_for("Review skill change");
    click(&mut tui, "[ Confirm change ]", accent);
    tui.wait_for("Installed · Updated skill 39 source");
    assert!(
        std::fs::read_to_string(installed.join("SKILL.md"))
            .unwrap()
            .contains("UPDATED SKILL 39")
    );
    assert_eq!(
        std::fs::read(installed.join("scripts/check")).unwrap(),
        b"edited after preview"
    );
    click(&mut tui, "Installed · Updated skill 39 source", accent);
    tui.wait_for("UPDATED SKILL 39");
    click(&mut tui, "[ Delete installed copy ]", accent);
    tui.wait_for("Delete this installed skill and all its resource files.");
    click(&mut tui, "[ Confirm change ]", accent);
    tui.wait_until(|screen| {
        screen.contains("Test collection · Updated skill 39 source")
            && !screen.contains("Installed ·")
    });
    assert!(!installed.exists());
    assert_eq!(
        std::fs::read(source.join("skill-39/scripts/check")).unwrap(),
        b"original resource"
    );
    tui.send(b"\x1b");
    tui.wait_until(|screen| !screen.contains("╭Skills"));
    tui.send(b"\x0b");
    tui.wait_for("› Skills");
    tui.send(b"\r");
    tui.wait_for("Updated skill 39 source");
    assert!(tui.screen.snapshot().unwrap().screen.contains("skill-39"));
    tui.send(b"\x1b");
    tui.wait_until(|screen| !screen.contains("╭Skills"));
    tui.send(b"\x11");
    tui.finish();
    assert!(!tui.screen.snapshot().unwrap().alternate_screen);
    for path in [&root, &host_root] {
        drop(RootOwner::open(path, &ns).unwrap());
    }
    // Keep both deployments and all endpoint selections unchanged. Removing the
    // selected source makes the actual Library initializer fail on restart;
    // ordinary UI startup must fall back to its independent recovery root.
    runtime.block_on(async {
        let ui = EventLog::for_root(Arc::new(RootOwner::open(&root, &ns).unwrap()))
            .await
            .unwrap();
        let intents = ui
            .application_deployment_intents(deployment, 0, 4)
            .await
            .unwrap();
        assert!(!intents.is_empty());
        assert_eq!(intents[0].reference.contract.name, "maka.skills.Library");
        let loaded = ui.load_application(deployment).await.unwrap().unwrap();
        assert_ne!(loaded.source.input.launch, loaded.source.input.recovery);
        assert!(loaded.source.input.instances.contains_key("skill-library"));
        ui.shutdown().await.unwrap();
    });
    std::fs::remove_dir_all(&source).unwrap();
    let mut recovered = Pty::spawn(&[
        "application",
        "run",
        "--root",
        root.to_str().unwrap(),
        "--deployment",
        &deployment.to_string(),
        "--connections",
        connections.to_str().unwrap(),
    ]);
    recovered.wait_until_with_timeout(
        |screen| screen.contains("maka.skills.Library / apply"),
        Duration::from_secs(30),
    );
    click(&mut recovered, "[ Check ]", accent);
    recovered.wait_for("Original skill result confirmed.");
    recovered.wait_for("installed:skill-39");
    assert!(!installed.exists());
    assert!(!source.exists());
    recovered.send(b"\x11");
    recovered.finish();
    assert!(!recovered.screen.snapshot().unwrap().alternate_screen);
    for path in [&root, &host_root] {
        drop(RootOwner::open(path, &ns).unwrap());
    }
}
