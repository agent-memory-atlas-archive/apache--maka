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

#[path = "product/studio.rs"]
mod studio;

use super::*;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use maka_plugin_core::deployment::RemoteObservation;
use maka_ui::actions::RemoteService;
use ratatui::{buffer::Buffer, layout::Rect, style::Color};
use std::{path::PathBuf, process::Command, sync::OnceLock};

struct Physical {
    server: Arc<maka_application_bridge::transport::Server>,
    peer: Arc<maka_application_bridge::transport::Peers>,
}
impl maka_application_bridge::transport::native::Connector for Physical {
    fn connect(
        &self,
        stop: tokio_util::sync::CancellationToken,
    ) -> futures_util::future::BoxFuture<
        'static,
        Result<maka_application_bridge::transport::native::Stream, String>,
    > {
        let server = self.server.clone();
        let peer = self.peer.clone();
        Box::pin(async move {
            if stop.is_cancelled() {
                return Err("physical opening cancelled before admission".into());
            }
            let (channel, accepted) = tokio::io::duplex(64 * 1024);
            let (reader, writer) = tokio::io::split(accepted);
            drop(server.serve(reader, writer, peer)?);
            Ok(Box::new(channel) as maka_application_bridge::transport::native::Stream)
        })
    }
}
fn draw(presenter: &mut maka_tui::application::Presenter) -> Buffer {
    draw_at(presenter, Rect::new(0, 0, 80, 24))
}
fn draw_at(presenter: &mut maka_tui::application::Presenter, area: Rect) -> Buffer {
    presenter.pump().unwrap();
    let mut buffer = Buffer::empty(area);
    let scene = presenter.paint(area, &mut buffer).unwrap();
    presenter.present(scene);
    buffer
}
fn screen(buffer: &Buffer) -> String {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn click(presenter: &mut maka_tui::application::Presenter, label: &str) {
    click_nth(presenter, label, 0);
}
fn click_nth(presenter: &mut maka_tui::application::Presenter, label: &str, nth: usize) {
    let (x, y) = wait_for_target(presenter, label, nth);
    presenter
        .input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
}
fn wait_for_target(
    presenter: &mut maka_tui::application::Presenter,
    label: &str,
    nth: usize,
) -> (u16, u16) {
    // Disabled buttons retain their label. This fixture uses the ordinary light
    // theme, whose enabled button foreground is its accent. Await that painted
    // state before one click; never replay a potentially committed operation.
    static BUTTON_COLOR: OnceLock<Color> = OnceLock::new();
    let enabled = label.starts_with("[ ").then(|| {
        *BUTTON_COLOR.get_or_init(|| {
            let module = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../packages/agent-product/dist/packages/appearance/tui.js");
            let output = Command::new("node")
                .args(["--input-type=module", "-e", "const {activate}=await import(process.argv[1]); const theme=activate({scheme:'light'}).Appearance.value; console.log(JSON.stringify({accent:theme.accent>>>8,muted:theme.muted>>>8}));"])
                .arg(format!("file://{}", module.display()))
                .output()
                .unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            let colors: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_ne!(colors["accent"], colors["muted"], "fixture must distinguish enabled and disabled buttons");
            let rgb = colors["accent"].as_u64().unwrap() as u32;
            Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
        })
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let (x, y) = loop {
        let buffer = draw(presenter);
        let position = (0..buffer.area.height)
            .filter_map(|y| {
                let row = (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>();
                row.find(label).map(|x| (x as u16, y))
            })
            .nth(nth);
        if let Some((x, y)) = position
            && enabled.is_none_or(|color| buffer[(x, y)].fg == color)
        {
            break (x, y);
        }
        assert!(
            std::time::Instant::now() < deadline,
            "missing enabled {label} #{nth} in {}",
            screen(&buffer)
        );
        std::thread::yield_now();
    };
    (x, y)
}
#[track_caller]
fn await_screen(presenter: &mut maka_tui::application::Presenter, expected: &str) {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let rendered = screen(&draw(presenter));
        if rendered.contains(expected) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "missing {expected}: {rendered}"
        );
        std::thread::yield_now();
    }
}
// Requires the normal Node build dependencies; uses the real packaged guest,
// original native editor, persisted caller intents and native authenticated run.
#[test]
#[ignore = "opt-in product authoring build and actual HTTP provider"]
fn packaged_product_creates_reads_sends_and_restores_through_actual_native_owners() {
    packaged_product(false);
}

#[test]
#[ignore = "opt-in product authoring build and actual HTTP provider"]
fn packaged_metadata_recovers_a_real_native_initialization_conflict() {
    packaged_product(true);
}

fn packaged_product(fail_labels_initialization: bool) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let build = Command::new("node")
        .arg(repository.join("packages/agent-product/scripts/build.mjs"))
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    const KEY: &str = "native-product-key-378f4a";
    let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let (respond, response_ready) = tokio::sync::oneshot::channel();
    let server = runtime.spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let body = request_with_key(&mut socket, KEY).await;
        assert_eq!(body["model"], "fixture-model-19");
        assert!(body.to_string().contains("Hello product"));
        response_ready.await.unwrap();
        response(&mut socket).await;
    });
    let peer = Arc::new(maka_application_bridge::transport::PeerPolicy::default());
    let mut fixture = runtime.block_on(Fixture::with_peer(&base, 1, None, Some(peer.clone())));
    // A short temporary prefix lets the original compact path actually retain
    // enough wide cells to require two terminal rows (macOS TMPDIR is longer).
    let wide_root = fail_labels_initialization.then(|| tempfile::tempdir_in("/tmp").unwrap());
    if let Some(root) = &wide_root {
        let wide = root.path().join("界".repeat(65));
        std::fs::create_dir(&wide).unwrap();
        let mut roots = fixture.owner.settings.workspace_roots.clone();
        roots[0].path = wide.clone();
        runtime.block_on(super::workspaces::publish_with_policy(
            &mut fixture,
            roots,
            peer.clone(),
        ));
        fixture.workspace = wide.canonicalize().unwrap();
    }
    runtime.block_on(super::selection::edit(&fixture, |changes| {
        changes.enabled_model_ids =
            Some((0..20).map(|at| format!("fixture-model-{at:02}")).collect());
    }));
    for (at, name) in [
        "W".repeat(256),
        format!("{}X", "\n".repeat(255)),
        "界".repeat(256),
    ]
    .into_iter()
    .enumerate()
    {
        let catalog = runtime.block_on(fixture.config.catalog()).unwrap();
        let connection = &catalog.connections[0];
        let result = runtime
            .block_on(
                fixture.config.create_connection(
                    serde_json::from_value(json!({
                        "expectedCatalogRevision":catalog.revision,
                        "connection": {"slug":format!("long-label-{at}"),"name":name,
                            "provider":connection.provider,"configuration":connection.configuration,
                            "enabled":false,"enabledModelIds":[]}
                    }))
                    .unwrap(),
                ),
            )
            .unwrap();
        assert!(matches!(result, CatalogMutationResult::Committed { .. }));
    }
    let ui_owner =
        Arc::new(RootOwner::create(&fixture._dir.path().join("ui"), &fixture.namespaces).unwrap());
    let caller = Arc::new(runtime.block_on(EventLog::for_root(ui_owner)).unwrap());
    let directory = repository.join("packages/agent-product/dist/packages");
    let names = [
        "workspace",
        "command-palette",
        "directory-picker",
        "session-details",
        "help",
        "deployment",
        "deployment-manager",
        "deployment-gateway",
        "receipts",
        "image-viewer",
        "composer",
        "models",
        "model-default",
        "connections",
        "credential",
        "connection-form",
        "input",
        "navigation",
        "timeline",
        "reading-area",
        "source-text",
        "appearance",
        "surface",
        "editor",
        "file",
        "reader",
        "history",
    ];
    let packages = names
        .iter()
        .map(|name| {
            let package = Package::read_from(&directory.join(name)).unwrap();
            if *name != "session-details" || !fail_labels_initialization {
                return package;
            }
            // This fixture changes one ordinary guest command's CAS basis.
            // The original native Textarea must produce the real conflict;
            // no fabricated observation or receipt enters the presenter.
            // Wrap the public export, never an unstable bundler-local name.
            let wrapped = Command::new("node")
                .arg("--input-type=module")
                .arg("-e")
                .arg(
                    r#"
import { build } from 'esbuild';
const contents = `
import { component, activate } from ${JSON.stringify(process.argv[1])};
export { component, activate };
let failedLabels = false;
for (const method of ['event', 'update']) {
    const original = component[method];
    component[method] = (...args) => {
        const out = original(...args);
        const command = out.frame.state.fields.labels.command;
        if (command && !failedLabels) {
            failedLabels = true;
            command.basedOn += 1000;
        }
        return out;
    };
}
`;
const output = await build({
    stdin: { contents, resolveDir: process.cwd(), loader: 'js' },
    bundle: true, format: 'esm', platform: 'neutral', target: 'es2022',
    write: false, logLevel: 'silent',
});
process.stdout.write(output.outputFiles[0].text);
"#,
                )
                .arg(directory.join(name).join("tui.js"))
                .current_dir(&repository)
                .output()
                .unwrap();
            assert!(
                wrapped.status.success(),
                "{}",
                String::from_utf8_lossy(&wrapped.stderr)
            );
            let mut files = package.files().clone();
            *files.get_mut("tui.js").unwrap() = wrapped.stdout;
            Package::new(files).unwrap()
        })
        .collect::<Vec<_>>();
    let refs = packages
        .iter()
        .map(|package| {
            (
                package.manifest().id.strip_prefix("maka.agent.").unwrap(),
                package.application_ref(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    // The authoring function accepts actual Rust-computed package refs. Do not
    // duplicate package hashing or native manifest schemas in a fixture.
    let script = r#"const {composition,packageNames}=await import(process.argv[1]);const refs=JSON.parse(process.argv[2]);const keys=Object.fromEntries(packageNames.map((name,i)=>[name,'00000000-0000-0000-0000-'+String(i+20).padStart(12,'0')]));keys.agent='00000000-0000-0000-0000-000000000001';keys.receipts='00000000-0000-0000-0000-000000000004';keys.settings='00000000-0000-0000-0000-000000000005';for(const name of packageNames)refs[name]??={id:'unused.'+name,revision:'sha256-'+ 'a'.repeat(64)};const result=composition({presentation:'tui',locale:'en',scheme:'light',packages:refs,keys,agent:{workspaceRoots:[{label:"Fixture",path:"/fixture-workspace"}],maxActiveRuns:1,maxSteps:1,tools:[]},hostRoot:'/fixture',hostDeployment:'00000000-0000-0000-0000-000000000080',managementDeployment:'00000000-0000-0000-0000-000000000081'});console.log(JSON.stringify({input:result.ui,observations:result.remoteExports,managementHost:result.managementHost,managementConfiguration:result.connections.hosts.find(h=>h.name==='manager')}));"#;
    let authored = Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(script)
        .arg(format!(
            "file://{}",
            repository
                .join("packages/agent-product/dist/authoring/agent-product/src/composition.mjs")
                .display()
        ))
        .arg(serde_json::to_string(&refs).unwrap())
        .output()
        .unwrap();
    assert!(
        authored.status.success(),
        "{}",
        String::from_utf8_lossy(&authored.stderr)
    );
    let authored: Value = serde_json::from_slice(&authored.stdout).unwrap();
    let input: Input = serde_json::from_value(authored["input"].clone()).unwrap();
    let observations: Vec<RemoteObservation> =
        serde_json::from_value(authored["observations"].clone()).unwrap();
    let mut declarations = maka_plugin_core::package::Packages::default();
    for package in &packages {
        declarations
            .insert(
                package.application_ref(),
                package.manifest().application.clone().unwrap(),
            )
            .unwrap();
    }
    let exports = observations
        .iter()
        .map(|observation| {
            (
                observation.capability.clone(),
                observation.declaration.clone(),
            )
        })
        .collect();
    let plan = maka_plugin_core::plan::Plan::prepare(&input, &declarations, &exports).unwrap();
    let consumers = plan.instances().values()
        .filter(|instance| !instance.disabled && instance.entry.is_some())
        .filter(|instance| instance.bindings.values().flatten().any(|binding|
            matches!(&binding.provider, maka_plugin_core::plan::Provider::Remote { capability } if capability.deployment == "local")))
        .map(|instance| instance.identity.clone()).collect();
    let deployment = runtime
        .block_on(caller.commit_application(
            Uuid::new_v4(),
            0,
            input,
            observations.clone(),
            packages,
        ))
        .unwrap();
    let receipt_binding = runtime
        .block_on(fixture.app.client().bind(
            fixture.app.client().activation(&id(2)).unwrap(),
            "receipts".into(),
        ))
        .unwrap()
        .remove(0);
    let settings_binding = runtime
        .block_on(fixture.app.client().bind(
            fixture.app.client().activation(&id(2)).unwrap(),
            "settings".into(),
        ))
        .unwrap()
        .remove(0);
    let bindings = [
        runtime.block_on(fixture.binding(2)),
        receipt_binding,
        settings_binding,
    ];
    use maka_application_bridge::transport::{
        Endpoint, Peer, Peers, Server,
        native::{Connection, Service},
    };
    let endpoints = bindings
        .iter()
        .enumerate()
        .map(|(index, binding)| {
            let maka_plugin_core::contract::Interface::Service { methods } =
                &binding.contract().interface
            else {
                panic!("service contract")
            };
            // This trusted product fixture explicitly grants the selected contracts.
            // Only the original receipt-reader endpoint has historical authority.
            (
                format!("service-{index}"),
                Endpoint::new(
                    binding.clone(),
                    maka_application_bridge::Grant {
                        methods: methods.keys().cloned().collect(),
                        recovery: index == 1,
                    },
                )
                .unwrap(),
            )
        })
        .collect();
    let wire_server = Server::new(fixture.app.client(), peer, endpoints).unwrap();
    let callers = Peers::new(Peer::new(deployment.clone(), consumers).unwrap());
    let connection = Connection::new(Arc::new(Physical {
        server: wire_server.clone(),
        peer: callers.clone(),
    }));
    let manager_input: Input = serde_json::from_value(authored["managementHost"].clone()).unwrap();
    let manager_configuration: maka_application_bridge::Host =
        serde_json::from_value(authored["managementConfiguration"].clone()).unwrap();
    let manager_packages = ["deployment-manager", "deployment-gateway", "receipts"]
        .iter()
        .map(|name| Package::read_from(&directory.join(name)).unwrap())
        .collect();
    runtime
        .block_on(caller.commit_application(
            manager_configuration.deployment,
            0,
            manager_input,
            vec![],
            manager_packages,
        ))
        .unwrap();
    let target = runtime
        .block_on(
            maka_runtime_host::application::management::ManagementTarget::from_source(
                caller.clone(),
                deployment.clone(),
            ),
        )
        .unwrap();
    let mut manager_native = maka_application_bridge::with_gateway(Default::default()).unwrap();
    maka_runtime_host::application::management::provide_target(
        &mut manager_native,
        maka_application_bridge::UI_DEPLOYMENT_MANAGER.into(),
        target,
    )
    .unwrap();
    let manager_policy = Arc::new(maka_application_bridge::transport::PeerPolicy::default());
    let manager_app = runtime
        .block_on(
            maka_runtime_host::application::Application::start_from_store(
                caller.clone(),
                manager_configuration.deployment,
                manager_native,
                manager_policy.issuer(),
                manager_policy.clone(),
                Default::default(),
            ),
        )
        .unwrap();
    runtime.block_on(manager_app.ready()).unwrap();
    let manager_client = manager_app.client();
    let mut manager_endpoints = BTreeMap::new();
    for (name, grant) in &manager_configuration.imports {
        let binding = runtime
            .block_on(
                manager_client.bind(
                    manager_client
                        .activation(&manager_configuration.consumer)
                        .unwrap(),
                    name.clone(),
                ),
            )
            .unwrap()
            .remove(0);
        manager_endpoints.insert(name.clone(), Endpoint::new(binding, grant.clone()).unwrap());
    }
    let manager_server = Server::new(manager_client, manager_policy, manager_endpoints).unwrap();
    let manager_consumers = plan.instances().values().filter(|instance| !instance.disabled && instance.entry.is_some())
        .filter(|instance| instance.bindings.values().flatten().any(|binding|
            matches!(&binding.provider,maka_plugin_core::plan::Provider::Remote {capability} if capability.deployment=="manager")))
        .map(|instance|instance.identity.clone()).collect();
    let manager_callers = Peers::new(Peer::new(deployment.clone(), manager_consumers).unwrap());
    let manager_connection = Connection::new(Arc::new(Physical {
        server: manager_server.clone(),
        peer: manager_callers.clone(),
    }));
    let registrations: Vec<_> = observations
        .iter()
        .enumerate()
        .map(|(index, observation)| {
            let (server, connection, peers, alias) =
                if observation.capability.deployment == "manager" {
                    (
                        &manager_server,
                        &manager_connection,
                        &manager_callers,
                        if observation.capability.export == "Management" {
                            "management".to_string()
                        } else {
                            "receipts".to_string()
                        },
                    )
                } else {
                    (
                        &wire_server,
                        &connection,
                        &callers,
                        format!("service-{index}"),
                    )
                };
            let selected = server.catalogue().select(&alias).unwrap();
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
    let start = || {
        runtime
            .block_on(maka_tui::application::Presenter::start_from_store(
                caller.clone(),
                deployment.clone(),
                runtime.handle().clone(),
                maka_tui::application::registry(),
                json!({}),
                registrations
                    .iter()
                    .map(|registration| RemoteService {
                        target: registration.target.clone(),
                        registration_epoch: registration.registration_epoch,
                        contract: registration.contract.clone(),
                        implementation: registration.implementation.clone(),
                    })
                    .collect(),
            ))
            .unwrap()
    };
    let mut presenter = start();
    await_screen(&mut presenter, "[ Show sidebar ]");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('b'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    wait_for_target(&mut presenter, "[ New conversation ]", 0);
    // The ordinary directory picker creates the initial Session through its
    // original observed selection, physical query focus and recorded Root intent.
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    await_screen(&mut presenter, "Search commands or conversations");
    presenter
        .input(Event::Paste("another folder".into()))
        .unwrap();
    wait_for_target(
        &mut presenter,
        "[ › New conversation in another folder ]",
        0,
    );
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(&mut presenter, "[ › Workspace ]");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "[ › Other workspace ]");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "[ › Workspace ]");
    // Native button focus owns Enter after Tab; query arrows cannot redirect it.
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
        .unwrap();
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
        .unwrap();
    assert!(screen(&draw(&mut presenter)).contains("[ › Workspace ]"));
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    click(&mut presenter, "[ Create here ]");
    await_screen(&mut presenter, "New conversation");
    draw(&mut presenter);
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "A place to think.");
    // Closing navigation restores the retained native composer through its
    // declared tab order. This command/draft path needs no mouse or forged input.
    presenter
        .input(Event::Paste("Hello product".into()))
        .unwrap();
    await_screen(&mut presenter, "Hello product");
    // Help is an ordinary guest under the existing modal. Native dismissal
    // restores the original composer owner and its draft without a new snapshot.
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "Keyboard shortcuts");
    await_screen(&mut presenter, "[ Close help ]");
    let narrow = draw_at(&mut presenter, Rect::new(0, 0, 40, 16));
    let (x, y) = (0..narrow.area.height)
        .find_map(|y| {
            let row = (0..narrow.area.width)
                .map(|x| narrow[(x, y)].symbol())
                .collect::<String>();
            row.find("[ Close help ]").map(|x| (x as u16, y))
        })
        .expect("help footer stays reachable at the original narrow terminal size");
    presenter
        .input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
    await_screen(&mut presenter, "Hello product");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "Keyboard shortcuts");
    let narrow = draw_at(&mut presenter, Rect::new(0, 0, 40, 16));
    assert!(screen(&narrow).contains("[ Close help ]"));
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "Hello product");
    presenter.input(Event::Paste("!".into())).unwrap();
    await_screen(&mut presenter, "Hello product!");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Backspace,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(&mut presenter, "Hello product");
    // Both fields retain their original native editor owners under the Dialog.
    // A real rejected initializer must leave its Retry physically reachable.
    click(&mut presenter, "[ Details ]");
    if fail_labels_initialization {
        click(&mut presenter, "[ Retry loading this field ]");
    }
    wait_for_target(&mut presenter, "[ Save details ]", 0);
    if fail_labels_initialization {
        assert!(
            screen(&draw(&mut presenter))
                .lines()
                .filter(|line| line.contains('界'))
                .count()
                >= 2
        );
    }
    let before = runtime
        .block_on(
            fixture
                .store
                .list_sessions::<SessionConfiguration>(None, None, 32),
        )
        .unwrap()
        .sessions
        .pop()
        .unwrap();
    let session_id = before.id.clone();
    let original_workspace = before.configuration.workspace.clone();
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter
        .input(Event::Paste("Metadata conversation".into()))
        .unwrap();
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
        .unwrap();
    presenter
        .input(Event::Paste("orchid\nrust".into()))
        .unwrap();
    click(&mut presenter, "[ Save details ]");
    await_screen(&mut presenter, "Conversation details saved.");
    let observed = runtime
        .block_on(fixture.owner.session(&session_id))
        .unwrap();
    assert_eq!(observed.configuration.title, "Metadata conversation");
    assert_eq!(observed.configuration.labels, ["orchid", "rust"]);
    assert_eq!(observed.configuration.workspace, original_workspace);
    assert_eq!(observed.revision, before.revision + 1);
    // Resolve and click the original foreground field from one flushed frame;
    // a concurrent canonical header refresh can add another title above it.
    let buffer = draw(&mut presenter);
    let (column, row) = screen(&buffer)
        .lines()
        .enumerate()
        .find_map(|(row, line)| {
            (row > 3 && row < 21)
                .then(|| {
                    line.find("Metadata conversation")
                        .map(|column| (column as u16, row as u16))
                })
                .flatten()
        })
        .expect("retained foreground title must be painted");
    presenter
        .input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    let restored_title = if fail_labels_initialization {
        "Old metadata target"
    } else {
        "New conversation"
    };
    presenter
        .input(Event::Paste(restored_title.into()))
        .unwrap();
    click(&mut presenter, "[ Save details ]");
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        draw(&mut presenter);
        let observed = runtime
            .block_on(fixture.owner.session(&session_id))
            .unwrap();
        if observed.configuration.title == restored_title {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "second original metadata operation did not settle: {} / {:?}\n{}",
            observed.configuration.title,
            observed.configuration.labels,
            screen(&draw(&mut presenter))
        );
        std::thread::yield_now();
    }
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "Hello product");
    if fail_labels_initialization {
        // Saved notice, retained old fields, an actual different selected
        // Session and a two-row CJK workspace path used to hide all actions.
        click(&mut presenter, "[ Show sidebar ]");
        click(&mut presenter, "[ New conversation ]");
        wait_for_target(&mut presenter, "[ New conversation ]", 1);
        click(&mut presenter, "[ Hide sidebar ]");
        click(&mut presenter, "[ Details ]");
        click(&mut presenter, "[ Review current details ]");
        await_screen(&mut presenter, "Review these saved details.");
        click(&mut presenter, "[ Discard draft and reload ]");
        click(&mut presenter, "[ Discard draft and reload ]");
        wait_for_target(&mut presenter, "[ Save details ]", 0);
        presenter
            .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .unwrap();
        presenter
            .surface_closed(&presenter.surface(), Ok(()))
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !presenter.can_shutdown() {
            presenter.pump().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        presenter.shutdown().unwrap();
        drop(presenter);
        runtime.block_on(async {
            let (physical, host_wire) = tokio::join!(connection.close(), wire_server.close());
            physical.unwrap();
            host_wire.unwrap();
            manager_connection.close().await.unwrap();
            manager_server.close().await.unwrap();
        });
        drop(registrations);
        runtime.block_on(manager_app.shutdown()).unwrap();
        runtime.block_on(fixture.app.shutdown()).unwrap();
        runtime.block_on(fixture.config.shutdown()).unwrap();
        runtime.block_on(caller.shutdown()).unwrap();
        return;
    }
    // The ordinary Root Dialog retains the original composer while the original
    // single-line query consumes foreground physical input and returns focus.
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    await_screen(&mut presenter, "another folder");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter.input(Event::Paste("Hello query".into())).unwrap();
    await_screen(&mut presenter, "Hello query");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    await_screen(&mut presenter, "Hello product");
    click(&mut presenter, "[ Commands ]");
    await_screen(&mut presenter, "Hello query");
    presenter
        .input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
    await_screen(&mut presenter, "Hello product");
    click(&mut presenter, "[ Commands ]");
    await_screen(&mut presenter, "Hello query");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter
        .input(Event::Paste("another folder".into()))
        .unwrap();
    wait_for_target(
        &mut presenter,
        "[ › New conversation in another folder ]",
        0,
    );
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(&mut presenter, "Filter folders on this page");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "Hello product");
    click(&mut presenter, "[ Commands ]");
    await_screen(&mut presenter, "another folder");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "Hello product");
    click(&mut presenter, "[ Plugins ]");
    await_screen(&mut presenter, "[ Refresh components ]");
    // The first requested component is the required Workspace. A recorded
    // refusal must preserve the real product and its retained native draft.
    await_screen(&mut presenter, "[ Disable ]");
    click(&mut presenter, "[ Disable ]");
    await_screen(&mut presenter, "Disable this component?");
    click(&mut presenter, "[ Confirm ]");
    await_screen(&mut presenter, "not allowed by the composition");
    assert_eq!(
        runtime
            .block_on(caller.application_head(deployment.deployment))
            .unwrap(),
        Some(deployment.clone())
    );
    click(&mut presenter, "[ Edit product composition ]");
    // Enabled Save is reached only after every original native initialization
    // receipt. Cancel this first confirmation without touching its buffer.
    click(&mut presenter, "[ Save composition ]");
    await_screen(&mut presenter, "Replace this product’s composition?");
    click(&mut presenter, "[ Cancel ]");
    click_nth(&mut presenter, "{", 0);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter.input(Event::Paste("{".into())).unwrap();
    click(&mut presenter, "[ Save composition ]");
    click(&mut presenter, "[ Confirm ]");
    await_screen(&mut presenter, "EOF while parsing");
    assert_eq!(
        runtime
            .block_on(caller.application_head(deployment.deployment))
            .unwrap(),
        Some(deployment.clone())
    );
    click(&mut presenter, "[ Reload saved source ]");
    await_screen(&mut presenter, "Discard this draft and reload?");
    click(&mut presenter, "[ Cancel ]");
    assert!(screen(&draw(&mut presenter)).contains('{'));
    click(&mut presenter, "[ Reload saved source ]");
    click(&mut presenter, "[ Confirm ]");
    click(&mut presenter, "[ Save composition ]");
    click(&mut presenter, "[ Confirm ]");
    await_screen(&mut presenter, "Changes saved.");
    let saved = runtime
        .block_on(caller.application_head(deployment.deployment))
        .unwrap()
        .unwrap();
    assert_eq!(saved.revision, deployment.revision + 1);
    let saved_input = runtime
        .block_on(caller.read_application_metadata_at(&saved))
        .unwrap()
        .unwrap()
        .source
        .input;
    let original_input = runtime
        .block_on(caller.read_application_metadata_at(&deployment))
        .unwrap()
        .unwrap()
        .source
        .input;
    assert_eq!(saved_input, original_input);
    // Keep a modified advanced draft while importing an immutable new package
    // through physical native path input, real capture and the original Host.
    click_nth(&mut presenter, "{", 0);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter.input(Event::Paste("{".into())).unwrap();
    click(&mut presenter, "[ Back ]");
    click(&mut presenter, "[ Install plugin ]");
    await_screen(&mut presenter, "Absolute path to a .maka-plugin file");
    let original_package = Package::read_from(&directory.join("appearance")).unwrap();
    let mut files = original_package.files().clone();
    let mut manifest: Value =
        serde_json::from_slice(files.get(maka_plugins::package::MANIFEST_FILE).unwrap()).unwrap();
    manifest["id"] = json!("local.imported-theme");
    files.insert(
        maka_plugins::package::MANIFEST_FILE.into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    let imported = Package::new(files).unwrap();
    let bundle = fixture._dir.path().join("selected-theme.maka-plugin");
    imported.export_to(&bundle).unwrap();
    click(&mut presenter, "Absolute path to a .maka-plugin file");
    presenter
        .input(Event::Paste(bundle.to_str().unwrap().into()))
        .unwrap();
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(&mut presenter, "selected-theme.maka-plugin");
    click(&mut presenter, "[ Install plugin ]");
    await_screen(&mut presenter, "Install this plugin package?");
    click(&mut presenter, "[ Cancel ]");
    click(&mut presenter, "[ Install plugin ]");
    // The selected native bytes stay immutable after the filesystem changes.
    std::fs::write(&bundle, b"replaced after selection").unwrap();
    click(&mut presenter, "[ Confirm ]");
    await_screen(&mut presenter, "Plugin package saved.");
    await_screen(&mut presenter, "local.imported-theme");
    let installed_head = runtime
        .block_on(caller.application_head(deployment.deployment))
        .unwrap()
        .unwrap();
    assert_eq!(installed_head.revision, saved.revision + 1);
    let installed = runtime
        .block_on(caller.read_application_metadata_at(&installed_head))
        .unwrap()
        .unwrap();
    assert_eq!(installed.source.input, original_input);
    assert!(installed.packages.contains(&imported.application_ref()));
    let loaded = runtime
        .block_on(caller.load_application_at(&installed_head))
        .unwrap()
        .unwrap();
    assert_eq!(
        loaded
            .packages
            .iter()
            .find(|package| package.application_ref() == imported.application_ref())
            .unwrap()
            .to_bundle(),
        imported.to_bundle()
    );
    click(&mut presenter, "[ Edit product composition ]");
    // The modified draft remains visible and still refers to its original source.
    assert!(screen(&draw(&mut presenter)).contains('{'));
    click(&mut presenter, "[ Save composition ]");
    click(&mut presenter, "[ Confirm ]");
    await_screen(&mut presenter, "Components changed.");
    assert_eq!(
        runtime
            .block_on(caller.application_head(deployment.deployment))
            .unwrap(),
        Some(installed_head)
    );
    // This fixture retains its explicitly admitted S1 registrations; production
    // foreground adoption is separately tested with the original Connections.
    click(&mut presenter, "[ Back ]");
    click(&mut presenter, "[ Back ]");
    await_screen(&mut presenter, "Hello product");
    // Ordinary settings owns the route; the original conversation editor is
    // retained while a distinct native SecretInput is captured by Settings.
    click(&mut presenter, "[ Connections ]");
    await_screen(&mut presenter, "[ API key ]");
    // Public fields use their original native owners. A schema-invalid but
    // syntactically valid override must remain fixable without retiring them.
    click(&mut presenter, "[ Edit connection ]");
    await_screen(&mut presenter, "[ Edit / replace: Name ]");
    click(&mut presenter, "[ Edit / replace: Name ]");
    await_screen(&mut presenter, "Fixture");
    click(&mut presenter, "Fixture");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter
        .input(Event::Paste("Public native name\nretained".into()))
        .unwrap();
    for _ in 0..4 {
        click(&mut presenter, "[ Next field ]");
    }
    await_screen(&mut presenter, "Model overrides");
    click(&mut presenter, "[ Edit / replace: Model overrides ]");
    await_screen(&mut presenter, "null");
    // The help text also mentions null; the second visible occurrence is the
    // original native editor rather than that noninteractive label.
    click_nth(&mut presenter, "null", 1);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter
        .input(Event::Paste(
            r#"{"fixture-model-19":{"vision":null}}"#.into(),
        ))
        .unwrap();
    assert!(screen(&draw(&mut presenter)).contains("vision"));
    click(&mut presenter, "[ Save connection ]");
    await_screen(&mut presenter, "Check the name");
    assert!(screen(&draw(&mut presenter)).contains("vision"));
    click(&mut presenter, "vision");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter
        .input(Event::Paste(
            r#"{"fixture-model-19":{"vision":true}}"#.into(),
        ))
        .unwrap();
    // A concurrent actor changes only the saved name. The retained local name
    // and overrides can only be applied after explicit review of the new basis.
    runtime.block_on(super::selection::edit(&fixture, |changes| {
        changes.name = Some("Remote name".into());
    }));
    click(&mut presenter, "[ Save connection ]");
    await_screen(&mut presenter, "The saved connection changed.");
    click(&mut presenter, "Review current connection");
    await_screen(&mut presenter, "[ Use this connection ]");
    click(&mut presenter, "[ Use this connection ]");
    click(&mut presenter, "[ Save connection ]");
    await_screen(&mut presenter, "[ Refresh connections ]");
    let public_row = runtime
        .block_on(fixture.config.catalog())
        .unwrap()
        .connections
        .remove(0);
    assert_eq!(public_row.name, "Public native name\nretained");
    assert_eq!(
        public_row.enabled_model_ids.len(),
        20,
        "unread model IDs must remain untouched"
    );
    assert_eq!(public_row.configuration, json!({"baseUrl":base}));
    assert_eq!(
        public_row.model_overrides.as_ref().unwrap()["fixture-model-19"].vision,
        Some(true)
    );
    // Create through the same ordinary Settings binding and real provider page,
    // then explicitly rebase a catalog CAS conflict without replacing owners.
    click(&mut presenter, "[ Add connection ]");
    await_screen(&mut presenter, "Choose a provider");
    await_screen(&mut presenter, "[ Refresh connections ]");
    for _ in 0..24 {
        if screen(&draw(&mut presenter)).contains("[ Chat Completions ]") {
            break;
        }
        click(&mut presenter, "[ More providers ]");
        await_screen(&mut presenter, "[ Refresh connections ]");
    }
    click(&mut presenter, "[ Chat Completions ]");
    await_screen(&mut presenter, "[ Save connection ]");
    click(&mut presenter, "Chat Completions");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter
        .input(Event::Paste("Created public".into()))
        .unwrap();
    click(&mut presenter, "[ Next field ]");
    click(&mut presenter, "Enter: Connection ID");
    presenter
        .input(Event::Paste("created-public".into()))
        .unwrap();
    click(&mut presenter, "[ Next field ]");
    click(&mut presenter, "Enter: Endpoint");
    presenter.input(Event::Paste(base.clone())).unwrap();
    click(&mut presenter, "[ Next field ]");
    click(&mut presenter, "Enter: Model IDs");
    presenter
        .input(Event::Paste("created-model".into()))
        .unwrap();
    click(&mut presenter, "[ Enabled ]");
    runtime.block_on(super::selection::edit(&fixture, |changes| {
        changes.name = Some("Public concurrent name".into());
    }));
    click(&mut presenter, "[ Save connection ]");
    await_screen(&mut presenter, "The saved connection changed.");
    click(&mut presenter, "Review current connection");
    await_screen(&mut presenter, "[ Use this connection ]");
    click(&mut presenter, "[ Use this connection ]");
    click(&mut presenter, "[ Save connection ]");
    await_screen(&mut presenter, "[ Refresh connections ]");
    let catalog = runtime.block_on(fixture.config.catalog()).unwrap();
    let created = catalog
        .connections
        .iter()
        .find(|row| row.slug == "created-public")
        .unwrap();
    assert_eq!(created.name, "Created public");
    assert_eq!(created.configuration, json!({"baseUrl":base}));
    assert_eq!(created.enabled_model_ids, vec!["created-model"]);
    assert!(!created.enabled);
    // Every original local page item is physically reachable at 80x24,
    // even for the full legal label length and embedded line breaks.
    click_nth(&mut presenter, "[ API key ]", 2);
    await_screen(&mut presenter, "No API key is saved.");
    click(&mut presenter, "[ Back ]");
    await_screen(&mut presenter, "[ More connections ]");
    click(&mut presenter, "[ More connections ]");
    await_screen(&mut presenter, "[ Previous connections ]");
    click(&mut presenter, "[ API key ]");
    await_screen(&mut presenter, "No API key is saved.");
    click(&mut presenter, "[ Back ]");
    await_screen(&mut presenter, "[ API key ]");
    click(&mut presenter, "[ API key ]");
    await_screen(&mut presenter, "An API key is saved.");
    click(&mut presenter, "Enter an API key");
    presenter.input(Event::Paste(KEY.into())).unwrap();
    assert!(!screen(&draw(&mut presenter)).contains(KEY));
    // Another actor deletes only the vault generation. Connection revision
    // stays unchanged, so the form must rely on its observed credential CAS.
    let row = runtime
        .block_on(fixture.config.catalog())
        .unwrap()
        .connections
        .remove(0);
    let locator: CredentialLocator = serde_json::from_value(json!({
        "scope":"connection", "connectionId":row.connection_id, "kind":"provider"
    }))
    .unwrap();
    let status = runtime
        .block_on(fixture.config.credential_status(locator.clone()))
        .unwrap();
    let CredentialVaultQueryResult::Status { status } = status else {
        panic!("status");
    };
    let target = ConnectionCredentialTarget {
        connection_id: row.connection_id,
        revision: row.revision,
        slug: row.slug,
        provider: row.provider,
        configuration: row.configuration,
    };
    assert!(matches!(
        runtime
            .block_on(fixture.config.delete_credential(DeleteCredentialInput {
                expected: status.basis().unwrap(),
                expected_connection: Some(target),
            }))
            .unwrap(),
        CredentialMutationResult::Committed { .. }
    ));
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(&mut presenter, "The connection or saved key changed.");
    assert!(!screen(&draw(&mut presenter)).contains(KEY));
    click(&mut presenter, "Review current connection");
    await_screen(&mut presenter, "[ Use this connection ]");
    click(&mut presenter, "[ Use this connection ]");
    click(&mut presenter, "•••");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(&mut presenter, "API key saved.");
    await_screen(&mut presenter, "Enter an API key");
    let envelope: maka_runtime::provider::Credential = serde_json::from_str(
        &runtime
            .block_on(fixture.config.credential_secret(&locator, None))
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(envelope.secret, KEY);
    click(&mut presenter, "[ Back ]");
    await_screen(&mut presenter, "[ Refresh connections ]");
    click(&mut presenter, "[ Back ]");
    await_screen(&mut presenter, "Hello product");
    await_screen(&mut presenter, "[ Choose model ]");
    click(&mut presenter, "[ Choose model ]");
    await_screen(&mut presenter, "fixture-model-00");
    for expected in ["03", "06", "09", "12", "15", "16", "19"] {
        click(&mut presenter, "More models");
        await_screen(&mut presenter, &format!("fixture-model-{expected}"));
    }
    assert!(screen(&draw(&mut presenter)).contains("Close"));
    click(&mut presenter, "fixture-model-19");
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while screen(&draw(&mut presenter)).contains("[ Close ]") {
        assert!(
            std::time::Instant::now() < deadline,
            "native model snapshot did not settle"
        );
        std::thread::yield_now();
    }
    assert!(screen(&draw(&mut presenter)).contains("Hello product"));
    // Opening reveals the retained native query input. Its first paste must
    // search the whole catalog, and arrows navigate without moving its caret.
    click(&mut presenter, "fixture-model-19");
    await_screen(&mut presenter, "Search models");
    presenter
        .input(Event::Paste("fixture-model-1".into()))
        .unwrap();
    await_screen(&mut presenter, "› fixture-model-10");
    for at in 11..20 {
        presenter
            .input(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
            .unwrap();
        await_screen(&mut presenter, &format!("› fixture-model-{at}"));
    }
    await_screen(&mut presenter, "› fixture-model-19");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while screen(&draw(&mut presenter)).contains("[ Close ]") {
        assert!(
            std::time::Instant::now() < deadline,
            "native Enter selection did not settle"
        );
        std::thread::yield_now();
    }
    // Defaults are a separate ordinary component. A catalog-only race must
    // reject its frozen confirmation, without replacing model 19 or the draft.
    click(&mut presenter, "fixture-model-19");
    await_screen(&mut presenter, "[ Default model ]");
    click(&mut presenter, "[ Default model ]");
    await_screen(&mut presenter, "[ Set selected as default ]");
    click(&mut presenter, "[ Set selected as default ]");
    await_screen(&mut presenter, "Use this model as the default?");
    let catalog = runtime.block_on(fixture.config.catalog()).unwrap();
    runtime
        .block_on(
            fixture.config.set_default_target(
                serde_json::from_value(
                    json!({"expectedCatalogRevision":catalog.revision,"target":null}),
                )
                .unwrap(),
            ),
        )
        .unwrap();
    click(&mut presenter, "[ Confirm ]");
    await_screen(&mut presenter, "Saved settings changed.");
    click(&mut presenter, "[ Refresh models ]");
    await_screen(&mut presenter, "[ Set selected as default ]");
    click(&mut presenter, "[ Set selected as default ]");
    click(&mut presenter, "[ Confirm ]");
    await_screen(&mut presenter, "Default model saved.");
    assert_eq!(
        runtime
            .block_on(fixture.config.catalog())
            .unwrap()
            .default_target
            .unwrap()
            .model_id,
        "fixture-model-19"
    );
    await_screen(&mut presenter, "[ Clear default ]");
    click(&mut presenter, "[ Clear default ]");
    await_screen(&mut presenter, "Clear this saved default?");
    click(&mut presenter, "[ Cancel ]");
    assert!(
        runtime
            .block_on(fixture.config.catalog())
            .unwrap()
            .default_target
            .is_some()
    );
    await_screen(&mut presenter, "[ Clear default ]");
    click(&mut presenter, "[ Clear default ]");
    click(&mut presenter, "[ Confirm ]");
    await_screen(&mut presenter, "Default model cleared.");
    assert!(
        runtime
            .block_on(fixture.config.catalog())
            .unwrap()
            .default_target
            .is_none()
    );
    click(&mut presenter, "[ Models ]");
    click(&mut presenter, "[ Close ]");
    assert!(screen(&draw(&mut presenter)).contains("Hello product"));
    await_screen(&mut presenter, "[ Send ]");
    click(&mut presenter, "Hello product");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    // The actual canonical user row must appear through guarded native follow
    // while the provider is still waiting; neither Latest nor a replayed action
    // is injected to expose it.
    await_screen(&mut presenter, "[ Read message source ]");
    assert!(screen(&draw(&mut presenter)).contains("Hello product"));
    respond.send(()).unwrap();
    await_screen(&mut presenter, "Elegant response");
    await_screen(&mut presenter, "[ Read message source ]");
    // The bounded tail may contain only the assistant beside empty canonical
    // boundaries; Earlier retrieves the user through the real sparse-row cursor.
    await_screen(&mut presenter, "[ Earlier messages ]");
    click(&mut presenter, "[ Earlier messages ]");
    await_screen(&mut presenter, "Hello product");
    runtime.block_on(server).unwrap();
    runtime.block_on(fixture.drained());
    presenter
        .surface_closed(&presenter.surface(), Ok(()))
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !presenter.can_shutdown() {
        presenter.pump().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    presenter.shutdown().unwrap();
    drop(presenter);
    // Preserve the original automatic restore assertion before publishing any
    // newer fixture Session that can legitimately become the catalog's latest.
    let mut restored = start();
    await_screen(&mut restored, "Elegant response");
    restored
        .surface_closed(&restored.surface(), Ok(()))
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !restored.can_shutdown() {
        restored.pump().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    restored.shutdown().unwrap();
    drop(restored);
    // A legal imported conversation supplies immutable tool fields through the
    // same original Host; the UI still discovers it using ordinary catalog reads.
    let tool_session = runtime.block_on(async {
        use maka_event_log::sessions::PluginSession;
        use maka_plugins::{composition::Scope as StorageScope, storage::Namespace};
        use maka_runtime::import::{Content, Record, Source};
        let mut configuration = fixture
            .store
            .scoped_sessions::<SessionConfiguration>(
                maka_event_log::sessions::CatalogScope::IdPrefix(fixture.owner.prefix.clone()),
                None,
                maka_event_log::sessions::SessionPosition::Start,
                false,
            )
            .await
            .unwrap()
            .sessions
            .remove(0)
            .configuration;
        configuration.title = "Native tool inspection".into();
        let session = format!("{}{}", fixture.owner.prefix, Uuid::new_v4().simple());
        fixture
            .store
            .begin_session_import(
                &PluginSession {
                    session_id: session.clone(),
                    creator: Namespace::new("native.tool.fixture", StorageScope::Profile).unwrap(),
                    fingerprint: "native-tool-import".into(),
                    managed: false,
                    authority_session_id: None,
                },
                &Source {
                    adapter: "fixture".into(),
                    session_id: "tools".into(),
                },
                &configuration,
                1,
            )
            .await
            .unwrap();
        let mut values = vec![1; 6000];
        values.push(99999);
        let value = json!({"count":42,"values":values});
        let mut result_values = vec![2; 6000];
        result_values.push(99998);
        let result = json!({"count":43,"values":result_values});
        let records = [
            Content::User {
                text: "Inspect original tool fields".into(),
            },
            Content::ToolCall {
                call_id: "call".into(),
                name: "Count".into(),
                input: Some(value.clone()),
            },
            Content::ToolResult {
                call_id: "call".into(),
                output: result,
                is_error: false,
            },
        ]
        .into_iter()
        .enumerate()
        .map(|(i, content)| Record {
            source_message_id: format!("m{i}"),
            source_turn_id: "turn".into(),
            timestamp: Some(1),
            content,
        })
        .collect();
        fixture
            .store
            .append_session_import(&session, 0, records)
            .await
            .unwrap();
        fixture
            .store
            .publish_session_import(&session, 3)
            .await
            .unwrap();
        session
    });
    let mut reopened = start();
    await_screen(&mut reopened, "[ Show sidebar ]");
    click(&mut reopened, "[ Show sidebar ]");
    // Wait for the enabled native action after catalog recovery, not just its
    // title in the loading frame.
    await_screen(&mut reopened, "[ Native tool inspection ]");
    click(&mut reopened, "[ Native tool inspection ]");
    click(&mut reopened, "[ Hide sidebar ]");
    await_screen(&mut reopened, "Tool output · Count");
    await_screen(&mut reopened, "[ Show details ]");
    for (nth, heading, beginning, ending) in [
        (0, "Tool arguments", "\"count\":42", "99999"),
        (1, "Tool result", "\"count\":43", "99998"),
    ] {
        click_nth(&mut reopened, "[ Show details ]", nth);
        let action = format!("[ {heading} ]");
        // Expansion can move the next card's controls outside this 80x24
        // viewport. Reveal them through the original native scroll command.
        if !screen(&draw(&mut reopened)).contains(&action) {
            await_screen(&mut reopened, "[ Scroll to end ]");
            click(&mut reopened, "[ Scroll to end ]");
        }
        await_screen(&mut reopened, &action);
        click(&mut reopened, &action);
        await_screen(&mut reopened, beginning);
        let source_screen = screen(&draw(&mut reopened));
        assert!(source_screen.contains(heading));
        assert!(source_screen.contains("Beginning of source"));
        click(&mut reopened, "[ End ]");
        await_screen(&mut reopened, ending);
        // End lands the native reading viewport at the final literal bytes;
        // the heading above the long excerpt is outside that viewport.
        assert!(!screen(&draw(&mut reopened)).contains("[ Next excerpt ]"));
        click(&mut reopened, "[ Beginning ]");
        await_screen(&mut reopened, beginning);
        click(&mut reopened, "[ Back to conversation ]");
        await_screen(&mut reopened, "[ Hide details ]");
        click(&mut reopened, "[ Hide details ]");
        await_screen(&mut reopened, "[ Show details ]");
    }
    assert!(
        runtime
            .block_on(fixture.owner.session(&tool_session))
            .is_ok()
    );
    click(&mut reopened, "[ Show sidebar ]");
    await_screen(&mut reopened, "[ New conversation ]");
    // Both enabled creation action and original catalog row have this label;
    // the plain header is not an action and must not affect the occurrence.
    click_nth(&mut reopened, "[ New conversation ]", 1);
    click(&mut reopened, "[ Hide sidebar ]");
    assert!(screen(&draw(&mut reopened)).contains("What would you like to work on?"));
    click(&mut reopened, "[ Connections ]");
    await_screen(&mut reopened, "[ API key ]");
    click(&mut reopened, "[ API key ]");
    await_screen(&mut reopened, "An API key is saved.");
    click(&mut reopened, "Enter an API key");
    const NEW_DRAFT: &str = "retained-new-native-key-draft";
    reopened.input(Event::Paste(NEW_DRAFT.into())).unwrap();
    click(&mut reopened, "Earlier key operations");
    await_screen(&mut reopened, "[ Check result ]");
    click_nth(&mut reopened, "[ Check result ]", 1);
    await_screen(&mut reopened, "Key saved for fixture");
    assert!(!screen(&draw(&mut reopened)).contains(NEW_DRAFT));
    click(&mut reopened, "[ Back ]");
    // The original secret remained live under the history projection. Its
    // physical native input still has content, so leaving requires discard.
    click(&mut reopened, "[ Back ]");
    await_screen(&mut reopened, "Discard the unsaved key and leave?");
    click(&mut reopened, "[ Keep editing ]");
    await_screen(&mut reopened, "[ Save key ]");
    assert!(!screen(&draw(&mut reopened)).contains(NEW_DRAFT));
    // Remove through the same ordinary form and physical TUI controls. A
    // changed saved version requires review and another explicit confirmation.
    click(&mut reopened, "[ Back ]");
    await_screen(&mut reopened, "Discard the unsaved key and leave?");
    click(&mut reopened, "[ Discard and leave ]");
    await_screen(&mut reopened, "[ Edit connection ]");
    let before_remove = runtime.block_on(fixture.config.catalog()).unwrap();
    let removed_id = before_remove.connections[0].connection_id.clone();
    assert!(matches!(
        runtime
            .block_on(
                fixture.config.set_default_target(
                    serde_json::from_value(json!({
                        "expectedCatalogRevision": before_remove.revision,
                        "target": {"connectionId": removed_id, "modelId":"fixture-model-19"}
                    }))
                    .unwrap()
                )
            )
            .unwrap(),
        CatalogMutationResult::Committed { .. }
    ));
    click(&mut reopened, "[ Edit connection ]");
    await_screen(&mut reopened, "[ Remove connection ]");
    click(&mut reopened, "[ Remove connection ]");
    await_screen(&mut reopened, "Remove this saved connection");
    runtime.block_on(super::selection::edit(&fixture, |changes| {
        changes.name = Some("Remote removal target".into());
    }));
    click(&mut reopened, "[ Remove connection ]");
    await_screen(&mut reopened, "The saved connection changed.");
    assert!(
        runtime
            .block_on(fixture.config.credential_secret(&locator, None))
            .unwrap()
            .is_some()
    );
    click(&mut reopened, "Review current connection");
    await_screen(&mut reopened, "[ Use this connection ]");
    click(&mut reopened, "[ Use this connection ]");
    click(&mut reopened, "[ Remove connection ]");
    await_screen(&mut reopened, "Remove this saved connection");
    click(&mut reopened, "[ Keep editing ]");
    assert!(
        runtime
            .block_on(fixture.config.catalog())
            .unwrap()
            .connections
            .iter()
            .any(|row| row.connection_id == removed_id)
    );
    click(&mut reopened, "[ Remove connection ]");
    await_screen(&mut reopened, "Remove this saved connection");
    click(&mut reopened, "[ Remove connection ]");
    await_screen(&mut reopened, "[ Refresh connections ]");
    let after_remove = runtime.block_on(fixture.config.catalog()).unwrap();
    assert!(
        !after_remove
            .connections
            .iter()
            .any(|row| row.connection_id == removed_id)
    );
    assert!(after_remove.default_target.is_none());
    // Secret access correctly rejects a removed connection basis. Inspect the
    // actual vault row to prove cleanup rather than treating denial as absence.
    let vault = rusqlite::Connection::open_with_flags(
        fixture.root.join("configuration-rust.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let credential_rows: i64 = vault
        .query_row(
            "SELECT count(*) FROM credentials WHERE json_extract(locator, '$.connectionId') = ?",
            [&removed_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(credential_rows, 0);
    drop(vault);
    click(&mut reopened, "[ Back ]");
    await_screen(&mut reopened, "Elegant response");
    for (root, database, sql) in [
        (
            &fixture.root,
            "runtime-rust.sqlite",
            "SELECT request_json || outcome_json || cleanup_json FROM application_operations",
        ),
        (
            &fixture._dir.path().join("ui"),
            "runtime-rust.sqlite",
            "SELECT reference_json FROM application_intents",
        ),
        (
            &fixture.root,
            "configuration-rust.sqlite",
            "SELECT target || request_fingerprint || phase FROM oauth_login_receipts",
        ),
    ] {
        let db = rusqlite::Connection::open_with_flags(
            root.join(database),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let mut statement = db.prepare(sql).unwrap();
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap();
        let mut count = 0;
        for row in rows {
            count += 1;
            let row = row.unwrap();
            assert!(
                !row.contains(KEY) && !row.contains(NEW_DRAFT),
                "native secret entered public operation history"
            );
        }
        assert!(count > 0, "actual journal must contain persisted facts");
    }
    assert!(
        !runtime
            .block_on(fixture.config.catalog())
            .unwrap()
            .connections
            .is_empty()
    );
    studio::switch(
        &runtime,
        &caller,
        &mut reopened,
        &directory,
        &registrations,
        &manager_callers,
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !reopened.can_shutdown() {
        reopened.pump().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    reopened.shutdown().unwrap();
    drop(reopened);
    runtime.block_on(async {
        let (physical, host_wire) = tokio::join!(connection.close(), wire_server.close());
        physical.unwrap();
        host_wire.unwrap();
    });
    runtime.block_on(async {
        manager_connection.close().await.unwrap();
        manager_server.close().await.unwrap();
    });
    drop(connection);
    drop(wire_server);
    drop(manager_connection);
    drop(manager_server);
    drop(registrations);
    runtime.block_on(manager_app.shutdown()).unwrap();
    drop(manager_app);
    runtime.block_on(fixture.app.shutdown()).unwrap();
    runtime.block_on(fixture.config.shutdown()).unwrap();
    runtime.block_on(caller.shutdown()).unwrap();
}
