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

async fn tool(socket: &mut TcpStream, name: &str, input: Value) {
    let delta = json!({"id":"skills-model","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":Uuid::new_v4().to_string(),"type":"function","function":{"name":name,"arguments":input.to_string()}}]},"finish_reason":null}]});
    let body = format!(
        "data: {delta}\n\ndata: {{\"id\":\"skills-model\",\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"tool_calls\"}}]}}\n\ndata: [DONE]\n\n"
    );
    socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
}
async fn next(listener: &TcpListener) -> (TcpStream, Value) {
    let (mut socket, _) = listener.accept().await.unwrap();
    let sent = request(&mut socket).await;
    let result = sent["messages"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|m| m["role"] == "tool")
        .unwrap();
    let content = result["content"].as_str().unwrap();
    let result = serde_json::from_str(content).unwrap_or_else(|_| Value::String(content.into()));
    (socket, result)
}
#[tokio::test]
async fn selected_skill_tools_use_actual_agent_context_and_original_grants_over_http() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("selected");
        std::fs::create_dir_all(root.join("review/references")).unwrap();
        std::fs::create_dir_all(root.join("unavailable")).unwrap();
        let instructions = format!("Use [guide](references/guide.txt). Lower-priority instructions.{}", " Review guidance.".repeat(600));
        std::fs::write(root.join("review/SKILL.md"), format!("---\nname: review\ndescription: Review changes\nrequired-tools: SkillSearch Skill\nallowed-tools: Superuser\n---\n{instructions}")).unwrap();
        std::fs::write(root.join("unavailable/SKILL.md"), "---\nname: unavailable\ndescription: Review changes\nrequired-tools: Superuser\n---\nCannot run").unwrap();
        std::fs::write(root.join("review/references/guide.txt"), "资源 🦀 ".repeat(4000)).unwrap();
        for granted in [true, false] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let setup = service_tools::Setup::skills(root.clone());
            let setup = if granted { setup } else { setup.without_grant() };
            let f = Fixture::with_services(&format!("http://{}/v1", listener.local_addr().unwrap()), 1, None, None, Some(setup)).await;
            let original = f.owner.settings.tools.clone();
            for (change, expected) in [
                ("json", "agent_skill_tool_projection_required"),
                ("method", "agent_skill_tool_method_invalid"),
                ("binding", "agent_tool_import_unavailable"),
            ] {
                let mut invalid = original.clone();
                match change {
                    "json" => invalid[0].input = contract::ToolInput::Json,
                    "method" => invalid[0].method = "list".into(),
                    "binding" => invalid[0].import = "absent".into(),
                    _ => unreachable!(),
                }
                assert_eq!(crate::tools::catalog(f.services.clone(), f.store.clone(), &invalid).err().as_deref(), Some(expected));
            }
            let session = f.create("Skill tools").await;
            f.send(&session).await.unwrap();
            let (mut socket, _) = listener.accept().await.unwrap();
            let sent = request(&mut socket).await;
            let schemas = sent["tools"].as_array().unwrap();
            assert_eq!(schemas.len(), 2);
            for tool in schemas {
                assert!(tool["function"]["parameters"]["properties"].get("tools").is_none());
                assert_eq!(tool["function"]["parameters"]["additionalProperties"], false);
            }
            tool(&mut socket, "SkillSearch", json!({"query":"Review"})).await;
            let (mut socket, result) = next(&listener).await;
            if granted {
                assert_eq!(result["totalEligible"], 1, "{result}");
                assert_eq!(result["matches"][0]["ref"], "source:fixture:review");
                let revision = result["revision"].clone();
                tool(&mut socket, "Skill", json!({"name":"source:fixture:review","revision":revision})).await;
                let (mut loaded_socket, loaded) = next(&listener).await;
                assert_eq!(loaded["kind"], "loaded", "{loaded}");
                assert!(loaded["instructions"].as_str().unwrap().contains("references/guide.txt"));
                assert_eq!(loaded["declaredTools"], json!(["Superuser"]));
                assert_eq!(loaded["receipt"]["success"], true);
                let mut complete = loaded["instructions"].as_str().unwrap().to_owned();
                let mut continuation = loaded["next"].clone();
                assert!(!continuation.is_null());
                while !continuation.is_null() {
                    tool(&mut loaded_socket, "Skill", continuation).await;
                    let (next_socket, page) = next(&listener).await;
                    assert_eq!(page["kind"], "loaded");
                    assert_eq!(page["truncated"], false);
                    assert_eq!(page["instructionOffset"], complete.len());
                    complete.push_str(page["instructions"].as_str().unwrap());
                    continuation = page["next"].clone();
                    loaded_socket = next_socket;
                }
                assert_eq!(complete, instructions);
                tool(&mut loaded_socket, "Skill", json!({"name":"source:fixture:review","revision":revision,"resource":{"path":"references/guide.txt"}})).await;
                let (mut resource_socket, resource) = next(&listener).await;
                assert_eq!(resource["kind"], "resource", "{resource}");
                assert!(!resource["next"].is_null());
                tool(&mut resource_socket, "Skill", resource["next"].clone()).await;
                let (mut continued_socket, continued) = next(&listener).await;
                assert_eq!(continued["kind"], "resource");
                assert_eq!(continued["revision"], revision);
                assert_ne!(continued["next"], resource["next"]);
                tool(&mut continued_socket, "SkillSearch", json!({"query":"Review","tools":["Superuser"]})).await;
                let (last_socket, rejected) = next(&listener).await;
                assert_ne!(rejected["totalEligible"], 2, "{rejected}");
                assert!(rejected.to_string().to_lowercase().contains("invalid"), "{rejected}");
                socket = last_socket;
            } else {
                assert_eq!(result, "native service invocation is not granted");
                assert!(result.get("revision").is_none());
            }
            response(&mut socket).await;
            f.drained().await;
            f.app.shutdown().await.unwrap();
            f.config.shutdown().await.unwrap();
        }
        let f = Fixture::with_services("http://127.0.0.1:9/v1", 1, None, None, Some(service_tools::Setup::forms())).await;
        let mut invalid = f.owner.settings.tools.clone();
        invalid[0].input = contract::ToolInput::Skill;
        assert_eq!(crate::tools::catalog(f.services.clone(), f.store.clone(), &invalid).err().as_deref(), Some("agent_skill_tool_contract_invalid"));
        f.app.shutdown().await.unwrap();
        f.config.shutdown().await.unwrap();
    }).await.unwrap();
}
