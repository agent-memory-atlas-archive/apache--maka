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

use super::preferences::{edit_labeled_input, scroll_to};
use super::*;
#[test]
#[ignore = "uses actual native model selection, private Root presets and retained settings across restart"]
fn standard_product_subagents_save_native_model_and_tools_then_reopen_original_presets() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let home = directory.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let root = directory.path().join("ui");
    let namespaces = RootNamespaces::for_current_account().unwrap();
    drop(RootOwner::create(&root, &namespaces).unwrap());
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        // Only catalog evidence is seeded; the UI chooses it and saves through
        // the actual WorkHub service. No provider request or account is involved.
        let host=std::sync::Arc::new(RootOwner::create(&root.join("host"),&namespaces).unwrap());
        let settings=maka_config::ConfigurationStore::for_root(host).await.unwrap();
        let result=settings.create_connection(serde_json::from_value(json!({"expectedCatalogRevision":0,
            "connection":{"slug":"fixture","name":"Fixture","enabled":true,
            "provider":{"packageId":"maka.providers","entryId":"maka.providers","scope":"profile","name":"openai-compatible"},
            "configuration":{"baseUrl":"http://127.0.0.1:1/v1"},"enabledModelIds":["fixture-model"]}})).unwrap()).await.unwrap();
        assert!(matches!(result,maka_runtime::configuration::CatalogMutationResult::Committed{..}));
        settings.shutdown().await.unwrap();
    });
    for first in [true, false] {
        let mut args = vec!["application", "launch", "--root", root.to_str().unwrap()];
        if first {
            args.extend([
                "--workspace",
                workspace.to_str().unwrap(),
                "--trust-workspace",
            ]);
        }
        let mut tui = Pty::spawn_with(&args, false, |command| {
            command.current_dir(&workspace).env("HOME", &home);
        });
        tui.wait_for_output("Opening product…", Duration::from_secs(60));
        tui.wait_until_with_timeout(
            |screen| screen.contains("Settings") && !screen.contains("Recover application"),
            Duration::from_secs(40),
        );
        if first {
            click_text(&mut tui, "Settings");
        }
        click_text(&mut tui, "Subagents");
        tui.wait_for("New subagent");
        tui.wait_until(|screen| !screen.contains("Loading subagents…"));
        if first {
            click_text(&mut tui, "New subagent");
            tui.wait_for("Available to WorkHub");
            edit_labeled_input(&mut tui, "Name", "Careful Reader", false);
            edit_labeled_input(&mut tui, "Description", "Inspect source files", false);
            click_text(&mut tui, "Appearance");
            tui.wait_for("Palette");
            click_text(&mut tui, "Subagents");
            tui.wait_for("Careful Reader");
            scroll_to(&mut tui, "Choose model");
            click_text(&mut tui, "Choose model");
            tui.wait_for("fixture-model");
            click_text(&mut tui, "fixture-model");
            click_text(&mut tui, "Apply");
            tui.wait_for("Model: fixture-model");
            scroll_to(&mut tui, "Isolated Git workspace: Off");
            click_text(&mut tui, "Isolated Git workspace: Off");
            tui.wait_for("Isolated Git workspace: On");
            scroll_to(&mut tui, "Local read tools");
            click_text(&mut tui, "Local read tools");
            scroll_to(&mut tui, "Selected tools: 4");
            preferences::narrow(&mut tui);
            tui.wait_for("Palette");
            scroll_to(&mut tui, "Save subagent");
            click_text(&mut tui, "Save subagent");
            tui.wait_for("Subagent configurations saved.");
        } else {
            click_text(&mut tui, "Careful Reader");
            tui.wait_for("Inspect source files");
            tui.wait_for("Model: fixture-model");
            scroll_to(&mut tui, "[x] Read");
            scroll_to(&mut tui, "Remove subagent");
            click_text(&mut tui, "Remove subagent");
            tui.wait_for("Remove this saved configuration?");
            tui.send(b"\x1b");
            tui.wait_until(|screen| !screen.contains("Remove this saved configuration?"));
        }
        tui.send(b"\x11");
        tui.finish();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let store = maka_event_log::EventLog::for_root(std::sync::Arc::new(
                RootOwner::open(&root, &namespaces).unwrap(),
            ))
            .await
            .unwrap();
            let launch = store.application_launch().await.unwrap().unwrap();
            let intents = store
                .application_deployment_intents(launch.deployment, 0, 64)
                .await
                .unwrap();
            let saves = intents
                .iter()
                .filter(|item| item.reference.method == "savePresets")
                .collect::<Vec<_>>();
            assert_eq!(
                saves.len(),
                1,
                "reopening or cancelling removal cannot repeat the save"
            );
            store.close().await.unwrap();
            let host = maka_event_log::EventLog::for_root(std::sync::Arc::new(
                RootOwner::open(&root.join("host"), &namespaces).unwrap(),
            ))
            .await
            .unwrap();
            let ns = maka_plugins::storage::Namespace::new(
                maka_agent_service::workhub::IMPLEMENTATION,
                maka_runtime::scope::Scope::Profile,
            )
            .unwrap();
            let mut after = None;
            let mut preset = None;
            loop {
                let page = host
                    .plugin_data_scan(
                        &ns,
                        maka_plugins::storage::Scan {
                            prefix: String::new(),
                            after,
                        },
                    )
                    .await
                    .unwrap();
                for entry in page.entries {
                    if entry.key.ends_with("presets") {
                        preset = entry.record.data.value().cloned();
                    }
                }
                after = page.next_after;
                if after.is_none() {
                    break;
                }
            }
            let preset = preset.unwrap();
            assert_eq!(preset[0]["name"], "Careful Reader");
            assert_eq!(preset[0]["model"]["modelId"], "fixture-model");
            assert_eq!(preset[0]["isolatedWorkspace"], true);
            assert_eq!(
                preset[0]["tools"],
                json!(["Glob", "Grep", "Read", "ReadImage"])
            );
            host.close().await.unwrap();
        });
    }
}
