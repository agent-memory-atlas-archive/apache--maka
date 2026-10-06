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
    contract::{Availability, Cardinality, Export, Import},
    package::{Declaration, EntryDeclaration, EntryTarget, Implementation},
};
use maka_ui::{
    NativeCapability, NativeExport,
    actions::{NativeService, ServiceCompletion, ServiceFuture, ServiceRequest},
};
use std::{
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
};

struct Chunks {
    pages: Vec<Value>,
    seen: Arc<Mutex<Vec<u64>>>,
}
impl NativeService for Chunks {
    fn call(&self, request: ServiceRequest) -> ServiceFuture {
        assert_eq!(request.method, "toolImage");
        assert_eq!(request.input["sessionId"], "canonical");
        assert_eq!(request.input["through"], 100);
        assert_eq!(request.input["sequence"], 40);
        assert!(request.resources.is_empty());
        let offset = request.input["offset"].as_u64().unwrap();
        self.seen.lock().unwrap().push(offset);
        let page = self.pages[(offset / 32768) as usize].clone();
        Box::pin(async {
            ServiceCompletion {
                result: Ok(page),
                cleanup: Ok(()),
            }
        })
    }
}
#[gpui::test]
#[ignore = "opt-in ordinary product build and original native image owner"]
fn packaged_image_viewer_crosses_actual_guest_frame_queries_and_native_receipts(
    cx: &mut TestAppContext,
) {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let build = Command::new("node")
        .arg(repository.join("packages/agent-product/scripts/build.mjs"))
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    // Real 1x1 GIF, with legal comment blocks spanning seven bounded packets.
    let data = Command::new("node").args(["--input-type=module", "-e", r#"
      import {createHash} from 'node:crypto';
      import {contracts} from './packages/agent-product/dist/authoring/plugin-sdk/src/index.mjs';
      import {activate} from './packages/agent-product/dist/packages/appearance/desktop.js';
      const b=[71,73,70,56,57,97,1,0,1,0,128,0,0,0,0,0,255,255,255,33,249,4,1,0,0,0,0,44,0,0,0,0,1,0,1,0,0,2,1,68,0,33,254];
      for(let i=0;i<800;i++) b.push(255,...Array(255).fill(120)); b.push(0,59);
      const bytes=Buffer.from(b), pages=[];
      for(let offset=0;offset<bytes.length;offset+=32768) pages.push({kind:'ready',through:100,sequence:40,offset,totalBytes:bytes.length,mimeType:'image/gif',digest:'sha256:'+createHash('sha256').update(bytes).digest('hex'),data:bytes.subarray(offset,offset+32768).toString('base64')});
      console.log(JSON.stringify({pages,agent:contracts.agent,theme:activate({scheme:'light'}).Appearance.value}));
    "#]).current_dir(&repository).output().unwrap();
    assert!(
        data.status.success(),
        "{}",
        String::from_utf8_lossy(&data.stderr)
    );
    let data: Value = serde_json::from_slice(&data.stdout).unwrap();
    let dir = repository.join("packages/agent-product/dist/packages/image-viewer");
    let viewer = maka_plugins::package::Package::new(
        ["maka.extension.json", "desktop.js", "tui.js"]
            .into_iter()
            .map(|name| (name.into(), std::fs::read(dir.join(name)).unwrap()))
            .collect(),
    )
    .unwrap();
    let contract = viewer.manifest().application.as_ref().unwrap().entries[&EntryTarget::Desktop]
        .exports["ImageViewer"]
        .contract
        .clone();
    let agent: maka_plugin_core::contract::Contract =
        serde_json::from_value(data["agent"].clone()).unwrap();
    let native = |id: &str, implementation: &str, export: &str, contract| {
        support::package(
            id,
            Declaration {
                composition: None,
                entries: BTreeMap::from([(
                    EntryTarget::Desktop,
                    EntryDeclaration {
                        scene: None,
                        implementation: Implementation::Native {
                            implementation: implementation.into(),
                        },
                        configuration: json!({"type":"object"}),
                        imports: BTreeMap::new(),
                        exports: BTreeMap::from([(
                            export.into(),
                            Export {
                                contract,
                                availability: Availability::Required,
                            },
                        )]),
                    },
                )]),
            },
            None,
        )
    };
    let image = native(
        "test.image",
        "maka.ui.image",
        "Image",
        maka_ui::image::image_contract(),
    );
    let service = native(
        "test.image.chunks",
        "test.image.chunks",
        "Agent",
        agent.clone(),
    );
    let seen = Arc::new(Mutex::new(Vec::new()));
    let chunks = Arc::new(Chunks {
        pages: serde_json::from_value(data["pages"].clone()).unwrap(),
        seen: seen.clone(),
    });
    let mut registry = widgets::registry();
    registry
        .register("test.image.chunks", move |_, _| {
            Ok(BTreeMap::from([(
                "Agent".into(),
                NativeExport {
                    contract: agent.clone(),
                    value: NativeCapability::Service(chunks.clone()),
                },
            )]))
        })
        .unwrap();
    let (mut input, mut packages) = support::fixture("");
    let old = packages.pop().unwrap();
    let mut declaration = old.manifest().application.clone().unwrap();
    let entry = declaration.entries.get_mut(&EntryTarget::Desktop).unwrap();
    entry.imports.insert(
        "viewer".into(),
        Import {
            contract,
            cardinality: Cardinality::One,
        },
    );
    let root_contract = serde_json::to_string(&entry.exports["App"].contract).unwrap();
    let source = r#"
      const contract=CONTRACT,theme=THEME;
      export function activate(){return {App:{kind:'component',contract,component:['app']}};}
      function frame(state){return {actions:[],events:[],frame:{state,revision:state.revision,surface:{kind:'window',binding:'surface',title:'Image',width:640,height:420},view:{kind:'component',key:'viewer',binding:'viewer',member:0,on:{closed:'closed'},props:{theme,locale:'en',active:state.open,image:state.open?{sessionId:'canonical',through:100,sequence:40}:null}}}};}
      export const app={mount(){return frame({revision:0,open:true});},update(state){return frame({...state,revision:state.revision+1});},event(state,input){return frame({...state,revision:state.revision+1,open:input.name==='closed'?false:state.open});}};
    "#.replace("CONTRACT", &root_contract).replace("THEME", &data["theme"].to_string());
    let root = support::package("dynamic.app", declaration, Some(source));
    let root_node = input.instances.get_mut("app").unwrap();
    root_node.package = root.application_ref().into();
    root_node.bindings.insert(
        "viewer".into(),
        vec![Target::Instance {
            path: vec!["viewer".into()],
            export: "ImageViewer".into(),
        }],
    );
    let mut viewer_node = support::node(&viewer, 3);
    for (binding, export) in [("agent", "Agent"), ("image", "Image")] {
        viewer_node.bindings.insert(
            binding.into(),
            vec![Target::Instance {
                path: vec![binding.into()],
                export: export.into(),
            }],
        );
    }
    input.instances.insert("viewer".into(), viewer_node);
    input
        .instances
        .insert("image".into(), support::node(&image, 4));
    input
        .instances
        .insert("agent".into(), support::node(&service, 5));
    packages.extend([root, viewer, image, service]);
    let timer = timer();
    let scene = Scene::with_registry(cx, &timer, "", (input, packages), registry);
    let viewer = scene
        .app
        .borrow()
        .child_mount(&scene.root, "viewer")
        .unwrap()
        .clone();
    for _ in 0..40 {
        scene.draw(cx);
        if scene.app.borrow().frame(&viewer).unwrap().state["observation"]["phase"] == "ready" {
            break;
        }
        if let Some(task) = scene.app.borrow_mut().start_task(&scene.root).unwrap() {
            let completion = timer.block_on(task.future);
            scene
                .app
                .borrow_mut()
                .finish_task(task.id, completion)
                .unwrap();
        }
    }
    let state = scene.app.borrow().frame(&viewer).unwrap().state.clone();
    assert_eq!(state["observation"]["phase"], "ready", "{state}");
    assert_eq!(state["observation"]["width"], 1);
    assert_eq!(state["received"], data["pages"][0]["totalBytes"]);
    assert!(state["packet"].is_null());
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        &[0, 32768, 65536, 98304, 131072, 163840, 196608]
    );
    let native = scene
        .app
        .borrow()
        .child_mount(&viewer, "pixels")
        .unwrap()
        .clone();
    let identity = scene.resources.borrow().owners[&native.id().to_string()]
        .identity
        .clone();
    scene.draw(cx);
    assert!(Rc::ptr_eq(
        &identity,
        &scene.resources.borrow().owners[&native.id().to_string()].identity
    ));
    let revision = scene.app.borrow().frame(&viewer).unwrap().revision;
    scene
        .app
        .borrow_mut()
        .dispatch(&viewer, revision, "image-back", Value::Null)
        .unwrap();
    scene.draw(cx);
    assert_eq!(scene.state()["open"], false);
    assert!(scene.app.borrow().frame(&viewer).unwrap().state["descriptor"].is_null());
    scene.close(cx);
}
