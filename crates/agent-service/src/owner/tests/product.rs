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

#[path = "product/message_details.rs"]
mod message_details;
#[path = "product/studio.rs"]
mod studio;
#[cfg(target_os = "macos")]
#[path = "product/terminal.rs"]
mod terminal;
#[cfg(target_os = "macos")]
#[path = "product/wire_loss.rs"]
mod wire_loss;

use super::*;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use maka_plugin_core::deployment::RemoteObservation;
use maka_ui::actions::RemoteService;
use ratatui::{buffer::Buffer, layout::Rect, style::Color};
use std::{path::PathBuf, process::Command, sync::OnceLock};

struct Physical {
    #[cfg(target_os = "macos")]
    loss: Option<Arc<wire_loss::Loss>>,
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
        #[cfg(target_os = "macos")]
        let loss = self.loss.clone();
        Box::pin(async move {
            if stop.is_cancelled() {
                return Err("physical opening cancelled before admission".into());
            }
            let (channel, accepted) = tokio::io::duplex(64 * 1024);
            let (reader, writer) = tokio::io::split(accepted);
            drop(server.serve(reader, writer, peer)?);
            #[cfg(target_os = "macos")]
            let channel = match loss {
                Some(loss) => loss.channel(channel),
                None => channel,
            };
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
#[track_caller]
fn click(presenter: &mut maka_tui::application::Presenter, label: &str) {
    click_nth(presenter, label, 0);
}
#[track_caller]
fn assert_file_copy(
    presenter: &mut maka_tui::application::Presenter,
    label: &str,
    path: &std::path::Path,
) {
    use base64::Engine as _;
    click(presenter, label);
    let mut copied = Vec::new();
    assert!(presenter.flush_clipboard(&mut copied).unwrap());
    assert_eq!(
        copied,
        format!(
            "\x1b]52;c;{}\x07",
            base64::engine::general_purpose::STANDARD.encode(path.to_str().unwrap())
        )
        .as_bytes()
    );
    assert!(!presenter.flush_clipboard(&mut Vec::new()).unwrap());
}
#[track_caller]
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
#[track_caller]
fn wait_for_target(
    presenter: &mut maka_tui::application::Presenter,
    label: &str,
    nth: usize,
) -> (u16, u16) {
    wait_for_target_at(presenter, label, nth, Rect::new(0, 0, 80, 24))
}
#[track_caller]
fn wait_for_target_at(
    presenter: &mut maka_tui::application::Presenter,
    label: &str,
    nth: usize,
    area: Rect,
) -> (u16, u16) {
    // Disabled buttons retain their label. This fixture uses the ordinary light
    // theme, whose enabled button foreground is its accent. Await that painted
    // state before one click; never replay a potentially committed operation.
    static COLORS: OnceLock<[Color; 3]> = OnceLock::new();
    let directory_row = label.starts_with("    ");
    let colors = (label.starts_with("[ ") || directory_row).then(|| {
        *COLORS.get_or_init(|| {
            let module = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../packages/agent-product/dist/packages/appearance/tui.js");
            let output = Command::new("node")
                .args(["--input-type=module", "-e", "const {activate}=await import(process.argv[1]); const theme=activate({scheme:'light'}).Appearance.value; console.log(JSON.stringify({accent:theme.accent>>>8,muted:theme.muted>>>8,text:theme.text>>>8}));"])
                .arg(format!("file://{}", module.display()))
                .output()
                .unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            let colors: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_ne!(colors["accent"], colors["muted"], "fixture must distinguish enabled and disabled buttons");
            ["accent", "muted", "text"].map(|key| {
                let rgb = colors[key].as_u64().unwrap() as u32;
                Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
            })
        })
    });
    let enabled = label.starts_with("[ ").then(|| colors.unwrap()[0]);
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let (x, y) = loop {
        let buffer = draw_at(presenter, area);
        let position = (0..buffer.area.height)
            .filter_map(|y| {
                let row = (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>();
                let byte = row.find(label)?;
                // Unicode previews precede trailing controls: byte offsets are
                // not terminal columns. Resolve against the painted cells.
                let mut offset = 0;
                (0..buffer.area.width)
                    .find(|x| {
                        let start = offset;
                        offset += buffer[(*x, y)].symbol().len();
                        start == byte
                    })
                    // An inline pane puts the main title beside its Hide
                    // button. Blank padding before that title is not the
                    // painted four-space prefix of an actual directory row.
                    .filter(|x| {
                        !directory_row || colors.unwrap()[1..].contains(&buffer[(*x, y)].fg)
                    })
                    .map(|x| (x, y))
            })
            .nth(nth);
        if let Some((x, y)) = position
            && enabled.is_none_or(|color| buffer[(x, y)].fg == color)
            && (!(label.starts_with("    ")
                || label.starts_with(['▸', '▾'])
                || matches!(
                    label,
                    "All" | "Needs You" | "Deny this call" | "Allow this call"
                ))
                || !buffer[(x, y)]
                    .modifier
                    .contains(ratatui::style::Modifier::DIM))
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
#[track_caller]
fn await_sidebar_closed(presenter: &mut maka_tui::application::Presenter) {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let rendered = screen(&draw(presenter));
        if !rendered.contains("[ Hide sidebar ]") {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "sidebar did not close: {rendered}"
        );
        std::thread::yield_now();
    }
}
#[track_caller]
fn await_models_closed(presenter: &mut maka_tui::application::Presenter) {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let rendered = screen(&draw(presenter));
        if !rendered.contains("[ Close ]") {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "model confirmation did not settle: {rendered}"
        );
        std::thread::yield_now();
    }
}
// Requires the normal Node build dependencies; uses the real packaged guest,
// original native editor, persisted caller intents and native authenticated run.
#[test]
#[ignore = "opt-in product authoring build and actual HTTP provider"]
fn packaged_product_creates_reads_sends_and_restores_through_actual_native_owners() {
    packaged_product(ProductCase::Full);
}

#[test]
#[ignore = "opt-in product authoring build and actual HTTP provider"]
fn packaged_metadata_recovers_a_real_native_initialization_conflict() {
    packaged_product(ProductCase::MetadataConflict);
}

#[test]
#[ignore = "opt-in product authoring build and actual HTTP provider"]
fn packaged_file_permission_uses_native_policy_controls_and_explicit_response() {
    packaged_product(ProductCase::FilePermissions);
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "opt-in product authoring build and actual native sandbox"]
fn packaged_process_permission_uses_original_native_approval_before_shell_effect() {
    packaged_product(ProductCase::ProcessPermissions);
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "opt-in product authoring build and actual dropped physical terminal reply"]
fn packaged_terminal_lost_reply_preserves_original_receipt_and_cleanup_fence() {
    packaged_product(ProductCase::TerminalLoss);
}

#[test]
#[ignore = "opt-in product authoring build and actual image-producing service"]
fn packaged_workspace_image_opens_original_viewer_and_preserves_native_draft() {
    packaged_product(ProductCase::Image);
}

#[test]
#[ignore = "opt-in product authoring build and actual native form editors"]
fn packaged_form_uses_original_native_editors_and_canonical_response() {
    packaged_product(ProductCase::Form);
}
#[derive(Clone, Copy)]
enum ProductCase {
    Form,
    Full,
    MetadataConflict,
    FilePermissions,
    #[cfg(target_os = "macos")]
    ProcessPermissions,
    #[cfg(target_os = "macos")]
    TerminalLoss,
    Image,
}
fn packaged_product(case: ProductCase) {
    let fail_labels_initialization = matches!(case, ProductCase::MetadataConflict);
    #[cfg(target_os = "macos")]
    let process_permissions = matches!(
        case,
        ProductCase::ProcessPermissions | ProductCase::TerminalLoss
    );
    #[cfg(not(target_os = "macos"))]
    let process_permissions = false;
    #[cfg(target_os = "macos")]
    let terminal_loss = matches!(case, ProductCase::TerminalLoss);
    #[cfg(target_os = "macos")]
    let loss = terminal_loss.then(|| Arc::new(wire_loss::Loss::default()));
    let file_permissions = matches!(case, ProductCase::FilePermissions) || process_permissions;
    let image = matches!(case, ProductCase::Image);
    let form = matches!(case, ProductCase::Form);
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
    let (respond, response_ready) = tokio::sync::oneshot::channel::<PathBuf>();
    let mut respond = Some(respond);
    let mut server = runtime.spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        if form {
            request_with_key(&mut socket, "private-fixture-key").await;
            response_ready.await.unwrap();
            super::interactions::form_response(&mut socket).await;
            let (mut next, _) = listener.accept().await.unwrap();
            let resumed = request_with_key(&mut next, "private-fixture-key").await;
            let result = resumed["messages"].as_array().unwrap().iter().find(|message| message["role"] == "tool").unwrap();
            assert_eq!(serde_json::from_str::<Value>(result["content"].as_str().unwrap()).unwrap(), json!({"kind":"answered","result":{"action":"accept","values":{"note":"Native delivery","count":2,"confirm":false,"format":"brief"}}}));
            response(&mut next).await;
            return;
        }
        if image {
            request_with_key(&mut socket, "private-fixture-key").await;
            response_ready.await.unwrap();
            super::files::tool_response(&mut socket, json!({"path":"native-image.gif"})).await;
            let (mut next, _) = listener.accept().await.unwrap();
            let resumed = request_with_key(&mut next, "private-fixture-key").await;
            assert_eq!(super::images::model_images(&resumed), vec![super::images::gif(0)]);
            response(&mut next).await;
            return;
        }
        if file_permissions {
            request_with_key(&mut socket, "private-fixture-key").await;
            let workspace = response_ready.await.unwrap();
            if process_permissions {
                let input = json!({"command":"printf 'native file contents' > native-approved.txt","login":false,"timeout_ms":5000,
                    "additional_permissions":{"filesystem":[{"path":workspace.join("native-approved.txt"),"scope":"exact","access":"write"}],"network":"denied"},
                    "justification":"Save the command output in the selected workspace."});
                let delta = json!({"id":"process-model","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"process-call","type":"function","function":{"name":"Execute","arguments":input.to_string()}}]},"finish_reason":null}]});
                let body = format!("data: {delta}\n\ndata: {{\"id\":\"process-model\",\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"tool_calls\"}}]}}\n\ndata: [DONE]\n\n");
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            } else {
                super::files::tool_response(&mut socket, json!({"path":"native-approved.txt","content":"native file contents"})).await;
            }
            let (mut next, _) = listener.accept().await.unwrap();
            let resumed = request_with_key(&mut next, "private-fixture-key").await;
            let tool = resumed["messages"].as_array().unwrap().iter().rev().find(|m| m["role"] == "tool").unwrap();
            assert_eq!(serde_json::from_str::<Value>(tool["content"].as_str().unwrap()).unwrap()["kind"], "completed");
            response_with_thinking(&mut next, "**Elegant response**", Some("**Reasoned decision** 中😀")).await;
            return;
        }
        let body = request_with_key(&mut socket, KEY).await;
        assert_eq!(body["model"], "fixture-model-19");
        assert_eq!(
            body["reasoning_effort"], "none",
            "explicit UI Off must override configured High"
        );
        assert!(body.to_string().contains("Hello product"));
        response_ready.await.unwrap();
        super::interactions::question_response(&mut socket).await;
        let (mut next, _) = listener.accept().await.unwrap();
        let resumed = request_with_key(&mut next, KEY).await;
        let result = resumed["messages"].as_array().unwrap().iter().find(|message| message["role"] == "tool").unwrap();
        assert_eq!(serde_json::from_str::<Value>(result["content"].as_str().unwrap()).unwrap(), json!({"kind":"answered","answers":[
            {"question":"Choose\nformat","answer":"Brief"}, {"question":"Any detail?","answer":"native answer"}, {"question":"Optional?","answer":null}
        ]}));
        response(&mut next).await;
    });
    let tooling = if form {
        Some(super::service_tools::Setup::forms())
    } else if image {
        Some(super::service_tools::Setup::files("readImage"))
    } else if process_permissions {
        Some(super::service_tools::Setup::processes())
    } else if file_permissions {
        Some(super::service_tools::Setup::files("write").titled(
            maka_runtime::display::Text::localized(
                "Save workspace file",
                "保存工作区文件",
                "儲存工作區檔案",
            ),
        ))
    } else {
        (!fail_labels_initialization)
            .then(|| super::service_tools::Setup::questions().require_confirmation())
    };
    let peer = tooling.as_ref().map_or_else(
        || Arc::new(maka_application_bridge::transport::PeerPolicy::default()),
        super::service_tools::Setup::policy,
    );
    let mut fixture = runtime.block_on(Fixture::with_services(
        &base,
        1,
        None,
        Some(peer.clone()),
        tooling,
    ));
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
        if image {
            changes.model_overrides = Patch::Set(BTreeMap::from([(
                "fixture-model-00".into(),
                ModelOverride {
                    vision: Some(true),
                    ..Default::default()
                },
            )]));
        }
    }));
    if image {
        std::fs::write(
            fixture.workspace.join("native-image.gif"),
            super::images::gif(0),
        )
        .unwrap();
    }
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
        "recovery",
        "command-palette",
        "directory-picker",
        "session-details",
        "queue-editor",
        "question-editor",
        "form-editor",
        "service-approval",
        "permission-approval",
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
        "tool-details",
        "diff",
        "reading-area",
        "rows",
        "source-text",
        "markdown",
        "terminal-view",
        "terminal-panel",
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
    let script = r#"const {composition,packageNames}=await import(process.argv[1]);const refs=JSON.parse(process.argv[2]);const keys=Object.fromEntries(packageNames.map((name,i)=>[name,'00000000-0000-0000-0000-'+String(i+20).padStart(12,'0')]));keys.agent='00000000-0000-0000-0000-000000000001';keys.receipts='00000000-0000-0000-0000-000000000004';keys.settings='00000000-0000-0000-0000-000000000005';keys['workspace-processes']='00000000-0000-0000-0000-000000000021';for(const name of packageNames)refs[name]??={id:'unused.'+name,revision:'sha256-'+ 'a'.repeat(64)};const result=composition({presentation:'tui',locale:'en',scheme:'light',packages:refs,keys,...(process.argv[3]==='true'?{processes:{filesystem:{default:'write',rules:[],denyGlobs:[]},network:'allowed',helper:null}}:{}),agent:{workspaceRoots:[{label:"Fixture",path:"/fixture-workspace"}],maxActiveRuns:1, permissions: { access: 'workspace_write', requestPermissions: true },maxSteps:1,tools:[]},hostRoot:'/fixture',hostDeployment:'00000000-0000-0000-0000-000000000080',managementDeployment:'00000000-0000-0000-0000-000000000081'});console.log(JSON.stringify({input:result.ui,observations:result.remoteExports,managementHost:result.managementHost,managementConfiguration:result.connections.hosts.find(h=>h.name==='manager')}));"#;
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
        .arg(process_permissions.to_string())
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
    let mut bindings = vec![
        runtime.block_on(fixture.binding(2)),
        receipt_binding,
        settings_binding,
    ];
    if process_permissions {
        bindings.push(
            runtime
                .block_on(fixture.app.client().bind(
                    fixture.app.client().activation(&id(2)).unwrap(),
                    "terminal".into(),
                ))
                .unwrap()
                .remove(0),
        );
    }
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
                        recovery: index == 1 || index == 3,
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
        #[cfg(target_os = "macos")]
        loss: loss.clone(),
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
        #[cfg(target_os = "macos")]
        loss: None,
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
                        if observation.capability.export == "terminal" {
                            "service-3".into()
                        } else {
                            format!("service-{index}")
                        },
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
    await_screen(&mut presenter, "[ Toggle sidebar ]");
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
    // Selection and Escape leave the explicitly pinned inline sidebar open.
    // Close it through the same real shortcut before the full-width workflow.
    wait_for_target(&mut presenter, "[ Hide sidebar ]", 0);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('b'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    await_sidebar_closed(&mut presenter);
    await_screen(&mut presenter, "A place to think.");
    // Closing navigation restores the retained native composer through its
    // declared tab order. This command/draft path needs no mouse or forged input.
    presenter
        .input(Event::Paste("Hello product".into()))
        .unwrap();
    await_screen(&mut presenter, "Hello product");
    // Narrow navigation uses the original modal input scope. Resizing keeps
    // that drawer, Escape returns to the same draft, and its close button is
    // reachable with an actual mouse click at the narrow geometry.
    let narrow_area = Rect::new(0, 0, 40, 16);
    draw_at(&mut presenter, narrow_area);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('b'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    wait_for_target_at(&mut presenter, "[ Hide sidebar ]", 0, narrow_area);
    assert!(screen(&draw(&mut presenter)).contains("[ Hide sidebar ]"));
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "Hello product");
    draw_at(&mut presenter, narrow_area);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('b'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    let (x, y) = wait_for_target_at(&mut presenter, "[ Hide sidebar ]", 0, narrow_area);
    presenter
        .input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        }))
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
    if form {
        click(&mut presenter, "[ Choose model ]");
        await_screen(&mut presenter, "fixture-model-00");
        click(&mut presenter, "fixture-model-00");
        click(&mut presenter, "[ Apply ]");
        await_models_closed(&mut presenter);
        click(&mut presenter, "[ Send ]");
        await_screen(&mut presenter, "What would you like to work on?");
        click(&mut presenter, "What would you like to work on?");
        presenter
            .input(Event::Paste("Form background draft".into()))
            .unwrap();
        respond
            .take()
            .unwrap()
            .send(fixture.workspace.clone())
            .unwrap();
        await_screen(&mut presenter, "[ Ctrl G · Review request ]");
        click(&mut presenter, "[ Ctrl G · Review request ]");
        await_screen(&mut presenter, "Delivery note");
        click_nth(&mut presenter, "Delivery note", 1);
        presenter
            .input(Event::Paste("Native delivery".into()))
            .unwrap();
        await_screen(&mut presenter, "Native delivery");
        presenter
            .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .unwrap();
        await_screen(&mut presenter, "Form background draft");
        click(&mut presenter, "[ Ctrl G · Review request ]");
        await_screen(&mut presenter, "Native delivery");
        click(&mut presenter, "[ 2 ]");
        click_nth(&mut presenter, "Copies", 1);
        presenter.input(Event::Paste("2".into())).unwrap();
        click(&mut presenter, "[ 3 ]");
        click(&mut presenter, "○ No");
        click(&mut presenter, "[ 4 ]");
        click(&mut presenter, "○ Brief");
        click(&mut presenter, "[ Submit form ]");
        await_screen(&mut presenter, "Answers recorded");
        let record = runtime
            .block_on(
                fixture
                    .store
                    .session_projection::<SessionConfiguration>(&session_id),
            )
            .unwrap()
            .unwrap();
        assert!(record.pending_interactions.is_empty());
        click(&mut presenter, "[ Close ]");
        await_screen(&mut presenter, "Form background draft");
        await_screen(&mut presenter, "Elegant response");
        runtime.block_on(&mut server).unwrap();
        runtime.block_on(fixture.drained());
    }
    if image {
        click(&mut presenter, "[ Choose model ]");
        await_screen(&mut presenter, "fixture-model-00");
        click(&mut presenter, "fixture-model-00");
        click(&mut presenter, "[ Apply ]");
        await_models_closed(&mut presenter);
        click(&mut presenter, "[ Send ]");
        await_screen(&mut presenter, "What would you like to work on?");
        click(&mut presenter, "What would you like to work on?");
        presenter
            .input(Event::Paste("Image background draft".into()))
            .unwrap();
        await_screen(&mut presenter, "Image background draft");
        respond
            .take()
            .unwrap()
            .send(fixture.workspace.clone())
            .unwrap();
        await_screen(&mut presenter, "Elegant response");
        runtime.block_on(&mut server).unwrap();
        runtime.block_on(fixture.drained());
        for _ in 0..2 {
            if screen(&draw(&mut presenter)).contains("[ View image ]") {
                break;
            }
            click(&mut presenter, "[ Show details ]");
        }
        click(&mut presenter, "[ View image ]");
        await_screen(
            &mut presenter,
            "Image preview is available in the desktop app.",
        );
        click(&mut presenter, "[ Back to conversation ]");
        await_screen(&mut presenter, "Image background draft");
    }
    if file_permissions {
        click(&mut presenter, "[ Details ]");
        // Native focus reveals policy choices below the original editors in a short viewport.
        for _ in 0..12 {
            if screen(&draw(&mut presenter)).contains("Read only") {
                break;
            }
            presenter
                .input(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
                .unwrap();
            draw(&mut presenter);
        }
        click(&mut presenter, "Read only");
        click(&mut presenter, "[ Save details ]");
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        loop {
            draw(&mut presenter);
            let current = runtime
                .block_on(fixture.owner.session(&session_id))
                .unwrap();
            if current.configuration.permissions.access == WorkspaceAccess::ReadOnly {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "native policy save did not settle"
            );
            std::thread::yield_now();
        }
        presenter
            .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .unwrap();
        await_screen(&mut presenter, "Hello product");
        click(&mut presenter, "[ Choose model ]");
        await_screen(&mut presenter, "fixture-model-00");
        click(&mut presenter, "fixture-model-00");
        click(&mut presenter, "[ Apply ]");
        await_models_closed(&mut presenter);
        click(&mut presenter, "[ Send ]");
        await_screen(&mut presenter, "What would you like to work on?");
        click(&mut presenter, "What would you like to work on?");
        presenter
            .input(Event::Paste("File background draft".into()))
            .unwrap();
        await_screen(&mut presenter, "File background draft");
        respond
            .take()
            .unwrap()
            .send(fixture.workspace.clone())
            .unwrap();
        await_screen(&mut presenter, "[ Ctrl G · Review request ]");
        click(&mut presenter, "[ Ctrl G · Review request ]");
        await_screen(&mut presenter, "native-approved.txt");
        let (x, y) = wait_for_target(&mut presenter, "[ Close ]", 0);
        assert!(
            draw(&mut presenter)[(x, y)]
                .modifier
                .contains(ratatui::style::Modifier::REVERSED)
        );
        assert!(!fixture.workspace.join("native-approved.txt").exists());
        presenter
            .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .unwrap();
        await_screen(&mut presenter, "File background draft");
        if !process_permissions {
            let deadline = std::time::Instant::now() + Duration::from_secs(20);
            loop {
                let rendered = screen(&draw(&mut presenter));
                let lines = rendered.lines().collect::<Vec<_>>();
                if lines.windows(2).any(|rows| {
                    rows[0].contains("Save workspace file")
                        && rows[1].contains("Waiting for your input")
                }) {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "tool-specific title and waiting status were not adjacent: {rendered}"
                );
                std::thread::yield_now();
            }
        }
        click(&mut presenter, "[ Ctrl G · Review request ]");
        await_screen(&mut presenter, "native-approved.txt");
        click(&mut presenter, "Deny this call");
        wait_for_target(&mut presenter, "Deny this call", 0);
        draw(&mut presenter);
        presenter
            .input(Event::Key(KeyEvent::new(
                KeyCode::Right,
                KeyModifiers::NONE,
            )))
            .unwrap();
        let (x, y) = wait_for_target(&mut presenter, "Allow this call", 0);
        assert!(
            draw(&mut presenter)[(x, y)]
                .modifier
                .contains(ratatui::style::Modifier::UNDERLINED)
        );
        presenter
            .input(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();
        click(&mut presenter, "This conversation");
        draw(&mut presenter);
        assert!(!fixture.workspace.join("native-approved.txt").exists());
        let pending = runtime
            .block_on(fixture.store.pending_interactions(&session_id))
            .unwrap();
        assert_eq!(pending.len(), 1);
        click(&mut presenter, "[ Confirm choice ]");
        await_screen(&mut presenter, "Approved access");
        let record = runtime
            .block_on(fixture.store.interaction(&pending[0].request_id))
            .unwrap()
            .unwrap();
        assert!(matches!(
            record.outcome,
            Some(
                maka_runtime::interaction::InteractionOutcome::PermissionsDecision {
                    decision: maka_sandbox::grant::Decision::Allow {
                        scope: maka_sandbox::grant::Scope::Session,
                        ..
                    },
                    ..
                }
            )
        ));
        click(&mut presenter, "[ Close ]");
        await_screen(&mut presenter, "File background draft");
        await_screen(&mut presenter, "Elegant response");
        runtime.block_on(&mut server).unwrap();
        runtime.block_on(fixture.drained());
        let (column, row) = wait_for_target(&mut presenter, "Elegant response", 0);
        let formatted = draw(&mut presenter);
        assert!(
            formatted[(column, row)]
                .modifier
                .contains(ratatui::style::Modifier::BOLD)
        );
        assert!(!screen(&formatted).contains("**Elegant response**"));
        // Read the preceding ordinary window: trailing canonical metadata can
        // leave only the answer in the bounded tail window.
        click(&mut presenter, "[ Earlier messages ]");
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        let formatted_thinking = loop {
            let frame = draw(&mut presenter);
            let bold_thinking = (0..frame.area.height).any(|y| {
                (0..frame.area.width.saturating_sub(17)).any(|x| {
                    "Reasoned decision"
                        .chars()
                        .enumerate()
                        .all(|(i, ch)| frame[(x + i as u16, y)].symbol() == ch.to_string())
                        && frame[(x, y)]
                            .modifier
                            .contains(ratatui::style::Modifier::BOLD)
                })
            });
            if bold_thinking {
                break frame;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "Thinking did not reach its canonical Markdown: {}",
                screen(&frame)
            );
            std::thread::yield_now();
        };
        assert!(!screen(&formatted_thinking).contains("**Reasoned decision**"));
        // The compact card exposes its original file path without hydrating
        // requested content, and retains its details component across disclosure.
        if !process_permissions {
            await_screen(&mut presenter, "Save workspace file");
            assert_file_copy(
                &mut presenter,
                "[ Copy · native-approved.txt ]",
                &fixture.workspace.join("native-approved.txt"),
            );
            assert!(!screen(&draw(&mut presenter)).contains("Requested content"));
            assert!(!screen(&draw(&mut presenter)).contains("native file contents"));
            presenter
                .input(Event::Key(KeyEvent::new(
                    KeyCode::Char('f'),
                    KeyModifiers::CONTROL,
                )))
                .unwrap();
            await_screen(&mut presenter, "Find…");
            presenter
                .input(Event::Paste("native-approved.txt".into()))
                .unwrap();
            await_screen(&mut presenter, "1/1");
            await_screen(&mut presenter, "Requested content");
            presenter
                .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
                .unwrap();
            click(&mut presenter, "[ Hide details ]");
            await_screen(&mut presenter, "[ Copy · native-approved.txt ]");
        }
        // The preceding service result exposes business JSON and the exact
        // original receipt through one retained, fenced source reader.
        click(&mut presenter, "[ Show details ]");
        if !process_permissions {
            await_screen(&mut presenter, "Requested content");
            await_screen(&mut presenter, "native file contents");
            await_screen(&mut presenter, "Recorded original content");
            assert_file_copy(
                &mut presenter,
                "[ Copy file path ]",
                &fixture.workspace.join("native-approved.txt"),
            );
        }
        click(&mut presenter, "[ Tool arguments ]");
        await_screen(&mut presenter, "native-approved.txt");
        click(&mut presenter, "[ Back to conversation ]");
        click(&mut presenter, "[ Tool result ]");
        await_screen(&mut presenter, "completed");
        assert!(!screen(&draw(&mut presenter)).contains("journal_v1"));
        click(&mut presenter, "[ Operation receipt ]");
        await_screen(&mut presenter, "Operation receipt");
        // The receipt can span excerpts; its beginning retains the original
        // reference, while the business result is separately selectable.
        await_screen(&mut presenter, "reference");
        click(&mut presenter, "[ Tool result ]");
        await_screen(&mut presenter, "completed");
        click(&mut presenter, "[ Back to conversation ]");
        click(&mut presenter, "[ Hide details ]");
        click(&mut presenter, "[ Read message source ]");
        await_screen(&mut presenter, "[ Back to conversation ]");
        await_screen(&mut presenter, "**Reasoned decision**");
        assert!(!screen(&draw(&mut presenter)).contains("**Elegant response**"));
        click(&mut presenter, "[ Back to conversation ]");
        await_screen(&mut presenter, "Reasoned decision");
        click(&mut presenter, "[ Latest messages ]");
        await_screen(&mut presenter, "Elegant response");

        // Find uses its original native query, then returns to the retained draft.
        for close_key in [
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        ] {
            presenter
                .input(Event::Key(KeyEvent::new(
                    KeyCode::Char('f'),
                    KeyModifiers::CONTROL,
                )))
                .unwrap();
            await_screen(&mut presenter, "Find…");
            presenter.input(Event::Paste("Elegant".into())).unwrap();
            await_screen(&mut presenter, "1/1");
            presenter
                .input(Event::Key(KeyEvent::new(
                    KeyCode::Enter,
                    KeyModifiers::SHIFT,
                )))
                .unwrap();
            await_screen(&mut presenter, "1/1");
            for key in [KeyCode::Enter, KeyCode::F(3)] {
                presenter
                    .input(Event::Key(KeyEvent::new(key, KeyModifiers::NONE)))
                    .unwrap();
                await_screen(&mut presenter, "1/1");
            }
            presenter.input(Event::Key(close_key)).unwrap();
            await_screen(&mut presenter, "File background draft");
            assert!(!screen(&draw(&mut presenter)).contains("[ ⌕ ]"));
            presenter.input(Event::Paste("!".into())).unwrap();
            await_screen(&mut presenter, "File background draft!");
            presenter
                .input(Event::Key(KeyEvent::new(
                    KeyCode::Backspace,
                    KeyModifiers::NONE,
                )))
                .unwrap();
            draw(&mut presenter);
        }
        presenter
            .input(Event::Key(KeyEvent::new(
                KeyCode::Char('f'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();
        await_screen(&mut presenter, "Find…");
        presenter.input(Event::Paste("Elegant".into())).unwrap();
        await_screen(&mut presenter, "1/1");
        presenter
            .input(Event::Key(KeyEvent::new(
                KeyCode::Char('f'),
                KeyModifiers::ALT,
            )))
            .unwrap();
        await_screen(&mut presenter, "[ › 1. **Elegant response** ]");
        await_screen(&mut presenter, "Message");
        let wide = screen(&draw_at(&mut presenter, Rect::new(0, 0, 160, 32)));
        assert!(
            wide.lines()
                .any(|line| line.contains("[ › 1.") && line.contains("Message")),
            "{wide}"
        );
        assert!(
            wide.lines()
                .any(|line| line.contains("[ ∞ ]") && line.contains("History")),
            "{wide}"
        );
        let narrow = screen(&draw_at(&mut presenter, Rect::new(0, 0, 40, 16)));
        assert!(narrow.contains("1/1"), "{narrow}");
        assert!(narrow.contains("Elegant response"), "{narrow}");
        assert!(
            !narrow
                .lines()
                .any(|line| line.contains("[ ∞ ]") && line.contains("History")),
            "{narrow}"
        );
        presenter.input(Event::Paste("x".into())).unwrap();
        await_screen(&mut presenter, "No matches");
        presenter
            .input(Event::Key(KeyEvent::new(
                KeyCode::Char('z'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();
        await_screen(&mut presenter, "[ › 1. **Elegant response** ]");
        draw(&mut presenter);
        click(&mut presenter, "History");
        await_screen(&mut presenter, "Current content");
        click(&mut presenter, "Current content");
        await_screen(&mut presenter, "[ › 1. **Elegant response** ]");
        click(&mut presenter, "[ › 1. **Elegant response** ]");
        presenter.input(Event::Paste("x".into())).unwrap();
        await_screen(&mut presenter, "No matches");
        presenter
            .input(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();
        presenter.input(Event::Paste("Elegant".into())).unwrap();
        await_screen(&mut presenter, "[ › 1. **Elegant response** ]");
        presenter
            .input(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();
        presenter.input(Event::Paste("native".into())).unwrap();
        await_screen(&mut presenter, "[ 2.");
        // The selected canonical tool row includes its name, body and arguments,
        // and uses the same retained disclosure as the ordinary timeline.
        await_screen(&mut presenter, "Message");
        await_screen(&mut presenter, "[ Hide details ]");
        click(&mut presenter, "[ Hide details ]");
        await_screen(&mut presenter, "[ Show details ]");
        assert!(!screen(&draw(&mut presenter)).contains("Message"));
        click(&mut presenter, "[ Show details ]");
        await_screen(&mut presenter, "Message");
        let (column, row) = wait_for_target(&mut presenter, "[ › 1.", 0);
        for (kind, selected) in [
            (MouseEventKind::ScrollDown, "[ › 2."),
            (MouseEventKind::ScrollUp, "[ › 1."),
        ] {
            presenter
                .input(Event::Mouse(MouseEvent {
                    kind,
                    column,
                    row,
                    modifiers: KeyModifiers::NONE,
                }))
                .unwrap();
            await_screen(&mut presenter, selected);
        }
        presenter.input(Event::Paste("-missing".into())).unwrap();
        await_screen(&mut presenter, "No matches");
        presenter
            .input(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();
        presenter.input(Event::Paste("Elegant".into())).unwrap();
        await_screen(&mut presenter, "[ › 1. **Elegant response** ]");
        let buffer = draw(&mut presenter);
        let (row, column) = screen(&buffer)
            .lines()
            .enumerate()
            .filter_map(|(row, line)| {
                line.find("Elegant response")
                    .map(|column| (row as u16, column as u16))
            })
            .last()
            .unwrap();
        for kind in [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Drag(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
        ] {
            presenter
                .input(Event::Mouse(MouseEvent {
                    kind,
                    column: column
                        + if matches!(kind, MouseEventKind::Down(_)) {
                            0
                        } else {
                            7
                        },
                    row,
                    modifiers: KeyModifiers::NONE,
                }))
                .unwrap();
        }
        presenter.input(Event::Paste("x".into())).unwrap();
        await_screen(&mut presenter, "No matches");
        presenter
            .input(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();
        presenter.input(Event::Paste("Elegant".into())).unwrap();
        await_screen(&mut presenter, "[ › 1. **Elegant response** ]");
        presenter
            .input(Event::Key(KeyEvent::new(
                KeyCode::PageDown,
                KeyModifiers::NONE,
            )))
            .unwrap();
        draw(&mut presenter);
        presenter
            .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .unwrap();
        await_screen(&mut presenter, "File background draft");
        assert_eq!(
            std::fs::read_to_string(fixture.workspace.join("native-approved.txt")).unwrap(),
            "native file contents"
        );
        click(&mut presenter, "[ Details ]");
        for _ in 0..16 {
            if screen(&draw(&mut presenter)).contains("Clear additional approvals on save") {
                break;
            }
            presenter
                .input(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
                .unwrap();
            draw(&mut presenter);
        }
        click(&mut presenter, "Clear additional approvals on save");
        click(&mut presenter, "[ Save details ]");
        // Execution advanced the canonical metadata revision; preserve the revoke
        // draft while explicitly reviewing and adopting its new CAS basis.
        await_screen(&mut presenter, "This conversation changed.");
        click(&mut presenter, "[ Review current details ]");
        await_screen(&mut presenter, "[ Use this version ]");
        click(&mut presenter, "[ Use this version ]");
        click(&mut presenter, "[ Save details ]");
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        loop {
            draw(&mut presenter);
            let current = runtime
                .block_on(fixture.owner.session(&session_id))
                .unwrap();
            if current.configuration.boundary_revision == 2 {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "native revoke did not settle"
            );
            std::thread::yield_now();
        }
        presenter
            .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .unwrap();
    }
    #[cfg(target_os = "macos")]
    if let Some(loss) = &loss {
        let original = terminal::lost_reply(&mut presenter, &runtime, &fixture, &session_id, loss);
        let consumer = plan
            .instances()
            .values()
            .find(|instance| instance.path == ["terminal-panel"])
            .expect("original terminal panel instance")
            .identity
            .clone();
        let intents = runtime
            .block_on(caller.application_intents(deployment.deployment, consumer.clone(), 0, 128))
            .unwrap();
        assert_eq!(
            intents.len(),
            1,
            "only the original start intent was dispatched"
        );
        assert_eq!(intents[0].reference, original);
        assert_eq!(intents[0].caller, deployment);
        assert_eq!(intents[0].consumer, consumer);
        assert!(!presenter.can_shutdown());
        let shutdown = presenter.shutdown().unwrap_err();
        assert!(
            matches!(
                shutdown,
                maka_ui::Error::Core(maka_plugin_core::Error::CleanupFailed(_))
            ),
            "original invocation failure must survive surface restoration: {shutdown}"
        );
        runtime.block_on(async {
            assert!(connection.close().await.is_err());
            wire_server.close().await.unwrap();
            manager_connection.close().await.unwrap();
            manager_server.close().await.unwrap();
            loss.drain().await;
        });
        // Isolated fixture destruction after actual Host channels drained is
        // not an acknowledgement of the failed original UI/native call.
        drop(presenter);
        drop(registrations);
        drop(connection);
        runtime.block_on(manager_app.shutdown()).unwrap();
        runtime.block_on(fixture.app.shutdown()).unwrap();
        runtime.block_on(fixture.config.shutdown()).unwrap();
        runtime.block_on(caller.shutdown()).unwrap();
        return;
    }
    #[cfg(target_os = "macos")]
    if process_permissions {
        terminal::run(&mut presenter, &runtime, &fixture, &session_id);
    }
    if fail_labels_initialization || file_permissions || image || form {
        if matches!(case, ProductCase::FilePermissions) {
            message_details::run(&mut presenter, &runtime, &fixture, &session_id);
        }
        if fail_labels_initialization {
            // Saved notice, retained old fields, an actual different selected
            // Session and a two-row CJK workspace path used to hide all actions.
            click(&mut presenter, "[ Toggle sidebar ]");
            click(&mut presenter, "[ New conversation ]");
            wait_for_target(&mut presenter, "    New conversation", 0);
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
        }
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
    // Actual catalog filtering changes only the directory. The native draft
    // survives an empty page, then canonical background changes arrive without refresh.
    click(&mut presenter, "[ Toggle sidebar ]");
    click(&mut presenter, "All");
    // Each physical key follows a published frame; activating All also
    // advances the ordinary Root frame even when its filter is unchanged.
    let (x, y) = wait_for_target(&mut presenter, "All", 0);
    assert!(
        draw(&mut presenter)[(x, y)]
            .modifier
            .contains(ratatui::style::Modifier::UNDERLINED)
    );
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Right,
            KeyModifiers::NONE,
        )))
        .unwrap();
    let (x, y) = wait_for_target(&mut presenter, "Needs You", 0);
    assert!(
        draw(&mut presenter)[(x, y)]
            .modifier
            .contains(ratatui::style::Modifier::UNDERLINED)
    );
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(&mut presenter, "No conversations need you");
    await_screen(&mut presenter, "Needs You (0)");
    runtime
        .block_on(
            fixture
                .store
                .establish_interaction(&super::catalog_interaction(
                    &session_id,
                    "native-catalog-request",
                )),
        )
        .unwrap();
    await_screen(&mut presenter, "◇ New");
    await_screen(&mut presenter, "Needs You (1)");
    assert!(
        screen(&draw(&mut presenter))
            .lines()
            .any(|line| line.contains("◇ New") && line.contains("Needs You"))
    );
    runtime
        .block_on(fixture.store.commit_interaction_outcome(
            "native-catalog-request",
            maka_runtime::interaction::InteractionOutcome::Closure {
                reason: maka_runtime::interaction::ClosureReason::ProducerCancelled,
                committed_at: 20,
            },
        ))
        .unwrap();
    await_screen(&mut presenter, "No conversations need you");
    await_screen(&mut presenter, "Needs You (0)");
    // A different conversation enters and leaves the inbox without selecting
    // it or replacing the original composer owner.
    let background_id = format!("{}{}", fixture.owner.prefix, "f".repeat(32));
    runtime.block_on(async {
        let mut configuration = before.configuration.clone();
        configuration.title = "Background inbox".into();
        fixture
            .store
            .create_session(
                &background_id,
                "native-background-catalog",
                &configuration,
                1,
            )
            .await
            .unwrap();
        fixture
            .store
            .establish_interaction(&super::catalog_interaction(
                &background_id,
                "native-background-request",
            ))
            .await
            .unwrap();
    });
    await_screen(&mut presenter, "◇ Background");
    await_screen(&mut presenter, "Needs You (1)");
    assert!(screen(&draw(&mut presenter)).contains("Hello product"));
    runtime.block_on(async {
        fixture
            .store
            .commit_interaction_outcome(
                "native-background-request",
                maka_runtime::interaction::InteractionOutcome::Closure {
                    reason: maka_runtime::interaction::ClosureReason::ProducerCancelled,
                    committed_at: 20,
                },
            )
            .await
            .unwrap();
        let record = fixture
            .store
            .get_session::<SessionConfiguration>(&background_id)
            .await
            .unwrap()
            .unwrap();
        fixture
            .store
            .begin_session_removal(&background_id, record.revision)
            .await
            .unwrap();
    });
    await_screen(&mut presenter, "No conversations need you");
    await_screen(&mut presenter, "Needs You (0)");
    click(&mut presenter, "All");
    wait_for_target(&mut presenter, "    New conversation", 0);
    click(&mut presenter, "[ Hide sidebar ]");
    await_sidebar_closed(&mut presenter);
    click(&mut presenter, "Hello product");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)))
        .unwrap();
    presenter.input(Event::Paste("!".into())).unwrap();
    await_screen(&mut presenter, "Hello product!");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Backspace,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(&mut presenter, "Hello product");
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
            r#"{"fixture-model-19":{"vision":null,"thinkingLevels":["off","low","high"],"defaultThinkingLevel":"high"}}"#.into(),
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
            r#"{"fixture-model-19":{"vision":true,"thinkingLevels":["off","low","high"],"defaultThinkingLevel":"high"}}"#.into(),
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
    // The picker is a native modal: its fixed controls remain visible in a
    // short terminal, and Escape preserves the covered Composer draft.
    let narrow = screen(&draw_at(&mut presenter, Rect::new(0, 0, 40, 16)));
    for label in [
        "Search models",
        "[ Close ]",
        "[ Refresh models ]",
        "[ Default model ]",
        "[ More models ]",
    ] {
        assert!(narrow.contains(label), "{narrow}");
    }
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "Hello product");
    assert!(!screen(&draw(&mut presenter)).contains("Search models"));
    click(&mut presenter, "[ Choose model ]");
    await_screen(&mut presenter, "fixture-model-00");
    // The original list is one Tab stop. Down follows native row focus;
    // Enter confirms that exact row, including its two-line presentation.
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
        .unwrap();
    draw(&mut presenter);
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "› fixture-model-01");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "› fixture-model-00");
    let focused_row = screen(&draw(&mut presenter));
    assert!(
        focused_row.contains("› fixture-model-00") && !focused_row.contains("› fixture-model-01"),
        "{focused_row}"
    );
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_models_closed(&mut presenter);
    await_screen(&mut presenter, "[ fixture-model-00 ]");
    assert!(!screen(&draw(&mut presenter)).contains("Search models"));
    click(&mut presenter, "[ fixture-model-00 ]");
    await_screen(&mut presenter, "fixture-model-00");
    // Wheel scrolling uses the original native Reading viewport, not guest
    // three-row pages; query arrows subsequently reveal their semantic choice.
    let (x, y) = wait_for_target(&mut presenter, "› fixture-model-00", 0);
    for _ in 0..3 {
        presenter
            .input(Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: x,
                row: y,
                modifiers: KeyModifiers::NONE,
            }))
            .unwrap();
        draw(&mut presenter);
    }
    let scrolled = screen(&draw(&mut presenter));
    assert!(!scrolled.contains("› fixture-model-00"), "{scrolled}");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
        .unwrap();
    // Search Down enters the original list at its current model; subsequent
    // arrows belong to the native group, not a product navigation mirror.
    await_screen(&mut presenter, "› fixture-model-00");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "› fixture-model-01");
    for at in 2..16 {
        presenter
            .input(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
            .unwrap();
        await_screen(&mut presenter, &format!("› fixture-model-{at:02}"));
    }
    let scrolled = screen(&draw(&mut presenter));
    assert!(!scrolled.contains("› fixture-model-00"), "{scrolled}");
    // More now advances only the original bounded catalog cursor (16..19).
    click(&mut presenter, "More models");
    await_screen(&mut presenter, "fixture-model-16");
    await_screen(&mut presenter, "fixture-model-19");
    assert!(screen(&draw(&mut presenter)).contains("Close"));
    click(&mut presenter, "fixture-model-19");
    click(&mut presenter, "[ Apply ]");
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
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
        .unwrap();
    draw(&mut presenter);
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
    // Reuse the original native choice nested inside the model dialog. Escape
    // cancels only its menu; explicit Off survives Apply, reopen and query Enter.
    click(&mut presenter, "[ fixture-model-19 ]");
    await_screen(&mut presenter, "Thinking");
    let narrow = screen(&draw_at(&mut presenter, Rect::new(0, 0, 40, 16)));
    assert!(
        narrow.contains("Thinking") && narrow.contains("[ Apply ]"),
        "{narrow}"
    );
    draw(&mut presenter);
    click(&mut presenter, "Thinking");
    await_screen(&mut presenter, "○ Off");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    assert!(screen(&draw(&mut presenter)).contains("[ Close ]"));
    click(&mut presenter, "Thinking");
    await_screen(&mut presenter, "○ Off");
    click(&mut presenter, "○ Off");
    click(&mut presenter, "[ Apply ]");
    await_models_closed(&mut presenter);
    click(&mut presenter, "[ fixture-model-19 ]");
    await_screen(&mut presenter, "Thinking");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_models_closed(&mut presenter);
    assert!(!screen(&draw(&mut presenter)).contains("[ Close ]"));
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
    // The same ordinary Composer now occupies the bottom of the conversation.
    // Long drafts adapt locally to the viewport; shrinking and restoring the
    // terminal keeps the tail editable without replacing the draft owner.
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter
        .input(Event::Paste(
            (0..20)
                .map(|n| format!("composer-row-{n:02}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ))
        .unwrap();
    await_screen(&mut presenter, "composer-row-19");
    for (width, height) in [(80, 30), (40, 16), (80, 24)] {
        let rendered = screen(&draw_at(&mut presenter, Rect::new(0, 0, width, height)));
        let lines: Vec<_> = rendered.lines().collect();
        let send = lines
            .iter()
            .position(|line| line.contains("[ Send ]"))
            .unwrap();
        assert!(
            lines[send - 1].contains('╭') && lines[send].contains("composer-row-"),
            "{rendered}"
        );
        assert!(
            lines[send..].iter().any(|line| line.contains('╰')),
            "{rendered}"
        );
        assert!(rendered.contains("composer-row-19"), "{rendered}");
        assert!(rendered.contains("[ Send ]"), "{rendered}");
        presenter.input(Event::Paste("!".into())).unwrap();
        let rendered = screen(&draw_at(&mut presenter, Rect::new(0, 0, width, height)));
        assert!(rendered.contains("composer-row-19!"), "{rendered}");
    }
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter
        .input(Event::Paste("Hello product".into()))
        .unwrap();
    await_screen(&mut presenter, "Hello product");
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
    // The same retained editor sends a followup while the first HTTP response
    // is held. Its ordinary Rows component observes the actual SQL queue.
    await_screen(&mut presenter, "What would you like to work on?");
    let layout = screen(&draw(&mut presenter));
    assert!(
        layout
            .lines()
            .position(|line| line.contains("Hello product"))
            .unwrap()
            < layout
                .lines()
                .position(|line| line.contains("What would you like to work on?"))
                .unwrap(),
        "{layout}"
    );
    let narrow = screen(&draw_at(&mut presenter, Rect::new(0, 0, 40, 16)));
    let lines: Vec<_> = narrow.lines().collect();
    let controls = lines
        .iter()
        .position(|line| line.contains("[ Stop ]"))
        .unwrap();
    assert!(lines[controls].contains("[ Send ]"), "{narrow}");
    assert!(
        lines[controls - 1].contains('╭') && lines[controls + 1].contains('╰'),
        "{narrow}"
    );
    click(&mut presenter, "What would you like to work on?");
    presenter
        .input(Event::Paste("Queued product followup".into()))
        .unwrap();
    await_screen(&mut presenter, "[ Send ]");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(&mut presenter, "Queued product followup");
    await_screen(&mut presenter, "What would you like to work on?");
    await_screen(&mut presenter, "[ Send ]");
    let extra_messages = ["Extra queued one", "Extra queued two", "Extra queued three"];
    for message in extra_messages {
        click(&mut presenter, "What would you like to work on?");
        presenter.input(Event::Paste(message.into())).unwrap();
        await_screen(&mut presenter, "[ Send ]");
        presenter
            .input(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();
        await_screen(&mut presenter, "What would you like to work on?");
        await_screen(&mut presenter, "[ Send ]");
    }
    // Four actual queue messages overflow the original three-line native strip.
    // Clicking the bottom of its track must keep typing in the original Composer.
    await_screen(&mut presenter, "│");
    click(&mut presenter, "│");
    await_screen(&mut presenter, "Extra queued three");
    let pending = runtime
        .block_on(fixture.store.message_queue(&session_id))
        .unwrap();
    assert_eq!(pending.entries.len(), 4);
    assert_eq!(
        pending.entries[0].source.unprepared_content.text,
        "Queued product followup"
    );
    // The original queue count enters the last row without a product focus mirror.
    click(&mut presenter, "≡ 4");
    let wide = screen(&draw(&mut presenter));
    assert!(
        extra_messages.iter().all(|message| wide.contains(message)),
        "{wide}"
    );
    // Wrapped product chrome leaves a smaller session body at the same surface
    // size used by the original narrow reference. Keep the focused last row.
    let narrow = screen(&draw_at(&mut presenter, Rect::new(0, 0, 40, 16)));
    assert!(narrow.contains("Extra queued three"), "{narrow}");
    assert!(
        !narrow.contains("Extra queued one") && !narrow.contains("Extra queued two"),
        "{narrow}"
    );
    assert!(
        narrow.contains("≡ 4") && narrow.contains("[ Send ]") && narrow.contains("[ Stop ]"),
        "{narrow}"
    );
    draw(&mut presenter);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(&mut presenter, "[ Save message ]");
    await_screen(&mut presenter, "Extra queued three");
    click(&mut presenter, "[ Cancel ]");
    await_screen(&mut presenter, "≡ 4");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    draw(&mut presenter);
    // Empty native Composer Up enters the last queued row through the
    // product's declared focus binding; Enter opens that exact row's editor.
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)))
        .unwrap();
    draw(&mut presenter);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(&mut presenter, "[ Save message ]");
    await_screen(&mut presenter, "Extra queued three");
    // The ordinary product uses the original editor's intrinsic row count.
    // Resize a long unsaved draft; both its tail and actual controls remain
    // visible, then reducing the content shrinks the same dialog again.
    let dialog_height = |buffer: &Buffer| {
        let corner = |symbol: &str| {
            (0..buffer.area.height)
                .find(|&y| (0..buffer.area.width).any(|x| buffer[(x, y)].symbol() == symbol))
                .expect("dialog corner")
        };
        corner("└") - corner("┌") + 1
    };
    await_screen(&mut presenter, "[ Save message ]");
    let compact_height = dialog_height(&draw(&mut presenter));
    assert!(compact_height < 20);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter
        .input(Event::Paste(
            (0..20)
                .map(|n| format!("resize-row-{n:02}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ))
        .unwrap();
    await_screen(&mut presenter, "resize-row-19");
    assert!(dialog_height(&draw(&mut presenter)) > compact_height);
    let small = screen(&draw_at(&mut presenter, Rect::new(0, 0, 40, 18)));
    assert!(small.contains("resize-row-19"), "{small}");
    assert!(small.contains("[ Save message ]"), "{small}");
    assert!(small.contains("[ Cancel ]"), "{small}");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter
        .input(Event::Paste("short unsaved draft".into()))
        .unwrap();
    await_screen(&mut presenter, "short unsaved draft");
    assert_eq!(dialog_height(&draw(&mut presenter)), compact_height);
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    draw(&mut presenter);
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    presenter
        .input(Event::Paste("Unsent composer draft".into()))
        .unwrap();
    await_screen(&mut presenter, "Unsent composer draft");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)))
        .unwrap();
    presenter.input(Event::Paste("!".into())).unwrap();
    await_screen(&mut presenter, "Unsent composer draft!");
    click(&mut presenter, "│");
    await_screen(&mut presenter, "Queued product followup");
    click(&mut presenter, "Queued product followup");
    draw(&mut presenter);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(&mut presenter, "[ Save message ]");
    // The original multiline Editor is initialized through its actual reset receipt.
    // Enter inserts a newline; Ctrl+S captures that native revision and submits Update.
    click(&mut presenter, "Queued product followup");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::End,
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    presenter
        .input(Event::Paste("Edited in native queue editor".into()))
        .unwrap();
    await_screen(&mut presenter, "Edited in native queue editor");
    draw(&mut presenter);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while screen(&draw(&mut presenter)).contains("Edit queued message") {
        assert!(
            std::time::Instant::now() < deadline,
            "native queue edit did not settle"
        );
        std::thread::yield_now();
    }
    let edited = runtime
        .block_on(fixture.store.message_queue(&session_id))
        .unwrap();
    assert_eq!(
        edited.entries[0].source.message.message_id,
        pending.entries[0].source.message.message_id
    );
    assert_eq!(
        edited.entries[0].source.unprepared_content.text,
        "Queued product followup\nEdited in native queue editor"
    );
    await_screen(&mut presenter, "Unsent composer draft");
    await_screen(&mut presenter, "Queued product followup");
    click(&mut presenter, "Queued product followup");
    draw(&mut presenter);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('e'),
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(&mut presenter, "[ Save message ]");
    click(&mut presenter, "Edited in native queue editor");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::End,
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter.input(Event::Paste(" again".into())).unwrap();
    await_screen(&mut presenter, "Edited in native queue editor again");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while screen(&draw(&mut presenter)).contains("Edit queued message") {
        assert!(
            std::time::Instant::now() < deadline,
            "native Ctrl+Enter save did not settle"
        );
        std::thread::yield_now();
    }
    await_screen(&mut presenter, "Unsent composer draft");
    let edited = runtime
        .block_on(fixture.store.message_queue(&session_id))
        .unwrap();
    assert_eq!(
        edited.entries[0].source.unprepared_content.text,
        "Queued product followup\nEdited in native queue editor again"
    );
    click(&mut presenter, "Queued product followup");
    draw(&mut presenter);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Delete,
            KeyModifiers::NONE,
        )))
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while screen(&draw(&mut presenter)).contains("Queued product followup") {
        assert!(
            std::time::Instant::now() < deadline,
            "native queue retraction did not settle"
        );
        std::thread::yield_now();
    }
    // Each original row exit restores Composer; fresh Tab enters at the last
    // queued message, including after the previous selected message disappears.
    for message in extra_messages.into_iter().rev() {
        await_screen(&mut presenter, message);
        presenter
            .input(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        draw(&mut presenter);
        presenter
            .input(Event::Key(KeyEvent::new(
                KeyCode::Delete,
                KeyModifiers::NONE,
            )))
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        while screen(&draw(&mut presenter)).contains(message) {
            assert!(
                std::time::Instant::now() < deadline,
                "extra queued message did not retract"
            );
            std::thread::yield_now();
        }
    }
    assert!(
        runtime
            .block_on(fixture.store.message_queue(&session_id))
            .unwrap()
            .entries
            .is_empty()
    );
    // Removal returns to the original Composer; the held Enter cannot send a
    // new message or replay the row action into the adjacent control.
    let mut held = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    held.kind = crossterm::event::KeyEventKind::Repeat;
    presenter.input(Event::Key(held)).unwrap();
    presenter
        .input(Event::Paste("Question background draft".into()))
        .unwrap();
    await_screen(&mut presenter, "Question background draft");
    respond
        .take()
        .unwrap()
        .send(fixture.workspace.clone())
        .unwrap();
    await_screen(&mut presenter, "[ Ctrl G · Review request ]");
    let review = wait_for_target(&mut presenter, "[ Ctrl G · Review request ]", 0);
    let draft = wait_for_target(&mut presenter, "Question background draft", 0);
    assert!(
        review.1 < draft.1 && draft.1 - review.1 <= 3,
        "review must remain beside the original composer"
    );
    let narrow = screen(&draw_at(&mut presenter, Rect::new(0, 0, 40, 16)));
    assert!(narrow.contains("[ Ctrl G · Review request ]"), "{narrow}");
    let wrapped_draft = narrow
        .lines()
        .map(|line| line.trim().trim_matches('│').trim())
        .collect::<String>();
    assert!(
        wrapped_draft.contains("Question background draft"),
        "{narrow}"
    );
    draw(&mut presenter);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('g'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    await_screen(&mut presenter, "Tool arguments");
    let (x, y) = wait_for_target(&mut presenter, "[ Close ]", 0);
    assert!(
        draw(&mut presenter)[(x, y)]
            .modifier
            .contains(ratatui::style::Modifier::REVERSED),
        "opening service confirmation must focus Close, never Allow"
    );
    let original = runtime
        .block_on(fixture.store.pending_interactions(&session_id))
        .unwrap();
    assert_eq!(original.len(), 1);
    assert!(matches!(
        original[0].request,
        maka_runtime::interaction::InteractionRequest::ServiceApproval { .. }
    ));
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('g'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    await_screen(&mut presenter, "Tool arguments");
    assert_eq!(
        runtime
            .block_on(fixture.store.pending_interactions(&session_id))
            .unwrap(),
        original,
        "Ctrl-G while reviewing cannot answer or replace the original request"
    );
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "Question background draft");
    click(&mut presenter, "[ Ctrl G · Review request ]");
    await_screen(&mut presenter, "Tool arguments");
    wait_for_target(&mut presenter, "Allow this call", 0);
    click(&mut presenter, "Deny this call");
    let (x, y) = wait_for_target(&mut presenter, "Deny this call", 0);
    assert!(
        draw(&mut presenter)[(x, y)]
            .modifier
            .contains(ratatui::style::Modifier::UNDERLINED),
        "the selected Deny frame must be presented before the next physical key"
    );
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Right,
            KeyModifiers::NONE,
        )))
        .unwrap();
    let (x, y) = wait_for_target(&mut presenter, "Allow this call", 0);
    assert!(
        draw(&mut presenter)[(x, y)]
            .modifier
            .contains(ratatui::style::Modifier::UNDERLINED),
        "native Right moves focus to Allow without approving"
    );
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    assert_eq!(
        runtime
            .block_on(fixture.store.pending_interactions(&session_id))
            .unwrap(),
        original,
        "selecting Allow must not execute before Confirm choice"
    );
    click(&mut presenter, "[ Confirm choice ]");
    await_screen(&mut presenter, "This call was approved");
    click(&mut presenter, "[ Close ]");
    await_screen(&mut presenter, "Question background draft");
    await_screen(&mut presenter, "[ Ctrl G · Review request ]");
    click(&mut presenter, "[ Ctrl G · Review request ]");
    await_screen(&mut presenter, "Choose\\u{A}format");
    let (x, y) = wait_for_target(&mut presenter, "[ Close ]", 0);
    assert!(
        draw(&mut presenter)[(x, y)]
            .modifier
            .contains(ratatui::style::Modifier::REVERSED),
        "opening a question must focus Close, never a decision"
    );
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    await_screen(&mut presenter, "Question background draft");
    click(&mut presenter, "[ Ctrl G · Review request ]");
    await_screen(&mut presenter, "Choose\\u{A}format");
    click(&mut presenter, "○ Brief");
    click(&mut presenter, "1 ✓");
    let (x, y) = wait_for_target(&mut presenter, "1 ✓", 0);
    assert!(
        draw(&mut presenter)[(x, y)]
            .modifier
            .contains(ratatui::style::Modifier::UNDERLINED)
    );
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Right,
            KeyModifiers::NONE,
        )))
        .unwrap();
    let (x, y) = wait_for_target(&mut presenter, "2 ○", 0);
    assert!(
        draw(&mut presenter)[(x, y)]
            .modifier
            .contains(ratatui::style::Modifier::UNDERLINED)
    );
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(&mut presenter, "Any detail?");
    click(&mut presenter, "[ Write an answer ]");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
        .unwrap();
    draw(&mut presenter);
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
        .unwrap();
    draw(&mut presenter);
    presenter
        .input(Event::Paste("native answer".into()))
        .unwrap();
    await_screen(&mut presenter, "native answer");
    click(&mut presenter, "3 ○");
    click(&mut presenter, "[ Skip this question ]");
    click(&mut presenter, "[ Submit answers ]");
    await_screen(&mut presenter, "Answers recorded");
    click(&mut presenter, "[ Close ]");
    await_screen(&mut presenter, "Question background draft");
    await_screen(&mut presenter, "Elegant response");
    await_screen(&mut presenter, "[ Read message source ]");
    // The bounded tail may contain only the assistant beside empty canonical
    // boundaries; Earlier retrieves the user through the real sparse-row cursor.
    await_screen(&mut presenter, "[ Earlier messages ]");
    for _ in 0..16 {
        if screen(&draw(&mut presenter)).contains("Hello product") {
            break;
        }
        click(&mut presenter, "[ Earlier messages ]");
        // The first page no longer has Earlier; Latest remains a load-gated control.
        wait_for_target(&mut presenter, "[ Latest messages ]", 0);
    }
    await_screen(&mut presenter, "Hello product");
    runtime.block_on(&mut server).unwrap();
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
                maka_event_log::sessions::CatalogFilter {
                    include_archived: false,
                    pending_only: false,
                },
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
        fixture
            .store
            .set_session_archived::<SessionConfiguration>(&session, true, 4)
            .await
            .unwrap();
        session
    });
    let mut reopened = start();
    await_screen(&mut reopened, "[ Toggle sidebar ]");
    click(&mut reopened, "[ Toggle sidebar ]");
    click(&mut reopened, "▸ Archived");
    // Wide selection preserves the pinned pane. Close it explicitly before
    // continuing the original full-width tool-source acceptance sequence.
    click(&mut reopened, "    Native tool inspection");
    await_screen(&mut reopened, "This conversation is archived.");
    // The same title is now also in the main header. Select its actual
    // directory row again, without hitting the adjacent pane's Hide button.
    click(&mut reopened, "    Native tool inspection");
    await_screen(&mut reopened, "This conversation is archived.");
    assert!(screen(&draw(&mut reopened)).contains("[ Hide sidebar ]"));
    click(&mut reopened, "[ Hide sidebar ]");
    await_sidebar_closed(&mut reopened);
    await_screen(&mut reopened, "This conversation is archived.");
    assert!(!screen(&draw(&mut reopened)).contains("[ Send ]"));
    await_screen(&mut reopened, "Tool activity · Count");
    await_screen(&mut reopened, "[ Show details ]");
    assert_eq!(
        screen(&draw(&mut reopened))
            .matches("[ Show details ]")
            .count(),
        1
    );
    for (heading, beginning, ending) in [
        ("Tool arguments", "\"count\":42", "99999"),
        ("Tool result", "\"count\":43", "99998"),
    ] {
        click(&mut reopened, "[ Show details ]");
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
    click(&mut reopened, "[ Toggle sidebar ]");
    await_screen(&mut reopened, "[ New conversation ]");
    click(&mut reopened, "    New conversation");
    click(&mut reopened, "[ Hide sidebar ]");
    await_sidebar_closed(&mut reopened);
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
