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

pub(super) fn edit_labeled_input(tui: &mut Pty, label: &str, value: &str, secret: bool) {
    use unicode_width::UnicodeWidthStr;
    let (row, col) = loop {
        scroll_to(tui, label);
        let screen = tui.screen.snapshot().unwrap().screen;
        let (row, col) = screen
            .lines()
            .enumerate()
            .find_map(|(row, line)| {
                line.trim_end().ends_with(label).then(|| {
                    let at = line.rfind(label).unwrap();
                    (row + 2, line[..at].width() + 1)
                })
            })
            .unwrap();
        if row <= screen.lines().count() {
            break (row, col);
        }
        // Seeing a label on the last row does not expose its editor yet.
        tui.send(format!("\x1b[<65;{col};{}M", row - 1).as_bytes());
        tui.wait_until(|next| next != screen);
    };
    tui.send(format!("\x1b[<0;{col};{row}M\x1b[<0;{col};{row}m").as_bytes());
    tui.wait_until(|_| true);
    tui.send(b"\x01");
    tui.send(value.as_bytes());
    if secret {
        tui.wait_until(|screen| screen.contains("••••") || screen.contains("****"));
    } else {
        tui.wait_for(value);
    }
}

#[test]
#[ignore = "starts the standard product and real Settings owner with retained native editors"]
fn standard_product_assistant_preferences_save_and_privacy_recover_across_restart() {
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
            // The empty navigation must keep its Settings entry at 50×18.
            narrow(&mut tui);
            tui.wait_until(|screen| !screen.contains("Settings"));
            tui.send(b"\x02");
            tui.wait_for("Settings");
            tui.wait_for("No sessions yet");
            click_text(&mut tui, "Settings");
            scroll_to(&mut tui, "Your name");
        } else {
            tui.wait_for("Assistant");
            click_text(&mut tui, "Assistant");
            tui.wait_for("Save personalization");
        }
        if first {
            super::insights::input(&mut tui, "Your name", "Private Tester");
            scroll_to(&mut tui, "Save personalization");
            click_text(&mut tui, "Save personalization");
            tui.wait_for("Preferences saved.");
            scroll_to(&mut tui, "Privacy mode: Off");
            click_text(&mut tui, "Privacy mode: Off");
            tui.wait_for("Privacy mode: On");
        } else {
            tui.wait_for("Private Tester");
            tui.wait_for("Privacy mode: On");
            click_text(&mut tui, "Privacy mode: On");
            tui.wait_for("Privacy mode: Off");
        }
        tui.send(b"\x11");
        tui.finish();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let namespaces = RootNamespaces::for_current_account().unwrap();
            let owner = std::sync::Arc::new(RootOwner::open(&root, &namespaces).unwrap());
            let host =
                std::sync::Arc::new(RootOwner::open(&root.join("host"), &namespaces).unwrap());
            let config = maka_config::ConfigurationStore::for_root(host)
                .await
                .unwrap();
            let policy = config.runtime_policy().await.unwrap();
            assert_eq!(policy.policy.personalization.display_name, "Private Tester");
            assert_eq!(policy.policy.privacy.incognito_active, first);
            let store = maka_event_log::EventLog::for_root(owner).await.unwrap();
            let launch = store.application_launch().await.unwrap().unwrap();
            let intents = store
                .application_deployment_intents(launch.deployment, 0, 64)
                .await
                .unwrap();
            assert_eq!(
                intents
                    .iter()
                    .filter(|intent| intent.reference.method == "updateAssistantPreferences")
                    .count(),
                if first { 2 } else { 3 }
            );
            store.close().await.unwrap();
            config.shutdown().await.unwrap();
        });
    }
}

// A single wheel position must traverse contributed settings along with the
// ordinary categories; no second inner viewport should trap the user.
pub(super) fn scroll_to(tui: &mut Pty, label: &str) {
    let mut seen = std::collections::HashSet::new();
    loop {
        tui.wait_until(|_| true);
        let before = tui.screen.snapshot().unwrap().screen;
        if before.contains(label) {
            return;
        }
        assert!(
            seen.insert(before.clone()),
            "settings scroll repeated before reaching {label}\n{before}"
        );
        let column = before
            .lines()
            .next()
            .map(unicode_width::UnicodeWidthStr::width)
            .unwrap()
            .saturating_sub(6)
            .max(1);
        tui.send(format!("\x1b[<65;{column};10M").as_bytes());
        tui.wait_until(|screen| screen != before);
    }
}

pub(super) fn narrow(tui: &mut Pty) {
    tui.frames.resize();
    tui.screen
        .resize(TerminalSize::new(50, 18).unwrap())
        .unwrap();
    let size = libc::winsize {
        ws_row: 18,
        ws_col: 50,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    assert_eq!(
        unsafe {
            libc::ioctl(
                tui.master.as_ref().unwrap().as_raw_fd(),
                libc::TIOCSWINSZ,
                &size,
            )
        },
        0
    );
}

#[test]
#[ignore = "updates the original Agent deployment and recovers Sessions settings across reactivation and restart"]
fn standard_product_session_defaults_reconfigure_and_recover_original_operation() {
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
        if first {
            tui.wait_until_with_timeout(
                |screen| screen.contains("Settings"),
                Duration::from_secs(45),
            );
            click_text(&mut tui, "Settings");
            click_text(&mut tui, "Sessions");
            tui.wait_for("Default workspace access: Read and write");
            click_text(&mut tui, "Read only");
        }
        tui.wait_until_with_timeout(
            |screen| {
                screen.contains("Default workspace access: Read only")
                    && screen.contains("Defaults saved.")
            },
            Duration::from_secs(60),
        );
        tui.wait_for("Allow permission requests: On");
        tui.send(b"\x11");
        tui.finish();
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
                    .filter(|intent| intent.reference.method == "configure")
                    .count(),
                1,
                "reactivation and restart retain the same original configuration operation"
            );
            store.close().await.unwrap();
        });
    }
}

#[test]
#[ignore = "saves Host proxy settings through native editors and recovers the original operation after restart"]
fn standard_product_host_proxy_keeps_private_password_and_recovers_original_operation() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let home = directory.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let root = directory.path().join("ui");
    let secret = "private-host-proxy-password";
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
        click_text(&mut tui, "Host");
        tui.wait_for("Use proxy:");
        if first {
            click_text(&mut tui, "Use proxy: Off");
            tui.wait_for("Use proxy: On");
            click_text(&mut tui, "Proxy authentication: Off");
            tui.wait_for("Proxy authentication: On");
            click_text(&mut tui, "Replace password");
            click_text(&mut tui, "New proxy password");
            tui.send(secret.as_bytes());
            tui.wait_until(|screen| !screen.contains("New proxy password"));
            assert!(!tui.screen.snapshot().unwrap().screen.contains(secret));
            // Keep the same native draft through a category round trip, then
            // save through the narrow outer Settings viewport.
            click_text(&mut tui, "Appearance");
            tui.wait_for("Palette");
            click_text(&mut tui, "Host");
            tui.wait_for("Use proxy: On");
            narrow(&mut tui);
            tui.wait_for("Palette");
            scroll_to(&mut tui, "Save proxy settings");
            click_text(&mut tui, "Save proxy settings");
            tui.wait_until(|screen| {
                screen.contains("Proxy settings saved.")
                    || screen.contains("Check the proxy fields")
                    || screen.contains("Save outcome is unknown")
            });
            assert!(
                tui.screen
                    .snapshot()
                    .unwrap()
                    .screen
                    .contains("Proxy settings saved.")
            );
        } else {
            tui.wait_for("Use proxy: On");
            tui.wait_for("Proxy authentication: On");
            tui.wait_for("Proxy password is saved.");
            click_text(&mut tui, "Proxy authentication: On");
            click_text(&mut tui, "Save proxy settings");
            tui.wait_for("Proxy settings saved.");
            tui.wait_for("No proxy password saved.");
        }
        assert!(!tui.screen.snapshot().unwrap().screen.contains(secret));
        tui.send(b"\x11");
        tui.finish();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let namespaces = RootNamespaces::for_current_account().unwrap();
            let owner = std::sync::Arc::new(RootOwner::open(&root, &namespaces).unwrap());
            let host =
                std::sync::Arc::new(RootOwner::open(&root.join("host"), &namespaces).unwrap());
            let config = maka_config::ConfigurationStore::for_root(host)
                .await
                .unwrap();
            let network = config.network_configuration().await.unwrap();
            assert!(network.proxy.enabled);
            assert_eq!(network.proxy.auth_enabled, first);
            assert_eq!(network.password.as_deref(), first.then_some(secret));
            let store = maka_event_log::EventLog::for_root(owner).await.unwrap();
            let launch = store.application_launch().await.unwrap().unwrap();
            let intents = store
                .application_deployment_intents(launch.deployment, 0, 64)
                .await
                .unwrap();
            assert_eq!(
                intents
                    .iter()
                    .filter(|intent| intent.reference.method == "updateNetwork")
                    .count(),
                if first { 1 } else { 2 }
            );
            assert!(!serde_json::to_string(&intents).unwrap().contains(secret));
            store.close().await.unwrap();
            config.shutdown().await.unwrap();
        });
    }
}

#[test]
#[ignore = "starts the standard product and real Jev owner with private native editors and original-operation recovery"]
fn standard_product_jev_settings_preserve_drafts_and_private_credentials_across_restart() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let home = directory.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let root = directory.path().join("ui");
    let secret = "private-jev-acceptance-key";
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
        click_text(&mut tui, "Jev");
        tui.wait_for("Jev enabled:");
        if first {
            edit_labeled_input(&mut tui, "Model", "acceptance-jev-model", false);
            click_text(&mut tui, "Appearance");
            tui.wait_for("Palette");
            click_text(&mut tui, "Jev");
            tui.wait_for("acceptance-jev-model");
            scroll_to(&mut tui, "Save settings");
            click_text(&mut tui, "Save settings");
            tui.wait_for("Saved");
            edit_labeled_input(&mut tui, "New key or header value", secret, true);
            assert!(!tui.screen.snapshot().unwrap().screen.contains(secret));
            narrow(&mut tui);
            tui.wait_for("Palette");
            scroll_to(&mut tui, "Save API key");
            click_text(&mut tui, "Save API key");
            // The saved key is read from its native owner after the receipt.
            tui.wait_until(|screen| {
                screen.contains("API key saved")
                    || screen.contains("outcome is unknown")
                    || screen.contains("Invalid settings")
            });
            assert!(
                tui.screen
                    .snapshot()
                    .unwrap()
                    .screen
                    .contains("API key saved")
            );
        } else {
            tui.wait_for("acceptance-jev-model");
            scroll_to(&mut tui, "API key saved");
            scroll_to(&mut tui, "Remove API key");
            click_text(&mut tui, "Remove API key");
            tui.wait_for("Remove this saved credential?");
            tui.send(b"\x1b");
            tui.wait_until(|screen| !screen.contains("Remove this saved credential?"));
            click_text(&mut tui, "Remove API key");
            tui.wait_for("Remove this saved credential?");
            click_text(&mut tui, "Remove credential");
            tui.wait_for("No API key saved");
        }
        assert!(!tui.screen.snapshot().unwrap().screen.contains(secret));
        tui.send(b"\x11");
        tui.finish();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let namespaces = RootNamespaces::for_current_account().unwrap();
            let owner = std::sync::Arc::new(RootOwner::open(&root, &namespaces).unwrap());
            let store = maka_event_log::EventLog::for_root(owner).await.unwrap();
            let launch = store.application_launch().await.unwrap().unwrap();
            let intents = store
                .application_deployment_intents(launch.deployment, 0, 64)
                .await
                .unwrap();
            let jev: Vec<_> = intents
                .iter()
                .filter(|intent| intent.reference.contract.name == "maka.workspace.Jev")
                .collect();
            assert_eq!(
                jev.iter()
                    .filter(|intent| intent.reference.method == "save")
                    .count(),
                1
            );
            assert_eq!(
                jev.iter()
                    .filter(|intent| intent.reference.method == "credential")
                    .count(),
                if first { 1 } else { 2 }
            );
            assert!(!serde_json::to_string(&jev).unwrap().contains(secret));
            store.close().await.unwrap();
        });
    }
}
