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
use clap::Parser;
use gpui_kit::{component::button::Button, *};
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
    application: Rc<RefCell<Application>>,
    mount: Mount,
}
impl Surface {
    fn node(&self, node: &Node, revision: u64, cx: &mut Context<Self>) -> AnyElement {
        match node {
            Node::Text { key, text, color } => div()
                .id(SharedString::from(key.clone()))
                .text_color(rgb(*color))
                .child(SharedString::from(text.clone()))
                .into_any_element(),
            Node::Button { key, label } => {
                let key = key.clone();
                Button::new(SharedString::from(key.clone()))
                    .label(SharedString::from(label.clone()))
                    .on_click(cx.listener(move |view, _, _, cx| {
                        let result = view.application.borrow_mut().dispatch(
                            &view.mount,
                            revision,
                            &key,
                            serde_json::json!({"kind":"click"}),
                        );
                        if let Err(error) = result {
                            eprintln!("component event rejected: {error}");
                        } else {
                            cx.notify();
                        }
                    }))
                    .into_any_element()
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
                    .children(children.iter().map(|child| self.node(child, revision, cx)))
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
            Ok(frame) => div()
                .size_full()
                .child(self.node(&frame.view, frame.revision, cx)),
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
    let dimensions = size(
        px(frame.window.width as f32),
        px(frame.window.height as f32),
    );
    let title = frame.window.title.clone();
    let application = Rc::new(RefCell::new(application));
    let startup_error = Rc::new(RefCell::new(None));
    let startup_result = startup_error.clone();
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
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
                application: content,
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

fn maka_js_limits() -> maka_js_runtime::plugin::Limits {
    maka_js_runtime::plugin::Limits {
        synchronous_slice: std::time::Duration::from_millis(100),
        ..Default::default()
    }
}
