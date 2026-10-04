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
use maka_plugin_core::{
    composition::Target,
    package::{EntryTarget, Implementation},
};
use std::collections::BTreeMap;
fn input_presenter(secret: bool) -> (tokio::runtime::Runtime, Presenter) {
    let (runtime, mut input, packages) = textarea::editor_fixture();
    let contract = if secret {
        maka_ui::input::secret_input_contract()
    } else {
        maka_ui::input::text_input_contract()
    };
    let name = if secret { "SecretInput" } else { "TextInput" };
    let old = serde_json::to_string(&maka_ui::textarea::textarea_contract()).unwrap();
    let new = serde_json::to_string(&contract).unwrap();
    for node in input.instances.values_mut() {
        for targets in node.bindings.values_mut() {
            for target in targets {
                if let Target::Instance { path, export } = target
                    && path == &vec!["editor".to_owned()]
                {
                    *export = name.into();
                }
            }
        }
    }
    let packages=packages.into_iter().map(|original|{
  let mut decl=original.manifest().application.clone().unwrap();
  let entry=decl.entries.get_mut(&EntryTarget::Tui).unwrap();
  if let Some(import)=entry.imports.get_mut("editor"){import.contract=contract.clone();}
  let mut files=BTreeMap::new();
  match &mut entry.implementation {
   Implementation::Native{implementation} if implementation=="maka.ui.textarea"=>{
    *implementation="maka.ui.input".into();let mut text=entry.exports.remove("Textarea").unwrap();text.contract=maka_ui::input::text_input_contract();let mut secret=text.clone();secret.contract=maka_ui::input::secret_input_contract();entry.exports.insert("TextInput".into(),text);entry.exports.insert("SecretInput".into(),secret);
   }
   Implementation::JavaScript{module,..}=>{
    if let Some(export)=entry.exports.get_mut("Textarea"){export.contract=contract.clone();}
    let source=std::str::from_utf8(original.file(module).unwrap()).unwrap().replace(&old,&new)
      .replace("initialText:'Start',", "").replace("menu:state.brand.menu,", "").replace("softWrap:true,", "")
      .replace("appearance:state.brand.textarea", "appearance:Object.fromEntries(Object.entries(state.brand.textarea).filter(([key])=>key!=='scrollbar'))")
      .replace("on:{changed:'changed'}", "on:{changed:'changed',submitted:'submitted'}")
      .replace("name:'changed',value:event.event", "name:event.name,value:event.event")
      .replace("else if(event.kind==='component')next.observation=event.event;", "else if(event.kind==='component'){if(event.name==='submitted')next.submits=(next.submits??0)+1;else next.observation=event.event;}");
    files.insert(module.clone(),source.into_bytes());
   }
   _=>{}
  }
  files.insert("maka.extension.json".into(),serde_json::to_vec(&json!({"schemaVersion":2,"id":original.manifest().id,"application":decl})).unwrap());
  let package=Package::new(files).unwrap();
  for node in input.instances.values_mut().filter(|node|matches!(&node.package,maka_plugin_core::package::PackageSelection::Pinned{package} if package.id==original.manifest().id)){node.package=package.application_ref().into();}
  package
 }).collect();
    let presenter = Presenter::new(
        &input,
        packages,
        runtime.handle().clone(),
        registry(),
        json!({}),
    )
    .unwrap();
    (runtime, presenter)
}
fn observation(tui: &Presenter) -> Value {
    tui.application.frame(&tui.root).unwrap().state["observation"].clone()
}
fn dispatch(tui: &mut Presenter, key: &str, value: Value) {
    let rev = tui.application.frame(&tui.root).unwrap().revision;
    tui.application
        .dispatch(&tui.root, rev, key, value)
        .unwrap();
    tui.pump().unwrap();
}
fn close(tui: &mut Presenter) {
    tui.surface_closed(Ok(())).unwrap();
    assert!(tui.can_shutdown());
    tui.shutdown().unwrap();
}
#[test]
fn native_single_line_filters_raw_paste_masks_and_captures_original_utf8_before_reset() {
    let (_runtime, mut tui) = input_presenter(true);
    draw(&mut tui, 80, 24);
    assert_eq!(observation(&tui)["snapshot"]["text"]["bytes"], 0);
    let secret = "密钥🙂e\u{301}".to_owned();
    tui.input(Event::Paste(secret.clone())).unwrap();
    let observed = observation(&tui);
    assert_eq!(observed["snapshot"]["text"]["bytes"], secret.len());
    assert!(!observed.to_string().contains(&secret));
    let buffer = draw(&mut tui, 80, 24);
    let rendered = buffer
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(!rendered.contains(&secret));
    assert!(rendered.contains('•'));
    for invalid in ["changed\nkey", "changed\rkey", "changed\tkey"] {
        tui.input(Event::Paste(invalid.into())).unwrap();
        assert_eq!(observation(&tui), observed);
    }
    let original = observed["snapshot"]["text"]["owner"].clone();
    let rev = observed["revision"].clone();
    dispatch(
        &mut tui,
        "read",
        json!({"end":secret.len(),"capture":{"kind":"at_revision","owner":original,"revision":rev},"command":{"owner":original,"sequence":1,"basedOn":rev,"edit":{"kind":"reset","text":""}}}),
    );
    assert_eq!(observation(&tui)["snapshot"]["text"]["bytes"], 0);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while tui.application.frame(&tui.root).unwrap().state["outcome"].is_null() {
        assert!(std::time::Instant::now() < deadline);
        tui.pump().unwrap();
        std::thread::yield_now();
    }
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["outcome"]["value"]["text"],
        secret
    );
    tui.input(Event::Key(KeyEvent::new(
        KeyCode::Char('z'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    assert_eq!(
        observation(&tui)["snapshot"]["text"]["bytes"],
        0,
        "Reset must clear secret history"
    );
    close(&mut tui);
}
#[test]
fn submit_preserves_retryable_reset_receipt_and_next_command_settles() {
    let (_runtime, mut tui) = input_presenter(false);
    draw(&mut tui, 80, 24);
    let original = observation(&tui)["snapshot"]["text"]["owner"].clone();
    dispatch(&mut tui, "reject", json!({"reject":true}));
    dispatch(
        &mut tui,
        "configure",
        json!({"command":{"owner":original,"sequence":1,"basedOn":0,"edit":{"kind":"reset","text":""}}}),
    );
    let outer = tui.application.child_mount(&tui.root, "editor").unwrap();
    let editor = tui.application.child_mount(outer, "draft").unwrap().clone();
    assert_eq!(
        tui.application.frame(&editor).unwrap().state["pending"],
        true
    );
    tui.input(Event::Key(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::NONE,
    )))
    .unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["submits"],
        1
    );
    assert_eq!(
        tui.application.frame(&editor).unwrap().state["pending"],
        true
    );
    dispatch(&mut tui, "reject", json!({"reject":false}));
    assert_eq!(
        tui.application.frame(&editor).unwrap().state["pending"],
        false
    );
    dispatch(
        &mut tui,
        "configure",
        json!({"command":{"owner":original,"sequence":2,"basedOn":0,"edit":{"kind":"replace","text":"next"}}}),
    );
    assert_eq!(observation(&tui)["snapshot"]["text"]["bytes"], 4);
    assert_eq!(observation(&tui)["receipt"]["sequence"], 2);
    close(&mut tui);
}
