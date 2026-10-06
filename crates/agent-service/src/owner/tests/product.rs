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
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use maka_plugin_core::deployment::RemoteObservation;
use maka_ui::actions::RemoteService;
use ratatui::{buffer::Buffer, layout::Rect};
use std::{path::PathBuf, process::Command};

struct Physical {
    server: Arc<maka_application_bridge::transport::Server>,
    peer: maka_application_bridge::transport::Peer,
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
    presenter.pump().unwrap();
    let area = Rect::new(0, 0, 80, 24);
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
    let buffer = draw(presenter);
    let (x, y) = (0..buffer.area.height)
        .filter_map(|y| {
            let row = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>();
            row.find(label).map(|x| (x as u16, y))
        })
        .nth(nth)
        .unwrap_or_else(|| panic!("missing {label} #{nth} in {}", screen(&buffer)));
    presenter
        .input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
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
    let fixture = runtime.block_on(Fixture::with_peer(&base, 1, None, Some(peer.clone())));
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
        "reader",
        "history",
    ];
    let packages = names
        .iter()
        .map(|name| Package::read_from(&directory.join(name)).unwrap())
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
    let script = r#"const {composition,packageNames}=await import(process.argv[1]);const refs=JSON.parse(process.argv[2]);const keys=Object.fromEntries(packageNames.map((name,i)=>[name,'00000000-0000-0000-0000-'+String(i+20).padStart(12,'0')]));keys.agent='00000000-0000-0000-0000-000000000001';keys.receipts='00000000-0000-0000-0000-000000000004';keys.settings='00000000-0000-0000-0000-000000000005';for(const name of packageNames)refs[name]??={id:'unused.'+name,revision:'sha256-'+ 'a'.repeat(64)};const result=composition({presentation:'tui',locale:'en',scheme:'light',packages:refs,keys,agent:{maxActiveRuns:1,maxSteps:1},hostRoot:'/fixture',hostDeployment:'00000000-0000-0000-0000-000000000080'});console.log(JSON.stringify({input:result.ui,observations:result.remoteExports}));"#;
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
        Endpoint, Peer, Server,
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
    let connection = Connection::new(Arc::new(Physical {
        server: wire_server.clone(),
        peer: Peer::new(deployment.clone(), consumers).unwrap(),
    }));
    let registrations: Vec<_> = observations
        .iter()
        .enumerate()
        .map(|(index, observation)| {
            let selected = wire_server
                .catalogue()
                .select(&format!("service-{index}"))
                .unwrap();
            let description = selected.description();
            RemoteService {
                target: observation.capability.clone(),
                registration_epoch: description.registration_epoch,
                contract: description.contract.clone(),
                implementation: Arc::new(
                    Service::new(connection.clone(), selected, deployment.clone()).unwrap(),
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
    click(&mut presenter, "[ Show sidebar ]");
    await_screen(&mut presenter, "[ New conversation ]");
    click(&mut presenter, "[ New conversation ]");
    click(&mut presenter, "[ Hide sidebar ]");
    await_screen(&mut presenter, "A place to think.");
    // Native draft is the first editor hit. Focus it through its actual visible
    // placeholder, never inject component or forged action-result events.
    click(&mut presenter, "What would you like to work on?");
    presenter
        .input(Event::Paste("Hello product".into()))
        .unwrap();
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
    assert!(screen(&draw(&mut presenter)).contains("Hello product"));
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
    click(&mut presenter, "[ Send ]");
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
    presenter.surface_closed(Ok(())).unwrap();
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
    restored.surface_closed(Ok(())).unwrap();
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
                &json!({"schemaVersion":1,"title":"Native tool inspection"}),
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
    // A disabled catalog entry is currently painted as plain text. Its title
    // alone does not prove the actual native action is enabled after recovery.
    await_screen(&mut reopened, "[ Native tool inspection ]");
    click(&mut reopened, "[ Native tool inspection ]");
    click(&mut reopened, "[ Hide sidebar ]");
    await_screen(&mut reopened, "Tool output · Count");
    await_screen(&mut reopened, "[ Tool details ]");
    for (nth, heading, beginning, ending) in [
        (0, "Tool arguments", "\"count\":42", "99999"),
        (1, "Tool result", "\"count\":43", "99998"),
    ] {
        click_nth(&mut reopened, "[ Tool details ]", nth);
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
        await_screen(&mut reopened, "[ Tool details ]");
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
    reopened.surface_closed(Ok(())).unwrap();
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
    drop(connection);
    drop(wire_server);
    drop(registrations);
    runtime.block_on(fixture.app.shutdown()).unwrap();
    runtime.block_on(fixture.config.shutdown()).unwrap();
    runtime.block_on(caller.shutdown()).unwrap();
}
