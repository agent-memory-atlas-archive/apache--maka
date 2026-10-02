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
        },
    };
    let source = format!(
        r#"
        const contract = {};
        export function activate(configuration) {{ return {{App:{{contract,component:['app']}}}}; }}
        function frame(state) {{ return {{revision:state.revision,state,window:{{surface:'surface',title:'Dynamic product',width:640,height:400}},view:{{kind:'stack',key:'root',direction:'column',padding:16,gap:8,background:1513239,children:[{{kind:'text',key:'count',text:String(state.count),color:16777215}},{{kind:'button',key:'inc',label:'Increment'}}]}}}}; }}
        export const app = {{mount(props){{ return frame({{count:props.count??0,revision:0}}); }},event(state,event){{ {} }} }};
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
