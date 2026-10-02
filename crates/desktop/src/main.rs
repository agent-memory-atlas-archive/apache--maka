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
use maka_ui::{Application, Direction, Node};
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
    application: Rc<RefCell<Application>>,
    resources: Rc<RefCell<resources::NativeResources>>,
    mount: Mount,
}
impl Surface {
    fn node(
        &self,
        application: &Application,
        mount: &Mount,
        node: &Node,
        revision: u64,
        lease: &resources::Lease,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match node {
            Node::Text { key, text, color } => div()
                .id(SharedString::from(key.clone()))
                .text_color(rgb(*color))
                .child(SharedString::from(text.clone()))
                .into_any_element(),
            Node::Button { key, label } => {
                let key = key.clone();
                let lease = lease.clone();
                Button::new(SharedString::from(key.clone()))
                    .label(SharedString::from(label.clone()))
                    .on_click(cx.listener(move |view, _, _, cx| {
                        let result = view.application.borrow_mut().dispatch(
                            lease.mount(),
                            revision,
                            &key,
                            serde_json::json!({"kind":"click"}),
                        );
                        if let Err(error) = result {
                            eprintln!("component event rejected: {error}");
                        } else {
                            if let Err(error) = view
                                .resources
                                .borrow_mut()
                                .collect(&mut view.application.borrow_mut())
                            {
                                eprintln!("native component cleanup unconfirmed: {error}");
                            }
                            cx.notify();
                        }
                    }))
                    .into_any_element()
            }
            Node::Component { key, .. } => {
                match application
                    .child_mount(mount, key)
                    .and_then(|child| application.frame(child).map(|frame| (child, frame)))
                {
                    Ok((child, frame)) => {
                        let lease = self.resources.borrow_mut().acquire(child);
                        owned::Owned {
                            content: self.node(
                                application,
                                child,
                                &frame.view,
                                frame.revision,
                                &lease,
                                cx,
                            ),
                            lease,
                        }
                        .into_any_element()
                    }
                    Err(_) => div().into_any_element(),
                }
            }
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
                            .map(|child| self.node(application, mount, child, revision, lease, cx)),
                    )
                    .into_any_element()
            }
        }
    }
}
impl Render for Surface {
    fn render(&mut self, _: &mut gpui_kit::Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Only committed Rust state is read here. Input callbacks run JS later.
        let application = self.application.borrow();
        match application.frame(&self.mount) {
            Ok(frame) => {
                let lease = self.resources.borrow_mut().acquire(&self.mount);
                div().size_full().child(owned::Owned {
                    content: self.node(
                        &application,
                        &self.mount,
                        &frame.view,
                        frame.revision,
                        &lease,
                        cx,
                    ),
                    lease,
                })
            }
            Err(_) => div().size_full(),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let mut application =
        Application::new(&input, packages, timer.handle().clone(), maka_js_limits())?;
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
        if let Err(error) = gpui_kit::open_window(options, cx, move |_, cx| {
            cx.new(|_| Surface {
                _cleanup: cleanup_task,
                application: content,
                resources,
                mount: content_mount,
            })
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

fn start_cleanup(
    cx: &App,
    resources: &Rc<RefCell<resources::NativeResources>>,
    application: &Rc<RefCell<Application>>,
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
    fn application(timer: &tokio::runtime::Runtime) -> Application {
        let (input, packages) =
            support::composed_fixture(support::CHILD_UPDATE, support::PARENT_EVENT);
        Application::new(&input, packages, timer.handle().clone(), Default::default()).unwrap()
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
        assert_eq!(resources.collect(&mut app).unwrap(), 1);
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
        let app = Rc::new(RefCell::new(app));
        let (wake, receipts) = tokio::sync::mpsc::unbounded_channel();
        let resources = Rc::new(RefCell::new(resources::NativeResources::new(
            root.clone(),
            wake,
        )));
        let (handle, _) = cx.update(|cx| {
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
                |_, cx| {
                    cx.new(|_| Surface {
                        application,
                        resources,
                        mount,
                        _cleanup: cleanup_task,
                    })
                },
            )
            .unwrap()
        });
        let draw = |cx: &mut TestAppContext| {
            cx.update_window(handle, |_, window, cx| {
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
        draw(cx);
        cx.run_until_parked();
        assert!(app.borrow().frame(&right).is_err());
        app.borrow_mut()
            .dispatch(&root, 1, "toggle", serde_json::Value::Null)
            .unwrap();
        draw(cx);
        let right = app.borrow().child_mount(&root, "right").unwrap().clone();
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
        assert_eq!(
            resources
                .borrow_mut()
                .collect(&mut app.borrow_mut())
                .unwrap(),
            0
        );
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
        drop(app);
        cx.run_until_parked();
        assert!(
            released.upgrade().is_none(),
            "receipt task must not keep the application alive"
        );
    }
}
