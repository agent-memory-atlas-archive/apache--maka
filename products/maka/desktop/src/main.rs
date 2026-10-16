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

#![cfg_attr(windows, windows_subsystem = "windows")]

extern crate gpui_kit as gpui;
mod actions;
mod owned;
mod resources;
use maka_render_gpui as widgets;
mod window_owner;
use clap::Parser;
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Render, Styled, div};
#[cfg(test)]
use gpui_kit::{
    App, AppContext, Bounds, InteractiveElement, TestAppContext, WindowBounds, WindowOptions,
    point, px, size,
};
use maka_plugin_core::{composition::PresentationTarget, runtime::Mount};
use maka_ui_runtime::{Application, NativeView, Surface as SurfaceDeclaration};
type DesktopApplication = Application<dyn widgets::Widget>;
use std::{cell::RefCell, path::PathBuf, rc::Rc, sync::Arc};

#[derive(Parser)]
#[command(
    name = "maka desktop",
    bin_name = "maka desktop",
    version,
    about = "Present the selected plugin product"
)]
pub(crate) struct Args {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) deployment: uuid::Uuid,
    /// Trusted local Host connections and explicit method/recovery grants.
    #[arg(long, value_name = "FILE")]
    pub(crate) connections: Option<PathBuf>,
}

struct Surface {
    application: Rc<RefCell<DesktopApplication>>,
    resources: Rc<RefCell<resources::NativeResources>>,
    mount: Mount,
}
impl Surface {
    /// The returned future owns only the storage snapshot read, not this entity.
    #[cfg(test)]
    fn load_committed_update(
        &self,
        source: maka_plugin_core::deployment::DeploymentRef,
    ) -> maka_ui_runtime::Result<
        impl std::future::Future<Output = maka_ui_runtime::Result<maka_ui_runtime::CommittedUpdate>>
        + Send
        + 'static
        + use<>,
    > {
        self.application.borrow().load_committed_update(source)
    }
    /// Desired-input acknowledgement is distinct from native synchronization.
    #[cfg(test)]
    fn adopt_update(
        &self,
        update: maka_ui_runtime::CommittedUpdate,
        registrations: Vec<maka_ui_runtime::actions::RemoteService>,
        window: &mut gpui_kit::Window,
        cx: &mut Context<Self>,
    ) -> maka_ui_runtime::Result<()> {
        self.application
            .borrow_mut()
            .adopt_update(update, registrations)?;
        if let Err(error) = self.sync_native(window, cx) {
            eprintln!("deployment native synchronization failed: {error}");
        }
        cx.notify();
        Ok(())
    }

    fn input(
        &self,
        lease: &resources::Lease,
        basis: (u64, u64),
        window_id: gpui_kit::WindowId,
        cx: &Context<Self>,
    ) -> widgets::Input {
        let (_, revision) = basis;
        let observer_lease = lease.clone();
        let observer_view = cx.weak_entity();
        let observer_application = Rc::downgrade(&self.application);
        let observe = Rc::new(
            move |key: &str, event, window: &mut gpui_kit::Window, cx: &mut gpui_kit::App| {
                if window.window_handle().window_id() != window_id || !observer_lease.is_attached()
                {
                    return Err(maka_ui_runtime::Error::Invalid(
                        "original native observation owner closed".into(),
                    ));
                }
                let application = observer_application.upgrade().ok_or_else(|| {
                    maka_ui_runtime::Error::Invalid(
                        "original observation application closed".into(),
                    )
                })?;
                // Publication also advances inside Surface::sync_native. It
                // reaches the existing commit owner without reentering Surface.
                let job = application.borrow_mut().dispatch_observation(
                    observer_lease.mount(),
                    revision,
                    key,
                    event,
                )?;
                cx.notify(observer_view.entity_id());
                Ok(job)
            },
        );
        let lease = lease.clone();
        let view = cx.weak_entity();
        widgets::Input::new(
            basis,
            Rc::new(move |key, event, window, cx| {
                if window.window_handle().window_id() != window_id {
                    return Err(maka_ui_runtime::Error::Invalid(
                        "window owner mismatch".into(),
                    ));
                }
                let result = view.update(cx, |view, cx| {
                    let result =
                        view.application
                            .borrow_mut()
                            .dispatch(lease.mount(), revision, key, event);
                    match result {
                        Ok(()) => (),
                        Err(error) => {
                            // Faulted realms can retain their last display pending
                            // recovery. Revoke their native input without replaying
                            // commands from that already committed frame.
                            if let Err(cleanup) = view.resources.borrow_mut().deactivate_unmounted(
                                &view.application.borrow(),
                                window,
                                cx,
                            ) {
                                eprintln!("component input deactivation failed: {cleanup}");
                            }
                            if let Some(owner) = cx.try_global::<actions::Owner>().cloned()
                                && let Err(cleanup) = owner.recover_launch(&view.mount, window)
                            {
                                eprintln!("recovery surface transition failed: {cleanup}");
                            }
                            cx.notify();
                            return Err(error);
                        }
                    };
                    // Admission is complete. Native defaults do not wait for the
                    // worker; the original owner publishes prepared candidates.
                    if let Err(error) = view.sync_native(window, cx) {
                        eprintln!("component native synchronization failed: {error}");
                    }
                    cx.notify();
                    maka_ui_runtime::Result::Ok(())
                });
                result.map_err(|error| maka_ui_runtime::Error::Invalid(error.to_string()))?
            }),
            observe,
        )
    }
    fn prepare_candidate(
        &self,
        application: &DesktopApplication,
        candidate: maka_ui_runtime::Candidate<'_, dyn widgets::Widget>,
        window: &mut gpui_kit::Window,
        cx: &mut Context<Self>,
    ) -> maka_ui_runtime::Result<Vec<(String, maka_ui_runtime::publication::Capacity)>> {
        for (mount, frame) in candidate.changed() {
            let resources = self.resources.borrow();
            if resources
                .owners
                .get(&mount.id().to_string())
                .is_some_and(|owner| owner.kind != frame.view.view.as_ref().type_id())
            {
                return Err(maka_ui_runtime::Error::Invalid(
                    "native recipe type changed on original mount; remount the component".into(),
                ));
            }
        }
        for (mount, frame) in candidate.changed() {
            let mut resources = self.resources.borrow_mut();
            resources.prepare_scope(mount);
            let id = mount.id().to_string();
            if !resources.owners.contains_key(&id) {
                let lease = resources.acquire(mount);
                let input = widgets::LiveInput::new(
                    Rc::new(RefCell::new(Some(
                        self.input(
                            &lease,
                            (
                                application
                                    .frame(&self.mount)
                                    .map_or(0, |frame| frame.revision),
                                frame.revision,
                            ),
                            window.window_handle().window_id(),
                            cx,
                        ),
                    ))),
                    resources.budget.clone(),
                    lease.resources(),
                    application.wake_callback(),
                )
                .with_checkpoint(application.checkpoint_scope(mount)?);
                let recipe = frame.view.view.clone();
                let state = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    recipe.create(input.clone(), window, cx)
                })) {
                    Ok(state) => state,
                    Err(_) => {
                        let reason = "desktop native creation cleanup unconfirmed".to_owned();
                        resources.preparation_failed(mount, reason.clone());
                        return Err(maka_ui_runtime::Error::Invalid(reason));
                    }
                };
                resources.owners.insert(
                    id,
                    resources::NativeInstance {
                        kind: recipe.as_ref().type_id(),
                        identity: Rc::new(()),
                        recipe,
                        revision: None,
                        visible: false,
                        state,
                        input,
                    },
                );
            }
        }
        let resources = self.resources.borrow();
        let mut reservations = Vec::new();
        for (mount, frame) in candidate.changed() {
            let id = mount.id().to_string();
            let owner = &resources.owners[&id];
            let capacity = owner.input.observations.prepare(
                frame
                    .view
                    .view
                    .publication_capacity(owner.state.as_ref(), cx),
            )?;
            reservations.push((id, capacity));
        }
        Ok(reservations)
    }

    /// Native creation/commands run after the whole declarative transaction,
    /// never during layout/paint. The store owns resources, not component state.
    fn prepare_native(
        &self,
        window: &mut gpui_kit::Window,
        cx: &mut Context<Self>,
    ) -> maka_ui_runtime::Result<()> {
        if self.resources.borrow().is_removed() {
            return Ok(());
        }
        let application = self.application.borrow();
        self.resources
            .borrow_mut()
            .deactivate_unmounted(&application, window, cx)?;
        for mount in application.mounted_subtree(&self.mount)? {
            self.resources.borrow_mut().prepare_scope(&mount);
            if application.is_retiring(&mount) {
                continue;
            }
            let frame = application.frame(&mount)?;
            let id = mount.id().to_string();
            let native = &frame.view;
            let lease = self.resources.borrow().acquire(&mount);
            let input = self.input(
                &lease,
                (application.frame(&self.mount)?.revision, frame.revision),
                window.window_handle().window_id(),
                cx,
            );
            let mut resources = self.resources.borrow_mut();
            let kind = native.view.as_ref().type_id();
            if resources
                .owners
                .get(&id)
                .is_some_and(|owner| owner.kind != kind)
            {
                return Err(maka_ui_runtime::Error::Invalid(
                    "native recipe type changed on original mount; remount the component".into(),
                ));
            }
            let budget = resources.budget.clone();
            let checkpoint = application.checkpoint_scope(&mount)?;
            let owner = resources.owners.entry(id).or_insert_with(|| {
                let input = widgets::LiveInput::new(
                    Rc::new(RefCell::new(Some(input.clone()))),
                    budget,
                    lease.resources(),
                    application.wake_callback(),
                )
                .with_checkpoint(checkpoint);
                resources::NativeInstance {
                    kind,
                    identity: Rc::new(()),
                    recipe: native.view.clone(),
                    revision: None,
                    visible: true,
                    state: native.view.create(input.clone(), window, cx),
                    input,
                }
            });
            *owner.input.current.borrow_mut() = Some(input);
            owner.recipe = native.view.clone();
        }
        Ok(())
    }
    fn deliver_action_results(
        &self,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) -> maka_ui_runtime::Result<usize> {
        self.application.borrow_mut().deliver_action_results(
            &self.mount,
            |target, recipe, witness| {
                self.resources
                    .borrow()
                    .validate_continuation(target, recipe, witness, window, cx)
            },
        )
    }
    fn commit_candidate(
        &self,
        window: &mut gpui_kit::Window,
        cx: &mut Context<Self>,
    ) -> maka_ui_runtime::Result<bool> {
        if self.resources.borrow().is_removed() {
            return Ok(false);
        }
        self.resources
            .borrow_mut()
            .bind_surface(window.window_handle(), cx.weak_entity())?;
        self.application.borrow_mut().advance_ui()?;
        {
            let application = self.application.borrow();
            let mut resources = self.resources.borrow_mut();
            // Completed original native work enters custody before the next
            // candidate can reserve capacity or execute a new native command.
            if application.frame(&self.mount).is_ok() {
                for mount in application.mounted_subtree(&self.mount)? {
                    if !application.is_retiring(&mount)
                        && let Some(owner) = resources.owners.get_mut(&mount.id().to_string())
                        && owner.input.current.borrow().is_some()
                    {
                        owner
                            .recipe
                            .publish_pending(owner.state.as_mut(), window, cx);
                    }
                }
            }
        }
        let mut reservations = Vec::new();
        let committed =
            self.application
                .borrow_mut()
                .commit_ui(&self.mount, |application, candidate| {
                    reservations = self.prepare_candidate(application, candidate, window, cx)?;
                    Ok(())
                })?;
        if committed {
            let resources = self.resources.borrow();
            for (mount, capacity) in reservations {
                if let Some(owner) = resources.owners.get(&mount) {
                    owner.input.observations.adopt(capacity);
                }
            }
        }
        Ok(committed)
    }
    fn sync_native(
        &self,
        window: &mut gpui_kit::Window,
        cx: &mut Context<Self>,
    ) -> maka_ui_runtime::Result<()> {
        self.commit_candidate(window, cx)?;
        if self.resources.borrow().is_removed() {
            return Ok(());
        }
        let advancement = if self.application.borrow().deployment_status().closing
            || self.application.borrow().is_retiring(&self.mount)
        {
            Ok(())
        } else {
            self.application.borrow_mut().advance_deployment()
        };
        self.resources
            .borrow_mut()
            .deactivate_unmounted(&self.application.borrow(), window, cx)?;
        if let Err(error) = advancement {
            eprintln!("deployment convergence failed: {error}");
        }
        self.prepare_native(window, cx)?;
        self.application
            .borrow_mut()
            .capture_actions(&self.mount, |target| {
                self.resources.borrow_mut().capture(target, window, cx)
            })?;
        let application = self.application.borrow();
        let mut visible_mounts = std::collections::BTreeSet::from([self.mount.id()]);
        for mount in application.mounted_subtree(&self.mount)? {
            if application.is_retiring(&mount) {
                continue;
            }
            let frame = application.frame(&mount)?;
            let native = &frame.view;
            let visible = visible_mounts.contains(&mount.id());
            if visible {
                for slot in native.visible_slots() {
                    visible_mounts.insert(application.child_mount(&mount, &slot.key)?.id());
                }
            }
            let mut resources = self.resources.borrow_mut();
            let owner = resources
                .owners
                .get_mut(&mount.id().to_string())
                .expect("prepared native owner");
            if owner.revision.is_none() || owner.visible != visible {
                owner.visible = visible;
                native
                    .view
                    .presentation(owner.state.as_mut(), visible, window, cx);
            }
            if owner.revision != Some(frame.revision) {
                // A consumed scene revision is not a successful native command:
                // widgets retain explicit applied/conflict receipts separately.
                native.view.commit(owner.state.as_mut(), window, cx);
                owner.revision = Some(frame.revision);
            }
            native
                .view
                .publish_pending(owner.state.as_mut(), window, cx);
        }
        let observations: Vec<_> = application
            .mounted_subtree(&self.mount)?
            .into_iter()
            .filter(|mount| !application.is_retiring(mount))
            .filter_map(|mount| {
                self.resources
                    .borrow()
                    .owners
                    .get(&mount.id().to_string())
                    .map(|owner| {
                        (
                            mount,
                            owner.input.clone(),
                            owner.input.current.borrow().clone(),
                        )
                    })
            })
            .collect();
        drop(application);
        for (mount, live, input) in observations {
            live.observations.retain(|key| {
                self.application
                    .borrow()
                    .frame(&mount)
                    .is_ok_and(|frame| frame.view.declares_input(key))
            });
            if let Some(input) = input {
                for error in live.advance(&input, window, cx) {
                    eprintln!("native observation candidate rejected: {error}");
                }
            }
        }
        self.resources
            .borrow_mut()
            .collect(&mut self.application.borrow_mut())?;
        let advancement = if self.application.borrow().deployment_status().closing
            || self.application.borrow().is_retiring(&self.mount)
        {
            Ok(())
        } else {
            self.application.borrow_mut().advance_deployment()
        };
        self.resources
            .borrow_mut()
            .deactivate_unmounted(&self.application.borrow(), window, cx)?;
        if let Err(error) = advancement {
            eprintln!("deployment convergence failed: {error}");
        }
        if self.application.borrow().is_retiring(&self.mount) {
            if let Some(owner) = cx.try_global::<actions::Owner>().cloned() {
                owner.recover_launch(&self.mount, window)?;
            }
            return Ok(());
        }
        if let Some(owner) = cx.try_global::<actions::Owner>().cloned()
            && owner.recover_launch(&self.mount, window)?
        {
            return Ok(());
        }
        actions::pump(
            self.application.clone(),
            self.mount.clone(),
            cx.weak_entity(),
            window,
            cx,
        )?;
        Ok(())
    }
    fn scene(
        &self,
        window: &gpui_kit::Window,
        cx: &Context<Self>,
    ) -> maka_ui_runtime::Result<Rc<RenderScene>> {
        let application = self.application.borrow();
        let resources = self.resources.borrow();
        let mut entries = std::collections::BTreeMap::<String, Rc<RenderScene>>::new();
        for mount in application.mounted_subtree(&self.mount)?.into_iter().rev() {
            let frame = application.frame(&mount)?.clone();
            let lease = resources.acquire(&mount);
            let input = self.input(
                &lease,
                (application.frame(&self.mount)?.revision, frame.revision),
                window.window_handle().window_id(),
                cx,
            );
            let mut children = std::collections::BTreeMap::new();
            let mut add = |slot: &maka_ui_runtime::ComponentSlot| -> maka_ui_runtime::Result<()> {
                let child = application.child_mount(&mount, &slot.key)?;
                let child = entries
                    .remove(&child.id().to_string())
                    .expect("captured child before parent");
                children.insert(slot.key.clone(), child);
                Ok(())
            };
            for slot in frame.view.slots() {
                add(slot)?;
            }
            let identity = resources
                .owners
                .get(&mount.id().to_string())
                .map(|o| o.identity.clone());
            entries.insert(
                mount.id().to_string(),
                Rc::new(RenderScene {
                    resources: self.resources.clone(),
                    window_id: window.window_handle().window_id(),
                    entry: RenderEntry {
                        frame,
                        lease,
                        input,
                        children,
                        identity,
                    },
                }),
            );
        }
        Ok(entries
            .remove(&self.mount.id().to_string())
            .expect("captured root"))
    }
}

/// Immutable observations of one committed tree. Callback custody retains exact
/// mounts; the kernel remains the only mutable state and child-binding authority.
struct RenderScene {
    window_id: gpui_kit::WindowId,
    resources: Rc<RefCell<resources::NativeResources>>,
    entry: RenderEntry,
}
struct RenderEntry {
    frame: maka_ui_runtime::Frame<NativeView<dyn widgets::Widget>>,
    lease: resources::Lease,
    input: widgets::Input,
    children: std::collections::BTreeMap<String, Rc<RenderScene>>,
    identity: Option<Rc<()>>,
}
impl RenderScene {
    fn render(
        self: &Rc<Self>,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) -> maka_ui_runtime::Result<AnyElement> {
        if window.window_handle().window_id() != self.window_id {
            return Err(maka_ui_runtime::Error::Invalid(
                "captured component belongs to another window".into(),
            ));
        }
        let entry = &self.entry;
        let id = entry.lease.mount().id().to_string();
        if !entry.lease.is_attached() {
            return Err(maka_ui_runtime::Error::Invalid(
                "captured mount detached".into(),
            ));
        }
        let resources = self.resources.borrow();
        let owner = resources
            .owners
            .get(&id)
            .filter(|owner| {
                entry
                    .identity
                    .as_ref()
                    .is_some_and(|token| Rc::ptr_eq(token, &owner.identity))
            })
            .ok_or_else(|| {
                maka_ui_runtime::Error::Invalid("captured native owner replaced".into())
            })?;
        if owner.revision != Some(entry.frame.revision) {
            return Err(maka_ui_runtime::Error::Invalid(
                "captured scene revision replaced".into(),
            ));
        }
        // An ancestor can hide this retained owner without changing its frame.
        // Old local Slots must obey the same current presentation admission.
        if !owner.visible {
            return Ok(gpui_kit::div().hidden().into_any_element());
        }
        let slots = self.slots();
        let content = entry
            .frame
            .view
            .view
            .render(owner.state.as_ref(), &slots, window, cx)?;
        Ok(owned::Owned {
            window_id: self.window_id,
            content,
            lease: entry.lease.clone(),
        }
        .into_any_element())
    }
    fn slots(self: &Rc<Self>) -> widgets::Slots {
        let entry = &self.entry;
        let keys = entry
            .frame
            .view
            .slots()
            .iter()
            .map(|s| s.key.clone())
            .collect();
        let scene = self.clone();
        widgets::Slots::new(keys, entry.input.clone(), move |key, window, cx| {
            if window.window_handle().window_id() != scene.window_id {
                return Err(maka_ui_runtime::Error::Invalid(
                    "captured slots belong to another window".into(),
                ));
            }
            let entry = &scene.entry;
            let parent = entry.lease.mount().id().to_string();
            if !entry.lease.is_attached() {
                return Err(maka_ui_runtime::Error::Invalid(
                    "slot parent detached".into(),
                ));
            }
            // Retained slots also require this original native owner and frame.
            if let Some(token) = &entry.identity
                && !scene
                    .resources
                    .borrow()
                    .owners
                    .get(&parent)
                    .is_some_and(|o| {
                        Rc::ptr_eq(token, &o.identity) && o.revision == Some(entry.frame.revision)
                    })
            {
                return Err(maka_ui_runtime::Error::Invalid(
                    "slot parent owner replaced".into(),
                ));
            }
            let child = entry
                .children
                .get(key)
                .ok_or_else(|| maka_ui_runtime::Error::Invalid("unknown component slot".into()))?;
            child.render(window, cx)
        })
    }
}

impl Render for Surface {
    fn render(
        &mut self,
        window: &mut gpui_kit::Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Render recipes and committed Rust state only; no JavaScript here.
        if self.resources.borrow().is_removed() {
            return div().size_full();
        }
        let content = self
            .scene(window, cx)
            .and_then(|scene| scene.render(window, cx));
        let content = match content {
            Ok(content) => content,
            Err(error) => {
                eprintln!("component rendering rejected: {error}");
                div().into_any_element()
            }
        };
        div().size_full().child(content)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    log::set_logger(&NativeLog)?;
    log::set_max_level(log::LevelFilter::Warn);
    Launcher::start(Args::parse(), widgets::registry())?.run()
}

fn compiled_host() -> Result<maka_host_runtime::application::NativeRegistry, String> {
    let mut native = maka_host_runtime::application::NativeRegistry::default();
    maka_product_native::provide(&mut native)?;
    Ok(native)
}

/// Trusted launcher owns the actual caller store and compiled availability.
/// Composition alone selects the plugin instances and native activation.
pub(crate) struct Launcher {
    pub(crate) timer: tokio::runtime::Runtime,
    pub(crate) store: Arc<maka_event_log::EventLog>,
    connections: maka_transport::Connections,
    application: DesktopApplication,
    mount: Mount,
}
impl Launcher {
    pub(crate) fn start(
        args: Args,
        components: maka_ui_runtime::NativeRegistry<dyn widgets::Widget>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let timer = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()?;
        let namespaces = maka_event_log::root::RootNamespaces::for_current_account()?;
        let configuration = args
            .connections
            .as_deref()
            .map(maka_transport::Configuration::read)
            .transpose()?
            .unwrap_or_default();
        let cohost_native = compiled_host()?;
        let helper = std::env::current_exe()?.with_file_name(if cfg!(windows) {
            "maka.exe"
        } else {
            "maka"
        });
        let owner = Arc::new(maka_event_log::root::RootOwner::open(
            &args.root,
            &namespaces,
        )?);
        let store = Arc::new(timer.block_on(maka_transport::open_store(owner))?);
        let mut connections = match timer.block_on(maka_transport::Connections::prepare(
            store.clone(),
            args.deployment,
            PresentationTarget::Desktop,
            configuration,
            namespaces,
            helper,
            cohost_native,
            Some(maka_host_js::factory(Default::default())),
        )) {
            Ok(connections) => connections,
            Err(error) => {
                timer.block_on(store.drain());
                return Err(error.into());
            }
        };
        if let Err(error) = timer.block_on(connections.start()) {
            timer.block_on(async {
                connections.drain().await;
                store.drain().await;
            });
            return Err(error.into());
        }
        let loaded = connections.take_initial().expect("initial UI snapshot");
        let registrations = connections.registrations();
        let started = timer.block_on(Application::start_loaded(
            store.clone(),
            loaded,
            PresentationTarget::Desktop,
            timer.handle().clone(),
            maka_ui_js::execution(maka_js_limits()),
            components,
            registrations,
        ));
        let mut application = match started {
            Ok(application) => application,
            Err(error) => {
                timer.block_on(async {
                    connections.drain().await;
                    store.drain().await;
                });
                return Err(error.into());
            }
        };
        application.defer_native_commit();
        let mount =
            match timer.block_on(application.mount_bootstrap_prepared(serde_json::json!({}))) {
                Ok(mount) => mount,
                Err(error) => {
                    drain_ui(&mut application, &timer, true);
                    timer.block_on(async {
                        connections.drain().await;
                        store.drain().await;
                    });
                    return Err(error.into());
                }
            };
        Ok(Self {
            timer,
            store,
            connections,
            application,
            mount,
        })
    }
    pub(crate) fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let Self {
            timer,
            store,
            mut connections,
            mut application,
            mount,
        } = self;
        // Startup may inspect the prepared window declaration; native resource
        // publication still commits the frame after the actual window exists.
        let frame = match application
            .frame(&mount)
            .or_else(|_| application.prepared_frame(&mount))
        {
            Ok(frame) => frame,
            Err(error) => {
                drain_ui(&mut application, &timer, true);
                timer.block_on(async {
                    connections.drain().await;
                    store.drain().await;
                });
                return Err(error.into());
            }
        };
        if !matches!(&frame.surface, Some(SurfaceDeclaration::Window { .. })) {
            drain_ui(&mut application, &timer, true);
            timer.block_on(async {
                connections.drain().await;
                store.drain().await;
            });
            return Err("desktop launch component must request a window".into());
        }
        let application = Rc::new(RefCell::new(application));
        let shutdown_application = application.clone();
        let connections = Arc::new(tokio::sync::Mutex::new(connections));
        let platform_connections = connections.clone();
        let platform_store = store.clone();
        let execution_owner = Rc::new(RefCell::new(None));
        let platform_owner = execution_owner.clone();
        gpui_kit::application()
            .with_quit_mode(gpui_kit::QuitMode::Explicit)
            .run(move |cx| {
                gpui_kit::init(cx);
                let owner = actions::own(
                    application.clone(),
                    Some(platform_connections),
                    Some(platform_store),
                    cx,
                )
                .expect("unique UI application owner");
                *platform_owner.borrow_mut() = Some(owner.clone());
                if let Err(error) = owner.open_launch(mount, serde_json::json!({}), cx) {
                    eprintln!("native launch failed: {error}");
                }
            });
        // GPUI's ordinary exit already drained the UI. An unexpected platform exit
        // cannot substitute for mount/worker receipts: retain both kernels and runtime.
        // A lost loop cannot synthesize native close receipts. The independent
        // foreground owner can still consume actual workers and close successors.
        let mut reported = std::collections::BTreeSet::new();
        loop {
            if let Some(owner) = execution_owner.borrow().as_ref()
                && let Err(error) = owner.pump()
            {
                let error = error.to_string();
                if reported.insert(error.clone()) {
                    eprintln!("UI completion unconfirmed: {error:?}");
                }
            }
            match shutdown_application.borrow_mut().shutdown() {
                Ok(()) => break,
                Err(error) => {
                    let error = error.to_string();
                    if reported.insert(error.clone()) {
                        eprintln!("UI cleanup unconfirmed: {error:?}");
                    }
                }
            }
            timer
                .block_on(async { tokio::time::sleep(std::time::Duration::from_millis(50)).await });
        }
        let updates = execution_owner
            .borrow()
            .as_ref()
            .and_then(actions::Owner::take_updates);
        let exit_mode = execution_owner
            .borrow()
            .as_ref()
            .map(actions::Owner::exit_mode)
            .unwrap_or(maka_transport::ExitMode::KeepHost);
        timer.block_on(async {
            if let Some(updates) = updates {
                updates.drain().await;
            }
            connections.lock().await.drain_with(exit_mode).await;
            store.drain().await;
        });
        if let Some(error) = execution_owner
            .borrow()
            .as_ref()
            .and_then(actions::Owner::failure)
        {
            return Err(error.into());
        }
        Ok(())
    }
}

fn drain_ui(
    application: &mut DesktopApplication,
    timer: &tokio::runtime::Runtime,
    never_presented: bool,
) {
    let mut reported = std::collections::BTreeSet::new();
    loop {
        match application.shutdown() {
            Ok(()) => return,
            Err(error) => {
                // Before opening a native window no renderer resources exist.
                // No such acknowledgment is legal after presentation starts.
                if never_presented {
                    for mount in application.retiring_mounts() {
                        let _ = application.close(&mount, Ok(()));
                    }
                }
                let error = error.to_string();
                if reported.insert(error.clone()) {
                    eprintln!("UI cleanup pending: {error:?}");
                }
            }
        }
        timer.block_on(async { tokio::time::sleep(std::time::Duration::from_millis(50)).await });
    }
}

/// GPUI reports platform failures through `log`; without a subscriber even
/// display-link startup failures are silent. Diagnostics stay on stderr.
struct NativeLog;
impl log::Log for NativeLog {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Warn
    }
    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!("{} {}: {}", record.level(), record.target(), record.args());
        }
    }
    fn flush(&self) {}
}

fn maka_js_limits() -> maka_js_runtime::plugin::Limits {
    maka_js_runtime::plugin::Limits {
        synchronous_slice: std::time::Duration::from_millis(100),
        ..Default::default()
    }
}

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../../../platform/transport/tests/support/mod.rs"]
mod bridge_support;
#[cfg(test)]
use bridge_support::counter as support;

#[cfg(test)]
mod tests {
    use super::*;
    fn timer() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_time()
            .build()
            .unwrap()
    }
    fn application(timer: &tokio::runtime::Runtime) -> DesktopApplication {
        let (input, packages) = support::native_fixture();
        Application::new(
            &input,
            packages,
            timer.handle().clone(),
            maka_ui_js::execution(Default::default()),
            widgets::registry(),
            Default::default(),
        )
        .unwrap()
    }

    fn settle_logic(app: &Rc<RefCell<DesktopApplication>>) {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while app.borrow().ui_pending() {
            app.borrow_mut().advance_ui().unwrap();
            assert!(
                std::time::Instant::now() < until,
                "original UI worker did not finish"
            );
            std::thread::park_timeout(std::time::Duration::from_millis(1));
        }
    }
    #[test]
    fn never_presented_child_can_close_but_visible_retired_mounts_stay_fenced() {
        let timer = timer();
        let mut app = application(&timer);
        let root = timer
            .block_on(app.mount_bootstrap_ready(serde_json::json!({})))
            .unwrap();
        let right = app.child_mount(&root, "right").unwrap().clone();
        let other_root = timer
            .block_on(app.mount_bootstrap_ready(serde_json::json!({})))
            .unwrap();
        let other_child = app.child_mount(&other_root, "right").unwrap().clone();
        app.retire(&other_root).unwrap();
        let (wake, _) = tokio::sync::mpsc::channel(1);
        let mut resources = resources::NativeResources::new(root.clone(), wake);
        app.dispatch(&root, 0, "toggle", serde_json::Value::Null)
            .unwrap();
        timer.block_on(app.wait_idle()).unwrap();
        assert_eq!(resources.collect(&mut app).unwrap(), 2);
        assert!(app.frame(&right).is_err());
        assert!(app.frame(&other_root).is_ok());
        assert!(app.frame(&other_child).is_ok());
        app.retire(&root).unwrap();
        assert_eq!(resources.collect(&mut app).unwrap(), 0);
        assert_eq!(app.mounted_subtree(&root).unwrap().len(), 2);
        assert!(app.frame(&root).is_ok());
    }

    #[gpui::test]
    fn unavailable_platform_receipt_does_not_create_resources_during_initial_window_draw(
        cx: &mut TestAppContext,
    ) {
        cx.executor().allow_parking();
        let timer = timer();
        let dir = tempfile::tempdir().unwrap();
        let ns = maka_event_log::root::RootNamespaces {
            ownership: dir.path().join("owners"),
            control: dir.path().join("control"),
        };
        let root_path = dir.path().join("root");
        let owner = Arc::new(maka_event_log::root::RootOwner::create(&root_path, &ns).unwrap());
        let store = Arc::new(
            timer
                .block_on(maka_event_log::EventLog::for_root(owner))
                .unwrap(),
        );
        let deployment = uuid::Uuid::new_v4();
        let (input, packages) = support::native_fixture();
        let source = timer
            .block_on(store.commit_application(deployment, 0, input, vec![], packages))
            .unwrap();
        let mut application = timer
            .block_on(Application::start_from_store(
                store,
                source,
                PresentationTarget::Desktop,
                timer.handle().clone(),
                maka_ui_js::execution(Default::default()),
                widgets::registry(),
                vec![],
            ))
            .unwrap();
        let root = timer
            .block_on(application.mount_bootstrap_ready(serde_json::json!({})))
            .unwrap();
        let app = Rc::new(RefCell::new(application));
        let (wake, wakes) = tokio::sync::mpsc::channel(1);
        let resources = Rc::new(RefCell::new(resources::NativeResources::new(
            root.clone(),
            wake,
        )));
        cx.update(|cx| {
            gpui_kit::init(cx);
            actions::own(app.clone(), None, None, cx).unwrap();
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                // A real failed claim exercises the startup path. GPUI still
                // draws the returned content once, even after remove_window.
                let _claimed = window.take_cleanup_receiver().unwrap();
                let error = window_owner::start(
                    window,
                    cx,
                    resources.clone(),
                    app.clone(),
                    wakes,
                    |_, _| panic!("no closing owner was created"),
                )
                .expect_err("receiver can only be claimed once");
                resources
                    .borrow_mut()
                    .window_removed(&mut app.borrow_mut())
                    .unwrap();
                app.borrow_mut().close(&root, Err(error)).unwrap_err();
                window.remove_window();
                let surface = cx.new(|_| Surface {
                    application: app.clone(),
                    resources: resources.clone(),
                    mount: root.clone(),
                });
                surface.update(cx, |s, cx| s.sync_native(window, cx).unwrap());
                surface
            })
            .unwrap();
        });
        cx.run_until_parked();
        assert!(resources.borrow().is_removed());
        assert!(resources.borrow().owners.is_empty());
        assert!(app.borrow().is_retiring(&root));
        assert!(app.borrow().frame(&root).is_ok());
        assert!(app.borrow_mut().shutdown().is_err());
        let weak = Rc::downgrade(&app);
        drop(app);
        drop(resources);
        cx.run_until_parked();
        assert!(
            weak.upgrade().is_some(),
            "failed startup keeps the independent Global owner"
        );
        assert_eq!(
            maka_event_log::root::RootOwner::open(&root_path, &ns)
                .err()
                .unwrap()
                .kind(),
            std::io::ErrorKind::WouldBlock
        );
    }

    #[gpui::test]
    fn closing_one_window_does_not_retire_or_collect_another_surface(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        let timer = timer();
        let mut application = application(&timer);
        let first = timer
            .block_on(application.mount_bootstrap_ready(serde_json::json!({})))
            .unwrap();
        let second = timer
            .block_on(application.mount_bootstrap_ready(serde_json::json!({})))
            .unwrap();
        let app = Rc::new(RefCell::new(application));
        let finished = Rc::new(std::cell::Cell::new(0));
        let open = |mount: &Mount| {
            let (wake, wakes) = tokio::sync::mpsc::channel(1);
            let resources = Rc::new(RefCell::new(resources::NativeResources::new(
                mount.clone(),
                wake,
            )));
            let (handle, surface) = cx.update(|cx| {
                gpui_kit::init(cx);
                gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                    let finished = finished.clone();
                    window_owner::start(
                        window,
                        cx,
                        resources.clone(),
                        app.clone(),
                        wakes,
                        move |outcome, _| {
                            outcome.unwrap();
                            finished.set(finished.get() + 1);
                        },
                    )
                    .unwrap()
                    .detach();
                    let surface = cx.new(|_| Surface {
                        application: app.clone(),
                        resources: resources.clone(),
                        mount: mount.clone(),
                    });
                    surface.update(cx, |s, cx| s.sync_native(window, cx).unwrap());
                    surface
                })
                .unwrap()
            });
            (handle, surface, resources)
        };
        let (first_window, first_surface, first_resources) = open(&first);
        let (second_window, second_surface, second_resources) = open(&second);
        let held = first_resources.borrow_mut().acquire(&first);
        cx.update_window(first_window, |_, window, _| window.remove_window())
            .unwrap();
        cx.update(|_| drop(first_surface));
        cx.run_until_parked();
        assert!(app.borrow().is_retiring(&first));
        assert!(!app.borrow().is_retiring(&second));
        assert_eq!(finished.get(), 0);
        app.borrow_mut()
            .dispatch(&second, 0, "parent", serde_json::Value::Null)
            .unwrap();
        settle_logic(&app);
        cx.update_window(second_window, |_, window, cx| {
            second_surface.update(cx, |s, cx| s.sync_native(window, cx).unwrap());
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        drop(held);
        cx.run_until_parked();
        assert_eq!(finished.get(), 1);
        assert!(first_resources.borrow().surface_closed);
        assert!(!second_resources.borrow().surface_closed);
        assert!(app.borrow().frame(&first).is_err());
        assert_eq!(app.borrow().frame(&second).unwrap().revision, 1);
        cx.update_window(second_window, |_, window, _| window.remove_window())
            .unwrap();
        cx.update(|_| drop(second_surface));
        cx.run_until_parked();
        assert_eq!(finished.get(), 2);
        assert!(app.borrow().frame(&second).is_err());
    }

    #[gpui::test]
    fn window_closing_owner_survives_surface_drop_and_waits_for_real_leases(
        cx: &mut TestAppContext,
    ) {
        cx.executor().allow_parking();
        let timer = timer();
        let mut application = application(&timer);
        let root = timer
            .block_on(application.mount_bootstrap_ready(serde_json::json!({})))
            .unwrap();
        let child = application.child_mount(&root, "right").unwrap().clone();
        let application = Rc::new(RefCell::new(application));
        let (wake, wakes) = tokio::sync::mpsc::channel(1);
        let resources = Rc::new(RefCell::new(resources::NativeResources::new(
            root.clone(),
            wake,
        )));
        resources.borrow_mut().prepare_scope(&root);
        resources.borrow_mut().prepare_scope(&child);
        let held_root = resources.borrow().acquire(&root);
        let held_child = resources.borrow_mut().acquire(&child);
        let finished = Rc::new(RefCell::new(None));
        let (handle, surface) = cx.update(|cx| {
            gpui_kit::init(cx);
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                let finished = finished.clone();
                window_owner::start(
                    window,
                    cx,
                    resources.clone(),
                    application.clone(),
                    wakes,
                    move |outcome, _| *finished.borrow_mut() = Some(outcome),
                )
                .unwrap()
                .detach();
                let surface = cx.new(|_| Surface {
                    application: application.clone(),
                    resources: resources.clone(),
                    mount: root.clone(),
                });
                surface.update(cx, |s, cx| s.sync_native(window, cx).unwrap());
                surface
            })
            .unwrap()
        });
        cx.update_window(handle, |_, window, _| window.remove_window())
            .unwrap();
        cx.update(|_| drop(surface));
        cx.run_until_parked();
        assert!(resources.borrow().owners.is_empty());
        assert!(application.borrow().is_retiring(&root));
        assert!(application.borrow().frame(&root).is_ok());
        assert!(
            finished.borrow().is_none(),
            "platform receipt alone cannot close the surface"
        );
        drop(held_root);
        cx.run_until_parked();
        assert!(
            application.borrow().frame(&root).is_ok(),
            "root waits for its child lease"
        );
        let weak = Rc::downgrade(&application);
        drop(held_child);
        cx.run_until_parked();
        assert_eq!(*finished.borrow(), Some(Ok(())));
        assert!(resources.borrow().surface_closed);
        assert!(application.borrow().retiring_mounts().is_empty());
        assert!(application.borrow().frame(&root).is_err());
        drop(application);
        assert!(
            weak.upgrade().is_none(),
            "completed task and close subscription must not self-retain"
        );
    }

    #[gpui::test]
    fn native_state_and_callback_release_drive_cleanup_without_mount_churn_leaks(
        cx: &mut TestAppContext,
    ) {
        cx.executor().allow_parking();
        let timer = timer();
        let mut app = application(&timer);
        let root = timer
            .block_on(app.mount_bootstrap_ready(serde_json::json!({})))
            .unwrap();
        let right = app.child_mount(&root, "right").unwrap().clone();
        let native = app.child_mount(&root, "nativeLabel").unwrap().clone();
        assert!(app.frame(&native).unwrap().view.slots().is_empty());
        let app = Rc::new(RefCell::new(app));
        let (wake, receipts) = tokio::sync::mpsc::channel(1);
        let resources = Rc::new(RefCell::new(resources::NativeResources::new(
            root.clone(),
            wake,
        )));
        let (handle, surface) = cx.update(|cx| {
            gpui_kit::init(cx);
            let application = app.clone();
            let resources = resources.clone();
            let mount = root.clone();
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(0.), px(0.)),
                        size(px(640.), px(400.)),
                    ))),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    window_owner::start(
                        window,
                        cx,
                        resources.clone(),
                        application.clone(),
                        receipts,
                        |outcome, _| outcome.unwrap(),
                    )
                    .unwrap()
                    .detach();
                    let surface = cx.new(|_| Surface {
                        application,
                        resources,
                        mount,
                    });
                    surface.update(cx, |surface, cx| surface.sync_native(window, cx).unwrap());
                    surface
                },
            )
            .unwrap()
        });
        let draw = |cx: &mut TestAppContext| {
            cx.update_window(handle, |_, window, cx| {
                surface.update(cx, |surface, cx| surface.sync_native(window, cx).unwrap());
                window.refresh();
                window.draw(cx).clear(cx);
            })
            .unwrap();
        };
        draw(cx);
        // First prove the actual GPUI state/callbacks alone retain the child.
        // No synthetic held resource can conceal an empty/skipped render here.
        app.borrow_mut()
            .dispatch(&root, 0, "toggle", serde_json::Value::Null)
            .unwrap();
        settle_logic(&app);
        assert_eq!(
            resources
                .borrow_mut()
                .collect(&mut app.borrow_mut())
                .unwrap(),
            0
        );
        assert!(app.borrow().is_retiring(&right));
        assert!(app.borrow().is_retiring(&native));
        assert!(app.borrow().frame(&native).is_ok());
        draw(cx);
        cx.run_until_parked();
        assert!(app.borrow().frame(&right).is_err());
        assert!(app.borrow().frame(&native).is_err());
        app.borrow_mut()
            .dispatch(&root, 1, "toggle", serde_json::Value::Null)
            .unwrap();
        settle_logic(&app);
        draw(cx);
        let right = app.borrow().child_mount(&root, "right").unwrap().clone();
        let native = app
            .borrow()
            .child_mount(&root, "nativeLabel")
            .unwrap()
            .clone();
        let held_resource = resources.borrow_mut().acquire(&right);
        let delayed_callback = held_resource.clone();
        app.borrow_mut()
            .dispatch(&root, 2, "toggle", serde_json::Value::Null)
            .unwrap();
        settle_logic(&app);
        assert_eq!(
            resources
                .borrow_mut()
                .collect(&mut app.borrow_mut())
                .unwrap(),
            0
        );
        assert!(app.borrow().is_retiring(&right));
        draw(cx);
        drop(held_resource);
        // The native label has no externally held lease and closes independently.
        assert_eq!(
            resources
                .borrow_mut()
                .collect(&mut app.borrow_mut())
                .unwrap(),
            1
        );
        assert!(app.borrow().frame(&native).is_err());
        assert!(app.borrow().frame(&right).is_ok());
        drop(delayed_callback);
        cx.run_until_parked();
        assert!(app.borrow().frame(&right).is_err());

        // Exceed the old 512-mount retention ceiling using real GPUI drawing.
        // Each detached child must release its native state and input closures.
        for revision in 3..=1102 {
            app.borrow_mut()
                .dispatch(&root, revision, "toggle", serde_json::Value::Null)
                .unwrap();
            settle_logic(&app);
            resources
                .borrow_mut()
                .collect(&mut app.borrow_mut())
                .unwrap();
            draw(cx);
            cx.run_until_parked();
            assert!(app.borrow().retiring_mounts().is_empty());
        }
        app.borrow_mut().retire(&root).unwrap();
        assert_eq!(
            resources
                .borrow_mut()
                .collect(&mut app.borrow_mut())
                .unwrap(),
            0
        );
        assert!(app.borrow().frame(&root).is_ok());
        let released = Rc::downgrade(&app);
        cx.update_window(handle, |_, window, _| window.remove_window())
            .unwrap();
        cx.update(|_| drop(surface));
        drop(app);
        cx.run_until_parked();
        assert!(
            released.upgrade().is_none(),
            "receipt task must not keep the application alive"
        );
    }
}

#[cfg(test)]
mod editor_tests;

#[cfg(test)]
#[test]
fn javascript_scene_style_overflow_rejects_batch_and_preserves_original_frame() {
    let timer = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_time()
        .build()
        .unwrap();
    for value in ["1e300", "-1e300"] {
        let (input, packages) = support::fixture(&format!(
            "const next=frame({{count:state.count+1,revision:state.revision+1}});next.frame.view.style={{minWidth:{value}}};return next;"
        ));
        let mut app = Application::new(
            &input,
            packages,
            timer.handle().clone(),
            maka_ui_js::execution(Default::default()),
            widgets::registry(),
            Default::default(),
        )
        .unwrap();
        let mount = timer
            .block_on(app.mount_bootstrap_ready(serde_json::json!({"count":4})))
            .unwrap();
        app.dispatch(&mount, 0, "inc", serde_json::Value::Null)
            .unwrap();
        assert!(timer.block_on(app.wait_idle()).is_err());
        assert_eq!(app.frame(&mount).unwrap().revision, 0);
        assert_eq!(app.frame(&mount).unwrap().state["count"], 4);
        assert!(!app.is_retiring(&mount));
        app.close(&mount, Ok(())).unwrap();
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match app.shutdown() {
                Ok(()) => break,
                Err(maka_ui_runtime::Error::Core(maka_plugin_core::Error::CleanupPending)) => {}
                Err(error) => panic!("original cleanup failed: {error}"),
            }
            assert!(std::time::Instant::now() < until);
            std::thread::park_timeout(std::time::Duration::from_millis(1));
        }
    }
}

#[cfg(test)]
mod canvas_tests;
