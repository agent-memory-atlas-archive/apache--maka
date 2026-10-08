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
use maka_plugin_core::package::{EntryTarget, Implementation};
use std::collections::BTreeMap;

pub(super) fn editor_fixture() -> (tokio::runtime::Runtime, Input, Vec<Package>) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_time()
        .build()
        .unwrap();
    let (mut input, packages) = support::resource_fixture();
    input.environment.presentation = Some(PresentationTarget::Tui);
    let packages = packages.into_iter().map(|original| {
        let mut declaration = original.manifest().application.clone().unwrap();
        let mut entry = declaration.entries.remove(&EntryTarget::Desktop).unwrap();
        let mut files = BTreeMap::new();
        match &mut entry.implementation {
            Implementation::Native {implementation} if implementation == "maka.ui.surface.desktop" => {
                *implementation = "maka.ui.surface.tui".into();
                entry.exports.get_mut("Surface").unwrap().contract = maka_ui::surface_contract(PresentationTarget::Tui);
            }
            Implementation::JavaScript {module, ..} => {
                if entry.scene.is_some() { entry.scene = Some(support::scene::language(PresentationTarget::Tui)); }
                if let Some(surface) = entry.imports.get_mut("surface") { surface.contract = maka_ui::surface_contract(PresentationTarget::Tui); }
                let mut source = std::str::from_utf8(original.file(module).unwrap()).unwrap()
                    .replace("{kind:'window',binding:'surface',title:'Immutable draft',width:640,height:480}", "{kind:'terminal',binding:'surface'}")
                    .replace("padding:16,gap:12", "padding:1,gap:1");
                if original.manifest().id == "dynamic.app" {
                    // The capture result remains complete in workflow state;
                    // its disposable status label must obey the scene budget.
                    source = source.replace("state.outcome?.value?.text??", "state.outcome?.value?.text?.slice(0,2048)??");
                    source = source.replace("if(event.kind==='action')", "if(event.key==='reject'){next.reject=event.event.reject;next.observation=null;}if(event.key==='configure'){next.options=event.event;}if(event.kind==='action')")
                        .replace("{kind:'button',key:'remove'", "{kind:'button',key:'reject',label:'Reject observation'},{kind:'button',key:'configure',label:'Configure editor'},{kind:'button',key:'remove'");
                    source.push_str("const normalFrame=frame;frame=(state)=>{const out=normalFrame(state);const editor=out.frame.view.children.find(node=>node.key==='editor');if(editor){editor.props={...editor.props,...state.options};delete editor.props.visible;delete editor.props.prefix;delete editor.props.readingViewport;}out.frame.view.children=out.frame.view.children.map(node=>node.key==='editor'?{kind:'stack',key:'presentation',direction:'column',gap:0,padding:0,background:out.frame.view.background,visible:state.options?.visible!==false,children:[node]}:node);if(state.options?.prefix)out.frame.view.children.unshift({kind:'text',key:'prefix',text:state.options.prefix,color:0xffffff});if(state.options?.readingViewport){const pane=out.frame.view.children.find(node=>node.key==='presentation');pane.scroll='manual';pane.layout={basis:6};pane.children.push({kind:'text',key:'long-tail',text:'tail\\n'.repeat(30),color:0xffffff});}if(state.reject&&state.observation)out.frame.view={kind:'invalid'};return out;};");
                }
                source = source.replace("on:{changed:'changed'}", "on:{changed:'changed',submitted:'submitted'}")
                    .replace("name:'changed',value:event.event", "name:event.name,value:event.event")
                    .replace("else if(event.kind==='component')next.observation=event.event;", "else if(event.kind==='component'){next.observation=event.event;if(event.name==='submitted')next.submissions=(state.submissions??0)+1;}");
                files.insert(module.clone(), source.into_bytes());
            }
            _ => {}
        }
        declaration.entries.insert(EntryTarget::Tui, entry);
        files.insert("maka.extension.json".into(), serde_json::to_vec(&json!({"schemaVersion":2,"id":original.manifest().id,"application":declaration})).unwrap());
        let package = Package::new(files).unwrap();
        for node in input.instances.values_mut().filter(|node| matches!(&node.package,maka_plugin_core::package::PackageSelection::Pinned {package} if package.id == original.manifest().id)) {
            node.package = package.application_ref().into();
        }
        package
    }).collect();
    (runtime, input, packages)
}
fn presenter() -> (tokio::runtime::Runtime, Presenter) {
    let (runtime, input, packages) = editor_fixture();
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

#[test]
fn intrinsic_dialog_editor_grows_shrinks_and_keeps_the_real_caret_and_controls_visible() {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    let field = json!({"kind":"component","key":"editor","binding":"editor","member":0,"on":{},"props":{
        "appearance":support::brand_appearance()["textarea"],"accessibilityLabel":"Message",
        "readOnly":false,"disabled":false,"softWrap":true,"terminalRows":{"min":3,"max":10},
        "tabIndex":0,"autoFocus":true,"initialText":""}});
    let source = Rc::new(RefCell::new(json!({
        "kind":"dialog","key":"dialog","label":"Edit queued message","open":true,
        "width":72,"height":20,"padding":1,"background_color":0,"border_color":0xffffff,
        "background":{"kind":"button","key":"outside","label":"Outside"},
        "content":{"kind":"stack","key":"content","direction":"column","gap":0,"padding":0,"background":0,
          "children":[{"kind":"stack","key":"form","direction":"column","gap":1,"padding":0,"background":0,"layout":{"grow":1},
            "children":[
              {"kind":"stack","key":"field","direction":"column","gap":0,"padding":0,"background":0,"layout":{"shrink":true},
                "children":[field]},
              {"kind":"text","key":"hint","text":"Ctrl+S to save","color":0xffffff},
              {"kind":"button","key":"back","label":"Save"}
            ]}]
        }
    })));
    let (_runtime, mut tui) = layout::native(
        layout::View::InteractiveScene(source),
        Rc::new(Cell::new(true)),
    );
    draw(&mut tui, 80, 30);
    let mount = tui
        .application
        .child_mount(&tui.root, "editor")
        .unwrap()
        .clone();
    let geometry = |tui: &Presenter| {
        tui.scene
            .iter()
            .find(|target| target.mount == mount)
            .unwrap()
            .area
    };
    let editor = |tui: &Presenter| {
        let owner = tui.resources.owners[&mount.id()]
            .state
            .downcast_ref::<super::super::editor::Owner>()
            .unwrap();
        (
            owner.editor.text().to_owned(),
            owner.editor.cursor_position(),
        )
    };
    assert_eq!(geometry(&tui).height, 5); // Original minimum three rows plus inset.
    let focus = tui.focus.clone().unwrap();
    assert_eq!(focus.mount, mount);
    tui.input(Event::Paste(
        (0..20)
            .map(|n| format!("line-{n:02}"))
            .collect::<Vec<_>>()
            .join("\n"),
    ))
    .unwrap();
    draw(&mut tui, 80, 30);
    assert_eq!(geometry(&tui).height, 12); // Original maximum ten rows plus inset.
    for (width, height) in [(80, 18), (30, 18), (80, 30)] {
        let buffer = draw(&mut tui, width, height);
        let area = geometry(&tui);
        assert_eq!(area.height, u32::from(area.visible().height));
        let (_, caret) = editor(&tui);
        let caret = caret.expect("last line remains visible inside the resized editor");
        assert!(area.visible().contains(caret));
        assert!(
            buffer
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .contains("line-19")
        );
        assert!(
            tui.scene
                .iter()
                .any(|target| target.key == "back" && !target.area.is_empty())
        );
        assert!(tui.focus.as_ref().unwrap().same(&focus));
        tui.input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: caret.x,
            row: caret.y,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
        tui.input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: caret.x,
            row: caret.y,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
        tui.input(Event::Paste("!".into())).unwrap();
        assert!(editor(&tui).0.ends_with('!'));
    }
    // Width changes use the original soft-wrap algorithm, including wide glyphs.
    tui.input(Event::Key(KeyEvent::new(
        KeyCode::Char('a'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    tui.input(Event::Paste("中".repeat(100))).unwrap();
    draw(&mut tui, 80, 30);
    assert_eq!(geometry(&tui).height, 6);
    draw(&mut tui, 30, 30);
    assert_eq!(geometry(&tui).height, 12);
    assert!(editor(&tui).1.is_some());
    tui.input(Event::Key(KeyEvent::new(
        KeyCode::Char('a'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    tui.input(Event::Paste("short".into())).unwrap();
    draw(&mut tui, 80, 30);
    assert_eq!(geometry(&tui).height, 5);
    assert_eq!(editor(&tui).0, "short");
    tui.input(key(KeyCode::Tab)).unwrap();
    assert_eq!(tui.focus.as_ref().unwrap().key, "back");
    tui.input(key(KeyCode::Enter)).unwrap();
    assert_eq!(tui.application.frame(&tui.root).unwrap().state["count"], 1);
    close(&mut tui);
}

#[test]
fn viewport_row_cap_uses_each_paint_without_replacing_the_native_draft_or_focus() {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    let source = Rc::new(RefCell::new(
        json!({"kind":"stack","key":"root","direction":"column","gap":0,"padding":0,"background":0,"children":[
            {"kind":"component","key":"editor","binding":"editor","member":0,"on":{},"props":{
                "appearance":support::brand_appearance()["textarea"],"accessibilityLabel":"Composer","readOnly":false,
                "disabled":false,"softWrap":true,"tabIndex":0,"autoFocus":true,"initialText":"",
                "terminalRows":{"min":1,"max":8,"maxViewportFraction":1.0/3.0}}},
            {"kind":"button","key":"back","label":"Send"}
        ]}),
    ));
    let (_runtime, mut tui) = layout::native(
        layout::View::InteractiveScene(source),
        Rc::new(Cell::new(true)),
    );
    draw(&mut tui, 80, 30);
    let focus = tui.focus.clone().unwrap();
    let mount = focus.mount.clone();
    let height = |tui: &Presenter| {
        tui.scene
            .iter()
            .find(|target| target.mount == mount)
            .unwrap()
            .area
            .height
    };
    assert_eq!(height(&tui), 3);
    tui.input(Event::Paste(
        (0..20)
            .map(|n| format!("line-{n:02}"))
            .collect::<Vec<_>>()
            .join("\n"),
    ))
    .unwrap();
    for (width, viewport, expected) in [(80, 30, 10), (25, 18, 8), (40, 12, 6), (80, 30, 10)] {
        let revision = tui.application.frame(&mount).unwrap().revision;
        let buffer = draw(&mut tui, width, viewport);
        assert_eq!(height(&tui), expected);
        assert_eq!(tui.application.frame(&mount).unwrap().revision, revision);
        assert!(tui.focus.as_ref().unwrap().same(&focus));
        assert!(
            buffer
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .contains("line-19")
        );
        let caret = tui.resources.owners[&mount.id()]
            .state
            .downcast_ref::<super::super::editor::Owner>()
            .unwrap()
            .editor
            .cursor_position()
            .unwrap();
        tui.input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: caret.x,
            row: caret.y,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
        tui.input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: caret.x,
            row: caret.y,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
        tui.input(Event::Paste("!".into())).unwrap();
        assert!(
            tui.resources.owners[&mount.id()]
                .state
                .downcast_ref::<super::super::editor::Owner>()
                .unwrap()
                .editor
                .text()
                .ends_with('!')
        );
    }
    tui.input(Event::Key(KeyEvent::new(
        KeyCode::Char('a'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    tui.input(Event::Paste("short".into())).unwrap();
    draw(&mut tui, 40, 12);
    assert_eq!(height(&tui), 3);
    close(&mut tui);
}

#[test]
fn composer_boundary_shares_focus_and_keeps_the_borderless_editor_at_its_actual_cells() {
    use ratatui::style::Color;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    let mut appearance = support::brand_appearance()["textarea"].clone();
    appearance["borderWidth"] = json!(0);
    let field = json!({"kind":"component","key":"editor","binding":"editor","member":0,"on":{},"props":{
        "appearance":appearance,"accessibilityLabel":"Composer","readOnly":false,"disabled":false,"softWrap":true,
        "tabIndex":0,"autoFocus":true,"initialText":"","terminalRows":{"min":1,"max":8,"maxViewportFraction":1.0/3.0}}});
    let composer = json!({"kind":"stack","key":"composer","direction":"row","gap":0,"padding":0,"background":0,
        "border":{"color":0x112233,"focused_color":0xabcdef,"style":"rounded"},"children":[
          {"kind":"stack","key":"editor-area","direction":"column","gap":0,"padding":0,"background":0,"layout":{"grow":1,"shrink":true},"children":[field]},
          {"kind":"button","key":"send","label":"Send"}]});
    let source = json!({"kind":"stack","key":"root","direction":"column","gap":0,"padding":0,"background":0,"children":[composer,
        {"kind":"button","key":"back","label":"Outside"}]});
    let (_runtime, mut tui) = layout::native(
        layout::View::InteractiveScene(Rc::new(RefCell::new(source))),
        Rc::new(Cell::new(true)),
    );
    draw(&mut tui, 40, 16);
    let buffer = draw(&mut tui, 40, 16);
    let focus = tui.focus.clone().unwrap();
    let area = |tui: &Presenter| {
        tui.scene
            .iter()
            .find(|target| target.mount == focus.mount)
            .unwrap()
            .area
    };
    assert_eq!(
        (
            area(&tui).x,
            area(&tui).y,
            area(&tui).width,
            area(&tui).height
        ),
        (1, 1, 30, 1)
    );
    assert_eq!(buffer[(0, 0)].symbol(), "╭");
    assert_eq!(buffer[(39, 2)].symbol(), "╯");
    assert_eq!(buffer[(0, 0)].fg, Color::Rgb(0xab, 0xcd, 0xef));
    assert_eq!(buffer[(31, 1)].symbol(), "[");
    for (key, expected) in [
        ("send", Color::Rgb(0xab, 0xcd, 0xef)),
        ("back", Color::Rgb(0x11, 0x22, 0x33)),
    ] {
        let target = tui
            .scene
            .iter()
            .find(|target| target.key == key)
            .unwrap()
            .area
            .visible();
        for kind in [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
        ] {
            tui.input(Event::Mouse(MouseEvent {
                kind,
                column: target.x,
                row: target.y,
                modifiers: KeyModifiers::NONE,
            }))
            .unwrap();
        }
        assert_eq!(draw(&mut tui, 40, 16)[(0, 0)].fg, expected);
    }
    let target = area(&tui).visible();
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        tui.input(Event::Mouse(MouseEvent {
            kind,
            column: target.x,
            row: target.y,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
    }
    tui.input(Event::Paste(
        (0..20)
            .map(|n| format!("line-{n:02}"))
            .collect::<Vec<_>>()
            .join("\n"),
    ))
    .unwrap();
    for (width, height, expected_rows) in [(40, 16, 5), (20, 12, 4), (80, 30, 8)] {
        let buffer = draw(&mut tui, width, height);
        assert_eq!(area(&tui).height, expected_rows);
        assert!(tui.focus.as_ref().unwrap().same(&focus));
        assert!(
            buffer
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .contains("line-19")
        );
        let caret = tui.resources.owners[&focus.mount.id()]
            .state
            .downcast_ref::<super::super::editor::Owner>()
            .unwrap()
            .editor
            .cursor_position()
            .unwrap();
        assert!(area(&tui).contains(caret));
        for kind in [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
        ] {
            tui.input(Event::Mouse(MouseEvent {
                kind,
                column: caret.x,
                row: caret.y,
                modifiers: KeyModifiers::NONE,
            }))
            .unwrap();
        }
        tui.input(Event::Paste("!".into())).unwrap();
        assert!(
            tui.resources.owners[&focus.mount.id()]
                .state
                .downcast_ref::<super::super::editor::Owner>()
                .unwrap()
                .editor
                .text()
                .ends_with('!')
        );
    }
    close(&mut tui);
}

#[test]
fn session_viewport_uses_remaining_body_without_leaking_to_siblings_or_cached_measurements() {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    let long = (0..20)
        .map(|n| format!("line-{n:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    let field = |key: &str, autofocus: bool| {
        json!({"kind":"component","key":key,"binding":"editor","member":0,"on":{},"props":{
        "appearance":support::brand_appearance()["textarea"],"accessibilityLabel":key,"readOnly":false,
        "disabled":false,"softWrap":true,"tabIndex":if autofocus {1} else {-1},"autoFocus":autofocus,"initialText":long,
        "terminalRows":{"min":1,"max":8,"maxViewportFraction":1.0/3.0}}})
    };
    let rows = json!({"kind":"component","key":"rows","binding":"rows","member":0,"on":{},"props":{
        "revision":1,"entry":"row-5","rows":(0..6).map(|n|json!({"key":format!("row-{n}"),"text":format!("pending {n}"),"actions":[]})).collect::<Vec<_>>(),
        "shortcuts":[],"maxRows":3,"compactBelow":{"height":12,"maxRows":1},"tabIndex":2,
        "foreground":0xffffffffu32,"muted":0x888888ffu32,"selected":0x222222ffu32,"hovered":0x333333ffu32,
        "scrollbar":{"thumb":"┃","track":"│","thumbColor":0xffffffffu32,"trackColor":0x888888ffu32}}});
    let body = json!({"kind":"stack","key":"body","direction":"column","gap":0,"padding":1,"background":0,"viewport":true,"layout":{"grow":1},"children":[
        {"kind":"stack","key":"reading","direction":"column","gap":0,"padding":0,"background":0,"layout":{"grow":1,"basis":1},"children":[]},
        rows,
        {"kind":"stack","key":"wrapper","direction":"column","gap":0,"padding":0,"background":0,"children":[field("editor",true)]},
        {"kind":"button","key":"back","label":"Send"}
    ]});
    let source = Rc::new(RefCell::new(
        json!({"kind":"stack","key":"root","direction":"column","gap":0,"padding":0,"background":0,"children":[
            {"kind":"text","key":"header","text":"Header\nHeader\nHeader","color":0xffffff}, body, field("outside",false)
        ]}),
    ));
    let (_runtime, mut tui) = layout::native(
        layout::View::InteractiveScene(source.clone()),
        Rc::new(Cell::new(true)),
    );
    draw(&mut tui, 40, 30);
    let focus = tui.focus.clone().unwrap();
    let height = |tui: &Presenter, child: &str| {
        let mount = tui.application.child_mount(&tui.root, child).unwrap();
        tui.scene
            .iter()
            .find(|target| target.mount == *mount)
            .unwrap()
            .area
            .height
    };
    for (header, editor_rows, queue_rows) in [(3, 7, 3), (6, 6, 3), (7, 5, 1), (3, 7, 3)] {
        source.borrow_mut()["children"][0]["text"] = json!(vec!["Header"; header].join("\n"));
        let root = tui.root.clone();
        let revision = tui.application.frame(&root).unwrap().revision;
        tui.application
            .dispatch(&root, revision, "back", json!({"kind":"click"}))
            .unwrap();
        tui.sync_native().unwrap();
        let buffer = draw(&mut tui, 40, 30);
        assert_eq!(height(&tui, "editor"), editor_rows);
        assert_eq!(height(&tui, "rows"), queue_rows);
        assert_eq!(
            height(&tui, "outside"),
            10,
            "sibling keeps the surface viewport"
        );
        assert!(tui.focus.as_ref().unwrap().same(&focus));
        assert!(
            buffer
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .contains("line-19")
        );
    }
    let cache = super::super::paint::Measurements::guarded(32);
    let mount = tui.application.child_mount(&tui.root, "editor").unwrap();
    for (height, expected) in [(12, 6), (30, 10), (12, 6)] {
        assert_eq!(
            tui.measure_mount(
                mount,
                38,
                cache.context(ratatui::layout::Size { width: 38, height })
            )
            .unwrap()
            .1,
            expected
        );
    }
    let node = tui
        .application
        .frame(&tui.root)
        .unwrap()
        .view
        .view
        .scene()
        .unwrap();
    let super::super::scene::Node::Stack { children, .. } = node else {
        panic!("root stack")
    };
    assert_eq!(
        tui.measure(
            &tui.root,
            &children[1],
            40,
            cache.context(ratatui::layout::Size {
                width: 40,
                height: 30
            })
        )
        .unwrap(),
        (2, 2),
        "contained descendants cannot inflate the body's intrinsic size"
    );
    tui.input(Event::Paste("!".into())).unwrap();
    assert!(
        tui.resources.owners[&focus.mount.id()]
            .state
            .downcast_ref::<super::super::editor::Owner>()
            .unwrap()
            .editor
            .text()
            .ends_with("line-19!")
    );
    close(&mut tui);
}

#[test]
fn terminal_row_limits_resize_the_original_editor_and_reject_inverted_ranges() {
    let (_runtime, mut tui) = presenter();
    draw(&mut tui, 50, 40);
    tui.input(Event::Paste("\nline".repeat(20))).unwrap();
    draw(&mut tui, 50, 40);
    let mount = draft(&tui);
    let height = |tui: &Presenter| {
        tui.scene
            .iter()
            .find(|target| target.mount == mount)
            .unwrap()
            .area
            .height
    };
    assert_eq!(height(&tui), 10); // Other products retain the default eight text rows.
    let original = observation(&tui);
    dispatch(
        &mut tui,
        "configure",
        json!({"terminalRows":{"min":1,"max":2}}),
    );
    draw(&mut tui, 50, 40);
    assert_eq!(height(&tui), 4);
    assert_eq!(observation(&tui), original);
    for fraction in [0.0, -0.5, 1.1] {
        let revision = tui.application.frame(&tui.root).unwrap().revision;
        assert!(
            tui.application
                .dispatch(
                    &tui.root.clone(),
                    revision,
                    "configure",
                    json!({"terminalRows":{"min":1,"max":8,"maxViewportFraction":fraction}})
                )
                .is_err()
        );
    }
    let revision = tui.application.frame(&tui.root).unwrap().revision;
    assert!(
        tui.application
            .dispatch(
                &tui.root.clone(),
                revision,
                "configure",
                json!({"terminalRows":{"min":4,"max":3}})
            )
            .is_err()
    );
    draw(&mut tui, 50, 40);
    assert_eq!(height(&tui), 4);
    assert_eq!(observation(&tui), original);
    close(&mut tui);
}

#[test]
fn full_capacity_reset_captures_every_byte_and_oversize_reset_preserves_original_draft() {
    let (_runtime, mut tui) = presenter();
    draw(&mut tui, 50, 20);
    let original = observation(&tui);
    let text = "中".repeat(21845) + "x";
    assert_eq!(text.len(), 65536);
    dispatch(
        &mut tui,
        "configure",
        json!({"command":{
            "owner":original["snapshot"]["text"]["owner"],"sequence":1,
            "basedOn":original["revision"],"edit":{"kind":"reset","text":text}
        }}),
    );
    let full = observation(&tui);
    assert_eq!(full["receipt"]["outcome"]["status"], "applied");
    assert_eq!(full["snapshot"]["text"]["bytes"], 65536);
    dispatch(
        &mut tui,
        "read",
        json!({"end":65536,"capture":{
            "kind":"at_revision","owner":full["snapshot"]["text"]["owner"],"revision":full["revision"]
        }}),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while tui.application.frame(&tui.root).unwrap().state["outcome"].is_null() {
        assert!(std::time::Instant::now() < deadline);
        tui.pump().unwrap();
        std::thread::yield_now();
    }
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["outcome"]["value"]["text"],
        text
    );
    // Within the schema's character budget but above the native UTF-8 byte budget.
    dispatch(
        &mut tui,
        "configure",
        json!({"command":{
            "owner":full["snapshot"]["text"]["owner"],"sequence":2,
            "basedOn":full["revision"],"edit":{"kind":"reset","text":"中".repeat(21846)}
        }}),
    );
    let kept = observation(&tui);
    assert_eq!(
        kept["receipt"]["outcome"],
        json!({"status":"conflict","reason":"representation"})
    );
    assert_eq!(kept["snapshot"], full["snapshot"]);
    close(&mut tui);
}
fn draft(tui: &Presenter) -> Mount {
    let composer = tui.application.child_mount(&tui.root, "editor").unwrap();
    tui.application
        .child_mount(composer, "draft")
        .unwrap()
        .clone()
}
fn observation(tui: &Presenter) -> Value {
    tui.application.frame(&tui.root).unwrap().state["observation"].clone()
}
fn click(tui: &mut Presenter, kind: MouseEventKind, x: u16, y: u16) {
    tui.input(Event::Mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    }))
    .unwrap();
}
fn dispatch(tui: &mut Presenter, key: &str, value: Value) {
    let revision = tui.application.frame(&tui.root).unwrap().revision;
    tui.application
        .dispatch(&tui.root, revision, key, value)
        .unwrap();
    tui.pump().unwrap();
}
fn close(tui: &mut Presenter) {
    tui.surface_closed(&tui.surface(), Ok(())).unwrap();
    assert!(tui.can_shutdown());
    tui.shutdown().unwrap();
}

#[test]
fn native_unicode_selection_and_undo_survive_component_updates_and_stale_owner_cannot_edit() {
    let (_runtime, mut tui) = presenter();
    draw(&mut tui, 50, 20);
    let original = observation(&tui)["snapshot"]["text"]["owner"].clone();
    tui.input(Event::Paste("中文👩🏽‍💻e\u{301}".into())).unwrap();
    let observed = observation(&tui);
    assert_eq!(observed["snapshot"]["text"]["owner"], original);
    assert_eq!(
        observed["snapshot"]["selection"]["end"],
        "Start中文👩🏽‍💻e\u{301}".encode_utf16().count()
    );
    tui.input(Event::Key(KeyEvent::new(
        KeyCode::Char('z'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    assert_eq!(observation(&tui)["snapshot"]["text"]["bytes"], 5);
    let mount = draft(&tui);
    // A captured target cannot gain authority over a replacement physical owner.
    tui.resources.close(&mount).unwrap();
    tui.sync_native().unwrap();
    assert_ne!(observation(&tui)["snapshot"]["text"]["owner"], original);
    tui.input(Event::Paste("foreign".into())).unwrap();
    assert_eq!(observation(&tui)["snapshot"]["text"]["bytes"], 5);
    close(&mut tui);
}

#[test]
fn failed_flush_keeps_previous_geometry_and_new_document_rejects_old_pointer_mapping() {
    let (_runtime, mut tui) = presenter();
    draw(&mut tui, 50, 20);
    let target = tui.scene[0].clone();
    let x = target.area.visible().x + 1;
    let y = target.area.visible().y + 1;
    // Paint a completely different viewport without publishing a flush.
    let area = Rect::new(10, 5, 30, 14);
    let mut buffer = Buffer::empty(area);
    let _unflushed = tui.paint(area, &mut buffer).unwrap();
    click(&mut tui, MouseEventKind::Down(MouseButton::Left), x, y);
    assert_eq!(observation(&tui)["snapshot"]["selection"]["end"], 0);
    click(&mut tui, MouseEventKind::Drag(MouseButton::Left), x + 3, y);
    assert_eq!(
        observation(&tui)["snapshot"]["selection"],
        json!({"start":0,"end":3})
    );
    click(&mut tui, MouseEventKind::Up(MouseButton::Left), x + 3, y);
    tui.input(Event::Paste("中".into())).unwrap();
    let after_edit = observation(&tui);
    click(&mut tui, MouseEventKind::Down(MouseButton::Left), x, y);
    assert_eq!(observation(&tui), after_edit);
    draw(&mut tui, 50, 20);
    click(&mut tui, MouseEventKind::Down(MouseButton::Left), x, y);
    assert_eq!(observation(&tui)["snapshot"]["selection"]["end"], 0);
    close(&mut tui);
}

#[test]
fn forwarding_capture_precedes_same_frame_reset_and_stale_reset_preserves_later_edits() {
    let (_runtime, mut tui) = presenter();
    draw(&mut tui, 50, 20);
    let revision = observation(&tui)["revision"].as_u64().unwrap();
    let original_owner = observation(&tui)["snapshot"]["text"]["owner"].clone();
    dispatch(
        &mut tui,
        "read",
        json!({"command":{"owner":original_owner,"sequence":1,"basedOn":revision,"edit":{"kind":"reset","text":""}}}),
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
        "Start"
    );
    tui.input(Event::Paste("新草稿".into())).unwrap();
    let current = observation(&tui)["revision"].as_u64().unwrap();
    // Capture against the accepted current revision, but clear against an old
    // revision: immutable read succeeds and the later native draft remains.
    let original_owner = observation(&tui)["snapshot"]["text"]["owner"].clone();
    dispatch(
        &mut tui,
        "read",
        json!({"end":9,"capture":{"kind":"at_revision","owner":original_owner,"revision":current},"command":{"owner":original_owner,"sequence":2,"basedOn":revision,"edit":{"kind":"reset","text":""}}}),
    );
    let observed = observation(&tui);
    assert_eq!(observed["snapshot"]["text"]["bytes"], 9);
    assert_eq!(
        observed["receipt"]["outcome"],
        json!({"status":"conflict","reason":"revision"})
    );
    dispatch(&mut tui, "remove", json!({}));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !tui.jobs.is_empty() {
        assert!(std::time::Instant::now() < deadline);
        tui.pump().unwrap();
        std::thread::yield_now();
    }
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["outcome"]["value"]["text"],
        "新草稿"
    );
    close(&mut tui);
}

#[test]
fn non_tab_stop_keeps_mouse_editing_and_readonly_disabled_gate_before_edit() {
    let (_runtime, mut tui) = presenter();
    dispatch(&mut tui, "configure", json!({"tabIndex":-1}));
    draw(&mut tui, 50, 24);
    assert_eq!(tui.focus.as_ref().unwrap().key, "read");
    let target = tui
        .scene
        .iter()
        .find(|target| target.key == "observe")
        .unwrap()
        .clone();
    click(
        &mut tui,
        MouseEventKind::Down(MouseButton::Left),
        target.area.visible().x + 1,
        target.area.visible().y + 1,
    );
    tui.input(Event::Paste("中".into())).unwrap();
    assert_eq!(observation(&tui)["snapshot"]["text"]["bytes"], 8);
    tui.input(key(KeyCode::Tab)).unwrap();
    assert_ne!(tui.focus.as_ref().unwrap().key, "observe");
    dispatch(&mut tui, "configure", json!({"tabIndex":2,"readOnly":true}));
    draw(&mut tui, 50, 24);
    tui.input(key(KeyCode::BackTab)).unwrap();
    assert_eq!(tui.focus.as_ref().unwrap().key, "observe");
    let before = observation(&tui);
    tui.input(Event::Paste("blocked".into())).unwrap();
    assert_eq!(observation(&tui), before);
    dispatch(&mut tui, "configure", json!({"tabIndex":0,"disabled":true}));
    // Old successfully presented geometry is still visible, but current native
    // props must reject editing before touching the original buffer.
    tui.input(Event::Paste("blocked".into())).unwrap();
    assert_eq!(observation(&tui), before);
    close(&mut tui);
}

#[test]
fn failed_observation_preserves_edits_and_original_command_receipt_until_publication() {
    let (_runtime, mut tui) = presenter();
    draw(&mut tui, 50, 24);
    let revision = observation(&tui)["revision"].as_u64().unwrap();
    let original_owner = observation(&tui)["snapshot"]["text"]["owner"].clone();
    dispatch(&mut tui, "reject", json!({"reject":true}));
    dispatch(
        &mut tui,
        "read",
        json!({"command":{"owner":original_owner,"sequence":1,"basedOn":revision,"edit":{"kind":"reset","text":""}}}),
    );
    assert!(tui.surface.is_live());
    assert!(observation(&tui).is_null());
    tui.input(Event::Paste("Later".into())).unwrap();
    assert!(tui.surface.is_live());
    dispatch(&mut tui, "reject", json!({"reject":false}));
    let observed = observation(&tui);
    assert_eq!(observed["receipt"]["sequence"], 1);
    assert_eq!(observed["receipt"]["outcome"]["status"], "applied");
    assert_eq!(observed["snapshot"]["text"]["bytes"], 5);
    tui.pump().unwrap();
    assert_eq!(observation(&tui), observed);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !tui.jobs.is_empty() {
        assert!(std::time::Instant::now() < deadline);
        tui.pump().unwrap();
        std::thread::yield_now();
    }
    close(&mut tui);
}

#[test]
fn wheel_hits_visible_editor_without_stealing_button_focus_or_snapping_back() {
    let (_runtime, mut tui) = presenter();
    draw(&mut tui, 50, 30);
    let text = (0..20).map(|i| format!("\nrow {i:02}")).collect::<String>();
    tui.input(Event::Paste(text)).unwrap();
    draw(&mut tui, 50, 30);
    tui.input(key(KeyCode::Tab)).unwrap();
    assert_eq!(tui.focus.as_ref().unwrap().key, "read");
    let before = draw(&mut tui, 50, 30);
    let editor = tui
        .scene
        .iter()
        .find(|target| target.key == "observe")
        .unwrap()
        .clone();
    click(
        &mut tui,
        MouseEventKind::ScrollUp,
        editor.area.visible().x + 1,
        editor.area.visible().y + 1,
    );
    assert_eq!(tui.focus.as_ref().unwrap().key, "read");
    let after = draw(&mut tui, 50, 30);
    assert_ne!(before, after);
    assert_eq!(after, draw(&mut tui, 50, 30));
    close(&mut tui);
}

#[test]
fn a_different_editor_with_equal_revision_cannot_capture_or_clear_the_original_draft() {
    let (_runtime_a, mut a) = presenter();
    let (_runtime_b, mut b) = presenter();
    draw(&mut a, 50, 20);
    draw(&mut b, 50, 20);
    let original = observation(&a);
    let replacement = observation(&b);
    assert_eq!(original["revision"], replacement["revision"]);
    assert_eq!(
        original["snapshot"]["text"]["bytes"],
        replacement["snapshot"]["text"]["bytes"]
    );
    assert_ne!(
        original["snapshot"]["text"]["owner"],
        replacement["snapshot"]["text"]["owner"]
    );
    let owner = &original["snapshot"]["text"]["owner"];
    let revision = &original["revision"];
    dispatch(
        &mut b,
        "configure",
        json!({"command":{"owner":owner,"sequence":900,"basedOn":revision,"edit":{"kind":"reset","text":""}}}),
    );
    assert_eq!(
        observation(&b)["receipt"]["outcome"],
        json!({"status":"conflict","reason":"owner"})
    );
    assert_eq!(observation(&b)["snapshot"]["text"]["bytes"], 5);
    dispatch(
        &mut b,
        "read",
        json!({"capture":{"kind":"at_revision","owner":owner,"revision":revision}}),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while b.application.frame(&b.root).unwrap().state["outcome"].is_null() {
        assert!(std::time::Instant::now() < deadline);
        b.pump().unwrap();
        std::thread::yield_now();
    }
    assert_eq!(
        b.application.frame(&b.root).unwrap().state["outcome"]["status"],
        "not_accepted"
    );
    let current_owner = observation(&b)["snapshot"]["text"]["owner"].clone();
    dispatch(
        &mut b,
        "configure",
        json!({"command":{"owner":current_owner,"sequence":1,"basedOn":revision,"edit":{"kind":"reset","text":""}}}),
    );
    assert_eq!(
        observation(&b)["receipt"]["outcome"],
        json!({"status":"applied"})
    );
    assert_eq!(observation(&b)["snapshot"]["text"]["bytes"], 0);
    close(&mut a);
    close(&mut b);
}

#[test]
fn completely_clipped_editor_cannot_receive_focus_or_physical_input() {
    let (_runtime, mut tui) = presenter();
    draw(&mut tui, 50, 20);
    let mount = draft(&tui);
    dispatch(
        &mut tui,
        "configure",
        json!({"prefix":"heading\n".repeat(40)}),
    );
    draw(&mut tui, 50, 20);
    assert!(!tui.scene.iter().any(|target| target.mount == mount));
    assert!(
        tui.focus
            .as_ref()
            .is_none_or(|target| target.mount != mount)
    );
    let before = observation(&tui);
    tui.input(Event::Paste("invisible edit".into())).unwrap();
    assert_eq!(observation(&tui), before);
    close(&mut tui);
}
#[test]
fn unhandled_editor_page_key_reaches_its_original_scene_scroll_ancestor() {
    let (_runtime, mut tui) = presenter();
    dispatch(&mut tui, "configure", json!({"readingViewport":true}));
    draw(&mut tui, 50, 20);
    let mount = draft(&tui);
    let target = tui
        .scene
        .iter()
        .find(|target| target.mount == mount)
        .unwrap()
        .clone();
    tui.focus = Some(target);
    let before = observation(&tui);
    tui.input(Event::Key(KeyEvent::new(
        KeyCode::PageDown,
        KeyModifiers::NONE,
    )))
    .unwrap();
    let state = tui.resources.owners[&tui.root.id()]
        .state
        .downcast_ref::<super::super::scroll::SceneState>()
        .unwrap();
    assert_eq!(state.scrolls["presentation"].offset, 6);
    assert_eq!(observation(&tui), before);
    close(&mut tui);
}

#[test]
fn pointer_capture_survives_tab_focus_change_and_release_keeps_the_new_focus() {
    let (_runtime, mut tui) = presenter();
    draw(&mut tui, 50, 20);
    let mount = draft(&tui);
    let target = tui
        .scene
        .iter()
        .find(|target| target.mount == mount)
        .unwrap()
        .clone();
    let point = target.area.visible();
    click(
        &mut tui,
        MouseEventKind::Down(MouseButton::Left),
        point.x + 1,
        point.y + 1,
    );
    assert!(
        tui.drag
            .as_ref()
            .is_some_and(|captured| captured.same(&target))
    );
    tui.input(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
        .unwrap();
    let next = tui.focus.clone().unwrap();
    assert!(!next.same(&target));
    click(
        &mut tui,
        MouseEventKind::Drag(MouseButton::Left),
        point.x + 4,
        point.y + 1,
    );
    assert_eq!(
        observation(&tui)["snapshot"]["selection"],
        json!({"start":0,"end":3})
    );
    assert!(tui.focus.as_ref().unwrap().same(&next));
    click(
        &mut tui,
        MouseEventKind::Up(MouseButton::Left),
        point.x + 4,
        point.y + 1,
    );
    assert!(tui.drag.is_none());
    assert!(tui.focus.as_ref().unwrap().same(&next));
    close(&mut tui);
}

#[test]
fn terminal_textarea_submit_keeps_native_draft_and_obeys_current_editor_policy() {
    let (_runtime, mut tui) = presenter();
    draw(&mut tui, 50, 20);
    dispatch(&mut tui, "configure", json!({"submit":"control_enter"}));
    draw(&mut tui, 50, 20);
    tui.input(Event::Paste("中文🙂".into())).unwrap();
    let original = observation(&tui);
    tui.input(Event::Key(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["submissions"],
        1
    );
    assert_eq!(observation(&tui), original);
    tui.input(key(KeyCode::Enter)).unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["submissions"],
        1
    );
    assert_eq!(
        observation(&tui)["snapshot"]["text"]["bytes"],
        original["snapshot"]["text"]["bytes"].as_u64().unwrap() + 1
    );
    for options in [
        json!({"submit":null}),
        json!({"submit":"control_enter","readOnly":true}),
        json!({"submit":"control_enter","disabled":true}),
    ] {
        dispatch(&mut tui, "configure", options);
        draw(&mut tui, 50, 20);
        tui.input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::CONTROL,
        )))
        .unwrap();
        assert_eq!(
            tui.application.frame(&tui.root).unwrap().state["submissions"],
            1
        );
    }
    close(&mut tui);
}

#[test]
fn terminal_enter_submit_keeps_shift_enter_control_j_and_paste_in_the_original_draft() {
    let (_runtime, mut tui) = presenter();
    draw(&mut tui, 50, 20);
    dispatch(&mut tui, "configure", json!({"submit":"enter"}));
    draw(&mut tui, 50, 20);
    tui.input(Event::Paste("first\n第二行".into())).unwrap();
    for event in [
        KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT),
        KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL),
    ] {
        let before = observation(&tui)["snapshot"]["text"]["bytes"]
            .as_u64()
            .unwrap();
        tui.input(Event::Key(event)).unwrap();
        assert_eq!(observation(&tui)["snapshot"]["text"]["bytes"], before + 1);
        assert_eq!(
            tui.application.frame(&tui.root).unwrap().state["submissions"]
                .as_u64()
                .unwrap_or(0),
            0
        );
    }
    let draft = observation(&tui);
    tui.input(key(KeyCode::Enter)).unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["submissions"],
        1
    );
    assert_eq!(observation(&tui), draft);
    tui.input(Event::Key(KeyEvent::new_with_kind(
        KeyCode::Enter,
        KeyModifiers::NONE,
        crossterm::event::KeyEventKind::Repeat,
    )))
    .unwrap();
    assert_eq!(observation(&tui), draft);
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["submissions"],
        1
    );
    close(&mut tui);
}

#[test]
fn failed_submit_is_not_replayed_and_preserves_unpublished_command_receipt() {
    let (_runtime, mut tui) = presenter();
    draw(&mut tui, 50, 24);
    let original = observation(&tui);
    dispatch(&mut tui, "reject", json!({"reject":true}));
    dispatch(
        &mut tui,
        "configure",
        json!({"submit":"control_enter","command":{
            "owner":original["snapshot"]["text"]["owner"],"sequence":1,
            "basedOn":original["revision"],"edit":{"kind":"reset","text":"Ready"}
        }}),
    );
    let editor = draft(&tui);
    assert_eq!(
        tui.application.frame(&editor).unwrap().state["pending"],
        true
    );
    assert!(
        tui.input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::CONTROL,
        )))
        .is_err()
    );
    assert!(tui.surface.is_live());
    assert_eq!(
        tui.application.frame(&editor).unwrap().state["pending"],
        true
    );
    tui.pump().unwrap();
    dispatch(&mut tui, "reject", json!({"reject":false}));
    let published = observation(&tui);
    assert_eq!(published["receipt"]["sequence"], 1);
    assert_eq!(published["receipt"]["outcome"]["status"], "applied");
    assert_eq!(published["snapshot"]["text"]["bytes"], 5);
    assert_eq!(
        tui.application.frame(&editor).unwrap().state["pending"],
        false
    );
    for _ in 0..3 {
        tui.pump().unwrap();
        assert!(tui.application.frame(&tui.root).unwrap().state["submissions"].is_null());
    }
    tui.input(Event::Key(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["submissions"],
        1
    );
    close(&mut tui);
}

#[test]
fn committed_addition_preserves_native_draft_selection_focus_and_undo() {
    use maka_event_log::{
        EventLog,
        root::{RootNamespaces, RootOwner},
    };
    use std::{rc::Rc, sync::Arc};
    let (runtime, mut input, packages) = editor_fixture();
    let dir = tempfile::tempdir().unwrap();
    let namespaces = RootNamespaces {
        ownership: dir.path().join("owners"),
        control: dir.path().join("control"),
    };
    let store = Arc::new(
        runtime
            .block_on(EventLog::for_root(Arc::new(
                RootOwner::create(&dir.path().join("root"), &namespaces).unwrap(),
            )))
            .unwrap(),
    );
    let deployment = Uuid::new_v4();
    let original = runtime
        .block_on(store.commit_application(deployment, 0, input.clone(), vec![], packages.clone()))
        .unwrap();
    let mut tui = runtime
        .block_on(Presenter::start_from_store(
            store.clone(),
            original,
            runtime.handle().clone(),
            registry(),
            json!({}),
            vec![],
        ))
        .unwrap();
    draw(&mut tui, 50, 20);
    tui.input(Event::Paste("中文👩🏽‍💻e\u{301}".into())).unwrap();
    draw(&mut tui, 50, 20);
    tui.input(Event::Key(KeyEvent::new(
        KeyCode::Left,
        KeyModifiers::SHIFT,
    )))
    .unwrap();
    let before = observation(&tui);
    let mount = draft(&tui);
    let identity = tui.resources.owners[&mount.id()].identity.clone();
    let focused = tui.focus.as_ref().unwrap().mount.clone();
    let mut extra = input.instances["editor"].clone();
    extra.key = Uuid::new_v4();
    input.instances.insert("unused-editor".into(), extra);
    let next = runtime
        .block_on(store.commit_application(deployment, 1, input.clone(), vec![], packages.clone()))
        .unwrap();
    let load = tui.load_committed_update(next.clone()).unwrap();
    // Rendering and input remain available while the immutable snapshot loads.
    draw(&mut tui, 50, 20);
    let update = runtime.block_on(load).unwrap();
    tui.adopt_update(update, vec![]).unwrap();
    tui.pump().unwrap();
    draw(&mut tui, 50, 20);
    assert_eq!(tui.deployment_status().effective, Some(next));
    assert_eq!(draft(&tui), mount);
    assert!(Rc::ptr_eq(
        &identity,
        &tui.resources.owners[&mount.id()].identity
    ));
    assert_eq!(tui.focus.as_ref().unwrap().mount, focused);
    assert_eq!(observation(&tui), before);

    let original = packages
        .iter()
        .find(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let mut declaration = original.manifest().application.clone().unwrap();
    if let Implementation::JavaScript { realm, .. } = &mut declaration
        .entries
        .get_mut(&EntryTarget::Tui)
        .unwrap()
        .implementation
    {
        *realm = maka_plugin_core::package::Realm::Dedicated;
    }
    let failing = support::package(
        "test.failed-addition",
        declaration,
        Some("export function activate(){throw Error('new isolated plugin failed');}".into()),
    );
    let mut extra = input.instances["app"].clone();
    extra.key = Uuid::new_v4();
    extra.package = failing.application_ref().into();
    input.instances.insert("failed-addition".into(), extra);
    let mut bytes = packages;
    bytes.push(failing);
    let failed = runtime
        .block_on(store.commit_application(deployment, 2, input, vec![], bytes))
        .unwrap();
    let update = runtime
        .block_on(tui.load_committed_update(failed.clone()).unwrap())
        .unwrap();
    tui.adopt_update(update, vec![]).unwrap();
    // A failed, independent candidate must not turn into terminal shutdown.
    tui.pump().unwrap();
    tui.pump().unwrap();
    draw(&mut tui, 50, 20);
    let status = tui.deployment_status();
    assert_eq!(status.desired, Some(failed));
    assert!(status.error.unwrap().contains("new isolated plugin failed"));
    assert!(status.converging);
    assert!(!tui.application.is_retiring(&tui.root));
    assert_eq!(observation(&tui), before);
    assert!(Rc::ptr_eq(
        &identity,
        &tui.resources.owners[&mount.id()].identity
    ));
    assert_eq!(tui.focus.as_ref().unwrap().mount, focused);
    tui.input(Event::Key(KeyEvent::new(
        KeyCode::Char('z'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    assert_eq!(observation(&tui)["snapshot"]["text"]["bytes"], 5);
    assert!(!tui.requires_surface_replacement());
    drop(identity);
    close(&mut tui);
    drop(tui);
    runtime
        .block_on(Arc::try_unwrap(store).ok().unwrap().close())
        .unwrap();
}
