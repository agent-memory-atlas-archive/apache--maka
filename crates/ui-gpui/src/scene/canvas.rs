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

//! Bounded desktop drawing and mechanical input. Business geometry lives in
//! the plugin's committed scene; native owners retain only focus and gestures.

use super::Style;
use crate::{Input, LiveInput};
use gpui_kit::{
    App, AppContext, Bounds, DispatchPhase, FocusHandle, Hitbox, HitboxBehavior,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Pixels, Point as NativePoint, SharedString, StatefulInteractiveElement, Styled,
    Subscription, Window, WindowId, canvas, div, fill, outline, point, px, rgba, size,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    time::Instant,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    fn validate(self) -> Result<(), String> {
        if [self.x, self.y]
            .into_iter()
            .all(|v| v.is_finite() && (-16000.0..=16000.0).contains(&v))
        {
            Ok(())
        } else {
            Err("invalid canvas position".into())
        }
    }
    fn native(self) -> NativePoint<Pixels> {
        point(px(self.x as f32), px(self.y as f32))
    }
    fn from_native(p: NativePoint<Pixels>) -> Option<Self> {
        let p = Self {
            x: f64::from(f32::from(p.x)),
            y: f64::from(f32::from(p.y)),
        };
        ([p.x, p.y]
            .into_iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.0))
        .then_some(p)
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Rect {
    pub origin: Point,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    fn validate(self) -> Result<(), String> {
        self.origin.validate()?;
        if [self.width, self.height]
            .into_iter()
            .all(|v| v.is_finite() && (0.0..=16000.0).contains(&v))
        {
            Ok(())
        } else {
            Err("invalid canvas extent".into())
        }
    }
    fn native(self, origin: NativePoint<Pixels>) -> Bounds<Pixels> {
        Bounds::new(
            origin + self.origin.native(),
            size(px(self.width as f32), px(self.height as f32)),
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Rect {
        rect: Rect,
        color: u32,
        #[serde(default)]
        radius: f64,
    },
    Line {
        from: Point,
        to: Point,
        color: u32,
        width: f64,
    },
    Text {
        position: Point,
        text: String,
        color: u32,
        size: f64,
        #[serde(default)]
        font: Option<String>,
    },
}
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    PointerDown,
    DragMove,
    Drop,
    KeyDown,
    Activate,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Region {
    pub key: String,
    pub rect: Rect,
    /// Corner radius of mechanical outlines; the hit area remains `rect`.
    #[serde(default)]
    pub radius: f64,
    pub label: String,
    #[serde(default)]
    pub draggable: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub tab_index: i16,
    #[serde(default)]
    pub listen: BTreeSet<EventKind>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Contents {
    pub commands: Vec<Command>,
    pub regions: Vec<Region>,
    pub focus_color: Option<u32>,
    pub drag_color: Option<u32>,
}
impl Contents {
    pub(super) fn preflight(
        source: &Value,
        work: &mut usize,
        deadline: Instant,
    ) -> Result<(), String> {
        let mut text_bytes = 0;
        for (field, limit) in [("commands", 1024), ("regions", 256)] {
            let Some(items) = source.get(field).and_then(Value::as_array) else {
                continue;
            };
            *work += items.len();
            if items.len() > limit || *work > 4096 {
                return Err("canvas work limit".into());
            }
            for item in items {
                if Instant::now() >= deadline {
                    return Err("scene compilation deadline".into());
                }
                for (name, max) in [("text", 4096), ("label", 4096), ("key", 128), ("font", 128)] {
                    if item
                        .get(name)
                        .and_then(Value::as_str)
                        .is_some_and(|s| s.len() > max)
                    {
                        return Err("canvas string limit".into());
                    }
                }
                text_bytes += item.get("text").and_then(Value::as_str).map_or(0, str::len);
                if text_bytes > 64 * 1024 {
                    return Err("canvas text limit".into());
                }
                if item
                    .get("listen")
                    .and_then(Value::as_array)
                    .is_some_and(|s| s.len() > 5)
                {
                    return Err("canvas subscription limit".into());
                }
            }
        }
        Ok(())
    }
    pub(super) fn validate(&self, deadline: Instant) -> Result<(), String> {
        if self.commands.len() > 1024 || self.regions.len() > 256 {
            return Err("canvas work limit".into());
        }
        let mut text_bytes = 0;
        for command in &self.commands {
            if Instant::now() >= deadline {
                return Err("scene compilation deadline".into());
            }
            match command {
                Command::Rect { rect, radius, .. } => {
                    rect.validate()?;
                    if !radius.is_finite() || !(0.0..=8000.0).contains(radius) {
                        return Err("invalid canvas radius".into());
                    }
                }
                Command::Line {
                    from, to, width, ..
                } => {
                    from.validate()?;
                    to.validate()?;
                    if !width.is_finite() || !(0.0..=128.0).contains(width) {
                        return Err("invalid canvas stroke".into());
                    }
                }
                Command::Text {
                    position,
                    text,
                    size,
                    font,
                    ..
                } => {
                    position.validate()?;
                    text_bytes += text.len();
                    if text.len() > 4096
                        || text_bytes > 64 * 1024
                        || text.contains(['\n', '\r'])
                        || !size.is_finite()
                        || !(1.0..=256.0).contains(size)
                        || font.as_ref().is_some_and(|s| s.is_empty() || s.len() > 128)
                    {
                        return Err("invalid canvas text".into());
                    }
                }
            }
        }
        let mut keys = BTreeSet::new();
        for region in &self.regions {
            if Instant::now() >= deadline {
                return Err("scene compilation deadline".into());
            }
            region.rect.validate()?;
            if !region.radius.is_finite() || !(0.0..=8000.0).contains(&region.radius) {
                return Err("invalid canvas region radius".into());
            }
            if region.key.is_empty()
                || region.key.len() > 128
                || !keys.insert(&region.key)
                || region.label.len() > 4096
                || region.tab_index < -1
            {
                return Err("invalid canvas region".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Modifiers {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub platform: bool,
    pub function: bool,
}
impl From<gpui_kit::Modifiers> for Modifiers {
    fn from(m: gpui_kit::Modifiers) -> Self {
        Self {
            control: m.control,
            alt: m.alt,
            shift: m.shift,
            platform: m.platform,
            function: m.function,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CanvasInput {
    PointerDown {
        region: String,
        position: Point,
        gesture: Option<u32>,
    },
    DragMove {
        region: String,
        gesture: u32,
        start: Point,
        position: Point,
        delta: Point,
    },
    Drop {
        region: String,
        gesture: u32,
        start: Point,
        position: Point,
        delta: Point,
        target: Option<String>,
    },
    KeyDown {
        region: String,
        key: String,
        modifiers: Modifiers,
    },
    Activate {
        region: String,
    },
}

struct RegionOwner {
    key: String,
    focus: FocusHandle,
    admitted: Cell<bool>,
    route: Cell<u32>,
    config: RefCell<Region>,
    _focus_out: Subscription,
}
impl Drop for RegionOwner {
    fn drop(&mut self) {
        self.admitted.set(false);
        let _ = self.focus.clone().tab_stop(false).tab_index(-1);
    }
}
#[derive(Clone)]
struct PaintedRegion {
    owner: Rc<RegionOwner>,
    route: u32,
    rect: Rect,
    radius: f64,
    listen: BTreeSet<EventKind>,
    draggable: bool,
    source_admitted: bool,
}
impl PaintedRegion {
    fn admitted(&self) -> bool {
        self.source_admitted && self.owner.admitted.get() && self.route == self.owner.route.get()
    }
    fn outline(&self, bounds: Bounds<Pixels>, color: u32) -> gpui_kit::PaintQuad {
        outline(bounds, rgba(color), gpui_kit::BorderStyle::default()).corner_radii(px(self
            .radius
            .min(self.rect.width / 2.0)
            .min(self.rect.height / 2.0)
            as f32))
    }
}
#[derive(Clone)]
struct Painted {
    bounds: Bounds<Pixels>,
    hitbox: Hitbox,
    regions: Vec<PaintedRegion>,
    input: Input,
}
impl Painted {
    fn hit(&self, position: NativePoint<Pixels>, window: &Window) -> Option<PaintedRegion> {
        if !self.hitbox.is_hovered_at(position, window) {
            return None;
        }
        self.regions
            .iter()
            .rev()
            .find(|r| r.admitted() && r.rect.native(self.bounds.origin).contains(&position))
            .cloned()
    }
    fn position(&self, position: NativePoint<Pixels>) -> Option<Point> {
        Point::from_native(position - self.bounds.origin)
    }
}
#[derive(Clone)]
struct Gesture {
    id: u32,
    interruption: u64,
    region: PaintedRegion,
    start: Point,
    start_window: NativePoint<Pixels>,
    bounds: Bounds<Pixels>,
    position: Point,
    delta: Point,
}
struct Shared {
    key: String,
    window: WindowId,
    live: LiveInput,
    alive: Cell<bool>,
    visible: Cell<bool>,
    sequence: Cell<u32>,
    regions: RefCell<BTreeMap<String, Rc<RegionOwner>>>,
    painted: RefCell<Option<Painted>>,
    gesture: RefCell<Option<Gesture>>,
}
impl Shared {
    fn enabled(&self, window: &Window) -> bool {
        self.alive.get()
            && self.visible.get()
            && self.window == window.window_handle().window_id()
            && self.live.0.borrow().is_some()
    }
    fn current_input(&self) -> Option<Input> {
        self.live.0.borrow().clone()
    }
    fn decision(
        &self,
        input: &Input,
        event: CanvasInput,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<maka_ui::InputDecision> {
        match (input.0)(
            &self.key,
            serde_json::to_value(event).expect("finite canvas input"),
            window,
            cx,
        ) {
            Ok(decision) => {
                if decision.prevent_default {
                    window.prevent_default();
                }
                if decision.stop_propagation {
                    cx.stop_propagation();
                }
                Some(decision)
            }
            Err(error) => {
                eprintln!("canvas input rejected: {error}");
                self.gesture.borrow_mut().take();
                window.prevent_default();
                cx.stop_propagation();
                window.refresh();
                None
            }
        }
    }
    fn down(&self, painted: &Painted, event: &MouseDownEvent, window: &mut Window, cx: &mut App) {
        if event.button != MouseButton::Left || !self.enabled(window) {
            return;
        }
        let Some(region) = painted.hit(event.position, window) else {
            return;
        };
        let Some(position) = painted.position(event.position) else {
            return;
        };
        // Div's automatic focus is a later bubble default. Replace that default
        // only after the synchronous transaction and original admission survive.
        window.prevent_default();
        self.gesture.borrow_mut().take();
        let gesture = region
            .draggable
            .then(|| self.sequence.get().checked_add(1))
            .flatten();
        if let Some(id) = gesture {
            self.sequence.set(id);
        }
        let interruption = window.input_interruption();
        let decision = if region.listen.contains(&EventKind::PointerDown) {
            self.decision(
                &painted.input,
                CanvasInput::PointerDown {
                    region: region.owner.key.clone(),
                    position,
                    gesture,
                },
                window,
                cx,
            )
        } else {
            Some(maka_ui::InputDecision::default())
        };
        if decision.is_some_and(|d| !d.prevent_default)
            && self.enabled(window)
            && region.admitted()
            && interruption == window.input_interruption()
            && window.input_focus_is_admitted(&region.owner.focus)
        {
            region.owner.focus.focus(window, cx);
            if let Some(id) = gesture {
                *self.gesture.borrow_mut() = Some(Gesture {
                    id,
                    interruption,
                    bounds: region.rect.native(painted.bounds.origin),
                    region,
                    start: position,
                    start_window: event.position,
                    position,
                    delta: Point { x: 0.0, y: 0.0 },
                });
            }
            window.refresh();
        }
    }
    fn move_pointer(&self, event: &MouseMoveEvent, window: &mut Window, cx: &mut App) {
        if !self.enabled(window) {
            return;
        }
        if event.pressed_button != Some(MouseButton::Left) {
            if self.gesture.borrow_mut().take().is_some() {
                window.refresh();
            }
            return;
        }
        let Some(gesture) = self.gesture.borrow().clone() else {
            return;
        };
        if gesture.interruption != window.input_interruption() || !gesture.region.admitted() {
            self.gesture.borrow_mut().take();
            return;
        }
        let Some(painted) = self.painted.borrow().clone() else {
            return;
        };
        let Some(position) = painted.position(event.position) else {
            return;
        };
        let Some(delta) = Point::from_native(event.position - gesture.start_window) else {
            return;
        };
        let decision = if gesture.region.listen.contains(&EventKind::DragMove) {
            let Some(input) = self.current_input() else {
                return;
            };
            self.decision(
                &input,
                CanvasInput::DragMove {
                    region: gesture.region.owner.key.clone(),
                    gesture: gesture.id,
                    start: gesture.start,
                    position,
                    delta,
                },
                window,
                cx,
            )
        } else {
            Some(maka_ui::InputDecision::default())
        };
        if decision.is_some_and(|d| !d.prevent_default)
            && self.enabled(window)
            && gesture.region.admitted()
            && gesture.interruption == window.input_interruption()
            && let Some(current) = self.gesture.borrow_mut().as_mut().filter(|g| {
                g.id == gesture.id && Rc::ptr_eq(&g.region.owner, &gesture.region.owner)
            })
        {
            current.position = position;
            current.delta = delta;
            window.refresh();
        }
    }
    fn up(&self, event: &MouseUpEvent, window: &mut Window, cx: &mut App) {
        if event.button != MouseButton::Left {
            return;
        }
        let Some(gesture) = self.gesture.borrow_mut().take() else {
            return;
        };
        window.refresh(); // End the native gesture before any fallible rule.
        if !self.enabled(window)
            || gesture.interruption != window.input_interruption()
            || !gesture.region.admitted()
            || !gesture.region.listen.contains(&EventKind::Drop)
        {
            return;
        }
        let Some(painted) = self.painted.borrow().clone() else {
            return;
        };
        let Some(position) = painted.position(event.position) else {
            return;
        };
        let Some(delta) = Point::from_native(event.position - gesture.start_window) else {
            return;
        };
        let target = painted
            .hit(event.position, window)
            .map(|r| r.owner.key.clone());
        let Some(input) = self.current_input() else {
            return;
        };
        self.decision(
            &input,
            CanvasInput::Drop {
                region: gesture.region.owner.key.clone(),
                gesture: gesture.id,
                start: gesture.start,
                position,
                delta,
                target,
            },
            window,
            cx,
        );
    }
}

// GPUI exposes window activation observation through an Entity Context.
// This bridge retains only a weak mechanical owner and never dispatches plugins.
struct Activation {
    _subscription: Subscription,
}
pub(super) struct State {
    shared: Rc<Shared>,
    keyboard: Option<Subscription>,
    activation: Option<gpui_kit::Entity<Activation>>,
}
impl State {
    pub(super) fn new(
        key: String,
        contents: &Contents,
        live: LiveInput,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let shared = Rc::new(Shared {
            key,
            window: window.window_handle().window_id(),
            live,
            alive: Cell::new(true),
            visible: Cell::new(true),
            sequence: Cell::new(0),
            regions: RefCell::new(BTreeMap::new()),
            painted: RefCell::new(None),
            gesture: RefCell::new(None),
        });
        let weak = Rc::downgrade(&shared);
        let keyboard = cx.intercept_keystrokes(move |event, window, cx| {
            let Some(shared) = weak.upgrade().filter(|s| s.enabled(window)) else {
                return;
            };
            let request = shared
                .regions
                .borrow()
                .values()
                .find(|r| {
                    r.admitted.get()
                        && r.focus.is_focused(window)
                        && window.input_focus_is_admitted(&r.focus)
                })
                .map(|r| {
                    (
                        r.clone(),
                        r.config.borrow().listen.contains(&EventKind::KeyDown),
                    )
                });
            let Some((region, true)) = request else {
                return;
            };
            let Some(input) = shared.current_input() else {
                return;
            };
            if event.keystroke.key.len() > 128 {
                window.prevent_default();
                cx.stop_propagation();
                return;
            }
            shared.decision(
                &input,
                CanvasInput::KeyDown {
                    region: region.key.clone(),
                    key: event.keystroke.key.clone(),
                    modifiers: event.keystroke.modifiers.into(),
                },
                window,
                cx,
            );
        });
        let weak = Rc::downgrade(&shared);
        let activation = cx.new(|cx| Activation {
            _subscription: cx.observe_window_activation(window, move |_, window, _| {
                if !window.is_window_active()
                    && let Some(shared) = weak.upgrade()
                {
                    shared.gesture.borrow_mut().take();
                    window.refresh();
                }
            }),
        });
        let mut state = Self {
            shared,
            keyboard: Some(keyboard),
            activation: Some(activation),
        };
        state.reconcile(contents, window, cx);
        state
    }
    pub(super) fn reconcile(&mut self, contents: &Contents, window: &mut Window, cx: &mut App) {
        let mut old = std::mem::take(&mut *self.shared.regions.borrow_mut());
        let mut current = BTreeMap::new();
        for region in &contents.regions {
            let retained = old.remove(&region.key).filter(|owner| {
                if owner.route.get() == u32::MAX {
                    owner.admitted.set(false);
                    let _ = owner.focus.clone().tab_stop(false).tab_index(-1);
                    if owner.focus.is_focused(window) {
                        window.blur(cx);
                    }
                    false
                } else {
                    true
                }
            });
            let owner = retained.unwrap_or_else(|| {
                let focus = cx.focus_handle();
                let weak = Rc::downgrade(&self.shared);
                let original_focus = focus.clone();
                let subscription = window.on_focus_out(&focus, cx, move |_, window, _| {
                    if let Some(shared) = weak.upgrade() {
                        let clear = shared
                            .gesture
                            .borrow()
                            .as_ref()
                            .is_some_and(|g| g.region.owner.focus == original_focus);
                        if clear {
                            shared.gesture.borrow_mut().take();
                            window.refresh();
                        }
                    }
                });
                Rc::new(RegionOwner {
                    key: region.key.clone(),
                    focus,
                    admitted: Cell::new(false),
                    route: Cell::new(0),
                    config: RefCell::new(region.clone()),
                    _focus_out: subscription,
                })
            });
            let changed = {
                let old = owner.config.borrow();
                old.disabled != region.disabled
                    || old.draggable != region.draggable
                    || old.listen != region.listen
            };
            if changed {
                owner.route.set(owner.route.get() + 1);
            }
            *owner.config.borrow_mut() = region.clone();
            owner
                .admitted
                .set(!region.disabled && self.shared.alive.get() && self.shared.visible.get());
            let _ = owner
                .focus
                .clone()
                .tab_stop(!region.disabled && self.shared.visible.get() && region.tab_index >= 0)
                .tab_index(if region.disabled || !self.shared.visible.get() {
                    -1
                } else {
                    region.tab_index as isize
                });
            if (region.disabled || !self.shared.visible.get()) && owner.focus.is_focused(window) {
                window.blur(cx);
            }
            current.insert(region.key.clone(), owner);
        }
        for owner in old.values() {
            owner.admitted.set(false);
            let _ = owner.focus.clone().tab_stop(false).tab_index(-1);
            if owner.focus.is_focused(window) {
                window.blur(cx);
            }
        }
        *self.shared.regions.borrow_mut() = current;
        if self
            .shared
            .gesture
            .borrow()
            .as_ref()
            .is_some_and(|g| !g.region.admitted())
        {
            self.shared.gesture.borrow_mut().take();
        }
    }
    pub(super) fn presentation(&mut self, visible: bool, window: &mut Window, cx: &mut App) {
        if self.shared.visible.replace(visible) == visible {
            return;
        }
        self.shared.gesture.borrow_mut().take();
        self.shared.painted.borrow_mut().take();
        for owner in self.shared.regions.borrow().values() {
            let route = owner.route.get().checked_add(1);
            if let Some(route) = route {
                owner.route.set(route);
            }
            let region = owner.config.borrow();
            let enabled = visible && !region.disabled && route.is_some() && self.shared.alive.get();
            owner.admitted.set(enabled);
            let _ = owner
                .focus
                .clone()
                .tab_stop(enabled && region.tab_index >= 0)
                .tab_index(if enabled {
                    region.tab_index as isize
                } else {
                    -1
                });
            if !enabled && owner.focus.is_focused(window) {
                window.blur(cx);
            }
        }
    }
    pub(super) fn deactivate(&mut self, window: &mut Window, cx: &mut App) {
        self.shared.alive.set(false);
        self.shared.gesture.borrow_mut().take();
        self.shared.painted.borrow_mut().take();
        self.keyboard.take();
        self.activation.take();
        for owner in self.shared.regions.borrow().values() {
            owner.admitted.set(false);
            let _ = owner.focus.clone().tab_stop(false).tab_index(-1);
            if owner.focus.is_focused(window) {
                window.blur(cx);
            }
        }
    }
    pub(super) fn render(
        &self,
        contents: &Contents,
        style: &Style,
        input: Input,
    ) -> gpui_kit::AnyElement {
        let shared = self.shared.clone();
        let paint_shared = shared.clone();
        let regions = contents
            .regions
            .iter()
            .filter_map(|region| {
                shared
                    .regions
                    .borrow()
                    .get(&region.key)
                    .map(|owner| PaintedRegion {
                        owner: owner.clone(),
                        route: owner.route.get(),
                        rect: region.rect,
                        radius: region.radius,
                        listen: region.listen.clone(),
                        draggable: region.draggable,
                        source_admitted: {
                            let current = owner.config.borrow();
                            !region.disabled
                                && current.disabled == region.disabled
                                && current.draggable == region.draggable
                                && current.listen == region.listen
                        },
                    })
            })
            .collect::<Vec<_>>();
        let commands = contents.commands.clone();
        let focus_color = contents.focus_color;
        let drag_color = contents.drag_color;
        let paint_regions = regions.clone();
        let paint_input = input.clone();
        let mut root = style.apply(
            div()
                .id(SharedString::from(format!("canvas:{}", shared.key)))
                .relative()
                .overflow_hidden()
                .occlude(),
        );
        root = root.child(
            canvas(
                |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                move |bounds, hitbox, window, cx| {
                    let snapshot = Painted {
                        bounds,
                        hitbox,
                        regions: paint_regions,
                        input: paint_input,
                    };
                    // Retired owners still display the last committed recipe.
                    // Admission gates input and mechanical decoration, not paint.
                    let enabled = paint_shared.enabled(window);
                    window.with_content_mask(Some(gpui_kit::ContentMask { bounds }), |window| {
                        paint(&commands, bounds.origin, window, cx);
                        if let Some(color) = focus_color.filter(|_| enabled) {
                            for region in &snapshot.regions {
                                if region.admitted() && region.owner.focus.is_focused(window) {
                                    window.paint_quad(
                                        region.outline(region.rect.native(bounds.origin), color),
                                    );
                                }
                            }
                        }
                        if let (Some(color), Some(gesture)) = (
                            drag_color,
                            paint_shared.gesture.borrow().clone().filter(|gesture| {
                                gesture.interruption == window.input_interruption()
                            }),
                        ) && gesture.region.admitted()
                        {
                            let delta = gesture.delta.native();
                            window.paint_quad(gesture.region.outline(
                                Bounds::new(gesture.bounds.origin + delta, gesture.bounds.size),
                                color,
                            ));
                        }
                    });
                    if !enabled {
                        return;
                    }
                    *paint_shared.painted.borrow_mut() = Some(snapshot.clone());
                    let down_shared = paint_shared.clone();
                    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                        if phase == DispatchPhase::Capture {
                            down_shared.down(&snapshot, event, window, cx);
                        }
                    });
                    let move_shared = paint_shared.clone();
                    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                        if phase == DispatchPhase::Capture {
                            move_shared.move_pointer(event, window, cx);
                        }
                    });
                    let up_shared = paint_shared.clone();
                    window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                        if phase == DispatchPhase::Capture {
                            up_shared.up(event, window, cx);
                        }
                    });
                },
            )
            .absolute()
            .inset_0()
            .size_full(),
        );
        for region in regions {
            let owner = region.owner.clone();
            let enabled =
                owner.admitted.get() && shared.alive.get() && shared.live.0.borrow().is_some();
            let current = owner.config.borrow().clone();
            let focus = owner
                .focus
                .clone()
                .tab_stop(enabled && current.tab_index >= 0)
                .tab_index(if enabled {
                    current.tab_index as isize
                } else {
                    -1
                });
            let focus_owner = owner.clone();
            let activate_owner = owner.clone();
            let alive = shared.clone();
            let activate_alive = shared.clone();
            let route = region.route;
            let activate_input = input.clone();
            let listen = region.listen.contains(&EventKind::Activate);
            root = root.child(
                div()
                    .id(SharedString::from(format!("region:{}", owner.key)))
                    .absolute()
                    .left(px(region.rect.origin.x as f32))
                    .top(px(region.rect.origin.y as f32))
                    .w(px(region.rect.width as f32))
                    .h(px(region.rect.height as f32))
                    .track_focus(&focus)
                    .capture_any_mouse_down(|_, window, _| window.prevent_default())
                    .tab_stop(focus.tab_stop)
                    .tab_index(focus.tab_index)
                    .role(gpui_kit::Role::Button)
                    .aria_label(SharedString::from(current.label))
                    .aria_disabled(!enabled)
                    .on_a11y_action(gpui_kit::AccessibleAction::Focus, move |_, window, cx| {
                        if alive.enabled(window)
                            && focus_owner.admitted.get()
                            && focus_owner.route.get() == route
                        {
                            focus_owner.focus.focus(window, cx);
                        }
                    })
                    .on_a11y_action(gpui_kit::AccessibleAction::Click, move |_, window, cx| {
                        if listen
                            && activate_alive.enabled(window)
                            && activate_owner.admitted.get()
                            && activate_owner.route.get() == route
                        {
                            // AX is not the foreground mouse/key event. Never set
                            // cancellation flags on an unrelated native dispatch.
                            if let Err(error) = (activate_input.0)(
                                &activate_alive.key,
                                serde_json::to_value(CanvasInput::Activate {
                                    region: activate_owner.key.clone(),
                                })
                                .unwrap(),
                                window,
                                cx,
                            ) {
                                eprintln!("canvas activation rejected: {error}");
                            }
                        }
                    }),
            );
        }
        root.into_any_element()
    }
}
impl Drop for State {
    fn drop(&mut self) {
        self.shared.alive.set(false);
        self.shared.gesture.borrow_mut().take();
        self.shared.painted.borrow_mut().take();
        for region in self.shared.regions.borrow().values() {
            region.admitted.set(false);
            let _ = region.focus.clone().tab_stop(false).tab_index(-1);
        }
    }
}
fn paint(commands: &[Command], origin: NativePoint<Pixels>, window: &mut Window, cx: &mut App) {
    for command in commands {
        match command {
            Command::Rect {
                rect,
                color,
                radius,
            } => {
                window.paint_quad(fill(rect.native(origin), rgba(*color)).corner_radii(px(
                    (*radius).min(rect.width / 2.0).min(rect.height / 2.0) as f32,
                )));
            }
            Command::Line {
                from,
                to,
                color,
                width,
            } => {
                if *width == 0.0 {
                    continue;
                }
                let mut path = gpui_kit::PathBuilder::stroke(px(*width as f32));
                path.move_to(origin + from.native());
                path.line_to(origin + to.native());
                if let Ok(path) = path.build() {
                    window.paint_path(path, rgba(*color));
                }
            }
            Command::Text {
                position,
                text,
                color,
                size,
                font,
            } => {
                let font = font
                    .as_ref()
                    .map(|f| gpui_kit::font(SharedString::from(f.clone())))
                    .unwrap_or_else(|| window.text_style().font());
                let run = gpui_kit::TextRun {
                    len: text.len(),
                    font,
                    color: rgba(*color).into(),
                    ..Default::default()
                };
                let line = window.text_system().shape_line(
                    SharedString::from(text.clone()),
                    px(*size as f32),
                    &[run],
                    None,
                );
                let _ = line.paint(
                    origin + position.native(),
                    px(*size as f32 * 1.4),
                    gpui_kit::TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests;
