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

use maka_plugin_core::{
    composition::{
        Configuration, Environment, ExecutionLocation, Input, Node, PackageUse, PresentationTarget,
        Target,
    },
    contract::{Availability, Cardinality, Contract, Export, Import, Interface, Transport},
    package::{Declaration, EntryDeclaration, EntryTarget, Implementation, Realm},
};
use maka_plugins::package::{MANIFEST_FILE, Package};
use maka_ui::surface_contract;
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;

pub(crate) fn node(package: &Package, key: u128) -> Node {
    Node {
        key: Uuid::from_u128(key),
        package: package.application_ref().into(),
        use_package: PackageUse::Entry,
        configuration: Configuration::default(),
        parameters: Configuration::default(),
        bindings: BTreeMap::new(),
        disabled: false,
        overrides: Vec::new(),
    }
}
pub(crate) fn package(id: &str, declaration: Declaration, source: Option<String>) -> Package {
    let mut files = BTreeMap::from([(
        MANIFEST_FILE.into(),
        serde_json::to_vec(&json!({"schemaVersion":2,"id":id,"application":declaration})).unwrap(),
    )]);
    if let Some(source) = source {
        files.insert("index.mjs".into(), source.into_bytes());
    }
    Package::new(files).unwrap()
}
pub fn fixture(handler: &str) -> (Input, Vec<Package>) {
    let surface = surface_contract(PresentationTarget::Desktop);
    let provider = package(
        "native.surface",
        Declaration {
            entries: BTreeMap::from([(
                EntryTarget::Desktop,
                EntryDeclaration {
                    implementation: Implementation::Native {
                        implementation: "maka.ui.surface.desktop".into(),
                    },
                    configuration: json!({"type":"object"}),
                    imports: BTreeMap::new(),
                    exports: BTreeMap::from([(
                        "Surface".into(),
                        Export {
                            contract: surface.clone(),
                            availability: Availability::Required,
                        },
                    )]),
                },
            )]),
            composition: None,
        },
        None,
    );
    let component = Contract {
        name: "example.App".into(),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Component {
            props: json!({"type":"object"}),
            events: BTreeMap::new(),
            captures: BTreeMap::new(),
        },
    };
    let source = format!(
        r#"
        const contract = {};
        export function activate(configuration) {{ return {{App:{{contract,component:['app']}}}}; }}
        function frame(state) {{ return {{actions:[],events:[],frame:{{revision:state.revision,state,window:{{surface:'surface',title:'Dynamic product',width:640,height:400}},view:{{kind:'stack',key:'root',direction:'column',padding:16,gap:8,background:1513239,children:[{{kind:'text',key:'count',text:String(state.count),color:16777215}},{{kind:'button',key:'inc',label:'Increment'}}]}}}}}}; }}
        export const app = {{mount(props){{ return frame({{count:props.count??0,revision:0}}); }},event(state,event){{ {} }},update(state,props){{ return frame({{...state,revision:state.revision+1}}); }} }};
    "#,
        serde_json::to_string(&component).unwrap(),
        handler
    );
    let app = package(
        "dynamic.app",
        Declaration {
            entries: BTreeMap::from([(
                EntryTarget::Desktop,
                EntryDeclaration {
                    implementation: Implementation::JavaScript {
                        module: "index.mjs".into(),
                        realm: Realm::Dedicated,
                    },
                    configuration: json!({"type":"object"}),
                    imports: BTreeMap::from([(
                        "surface".into(),
                        Import {
                            contract: surface,
                            cardinality: Cardinality::One,
                        },
                    )]),
                    exports: BTreeMap::from([(
                        "App".into(),
                        Export {
                            contract: component,
                            availability: Availability::Required,
                        },
                    )]),
                },
            )]),
            composition: None,
        },
        Some(source),
    );
    let mut root = node(&app, 2);
    root.bindings.insert(
        "surface".into(),
        vec![Target::Instance {
            path: vec!["surface".into()],
            export: "Surface".into(),
        }],
    );
    (
        Input {
            environment: Environment {
                location: ExecutionLocation::Ui,
                presentation: Some(PresentationTarget::Desktop),
            },
            instances: BTreeMap::from([
                ("surface".into(), node(&provider, 1)),
                ("app".into(), root),
            ]),
            launch: Some(Target::Instance {
                path: vec!["app".into()],
                export: "App".into(),
            }),
        },
        vec![provider, app],
    )
}

pub(crate) fn composed_fixture(child_update: &str, parent_event: &str) -> (Input, Vec<Package>) {
    let (mut input, mut packages) = fixture("");
    let root = packages.pop().unwrap();
    let mut declaration = root.manifest().application.clone().unwrap();
    let root_contract = declaration.entries[&EntryTarget::Desktop].exports["App"]
        .contract
        .clone();
    let counter_contract = Contract {
        name: "example.Counter".into(),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Component {
            props: json!({"type":"object","properties":{"label":{"type":"string"},"initial":{"type":"integer"}},"required":["label","initial"],"additionalProperties":false}),
            events: BTreeMap::new(),
            captures: BTreeMap::new(),
        },
    };
    declaration
        .entries
        .get_mut(&EntryTarget::Desktop)
        .unwrap()
        .imports
        .insert(
            "counter".into(),
            Import {
                contract: counter_contract.clone(),
                cardinality: Cardinality::One,
            },
        );
    let source = format!(
        r#"
        export function activate(){{return {{App:{{contract:{},component:['app']}}}};}}
        function frame(state){{
            const children=[{{kind:'component',key:'left',binding:'counter',member:0,on:{{}},props:{{label:'Left '+state.version,initial:10}}}}];
            if(state.right)children.push({{kind:'component',key:'right',binding:'counter',member:0,on:{{}},props:{{label:'Right '+state.version,initial:20}}}});
            return {{actions:[],events:[],frame:{{state,revision:state.revision,window:{{surface:'surface',title:'Composed product',width:640,height:400}},view:{{kind:'stack',key:'root',direction:'column',gap:8,padding:16,background:1513239,children:[{{kind:'stack',key:'counters',direction:'row',gap:16,padding:0,background:1513239,children}},{{kind:'button',key:'parent',label:'Change labels'}},{{kind:'button',key:'toggle',label:'Toggle right counter'}}]}}}}}};
        }}
        export const app={{mount(){{return frame({{revision:0,version:0,right:true}});}},event(state,event){{{}}},update(state,props){{return frame({{...state,revision:state.revision+1}});}}}};
    "#,
        serde_json::to_string(&root_contract).unwrap(),
        parent_event
    );
    let root = package("dynamic.app", declaration, Some(source));
    let source = format!(
        r#"
        export function activate(){{return {{Counter:{{contract:{},component:['counter']}}}};}}
        function frame(state){{return {{actions:[],events:[],frame:{{state,revision:state.revision,window:null,view:{{kind:'stack',key:'counter',direction:'column',gap:8,padding:8,background:1513239,children:[{{kind:'text',key:'label',text:state.label,color:16777215}},{{kind:'text',key:'count',text:String(state.count),color:16777215}},{{kind:'button',key:'inc',label:'Increment '+state.label}}]}}}}}};}}
        export const counter={{mount(props){{return frame({{count:props.initial,label:props.label,revision:0}});}},event(state,event){{return frame({{...state,count:state.count+1,revision:state.revision+1}});}},update(state,props){{{}}}}};
    "#,
        serde_json::to_string(&counter_contract).unwrap(),
        child_update
    );
    let counter = package(
        "dynamic.counter",
        Declaration {
            entries: BTreeMap::from([(
                EntryTarget::Desktop,
                EntryDeclaration {
                    implementation: Implementation::JavaScript {
                        module: "index.mjs".into(),
                        realm: Realm::Dedicated,
                    },
                    configuration: json!({"type":"object"}),
                    imports: BTreeMap::new(),
                    exports: BTreeMap::from([(
                        "Counter".into(),
                        Export {
                            contract: counter_contract,
                            availability: Availability::Required,
                        },
                    )]),
                },
            )]),
            composition: None,
        },
        Some(source),
    );
    input.instances.get_mut("app").unwrap().package = root.application_ref().into();
    input.instances.get_mut("app").unwrap().bindings.insert(
        "counter".into(),
        vec![Target::Instance {
            path: vec!["counter".into()],
            export: "Counter".into(),
        }],
    );
    input.instances.insert("counter".into(), node(&counter, 3));
    packages.extend([root, counter]);
    (input, packages)
}

pub(crate) const CHILD_UPDATE: &str =
    "return frame({...state,label:props.label,revision:state.revision+1});";
pub(crate) const PARENT_EVENT: &str = "const next={...state,revision:state.revision+1};if(event.key==='toggle')next.right=!state.right;else next.version++;return frame(next);";

/// Mixed JS/native composition. The package selects a compiled implementation;
/// placing, updating and removing it remains ordinary product component code.
pub(crate) fn native_fixture() -> (Input, Vec<Package>) {
    let (mut input, mut packages) = composed_fixture(CHILD_UPDATE, PARENT_EVENT);
    let index = packages
        .iter()
        .position(|package| package.manifest().id == "dynamic.app")
        .unwrap();
    let root = packages.remove(index);
    let mut declaration = root.manifest().application.clone().unwrap();
    let contract = maka_ui::label_contract();
    declaration
        .entries
        .get_mut(&EntryTarget::Desktop)
        .unwrap()
        .imports
        .insert(
            "label".into(),
            Import {
                contract: contract.clone(),
                cardinality: Cardinality::One,
            },
        );
    let source = std::str::from_utf8(root.file("index.mjs").unwrap()).unwrap();
    let source = source.replace("children:[{kind:'stack',key:'counters'", "children:[...(state.right?[{kind:'component',key:'nativeLabel',binding:'label',member:0,on:{},props:{text:'Native '+state.version,emphasis:true}}]:[]),{kind:'stack',key:'counters'");
    let root = package("dynamic.app", declaration, Some(source));
    let label = package(
        "native.label",
        Declaration {
            entries: BTreeMap::from([(
                EntryTarget::Desktop,
                EntryDeclaration {
                    implementation: Implementation::Native {
                        implementation: "maka.ui.label".into(),
                    },
                    configuration: json!({"type":"object"}),
                    imports: BTreeMap::new(),
                    exports: BTreeMap::from([(
                        "Label".into(),
                        Export {
                            contract,
                            availability: Availability::Required,
                        },
                    )]),
                },
            )]),
            composition: None,
        },
        None,
    );
    let app = input.instances.get_mut("app").unwrap();
    app.package = root.application_ref().into();
    app.bindings.insert(
        "label".into(),
        vec![Target::Instance {
            path: vec!["label".into()],
            export: "Label".into(),
        }],
    );
    input.instances.insert("label".into(), node(&label, 4));
    packages.extend([root, label]);
    (input, packages)
}

pub(crate) fn editor_fixture() -> (Input, Vec<Package>) {
    let (mut input, mut packages) = native_fixture();
    let index = packages
        .iter()
        .position(|package| package.manifest().id == "dynamic.app")
        .unwrap();
    let root = packages.remove(index);
    let mut declaration = root.manifest().application.clone().unwrap();
    let contract = maka_ui::editor::editor_contract();
    declaration
        .entries
        .get_mut(&EntryTarget::Desktop)
        .unwrap()
        .imports
        .insert(
            "editor".into(),
            Import {
                contract: contract.clone(),
                cardinality: Cardinality::One,
            },
        );
    let source = std::str::from_utf8(root.file("index.mjs").unwrap()).unwrap();
    let source = source.replace("...(state.right?[{kind:'component',key:'nativeLabel'", "...(state.right?[{kind:'component',key:'editor',binding:'editor',member:0,on:{},props:{initialText:'Start',placeholder:'Write here',command:state.command??null}},{kind:'component',key:'nativeLabel'");
    let source = source.replace("const next={...state,revision:state.revision+1};", "const next={...state,revision:state.revision+1};if(event.event?.command)next.command=event.event.command;");
    let source = source.replace(
        "title:'Composed product',width:640,height:400",
        "title:'Native editor product',width:640,height:600",
    );
    let root = package("dynamic.app", declaration, Some(source));
    let editor = package(
        "native.editor",
        Declaration {
            entries: BTreeMap::from([(
                EntryTarget::Desktop,
                EntryDeclaration {
                    implementation: Implementation::Native {
                        implementation: "maka.ui.editor".into(),
                    },
                    configuration: json!({"type":"object"}),
                    imports: BTreeMap::new(),
                    exports: BTreeMap::from([(
                        "Editor".into(),
                        Export {
                            contract,
                            availability: Availability::Required,
                        },
                    )]),
                },
            )]),
            composition: None,
        },
        None,
    );
    let app = input.instances.get_mut("app").unwrap();
    app.package = root.application_ref().into();
    app.bindings.insert(
        "editor".into(),
        vec![Target::Instance {
            path: vec!["editor".into()],
            export: "Editor".into(),
        }],
    );
    input.instances.insert("editor".into(), node(&editor, 5));
    packages.extend([root, editor]);
    (input, packages)
}

/// A product component subscribes to the Editor's public metadata event. No
/// global lookup or text-resource read is granted by receiving this payload.
#[allow(dead_code)] // Shared with tests that do not load the Desktop editor.
pub(crate) const EDITOR_RECEIVE: &str = "if(event.kind==='component'&&event.name==='changed'){return frame({...state,revision:state.revision+1,observation:event.event});}";
#[allow(dead_code)] // Shared with tests that do not load the Desktop editor.
pub(crate) fn editor_events_fixture() -> (Input, Vec<Package>) {
    let (mut input, mut packages) = editor_fixture();
    let index = packages
        .iter()
        .position(|package| package.manifest().id == "dynamic.app")
        .unwrap();
    let old = packages.remove(index);
    let source=std::str::from_utf8(old.file("index.mjs").unwrap()).unwrap()
        .replace("on:{},props:{initialText:","on:{changed:'editor_changed'},props:{initialText:")
        .replace("const next={...state,revision:state.revision+1};",&format!("{EDITOR_RECEIVE}const next={{...state,revision:state.revision+1}};"))
        .replace("children:[...(state.right?", "children:[{kind:'text',key:'editor_status',text:'Draft revision '+(state.observation?.revision??0),color:16777215},...(state.right?");
    let root = package(
        "dynamic.app",
        old.manifest().application.clone().unwrap(),
        Some(source),
    );
    input.instances.get_mut("app").unwrap().package = root.application_ref().into();
    packages.push(root);
    (input, packages)
}

/// Root consumes the public Composer contract; its private editor slot is only
/// known by that wrapper. All resource authority stays in the request sidecar.
pub(crate) fn resource_fixture() -> (Input, Vec<Package>) {
    let (mut input, mut packages) = editor_fixture();
    let root_index = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let old = packages.remove(root_index);
    let mut declaration = old.manifest().application.clone().unwrap();
    let entry = declaration.entries.get_mut(&EntryTarget::Desktop).unwrap();
    entry.imports.remove("counter");
    entry.imports.remove("label");
    entry.imports.insert(
        "reader".into(),
        Import {
            contract: maka_ui::text::reader_contract(),
            cardinality: Cardinality::One,
        },
    );
    let root_source = format!(
        r#"
    const contract={};
    export function activate(){{return {{App:{{contract,component:['app']}}}};}}
    function frame(state){{return {{actions:[],events:[],frame:{{revision:state.revision,state,window:{{surface:'surface',title:'Immutable draft',width:640,height:480}},view:{{kind:'stack',key:'root',direction:'column',padding:16,gap:12,background:1513239,children:[{{kind:'text',key:'status',text:'Captured: '+(state.outcome?.value?.text??state.outcome?.status??'—'),color:16777215}},...(state.show?[{{kind:'component',key:'editor',binding:'editor',member:0,on:{{changed:'changed'}},props:{{initialText:'Start',placeholder:'Write here',command:state.command??null}}}}]:[]),{{kind:'button',key:'read',label:'Capture draft'}},{{kind:'button',key:'remove',label:'Remove composer'}}]}}}}}};}}
    export const app={{mount(){{return frame({{revision:0,show:true}});}},update(state){{return frame({{...state,revision:state.revision+1}});}},event(state,event){{
        const next={{...state,revision:state.revision+1}};
        if(event.kind==='action')next.outcome=event.event;
        else if(event.kind==='component')next.observation=event.event;
        else if(event.key==='remove')next.show=false;
        else if(event.key==='read'){{if(event.event?.command)next.command=event.event.command;const out=frame(next);out.actions.push({{key:'read_done',binding:'reader',member:0,method:'read',input:{{start:event.event?.start??0,end:event.event?.end??5}},resources:{{text:{{child:'editor',method:'snapshot',input:event.event?.capture??{{kind:'current'}}}}}}}});return out;}}
        return frame(next);
    }}}};
    "#,
        serde_json::to_string(&entry.exports["App"].contract).unwrap()
    );
    let root = package("dynamic.app", declaration, Some(root_source));
    let editor = maka_ui::editor::editor_contract();
    let composer = package(
        "dynamic.composer",
        Declaration {
            composition: None,
            entries: BTreeMap::from([(
                EntryTarget::Desktop,
                EntryDeclaration {
                    implementation: Implementation::JavaScript {
                        module: "index.mjs".into(),
                        realm: Realm::Shared,
                    },
                    configuration: json!({"type":"object"}),
                    imports: BTreeMap::from([(
                        "editor".into(),
                        Import {
                            contract: editor.clone(),
                            cardinality: Cardinality::One,
                        },
                    )]),
                    exports: BTreeMap::from([(
                        "Editor".into(),
                        Export {
                            contract: editor.clone(),
                            availability: Availability::Required,
                        },
                    )]),
                },
            )]),
        },
        Some(format!(
            r#"
        const contract={};export function activate(){{return {{Editor:{{contract,component:['composer']}}}};}}
        function frame(state,props,events=[]){{return {{actions:[],events,frame:{{revision:state.revision,state,window:null,view:{{kind:'component',key:'draft',binding:'editor',member:0,props,on:{{changed:'changed'}}}}}}}};}}
        export const composer={{mount(props){{return frame({{revision:0}},props);}},update(state,props){{return frame({{...state,revision:state.revision+1}},props);}},event(state,event,props){{return frame({{revision:state.revision+1}},props,[{{name:'changed',value:event.event}}]);}},capture(state,props,method,input){{return {{kind:'forward',source:{{child:'draft',method,input}}}};}}}};
    "#,
            serde_json::to_string(&editor).unwrap()
        )),
    );
    let reader = package(
        "native.reader",
        Declaration {
            composition: None,
            entries: BTreeMap::from([(
                EntryTarget::Desktop,
                EntryDeclaration {
                    implementation: Implementation::Native {
                        implementation: "maka.text.reader".into(),
                    },
                    configuration: json!({"type":"object"}),
                    imports: BTreeMap::new(),
                    exports: BTreeMap::from([(
                        "Reader".into(),
                        Export {
                            contract: maka_ui::text::reader_contract(),
                            availability: Availability::Required,
                        },
                    )]),
                },
            )]),
        },
        None,
    );
    let root_node = input.instances.get_mut("app").unwrap();
    root_node.package = root.application_ref().into();
    root_node.bindings.remove("counter");
    root_node.bindings.remove("label");
    root_node.bindings.insert(
        "editor".into(),
        vec![Target::Instance {
            path: vec!["composer".into()],
            export: "Editor".into(),
        }],
    );
    root_node.bindings.insert(
        "reader".into(),
        vec![Target::Instance {
            path: vec!["reader".into()],
            export: "Reader".into(),
        }],
    );
    let mut composer_node = node(&composer, 6);
    composer_node.bindings.insert(
        "editor".into(),
        vec![Target::Instance {
            path: vec!["editor".into()],
            export: "Editor".into(),
        }],
    );
    input.instances.insert("composer".into(), composer_node);
    input.instances.insert("reader".into(), node(&reader, 7));
    input.instances.remove("counter");
    input.instances.remove("label");
    packages.retain(|p| !matches!(p.manifest().id.as_str(), "dynamic.counter" | "native.label"));
    packages.extend([root, composer, reader]);
    (input, packages)
}
