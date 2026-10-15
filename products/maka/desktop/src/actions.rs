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

//! Accepted work survives its originating Surface and window.
mod launch;

use super::{DesktopApplication, Surface};
use futures_util::{
    FutureExt,
    future::{LocalBoxFuture, Shared},
};
use gpui_kit::{App, Context, WeakEntity, Window};
use maka_ui_runtime::actions::{ActionTask, TaskCompletion};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    sync::Arc,
    task::{Wake, Waker},
};
gpui_kit::actions!(maka, [Quit, CloseUiAndKeepHost]);

type Connections = Arc<tokio::sync::Mutex<maka_transport::Connections>>;
type Observer = Shared<LocalBoxFuture<'static, ()>>;
struct Job {
    observer: Observer,
    completion: Rc<RefCell<Option<TaskCompletion>>>,
}
struct State {
    application: Rc<RefCell<DesktopApplication>>,
    jobs: RefCell<BTreeMap<uuid::Uuid, Job>>,
    connections: Option<Connections>,
    caller: Option<Arc<maka_event_log::EventLog>>,
    draining: Cell<bool>,
    quitting: Cell<bool>,
    exit_mode: Cell<maka_transport::ExitMode>,
    launch: RefCell<Option<launch::State>>,
    failure: RefCell<Option<String>>,
    native: RefCell<BTreeMap<uuid::Uuid, Rc<RefCell<super::resources::NativeResources>>>>,
    updates: RefCell<Option<maka_transport::Updates>>,
    update_errors: RefCell<BTreeSet<String>>,
}
/// One actual foreground execution owner, shared by GPUI and its launcher.
/// Join observers survive a lost platform loop; no completion is fabricated.
#[derive(Clone)]
pub(super) struct Owner(Rc<State>);
impl gpui_kit::Global for Owner {}
struct UiWake(tokio::sync::mpsc::Sender<()>);
impl Wake for UiWake {
    fn wake(self: Arc<Self>) {
        let _ = self.0.try_send(());
    }
    fn wake_by_ref(self: &Arc<Self>) {
        let _ = self.0.try_send(());
    }
}
impl Owner {
    pub(super) fn exit_mode(&self) -> maka_transport::ExitMode {
        self.0.exit_mode.get()
    }
    pub(super) fn retain_native(&self, resources: Rc<RefCell<super::resources::NativeResources>>) {
        let root = resources.borrow().surface().id();
        let mut native = self.0.native.borrow_mut();
        if let Some(original) = native.get(&root) {
            assert!(Rc::ptr_eq(original, &resources));
        }
        native.insert(root, resources);
    }
    pub(super) fn release_native(
        &self,
        resources: &Rc<RefCell<super::resources::NativeResources>>,
    ) {
        assert!(resources.borrow().surface_closed);
        let root = resources.borrow().surface().id();
        let original = self
            .0
            .native
            .borrow_mut()
            .remove(&root)
            .expect("original native surface custody");
        assert!(Rc::ptr_eq(&original, resources));
    }
    pub(super) fn take_updates(&self) -> Option<maka_transport::Updates> {
        let mut updates = self.0.updates.borrow_mut().take();
        if let Some(updates) = updates.as_mut() {
            updates.stop();
        }
        updates
    }
    fn poll_updates(&self, cx: &mut App) {
        let result = self
            .0
            .updates
            .borrow_mut()
            .as_mut()
            .map(|updates| updates.poll(&self.0.application.borrow()));
        match result {
            Some(Ok(Some(update))) => {
                if let Err(error) = self.adopt_prepared(update.prepared, update.registrations, cx) {
                    eprintln!("deployment update rejected: {error}");
                }
            }
            Some(Err(error)) => {
                let mut errors = self.0.update_errors.borrow_mut();
                if errors.len() < 64 && errors.insert(error.clone()) {
                    eprintln!("deployment update rejected: {error}");
                }
            }
            _ => {}
        }
        quit_when_drained(&self.0.application, cx);
    }
    pub(super) fn start(&self, action: ActionTask) -> Observer {
        let worker = self.0.application.borrow().scheduler().spawn(action.future);
        let completion = Rc::new(RefCell::new(None));
        let result = completion.clone();
        let observer = async move {
            *result.borrow_mut() = Some(
                worker
                    .await
                    .unwrap_or_else(|error| TaskCompletion::Interrupted(error.to_string())),
            );
        }
        .boxed_local()
        .shared();
        self.0.jobs.borrow_mut().insert(
            action.id,
            Job {
                observer: observer.clone(),
                completion,
            },
        );
        observer
    }
    fn complete(&self, id: uuid::Uuid) -> maka_ui_runtime::Result<Option<Observer>> {
        let job = self
            .0
            .jobs
            .borrow_mut()
            .remove(&id)
            .expect("owned action observer");
        let completion = job
            .completion
            .borrow_mut()
            .take()
            .expect("completed worker");
        let next = self
            .0
            .application
            .borrow_mut()
            .finish_task(id, completion)?;
        Ok(next.map(|next| self.start(next)))
    }
    /// Called on the same foreground thread after an unexpected platform return.
    /// It consumes actual receipts and launches the kernel's real close successors.
    pub(super) fn pump(&self) -> maka_ui_runtime::Result<()> {
        let pending = self
            .0
            .jobs
            .borrow()
            .iter()
            .map(|(id, job)| (*id, job.observer.clone()))
            .collect::<Vec<_>>();
        for (id, observer) in pending {
            if observer.now_or_never().is_some() {
                self.complete(id)?;
            }
        }
        Ok(())
    }
}
pub(super) fn own(
    application: Rc<RefCell<DesktopApplication>>,
    connections: Option<Connections>,
    caller: Option<Arc<maka_event_log::EventLog>>,
    cx: &mut App,
) -> maka_ui_runtime::Result<Owner> {
    if let Some(owner) = cx.try_global::<Owner>() {
        if !Rc::ptr_eq(&owner.0.application, &application) {
            return Err(maka_ui_runtime::Error::Invalid(
                "prior application cleanup not completed".into(),
            ));
        }
        if connections.as_ref().is_some_and(|provided| {
            owner
                .0
                .connections
                .as_ref()
                .is_none_or(|original| !Arc::ptr_eq(original, provided))
        }) || caller.as_ref().is_some_and(|provided| {
            owner
                .0
                .caller
                .as_ref()
                .is_none_or(|original| !Arc::ptr_eq(original, provided))
        }) {
            return Err(maka_ui_runtime::Error::Invalid(
                "foreground backend custody must be supplied at original creation".into(),
            ));
        }
        return Ok(owner.clone());
    }
    let mut updates = match (
        &connections,
        &caller,
        application.borrow().deployment_status().desired,
    ) {
        (Some(connections), Some(store), Some(source)) => Some(
            maka_transport::Updates::new(
                store.clone(),
                source,
                connections.clone(),
                application.borrow().scheduler().clone(),
            )
            .map_err(maka_ui_runtime::Error::Invalid)?,
        ),
        _ => None,
    };
    let (wake, mut wakes) = tokio::sync::mpsc::channel(1);
    let waker = Waker::from(Arc::new(UiWake(wake)));
    application.borrow_mut().set_waker(waker.clone());
    if let Some(updates) = updates.as_mut() {
        updates.set_waker(waker);
    }
    let owner = Owner(Rc::new(State {
        application,
        jobs: RefCell::new(BTreeMap::new()),
        connections,
        caller,
        draining: Cell::new(false),
        quitting: Cell::new(false),
        exit_mode: Cell::new(maka_transport::ExitMode::Quit),
        launch: RefCell::new(None),
        failure: RefCell::new(None),
        native: RefCell::new(BTreeMap::new()),
        updates: RefCell::new(updates),
        update_errors: RefCell::new(BTreeSet::new()),
    }));
    cx.set_global(owner.clone());
    let watched = Rc::downgrade(&owner.0);
    let executor = cx.background_executor().clone();
    cx.spawn(async move |cx| {
        loop {
            let continuing = cx.update(|cx| {
                let state = watched.upgrade()?;
                if !cx.has_global::<Owner>() || state.draining.get() {
                    return None;
                }
                let watched = Owner(state);
                if let Err(error) = watched.advance_ui() {
                    eprintln!("UI candidate rejected: {error}");
                }
                watched.poll_updates(cx);
                let live: Vec<_> = watched
                    .0
                    .native
                    .borrow()
                    .values()
                    .filter_map(|resources| resources.borrow().surface_binding())
                    .collect();
                for (window, surface) in live {
                    let _ = window.update(cx, |_, window, cx| {
                        let _ = surface.update(cx, |surface, cx| {
                            if let Err(error) = surface.sync_native(window, cx) {
                                eprintln!("native UI wake failed: {error}");
                            }
                            cx.notify();
                        });
                    });
                }
                quit_when_drained(&watched.0.application, cx);
                Some(
                    watched
                        .0
                        .updates
                        .borrow()
                        .as_ref()
                        .and_then(maka_transport::Updates::next_deadline),
                )
            });
            let Some(at) = continuing else { break };
            let timer = match at {
                Some(at) => executor
                    .timer(at.saturating_duration_since(std::time::Instant::now()))
                    .boxed_local(),
                None => std::future::pending::<()>().boxed_local(),
            };
            tokio::select! {
                wake = wakes.recv() => { if wake.is_none() { break; } },
                _ = timer => {},
            }
            // One foreground turn per poll. Native publication may wake this
            // owner immediately; its producer ACK and physical input must run
            // before another turn instead of being starved by a ready channel.
            tokio::task::yield_now().await;
        }
    })
    .detach();
    // Installed once per App. The callback captures no application/Root lease.
    if !cx.has_global::<QuitGate>() {
        cx.on_action(|_: &Quit, cx| {
            // Menu/key actions run inside the active Window's update. Release
            // that borrow before requesting removal of every native window.
            cx.defer(|cx| {
                request_quit(cx);
            });
        });
        cx.on_action(|_: &CloseUiAndKeepHost, cx| {
            if let Some(owner) = cx.try_global::<Owner>() {
                if owner.0.quitting.get() || owner.0.draining.get() {
                    return;
                }
                owner.0.exit_mode.set(maka_transport::ExitMode::KeepHost);
            }
            cx.defer(|cx| {
                request_quit(cx);
            });
        });
        cx.set_menus([gpui_kit::Menu::new("Maka").items([
            gpui_kit::MenuItem::action("Close UI and Keep Host", CloseUiAndKeepHost),
            gpui_kit::MenuItem::action("Quit Maka", Quit),
        ])]);
        cx.bind_keys([gpui_kit::KeyBinding::new(
            if cfg!(target_os = "macos") {
                "cmd-q"
            } else {
                "ctrl-q"
            },
            Quit,
            None,
        )]);
        cx.on_request_quit(request_quit)
            .map_err(|e| maka_ui_runtime::Error::Invalid(e.to_string()))?;
        cx.set_global(QuitGate);
    }
    Ok(owner)
}
struct QuitGate;
impl gpui_kit::Global for QuitGate {}
pub(super) fn request_quit(cx: &mut App) -> bool {
    let Some(owner) = cx.try_global::<Owner>().cloned() else {
        return true;
    };
    // Prohibit replacement before removal can deliver a reentrant receipt.
    owner.0.quitting.set(true);
    if let Some(updates) = owner.0.updates.borrow_mut().as_mut() {
        updates.stop();
    }
    // Stop all new kernel admission; pending physical mounts/work stay fenced.
    let _ = owner.0.application.borrow_mut().shutdown();
    // Logical removal only: actual window owners still report physical cleanup.
    for window in cx.windows() {
        let _ = window.update(cx, |_, window, _| window.remove_window());
    }
    quit_when_drained(&owner.0.application, cx);
    !cx.has_global::<Owner>()
}

pub(super) fn pump(
    application: Rc<RefCell<DesktopApplication>>,
    root: maka_plugin_core::runtime::Mount,
    surface: WeakEntity<Surface>,
    window: &mut Window,
    cx: &mut Context<Surface>,
) -> maka_ui_runtime::Result<()> {
    loop {
        let Some(action) = ({ application.borrow_mut().start_task(&root)? }) else {
            break;
        };
        execute(
            application.clone(),
            surface.clone(),
            root.clone(),
            window.window_handle(),
            action,
            cx,
        );
    }
    // Feedback is a new bounded transaction after the originating event and
    // entity borrows finish. Native input cannot address these internal keys.
    let app = application.clone();
    window.defer(cx, move |window, cx| {
        let result =
            match surface.update(cx, |surface, cx| surface.deliver_action_results(window, cx)) {
                Ok(result) => result,
                Err(_) => app
                    .borrow_mut()
                    .deliver_action_results(&root, |_, _, _| Ok(false)),
            };
        match result {
            Ok(0) => {}
            result => {
                if let Err(error) = result {
                    eprintln!("action result rejected: {error}");
                }
                let _ = surface.update(cx, |surface, cx| {
                    if let Err(error) = surface.sync_native(window, cx) {
                        eprintln!("native action feedback failed: {error}");
                    }
                    cx.notify();
                });
            }
        }
    });
    Ok(())
}
fn execute(
    application: Rc<RefCell<DesktopApplication>>,
    surface: WeakEntity<Surface>,
    root: maka_plugin_core::runtime::Mount,
    window: gpui_kit::AnyWindowHandle,
    action: ActionTask,
    cx: &mut App,
) {
    let owner = own(application.clone(), None, None, cx).expect("unique foreground owner");
    let id = action.id;
    let mut observer = owner.start(action);
    cx.spawn(async move |cx| {
        loop {
            observer.await;
            let next = cx.update(|cx| {
                let next = match owner.complete(id) {
                    Ok(next) => next,
                    Err(error) => {
                        eprintln!("service cleanup unconfirmed: {error}");
                        None
                    }
                };
                let delivered = window.update(cx, |_, window, cx| {
                    let _ = surface.update(cx, |surface, cx| {
                        if let Err(error) = surface.deliver_action_results(window, cx) {
                            eprintln!("action result rejected: {error}");
                        }
                        if let Err(error) = surface.sync_native(window, cx) {
                            eprintln!("native action completion failed: {error}");
                        }
                        cx.notify();
                    });
                });
                if delivered.is_err()
                    && let Err(error) = application
                        .borrow_mut()
                        .deliver_action_results(&root, |_, _, _| Ok(false))
                {
                    eprintln!("action result rejected: {error}");
                }
                quit_when_drained(&application, cx);
                next
            });
            let Some(next) = next else {
                break;
            };
            observer = next;
        }
    })
    .detach();
}

/// Last-window removal alone is not platform cleanup. Both all real mount
/// receipts and independent accepted Work must finish before automatic exit.
pub(super) fn quit_when_drained(application: &Rc<RefCell<DesktopApplication>>, cx: &mut App) {
    let Some(owner) = cx.try_global::<Owner>().cloned() else {
        return;
    };
    // Handoff is independent of unrelated windows and accepted work.
    if owner.replacing() && !owner.0.quitting.get() {
        if let Err(error) = owner.resume_launch(cx) {
            eprintln!("launch convergence failed: {error}");
        }
        return;
    }
    if !cx.windows().is_empty() {
        return;
    }
    // Logical last-window removal stops candidate admission immediately, even
    // while original native entities or accepted work still hold cleanup fences.
    owner.0.quitting.set(true);
    if let Some(updates) = owner.0.updates.borrow_mut().as_mut() {
        updates.stop();
    }
    if !application.borrow().can_shutdown() {
        return;
    }
    if owner.0.draining.get() {
        return;
    }
    if let Err(error) = application.borrow_mut().shutdown() {
        eprintln!("application cleanup unconfirmed: {error}");
        return;
    }
    if owner.0.connections.is_some() || owner.0.caller.is_some() {
        let connections = owner.0.connections.clone();
        let caller = owner.0.caller.clone();
        let updates = owner.take_updates();
        let exit_mode = owner.exit_mode();
        owner.0.draining.set(true);
        let task = application.borrow().scheduler().spawn(async move {
            if let Some(updates) = updates {
                updates.drain().await;
            }
            if let Some(connections) = connections {
                connections.lock().await.drain_with(exit_mode).await;
            }
            if let Some(caller) = caller {
                caller.drain().await;
            }
        });
        cx.spawn(async move |cx| {
            if let Err(error) = task.await {
                eprintln!("backend cleanup observer failed: {error}");
                return;
            }
            cx.update(|cx| {
                let retained = cx.remove_global::<Owner>();
                debug_assert!(Rc::ptr_eq(&retained.0, &owner.0));
                // Every UI, Host and SQL receipt precedes native termination.
                cx.quit();
            });
        })
        .detach();
    } else {
        let retained = cx.remove_global::<Owner>();
        debug_assert!(Rc::ptr_eq(&retained.0, &owner.0));
        cx.quit();
    }
}
