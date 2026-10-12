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

pub(super) fn input(tui: &mut Pty, label: &str, value: &str) {
    super::preferences::edit_labeled_input(tui, label, value, false);
}

#[test]
#[ignore = "starts the standard product and real Settings owner with native price editors"]
fn standard_product_prices_keep_native_drafts_and_reopen_the_original_saved_operation() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let home = directory.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let root = directory.path().join("ui");
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
        tui.wait_for("Usage & pricing");
        click_text(&mut tui, "Usage & pricing");
        tui.wait_for("Estimated cost");
        click_text(&mut tui, "Pricing");
        tui.wait_for("New price");
        if first {
            click_text(&mut tui, "New price");
            input(
                &mut tui,
                "Model key (provider:model)",
                "acceptance:retained-price",
            );
            input(&mut tui, "Input", "1.25");
            input(&mut tui, "Output", "5.5");
            click_text(&mut tui, "Overview");
            tui.wait_for("Estimated cost");
            click_text(&mut tui, "Pricing");
            tui.wait_until(|screen| {
                screen.contains("acceptance:retained-price")
                    && screen.contains("1.25")
                    && screen.contains("5.5")
            });
            super::preferences::narrow(&mut tui);
            tui.wait_for("Palette");
            super::preferences::scroll_to(&mut tui, "Save price");
            click_text(&mut tui, "Save price");
            // The editor collapses after the original receipt. Reveal the
            // preceding feedback in the same outer settings viewport.
            for _ in 0..50 {
                tui.send(b"\x1b[<64;44;10M");
                tui.read();
            }
            tui.wait_for("Palette");
            super::preferences::scroll_to(&mut tui, "Price saved.");
        }
        if first {
            super::preferences::scroll_to(&mut tui, "acceptance:retained-price");
        } else {
            tui.wait_for("acceptance:retained-price");
        }
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
            assert_eq!(
                intents
                    .iter()
                    .filter(|i| i.reference.method == "updatePrice")
                    .count(),
                1,
                "reopening observes the original receipt and never repeats the price mutation"
            );
            store.close().await.unwrap();
        });
    }
}
