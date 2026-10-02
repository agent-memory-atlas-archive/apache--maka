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
use gpui_kit::{App, Context, WeakEntity, Window};
use maka_ui::actions::{ActionTask, TaskCompletion};
use std::{cell::RefCell, rc::Rc};
/// One application lifetime owner, not a second runtime or state copy. It keeps
/// failed cleanup and its recovery entry alive after every window/task exits.
struct Owner(Rc<RefCell<DesktopApplication>>);
impl gpui_kit::Global for Owner {}
pub(super) fn own(
    application: Rc<RefCell<DesktopApplication>>,
    cx: &mut App,
) -> maka_ui::Result<()> {
    if let Some(owner) = cx.try_global::<Owner>() {
        if !Rc::ptr_eq(&owner.0, &application) {
            return Err(maka_ui::Error::Invalid(
                "prior application cleanup not completed".into(),
            ));
        }
    } else {
        cx.set_global(Owner(application));
    }
    Ok(())
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
        let result = app.borrow_mut().deliver_action_results(&root);
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
    mut action: ActionTask,
    cx: &mut App,
) {
    // Detached from Surface: the owner retains Application until actual worker
    // completion, including when the window and its native owners have closed.
    cx.spawn(async move |cx| {
        loop {
            let worker = application.borrow().scheduler().spawn(action.future);
            let completion = worker
                .await
                .unwrap_or_else(|error| TaskCompletion::Interrupted(error.to_string()));
            let next = cx.update(|cx| {
                let next = match application.borrow_mut().finish_task(action.id, completion) {
                    Ok(next) => next,
                    Err(error) => {
                        eprintln!("service cleanup unconfirmed: {error}");
                        None
                    }
                };
                if let Err(error) = application.borrow_mut().deliver_action_results(&root) {
                    eprintln!("action result rejected: {error}");
                }
                let _ = surface.update(cx, |_, cx| cx.notify());
                let _ = window.update(cx, |_, window, cx| {
                    let _ = surface.update(cx, |surface, cx| {
                        if let Err(error) = surface.sync_native(window, cx) {
                            eprintln!("native action completion failed: {error}");
                        }
                        cx.notify();
                    });
                });
                quit_when_drained(&application, cx);
                next
            });
            let Some(next) = next else {
                break;
            };
            action = next;
        }
    })
    .detach();
}

/// Last-window removal alone is not platform cleanup. Both all real mount
/// receipts and independent accepted Work must finish before automatic exit.
pub(super) fn quit_when_drained(application: &Rc<RefCell<DesktopApplication>>, cx: &mut App) {
    if cx.windows().is_empty() && application.borrow().can_shutdown() {
        match application.borrow_mut().shutdown() {
            Ok(()) => {
                if cx.has_global::<Owner>() {
                    let owner = cx.remove_global::<Owner>();
                    debug_assert!(Rc::ptr_eq(&owner.0, application));
                }
                cx.quit();
            }
            Err(error) => eprintln!("application cleanup unconfirmed: {error}"),
        }
    }
}
