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

use super::super::{Surface, resources::NativeResources, window_owner};
use super::Owner;
use gpui_kit::{
    AnyWindowHandle, App, AppContext, Bounds, TitlebarOptions, WeakEntity, WindowBounds,
    WindowOptions, px, size,
};
use maka_plugin_core::composition::RootKind;
#[cfg(test)]
use maka_plugin_core::deployment::DeploymentRef;
use maka_plugin_core::runtime::Mount;
#[cfg(test)]
use maka_ui_runtime::CommittedUpdate;
use maka_ui_runtime::{
    Error, PreparedUpdate, Result, Surface as Declaration, actions::RemoteService,
};
use serde_json::Value;
use std::{cell::RefCell, rc::Rc};

pub(super) enum Cleanup {
    Pending,
    Confirmed,
    Failed,
}
pub(super) enum State {
    Preparing {
        mount: Mount,
        props: Value,
    },
    Opening {
        mount: Mount,
    },
    Live {
        mount: Mount,
        window: AnyWindowHandle,
        surface: WeakEntity<Surface>,
        props: Value,
    },
    Replacing {
        mount: Mount,
        props: Value,
        kind: RootKind,
        cleanup: Cleanup,
    },
    Failed {
        mount: Mount,
    },
}
impl Owner {
    pub(super) fn advance_ui(&self) -> Result<()> {
        // Preparing follows the old window's confirmed cleanup. This candidate
        // has not acquired a physical surface or native resource owner yet.
        let replacement = match &*self.0.launch.borrow() {
            Some(State::Preparing { mount, props })
                if self.0.application.borrow().can_recover(mount)
                    && !self.0.native.borrow().contains_key(&mount.id()) =>
            {
                Some((mount.clone(), props.clone()))
            }
            _ => None,
        };
        let result = self.0.application.borrow_mut().advance_ui();
        if let (Some((mount, props)), Err(error @ (Error::Script(_) | Error::Invalid(_)))) =
            (replacement, &result)
            && !self.0.application.borrow().logical_pending()
            && self.0.application.borrow().frame(&mount).is_err()
            && self.0.application.borrow().prepared_frame(&mount).is_err()
            && !self.0.native.borrow().contains_key(&mount.id())
        {
            eprintln!("replacement UI candidate rejected: {error}");
            *self.0.launch.borrow_mut() = Some(State::Replacing {
                mount,
                props,
                kind: RootKind::Recovery,
                cleanup: Cleanup::Confirmed,
            });
            return Ok(());
        }
        result.map(|_| ())
    }

    /// Called against the actual current window, before its removal. Original
    /// window/resource receipts still gate mounting the independent root.
    pub(crate) fn recover_launch(
        &self,
        mount: &Mount,
        window: &mut gpui_kit::Window,
    ) -> Result<bool> {
        if self.0.quitting.get() || self.0.draining.get() {
            return Ok(false);
        }
        let kind = {
            let application = self.0.application.borrow();
            if application.is_retiring(mount) && application.can_recover(mount) {
                RootKind::Recovery
            } else if application.can_resume_launch(mount) {
                RootKind::Launch
            } else {
                return Ok(false);
            }
        };
        let props = match &*self.0.launch.borrow() {
            Some(State::Live {
                mount: current,
                window: original,
                props,
                ..
            }) if current == mount
                && original.window_id() == window.window_handle().window_id() =>
            {
                props.clone()
            }
            _ => return Ok(false),
        };
        self.0.application.borrow_mut().retire(mount)?;
        *self.0.launch.borrow_mut() = Some(State::Replacing {
            mount: mount.clone(),
            props,
            kind,
            cleanup: Cleanup::Pending,
        });
        window.remove_window();
        Ok(true)
    }

    #[cfg(test)]
    pub(crate) fn current_launch(&self) -> Option<(Mount, AnyWindowHandle, WeakEntity<Surface>)> {
        match &*self.0.launch.borrow() {
            Some(State::Live {
                mount,
                window,
                surface,
                ..
            }) => Some((mount.clone(), *window, surface.clone())),
            _ => None,
        }
    }
    pub(crate) fn failure(&self) -> Option<String> {
        self.0.failure.borrow().clone()
    }
    fn fail_launch(&self, mount: &Mount, error: String) {
        self.0.failure.borrow_mut().get_or_insert(error);
        *self.0.launch.borrow_mut() = Some(State::Failed {
            mount: mount.clone(),
        });
    }
    pub(crate) fn replacing(&self) -> bool {
        matches!(
            *self.0.launch.borrow(),
            Some(State::Replacing { .. } | State::Preparing { .. })
        )
    }
    #[cfg(test)]
    pub(crate) fn load_committed_update(
        &self,
        source: DeploymentRef,
    ) -> Result<impl std::future::Future<Output = Result<CommittedUpdate>> + Send + 'static + use<>>
    {
        if self.0.quitting.get() || self.0.draining.get() {
            return Err(Error::Invalid("desktop application is exiting".into()));
        }
        self.0.application.borrow().load_committed_update(source)
    }
    #[cfg(test)]
    pub(crate) fn adopt_update(
        &self,
        update: CommittedUpdate,
        registrations: Vec<RemoteService>,
        cx: &mut App,
    ) -> Result<()> {
        let prepared = self.0.application.borrow().prepare_update(update)?;
        self.adopt_prepared(prepared, registrations, cx)
    }
    pub(crate) fn adopt_prepared(
        &self,
        update: PreparedUpdate,
        registrations: Vec<RemoteService>,
        cx: &mut App,
    ) -> Result<()> {
        if self.0.quitting.get() || self.0.draining.get() {
            return Err(Error::Invalid("desktop application is exiting".into()));
        }
        let live = match &*self.0.launch.borrow() {
            Some(State::Live {
                mount,
                window,
                surface,
                props,
            }) => Some((
                mount.clone(),
                *window,
                surface.clone(),
                props.clone(),
                !self.0.application.borrow().is_retiring(mount),
                self.0.application.borrow().root_kind(mount)?,
            )),
            _ => None,
        };
        self.0
            .application
            .borrow_mut()
            .adopt_prepared(update, registrations)?;
        if let Some((mount, window, surface, props, was_live, kind)) = live {
            let replacing = was_live && self.0.application.borrow().is_retiring(&mount);
            if replacing {
                *self.0.launch.borrow_mut() = Some(State::Replacing {
                    mount: mount.clone(),
                    props,
                    kind,
                    cleanup: Cleanup::Pending,
                });
            }
            let result = window.update(cx, |_, window, cx| {
                // Even failed advancement first revokes retired native inputs.
                let result = surface.update(cx, |surface, cx| surface.sync_native(window, cx));
                if replacing {
                    window.remove_window();
                }
                result
            });
            match result {
                Err(error) | Ok(Err(error)) => self.fail_launch(&mount, error.to_string()),
                Ok(Ok(Err(error))) => {
                    eprintln!("deployment native synchronization failed: {error}")
                }
                Ok(Ok(Ok(()))) => {}
            }
        }
        super::quit_when_drained(&self.0.application, cx);
        Ok(())
    }
    pub(crate) fn resume_launch(&self, cx: &mut App) -> Result<bool> {
        if self.0.quitting.get() || self.0.draining.get() {
            return Ok(false);
        }
        let prepared = match &*self.0.launch.borrow() {
            Some(State::Preparing { mount, props }) => Some((mount.clone(), props.clone())),
            _ => None,
        };
        if let Some((mount, props)) = prepared {
            if self.0.application.borrow().frame(&mount).is_err()
                && self.0.application.borrow().prepared_frame(&mount).is_err()
            {
                if self.0.application.borrow().is_retiring(&mount) {
                    self.fail_launch(&mount, "replacement mount preparation failed".into());
                    return Err(Error::Invalid(
                        "replacement mount preparation failed".into(),
                    ));
                }
                return Ok(false);
            }
            self.open_launch(mount, props, cx)?;
            return Ok(true);
        }
        let (props, kind) = match &*self.0.launch.borrow() {
            Some(State::Replacing {
                cleanup: Cleanup::Confirmed,
                props,
                kind,
                ..
            }) => (props.clone(), *kind),
            _ => return Ok(false),
        };
        let Some(mount) = self
            .0
            .application
            .borrow_mut()
            .mount_replacement_root(kind, props.clone())?
        else {
            return Ok(false);
        };
        if self.0.application.borrow().frame(&mount).is_err()
            && self.0.application.borrow().prepared_frame(&mount).is_err()
        {
            *self.0.launch.borrow_mut() = Some(State::Preparing { mount, props });
            return Ok(false);
        }
        self.open_launch(mount, props, cx)?;
        Ok(true)
    }
    fn window_finished(
        &self,
        mount: &Mount,
        outcome: std::result::Result<(), String>,
        cx: &mut App,
    ) {
        let matched = {
            let mut launch = self.0.launch.borrow_mut();
            match launch.as_mut() {
                Some(State::Replacing {
                    mount: original,
                    cleanup,
                    ..
                }) if original == mount && matches!(cleanup, Cleanup::Pending) => {
                    // The independent window owner confirms platform AND every lease.
                    *cleanup = if outcome.is_ok() {
                        Cleanup::Confirmed
                    } else {
                        Cleanup::Failed
                    };
                    true
                }
                Some(
                    State::Live {
                        mount: original, ..
                    }
                    | State::Opening { mount: original }
                    | State::Preparing {
                        mount: original, ..
                    },
                ) if original == mount => {
                    *launch = None;
                    true
                }
                Some(State::Failed { mount: original }) if original == mount => true,
                _ => false,
            }
        };
        if !matched {
            return;
        }
        if let Err(error) = outcome {
            self.0.failure.borrow_mut().get_or_insert(error);
        }
        // All native/kernel/launch-state borrows ended before possible reentry.
        super::quit_when_drained(&self.0.application, cx);
    }
    pub(crate) fn open_launch(&self, mount: Mount, props: Value, cx: &mut App) -> Result<()> {
        if self.0.quitting.get() || self.0.draining.get() {
            return Err(Error::Invalid("desktop application is exiting".into()));
        }
        let (title, width, height) = {
            let app = self.0.application.borrow();
            let Some(Declaration::Window {
                title,
                width,
                height,
                ..
            }) = &app
                .frame(&mount)
                .or_else(|_| app.prepared_frame(&mount))?
                .surface
            else {
                return Err(Error::Invalid(
                    "desktop launch component must request a window".into(),
                ));
            };
            (title.clone(), *width, *height)
        };
        *self.0.launch.borrow_mut() = Some(State::Opening {
            mount: mount.clone(),
        });
        let (wake, receipts) = tokio::sync::mpsc::channel(1);
        let resources = Rc::new(RefCell::new(NativeResources::new(mount.clone(), wake)));
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(width as f32), px(height as f32)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some(title.into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let owner = self.clone();
        let original = mount.clone();
        let failed = Rc::new(RefCell::new(None));
        let failure = failed.clone();
        let result = gpui_kit::open_window(options, cx, move |window, cx| {
            let finished = owner.clone();
            let closed_mount = original.clone();
            let closing = window_owner::start(
                window,
                cx,
                resources.clone(),
                owner.0.application.clone(),
                receipts,
                move |outcome, cx| finished.window_finished(&closed_mount, outcome, cx),
            );
            let surface = cx.new(|_| Surface {
                application: owner.0.application.clone(),
                resources: resources.clone(),
                mount: original.clone(),
            });
            match closing {
                Ok(task) => {
                    task.detach();
                    *owner.0.launch.borrow_mut() = Some(State::Live {
                        mount: original.clone(),
                        window: window.window_handle(),
                        surface: surface.downgrade(),
                        props,
                    });
                    let native = surface.update(cx, |surface, cx| surface.sync_native(window, cx));
                    if let Err(error) = native {
                        let error = error.to_string();
                        owner.fail_launch(&original, error.clone());
                        *failure.borrow_mut() = Some(error);
                        let _ = resources
                            .borrow_mut()
                            .window_removed(&mut owner.0.application.borrow_mut());
                        window.remove_window();
                    }
                }
                Err(error) => {
                    owner.fail_launch(&original, error.clone());
                    *failure.borrow_mut() = Some(error.clone());
                    let _ = resources
                        .borrow_mut()
                        .window_removed(&mut owner.0.application.borrow_mut());
                    let _ = owner
                        .0
                        .application
                        .borrow_mut()
                        .close(&original, Err(error));
                    window.remove_window();
                }
            }
            surface
        });
        if let Err(error) = result {
            let message = format!("native window creation failed; cleanup unconfirmed: {error}");
            self.fail_launch(&mount, message.clone());
            let _ = self
                .0
                .application
                .borrow_mut()
                .close(&mount, Err(message.clone()));
            return Err(Error::Invalid(message));
        }
        if let Some(error) = failed.borrow_mut().take() {
            return Err(Error::Invalid(error));
        }
        cx.activate(true);
        Ok(())
    }
}
