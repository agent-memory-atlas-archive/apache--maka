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
use maka_plugins::package::{MANIFEST_FILE, Package};
use serde_json::Value;
use std::collections::BTreeMap;

fn click_action(tui: &mut Pty, label: &str) {
    use unicode_width::UnicodeWidthStr;
    let locate = |screen: &str| {
        let modal = screen.contains("╭Confirm");
        screen
            .lines()
            .enumerate()
            .filter(|(_, line)| !modal || line.contains('│'))
            .filter_map(|(row, line)| line.find(label).map(|at| (row + 1, line[..at].width() + 1)))
            .last()
    };
    tui.wait_until(|screen| locate(screen).is_some());
    let (row, col) = locate(&tui.screen.snapshot().unwrap().screen).unwrap();
    tui.send(format!("\x1b[<0;{col};{row}M\x1b[<0;{col};{row}m").as_bytes());
}

fn wait_data(tui: &mut Pty, text: &str) {
    tui.wait_until_with_timeout(
        |screen| screen.contains(text) && !screen.contains("Connecting…"),
        Duration::from_secs(45),
    );
}

fn notebook(directory: &std::path::Path) -> std::path::PathBuf {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let apps: Value = serde_json::from_slice(
        &std::fs::read(repo.join("packages/agent-product/dist/packages/apps/maka.extension.json"))
            .unwrap(),
    )
    .unwrap();
    let entry = &apps["application"]["entries"]["tui"];
    let contract = &entry["imports"]["pages"]["contract"];
    let source = r#"
        import {presentations,output} from './packages/agent-product/src/view.ts';
        const contract=CONTRACT;
        let configuration;
        export function activate(value){configuration=value;return {Page:{kind:'component',contract,component:['page']}};}
        function frame(state,props){
            const ui=presentations.tui.paint(props.theme);
            return output(state,ui.box('notebook',[
                ui.text('title',configuration.title,16),
                ui.text('count','Notebook count: '+state.count,14),
                ui.button('increment','Increment notebook',false),
                presentations.tui.slot('draft','editor',{
                    appearance:props.theme.textarea,accessibilityLabel:'Notebook text',placeholder:'Write a notebook draft',
                    readOnly:false,disabled:!props.open,tabIndex:0,softWrap:true,terminalRows:{min:3,max:6},autoFocus:true
                },{})
            ],{visible:props.open,gap:1,terminal:{grow:1}}));
        }
        export const page={
            mount(props){return frame({revision:0,count:0},props);},
            update(state,props){return frame({...state,revision:state.revision+1},props);},
            event(state,input,props){return frame({...state,revision:state.revision+1,count:state.count+(input.kind==='raw'&&input.key==='increment'?1:0)},props);}
        };
    "#.replace("CONTRACT", &contract.to_string());
    let built = Command::new("node").current_dir(&repo).args(["--input-type=module","-e",r#"
        import {build} from 'esbuild';
        const result=await build({stdin:{contents:process.argv[1],resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',platform:'neutral',write:false});
        process.stdout.write(result.outputFiles[0].contents);
    "#]).arg(source).output().unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let manifest = json!({"schemaVersion":2,"id":"example.notebook","displayName":"Notebook","application":{"entries":{"tui":{
        "implementation":{"kind":"javascript","module":"notebook.js","realm":"dedicated"},
        "scene":entry["scene"],
        "configuration":{"type":"object","additionalProperties":false,"properties":{"title":{"type":"string","title":"Title","default":"Native notebook"}},"required":["title"]},
        "imports":{"editor":entry["imports"]["editor"]},
        "exports":{"Page":{"contract":contract,"availability":"required"}}
    }}}});
    let package = Package::new(BTreeMap::from([
        (MANIFEST_FILE.into(), serde_json::to_vec(&manifest).unwrap()),
        ("notebook.js".into(), built.stdout),
    ]))
    .unwrap();
    let file = directory.join("notebook.maka-plugin");
    std::fs::write(&file, package.to_bundle()).unwrap();
    file
}

#[test]
#[ignore = "installs and configures a dynamic app through native forms and the real Host"]
fn standard_product_apps_install_bind_retain_close_remove_and_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let root = directory.path().join("ui");
    let file = notebook(directory.path());
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
            command.current_dir(&workspace);
        });
        tui.wait_until_with_timeout(
            |screen| screen.contains(if first { "Settings" } else { "No apps yet." }),
            Duration::from_secs(60),
        );
        if first {
            click_text(&mut tui, "Apps");
        }
        wait_data(&mut tui, "No apps yet.");
        if first {
            click_action(&mut tui, "Install package");
            tui.wait_for("Plugin file path");
            click_text(&mut tui, "Absolute path to a .maka-plugin file");
            tui.send(format!("\x1b[200~{}\x1b[201~\r", file.display()).as_bytes());
            tui.wait_until(|screen| {
                screen
                    .lines()
                    .any(|line| line.trim() == "notebook.maka-plugin")
            });
            click_action(&mut tui, "Install package");
            tui.wait_for("╭Confirm");
            click_action(&mut tui, "Install package");
            tui.wait_until_with_timeout(
                |screen| screen.contains("Package installed."),
                Duration::from_secs(45),
            );
            eprintln!("Apps: original package install completed");
            wait_data(&mut tui, "Notebook");
            click_text(&mut tui, "Notebook");
            wait_data(&mut tui, "Choose the capabilities this app can use.");
            click_text(&mut tui, "editor · maka.ui.Textarea · 0");
            wait_data(&mut tui, "editor / Textarea");
            click_text(&mut tui, "editor / Textarea");
            click_text(&mut tui, "Continue");
            tui.wait_for("editor · maka.ui.Textarea · 1");
            click_text(&mut tui, "Review app");
            tui.wait_for("Add this component with the settings");
            click_action(&mut tui, "Add app");
            tui.wait_until_with_timeout(
                |screen| screen.contains("App added."),
                Duration::from_secs(45),
            );
            eprintln!("Apps: explicit instance and binding committed");
            wait_data(&mut tui, "Notebook");
            click_text(&mut tui, "Notebook");
            tui.wait_for("Native notebook");
            tui.wait_for("Notebook count: 0");
            click_text(&mut tui, "Increment notebook");
            tui.wait_for("Notebook count: 1");
            click_action(&mut tui, "Write a notebook draft");
            tui.send(b"Retained native draft");
            tui.wait_for("Retained native draft");
            click_text(&mut tui, "Back");
            tui.wait_for("Remove from Apps");
            click_text(&mut tui, "Notebook");
            tui.wait_for("Notebook count: 1");
            tui.wait_for("Retained native draft");
            click_text(&mut tui, "Close app");
            tui.wait_for("Remove from Apps");
            click_text(&mut tui, "Notebook");
            tui.wait_for("Notebook count: 0");
            tui.wait_for("Write a notebook draft");
            assert!(
                !tui.screen
                    .snapshot()
                    .unwrap()
                    .screen
                    .contains("Retained native draft")
            );
            click_text(&mut tui, "Back");
            click_action(&mut tui, "Remove from Apps");
            tui.wait_for("Its plugin and saved data");
            click_action(&mut tui, "Remove from Apps");
            tui.wait_until_with_timeout(
                |screen| screen.contains("App removed."),
                Duration::from_secs(45),
            );
            eprintln!("Apps: native view retained, closed, and removed");
            wait_data(&mut tui, "No apps yet.");
        }
        tui.send(b"\x11");
        tui.finish();
        assert!(!String::from_utf8_lossy(&tui.output).contains("invalid UI activation"));
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let ns = RootNamespaces::for_current_account().unwrap();
            let owner = std::sync::Arc::new(RootOwner::open(&root, &ns).unwrap());
            let store = maka_event_log::EventLog::for_root(owner).await.unwrap();
            let launch = store.application_launch().await.unwrap().unwrap();
            let loaded = store
                .load_application(launch.deployment)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                loaded.source.input.instances["apps"].configuration.values["entries"],
                json!([])
            );
            assert!(loaded.source.input.instances["apps"].bindings["pages"].is_empty());
            assert!(
                loaded
                    .packages
                    .iter()
                    .any(|p| p.manifest().id == "example.notebook")
            );
            assert_eq!(
                loaded
                    .source
                    .input
                    .instances
                    .values()
                    .filter(|n| matches!(&n.package, maka_plugin_core::package::PackageSelection::Pinned { package } if package.id == "example.notebook"))
                    .count(),
                1
            );
            let intents = store
                .application_deployment_intents(launch.deployment, 0, 32)
                .await
                .unwrap();
            assert_eq!(
                intents
                    .iter()
                    .filter(
                        |i| ["install", "editComposition"].contains(&i.reference.method.as_str())
                    )
                    .count(),
                3
            );
            store.close().await.unwrap();
        });
        if first {
            std::fs::remove_file(&file).unwrap();
        }
    }
}
