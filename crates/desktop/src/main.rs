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

extern crate gpui_kit as gpui;
mod actions;
mod owned;
mod resources;
mod widgets;
mod window_owner;
use clap::Parser;
use gpui_kit::{
    AnyElement, AppContext, Bounds, Context, InteractiveElement, IntoElement, ParentElement,
    Render, SharedString, Styled, TitlebarOptions, WindowBounds, WindowOptions,
    component::button::Button, div, px, rgb, size,
};
#[cfg(test)]
use gpui_kit::{App, TestAppContext, point};
use maka_plugin_core::{
    composition::{Input, PresentationTarget},
    runtime::Mount,
};
use maka_ui::{Application, Direction, Node, Presentation};
type DesktopApplication = Application<dyn widgets::Widget>;
use std::{cell::RefCell, io::Read, path::PathBuf, rc::Rc};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    input: PathBuf,
    #[arg(long = "package", required = true)]
    packages: Vec<PathBuf>,
}

struct Surface {
    application: Rc<RefCell<DesktopApplication>>,
    resources: Rc<RefCell<resources::NativeResources>>,
    mount: Mount,
}
struct RenderScope<'a> {
    application: &'a DesktopApplication,
    mount: &'a Mount,
    input: widgets::Input,
}
impl Surface {
    fn input(&self, lease: &resources::Lease, revision: u64, cx: &Context<Self>) -> widgets::Input {
        let lease = lease.clone();
        let view = cx.weak_entity();
        widgets::Input(Rc::new(move |key, event, window, cx| {
            let result = view.update(cx, |view, cx| {
                view.application
                    .borrow_mut()
                    .dispatch(lease.mount(), revision, key, event)?;
                view.sync_native(window, cx)?;
                cx.notify();
                maka_ui::Result::Ok(())
            });
            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => eprintln!("component input rejected: {error}"),
                Err(error) => eprintln!("component input owner unavailable: {error}"),
            }
        }))
    }
    /// Native creation/commands run after the whole declarative transaction,
    /// never during layout/paint. The store owns resources, not component state.
    fn prepare_native(
        &self,
        window: &mut gpui_kit::Window,
        cx: &mut Context<Self>,
    ) -> maka_ui::Result<()> {
        if self.resources.borrow().is_removed() {
            return Ok(());
        }
        let application = self.application.borrow();
        for mount in application.mounted_subtree(&self.mount)? {
            let frame = application.frame(&mount)?;
            let id = mount.id().to_string();
            let Presentation::Native(native) = &frame.view else {
                self.resources.borrow_mut().owners.remove(&id);
                continue;
            };
            let lease = self.resources.borrow_mut().acquire(&mount);
            let input = self.input(&lease, frame.revision, cx);
            let mut resources = self.resources.borrow_mut();
            let kind = native.view.as_ref().type_id();
            if resources
                .owners
                .get(&id)
                .is_some_and(|owner| owner.kind != kind)
            {
                resources.owners.remove(&id);
            }
            let owner = resources.owners.entry(id).or_insert_with(|| {
                let input = widgets::LiveInput(Rc::new(RefCell::new(Some(input.clone()))));
                resources::NativeInstance {
                    kind,
                    recipe: native.view.clone(),
                    revision: None,
                    state: native.view.create(input.clone(), window, cx),
                    input,
                }
            });
            *owner.input.0.borrow_mut() = Some(input);
            owner.recipe = native.view.clone();
        }
        Ok(())
    }
    fn sync_native(
        &self,
        window: &mut gpui_kit::Window,
        cx: &mut Context<Self>,
    ) -> maka_ui::Result<()> {
        if self.resources.borrow().is_removed() {
            return Ok(());
        }
        self.prepare_native(window, cx)?;
        self.application
            .borrow_mut()
            .capture_actions(&self.mount, |target| {
                let mut resources = self.resources.borrow_mut();
                let owner = resources
                    .owners
                    .get_mut(&target.mount().id().to_string())
                    .ok_or_else(|| {
                        maka_ui::Error::Invalid("native capture owner unavailable".into())
                    })?;
                let recipe = owner.recipe.clone();
                recipe.capture(owner.state.as_mut(), target, window, cx)
            })?;
        let application = self.application.borrow();
        for mount in application.mounted_subtree(&self.mount)? {
            let frame = application.frame(&mount)?;
            let Presentation::Native(native) = &frame.view else {
                continue;
            };
            let mut resources = self.resources.borrow_mut();
            let owner = resources
                .owners
                .get_mut(&mount.id().to_string())
                .expect("prepared native owner");
            if owner.revision != Some(frame.revision) {
                // A consumed scene revision is not a successful native command:
                // widgets retain explicit applied/conflict receipts separately.
                native.view.commit(owner.state.as_mut(), window, cx);
                owner.revision = Some(frame.revision);
            }
        }
        drop(application);
        self.resources
            .borrow_mut()
            .collect(&mut self.application.borrow_mut())?;
        actions::pump(
            self.application.clone(),
            self.mount.clone(),
            cx.weak_entity(),
            window,
            cx,
        )?;
        Ok(())
    }
    fn presentation(
        &self,
        application: &DesktopApplication,
        mount: &Mount,
        window: &mut gpui_kit::Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Ok(frame) = application.frame(mount) else {
            return div().into_any_element();
        };
        let lease = self.resources.borrow_mut().acquire(mount);
        let input = self.input(&lease, frame.revision, cx);
        let content = match &frame.view {
            Presentation::Tree(node) => self.node(
                &RenderScope {
                    application,
                    mount,
                    input,
                },
                node,
                window,
                cx,
            ),
            Presentation::Native(native) => {
                let resources = self.resources.borrow();
                let owner = resources
                    .owners
                    .get(&mount.id().to_string())
                    .expect("native owner synced before rendering");
                native.view.render(owner.state.as_ref(), window, cx)
            }
        };
        owned::Owned { content, lease }.into_any_element()
    }
    fn node(
        &self,
        scope: &RenderScope<'_>,
        node: &Node,
        window: &mut gpui_kit::Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match node {
            Node::Text { key, text, color } => div()
                .id(SharedString::from(key.clone()))
                .text_color(rgb(*color))
                .child(SharedString::from(text.clone()))
                .into_any_element(),
            Node::Button { key, label } => Button::new(SharedString::from(key.clone()))
                .label(SharedString::from(label.clone()))
                .on_click(
                    scope
                        .input
                        .bind(key.clone(), |_| serde_json::json!({"kind":"click"})),
                )
                .into_any_element(),
            Node::Component { key, .. } => match scope.application.child_mount(scope.mount, key) {
                Ok(child) => self.presentation(scope.application, child, window, cx),
                Err(_) => div().into_any_element(),
            },
            Node::Stack {
                key,
                direction,
                gap,
                padding,
                background,
                children,
            } => {
                let element = div()
                    .id(SharedString::from(key.clone()))
                    .flex()
                    .gap(px(*gap as f32))
                    .p(px(*padding as f32))
                    .bg(rgb(*background));
                let element = match direction {
                    Direction::Row => element.flex_row(),
                    Direction::Column => element.flex_col(),
                };
                element
                    .children(
                        children
                            .iter()
                            .map(|child| self.node(scope, child, window, cx)),
                    )
                    .into_any_element()
            }
        }
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
        let application = self.application.borrow();
        div()
            .size_full()
            .child(self.presentation(&application, &self.mount, window, cx))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    log::set_logger(&NativeLog)?;
    log::set_max_level(log::LevelFilter::Warn);
    let args = Args::parse();
    let mut bytes = Vec::new();
    std::fs::File::open(args.input)?
        .take(512 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 512 * 1024 {
        return Err("application input exceeds 512 KiB".into());
    }
    let input: Input = serde_json::from_slice(&bytes)?;
    if input.environment.presentation != Some(PresentationTarget::Desktop) {
        return Err("desktop presentation required".into());
    }
    let packages = args
        .packages
        .into_iter()
        .map(|path| maka_plugins::package::Package::read_from(&std::path::absolute(path)?))
        .collect::<Result<Vec<_>, _>>()?;
    let timer = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_time()
        .build()?;
    let mut application = Application::new(
        &input,
        packages,
        timer.handle().clone(),
        maka_js_limits(),
        widgets::registry(),
    )?;
    let mount = application.mount_launch(serde_json::json!({}))?;
    let frame = application.frame(&mount)?;
    let window = frame
        .window
        .as_ref()
        .ok_or("launch component has no window")?;
    let dimensions = size(px(window.width as f32), px(window.height as f32));
    let title = window.title.clone();
    let application = Rc::new(RefCell::new(application));
    let startup_error = Rc::new(RefCell::new(None));
    let startup_result = startup_error.clone();
    gpui_kit::application()
        .with_quit_mode(gpui_kit::QuitMode::Explicit)
        .run(move |cx| {
            gpui_kit::init(cx);
            let (wake, receipts) = tokio::sync::mpsc::unbounded_channel();
            let resources = Rc::new(RefCell::new(resources::NativeResources::new(
                mount.clone(),
                wake,
            )));
            let bounds = Bounds::centered(None, dimensions, cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(title.into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            actions::own(application.clone(), cx).expect("unique UI application owner");
            let content = application.clone();
            let content_mount = mount.clone();
            let content_error = startup_error.clone();
            let quit_application = application.clone();
            if let Err(error) = gpui_kit::open_window(options, cx, move |window, cx| {
                let result = content_error.clone();
                match window_owner::start(
                    window,
                    cx,
                    resources.clone(),
                    content.clone(),
                    receipts,
                    move |outcome, cx| {
                        if let Err(error) = outcome {
                            eprintln!("native cleanup unconfirmed: {error}");
                            *result.borrow_mut() =
                                Some(format!("native cleanup unconfirmed: {error}"));
                        }
                        actions::quit_when_drained(&quit_application, cx);
                    },
                ) {
                    Ok(task) => task.detach(),
                    Err(error) => {
                        eprintln!("native window cleanup unavailable: {error}");
                        let _ = resources
                            .borrow_mut()
                            .window_removed(&mut content.borrow_mut());
                        let _ = content
                            .borrow_mut()
                            .close(&content_mount, Err(error.clone()));
                        *content_error.borrow_mut() = Some(error);
                        window.remove_window();
                        cx.quit();
                    }
                }
                let surface = cx.new(|_| Surface {
                    application: content,
                    resources,
                    mount: content_mount,
                });
                surface.update(cx, |surface, cx| {
                    surface
                        .sync_native(window, cx)
                        .expect("committed native mounts")
                });
                surface
            }) {
                let _ = application.borrow_mut().retire(&mount);
                *startup_error.borrow_mut() =
                    Some(format!("native window creation failed: {error}"));
                cx.quit();
            } else {
                cx.activate(true);
            }
        });
    if let Some(error) = startup_result.borrow_mut().take() {
        return Err(error.into());
    }
    Ok(())
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
#[path = "../../ui/examples/counter/support.rs"]
mod support;

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
            Default::default(),
            widgets::registry(),
        )
        .unwrap()
    }

    #[test]
    fn never_presented_child_can_close_but_visible_retired_mounts_stay_fenced() {
        let timer = timer();
        let mut app = application(&timer);
        let root = app.mount_launch(serde_json::json!({})).unwrap();
        let right = app.child_mount(&root, "right").unwrap().clone();
        let other_root = app.mount_launch(serde_json::json!({})).unwrap();
        let other_child = app.child_mount(&other_root, "right").unwrap().clone();
        app.retire(&other_root).unwrap();
        let (wake, _) = tokio::sync::mpsc::unbounded_channel();
        let mut resources = resources::NativeResources::new(root.clone(), wake);
        app.dispatch(&root, 0, "toggle", serde_json::Value::Null)
            .unwrap();
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
        let timer = timer();
        let mut application = application(&timer);
        let root = application.mount_launch(serde_json::json!({})).unwrap();
        let app = Rc::new(RefCell::new(application));
        let (wake, wakes) = tokio::sync::mpsc::unbounded_channel();
        let resources = Rc::new(RefCell::new(resources::NativeResources::new(
            root.clone(),
            wake,
        )));
        cx.update(|cx| {
            gpui_kit::init(cx);
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
    }

    #[gpui::test]
    fn closing_one_window_does_not_retire_or_collect_another_surface(cx: &mut TestAppContext) {
        let timer = timer();
        let mut application = application(&timer);
        let first = application.mount_launch(serde_json::json!({})).unwrap();
        let second = application.mount_launch(serde_json::json!({})).unwrap();
        let app = Rc::new(RefCell::new(application));
        let finished = Rc::new(std::cell::Cell::new(0));
        let open = |mount: &Mount| {
            let (wake, wakes) = tokio::sync::mpsc::unbounded_channel();
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
        let timer = timer();
        let mut application = application(&timer);
        let root = application.mount_launch(serde_json::json!({})).unwrap();
        let child = application.child_mount(&root, "right").unwrap().clone();
        let application = Rc::new(RefCell::new(application));
        let (wake, wakes) = tokio::sync::mpsc::unbounded_channel();
        let resources = Rc::new(RefCell::new(resources::NativeResources::new(
            root.clone(),
            wake,
        )));
        let held_root = resources.borrow_mut().acquire(&root);
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
        let timer = timer();
        let mut app = application(&timer);
        let root = app.mount_launch(serde_json::json!({})).unwrap();
        let right = app.child_mount(&root, "right").unwrap().clone();
        let native = app.child_mount(&root, "nativeLabel").unwrap().clone();
        assert!(matches!(
            app.frame(&native).unwrap().view,
            maka_ui::Presentation::Native(_)
        ));
        let app = Rc::new(RefCell::new(app));
        let (wake, receipts) = tokio::sync::mpsc::unbounded_channel();
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
