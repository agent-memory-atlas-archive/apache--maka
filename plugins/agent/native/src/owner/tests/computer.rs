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
use maka_agent_facts::{
    event::{Fact, ToolOutcome},
    interaction::{InteractionRecord, InteractionRequest},
    tool_output::DurableToolProjection,
};

async fn pending(fixture: &Fixture, session: &str) -> InteractionRecord {
    let mut commits = fixture.store.subscribe_commits();
    loop {
        commits.borrow_and_update();
        if let Some(record) = fixture
            .store
            .pending_interactions(session)
            .await
            .unwrap()
            .into_iter()
            .next()
        {
            return record;
        }
        commits.changed().await.unwrap();
    }
}
async fn answer(f: &Fixture, record: &InteractionRecord, allow: bool) {
    let result = f.mutation(2, "respond", json!({"sessionId":record.session_id,"requestId":record.request_id,"answer":{"kind":"client_capability","decision":if allow { "allow" } else { "deny" }}}), Uuid::new_v4()).await.unwrap();
    assert_eq!(result["kind"], "recorded", "{result}");
}
async fn start(
    listener: &TcpListener,
    f: &Fixture,
    session: &str,
    method: &str,
    input: Value,
    expected_state: &str,
) {
    f.send(session).await.unwrap();
    let (mut socket, _) = listener.accept().await.unwrap();
    let sent = request(&mut socket).await;
    let tools = sent["tools"].as_array().unwrap();
    let tool = tools
        .iter()
        .find(|tool| tool["function"]["name"] == method)
        .expect("actual model tool descriptor");
    assert!(
        tool["function"]["description"]
            .as_str()
            .unwrap()
            .contains(expected_state),
        "{tool}"
    );
    service_tools::named_tool_response(&mut socket, method, input).await;
}
async fn finish(listener: &TcpListener, f: &Fixture) -> Value {
    let (mut socket, _) = listener.accept().await.unwrap();
    let sent = request(&mut socket).await;
    response(&mut socket).await;
    f.drained().await;
    sent
}
fn tool_text(sent: &Value) -> String {
    sent["messages"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|message| message["role"] == "tool")
        .unwrap()["content"]
        .to_string()
}

#[tokio::test]
async fn computer_product_tools_preserve_approval_repl_media_and_revocation() {
    tokio::time::timeout(Duration::from_secs(90), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let f = Fixture::with_services(&format!("http://{}/v1", listener.local_addr().unwrap()), None, None, Some(service_tools::Setup::computer())).await;
        images::enable_vision(&f).await;
        let session = f.create("Computer").await;
        let other = f.create("Other Computer").await;
        start(&listener, &f, &session, "cua_repl", json!({"code":"globalThis.deniedEffect = true;"}), "fresh").await;
        let denied = pending(&f, &session).await;
        assert!(matches!(&denied.request, InteractionRequest::ClientCapability { target, base_revision: Some(0), .. } if target == &maka_workspace_api::computer::target()));
        answer(&f, &denied, false).await;
        assert!(tool_text(&finish(&listener, &f).await).contains("not granted"));
        assert!(f.store.client_capability_grant(&session, &maka_workspace_api::computer::target()).await.unwrap().is_none());
        // Real native facade operations stay journaled under the original parent;
        // browser inventory with no configured browser never touches a user app.
        // A >1 MiB image crosses the ordinary service through a durable artifact,
        // without raising the Host JSON ceiling or losing the exact image bytes.
        let png: Vec<u8> = {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aM7cAAAAASUVORK5CYII=").unwrap()
        };
        let mut expected = vec![0; 1100 * 1024]; expected[..png.len()].copy_from_slice(&png);
        let code = format!("if (globalThis.deniedEffect) throw Error('denied cell ran'); globalThis.counter = 41; await cua.listBrowsers(); const screenshot = new Uint8Array(1100*1024); screenshot.set({}); await nodeRepl.emitImage({{bytes:screenshot,mimeType:'image/png'}}); nodeRepl.write('computer-effect-41');", serde_json::to_string(&png).unwrap());
        start(&listener, &f, &session, "cua_repl", json!({"code":code,"title":"Inspect selected browsers"}), "fresh").await;
        let allowed = pending(&f, &session).await;
        answer(&f, &allowed, true).await;
        let sent = finish(&listener, &f).await;
        assert!(sent.to_string().contains("computer-effect-41"), "Computer effect output was not projected");
        assert_eq!(images::model_images(&sent), vec![expected.clone()]);
        let observed = f.query("observe", json!({"sessionId":session})).await.unwrap();
        let through = observed["through"].as_u64().unwrap();
        let events = service_tools::facts(&f, &session).await;
        assert!(events.iter().any(|event| matches!(event, Fact::ToolDispatched { name, call, .. } if name == "cua_operation" && matches!(&call.origin, maka_agent_facts::tool_call::ToolOrigin::HostSdk { parent_operation_id: Some(_), .. }))));
        assert!(events.iter().any(|event| matches!(event, Fact::ToolDispatched { name, title: Some(title), .. } if name == "cua_repl" && title.fallback == "Inspect selected browsers")));
        assert!(events.iter().any(|event| matches!(event, Fact::ToolSettled { outcome: ToolOutcome::Succeeded { model_projection: DurableToolProjection::Content { parts }, .. }, .. } if parts.iter().any(|part| part.media().is_some()))));
        assert!(through > 0);
        start(&listener, &f, &session, "cua_repl", json!({"code":"nodeRepl.write(++globalThis.counter);"}), "ready").await;
        assert!(tool_text(&finish(&listener, &f).await).contains("42"));
        assert!(f.store.pending_interactions(&session).await.unwrap().is_empty());
        // Grants and VM globals never leak into another Session.
        start(&listener, &f, &other, "cua_repl", json!({"code":"nodeRepl.write(typeof globalThis.counter);"}), "fresh").await;
        answer(&f, &pending(&f, &other).await, true).await;
        assert!(tool_text(&finish(&listener, &f).await).contains("undefined"));
        // Revocation stops the cell; reset joins the VM before replacing it.
        start(&listener, &f, &session, "cua_repl", json!({"code":"await cua.listBrowsers(); nodeRepl.write('waiting'); await new Promise(() => {});","timeout_ms":120000}), "ready").await;
        let mut commits = f.store.subscribe_commits();
        loop {
            commits.borrow_and_update();
            let events = service_tools::facts(&f, &session).await;
            if events.iter().filter(|event| matches!(event, Fact::ToolDispatched { input, name, .. } if name == "cua_operation" && input["method"] == "listBrowsers")).count() >= 2 { break; }
            commits.changed().await.unwrap();
        }
        assert_eq!(files::change(&f, &session, "workspace_write", true, true).await["kind"], "committed");
        let cancelled = finish(&listener, &f).await;
        let stopped = tool_text(&cancelled);
        assert!(stopped.contains("changed") || stopped.contains("revoked") || stopped.contains("cancelled"), "{stopped}");
        assert!(stopped.contains("reset_required"), "{stopped}");
        start(&listener, &f, &session, "cua_reset", json!({}), "reset_required").await;
        assert!(tool_text(&finish(&listener, &f).await).contains("reset"));
        start(&listener, &f, &session, "cua_repl", json!({"code":"nodeRepl.write(typeof globalThis.counter);"}), "fresh").await;
        let renewal = pending(&f, &session).await;
        assert!(matches!(renewal.request, InteractionRequest::ClientCapability { base_revision: Some(1), .. }));
        answer(&f, &renewal, true).await;
        assert!(tool_text(&finish(&listener, &f).await).contains("undefined"));
        f.app.shutdown().await.unwrap(); f.config.shutdown().await.unwrap();
    }).await.unwrap();
}
