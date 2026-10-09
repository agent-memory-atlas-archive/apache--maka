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

//! Native retained terminal with ordinary component observations and real IME.
mod input;
use crate::{
    LiveInput, Slots, Widget,
    appearance::{color, font},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, App, AppContext, Bounds, Context, Entity, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, Pixels, Render, StatefulInteractiveElement, Styled, Window, canvas,
    div, point, px, size,
};
use maka_ui::{
    ComponentCall, ComponentOutput, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView, terminal::*,
};
use std::{any::Any, collections::BTreeMap, rc::Rc};
#[derive(Clone)]
struct Geometry {
    bounds: Bounds<Pixels>,
    cell: Pixels,
    line: Pixels,
    epoch: u64,
}
struct Owner {
    input: LiveInput,
    focus: FocusHandle,
    props: TerminalViewProps,
    core: state::State,
    geometry: Option<Geometry>,
    epoch: u64,
    visible: bool,
    alive: bool,
    publishing: bool,
    marked: String,
    selected: std::ops::Range<usize>,
}
impl Owner {
    fn new(
        props: &TerminalViewProps,
        input: LiveInput,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let keep = input
            .0
            .borrow()
            .as_ref()
            .expect("live terminal mount")
            .clone();
        cx.new(|cx| {
            let focus = cx
                .focus_handle()
                .tab_index(props.tab_index as isize)
                .tab_stop(props.tab_index >= 0);
            cx.on_release(move |_: &mut Self, _| drop(keep)).detach();
            cx.on_focus(&focus, window, |owner: &mut Self, window, cx| {
                owner.core.capture(true);
                owner.schedule(window, cx);
                cx.notify();
            })
            .detach();
            cx.on_blur(&focus, window, |owner: &mut Self, window, cx| {
                owner.core.capture(false);
                owner.marked.clear();
                owner.selected = 0..0;
                owner.schedule(window, cx);
                cx.notify();
            })
            .detach();
            Self {
                core: state::State::new(&input.1),
                input,
                focus,
                props: props.clone(),
                geometry: None,
                epoch: 0,
                visible: false,
                alive: true,
                publishing: false,
                marked: String::new(),
                selected: 0..0,
            }
        })
    }
    fn enabled(&self) -> bool {
        self.alive && self.visible && self.input.0.borrow().is_some()
    }
    fn focused(&self, window: &Window) -> bool {
        self.enabled()
            && self.focus.is_focused(window)
            && self
                .geometry
                .as_ref()
                .is_some_and(|g| g.epoch == self.epoch)
    }
    fn admits(&self, window: &Window) -> bool {
        self.focused(window) && self.core.ready()
    }
    fn apply(&mut self, props: &TerminalViewProps, window: &mut Window, cx: &mut Context<Self>) {
        if self.props.source != props.source
            || self.props.controller != props.controller
            || self.props.appearance != props.appearance
            || self.props.rows != props.rows
        {
            self.epoch += 1;
            self.geometry = None;
            self.core.presentation(false);
            self.marked.clear();
            self.selected = 0..0;
        }
        self.props = props.clone();
        self.core.apply(props);
        self.focus = self
            .focus
            .clone()
            .tab_index(props.tab_index as isize)
            .tab_stop(self.enabled() && props.tab_index >= 0);
        self.schedule(window, cx);
        cx.notify();
    }
    fn visibility(&mut self, visible: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.visible = visible;
        self.focus = self
            .focus
            .clone()
            .tab_stop(self.enabled() && self.props.tab_index >= 0);
        if !self.enabled() {
            self.epoch += 1;
            self.geometry = None;
            self.marked.clear();
            self.selected = 0..0;
            self.core.presentation(false);
            if self.focus.is_focused(window) {
                window.blur(cx);
            }
        }
        self.schedule(window, cx);
        cx.notify();
    }
    fn schedule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.publishing
            || self.core.pending_observation().is_none()
            || self.input.0.borrow().is_none()
        {
            return;
        }
        self.publishing = true;
        let weak = cx.weak_entity();
        window.defer(cx, move |window, cx| {
            let Ok(Some((input, value))) = weak.update(cx, |owner, _| {
                owner.publishing = false;
                Some((
                    owner.input.0.borrow().clone()?,
                    owner.core.pending_observation()?,
                ))
            }) else {
                return;
            };
            if (input.0)(
                "observe",
                serde_json::to_value(&value).expect("terminal observation"),
                window,
                cx,
            )
            .is_ok()
            {
                let _ = weak.update(cx, |owner, _| owner.core.published(value.revision));
            }
        });
    }
    fn present(&mut self, geometry: Geometry, window: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled() || geometry.epoch != self.epoch {
            return;
        }
        let before = self.core.observation().revision;
        self.core.presentation(true);
        self.core.resize(
            (f32::from(geometry.bounds.size.width) / f32::from(geometry.cell))
                .floor()
                .clamp(2., 240.) as u16,
            (f32::from(geometry.bounds.size.height) / f32::from(geometry.line))
                .floor()
                .clamp(1., 100.) as u16,
        );
        self.core.capture(self.focus.is_focused(window));
        self.geometry = Some(geometry);
        self.schedule(window, cx);
        if self.core.observation().revision != before {
            cx.notify();
        }
    }
}
impl Render for Owner {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.enabled() {
            return div().hidden().into_any_element();
        }
        let a = self.props.appearance.clone();
        let mut face = font(&a.font);
        face.features = gpui_kit::FontFeatures(std::sync::Arc::new(vec![
            ("calt".into(), 0),
            ("liga".into(), 0),
        ]));
        let font_size = px(a.font.size);
        let cell = window
            .text_system()
            .advance(window.text_system().resolve_font(&face), font_size, 'M')
            .map(|s| s.width)
            .unwrap_or(px(a.font.size * 0.6))
            .max(px(1.));
        let line = px(a.font.size * a.font.line_height);
        let rows = self.props.rows;
        let source = self.core.text(usize::from(rows));
        let screen = self.core.display.as_ref().map(|d| d.cursor.clone());
        let cursor = self.core.captured() && self.core.scroll == 0;
        let marked = self.marked.clone();
        let focus = self.focus.clone();
        let entity = cx.entity();
        let weak = cx.weak_entity();
        let epoch = self.epoch;
        let background = self.props.background;
        let caret = self.props.caret;
        let grid = Grid {
            face,
            font_size,
            cell,
            line,
            foreground: color(a.foreground),
            background: color(background),
        };
        div()
            .id(("terminal", entity.entity_id()))
            .w_full()
            .h(px(f32::from(rows) * f32::from(line)))
            .overflow_hidden()
            .bg(color(background))
            .track_focus(&focus)
            .key_context("TerminalView")
            .role(gpui_kit::Role::Terminal)
            .aria_label(self.props.accessibility_label.clone())
            .aria_readonly(!self.core.ready())
            .when(window.is_a11y_active(), |this| {
                this.aria_value(source.clone())
            })
            .capture_key_down(cx.listener(Self::key))
            .on_any_mouse_down(cx.listener(Self::mouse_down))
            .on_mouse_up(gpui_kit::MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up(gpui_kit::MouseButton::Middle, cx.listener(Self::mouse_up))
            .on_mouse_up(gpui_kit::MouseButton::Right, cx.listener(Self::mouse_up))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_scroll_wheel(cx.listener(Self::wheel))
            .child(
                canvas(
                    |bounds, _, _| bounds,
                    move |bounds, _, window, cx| {
                        let geometry = Geometry {
                            bounds,
                            cell,
                            line,
                            epoch,
                        };
                        let visible = entity.read(cx).enabled() && entity.read(cx).epoch == epoch;
                        if !visible {
                            return;
                        }
                        window.handle_input(
                            &focus,
                            gpui_kit::ElementInputHandler::new(bounds, entity.clone()),
                            cx,
                        );
                        window.with_content_mask(
                            Some(gpui_kit::ContentMask { bounds }),
                            |window| {
                                for (row, text) in
                                    source.lines().take(usize::from(rows)).enumerate()
                                {
                                    grid.paint(
                                        text,
                                        bounds.origin + point(px(0.), line * row as f32),
                                        window,
                                        cx,
                                    );
                                }
                                if let Some(at) = screen.as_ref().filter(|at| at.visible && cursor)
                                {
                                    let origin = bounds.origin
                                        + point(cell * f32::from(at.x), line * f32::from(at.y));
                                    window.paint_quad(gpui_kit::fill(
                                        Bounds::new(origin, size(px(2.), line)),
                                        color(caret),
                                    ));
                                    if !marked.is_empty() {
                                        grid.paint(&marked, origin, window, cx);
                                    }
                                }
                            },
                        );
                        window.defer(cx, move |window, cx| {
                            let _ =
                                weak.update(cx, |owner, cx| owner.present(geometry, window, cx));
                        });
                    },
                )
                .size_full(),
            )
            .into_any_element()
    }
}
struct Recipe(TerminalViewProps);
impl Widget for Recipe {
    fn create(&self, input: LiveInput, window: &mut Window, cx: &mut App) -> Box<dyn Any> {
        Box::new(Owner::new(&self.0, input, window, cx))
    }
    fn commit(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<Owner>>()
            .unwrap()
            .update(cx, |owner, cx| owner.apply(&self.0, window, cx));
    }
    fn presentation(&self, state: &mut dyn Any, visible: bool, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<Owner>>()
            .unwrap()
            .update(cx, |owner, cx| owner.visibility(visible, window, cx));
    }
    fn deactivate(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<Owner>>()
            .unwrap()
            .update(cx, |owner, cx| {
                owner.alive = false;
                owner.visibility(false, window, cx);
            });
    }
    fn publish_pending(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<Owner>>()
            .unwrap()
            .update(cx, |owner, cx| owner.schedule(window, cx));
    }
    fn render(
        &self,
        state: &dyn Any,
        _: &Slots,
        _: &mut Window,
        _: &mut App,
    ) -> maka_ui::Result<AnyElement> {
        Ok(state
            .downcast_ref::<Entity<Owner>>()
            .unwrap()
            .clone()
            .into_any_element())
    }
}
struct Component;
impl NativeComponent<dyn Widget> for Component {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let output = reduce(call)?;
        let view = NativeView::<dyn Widget>::new(
            Rc::new(Recipe(output.frame.view.clone())),
            BTreeMap::from([("observe".into(), observation_schema())]),
            vec![],
        )?;
        Ok(output.map_view(|_| view))
    }
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register("maka.ui.terminal", |_, _| {
            Ok(BTreeMap::from([(
                "TerminalView".into(),
                NativeExport {
                    contract: terminal_view_contract(),
                    value: NativeCapability::Component(Rc::new(Component)),
                },
            )]))
        })
        .expect("unique native terminal view");
}

struct Grid {
    face: gpui_kit::Font,
    font_size: Pixels,
    cell: Pixels,
    line: Pixels,
    foreground: gpui_kit::Hsla,
    background: gpui_kit::Hsla,
}
impl Grid {
    fn paint(
        &self,
        text: &str,
        origin: gpui_kit::Point<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let clean: String = text.chars().filter(|c| !c.is_control()).collect();
        for (range, column) in cell_spans(&clean) {
            let text = &clean[range];
            let shaped = window.text_system().shape_line(
                text.to_owned().into(),
                self.font_size,
                &[gpui_kit::TextRun {
                    len: text.len(),
                    font: self.face.clone(),
                    color: self.foreground,
                    background_color: Some(self.background),
                    underline: None,
                    strikethrough: None,
                }],
                text.is_ascii().then_some(self.cell),
            );
            let _ = shaped.paint(
                origin + point(self.cell * column as f32, px(0.)),
                self.line,
                gpui_kit::TextAlign::Left,
                None,
                window,
                cx,
            );
        }
    }
}
/// ASCII runs share one shape; wide and combining graphemes retain actual cells.
fn cell_spans(text: &str) -> Vec<(std::ops::Range<usize>, usize)> {
    use unicode_segmentation::UnicodeSegmentation;
    use unicode_width::UnicodeWidthStr;
    let mut spans: Vec<(std::ops::Range<usize>, usize)> = vec![];
    let mut column = 0;
    let mut ascii = false;
    for (byte, grapheme) in text.grapheme_indices(true) {
        let width = grapheme.width();
        if column + width > 240 {
            break;
        }
        if grapheme.is_ascii() && ascii {
            spans.last_mut().unwrap().0.end = byte + grapheme.len();
        } else {
            spans.push((byte..byte + grapheme.len(), column));
        }
        column += width;
        ascii = grapheme.is_ascii();
    }
    spans
}

#[cfg(test)]
mod tests;
