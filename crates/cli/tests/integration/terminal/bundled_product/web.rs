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

#[test]
#[ignore = "starts the standard product and real Web owner with a native secret input"]
fn standard_product_web_settings_keep_private_keys_and_recover_across_restart() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let home = directory.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let root = directory.path().join("ui");
    let secret = "private-web-terminal-key";
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
        tui.wait_for("Web");
        click_text(&mut tui, "Web");
        tui.wait_for("Search source");
        if first {
            click_text(&mut tui, "Web tools: Off");
            tui.wait_for("Web tools: On");
            click_text(&mut tui, "Tavily");
            tui.wait_for("Tavily search requires a saved API key.");
            click_text(&mut tui, "Tavily API key");
            tui.send(secret.as_bytes());
            tui.wait_until(|screen| !screen.contains("Tavily API key"));
            assert!(!tui.screen.snapshot().unwrap().screen.contains(secret));
            // A category round trip keeps the same native secret editor.
            click_text(&mut tui, "Appearance");
            tui.wait_for("Palette");
            click_text(&mut tui, "Web");
            tui.wait_for("Search source");
            click_text(&mut tui, "Save key");
            tui.wait_for("Tavily key saved");
            tui.wait_for("Tavily API key");
        } else {
            tui.wait_for("Web tools: On");
            tui.wait_for("Tavily key saved");
            click_text(&mut tui, "Remove key");
            tui.wait_for("Remove the saved Tavily key?");
            tui.send(b"\x1b");
            tui.wait_until(|screen| !screen.contains("Remove the saved Tavily key?"));
            tui.wait_for("Tavily key saved");
            click_text(&mut tui, "Remove key");
            tui.wait_for("Remove the saved Tavily key?");
            click_text(&mut tui, "Remove key");
            tui.wait_for("No Tavily key");
        }
        assert!(!tui.screen.snapshot().unwrap().screen.contains(secret));
        tui.send(b"\x11");
        tui.finish();
        assert!(!tui.screen.snapshot().unwrap().alternate_screen);
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let namespaces = RootNamespaces::for_current_account().unwrap();
            let owner = std::sync::Arc::new(RootOwner::open(&root, &namespaces).unwrap());
            let store = maka_event_log::EventLog::for_root(owner).await.unwrap();
            let launch = store.application_launch().await.unwrap().unwrap();
            let intents = store
                .application_deployment_intents(launch.deployment, 0, 64)
                .await
                .unwrap();
            for (method, count) in [
                ("save", 2),
                ("saveKey", 1),
                ("deleteKey", usize::from(!first)),
            ] {
                assert_eq!(
                    intents
                        .iter()
                        .filter(|i| i.reference.contract.name == "maka.workspace.Web"
                            && i.reference.method == method)
                        .count(),
                    count,
                    "reopening never repeats {method}"
                );
            }
            assert!(!serde_json::to_string(&intents).unwrap().contains(secret));
            store.close().await.unwrap();
        });
    }
}
