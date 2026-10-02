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
mod owned;
mod resources;
mod widgets;
use clap::Parser;
use gpui_kit::{
    AnyElement, App, AppContext, Bounds, Context, InteractiveElement, IntoElement, ParentElement,
    Render, SharedString, Styled, Task, TitlebarOptions, WindowBounds, WindowOptions,
    component::button::Button, div, px, rgb, size,
};
#[cfg(test)]
use gpui_kit::{TestAppContext, point};
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
    _cleanup: Task<()>,
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
                if let Err(error) = view
                    .resources
                    .borrow_mut()
                    .collect(&mut view.application.borrow_mut())
                {
                    eprintln!("native component cleanup unconfirmed: {error}");
                }
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
    fn sync_native(
        &self,
        window: &mut gpui_kit::Window,
        cx: &mut Context<Self>,
    ) -> maka_ui::Result<()> {
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
                    revision: None,
                    state: native.view.create(input.clone(), window, cx),
                    input,
                }
            });
            *owner.input.0.borrow_mut() = Some(input);
            if owner.revision != Some(frame.revision) {
                // A consumed scene revision is not a successful native command:
                // widgets retain explicit applied/conflict receipts separately.
                native.view.commit(owner.state.as_mut(), window, cx);
                owner.revision = Some(frame.revision);
            }
        }
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
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        let (wake, receipts) = tokio::sync::mpsc::unbounded_channel();
        let resources = Rc::new(RefCell::new(resources::NativeResources::new(
            mount.clone(),
            wake,
        )));
        let cleanup_task = start_cleanup(cx, &resources, &application, receipts);
        let bounds = Bounds::centered(None, dimensions, cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some(title.into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let cleanup = application.clone();
        let retiring = mount.clone();
        cx.on_window_closed(move |cx, _| {
            if cx.windows().is_empty() {
                // GPUI reports logical removal before the platform window is
                // destroyed. Revoke now; do not forge a native close receipt.
                // This process exits, with the cleanup fence retained until
                // teardown. Reusable in-process surfaces need a platform ack.
                if let Err(error) = cleanup.borrow_mut().retire(&retiring) {
                    eprintln!("component retirement failed: {error}");
                }
                cx.quit();
            }
        })
        .detach();
        let content = application.clone();
        let content_mount = mount.clone();
        if let Err(error) = gpui_kit::open_window(options, cx, move |window, cx| {
            let surface = cx.new(|_| Surface {
                _cleanup: cleanup_task,
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
            *startup_error.borrow_mut() = Some(format!("native window creation failed: {error}"));
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

fn start_cleanup(
    cx: &App,
    resources: &Rc<RefCell<resources::NativeResources>>,
    application: &Rc<RefCell<DesktopApplication>>,
    mut receipts: tokio::sync::mpsc::UnboundedReceiver<()>,
) -> Task<()> {
    let resources = Rc::downgrade(resources);
    let application = Rc::downgrade(application);
    cx.spawn(async move |cx| {
        while receipts.recv().await.is_some() {
            let (Some(resources), Some(application)) = (resources.upgrade(), application.upgrade())
            else {
                break;
            };
            cx.update(|_| {
                if let Err(error) = resources
                    .borrow_mut()
                    .collect(&mut application.borrow_mut())
                {
                    eprintln!("native component cleanup unconfirmed: {error}");
                }
            });
        }
    })
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
            let cleanup_task = start_cleanup(cx, &resources, &application, receipts);
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
                    let surface = cx.new(|_| Surface {
                        application,
                        resources,
                        mount,
                        _cleanup: cleanup_task,
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
