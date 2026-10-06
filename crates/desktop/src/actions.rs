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
use super::{DesktopApplication, Surface};
use futures_util::{
    FutureExt,
    future::{LocalBoxFuture, Shared},
};
use gpui_kit::{App, Context, WeakEntity, Window};
use maka_ui::actions::{ActionTask, TaskCompletion};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
    sync::Arc,
};
type Connections = Arc<tokio::sync::Mutex<maka_application_bridge::Connections>>;
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
}
/// One actual foreground execution owner, shared by GPUI and its launcher.
/// Join observers survive a lost platform loop; no completion is fabricated.
#[derive(Clone)]
pub(super) struct Owner(Rc<State>);
impl gpui_kit::Global for Owner {}
impl Owner {
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
    fn complete(&self, id: uuid::Uuid) -> maka_ui::Result<Option<Observer>> {
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
    pub(super) fn pump(&self) -> maka_ui::Result<()> {
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
) -> maka_ui::Result<Owner> {
    if let Some(owner) = cx.try_global::<Owner>() {
        if !Rc::ptr_eq(&owner.0.application, &application) {
            return Err(maka_ui::Error::Invalid(
                "prior application cleanup not completed".into(),
            ));
        }
        return Ok(owner.clone());
    }
    let owner = Owner(Rc::new(State {
        application,
        jobs: RefCell::new(BTreeMap::new()),
        connections,
        caller,
        draining: Cell::new(false),
    }));
    cx.set_global(owner.clone());
    // Installed once per App. The callback captures no application/Root lease.
    if !cx.has_global::<QuitGate>() {
        cx.on_request_quit(request_quit)
            .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
        cx.set_global(QuitGate);
    }
    Ok(owner)
}
struct QuitGate;
impl gpui_kit::Global for QuitGate {}
fn request_quit(cx: &mut App) -> bool {
    let Some(owner) = cx.try_global::<Owner>().cloned() else {
        return true;
    };
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
) -> maka_ui::Result<()> {
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
    if !cx.windows().is_empty() || !application.borrow().can_shutdown() {
        return;
    }
    let Some(owner) = cx.try_global::<Owner>().cloned() else {
        return;
    };
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
        owner.0.draining.set(true);
        let task = application.borrow().scheduler().spawn(async move {
            tokio::join!(
                async {
                    if let Some(connections) = connections {
                        connections.lock().await.drain().await;
                    }
                },
                async {
                    if let Some(caller) = caller {
                        maka_application_bridge::drain_store(&caller).await;
                    }
                }
            );
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
