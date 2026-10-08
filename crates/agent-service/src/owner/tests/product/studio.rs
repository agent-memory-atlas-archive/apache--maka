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
use maka_application_bridge::transport::{
    Endpoint, Peer, Peers, Server,
    native::{Connection, Service},
};
use maka_plugin_core::deployment::management::DeploymentDocument;
use std::path::Path;

fn consumers(
    plan: &maka_plugin_core::plan::Plan,
    host: &str,
) -> std::collections::BTreeSet<InstanceId> {
    plan.instances()
        .values()
        .filter(|instance| !instance.disabled && instance.entry.is_some())
        .filter(|instance| {
            instance.bindings.values().flatten().any(|binding| {
                matches!(&binding.provider, maka_plugin_core::plan::Provider::Remote {capability}
                if capability.deployment == host)
            })
        })
        .map(|instance| instance.identity.clone())
        .collect()
}
// Actual packaged Agent -> Workbench switch through original FilePicker,
// Management install/replace and a distinct persisted settings Host. The native
// Buffer owner supplies surface cleanup; this is not OS/Guard/visual acceptance.
pub(super) fn switch(
    runtime: &tokio::runtime::Runtime,
    caller: &Arc<EventLog>,
    presenter: &mut maka_tui::application::Presenter,
    directory: &Path,
    manager_registrations: &[RemoteService],
    manager_peers: &Arc<Peers>,
) {
    let original_manager_source = presenter.deployment_status().effective.clone().unwrap();
    let old_source = runtime
        .block_on(
            caller.application_head(
                presenter
                    .deployment_status()
                    .effective
                    .as_ref()
                    .unwrap()
                    .deployment,
            ),
        )
        .unwrap()
        .unwrap();
    let old = runtime
        .block_on(caller.load_application_at(&old_source))
        .unwrap()
        .unwrap();
    let studio_package = Package::read_from(&directory.join("studio")).unwrap();
    assert!(
        !old.packages
            .iter()
            .any(|p| p.application_ref() == studio_package.application_ref())
    );
    let temporary = tempfile::tempdir().unwrap();
    let bundle = temporary.path().join("configuration-workbench.maka-plugin");
    studio_package.export_to(&bundle).unwrap();
    click(presenter, "[ Plugins ]");
    click(presenter, "[ Install plugin ]");
    await_screen(presenter, "Absolute path to a .maka-plugin file");
    click(presenter, "Absolute path to a .maka-plugin file");
    presenter
        .input(Event::Paste(bundle.to_str().unwrap().into()))
        .unwrap();
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(presenter, "configuration-workbench.maka-plugin");
    click(presenter, "[ Install plugin ]");
    click(presenter, "[ Confirm ]");
    await_screen(presenter, "Plugin package saved.");
    let installed_source = runtime
        .block_on(caller.application_head(old_source.deployment))
        .unwrap()
        .unwrap();
    let installed = runtime
        .block_on(caller.load_application_at(&installed_source))
        .unwrap()
        .unwrap();
    assert_eq!(installed.source.input, old.source.input);
    assert!(
        installed
            .packages
            .iter()
            .any(|p| p.to_bundle() == studio_package.to_bundle())
    );

    let names: Vec<String> = serde_json::from_slice(
        &std::fs::read(directory.parent().unwrap().join("packages.json")).unwrap(),
    )
    .unwrap();
    let refs: BTreeMap<_, _> = names
        .iter()
        .map(|name| {
            (
                name,
                Package::read_from(&directory.join(name))
                    .unwrap()
                    .application_ref(),
            )
        })
        .collect();
    let mut keys: BTreeMap<String, String> = names
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), Uuid::from_u128(i as u128 + 20).to_string()))
        .collect();
    for (name, node) in &old.source.input.instances {
        keys.insert(name.clone(), node.key.to_string());
    }
    for observation in &old.source.remote_exports {
        let name = match observation.capability.export.as_str() {
            "Settings" => "settings",
            "Receipts" => "receipts",
            "Management" => "deployment-manager",
            _ => continue,
        };
        keys.insert(
            name.into(),
            observation.capability.instance.0[0].to_string(),
        );
    }
    keys.insert("studio".into(), Uuid::from_u128(1001).to_string());
    keys.insert("studio-gateway".into(), Uuid::from_u128(1002).to_string());
    let configuration = json!({"presentation":"tui", "locale":"en", "scheme":"light",
        "keys":keys, "packages":refs, "hostRoot":temporary.path().join("host"),
        "hostDeployment":Uuid::from_u128(2001), "managementDeployment":Uuid::from_u128(2002)});
    let module = directory
        .parent()
        .unwrap()
        .join("authoring/agent-product/src/composition.mjs");
    let result = Command::new("node").args(["--input-type=module", "-e", r#"
        const {studioComposition}=await import(process.argv[1]);
        const result=studioComposition(JSON.parse(process.argv[2]));
        const packages=[...new Set(Object.values(result.ui.instances).map(n=>JSON.stringify(n.package.package)))].map(JSON.parse);
        console.log(JSON.stringify({...result, packages}));
    "#]).arg(format!("file://{}",module.display())).arg(configuration.to_string()).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let authored: Value = serde_json::from_slice(&result.stdout).unwrap();
    let document: DeploymentDocument = serde_json::from_value(json!({
        "input":authored["ui"], "remoteExports":authored["remoteExports"], "packages":authored["packages"]
    })).unwrap();
    assert!(!document.input.instances.contains_key("workspace"));
    let host_input: Input = serde_json::from_value(authored["host"].clone()).unwrap();
    assert!(!host_input.instances.contains_key("agent"));
    let host_configuration: maka_application_bridge::Host = serde_json::from_value(
        authored["connections"]["hosts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|host| host["name"] == "local")
            .unwrap()
            .clone(),
    )
    .unwrap();
    assert!(!host_configuration.imports.contains_key("agent"));
    let ns = RootNamespaces {
        ownership: temporary.path().join("owners"),
        control: temporary.path().join("control"),
    };
    let owner = Arc::new(RootOwner::create(&temporary.path().join("host"), &ns).unwrap());
    let host_store = Arc::new(runtime.block_on(EventLog::for_root(owner)).unwrap());
    let host_packages = ["settings", "receipts", "studio-gateway"]
        .iter()
        .map(|name| Package::read_from(&directory.join(name)).unwrap())
        .collect();
    runtime
        .block_on(host_store.commit_application(
            host_configuration.deployment,
            0,
            host_input,
            vec![],
            host_packages,
        ))
        .unwrap();
    let mut native = maka_application_bridge::with_gateway(Default::default()).unwrap();
    maka_settings_service::provide(&mut native).unwrap();
    let policy = Arc::new(maka_application_bridge::transport::PeerPolicy::default());
    let app = runtime
        .block_on(Application::start_from_store(
            host_store.clone(),
            host_configuration.deployment,
            native,
            policy.issuer(),
            policy.clone(),
            Default::default(),
        ))
        .unwrap();
    runtime.block_on(app.ready()).unwrap();
    let client = app.client();
    let mut endpoints = BTreeMap::new();
    for (name, grant) in &host_configuration.imports {
        let binding = runtime
            .block_on(client.bind(
                client.activation(&host_configuration.consumer).unwrap(),
                name.clone(),
            ))
            .unwrap()
            .remove(0);
        endpoints.insert(name.clone(), Endpoint::new(binding, grant.clone()).unwrap());
    }
    let server = Server::new(client, policy, endpoints).unwrap();

    click(presenter, "[ Edit product composition ]");
    // The retained editor may carry an old draft. Explicit reload observes the
    // just-installed immutable package before replacing the complete source.
    click(presenter, "[ Reload saved source ]");
    click(presenter, "[ Confirm ]");
    wait_for_target(presenter, "[ Save composition ]", 0);
    click(presenter, "{");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter
        .input(Event::Paste(serde_json::to_string(&document).unwrap()))
        .unwrap();
    click(presenter, "[ Save composition ]");
    click(presenter, "[ Confirm ]");
    await_screen(presenter, "Changes saved.");
    let next = runtime
        .block_on(caller.application_head(old_source.deployment))
        .unwrap()
        .unwrap();
    let loaded = runtime
        .block_on(caller.load_application_at(&next))
        .unwrap()
        .unwrap();
    assert_eq!(loaded.source.input, document.input);
    let mut declarations = maka_plugin_core::package::Packages::default();
    for package in &loaded.packages {
        declarations
            .insert(
                package.application_ref(),
                package.manifest().application.clone().unwrap(),
            )
            .unwrap();
    }
    let exports = loaded
        .source
        .remote_exports
        .iter()
        .map(|r| (r.capability.clone(), r.declaration.clone()))
        .collect();
    let plan =
        maka_plugin_core::plan::Plan::prepare(&document.input, &declarations, &exports).unwrap();
    let peers = Peers::new(Peer::new(next.clone(), consumers(&plan, "local")).unwrap());
    let connection = Connection::new(Arc::new(Physical {
        server: server.clone(),
        peer: peers.clone(),
    }));
    // The shared Deployment provider is unchanged and still executes from its
    // original committed source. Root retirement is not provider retirement.
    assert_eq!(
        document.input.instances["deployment"],
        old.source.input.instances["deployment"]
    );
    manager_peers
        .replace(vec![
            Peer::new(original_manager_source, consumers(&plan, "manager")).unwrap(),
            Peer::new(next.clone(), consumers(&plan, "manager")).unwrap(),
        ])
        .unwrap();
    let registrations = loaded
        .source
        .remote_exports
        .iter()
        .map(|observation| {
            if observation.capability.deployment == "manager" {
                let original = manager_registrations
                    .iter()
                    .find(|r| r.target == observation.capability)
                    .unwrap();
                return RemoteService {
                    target: original.target.clone(),
                    registration_epoch: original.registration_epoch,
                    contract: original.contract.clone(),
                    implementation: original.implementation.clone(),
                };
            }
            let alias = if observation.capability.export == "Settings" {
                "settings"
            } else {
                "receipts"
            };
            let selected = server.catalogue().select(alias).unwrap();
            let description = selected.description();
            RemoteService {
                target: observation.capability.clone(),
                registration_epoch: description.registration_epoch,
                contract: description.contract.clone(),
                implementation: Arc::new(
                    Service::new(connection.clone(), selected, peers.clone()).unwrap(),
                ),
            }
        })
        .collect();
    let update = runtime
        .block_on(presenter.load_committed_update(next.clone()).unwrap())
        .unwrap();
    let old_mount = presenter.surface();
    presenter.adopt_update(update, registrations).unwrap();
    assert!(presenter.requires_surface_replacement());
    assert!(presenter.try_replace_surface(json!({})).is_err());
    presenter.surface_closed(&old_mount, Ok(())).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !presenter.try_replace_surface(json!({})).unwrap() {
        presenter.pump().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert_ne!(presenter.surface(), old_mount);
    assert_eq!(presenter.deployment_status().effective, Some(next.clone()));
    assert!(presenter.surface_closed(&old_mount, Ok(())).is_err());
    await_screen(presenter, "Configuration Studio");
    let visible = screen(&draw(presenter));
    assert!(!visible.contains("Elegant response") && !visible.contains("[ Send ]"));
    click(presenter, "[ Connections ]");
    await_screen(presenter, "No connections yet.");
    click(presenter, "[ Models ]");
    click(presenter, "[ Choose model ]");
    await_screen(presenter, "Add a connection to choose a model.");
    // Models now owns a real native modal, including in this independent root.
    // Close it before navigating the covered Studio pages.
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    await_screen(presenter, "[ Plugins ]");
    click(presenter, "[ Plugins ]");
    click(presenter, "[ Edit product composition ]");
    wait_for_target(presenter, "[ Save composition ]", 0);
    click(presenter, "{");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter
        .input(Event::Paste("retained studio composition draft".into()))
        .unwrap();
    click(presenter, "[ Overview ]");
    assert!(!screen(&draw(presenter)).contains("retained studio composition draft"));
    click(presenter, "[ Plugins ]");
    await_screen(presenter, "retained studio composition draft");
    assert_eq!(
        runtime
            .block_on(caller.application_head(next.deployment))
            .unwrap(),
        Some(next)
    );
    // Retire this native surface before closing its distinct Host; original
    // manager/A remain owned by the outer test until their actual acknowledgements.
    presenter
        .surface_closed(&presenter.surface(), Ok(()))
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !presenter.can_shutdown() {
        presenter.pump().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    runtime.block_on(connection.close()).unwrap();
    runtime.block_on(server.close()).unwrap();
    drop(connection);
    drop(server);
    runtime.block_on(app.shutdown()).unwrap();
    drop(app);
    runtime.block_on(host_store.shutdown()).unwrap();
}
