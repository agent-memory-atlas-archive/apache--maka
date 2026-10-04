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
use maka_ui::actions::{RemoteService, ServiceSource};
use ratatui::{buffer::Buffer, layout::Rect};
use std::{path::PathBuf, process::Command};

struct Scopes(Issuer);
impl maka_application_bridge::Scopes for Scopes {
    fn scope(&self, _: &ServiceSource) -> Result<Scope, String> {
        Ok(scope(&self.0, true))
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
    let server = runtime.spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let body = request_with_key(&mut socket, KEY).await;
        assert_eq!(body["model"], "fixture-model-19");
        assert!(body.to_string().contains("Hello product"));
        response(&mut socket).await;
    });
    let fixture = runtime.block_on(Fixture::new(&base, 1, None));
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
        "composer",
        "models",
        "connections",
        "credential",
        "input",
        "navigation",
        "timeline",
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
    let deployment = runtime
        .block_on(caller.commit_application(
            Uuid::new_v4(),
            0,
            input,
            observations.clone(),
            packages,
        ))
        .unwrap();
    let registration = |observation: &RemoteObservation, binding: Binding| RemoteService {
        target: observation.capability.clone(),
        registration_epoch: binding.registration_epoch(),
        contract: binding.contract().clone(),
        implementation: Arc::new(maka_application_bridge::LocalService::new(
            fixture.app.client(),
            binding,
            Arc::new(Scopes(fixture.issuer.clone())),
        )),
    };
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
    let start = || {
        runtime
            .block_on(maka_tui::application::Presenter::start_from_store(
                caller.clone(),
                deployment.clone(),
                runtime.handle().clone(),
                maka_tui::application::registry(),
                json!({}),
                observations
                    .iter()
                    .zip(bindings.iter())
                    .map(|(observation, binding)| registration(observation, binding.clone()))
                    .collect(),
            ))
            .unwrap()
    };
    let mut presenter = start();
    await_screen(&mut presenter, "[ New conversation ]");
    click(&mut presenter, "New conversation");
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
    await_screen(&mut presenter, "[ Send ]");
    click(&mut presenter, "[ Send ]");
    await_screen(&mut presenter, "Elegant response");
    assert!(screen(&draw(&mut presenter)).contains("Hello product"));
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
    let mut reopened = start();
    await_screen(&mut reopened, "Elegant response");
    await_screen(&mut reopened, "[ New conversation ]");
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
    runtime.block_on(fixture.app.shutdown()).unwrap();
    runtime.block_on(fixture.config.shutdown()).unwrap();
    runtime.block_on(caller.shutdown()).unwrap();
}
