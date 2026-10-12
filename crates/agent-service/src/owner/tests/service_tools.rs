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
    contract::{Contract, Interface, Method, MethodMode, Transport},
    runtime::Published,
};
use maka_runtime::{
    archive::outcome_projection,
    event::{Fact, ToolOutcome},
    tool_output::DurableToolProjection,
};
use maka_runtime_host::application::{Context, Handler, NativeOwner};
use std::sync::atomic::{AtomicUsize, Ordering};

fn counter_contract() -> Contract {
    let write = Method {
        progress: None,
        mode: MethodMode::Mutation,
        input: json!({"type":"object","properties":{"amount":{"type":"integer","minimum":1},"note":{"type":"string"}},"required":["amount"],"additionalProperties":false}),
        output: json!({"type":"object","properties":{"effects":{"type":"integer"},"payload":{"type":"string"}},"required":["effects"],"additionalProperties":false}),
        resources: BTreeMap::new(),
    };
    let read = Method {
        progress: None,
        mode: MethodMode::Query,
        ..write.clone()
    };
    Contract {
        name: "fixture.Counter".into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service {
            methods: BTreeMap::from([("write".into(), write), ("read".into(), read)]),
        },
    }
}
#[derive(Clone, Copy)]
enum Behavior {
    Returned,
    Unknown,
    CleanupFailed,
    LargeCleanupFailed,
    Blocked,
}
struct Counter {
    behavior: Behavior,
    effects: AtomicUsize,
    entered: Arc<Semaphore>,
    cancelled: Arc<Semaphore>,
    release: Arc<Semaphore>,
}
impl Counter {
    fn new(behavior: Behavior) -> Arc<Self> {
        Arc::new(Self {
            behavior,
            effects: AtomicUsize::new(0),
            entered: Arc::new(Semaphore::new(0)),
            cancelled: Arc::new(Semaphore::new(0)),
            release: Arc::new(Semaphore::new(0)),
        })
    }
}
impl Handler for Counter {
    fn call(
        &self,
        method: String,
        input: Value,
        context: Context,
    ) -> BoxFuture<'static, Result<Value, String>> {
        let count = if method == "read" {
            assert!(context.operation().is_none());
            self.effects.load(Ordering::SeqCst)
        } else {
            assert!(context.operation().is_some());
            self.effects
                .fetch_add(input["amount"].as_u64().unwrap() as usize, Ordering::SeqCst)
                + 1
        };
        let behavior = self.behavior;
        let entered = self.entered.clone();
        let cancelled = self.cancelled.clone();
        let release = self.release.clone();
        Box::pin(async move {
            entered.add_permits(1);
            match behavior {
                Behavior::Returned => Ok(json!({"effects":count})),
                Behavior::Unknown => Err("lost reply after original effect".into()),
                Behavior::CleanupFailed | Behavior::LargeCleanupFailed => {
                    let mut ticket = context.scope().resources.reserve().unwrap();
                    ticket.start();
                    ticket.complete(Err("counter cleanup failed".into()));
                    if matches!(behavior, Behavior::LargeCleanupFailed) {
                        let value = json!({"effects":count,"payload":"x".repeat(1024*1024-128)});
                        assert!(serde_json::to_vec(&value).unwrap().len() <= 1024 * 1024);
                        Ok(value)
                    } else {
                        Ok(json!({"effects":count}))
                    }
                }
                Behavior::Blocked => {
                    context.scope().cancellation.cancelled().await;
                    cancelled.add_permits(1);
                    release.acquire().await.unwrap().forget();
                    Ok(json!({"effects":count}))
                }
            }
        })
    }
}
impl NativeOwner for Counter {
    fn close(&self) -> BoxFuture<'static, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }
}
pub(super) struct Setup {
    title: Option<maka_runtime::display::Text>,
    counter: Option<Arc<Counter>>,
    granted: bool,
    query: bool,
    questions: bool,
    forms: bool,
    files: Option<&'static str>,
    processes: bool,
    isolated_workspaces: bool,
    web: bool,
    jev: bool,
    protocol: bool,
    skills: Option<std::path::PathBuf>,
    bundled_skills: bool,
    standard_sources: bool,
    prepare_skills: bool,
    skill_tools: bool,
    preparation: Option<Arc<PreparationProbe>>,
    images: Option<images::Source>,
    confirmation: contract::ToolConfirmation,
}
impl Setup {
    pub(super) fn titled(mut self, title: maka_runtime::display::Text) -> Self {
        self.title = Some(title);
        self
    }
    fn javascript() -> Self {
        Self {
            title: None,
            counter: None,
            granted: true,
            query: false,
            questions: false,
            forms: false,
            files: None,
            processes: false,
            isolated_workspaces: false,
            web: false,
            jev: false,
            protocol: false,
            skills: None,
            bundled_skills: false,
            standard_sources: false,
            prepare_skills: false,
            skill_tools: true,
            preparation: None,
            images: None,
            confirmation: contract::ToolConfirmation::Preapproved,
        }
    }
    fn native(counter: Arc<Counter>, granted: bool) -> Self {
        Self {
            title: None,
            counter: Some(counter),
            granted,
            query: false,
            questions: false,
            forms: false,
            files: None,
            processes: false,
            isolated_workspaces: false,
            web: false,
            jev: false,
            protocol: false,
            skills: None,
            bundled_skills: false,
            standard_sources: false,
            prepare_skills: false,
            skill_tools: true,
            preparation: None,
            images: None,
            confirmation: contract::ToolConfirmation::Preapproved,
        }
    }
    pub(super) fn forms() -> Self {
        Self {
            forms: true,
            ..Self::javascript()
        }
    }
    pub(super) fn questions() -> Self {
        Self {
            title: None,
            counter: None,
            granted: true,
            query: false,
            questions: true,
            forms: false,
            files: None,
            processes: false,
            isolated_workspaces: false,
            web: false,
            jev: false,
            protocol: false,
            skills: None,
            bundled_skills: false,
            standard_sources: false,
            prepare_skills: false,
            skill_tools: true,
            preparation: None,
            images: None,
            confirmation: contract::ToolConfirmation::Preapproved,
        }
    }
    pub(super) fn files(method: &'static str) -> Self {
        Self {
            files: Some(method),
            ..Self::javascript()
        }
    }
    pub(super) fn skills(path: std::path::PathBuf) -> Self {
        Self {
            skills: Some(path),
            ..Self::javascript()
        }
    }
    pub(super) fn with_standard_sources(mut self) -> Self {
        self.standard_sources = true;
        self
    }
    pub(super) fn with_bundled_skills(mut self) -> Self {
        self.bundled_skills = true;
        self
    }
    pub(super) fn skill_preparation(path: std::path::PathBuf, tools: bool) -> Self {
        Self {
            skills: Some(path),
            prepare_skills: true,
            skill_tools: tools,
            ..Self::javascript()
        }
    }
    pub(super) fn skill_import(&self) -> Option<String> {
        self.prepare_skills.then(|| "service".into())
    }
    pub(super) fn protocol() -> Self {
        Self {
            protocol: true,
            ..Self::javascript()
        }
    }
    pub(super) fn jev() -> Self {
        Self {
            jev: true,
            ..Self::javascript()
        }
    }
    pub(super) fn web() -> Self {
        Self {
            web: true,
            ..Self::javascript()
        }
    }
    pub(super) fn isolated_processes() -> Self {
        Self {
            isolated_workspaces: true,
            ..Self::processes()
        }
    }
    pub(super) fn processes() -> Self {
        Self {
            processes: true,
            ..Self::javascript()
        }
    }
    pub(super) fn images(source: images::Source) -> Self {
        Self {
            images: Some(source),
            ..Self::javascript()
        }
    }
    pub(super) fn without_grant(mut self) -> Self {
        self.granted = false;
        self
    }
    pub(super) fn selections(&self) -> Vec<contract::ServiceTool> {
        if self.protocol || (self.skills.is_some() && !self.skill_tools) {
            return vec![];
        }
        if self.web {
            return [("fetch", "WebFetch"), ("search", "WebSearch")]
                .into_iter()
                .map(|(method, name)| contract::ServiceTool {
                    method: method.into(),
                    name: name.into(),
                    ..self.selection()
                })
                .collect();
        }
        if self.skills.is_some() {
            if self.prepare_skills {
                [("search", "FindGuide"), ("load", "LoadGuide")]
            } else {
                [("search", "SkillSearch"), ("load", "Skill")]
            }
            .into_iter()
            .map(|(method, name)| contract::ServiceTool {
                input: contract::ToolInput::Skill,
                name: name.into(),
                method: method.into(),
                ..self.selection()
            })
            .collect()
        } else if self.processes {
            [
                ("execute", "Execute"),
                ("read", "ReadProcess"),
                ("writeStdin", "WriteStdin"),
                ("stop", "StopProcess"),
            ]
            .into_iter()
            .map(|(method, name)| contract::ServiceTool {
                input: contract::ToolInput::Json,
                title: None,
                name: name.into(),
                method: method.into(),
                presentation: Some(format!("maka.workspace.Processes@1/{method}")),
                ..self.selection()
            })
            .collect()
        } else {
            vec![self.selection()]
        }
    }
    fn contract(&self) -> Contract {
        if self.jev {
            return crate::jev::contract();
        }
        if self.web {
            return crate::web::contract();
        }
        if self.skills.is_some() {
            crate::skills::contract()
        } else if let Some(source) = &self.images {
            source.contract()
        } else if self.protocol {
            crate::processes::protocol::contract()
        } else if self.processes {
            crate::processes::contract()
        } else if self.files.is_some() {
            crate::files::contract()
        } else if self.forms {
            crate::forms::contract()
        } else if self.questions {
            crate::questions::contract()
        } else {
            counter_contract()
        }
    }
    fn export(&self) -> &str {
        if self.jev {
            return crate::jev::EXPORT;
        }
        if self.web {
            return crate::web::EXPORT;
        }
        if self.skills.is_some() {
            crate::skills::EXPORT
        } else if self.images.is_some() {
            "Images"
        } else if self.protocol {
            crate::processes::protocol::EXPORT
        } else if self.processes {
            crate::processes::EXPORT
        } else if self.files.is_some() {
            crate::files::EXPORT
        } else if self.forms {
            crate::forms::EXPORT
        } else if self.questions {
            crate::questions::EXPORT
        } else {
            "Counter"
        }
    }
    pub(super) fn method(&self) -> &str {
        if self.jev {
            return "evaluate";
        }
        if self.web {
            return "fetch";
        }
        if self.skills.is_some() {
            "search"
        } else if self.images.is_some() {
            "produce"
        } else if self.processes {
            "execute"
        } else if let Some(method) = self.files {
            method
        } else if self.questions || self.forms {
            "ask"
        } else if self.query {
            "read"
        } else {
            "write"
        }
    }
    pub(super) fn selection(&self) -> contract::ServiceTool {
        contract::ServiceTool {
            input: if self.web {
                contract::ToolInput::Web
            } else {
                contract::ToolInput::Json
            },
            presentation: self
                .files
                .filter(|method| {
                    matches!(
                        *method,
                        "readText" | "glob" | "grep" | "write" | "edit" | "applyPatch"
                    )
                })
                .map(|method| format!("maka.workspace.Files@1/{method}")),
            title: self.title.clone(),
            confirmation: self.confirmation,
            result: if self.files == Some("readImage")
                || self.images.as_ref().is_some_and(|source| source.is_image())
            {
                contract::ToolResult::Image
            } else {
                contract::ToolResult::Json
            },
            name: if self.web {
                "WebFetch"
            } else if self.files.is_some() || self.images.is_some() {
                "WorkspaceFile"
            } else if self.forms {
                "AskUserForm"
            } else if self.questions {
                "AskUserQuestion"
            } else {
                "Counter"
            }
            .into(),
            description: if self.files.is_some() {
                "Use the selected workspace files."
            } else if self.questions {
                "Ask required bounded questions."
            } else {
                "Increment the explicitly selected counter."
            }
            .into(),
            import: "service".into(),
            method: self.method().into(),
        }
    }
    pub(super) fn install(&self, input: &mut Input, packages: &mut Vec<Package>) {
        let project_roots =
            input.instances["provider"].configuration.values["workspaceRoots"].clone();
        let mut manifest = packages[0].manifest().clone();
        manifest
            .application
            .as_mut()
            .unwrap()
            .entries
            .get_mut(&EntryTarget::Host)
            .unwrap()
            .imports
            .insert(
                "service".into(),
                Import {
                    contract: self.contract(),
                    cardinality: Cardinality::One,
                },
            );
        let provider = Package::new(BTreeMap::from([(
            MANIFEST_FILE.into(),
            serde_json::to_vec(&manifest).unwrap(),
        )]))
        .unwrap();
        input.instances.get_mut("provider").unwrap().package = provider.application_ref().into();
        input
            .instances
            .get_mut("provider")
            .unwrap()
            .bindings
            .insert(
                "service".into(),
                vec![Target::Instance {
                    path: vec!["service".into()],
                    export: self.export().into(),
                }],
            );
        packages[0] = provider;
        let implementation = if self.jev {
            json!({"kind":"native","implementation":crate::jev::IMPLEMENTATION})
        } else if self.web {
            json!({"kind":"native","implementation":crate::web::IMPLEMENTATION})
        } else if self.skills.is_some() {
            json!({"kind":"native","implementation":crate::skills::IMPLEMENTATION})
        } else if let Some(source) = &self.images {
            source.implementation()
        } else if self.protocol {
            json!({"kind":"native","implementation":crate::processes::protocol::IMPLEMENTATION})
        } else if self.processes {
            json!({"kind":"native","implementation":crate::processes::IMPLEMENTATION})
        } else if self.files.is_some() {
            json!({"kind":"native","implementation":crate::files::IMPLEMENTATION})
        } else if self.forms {
            json!({"kind":"native","implementation":crate::forms::IMPLEMENTATION})
        } else if self.questions {
            json!({"kind":"native","implementation":crate::questions::IMPLEMENTATION})
        } else if self.counter.is_some() {
            json!({"kind":"native","implementation":"fixture.counter"})
        } else {
            json!({"kind":"javascript","module":"counter.mjs","realm":"shared"})
        };
        let mut files = BTreeMap::from([(MANIFEST_FILE.into(), serde_json::to_vec(&json!({"schemaVersion":2,"id":"fixture.counter","application":{"entries":{"host":{
            "implementation":implementation,"configuration":if self.jev { crate::jev::configuration_schema() } else if self.web { crate::web::configuration_schema() } else if self.skills.is_some() { crate::skills::configuration_schema() } else if self.processes || self.protocol { crate::processes::configuration_schema() } else { json!({"type":"object","additionalProperties":false}) },"imports":{},
            "exports":if self.processes {
                json!({(self.export()):{"contract":self.contract(),"availability":"required"},
                    "terminal":{"contract":crate::processes::terminal::contract(),"availability":"required"}})
            } else { json!({(self.export()):{"contract":self.contract(),"availability":"required"}}) }
        }}}})).unwrap())]);
        if let Some(source) = &self.images {
            if let Some(code) = source.javascript() {
                files.insert("images.mjs".into(), code.into_bytes());
            }
        } else if !self.web
            && !self.jev
            && self.counter.is_none()
            && !self.forms
            && !self.questions
            && self.files.is_none()
            && !self.processes
            && !self.protocol
            && self.skills.is_none()
        {
            files.insert("counter.mjs".into(), format!(r#"let effects=0; export function activate(){{return {{Counter:{{contract:{},service:['counter']}}}};}} export const counter={{write({{amount}}){{return {{effects:effects+=amount}};}},read(){{return {{effects}};}}}};"#, serde_json::to_string(&counter_contract()).unwrap()).into_bytes());
        }
        let package = Package::new(files).unwrap();
        input.instances.insert(
            "service".into(),
            Node {
                key: Uuid::from_u128(21),
                package: package.application_ref().into(),
                use_package: PackageUse::Entry,
                configuration: if self.web || self.jev { NodeConfiguration { values: serde_json::from_value(json!({"network":"allowed"})).unwrap(), parameters: BTreeMap::new() } } else if let Some(path) = &self.skills {
                    NodeConfiguration { values: serde_json::from_value(json!({
                        "bundled":self.bundled_skills,
                        "home":self.standard_sources.then_some(path),
                        "projectRoots":if self.standard_sources {project_roots.clone()} else {json!([])},
                        "sources":if self.standard_sources {json!([])} else {json!([{"id":"fixture","label":"Fixture","path":path}])}, "manage":false, "preferences":false
                    })).unwrap(), parameters: BTreeMap::new() }
                } else if self.processes || self.protocol {
                    NodeConfiguration { values: serde_json::from_value(json!({
                        "filesystem":{"default":"write","rules":[],"denyGlobs":[]}, "network":"allowed", "helper":null, "sessionOwner":id(1), "isolatedWorkspaces":self.isolated_workspaces
                    })).unwrap(), parameters: BTreeMap::new() }
                } else { Default::default() },
                parameters: Default::default(),
                disabled: false,
                overrides: vec![],
                bindings: BTreeMap::new(),
            },
        );
        packages.push(package);
        if self.processes || self.protocol {
            let consumer = &packages[1];
            let mut files = consumer.files().clone();
            let mut manifest = consumer.manifest().clone();
            manifest
                .application
                .as_mut()
                .unwrap()
                .entries
                .get_mut(&EntryTarget::Host)
                .unwrap()
                .imports
                .insert(
                    if self.protocol {
                        "protocol"
                    } else {
                        "terminal"
                    }
                    .into(),
                    Import {
                        contract: if self.protocol {
                            crate::processes::protocol::contract()
                        } else {
                            crate::processes::terminal::contract()
                        },
                        cardinality: Cardinality::One,
                    },
                );
            files.insert(MANIFEST_FILE.into(), serde_json::to_vec(&manifest).unwrap());
            let consumer = Package::new(files).unwrap();
            for name in ["a", "b"] {
                let node = input.instances.get_mut(name).unwrap();
                node.package = consumer.application_ref().into();
                node.bindings.insert(
                    if self.protocol {
                        "protocol"
                    } else {
                        "terminal"
                    }
                    .into(),
                    vec![Target::Instance {
                        path: vec!["service".into()],
                        export: if self.protocol {
                            "protocol"
                        } else {
                            "terminal"
                        }
                        .into(),
                    }],
                );
            }
            packages[1] = consumer;
        }
    }
    pub(super) fn provide(&self, registry: &mut NativeRegistry) {
        if self.jev {
            crate::jev::provide(registry).unwrap();
        }
        if self.web {
            crate::web::provide(registry).unwrap();
        }
        if let Some(probe) = &self.preparation {
            let probe = probe.clone();
            registry
                .insert(
                    crate::skills::IMPLEMENTATION.into(),
                    Arc::new(move |_, _| {
                        let probe = probe.clone();
                        Box::pin(async move {
                            NativeActivation {
                                exports: Ok(BTreeMap::from([(
                                    crate::skills::EXPORT.into(),
                                    Published {
                                        contract: crate::skills::contract(),
                                        value: probe.clone() as Arc<dyn Handler>,
                                    },
                                )])),
                                owner: probe,
                            }
                        })
                    }),
                )
                .unwrap();
        } else if self.skills.is_some() {
            crate::skills::provide(registry).unwrap();
        }
        if let Some(source) = &self.images {
            source.provide(registry);
        }
        if self.processes || self.protocol {
            crate::processes::provide(registry).unwrap();
        }
        if self.files.is_some() {
            crate::files::provide(registry).unwrap();
        }
        if self.forms {
            crate::forms::provide(registry).unwrap();
        }
        if self.questions {
            crate::questions::provide(registry).unwrap();
        }
        if let Some(counter) = &self.counter {
            let counter = counter.clone();
            registry
                .insert(
                    "fixture.counter".into(),
                    Arc::new(move |_, _| {
                        let counter = counter.clone();
                        Box::pin(async move {
                            NativeActivation {
                                exports: Ok(BTreeMap::from([(
                                    "Counter".into(),
                                    Published {
                                        contract: counter_contract(),
                                        value: counter.clone() as Arc<dyn Handler>,
                                    },
                                )])),
                                owner: counter,
                            }
                        })
                    }),
                )
                .unwrap();
        }
    }
    pub(super) fn policy(&self) -> Arc<maka_application_bridge::transport::PeerPolicy> {
        use maka_application_bridge::transport::{NativeGrant, PeerPolicy};
        Arc::new(
            PeerPolicy::with_native_grants(if self.granted {
                vec![NativeGrant {
                    origin: id(1),
                    consumer: id(1),
                    provider: id(21),
                    export: self.export().into(),
                    methods: self
                        .selections()
                        .into_iter()
                        .map(|tool| tool.method)
                        .chain(self.prepare_skills.then(|| "prepare".into()))
                        .chain(self.web.then(|| "capture".into()))
                        .chain(
                            self.protocol
                                .then_some(["run".to_owned(), "input".to_owned()])
                                .into_iter()
                                .flatten(),
                        )
                        .collect(),
                    recovery: true,
                }]
            } else {
                vec![]
            })
            .unwrap(),
        )
    }
}
async fn tool_response(socket: &mut TcpStream) {
    tool_response_input(socket, json!({"amount":1})).await;
}
async fn tool_response_input(socket: &mut TcpStream, input: Value) {
    let delta = json!({"id":"tool-model","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"counter-call","type":"function","function":{"name":"Counter","arguments":input.to_string()}}]},"finish_reason":null}]});
    let body = format!(
        "data: {delta}\n\ndata: {{\"id\":\"tool-model\",\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"tool_calls\"}}]}}\n\ndata: [DONE]\n\n"
    );
    socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
}
pub(super) async fn facts(fixture: &Fixture, session: &str) -> Vec<Fact> {
    fixture
        .store
        .scoped_prefix(
            maka_runtime::event::LogScope::Session { id: session.into() },
            256,
            4 * 1024 * 1024,
        )
        .await
        .unwrap()
        .events
        .into_iter()
        .map(|event| event.event.fact)
        .collect()
}

async fn pending_approval(
    fixture: &Fixture,
    session: &str,
) -> maka_runtime::interaction::InteractionRecord {
    let mut commits = fixture.store.subscribe_commits();
    loop {
        if let Some(record) = fixture
            .store
            .pending_interactions(session)
            .await
            .unwrap()
            .into_iter()
            .next()
        {
            assert!(matches!(
                record.request,
                maka_runtime::interaction::InteractionRequest::ServiceApproval { .. }
            ));
            return record;
        }
        commits.changed().await.unwrap();
    }
}

#[tokio::test]
async fn service_confirmation_reviews_exact_large_arguments_and_never_replaces_grant_admission() {
    tokio::time::timeout(Duration::from_secs(60), async {
        for (granted, query, decision) in [(true, false, "allow"), (true, false, "deny"), (false, false, "allow"), (true, true, "allow")] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let counter = Counter::new(Behavior::Returned);
            let mut setup = Setup::native(counter.clone(), granted).titled(maka_runtime::display::Text::localized("Review counter", "审核计数器", "審核計數器"));
            setup.query = query;
            setup.confirmation = contract::ToolConfirmation::Required;
            let fixture = Fixture::with_services(&format!("http://{}/v1", listener.local_addr().unwrap()), 1, None, None, Some(setup)).await;
            let session = fixture.create("Review exact service call").await;
            fixture.send(&session).await.unwrap();
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            let input = json!({"amount":7,"note":"界🙂\n".repeat(5000)});
            tool_response_input(&mut socket, input.clone()).await;
            let record = pending_approval(&fixture, &session).await;
            assert_eq!(counter.effects.load(Ordering::SeqCst), 0);
            assert_eq!(counter.entered.available_permits(), 0);
            let maka_runtime::interaction::InteractionRequest::ServiceApproval { through, ref tool_use_id, .. } = record.request else { unreachable!() };
            assert_eq!(fixture.query("observe", json!({"sessionId":session})).await.unwrap()["run"]["state"], "waiting_for_user");
            let pending = fixture.query("pendingInteractions", json!({"sessionId":session})).await.unwrap();
            assert_eq!(pending["entries"][0]["kind"], "service_approval");
            assert_eq!(pending["entries"][0]["toolUseId"], *tool_use_id);
            let mut position = json!({"kind":"start"});
            let mut complete = String::new();
            loop {
                let page = fixture.query("source", json!({"sessionId":session,"through":through,
                    "source":{"kind":"tool_use","toolUseId":tool_use_id},"field":"arguments","position":position})).await.unwrap();
                if page["kind"] == "preparing" { continue; }
                assert_eq!(page["source"]["tool"]["toolUseId"], *tool_use_id);
                assert_eq!(page["source"]["tool"]["call"]["title"]["translations"]["zh-CN"], "审核计数器");
                assert_eq!(page["source"]["tool"]["call"]["pending"], true);
                assert!(page["source"]["tool"]["result"].is_null());
                let text = page["source"]["text"].as_str().unwrap();
                assert!(text.len() <= 8192);
                complete.push_str(text);
                let end = page["source"]["offset"].as_u64().unwrap() + text.len() as u64;
                if end == page["source"]["totalBytes"].as_u64().unwrap() { break; }
                position = json!({"kind":"after","offset":end});
            }
            assert_eq!(serde_json::from_str::<Value>(&complete).unwrap(), input);
            let other = fixture.create("Other service call owner").await;
            let wrong = fixture.mutation(2, "respond", json!({"sessionId":other,"requestId":record.request_id,
                "answer":{"kind":"service_approval","decision":"allow"}}), Uuid::new_v4()).await.unwrap();
            assert_eq!(wrong["reason"], "unavailable");
            let wrong = fixture.mutation(2, "respond", json!({"sessionId":session,"requestId":record.request_id,
                "answer":{"kind":"question","answers":["yes"]}}), Uuid::new_v4()).await.unwrap();
            assert_eq!(wrong["reason"], "invalid_answer");
            let operation = Uuid::new_v4();
            let answer = json!({"sessionId":session,"requestId":record.request_id,"answer":{"kind":"service_approval","decision":decision}});
            let receipt = fixture.mutation(2, "respond", answer.clone(), operation).await.unwrap();
            assert_eq!(receipt["record"]["outcome"]["decision"], decision);
            assert_eq!(fixture.mutation(2, "respond", answer, operation).await.unwrap(), receipt);
            let opposite = if decision == "allow" { "deny" } else { "allow" };
            assert_eq!(fixture.mutation(2, "respond", json!({"sessionId":session,"requestId":record.request_id,
                "answer":{"kind":"service_approval","decision":opposite}}), Uuid::new_v4()).await.unwrap(), receipt);
            let (mut resumed, _) = listener.accept().await.unwrap();
            request(&mut resumed).await;
            response(&mut resumed).await;
            fixture.drained().await;
            let admitted = granted && decision == "allow";
            assert_eq!(counter.entered.available_permits(), usize::from(admitted));
            assert_eq!(counter.effects.load(Ordering::SeqCst), if admitted && !query { 7 } else { 0 });
            fixture.app.shutdown().await.unwrap();
            fixture.config.shutdown().await.unwrap();
        }
    }).await.unwrap();
}

#[tokio::test]
async fn stop_closes_original_service_confirmation_without_an_effect_or_replay() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let counter = Counter::new(Behavior::Returned);
        let mut setup = Setup::native(counter.clone(), true);
        setup.confirmation = contract::ToolConfirmation::Required;
        let fixture = Fixture::with_services(
            &format!("http://{}/v1", listener.local_addr().unwrap()),
            1,
            None,
            None,
            Some(setup),
        )
        .await;
        let session = fixture.create("Stop service confirmation").await;
        let started = fixture.send(&session).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        request(&mut socket).await;
        tool_response(&mut socket).await;
        let record = pending_approval(&fixture, &session).await;
        fixture
            .mutation(
                2,
                "stop",
                json!({"sessionId":session,"invocationId":started["invocationId"]}),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        fixture.drained().await;
        let late = fixture
            .mutation(
                2,
                "respond",
                json!({"sessionId":session,"requestId":record.request_id,
            "answer":{"kind":"service_approval","decision":"allow"}}),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        assert_eq!(late["record"]["outcome"]["kind"], "closure");
        assert_eq!(counter.effects.load(Ordering::SeqCst), 0);
        assert_eq!(counter.entered.available_permits(), 0);
        assert!(
            fixture
                .store
                .pending_interactions(&session)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(fixture.owner.permits.available_permits(), 1);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn installed_javascript_service_is_a_real_model_tool_with_business_projection_and_original_receipt()
 {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let first = request(&mut socket).await;
            assert_eq!(first["tools"].as_array().unwrap().len(), 1);
            assert_eq!(first["tools"][0]["function"]["name"], "Counter");
            assert_eq!(
                first["tools"][0]["function"]["parameters"]["required"],
                json!(["amount"])
            );
            tool_response(&mut socket).await;
            let (mut socket, _) = listener.accept().await.unwrap();
            let next = request(&mut socket).await;
            let result = next["messages"]
                .as_array()
                .unwrap()
                .iter()
                .find(|message| message["role"] == "tool")
                .unwrap();
            assert_eq!(result["tool_call_id"], "counter-call");
            assert_eq!(
                serde_json::from_str::<Value>(result["content"].as_str().unwrap()).unwrap(),
                json!({"effects":1})
            );
            assert!(!next.to_string().contains("stateDomain"));
            response(&mut socket).await;
        });
        let fixture = Fixture::with_services(&base, 1, None, None, Some(Setup::javascript())).await;
        let session = fixture.create("Actual service tool").await;
        fixture.send(&session).await.unwrap();
        fixture.drained().await;
        let observed = facts(&fixture, &session).await;
        server.await.unwrap();
        let facts = observed;
        assert_eq!(
            facts
                .iter()
                .filter(|fact| matches!(fact, Fact::ToolDispatched { .. }))
                .count(),
            1
        );
        let outcome = facts
            .iter()
            .find_map(|fact| {
                if let Fact::ToolSettled { outcome, .. } = fact {
                    Some(outcome)
                } else {
                    None
                }
            })
            .unwrap();
        let ToolOutcome::Succeeded {
            model_projection, ..
        } = outcome
        else {
            panic!("{outcome:?}")
        };
        assert_eq!(
            model_projection,
            &DurableToolProjection::Json {
                value: json!({"effects":1})
            }
        );
        let prefix = fixture
            .store
            .scoped_prefix(
                maka_runtime::event::LogScope::Session {
                    id: session.clone(),
                },
                256,
                4 * 1024 * 1024,
            )
            .await
            .unwrap();
        let event = prefix
            .events
            .iter()
            .find(|event| matches!(event.event.fact, Fact::ToolSettled { .. }))
            .unwrap();
        let raw = fixture
            .store
            .resolve_tool_result(&session, &event.event.id)
            .await
            .unwrap();
        let text = serde_json::to_string(&raw).unwrap();
        assert!(text.contains("journal_v1"));
        assert!(text.contains("00000000-0000-0000-0000-000000000015"));
        assert!(matches!(
            raw,
            maka_runtime::tool_output::ToolOutput::Operation(_)
        ));
        let through = *fixture.store.subscribe_commits().borrow();
        while !fixture
            .store
            .prepare_transcript(&session, through, 32)
            .await
            .unwrap()
        {}
        use maka_plugins::session::history::{
            Page, Position, SourceField, SourceFormat, SourcePosition,
        };
        let Page::Ready { chunks, .. } = fixture
            .store
            .history_text(
                &session,
                through,
                Position::Start,
                false,
                8192,
                8,
                64 * 1024,
            )
            .await
            .unwrap()
        else {
            panic!("ready")
        };
        let result = chunks
            .iter()
            .find(|chunk| chunk.role == maka_plugins::session::history::Role::ToolResult)
            .unwrap();
        assert!(
            result.text.is_empty(),
            "numeric business result must not display receipt strings"
        );
        for (field, expected) in [
            (
                SourceField::Result,
                serde_json::to_string(&json!({"effects":1})).unwrap(),
            ),
            (
                SourceField::Receipt,
                serde_json::to_string(&raw.to_json()).unwrap(),
            ),
        ] {
            for reverse in [false, true] {
                let mut position = if reverse {
                    SourcePosition::Tail
                } else {
                    SourcePosition::Start
                };
                let mut actual = String::new();
                loop {
                    let source = fixture
                        .store
                        .history_source(
                            &session,
                            through,
                            result.sequence,
                            field,
                            None,
                            position,
                            13,
                            64 * 1024,
                        )
                        .await
                        .unwrap();
                    assert_eq!(source.format, SourceFormat::Json);
                    assert_eq!(
                        source.fields,
                        [SourceField::Body, SourceField::Result, SourceField::Receipt]
                    );
                    assert_eq!(source.total_bytes, expected.len() as u64);
                    if reverse {
                        actual.insert_str(0, &source.text);
                    } else {
                        actual.push_str(&source.text);
                    }
                    let end = source.offset + source.text.len() as u64;
                    if (reverse && source.offset == 0) || (!reverse && end == source.total_bytes) {
                        break;
                    }
                    position = if reverse {
                        SourcePosition::Before {
                            offset: source.offset,
                        }
                    } else {
                        SourcePosition::After { offset: end }
                    };
                }
                assert_eq!(actual, expected);
            }
        }
        let matches = fixture
            .store
            .search_transcript(
                &session,
                maka_event_log::transcript::search::TranscriptSearch {
                    through,
                    after: 0,
                    query: "journal_v1".into(),
                    include_internal: true,
                    max_matches: 64,
                },
            )
            .await
            .unwrap();
        assert!(
            matches.matches.is_empty(),
            "receipt metadata is not conversation search text"
        );
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn unknown_service_outcome_has_typed_original_evidence_and_never_enters_model_diagnostic() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            tool_response(&mut socket).await;
            let (mut socket, _) = listener.accept().await.unwrap();
            let next = request(&mut socket).await;
            let text = next.to_string();
            assert!(text.contains("result is unknown"));
            assert!(!text.contains("journal_v1"));
            assert!(!text.contains("fixture.Counter"));
            response(&mut socket).await;
        });
        let counter = Counter::new(Behavior::Unknown);
        let fixture = Fixture::with_services(
            &base,
            1,
            None,
            None,
            Some(Setup::native(counter.clone(), true)),
        )
        .await;
        let session = fixture.create("Unknown original effect").await;
        fixture.send(&session).await.unwrap();
        fixture.drained().await;
        let observed = facts(&fixture, &session).await;
        server.await.unwrap();
        let facts = observed;
        let outcome = facts
            .iter()
            .find_map(|fact| {
                if let Fact::ToolSettled { outcome, .. } = fact {
                    Some(outcome)
                } else {
                    None
                }
            })
            .unwrap();
        let ToolOutcome::Unknown {
            operation: Some(operation),
            message,
        } = outcome
        else {
            panic!("{outcome:?}")
        };
        assert_eq!(counter.effects.load(Ordering::SeqCst), 1);
        assert_eq!(operation.reference.namespace, id(21));
        assert_eq!(operation.reference.method, "write");
        assert!(matches!(
            operation.receipt,
            Receipt::Recorded {
                outcome: OperationOutcome::Unknown { .. },
                cleanup: maka_plugin_core::operation::OperationCleanup::Confirmed
            }
        ));
        assert_eq!(
            outcome_projection(outcome).as_ref(),
            &DurableToolProjection::Text {
                text: message.clone()
            }
        );
        let recovered = fixture
            .store
            .operation_observation(&operation.reference)
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            recovered.outcome,
            OperationOutcome::Unknown { .. }
        ));
        assert_eq!(fixture.owner.permits.available_permits(), 1);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn unlisted_native_tool_edge_never_executes_the_selected_provider() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            tool_response(&mut socket).await;
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            response(&mut socket).await;
        });
        let counter = Counter::new(Behavior::Returned);
        let fixture = Fixture::with_services(
            &base,
            1,
            None,
            None,
            Some(Setup::native(counter.clone(), false)),
        )
        .await;
        let session = fixture.create("No implicit grant").await;
        fixture.send(&session).await.unwrap();
        fixture.drained().await;
        server.await.unwrap();
        assert_eq!(counter.effects.load(Ordering::SeqCst), 0);
        let actual = facts(&fixture, &session).await;
        assert!(actual.iter().any(|fact| matches!(
            fact,
            Fact::ToolSettled {
                outcome: ToolOutcome::Failed { .. },
                ..
            }
        )));
        assert!(!actual.iter().any(|fact| matches!(
            fact,
            Fact::ToolSettled {
                outcome: ToolOutcome::Unknown { .. },
                ..
            }
        )));
        assert!(!facts(&fixture, &session).await.iter().any(|fact| matches!(
            fact,
            Fact::ToolSettled {
                outcome: ToolOutcome::Succeeded { .. },
                ..
            }
        )));
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn stopping_real_agent_run_cancels_tool_but_retains_capacity_until_physical_cleanup() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            tool_response(&mut socket).await;
        });
        let counter = Counter::new(Behavior::Blocked);
        let fixture = Fixture::with_services(
            &base,
            1,
            None,
            None,
            Some(Setup::native(counter.clone(), true)),
        )
        .await;
        let session = fixture.create("Stop original tool").await;
        let started = fixture.send(&session).await.unwrap();
        counter.entered.acquire().await.unwrap().forget();
        fixture
            .mutation(
                2,
                "stop",
                json!({"sessionId":session,"invocationId":started["invocationId"]}),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        counter.cancelled.acquire().await.unwrap().forget();
        assert_eq!(fixture.owner.permits.available_permits(), 0);
        assert_eq!(fixture.owner.workers.len(), 1);
        let other = fixture.create("Not available yet").await;
        assert!(
            fixture
                .send(&other)
                .await
                .unwrap_err()
                .contains("agent_capacity_exhausted")
        );
        counter.release.add_permits(1);
        fixture.drained().await;
        server.await.unwrap();
        assert_eq!(counter.effects.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.owner.permits.available_permits(), 1);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn returned_effect_with_failed_cleanup_preserves_receipt_without_settling_or_reusing_capacity()
 {
    tokio::time::timeout(Duration::from_secs(30), async {
        for behavior in [Behavior::CleanupFailed, Behavior::LargeCleanupFailed] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}/v1", listener.local_addr().unwrap());
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                request(&mut socket).await;
                tool_response(&mut socket).await;
            });
            let counter = Counter::new(behavior);
            let fixture = Fixture::with_services(
                &base,
                1,
                None,
                None,
                Some(Setup::native(counter.clone(), true)),
            )
            .await;
            let session = fixture.create("Failed original cleanup").await;
            fixture.send(&session).await.unwrap();
            fixture.drained().await;
            let observed = facts(&fixture, &session).await;
            server.await.unwrap();
            let facts = observed;
            assert!(
                !facts
                    .iter()
                    .any(|fact| matches!(fact, Fact::ToolSettled { .. }))
            );
            let observation = facts
                .iter()
                .find_map(|fact| {
                    if let Fact::ToolOperationObserved { observation, .. } = fact {
                        Some(observation)
                    } else {
                        None
                    }
                })
                .unwrap();
            assert_eq!(observation.reference.namespace, id(21));
            if matches!(behavior, Behavior::LargeCleanupFailed) {
                assert!(serde_json::to_vec(observation).unwrap().len() > 1024 * 1024);
            }
            assert!(matches!(
                observation.receipt,
                Receipt::Recorded {
                    outcome: OperationOutcome::Returned { .. },
                    cleanup: maka_plugin_core::operation::OperationCleanup::Failed { .. }
                }
            ));
            assert_eq!(fixture.owner.permits.available_permits(), 0);
            assert_eq!(fixture.owner.runs.lock().await.quarantined.len(), 1);
            assert!(fixture.app.shutdown().await.is_err());
            assert!(!fixture.app.client().snapshot().closed);
            fixture.config.shutdown().await.unwrap();
            // Failed cleanup does not authorize destruction of the original Root.
            eprintln!(
                "Retained failed-cleanup Root: {}",
                fixture._dir.keep().display()
            );
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn query_tool_uses_original_query_gate_and_projects_plain_business_json() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            tool_response(&mut socket).await;
            let (mut socket, _) = listener.accept().await.unwrap();
            let next = request(&mut socket).await;
            let result = next["messages"]
                .as_array()
                .unwrap()
                .iter()
                .find(|message| message["role"] == "tool")
                .unwrap();
            assert_eq!(
                serde_json::from_str::<Value>(result["content"].as_str().unwrap()).unwrap(),
                json!({"effects":0})
            );
            response(&mut socket).await;
        });
        let counter = Counter::new(Behavior::Returned);
        let mut setup = Setup::native(counter.clone(), true);
        setup.query = true;
        let fixture = Fixture::with_services(&base, 1, None, None, Some(setup)).await;
        let session = fixture.create("Actual query tool").await;
        fixture.send(&session).await.unwrap();
        fixture.drained().await;
        server.await.unwrap();
        let prefix = fixture
            .store
            .scoped_prefix(
                maka_runtime::event::LogScope::Session {
                    id: session.clone(),
                },
                256,
                4 * 1024 * 1024,
            )
            .await
            .unwrap();
        let event = prefix
            .events
            .iter()
            .find(|event| {
                matches!(
                    event.event.fact,
                    Fact::ToolSettled {
                        outcome: ToolOutcome::Succeeded { .. },
                        ..
                    }
                )
            })
            .unwrap();
        assert_eq!(
            fixture
                .store
                .resolve_tool_result(&session, &event.event.id)
                .await
                .unwrap(),
            maka_runtime::tool_output::ToolOutput::Json(json!({"effects":0}))
        );
        assert_eq!(counter.effects.load(Ordering::SeqCst), 0);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn unresponsive_native_tool_reaches_original_cleanup_budget_and_keeps_host_work_owned() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            tool_response(&mut socket).await;
        });
        let counter = Counter::new(Behavior::Blocked);
        let fixture = Fixture::with_services(
            &base,
            1,
            None,
            None,
            Some(Setup::native(counter.clone(), true)),
        )
        .await;
        let session = fixture.create("Unresponsive original tool").await;
        let started = fixture.send(&session).await.unwrap();
        counter.entered.acquire().await.unwrap().forget();
        fixture
            .mutation(
                2,
                "stop",
                json!({"sessionId":session,"invocationId":started["invocationId"]}),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        counter.cancelled.acquire().await.unwrap().forget();
        // Keep the physical original callback blocked beyond the real 5s budget.
        // Old observer-first code never drained here, even after that budget.
        tokio::time::timeout(Duration::from_secs(8), fixture.drained())
            .await
            .unwrap();
        assert_eq!(fixture.owner.permits.available_permits(), 0);
        assert_eq!(fixture.owner.runs.lock().await.quarantined.len(), 1);
        let snapshot = fixture.app.client().snapshot();
        assert!(
            snapshot
                .instances
                .iter()
                .any(|status| status.activation.instance() == &id(21) && status.calls > 0)
        );
        assert!(
            !facts(&fixture, &session)
                .await
                .iter()
                .any(|fact| matches!(fact, Fact::ToolSettled { .. }))
        );
        assert_eq!(counter.effects.load(Ordering::SeqCst), 1);
        counter.release.add_permits(1);
        server.await.unwrap();
        assert!(fixture.app.shutdown().await.is_err());
        assert!(!fixture.app.client().snapshot().closed);
        fixture.config.shutdown().await.unwrap();
        eprintln!(
            "Retained unresponsive-tool Root: {}",
            fixture._dir.keep().display()
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn admitted_message_keeps_original_model_workspace_and_dispatcher_when_opening_fails() {
    let fixture = Fixture::with_services(
        "http://127.0.0.1:1/v1",
        1,
        None,
        None,
        Some(Setup::javascript()),
    )
    .await;
    let session = fixture.create("Pending original basis").await;
    let raw =
        rusqlite::Connection::open(fixture.root.join(maka_event_log::root::ROOT_DATABASE)).unwrap();
    raw.execute_batch("CREATE TRIGGER hold_opening BEFORE INSERT ON event_log WHEN NEW.kind='invocation_opened' BEGIN SELECT RAISE(ABORT, 'fixture opening failure'); END;").unwrap();
    let operation = Uuid::new_v4();
    assert!(fixture.mutation(2, "send", json!({
        "sessionId":session,"message":"durable original", "model":fixture.selection().await
    }), operation).await.is_err());
    fixture.drained().await;
    let queued = fixture.store.pending_messages(&session).await.unwrap();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].execution.owner, fixture.owner.prefix);
    let encoded = serde_json::to_string(&queued[0].execution.basis).unwrap();
    assert!(!encoded.contains("private-fixture-key"));
    let basis: MessageBasis = serde_json::from_str(&encoded).unwrap();
    assert_eq!(basis.model.unwrap().binding.model, "fixture-model");
    assert_eq!(
        basis.workspace,
        fixture
            .owner
            .session(&session)
            .await
            .unwrap()
            .configuration
            .workspace
    );
    assert!(!basis.fingerprint.is_empty());
    assert_eq!(basis.tools.len(), 1);
    assert!(basis.tools == fixture.owner.tool_basis);
    assert_eq!(basis.max_steps, fixture.owner.settings.max_steps);
    assert_eq!(
        fixture
            .store
            .pending_message_sessions(&fixture.owner.prefix, None)
            .await
            .unwrap(),
        [session]
    );
    assert!(
        fixture
            .store
            .pending_message_sessions("maka-runtime-host", None)
            .await
            .unwrap()
            .is_empty()
    );
    raw.execute_batch("DROP TRIGGER hold_opening").unwrap();
    drop(raw);
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}

#[tokio::test]
async fn queue_promotion_steers_original_run_at_the_next_step_without_a_successor() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let fixture = Fixture::with_services(&base, 1, None, None, Some(Setup::javascript())).await;
        let session = fixture.create("Steer original run").await;
        let started = fixture.send(&session).await.unwrap();
        let (mut first, _) = listener.accept().await.unwrap();
        request(&mut first).await;
        let queued = fixture.mutation(2, "send", json!({"sessionId":session,"message":"steering instruction","model":fixture.selection().await}), Uuid::new_v4()).await.unwrap();
        let promoted = fixture.mutation(2, "editQueue", json!({"sessionId":session,"expectedRevision":queued["revision"],"edit":{"kind":"promote","messageId":queued["messageId"]}}), Uuid::new_v4()).await.unwrap();
        assert_eq!(promoted["kind"], "committed");
        assert_eq!(fixture.query("queue", json!({"sessionId":session})).await.unwrap()["entries"][0]["kind"], "steering");
        tool_response(&mut first).await;
        let (mut second, _) = listener.accept().await.unwrap();
        let next = request(&mut second).await;
        assert!(next["messages"].as_array().unwrap().iter().any(|message| message["content"] == "The user sent a message while you were working:\n<user_query>\nsteering instruction\n</user_query>"));
        assert_eq!(fixture.query("observe", json!({"sessionId":session})).await.unwrap()["run"]["invocationId"], started["invocationId"]);
        response(&mut second).await;
        fixture.drained().await;
        assert!(fixture.store.pending_messages(&session).await.unwrap().is_empty());
        let work=fixture.query("work",json!({"sessionId":session,"target":{"kind":"message","messageId":queued["messageId"]}})).await.unwrap();
        assert_eq!(work["kind"],"delivered");assert_eq!(work["shared"],true);
        assert_eq!(work["execution"]["invocationId"],started["invocationId"]);
        assert_eq!(work["answer"]["text"],"Elegant response");

        let recorded = facts(&fixture, &session).await;
        assert_eq!(recorded.iter().filter(|fact| matches!(fact, Fact::InvocationOpened { .. })).count(), 1);
        assert!(recorded.iter().any(|fact| matches!(fact, Fact::MessageSteered { .. })));
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}

// The provider finishes its physical work after cancellation, independently of the observer.
struct PreparationProbe {
    entered: Arc<Semaphore>,
    cancelled: Arc<Semaphore>,
    release: Arc<Semaphore>,
    cleanup_failed: bool,
}
impl Handler for PreparationProbe {
    fn call(
        &self,
        method: String,
        _: Value,
        context: Context,
    ) -> BoxFuture<'static, Result<Value, String>> {
        assert_eq!(method, "prepare");
        let mut ticket = context.scope().resources.reserve().unwrap();
        ticket.start();
        self.entered.add_permits(1);
        let failed = self.cleanup_failed;
        let cancelled = self.cancelled.clone();
        let release = self.release.clone();
        Box::pin(async move {
            context.scope().cancellation.cancelled().await;
            cancelled.add_permits(1);
            release.acquire().await.unwrap().forget();
            if failed {
                ticket.complete(Err("preparation cleanup failed".into()));
            } else {
                ticket.complete(Ok(()));
            }
            Err("preparation cancelled".into())
        })
    }
}
impl NativeOwner for PreparationProbe {
    fn close(&self) -> BoxFuture<'static, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }
}

#[tokio::test]
async fn skill_preparation_parent_scope_owns_work_after_observer_loss_and_cleanup_failure() {
    tokio::time::timeout(Duration::from_secs(60), async {
        for failed in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let probe = Arc::new(PreparationProbe {
                entered: Arc::new(Semaphore::new(0)),
                cancelled: Arc::new(Semaphore::new(0)),
                release: Arc::new(Semaphore::new(0)),
                cleanup_failed: failed,
            });
            let mut setup = Setup::skill_preparation(dir.path().into(), false);
            setup.preparation = Some(probe.clone());
            let f =
                Fixture::with_services("http://127.0.0.1:9/v1", 1, None, None, Some(setup)).await;
            let parent = scope(&f.issuer, true);
            let owner = f.owner.clone();
            let request = parent.clone();
            let observer = tokio::spawn(async move {
                owner
                    .prepare_skills("hello".into(), None, &request, vec![])
                    .await
            });
            probe.entered.acquire().await.unwrap().forget();
            observer.abort();
            assert!(matches!(observer.await, Err(error) if error.is_cancelled()));
            let cleanup = tokio::spawn(async move { parent.finish().await });
            probe.cancelled.acquire().await.unwrap().forget();
            assert!(
                !cleanup.is_finished(),
                "original parent cannot settle while provider work remains"
            );
            probe.release.add_permits(1);
            assert_eq!(cleanup.await.unwrap().is_err(), failed);
            assert_eq!(f.owner.permits.available_permits(), 1);
            // The same physical cleanup failure also remains visible to the Host owner.
            assert_eq!(f.app.shutdown().await.is_err(), failed);
            f.config.shutdown().await.unwrap();
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn resume_replays_original_settlements_without_redispatching_effects() {
    tokio::time::timeout(Duration::from_secs(60), async {
        for behavior in [Behavior::Returned, Behavior::Unknown] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let counter = Counter::new(behavior);
            let fixture = Fixture::with_services(
                &format!("http://{}/v1", listener.local_addr().unwrap()), 1, None, None,
                Some(Setup::native(counter.clone(), true)),
            ).await;
            let session = fixture.create("Resume settled tool").await;
            let sent = fixture.send(&session).await.unwrap();
            let (mut first, _) = listener.accept().await.unwrap();
            request(&mut first).await;
            tool_response(&mut first).await;
            let (mut next, _) = listener.accept().await.unwrap();
            request(&mut next).await;
            assert_eq!(counter.effects.load(Ordering::SeqCst), 1);
            fixture.mutation(2, "stop", json!({"sessionId":session,"invocationId":sent["invocationId"]}), Uuid::new_v4()).await.unwrap();
            fixture.drained().await;
            let source = fixture.query("execution", json!({"sessionId":session,"invocationId":sent["invocationId"]})).await.unwrap();
            assert_eq!(source["execution"]["state"], "cancelled");
            let resumed = fixture.mutation(2, "resume", json!({
                "sessionId":session, "sourceInvocationId":sent["invocationId"],
                "sourceHighWater":source["execution"]["highWater"], "model":fixture.selection().await
            }), Uuid::new_v4()).await.unwrap();
            assert_eq!(resumed["kind"], "started", "{resumed}");
            let (mut replay, _) = listener.accept().await.unwrap();
            let prompt = request(&mut replay).await;
            let result = prompt["messages"].as_array().unwrap().iter().find(|message| message["role"] == "tool").unwrap();
            if matches!(behavior, Behavior::Returned) {
                assert_eq!(serde_json::from_str::<Value>(result["content"].as_str().unwrap()).unwrap()["effects"], 1);
            } else {
                assert!(result["content"].as_str().unwrap().contains("result is unknown"));
            }
            response(&mut replay).await;
            fixture.drained().await;
            let observed = facts(&fixture, &session).await;
            assert_eq!(observed.iter().filter(|fact| matches!(fact, Fact::ToolDispatched { .. })).count(), 1);
            if matches!(behavior, Behavior::Unknown) {
                let reference = observed.iter().find_map(|fact| match fact {
                    Fact::ToolSettled { outcome: ToolOutcome::Unknown { operation: Some(operation), .. }, .. } => Some(&operation.reference),
                    _ => None,
                }).unwrap();
                let receipt = fixture.store.operation_observation(reference).await.unwrap().unwrap();
                assert!(matches!(receipt.outcome, OperationOutcome::Unknown { .. }));
            }
            assert_eq!(counter.effects.load(Ordering::SeqCst), 1, "replay never redispatches original service mutation");
            assert_eq!(fixture.owner.permits.available_permits(), 1);
            fixture.app.shutdown().await.unwrap();
            fixture.config.shutdown().await.unwrap();
        }
    }).await.unwrap();
}

#[tokio::test]
async fn session_tool_selection_is_an_execution_ceiling_and_is_inherited() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let counter = Counter::new(Behavior::Returned);
        let fixture = Fixture::with_services(
            &format!("http://{}/v1", listener.local_addr().unwrap()), 1, None, None,
            Some(Setup::native(counter.clone(), true)),
        ).await;
        assert_eq!(fixture.query("tools", json!({})).await.unwrap()["names"], json!(["Counter"]));
        let mut input = fixture.create_input("Restricted worker").await;
        input["tools"] = json!([]);
        let created = fixture.mutation(2, "create", input, Uuid::new_v4()).await.unwrap();
        let session = created["session"]["id"].as_str().unwrap();
        assert_eq!(created["session"]["tools"], json!([]));
        let inherited = fixture.mutation(2, "create", json!({"title":"Inherited", "labels":[], "tools":["Counter"],
            "workspace":{"kind":"session","sessionId":session}}), Uuid::new_v4()).await.unwrap();
        assert_eq!(inherited["session"]["tools"], json!([]));
        fixture.send(session).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        let input = request(&mut socket).await;
        assert!(input["tools"].is_null() || input["tools"].as_array().unwrap().is_empty());
        tool_response(&mut socket).await;
        let (mut socket, _) = listener.accept().await.unwrap();
        request(&mut socket).await;
        response(&mut socket).await;
        fixture.drained().await;
        assert_eq!(counter.effects.load(Ordering::SeqCst), 0);
        assert!(facts(&fixture, session).await.iter().any(|fact| matches!(fact, Fact::ToolRejected { name, reason: maka_runtime::tool_call::ToolRejection::Unavailable, .. } if name == "Counter")));
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}
