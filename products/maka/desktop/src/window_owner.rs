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

use super::{DesktopApplication, resources::NativeResources};
use gpui_kit::{App, AppContext, Task, Window};
use std::{cell::RefCell, rc::Rc};
use tokio::sync::mpsc::Receiver;

/// Owns cleanup independently of the visual Surface. Termination is governed by
/// the external platform receipt AND every real renderer-resource lease, not by
/// this owner's own wake sender. Neither receiver retains a platform window.
pub(super) fn start(
    window: &mut Window,
    cx: &mut App,
    resources: Rc<RefCell<NativeResources>>,
    application: Rc<RefCell<DesktopApplication>>,
    mut wakes: Receiver<()>,
    finished: impl FnOnce(Result<(), String>, &mut App) + 'static,
) -> Result<Task<()>, String> {
    let owner = super::actions::own(application.clone(), None, None, cx)
        .map_err(|error| error.to_string())?;
    owner.retain_native(resources.clone());
    let mut platform = window.take_cleanup_receiver().map_err(|e| e.to_string())?;
    let handle = window.window_handle();
    let id = handle.window_id();
    let closing_resources = resources.clone();
    let closing_application = application.clone();
    let removal_error = Rc::new(RefCell::new(None));
    let closing_error = removal_error.clone();
    let closed = cx.on_window_closed(move |_, closed_id| {
        if id == closed_id
            && let Err(error) = closing_resources
                .borrow_mut()
                .window_removed(&mut closing_application.borrow_mut())
        {
            *closing_error.borrow_mut() = Some(format!("component retirement failed: {error}"));
        }
    });
    Ok(cx.spawn(async move |cx| {
        // This subscription is released with the finite closing owner, including
        // cancellation. The task intentionally survives Surface disappearance.
        let _closed = closed;
        let mut platform_pending = true;
        loop {
            tokio::select! {
                outcome = &mut platform, if platform_pending => {
                    match outcome {
                        Ok(Ok(())) => { resources.borrow_mut().confirm_platform_cleanup(); platform_pending = false; }
                        outcome => {
                            let error = match outcome {
                                Ok(Err(error)) => error,
                                Err(error) => format!("platform cleanup receipt canceled: {error}"),
                                Ok(Ok(())) => unreachable!(),
                            };
                            cx.update(|cx| {
                                resources.borrow_mut().fail_platform_cleanup(&mut application.borrow_mut(), error.clone());
                                finished(Err(error), cx);
                            });
                            return;
                        }
                    }
                }
                wake = wakes.recv() => {
                    if wake.is_none() {
                        cx.update(|cx| finished(Err("native resource wake stream canceled".into()), cx));
                        return;
                    }
                }
            }
            if let Some(error) = removal_error.borrow_mut().take() {
                cx.update(|cx| finished(Err(error), cx));
                return;
            }
            let result = cx.update(|cx| {
                if !resources.borrow().is_removed() {
                    // A live window can clear its retired controls' focus. Once
                    // removed, focus belongs to the destroyed platform window.
                    cx.update_window(handle, |_, window, cx| {
                        resources.borrow_mut().deactivate_unmounted(&application.borrow(), window, cx)?;
                        // Background native observations wake this original window;
                        // they do not constitute a cleanup acknowledgement.
                        for owner in resources.borrow_mut().owners.values_mut() {
                            if owner.input.current.borrow().is_some() {
                                owner.recipe.publish_pending(owner.state.as_mut(), window, cx);
                            }
                        }
                        Ok::<_, maka_ui_runtime::Error>(())
                    }).map_err(|error| maka_ui_runtime::Error::Invalid(error.to_string()))??;
                }
                resources.borrow_mut().collect(&mut application.borrow_mut())
            });
            if let Err(error) = result {
                cx.update(|cx| finished(Err(error.to_string()), cx));
                return;
            }
            if resources.borrow().surface_closed {
                owner.release_native(&resources);
                cx.update(|cx| {
                    finished(Ok(()), cx);
                    super::actions::quit_when_drained(&application, cx);
                });
                return;
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::WindowCleanup;
    use std::cell::Cell;

    #[test]
    fn platform_receipt_waits_for_resource_drop_and_the_last_native_destructor_guard() {
        struct Resource(Rc<Cell<bool>>);
        impl Drop for Resource {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        struct Backend {
            _resource: Resource,
            cleanup: WindowCleanup,
        }
        let dropped = Rc::new(Cell::new(false));
        let mut backend = Backend {
            _resource: Resource(dropped.clone()),
            cleanup: WindowCleanup::default(),
        };
        let mut receipt = backend.cleanup.take_receiver().unwrap();
        assert!(backend.cleanup.take_receiver().is_err());
        let native_destructor = backend.cleanup.guard();
        drop(backend);
        assert!(dropped.get());
        assert_eq!(receipt.try_recv().unwrap(), None);
        native_destructor.confirm_native_destroyed();
        assert_eq!(
            receipt.try_recv().unwrap(),
            None,
            "super destructor must return before final guard drops"
        );
        drop(native_destructor);
        assert_eq!(receipt.try_recv().unwrap(), Some(Ok(())));
    }

    #[test]
    fn unknown_or_failed_platform_destruction_cannot_become_success() {
        let mut unknown = WindowCleanup::default();
        let mut receipt = unknown.take_receiver().unwrap();
        drop(unknown);
        assert!(receipt.try_recv().unwrap().unwrap().is_err());
        let mut failed = WindowCleanup::default();
        let mut receipt = failed.take_receiver().unwrap();
        let guard = failed.guard();
        guard.fail("destroy failed");
        assert_eq!(
            receipt.try_recv().unwrap(),
            Some(Err("destroy failed".into())),
            "known failure must be reported even while backend guards survive"
        );
        guard.confirm_native_destroyed();
        assert!(
            guard.native_destroyed(),
            "a failed cleanup cannot erase the observed native destruction fact"
        );
        guard.fail("later failure");
        drop(failed);
        drop(guard);
    }
}
