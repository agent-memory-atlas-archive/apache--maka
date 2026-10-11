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

fn change_field(tui: &mut Pty, label: &str, value: &str) {
    use unicode_width::UnicodeWidthStr;
    tui.wait_for(label);
    let screen = tui.screen.snapshot().unwrap().screen;
    let (row, col) = screen
        .lines()
        .enumerate()
        .find_map(|(row, line)| {
            line.find(label)
                .map(|at| (row + 1, line[..at].width() + 25))
        })
        .unwrap();
    tui.send(format!("\x1b[<0;{col};{row}M\x1b[<0;{col};{row}m\x1b[H\x1b[1;2F{value}").as_bytes());
}
fn confirm(tui: &mut Pty, label: &str) {
    tui.wait_for("╭Review task");
    click_text(tui, label);
    tui.wait_until(|screen| !screen.contains("╭Review task") && screen.contains("Task saved."));
}
fn run_now(tui: &mut Pty) {
    click_text(tui, "Run now");
    confirm(tui, "Run now");
    tui.wait_for("Acknowledge");
}
#[test]
#[ignore = "uses native editors and the real Host to exercise the complete scheduled-task workflow"]
fn standard_product_scheduler_create_edit_pause_trigger_acknowledge_and_original_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let root = directory.path().join("ui");
    let home = directory.path().join("home");
    std::fs::create_dir(&home).unwrap();
    for phase in 0..3 {
        let mut args = vec!["application", "launch", "--root", root.to_str().unwrap()];
        if phase == 0 {
            args.extend([
                "--workspace",
                workspace.to_str().unwrap(),
                "--trust-workspace",
            ]);
        }
        let started = Instant::now();
        let mut tui = Pty::spawn_with(&args, false, |command| {
            command.current_dir(&workspace).env("HOME", &home);
        });
        // First installation commits the UI, Host and published bridge before
        // opening the same bounded native view used on subsequent launches.
        tui.wait_for_output("Opening product…", Duration::from_secs(60));
        eprintln!(
            "scheduler phase {phase}: preparation {:?}",
            started.elapsed()
        );
        tui.wait_until_with_timeout(
            |screen| screen.contains("Settings"),
            Duration::from_secs(30),
        );
        eprintln!(
            "scheduler phase {phase}: first frame {:?}",
            started.elapsed()
        );
        if phase == 0 {
            click_text(&mut tui, "Scheduled tasks");
            tui.wait_for("No scheduled tasks.");
            click_text(&mut tui, "New task");
            change_field(&mut tui, "Title", "Reminder before edit");
            change_field(
                &mut tui,
                "Instructions or reminder",
                "Keep this reminder through restart",
            );
            click_text(&mut tui, "Once");
            click_text(&mut tui, "Interval");
            tui.wait_for("Every (seconds)");
            click_text(&mut tui, "Review task");
            confirm(&mut tui, "Approve and enable");
            tui.wait_for("Run history");
            click_text(&mut tui, "Pause");
            tui.wait_until(|screen| screen.contains("Paused") && screen.contains("Task saved."));
            click_text(&mut tui, "Resume");
            confirm(&mut tui, "Approve and enable");
            click_text(&mut tui, "Edit");
            tui.wait_for("Instructions or reminder");
            change_field(&mut tui, "Title", "Original reminder");
            click_text(&mut tui, "Review task");
            confirm(&mut tui, "Approve and save");
            run_now(&mut tui);
            click_text(&mut tui, "Remind in 5 minutes");
            tui.wait_until(|screen| {
                screen.contains("Task saved.")
                    && screen.contains("Run now")
                    && !screen.contains("Acknowledge")
            });
            run_now(&mut tui);
        } else if phase == 1 {
            tui.wait_for("Original reminder");
            click_text(&mut tui, "Reminder: Original reminder");
            tui.wait_for("Run history");
            tui.wait_for("Acknowledge");
            click_text(&mut tui, "Acknowledge");
            tui.wait_for("Reminder acknowledged");
            click_text(&mut tui, "Clear history");
            confirm(&mut tui, "Clear history");
            tui.wait_for("No triggers yet.");
            click_text(&mut tui, "Delete task");
            confirm(&mut tui, "Delete task");
            tui.wait_for("No scheduled tasks.");
        } else {
            tui.wait_for("No scheduled tasks.");
            tui.wait_for("Task saved.");
            assert!(
                !tui.screen
                    .snapshot()
                    .unwrap()
                    .screen
                    .contains("Original reminder")
            );
            click_text(&mut tui, "New task");
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
            tui.wait_for("Instructions or reminder");
            tui.send(b"Narrow draft");
            tui.wait_until(|screen| {
                let lines = screen.lines().collect::<Vec<_>>();
                lines
                    .windows(2)
                    .any(|pair| pair[0].trim() == "Title" && pair[1].contains("Narrow draft"))
            });
            click_text(&mut tui, "Review task");
            tui.wait_for("╭Review task");
            click_text(&mut tui, "Cancel");
            tui.wait_until(|screen| {
                !screen.contains("╭Review task") && screen.contains("Narrow draft")
            });
        }
        tui.send(b"\x11");
        tui.finish();
        assert!(!tui.screen.snapshot().unwrap().alternate_screen);
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let namespaces = RootNamespaces::for_current_account().unwrap();
            let owner = std::sync::Arc::new(RootOwner::open(&root, &namespaces).unwrap());
            let store = maka_event_log::EventLog::for_root(owner).await.unwrap();
            let launch = store.application_launch().await.unwrap().unwrap();
            let mut intents = Vec::new();
            let mut after = 0;
            loop {
                let page = store
                    .application_deployment_intents(launch.deployment, after, 64)
                    .await
                    .unwrap();
                let Some(last) = page.last() else { break };
                assert!(last.cursor > after);
                after = last.cursor;
                intents.extend(page);
            }
            assert_eq!(
                intents
                    .iter()
                    .filter(|i| i.reference.method == "schedule")
                    .count(),
                if phase == 0 { 7 } else { 10 },
                "reopen only observes original operations"
            );
            store.close().await.unwrap();
        });
    }
}
