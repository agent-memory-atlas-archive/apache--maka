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

//! Presentation only; the original native selection owns every menu operation.
use crate::{
    LiveInput, Slots, Widget,
    appearance::{color, font},
};
use gpui_kit::{
    AccessibleAction, AnyElement, App, AppContext, Bounds, ClipboardItem, Context, Element,
    ElementId, Entity, FontWeight, GlobalElementId, HitboxBehavior, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, ParentElement, Pixels, Render, Role, ScrollHandle,
    StatefulInteractiveElement, Styled, Subscription, WeakEntity, Window,
    base::{POPUP_PRIORITY, Placement, Positioner, TextSelection, TextSelectionGroup},
    canvas, deferred, div, px, relative,
};
use maka_ui_runtime::{
    ComponentCall, ComponentOutput, Frame, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView, selection_menu::*,
};
use serde_json::json;
use std::{any::Any, collections::BTreeMap, rc::Rc};

struct Owner {
    props: SelectionMenuProps,
    input: LiveInput,
    visible: bool,
    group: Option<TextSelectionGroup>,
    observation: Option<Subscription>,
    scroll: ScrollHandle,
    #[cfg(test)]
    paints: Rc<std::cell::Cell<usize>>,
    #[cfg(test)]
    renders: usize,
}
impl Owner {
    fn enabled(&self) -> bool {
        self.visible && !self.props.disabled && self.input.current.borrow().is_some()
    }
    fn bind(&mut self, group: Option<TextSelectionGroup>, cx: &mut Context<Self>) {
        let unchanged = match (&self.group, &group) {
            (Some(old), Some(next)) => old.same_owner(next),
            (None, None) => true,
            _ => false,
        };
        if !self.enabled() || unchanged {
            return;
        }
        self.observation = None;
        self.group = group;
        if let Some(group) = &self.group {
            let weak = cx.weak_entity();
            self.observation = group.observe_touch_selection(cx, move |cx| {
                let _ = weak.update(cx, |_, cx| cx.notify());
            });
        }
        cx.notify();
    }
    fn presentation(&mut self, visible: bool, cx: &mut Context<Self>) {
        self.visible = visible;
        if !self.enabled() {
            self.group = None;
            self.observation = None;
        }
        cx.notify();
    }
    fn menu(&self, window: &mut Window, cx: &Context<Self>) -> Option<AnyElement> {
        let group = self.group.as_ref()?;
        if !self.enabled()
            || !group.is_current(window, cx)
            || !TextSelection::group_for_element(window, cx)
                .is_some_and(|painted| painted.same_owner(group))
        {
            return None;
        }
        let selection = group.touch_selection(cx)?;
        if !selection.is_menu_open() {
            return None;
        }
        let anchor = selection.bounds()?;
        let a = &self.props.appearance;
        let content = window.window_bounds().get_bounds().size;
        let margin = px(a.window_margin)
            .min(content.width / 4.)
            .min(content.height / 4.);
        let available = Positioner::available_size(window, margin);
        let width = px(a.width).min(available.width);
        if width <= px(a.padding.left + a.padding.right + 2. * a.border_width)
            || available.height <= px(a.padding.top + a.padding.bottom + 2. * a.border_width)
        {
            return None;
        }
        let mut rows = div()
            .id("selection-menu-rows")
            .flex()
            .flex_col()
            .min_w_0()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll);
        for (index, edit) in [Edit::Copy, Edit::SelectAll].into_iter().enumerate() {
            let label = match edit {
                Edit::Copy => &self.props.labels.copy,
                Edit::SelectAll => &self.props.labels.select_all,
            }
            .clone();
            let click = cx.weak_entity();
            let click_group = group.clone();
            let ax = click.clone();
            let ax_group = group.clone();
            let active_background = color(a.active_background);
            let active_foreground = color(a.active_foreground);
            let p = a.row_padding;
            rows = rows.child(
                div()
                    .id(("selection-menu-row", index))
                    .flex_shrink_0()
                    .w_full()
                    .overflow_hidden()
                    .pt(px(p.top))
                    .pr(px(p.right))
                    .pb(px(p.bottom))
                    .pl(px(p.left))
                    .role(Role::MenuItem)
                    .aria_label(label.clone())
                    .hover(move |style| style.bg(active_background).text_color(active_foreground))
                    .active(move |style| style.bg(active_background).text_color(active_foreground))
                    .on_click(move |_, window, cx| activate(&click, &click_group, edit, window, cx))
                    .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                        activate(&ax, &ax_group, edit, window, cx)
                    })
                    .child(div().w_full().truncate().child(label)),
            );
        }
        let painted_group = group.clone();
        #[cfg(test)]
        let paints = self.paints.clone();
        let p = a.padding;
        let body = div()
            .id("selection-menu")
            .flex()
            .flex_col()
            .relative()
            .w(width)
            .max_h(available.height)
            .overflow_hidden()
            .font(font(&a.text.font))
            .text_size(px(a.text.font.size))
            .line_height(relative(a.text.font.line_height))
            .font_weight(FontWeight(a.text.font.weight as f32))
            .text_color(color(a.text.foreground))
            .bg(color(a.background))
            .border(px(a.border_width))
            .border_color(color(a.border))
            .rounded(px(a.radius))
            .pt(px(p.top))
            .pr(px(p.right))
            .pb(px(p.bottom))
            .pl(px(p.left))
            .role(Role::Menu)
            .aria_label(self.props.labels.title.clone())
            .child(rows)
            .child(
                canvas(
                    |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                    move |_, hitbox, window, cx| {
                        if painted_group.is_current(window, cx) {
                            #[cfg(test)]
                            paints.set(paints.get() + 1);
                            TextSelection::register_touch_ui(&hitbox, window, cx);
                        }
                    },
                )
                .absolute()
                .inset_0(),
            );
        Some(
            deferred(CurrentGroup {
                group: group.clone(),
                child: Positioner::side(anchor)
                    .placement(Placement::Top)
                    .margin(margin)
                    .occlude()
                    .child(body)
                    .into_any_element(),
            })
            .with_priority(POPUP_PRIORITY)
            .into_any_element(),
        )
    }
}
// A deferred surface must still belong to the same physical group when it
// actually prepaints and paints; layout alone cannot admit an old menu.
struct CurrentGroup {
    group: TextSelectionGroup,
    child: AnyElement,
}
impl IntoElement for CurrentGroup {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for CurrentGroup {
    type RequestLayoutState = ();
    type PrepaintState = bool;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        let current = self.group.is_current(window, cx);
        if current {
            self.child.prepaint(window, cx);
        }
        current
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        prepaint: &mut bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        if *prepaint && self.group.is_current(window, cx) {
            self.child.paint(window, cx);
        }
    }
}

impl Render for Owner {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(test)]
        {
            self.renders += 1;
        }
        if !self.enabled() {
            return div().size_0();
        }
        let menu = self.menu(window, cx);
        let owner = cx.weak_entity();
        div().size_0().children(menu).child(canvas(
            |_, _, _| (),
            move |_, _, window, cx| {
                // Acquire only from actual painted original placement, never from
                // create-time focus or a speculative prepaint transaction.
                let group = TextSelection::group_for_element(window, cx);
                let owner = owner.clone();
                window.defer(cx, move |_, cx| {
                    let _ = owner.update(cx, |owner, cx| owner.bind(group, cx));
                });
            },
        ))
    }
}
#[derive(Clone, Copy)]
enum Edit {
    Copy,
    SelectAll,
}
fn activate(
    owner: &WeakEntity<Owner>,
    group: &TextSelectionGroup,
    edit: Edit,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(owner) = owner.upgrade() else {
        return;
    };
    if !owner.read(cx).enabled()
        || !owner
            .read(cx)
            .group
            .as_ref()
            .is_some_and(|current| current.same_owner(group))
        || !group.is_current(window, cx)
        || !group
            .touch_selection(cx)
            .is_some_and(|selection| selection.is_menu_open())
    {
        return;
    }
    match edit {
        Edit::Copy => {
            let interruption = window.input_interruption();
            let Some(text) = group.selected_text(window, cx) else {
                return;
            };
            // Native participant copy callbacks may retire this menu or change
            // the original opening. Read and clipboard/close are one activation.
            if interruption != window.input_interruption()
                || !owner.read(cx).enabled()
                || !owner
                    .read(cx)
                    .group
                    .as_ref()
                    .is_some_and(|current| current.same_owner(group))
                || !group.is_current(window, cx)
            {
                return;
            }
            if !text.is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
            group.close_edit_menu(window, cx);
        }
        Edit::SelectAll => TextSelection::select_all(window, cx),
    }
}
struct Recipe(SelectionMenuProps);
impl Widget for Recipe {
    fn create(&self, input: LiveInput, _: &mut Window, cx: &mut App) -> Box<dyn Any> {
        let props = self.0.clone();
        let keep = input
            .current
            .borrow()
            .as_ref()
            .expect("live selection menu mount")
            .clone();
        Box::new(cx.new(|cx| {
            cx.on_release(move |_, _| drop(keep)).detach();
            Owner {
                props,
                input,
                visible: true,
                group: None,
                observation: None,
                scroll: ScrollHandle::default(),
                #[cfg(test)]
                paints: Default::default(),
                #[cfg(test)]
                renders: 0,
            }
        }))
    }
    fn commit(&self, state: &mut dyn Any, _: &mut Window, cx: &mut App) {
        state
            .downcast_mut::<Entity<Owner>>()
            .expect("selection menu owner")
            .update(cx, |owner, cx| {
                owner.props = self.0.clone();
                owner.presentation(owner.visible, cx);
            });
    }
    fn presentation(&self, state: &mut dyn Any, visible: bool, _: &mut Window, cx: &mut App) {
        state
            .downcast_mut::<Entity<Owner>>()
            .expect("selection menu owner")
            .update(cx, |owner, cx| owner.presentation(visible, cx));
    }
    fn deactivate(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        self.presentation(state, false, window, cx);
    }
    fn render(
        &self,
        state: &dyn Any,
        _: &Slots,
        _: &mut Window,
        _: &mut App,
    ) -> maka_ui_runtime::Result<AnyElement> {
        Ok(state
            .downcast_ref::<Entity<Owner>>()
            .expect("selection menu owner")
            .clone()
            .into_any_element())
    }
}
struct Component;
impl NativeComponent<dyn Widget> for Component {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui_runtime::abi::ValueBindings,
    ) -> maka_ui_runtime::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let (props, revision) = match call {
            ComponentCall::Mount { props } => (props, 0),
            ComponentCall::Update { props, state } => (
                props,
                state["revision"].as_u64().expect("selection menu revision") + 1,
            ),
            ComponentCall::Event { .. } => {
                return Err(maka_ui_runtime::Error::Invalid(
                    "SelectionMenu has no component events".into(),
                ));
            }
        };
        let declaration = props.clone();
        let props: SelectionMenuProps = serde_json::from_value(props.clone())
            .map_err(|e| maka_ui_runtime::Error::Invalid(e.to_string()))?;
        Ok(Frame {
            revision,
            state: json!({"revision":revision}),
            surface: None,
            view: NativeView::<dyn Widget>::new(Rc::new(Recipe(props)), BTreeMap::new(), vec![])?
                .with_declaration(declaration)?,
        }
        .output(vec![]))
    }
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register(maka_ui_runtime::selection_menu::IMPLEMENTATION, |_, _| {
            Ok(BTreeMap::from([(
                "SelectionMenu".into(),
                NativeExport {
                    contract: selection_menu_contract(),
                    value: NativeCapability::Component(Rc::new(Component)),
                },
            )]))
        })
        .expect("unique selection menu implementation");
}

#[cfg(test)]
mod tests;
