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
use std::{collections::BTreeMap, rc::Rc};
fn input_presenter(secret: bool, dialog: bool) -> (tokio::runtime::Runtime, Presenter) {
    configured_input(secret, dialog, 65536, "reject_controls")
}
fn configured_input(
    secret: bool,
    dialog: bool,
    max_bytes: u32,
    paste: &str,
) -> (tokio::runtime::Runtime, Presenter) {
    let (runtime, mut input, packages) = textarea::editor_fixture();
    let contract = if secret {
        maka_ui_runtime::input::secret_input_contract()
    } else {
        maka_ui_runtime::input::text_input_contract()
    };
    let name = if secret { "SecretInput" } else { "TextInput" };
    let old = serde_json::to_string(&maka_ui_runtime::textarea::textarea_contract()).unwrap();
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
  let mut decl=original.manifest().application.as_ref().unwrap().declaration().clone();
  let entry=decl.entries.get_mut(&EntryTarget::Tui).unwrap();
  if let Some(import)=entry.imports.get_mut("editor"){import.contract=contract.clone();}
  let mut files=BTreeMap::new();
  match &mut entry.implementation {
   Implementation::Native{implementation} if implementation=="maka.ui.textarea"=>{
    *implementation="maka.ui.input".into();let mut text=entry.exports.remove("Textarea").unwrap();text.contract=maka_ui_runtime::input::text_input_contract();let mut secret=text.clone();secret.contract=maka_ui_runtime::input::secret_input_contract();entry.exports.insert("TextInput".into(),text);entry.exports.insert("SecretInput".into(),secret);
   }
   Implementation::JavaScript{module,..}=>{
    if let Some(export)=entry.exports.get_mut("Textarea"){export.contract=contract.clone();}
    let mut source=std::str::from_utf8(original.file(module).unwrap()).unwrap().replace(&old,&new)
      .replace("initialText:'Start',", &format!("maxBytes:{max_bytes},terminalPaste:'{paste}',")).replace("menu:state.brand.menu,", "").replace("softWrap:true,", "")
      .replace("appearance:state.brand.textarea", "appearance:Object.fromEntries(Object.entries(state.brand.textarea).filter(([key])=>key!=='scrollbar'))")
      .replace("on:{changed:'changed',submitted:'submitted',navigated:'navigated'}", "on:{changed:'changed',submitted:'submitted'}")
      .replace("name:'changed',value:event.event", "name:event.name,value:event.event")
      .replace("else if(event.kind==='component'){if(event.name==='navigated'){next.navigation=event.event;next.navigations=(state.navigations??0)+1;}else next.observation=event.event;if(event.name==='submitted')next.submissions=(state.submissions??0)+1;}", "else if(event.kind==='component'){if(event.name==='submitted')next.submits=(next.submits??0)+1;else next.observation=event.event;}");
    if original.manifest().id=="dynamic.app" {
      source=source.replace("delete editor.props.readingViewport;", "delete editor.props.readingViewport;delete editor.props.submitVeto;delete editor.props.submitReject;");
      source.push_str("const submitEvent=app.event;app.event=(state,event,...rest)=>{const submitting=event.kind==='component'&&event.name==='submitted';if(submitting&&state.options?.submitReject)throw Error('submission rejected');const out=submitEvent(state,event,...rest);return out;};");
    }
    if dialog && original.manifest().id=="dynamic.app" {
      source=source.replace("if(event.key==='configure')", "if(event.key==='modalToggle')next.modal=event.event.open;if(event.key==='dialog')next.modal=false;if(event.key==='configure')");
      source.push_str(r#"
        const ordinaryFrame=frame;
        frame=function(state){
          const out=ordinaryFrame(state),background=out.frame.view;
          const original=background.children.find(child=>child.key==='presentation').children[0];
          background.children.push({kind:'button',key:'modalToggle',label:'Commands'});
          out.frame.view={kind:'dialog',key:'dialog',label:'Commands',open:state.modal??false,
            width:28,height:8,padding:0,background_color:0x17171a,border_color:0x888888,
            background,content:{kind:'component',key:'query',binding:'editor',member:0,
              props:{...original.props,command:null,autoFocus:true,accessibilityLabel:'Query'},
              on:{changed:'changed',submitted:'submitted'}}};
          return out;
        };
      "#);
    }
    files.insert(module.clone(),source.into_bytes());
   }
   _=>{}
  }
  files.insert("maka.extension.json".into(),serde_json::to_vec(&json!({"schemaVersion":1,"id":original.manifest().id,"application":decl})).unwrap());
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
        maka_ui_js::execution(Default::default()),
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
    tui.pump_settled().unwrap();
}
fn close(tui: &mut Presenter) {
    tui.surface_closed(&tui.surface(), Ok(())).unwrap();
    assert!(tui.can_shutdown());
    tui.shutdown_settled().unwrap();
}
#[test]
fn input_border_appearance_changes_geometry_without_replacing_text_or_focus() {
    for secret in [false, true] {
        let (_runtime, mut tui) = input_presenter(secret, false);
        draw(&mut tui, 50, 24);
        let focus = tui.focus.clone().unwrap();
        let mut appearance = support::brand_appearance()["textarea"].clone();
        appearance.as_object_mut().unwrap().remove("scrollbar");
        appearance["focusedBorder"] = json!(0x123456ffu32);
        appearance["radius"] = json!(0);
        for border in [1, 0, 1] {
            appearance["borderWidth"] = json!(border);
            dispatch(&mut tui, "configure", json!({"appearance":appearance}));
            let buffer = draw(&mut tui, 50, 24);
            assert!(tui.focus.as_ref().unwrap().same(&focus));
            let area = tui
                .scene
                .iter()
                .find(|target| target.same(&focus))
                .unwrap()
                .area
                .visible();
            assert_eq!(area.height, if border == 0 { 1 } else { 3 });
            if border > 0 {
                assert_eq!(buffer[(area.x, area.y)].symbol(), "┌");
                assert_eq!(
                    buffer[(area.x, area.y)].fg,
                    ratatui::style::Color::Rgb(0x12, 0x34, 0x56)
                );
            }
            tui.input_settled(Event::Paste("x".into())).unwrap();
            draw(&mut tui, 50, 24);
            let owner = editor_owner(&*tui.resources.owners[&focus.mount.id()].state);
            assert!(area.contains(owner.editor.cursor_position().unwrap()));
        }
        assert_eq!(
            editor_owner(&*tui.resources.owners[&focus.mount.id()].state)
                .editor
                .text(),
            "xxx"
        );
        close(&mut tui);
    }
}

#[test]
fn retained_presentation_preserves_input_custody_and_only_successful_flush_hides_geometry() {
    for secret in [false, true] {
        let (_runtime, mut tui) = input_presenter(secret, false);
        draw(&mut tui, 80, 24);
        tui.input_settled(Event::Paste("Draft🙂".into())).unwrap();
        draw(&mut tui, 80, 24);
        let original = observation(&tui);
        let outer = tui
            .application
            .child_mount(&tui.root, "editor")
            .unwrap()
            .clone();
        let mount = tui
            .application
            .child_mount(&outer, "draft")
            .unwrap()
            .clone();
        let identity = tui.resources.owners[&mount.id()].identity.clone();
        let child_revision = tui.application.frame(&mount).unwrap().revision;
        let old_target = tui
            .scene
            .iter()
            .find(|target| target.mount == mount)
            .unwrap()
            .clone();
        let before = tui
            .measure_mount(
                &tui.root,
                80,
                paint::Measurements::default().context(Default::default()),
            )
            .unwrap()
            .1;
        let editor_height = tui
            .measure_mount(
                &outer,
                78,
                paint::Measurements::default().context(Default::default()),
            )
            .unwrap()
            .1;
        dispatch(&mut tui, "configure", json!({"visible":false}));
        assert_eq!(
            tui.application.frame(&mount).unwrap().revision,
            child_revision
        );
        assert_eq!(
            tui.application.frame(&tui.root).unwrap().view.slots().len(),
            1
        );
        assert_eq!(
            tui.application
                .frame(&tui.root)
                .unwrap()
                .view
                .visible_slots()
                .count(),
            0
        );
        assert_eq!(
            before,
            tui.measure_mount(
                &tui.root,
                80,
                paint::Measurements::default().context(Default::default())
            )
            .unwrap()
            .1 + editor_height
                + 1,
            "hidden child occupies neither space nor gap"
        );
        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);
        let pending = tui.paint(area, &mut buffer).unwrap();
        assert!(
            editor_owner(&*tui.resources.owners[&mount.id()].state)
                .presented
                .is_some(),
            "unflushed hiding cannot revoke the last presented geometry"
        );
        tui.present(pending);
        assert!(
            editor_owner(&*tui.resources.owners[&mount.id()].state)
                .presented
                .is_none()
        );
        assert!(
            tui.resources
                .input(&old_target, &Event::Paste("stale".into()))
                .unwrap()
                .is_none()
        );
        tui.input_settled(Event::Paste("hidden".into())).unwrap();
        assert_eq!(observation(&tui), original);
        dispatch(&mut tui, "configure", json!({"visible":true}));
        draw(&mut tui, 80, 24);
        assert!(Rc::ptr_eq(
            &identity,
            &tui.resources.owners[&mount.id()].identity
        ));
        let restored = tui
            .scene
            .iter()
            .find(|target| target.mount == mount)
            .unwrap()
            .clone();
        tui.input_settled(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: restored.area.visible().x + 1,
            row: restored.area.visible().y + 1,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
        // Undo is still owned by the original editor after hide/show.
        tui.input_settled(Event::Key(KeyEvent::new(
            KeyCode::Char('z'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
        assert_eq!(observation(&tui)["snapshot"]["text"]["bytes"], 0);
        tui.input_settled(Event::Paste("Captured while hidden🙂".into()))
            .unwrap();
        let captured = observation(&tui);
        dispatch(&mut tui, "configure", json!({"visible":false}));
        draw(&mut tui, 80, 24);
        dispatch(
            &mut tui,
            "read",
            json!({"end":captured["snapshot"]["text"]["bytes"],"capture":{"kind":"at_revision","owner":captured["snapshot"]["text"]["owner"],"revision":captured["revision"]},"command":{"owner":captured["snapshot"]["text"]["owner"],"sequence":1,"basedOn":captured["revision"],"edit":{"kind":"reset","text":""}}}),
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while tui.application.frame(&tui.root).unwrap().state["outcome"].is_null() {
            assert!(std::time::Instant::now() < deadline);
            tui.pump_settled().unwrap();
            std::thread::yield_now();
        }
        assert_eq!(
            tui.application.frame(&tui.root).unwrap().state["outcome"]["value"]["text"],
            "Captured while hidden🙂"
        );
        assert_eq!(observation(&tui)["receipt"]["sequence"], 1);
        assert_eq!(observation(&tui)["snapshot"]["text"]["bytes"], 0);
        assert_eq!(
            observation(&tui)["snapshot"]["text"]["owner"],
            original["snapshot"]["text"]["owner"]
        );
        assert!(Rc::ptr_eq(
            &identity,
            &tui.resources.owners[&mount.id()].identity
        ));
        close(&mut tui);
    }
}
#[test]
fn native_single_line_filters_raw_paste_masks_and_captures_original_utf8_before_reset() {
    let (_runtime, mut tui) = input_presenter(true, false);
    draw(&mut tui, 80, 24);
    assert_eq!(observation(&tui)["snapshot"]["text"]["bytes"], 0);
    let secret = "密钥🙂e\u{301}".to_owned();
    tui.input_settled(Event::Paste(secret.clone())).unwrap();
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
        tui.input_settled(Event::Paste(invalid.into())).unwrap();
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
        tui.pump_settled().unwrap();
        std::thread::yield_now();
    }
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["outcome"]["value"]["text"],
        secret
    );
    tui.input_settled(Event::Key(KeyEvent::new(
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
    let (_runtime, mut tui) = input_presenter(false, false);
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
    tui.input_settled(Event::Key(KeyEvent::new(
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

fn shortcut_presenter() -> (tokio::runtime::Runtime, Presenter) {
    reserved_shortcut_presenter(false)
}
fn reserved_shortcut_presenter(reserved: bool) -> (tokio::runtime::Runtime, Presenter) {
    let (runtime, mut input, packages) = textarea::editor_fixture();
    let packages = packages.into_iter().map(|original| {
        let declaration = original.manifest().application.as_ref().unwrap().declaration().clone();
        let mut files = BTreeMap::new();
        if let Implementation::JavaScript { module, .. } = &declaration.entries[&EntryTarget::Tui].implementation {
            let mut source = std::str::from_utf8(original.file(module).unwrap()).unwrap().to_owned();
            if original.manifest().id == "dynamic.app" {
                source = source.replace("const next={...state,revision:state.revision+1};", "const next={...state,revision:state.revision+1};if(event.kind==='raw'&&['outer','new','undo'].includes(event.key))next.commands=(state.commands??0)+1;");
                source.push_str("const shortcutFrame=frame;frame=(state)=>{const out=shortcutFrame(state);if(state.options?.empty)out.frame.view.children=[{kind:'text',key:'empty',text:'Empty',color:0}];out.frame.view.shortcuts=[{key:'b',modifiers:['control'],input:'outer',enabled:true},{key:'n',modifiers:['control'],input:'new',enabled:true},{key:'z',modifiers:['control'],input:'undo',enabled:true}];return out;};");
            }
            if original.manifest().id == "dynamic.composer" {
                source = source.replace("event(state,event,props){return", "event(state,event,props){if(event.key==='disable')return frame({...state,revision:state.revision+1,disabled:true},props);if(event.kind==='raw')return frame({...state,revision:state.revision+1,commands:(state.commands??0)+1},props);return");
                source = source.replace("return frame({revision:state.revision+1}", "return frame({...state,revision:state.revision+1}");
                source.push_str("const shortcutFrame=frame;frame=(state,props,events=[])=>{const out=shortcutFrame(state,props,events);if(state.disabled)out.frame.view.props.disabled=true;out.frame.view={kind:'stack',key:'scope',direction:'column',gap:0,padding:0,background:0,shortcuts:[{key:'b',modifiers:['control'],input:'inner',enabled:!props.readOnly},{key:'d',modifiers:['control'],input:'disable',enabled:true}],children:[out.frame.view]};return out;};");
            }
            if reserved { source = source.replace("shortcuts:[{key:'b'", "shortcuts:[{key:'tab',modifiers:[],input:'close',enabled:true},{key:'tab',modifiers:['shift'],input:'close',enabled:true},{key:'c',modifiers:['control'],input:'close',enabled:true},{key:'q',modifiers:['control'],input:'close',enabled:true},{key:'b'"); }
            files.insert(module.clone(), source.into_bytes());
        }
        files.insert("maka.extension.json".into(), serde_json::to_vec(&json!({"schemaVersion":1,"id":original.manifest().id,"application":declaration})).unwrap());
        let package = Package::new(files).unwrap();
        for node in input.instances.values_mut().filter(|node| matches!(&node.package,maka_plugin_core::package::PackageSelection::Pinned{package} if package.id==original.manifest().id)) {
            node.package = package.application_ref().into();
        }
        package
    }).collect();
    let presenter = Presenter::new(
        &input,
        packages,
        runtime.handle().clone(),
        registry(),
        json!({}),
        maka_ui_js::execution(Default::default()),
    )
    .unwrap();
    (runtime, presenter)
}
#[test]
fn shortcuts_bubble_across_mounts_after_editing_and_disabled_scopes_shadow() {
    let (_runtime, mut tui) = shortcut_presenter();
    draw(&mut tui, 80, 24);
    assert!(matches!(
        tui.focus.as_ref().unwrap().kind,
        paint::TargetKind::Native
    ));
    tui.input_settled(Event::Paste("Draft".into())).unwrap();
    draw(&mut tui, 80, 24);
    let child = tui
        .application
        .child_mount(&tui.root, "editor")
        .unwrap()
        .clone();
    tui.input_settled(Event::Key(KeyEvent::new(
        KeyCode::Char('b'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    assert_eq!(tui.application.frame(&child).unwrap().state["commands"], 1);
    assert!(tui.application.frame(&tui.root).unwrap().state["commands"].is_null());
    draw(&mut tui, 80, 24);
    tui.input_settled(Event::Key(KeyEvent::new(
        KeyCode::Char('n'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["commands"],
        1
    );
    draw(&mut tui, 80, 24);
    tui.input_settled(Event::Key(KeyEvent::new(
        KeyCode::Char('z'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["commands"],
        1,
        "native undo consumes before a product declaration"
    );
    dispatch(&mut tui, "configure", json!({"readOnly":true}));
    draw(&mut tui, 80, 24);
    tui.input_settled(Event::Key(KeyEvent::new(
        KeyCode::Char('b'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    assert_eq!(tui.application.frame(&child).unwrap().state["commands"], 1);
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["commands"],
        1,
        "disabled nearest declaration shadows the outer command"
    );
    close(&mut tui);
}
#[test]
fn hidden_parent_revokes_old_shortcut_without_changing_child_revision_or_flushing() {
    let (_runtime, mut tui) = shortcut_presenter();
    draw(&mut tui, 80, 24);
    assert!(matches!(
        tui.focus.as_ref().unwrap().kind,
        paint::TargetKind::Native
    ));
    let child = tui
        .application
        .child_mount(&tui.root, "editor")
        .unwrap()
        .clone();
    let revision = tui.application.frame(&child).unwrap().revision;
    dispatch(&mut tui, "configure", json!({"visible":false}));
    assert_eq!(tui.application.frame(&child).unwrap().revision, revision);
    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    let _unflushed = tui.paint(area, &mut buffer).unwrap();
    tui.input_settled(Event::Key(KeyEvent::new(
        KeyCode::Char('b'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    assert_eq!(tui.application.frame(&child).unwrap().revision, revision);
    assert!(tui.application.frame(&child).unwrap().state["commands"].is_null());
    assert!(
        tui.application.frame(&tui.root).unwrap().state["commands"].is_null(),
        "invalid inner declaration remains a barrier"
    );
    close(&mut tui);
}

#[test]
fn unfocused_root_commands_use_the_successfully_painted_scope() {
    let (_runtime, mut tui) = shortcut_presenter();
    dispatch(&mut tui, "configure", json!({"empty":true}));
    draw(&mut tui, 80, 24);
    assert!(tui.focus.is_none());
    tui.input_settled(Event::Key(KeyEvent::new(
        KeyCode::Char('n'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["commands"],
        1
    );
    close(&mut tui);
}

#[test]
fn disabled_editor_leaves_outer_commands_available_before_another_flush() {
    let (_runtime, mut tui) = shortcut_presenter();
    draw(&mut tui, 80, 24);
    assert!(matches!(
        tui.focus.as_ref().unwrap().kind,
        paint::TargetKind::Native
    ));
    let child = tui
        .application
        .child_mount(&tui.root, "editor")
        .unwrap()
        .clone();
    let revision = tui.application.frame(&tui.root).unwrap().revision;
    tui.application
        .dispatch(
            &child,
            tui.application.frame(&child).unwrap().revision,
            "disable",
            json!({"kind":"click"}),
        )
        .unwrap();
    tui.pump_settled().unwrap();
    assert_eq!(tui.application.frame(&tui.root).unwrap().revision, revision);
    let original = observation(&tui);
    tui.input_settled(Event::Paste("must not edit".into()))
        .unwrap();
    assert_eq!(observation(&tui), original);
    tui.input_settled(Event::Key(KeyEvent::new(
        KeyCode::Char('n'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["commands"],
        1
    );
    close(&mut tui);
}

#[test]
fn reveal_focus_uses_committed_visibility_and_only_successful_native_presentation() {
    for secret in [false, true] {
        let (_runtime, mut tui) = input_presenter(secret, false);
        draw(&mut tui, 80, 24);
        let outer = tui
            .application
            .child_mount(&tui.root, "editor")
            .unwrap()
            .clone();
        let mount = tui
            .application
            .child_mount(&outer, "draft")
            .unwrap()
            .clone();
        let identity = tui.resources.owners[&mount.id()].identity.clone();
        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);
        let older = tui.paint(area, &mut buffer).unwrap();
        dispatch(&mut tui, "configure", json!({"autoFocus":true}));
        tui.sync_native().unwrap();
        tui.present(older);
        let discarded = tui.paint(area, &mut buffer).unwrap();
        assert!(
            discarded
                .targets
                .iter()
                .any(|target| target.mount == mount && target.autofocus)
        );
        // Painting a frame that was never flushed cannot consume its offer.
        tui.focus = None;
        drop(discarded);
        let presented = tui.paint(area, &mut buffer).unwrap();
        tui.present(presented);
        assert_eq!(tui.focus.as_ref().unwrap().mount, mount);
        tui.input_settled(Event::Paste("First密钥🙂".into()))
            .unwrap();
        draw(&mut tui, 80, 24);
        let before = observation(&tui);
        let other = tui
            .scene
            .iter()
            .find(|target| target.mount == tui.root)
            .unwrap()
            .clone();
        tui.focus = Some(other.clone());
        draw(&mut tui, 80, 24);
        assert!(
            tui.focus.as_ref().unwrap().same(&other),
            "stable native render must not steal manual focus"
        );
        dispatch(
            &mut tui,
            "configure",
            json!({"autoFocus":true,"visible":false}),
        );
        draw(&mut tui, 80, 24);
        let child_revision = tui.application.frame(&mount).unwrap().revision;
        dispatch(
            &mut tui,
            "configure",
            json!({"autoFocus":true,"visible":true}),
        );
        assert_eq!(
            tui.application.frame(&mount).unwrap().revision,
            child_revision,
            "ancestor reveal does not change original child props"
        );
        tui.restore_focus = Some(crate::application::dialog::WeakTarget::new(&other));
        draw(&mut tui, 80, 24);
        assert_eq!(tui.focus.as_ref().unwrap().mount, mount);
        draw(&mut tui, 80, 24);
        assert_eq!(
            tui.focus.as_ref().unwrap().mount,
            mount,
            "a modal return displaced by native reveal cannot steal focus next frame"
        );
        assert!(Rc::ptr_eq(
            &identity,
            &tui.resources.owners[&mount.id()].identity
        ));
        assert_eq!(
            observation(&tui),
            before,
            "focus changes preserve original native buffer and selection"
        );
        tui.input_settled(Event::Paste("!".into())).unwrap();
        assert_eq!(
            observation(&tui)["snapshot"]["text"]["bytes"],
            "First密钥🙂!".len()
        );
        // A held scene cannot cross an ancestor hide/show, even though the
        // original input's own props/revision never change.
        dispatch(&mut tui, "configure", json!({"autoFocus":false}));
        draw(&mut tui, 80, 24);
        dispatch(&mut tui, "configure", json!({"autoFocus":true}));
        tui.sync_native().unwrap();
        let held = tui.paint(area, &mut buffer).unwrap();
        let unchanged = tui.application.frame(&mount).unwrap().revision;
        dispatch(
            &mut tui,
            "configure",
            json!({"autoFocus":true,"visible":false}),
        );
        dispatch(
            &mut tui,
            "configure",
            json!({"autoFocus":true,"visible":true}),
        );
        assert_eq!(tui.application.frame(&mount).unwrap().revision, unchanged);
        tui.focus = Some(other.clone());
        tui.present(held);
        assert_ne!(
            tui.focus.as_ref().unwrap().mount,
            mount,
            "held offer cannot cross changed original route frames"
        );
        let fresh = tui.paint(area, &mut buffer).unwrap();
        let twin = tui.paint(area, &mut buffer).unwrap();
        assert!(
            fresh
                .targets
                .iter()
                .any(|target| target.mount == mount && target.autofocus)
        );
        tui.present(fresh);
        assert_eq!(tui.focus.as_ref().unwrap().mount, mount);
        tui.focus = Some(other.clone());
        tui.present(twin);
        assert_ne!(
            tui.focus.as_ref().unwrap().mount,
            mount,
            "the same already consumed offer cannot steal focus in a second held scene"
        );
        // Viewport clipping alone cannot re-arm a structural reveal offer.
        draw(&mut tui, 80, 1);
        let restored = tui.paint(area, &mut buffer).unwrap();
        assert!(
            !restored
                .targets
                .iter()
                .any(|target| target.mount == mount && target.autofocus)
        );
        tui.focus = Some(other);
        tui.present(restored);
        assert_ne!(tui.focus.as_ref().unwrap().mount, mount);
        close(&mut tui);
    }
}

#[test]
fn single_line_arrows_leave_list_navigation_to_the_original_outer_scope() {
    for secret in [false, true] {
        let (_runtime, mut tui) = input_presenter(secret, false);
        draw(&mut tui, 80, 24);
        tui.input_settled(Event::Paste("query".into())).unwrap();
        draw(&mut tui, 80, 24);
        let target = tui.focus.clone().unwrap();
        let before = observation(&tui);
        for code in [
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::PageUp,
            KeyCode::PageDown,
        ] {
            let result = tui.resources.input(&target, &key(code)).unwrap();
            assert!(match code {
                KeyCode::Up | KeyCode::Down => matches!(result, Some(WidgetInput::Navigate)),
                _ => matches!(result, Some(WidgetInput::Unhandled)),
            });
        }
        assert_eq!(observation(&tui), before);
        close(&mut tui);
    }
}

#[test]
fn dialog_js_composition_keeps_original_editor_and_routes_first_text_paste_submit_and_return() {
    let (_runtime, mut tui) = input_presenter(false, true);
    draw(&mut tui, 50, 20);
    let original = tui
        .application
        .child_mount(&tui.root, "editor")
        .unwrap()
        .clone();
    let original = tui
        .application
        .child_mount(&original, "draft")
        .unwrap()
        .clone();
    let query = tui
        .application
        .child_mount(&tui.root, "query")
        .unwrap()
        .clone();
    let query = tui
        .application
        .child_mount(&query, "draft")
        .unwrap()
        .clone();
    let text = |tui: &Presenter, mount: &Mount| {
        editor_owner(&*tui.resources.owners[&mount.id()].state)
            .editor
            .text()
            .to_owned()
    };
    tui.input_settled(key(KeyCode::Char('x'))).unwrap();
    assert_eq!(text(&tui, &original), "x");
    let background_focus = tui.focus.clone().unwrap();
    dispatch(&mut tui, "modalToggle", json!({"open":true}));
    assert!(!tui.physical_admitted());
    tui.input_settled(key(KeyCode::Char('z'))).unwrap();
    tui.input_settled(Event::Paste("blocked".into())).unwrap();
    assert_eq!(text(&tui, &original), "x");
    assert_eq!(text(&tui, &query), "");
    draw(&mut tui, 50, 20);
    assert_eq!(tui.focus.as_ref().unwrap().mount, query);
    tui.input_settled(key(KeyCode::Char('界'))).unwrap();
    tui.input_settled(Event::Paste(" ab".into())).unwrap();
    assert_eq!(text(&tui, &query), "界 ab");
    assert_eq!(text(&tui, &original), "x");
    draw(&mut tui, 50, 20);
    tui.input_settled(key(KeyCode::Enter)).unwrap();
    assert!(
        tui.application.frame(&tui.root).unwrap().state["modal"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["submits"],
        1
    );
    draw(&mut tui, 50, 20);
    tui.input_settled(key(KeyCode::Esc)).unwrap();
    assert!(
        !tui.application.frame(&tui.root).unwrap().state["modal"]
            .as_bool()
            .unwrap()
    );
    assert!(!tui.physical_admitted());
    tui.input_settled(Event::Paste("blocked".into())).unwrap();
    assert_eq!(text(&tui, &original), "x");
    draw(&mut tui, 50, 20);
    assert!(tui.focus.as_ref().unwrap().same(&background_focus));
    tui.input_settled(key(KeyCode::Char('y'))).unwrap();
    assert_eq!(text(&tui, &original), "xy");
    assert_eq!(text(&tui, &query), "界 ab");
    close(&mut tui);
}

#[test]
fn bounded_query_paste_uses_one_native_edit_and_capacity_survives_commands_and_undo() {
    let (_runtime, mut tui) = configured_input(false, false, 512, "spaces");
    draw(&mut tui, 80, 24);
    tui.input_settled(Event::Paste("中\r\n😀\tquery\rnext\nlast".into()))
        .unwrap();
    let observed = observation(&tui);
    let original = observed["snapshot"]["text"]["owner"].clone();
    let native = tui.focus.as_ref().unwrap().mount.clone();
    let text = |tui: &Presenter| {
        editor_owner(&*tui.resources.owners[&native.id()].state)
            .editor
            .text()
            .to_owned()
    };
    assert_eq!(text(&tui), "中 😀 query next last");
    tui.input_settled(Event::Key(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::SHIFT,
    )))
    .unwrap();
    assert!(tui.application.frame(&tui.root).unwrap().state["submits"].is_null());
    tui.input_settled(Event::Key(KeyEvent::new(
        KeyCode::Char('z'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    assert_eq!(text(&tui), "");
    let full = format!("{}ab", "中".repeat(170));
    tui.input_settled(Event::Paste(full.clone())).unwrap();
    assert_eq!(observation(&tui)["snapshot"]["text"]["bytes"], 512);
    tui.input_settled(Event::Paste("x".into())).unwrap();
    assert_eq!(text(&tui), full);
    let rev = observation(&tui)["revision"].clone();
    dispatch(
        &mut tui,
        "configure",
        json!({"command":{"owner":original,"sequence":1,"basedOn":rev,"edit":{"kind":"replace","text":"x".repeat(513)}}}),
    );
    assert_eq!(
        observation(&tui)["receipt"]["outcome"]["reason"],
        "representation"
    );
    assert_eq!(text(&tui), full);
    assert_eq!(observation(&tui)["snapshot"]["text"]["owner"], original);
    close(&mut tui);
}

#[test]
fn submit_copy_requires_physical_enter_current_query_and_accepted_original_callback() {
    let (_runtime, mut tui) = input_presenter(false, false);
    draw(&mut tui, 80, 24);
    tui.input_settled(Event::Paste("file".into())).unwrap();
    let basis = observation(&tui)["snapshot"]["text"].clone();
    let copy = json!({"basis":basis,"text":"/remote/原文 file.rs"});
    dispatch(&mut tui, "configure", json!({"submitCopy":copy}));
    draw(&mut tui, 80, 24);
    let native = tui.focus.as_ref().unwrap().mount.clone();
    let revision = tui.application.frame(&native).unwrap().revision;
    tui.application
        .dispatch(&native, revision, "submit", json!({}))
        .unwrap();
    let mut clipboard = Vec::new();
    assert!(!tui.flush_clipboard(&mut clipboard).unwrap());
    draw(&mut tui, 80, 24);
    tui.input_settled(key(KeyCode::Enter)).unwrap();
    assert!(tui.flush_clipboard(&mut clipboard).unwrap());
    let mut expected = Vec::new();
    crate::terminal::copy(&mut expected, "/remote/原文 file.rs").unwrap();
    assert_eq!(clipboard, expected);
    assert!(!tui.flush_clipboard(&mut clipboard).unwrap());

    dispatch(
        &mut tui,
        "configure",
        json!({"submitCopy":null,"autoFocus":true}),
    );
    draw(&mut tui, 80, 24);
    tui.input_settled(key(KeyCode::Enter)).unwrap();
    assert!(!tui.flush_clipboard(&mut clipboard).unwrap());
    for option in ["disabled", "readOnly"] {
        dispatch(
            &mut tui,
            "configure",
            json!({"submitCopy":copy,"autoFocus":true,option:true}),
        );
        draw(&mut tui, 80, 24);
        tui.input_settled(key(KeyCode::Enter)).unwrap();
        assert!(!tui.flush_clipboard(&mut clipboard).unwrap());
    }
    dispatch(
        &mut tui,
        "configure",
        json!({"submitCopy":copy,"autoFocus":true}),
    );
    draw(&mut tui, 80, 24);
    tui.input_settled(Event::Paste(" changed".into())).unwrap();
    draw(&mut tui, 80, 24);
    let submitted = tui.application.frame(&tui.root).unwrap().state["submits"].clone();
    tui.input_settled(key(KeyCode::Enter)).unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["submits"],
        submitted
    );
    assert!(!tui.flush_clipboard(&mut clipboard).unwrap());
    assert_eq!(clipboard, expected);
    let rejected_copy =
        json!({"basis":observation(&tui)["snapshot"]["text"],"text":"/remote/rejected copy"});
    dispatch(
        &mut tui,
        "configure",
        json!({"submitCopy":rejected_copy,"submitReject":true,"autoFocus":true}),
    );
    draw(&mut tui, 80, 24);
    assert!(tui.input_settled(key(KeyCode::Enter)).is_err());
    assert!(!tui.flush_clipboard(&mut clipboard).unwrap());
    close(&mut tui);
}

#[test]
fn declared_tab_and_interrupt_keep_original_focus_and_cannot_run_after_hiding() {
    let (_runtime, mut tui) = reserved_shortcut_presenter(true);
    draw(&mut tui, 80, 24);
    let focus = tui.focus.clone().unwrap();
    let child = tui
        .application
        .child_mount(&tui.root, "editor")
        .unwrap()
        .clone();
    for (code, modifiers) in [
        (KeyCode::Tab, KeyModifiers::NONE),
        (KeyCode::BackTab, KeyModifiers::NONE),
        (KeyCode::Tab, KeyModifiers::SHIFT),
    ] {
        tui.input_settled(Event::Key(KeyEvent::new(code, modifiers)))
            .unwrap();
        assert!(tui.focus.as_ref().unwrap().same(&focus));
        draw(&mut tui, 80, 24);
    }
    let interrupt = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert!(!tui.captures_keyboard().unwrap());
    assert!(tui.dispatch_focused_shortcut(&interrupt).unwrap());
    tui.settle().unwrap();
    assert_eq!(tui.application.frame(&child).unwrap().state["commands"], 3);
    // The original nonempty textarea consumes the first interrupt as an edit.
    draw(&mut tui, 80, 24);
    assert!(tui.dispatch_focused_shortcut(&interrupt).unwrap());
    tui.settle().unwrap();
    assert_eq!(tui.application.frame(&child).unwrap().state["commands"], 4);
    draw(&mut tui, 80, 24);
    let quit = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
    assert!(tui.dispatch_focused_shortcut(&quit).unwrap());
    tui.settle().unwrap();
    assert_eq!(tui.application.frame(&child).unwrap().state["commands"], 5);
    dispatch(&mut tui, "configure", json!({"visible":false}));
    assert!(!tui.dispatch_focused_shortcut(&interrupt).unwrap());
    assert!(!tui.dispatch_focused_shortcut(&quit).unwrap());
    assert_eq!(tui.application.frame(&child).unwrap().state["commands"], 5);
    close(&mut tui);
}

#[test]
fn closing_a_surface_waits_for_its_native_child_without_reporting_cleanup_failure() {
    let (_runtime, mut tui) = input_presenter(false, false);
    draw(&mut tui, 40, 12);
    let outer = tui.application.child_mount(&tui.root, "editor").unwrap();
    let child = tui.application.child_mount(outer, "draft").unwrap().clone();
    let mut original = tui.resources.owners[&child.id()]
        .resources
        .reserve()
        .unwrap();
    original.start();
    tui.retire().unwrap();
    tui.surface_closed(&tui.surface(), Ok(())).unwrap();
    assert!(!tui.can_shutdown());
    assert!(tui.application.frame(&child).is_ok());
    assert!(tui.application.frame(&tui.root).is_ok());
    tui.pump_settled().unwrap();
    assert!(!tui.can_shutdown());
    original.complete(Ok(()));
    tui.pump_settled().unwrap();
    assert!(tui.can_shutdown());
    tui.shutdown_settled().unwrap();
}

#[test]
fn ordinary_inputs_ctrl_v_normalizes_text_and_keeps_secret_menu_private() {
    for secret in [false, true] {
        let (_runtime, mut tui) = input_presenter(secret, false);
        dispatch(
            &mut tui,
            "configure",
            json!({"menu":support::brand_appearance()["menu"],"terminalPaste":"spaces"}),
        );
        draw(&mut tui, 50, 24);
        tui.input_settled(Event::Paste("replace me".into()))
            .unwrap();
        tui.input_settled(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
        let target = tui.focus.clone().unwrap();
        tui.resources
            .owners
            .get_mut(&target.mount.id())
            .unwrap()
            .state
            .downcast_mut::<super::super::editor::Owner>()
            .unwrap()
            .clipboard
            .as_mut()
            .unwrap()
            .set_clipboard_text("one\r\ntwo\t三".into());
        tui.input_settled(Event::Key(KeyEvent::new(
            KeyCode::Char('v'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            tui.pump_settled().unwrap();
            let owner = tui.resources.owners[&target.mount.id()]
                .state
                .downcast_ref::<super::super::editor::Owner>()
                .unwrap();
            if owner.editor.text() == "one two 三" {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        tui.input_settled(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
        draw(&mut tui, 50, 24);
        let (_, _, enabled) = tui.resources.editing_menu(&target).unwrap().unwrap();
        assert_eq!(enabled, [!secret, !secret, true, true]);
        dispatch(
            &mut tui,
            "configure",
            json!({"menu":support::brand_appearance()["menu"],"readOnly":true}),
        );
        draw(&mut tui, 50, 24);
        let target = tui.focus.clone().unwrap();
        let (_, _, enabled) = tui.resources.editing_menu(&target).unwrap().unwrap();
        assert_eq!(enabled, [!secret, false, false, true]);
        tui.input_settled(Event::Key(KeyEvent::new(
            KeyCode::Char('v'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
        assert_eq!(
            tui.resources.owners[&target.mount.id()]
                .state
                .downcast_ref::<super::super::editor::Owner>()
                .unwrap()
                .editor
                .text(),
            "one two 三"
        );
        close(&mut tui);
    }
}

#[test]
fn physical_cursor_tracks_unicode_input_and_the_foreground_dialog() {
    use ratatui::layout::Position;
    for secret in [false, true] {
        let (_runtime, mut tui) = input_presenter(secret, true);
        draw(&mut tui, 50, 24);
        tui.input_settled(Event::Paste("中🙂".into())).unwrap();
        let focus = tui.focus.clone().unwrap();
        let area = tui
            .scene
            .iter()
            .find(|target| target.same(&focus))
            .unwrap()
            .area
            .visible();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 50, 24));
        let pending = tui.paint(buffer.area, &mut buffer).unwrap();
        assert_eq!(
            pending.cursor_position(),
            Some(Position::new(
                area.x + 1 + if secret { 2 } else { 4 },
                area.y + 1
            ))
        );
        tui.present(pending);
        dispatch(&mut tui, "modalToggle", json!({"open":true}));
        let pending = tui.paint(buffer.area, &mut buffer).unwrap();
        assert_eq!(
            pending.cursor_position(),
            None,
            "background editor must not anchor the new modal's IME"
        );
        tui.present(pending);
        draw(&mut tui, 50, 24);
        let query = tui.focus.clone().unwrap();
        assert!(!query.same(&focus));
        let area = tui
            .scene
            .iter()
            .find(|target| target.same(&query))
            .unwrap()
            .area
            .visible();
        let pending = tui.paint(buffer.area, &mut buffer).unwrap();
        assert_eq!(
            pending.cursor_position(),
            Some(Position::new(area.x + 1, area.y + 1))
        );
        close(&mut tui);
    }
}
