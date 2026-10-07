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

//! Physical foreground admission, derived from the original native dispatch tree.
use super::*;
use crate::InspectorElementId;
use std::rc::Weak as RcWeak;

struct Scope {
    focus: FocusId,
    window: WindowId,
    admitted: Cell<bool>,
    interruption: std::rc::Weak<ClickInputState>,
}
struct Gesture {
    input: RcWeak<ClickInputState>,
    interruption: u64,
}
impl Gesture {
    fn is_current(&self) -> bool {
        self.input
            .upgrade()
            .is_some_and(|input| input.interruption.get() == self.interruption)
    }
}
/// One native gesture. Ending it withdraws weak timer and deferred observers.
/// It observes the original window's interruption without retaining the window.
pub struct InputGesture(Rc<Gesture>);
/// A weak observation of one original gesture, never a gesture owner.
#[derive(Clone)]
pub struct WeakInputGesture(RcWeak<Gesture>);
impl std::fmt::Debug for WeakInputGesture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeakInputGesture")
            .field("current", &self.is_current())
            .finish()
    }
}
impl std::fmt::Debug for InputGesture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InputGesture")
            .field("current", &self.is_current())
            .finish()
    }
}
impl PartialEq for WeakInputGesture {
    fn eq(&self, other: &Self) -> bool {
        self.same_gesture(other)
    }
}
impl Eq for WeakInputGesture {}
impl InputGesture {
    /// Whether the original window has left this gesture uninterrupted.
    pub fn is_current(&self) -> bool {
        self.0.is_current()
    }
    /// Observe this gesture without extending its lifetime.
    pub fn downgrade(&self) -> WeakInputGesture {
        WeakInputGesture(Rc::downgrade(&self.0))
    }
}
impl WeakInputGesture {
    /// Whether this same gesture is still owned and its window uninterrupted.
    pub fn is_current(&self) -> bool {
        self.0.upgrade().is_some_and(|gesture| gesture.is_current())
    }
    /// Compare original gesture identity, including after its owner has ended.
    pub fn same_gesture(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}
/// One revocable modal opening. Dropping or revoking it withdraws old physical input.
/// It does not own product open state, native focus, or an editor capability.
pub struct InputScope(Rc<Scope>);
/// A weak observer of an original modal opening, suitable for retained paint.
#[derive(Clone)]
pub struct WeakInputScope(RcWeak<Scope>);
impl InputScope {
    /// Withdraw this opening before any repaint or replacement.
    pub fn revoke(&self) {
        if self.0.admitted.replace(false) {
            if let Some(input) = self.0.interruption.upgrade() {
                input.interrupt();
            }
        }
    }
    /// Wrap the foreground without transferring layout, focus, or ownership.
    pub fn element(&self, element: impl IntoElement) -> impl IntoElement {
        ScopedInput {
            child: element.into_any_element(),
            scope: self.downgrade(),
        }
    }
    /// Observe this opening without extending its owner lifetime.
    pub fn downgrade(&self) -> WeakInputScope {
        WeakInputScope(Rc::downgrade(&self.0))
    }
}
impl Drop for InputScope {
    fn drop(&mut self) {
        self.revoke();
    }
}
impl WeakInputScope {
    pub(super) fn admitted(&self, window: &Window) -> bool {
        self.0
            .upgrade()
            .is_some_and(|scope| scope.admitted.get() && scope.window == window.handle.window_id())
    }
    pub(super) fn focus(&self, window: &Window) -> Option<FocusId> {
        self.0
            .upgrade()
            .filter(|scope| scope.admitted.get() && scope.window == window.handle.window_id())
            .map(|scope| scope.focus)
    }
}
impl Window {
    /// Begin one mechanical gesture against the original window interruption.
    pub fn input_gesture(&self) -> InputGesture {
        InputGesture(Rc::new(Gesture {
            input: Rc::downgrade(&self.click_input),
            interruption: self.input_interruption(),
        }))
    }
    pub(super) fn focus_move_is_admitted(&self, focus: &FocusHandle) -> bool {
        let scope = self
            .input_scope
            .as_ref()
            .filter(|scope| scope.admitted(self));
        if scope.is_some_and(|scope| scope.focus(self) == Some(focus.id)) {
            return true;
        }
        let tree = if !self.invalidator.not_drawing()
            && self
                .next_frame
                .dispatch_tree
                .focusable_node_id(focus.id)
                .is_some()
        {
            &self.next_frame.dispatch_tree
        } else {
            &self.rendered_frame.dispatch_tree
        };
        if tree
            .focusable_node_id(focus.id)
            .is_some_and(|node| !self.node_scopes_are_admitted(tree, node))
        {
            return false;
        }
        scope.is_none_or(|scope| {
            scope
                .focus(self)
                .is_some_and(|parent| tree.focus_contains(parent, focus.id))
        })
    }
    /// Native interruption epoch for an original press/drag. Scope changes and window interruption invalidate it.
    pub fn input_interruption(&self) -> u64 {
        self.click_input.interruption.get()
    }
    pub(crate) fn input_origin_is_presented(
        &self,
        origin: &std::rc::Weak<crate::DispatchOrigin>,
    ) -> bool {
        let tree = &self.next_frame.dispatch_tree;
        let Some(node) = tree.origin_node(origin) else {
            return false;
        };
        self.node_scopes_are_admitted(tree, node)
            && self
                .next_frame
                .input_scopes
                .iter()
                .rev()
                .find(|scope| scope.admitted(self))
                .is_none_or(|scope| {
                    scope
                        .focus(self)
                        .is_some_and(|focus| tree.focus_contains_node(focus, node))
                })
    }
    pub(super) fn focus_is_presented(&self, focus: FocusId) -> bool {
        let tree = &self.next_frame.dispatch_tree;
        let Some(node) = tree.focusable_node_id(focus) else {
            return false;
        };
        let scope = self
            .next_frame
            .input_scopes
            .iter()
            .rev()
            .find(|scope| scope.admitted(self))
            .or_else(|| {
                self.input_scope
                    .as_ref()
                    .filter(|scope| scope.admitted(self))
            });
        self.node_scopes_are_admitted(tree, node)
            && scope.is_none_or(|scope| {
                scope
                    .focus(self)
                    .is_some_and(|parent| tree.focus_contains(parent, focus))
            })
    }

    /// Begin an exclusive physical input opening at the original focus group.
    /// Old input is blocked immediately, before that group's first paint.
    pub fn input_scope(&mut self, focus: &FocusHandle, cx: &mut App) -> InputScope {
        self.click_input.interrupt();
        self.clear_pending_keystrokes(cx);
        cx.stop_active_drag(self);
        self.captured_hitbox = None;
        self.long_press_timer = None;
        self.long_press_capture = None;
        self.touch_gestures.interrupt();
        let scope = InputScope(Rc::new(Scope {
            focus: focus.id,
            window: self.handle.window_id(),
            admitted: Cell::new(true),
            interruption: Rc::downgrade(&self.click_input),
        }));
        self.input_scope = Some(scope.downgrade());
        self.refresh();
        scope
    }
    /// Publish a live opening before painting its foreground. Retained paint replays this declaration.
    pub fn declare_input_scope(&mut self, scope: &WeakInputScope) {
        self.invalidator.debug_assert_paint();
        if scope.admitted(self) {
            self.next_frame.input_scopes.push(scope.clone());
        }
    }
    /// Admission for creation-time native interceptors with an original focus owner.
    pub fn input_focus_is_admitted(&self, focus: &FocusHandle) -> bool {
        self.focus_is_admitted_in(&self.rendered_frame.dispatch_tree, focus.id)
    }
    pub(super) fn focus_is_admitted_in(&self, tree: &DispatchTree, focus: FocusId) -> bool {
        let own_scope = self
            .input_scope
            .as_ref()
            .and_then(|scope| scope.focus(self));
        if own_scope == Some(focus) {
            return true;
        }
        if let Some(node) = tree.focusable_node_id(focus) {
            if !self.node_scopes_are_admitted(tree, node) {
                return false;
            }
        }
        self.input_scope
            .as_ref()
            .is_none_or(|_| own_scope.is_some_and(|parent| tree.focus_contains(parent, focus)))
    }
    pub(crate) fn input_origin_is_admitted(
        &self,
        origin: &std::rc::Weak<crate::DispatchOrigin>,
    ) -> bool {
        let tree = &self.rendered_frame.dispatch_tree;
        let Some(node) = tree.origin_node(origin) else {
            return false;
        };
        self.node_scopes_are_admitted(tree, node)
            && self.input_scope.as_ref().is_none_or(|scope| {
                scope
                    .focus(self)
                    .is_some_and(|focus| tree.focus_contains_node(focus, node))
            })
    }
    pub(crate) fn input_handler_is_admitted(
        &self,
        focus: FocusId,
        origin: &std::rc::Weak<crate::DispatchOrigin>,
    ) -> bool {
        self.focus == Some(focus) && self.input_origin_is_admitted(origin)
    }
    pub(super) fn input_dispatch_path(
        &self,
        node: DispatchNodeId,
    ) -> Option<SmallVec<[DispatchNodeId; 32]>> {
        let tree = &self.rendered_frame.dispatch_tree;
        if !self.node_scopes_are_admitted(tree, node) {
            return None;
        }
        let mut path = tree.dispatch_path(node);
        if let Some(scope) = &self.input_scope {
            let focus = scope.focus(self)?;
            let scope = tree.focusable_node_id(focus)?;
            let start = path.iter().position(|node| *node == scope)?;
            path.drain(..start);
        }
        Some(path)
    }
    fn node_scopes_are_admitted(&self, tree: &DispatchTree, node: DispatchNodeId) -> bool {
        let mut node = Some(node);
        while let Some(current) = node {
            let current = tree.node(current);
            if current
                .input_scope
                .as_ref()
                .is_some_and(|scope| !scope.admitted(self))
            {
                return false;
            }
            node = current.parent;
        }
        true
    }
    pub(super) fn scoped_tab_stop(&self, forward: bool) -> Option<FocusHandle> {
        let stops = &self.rendered_frame.tab_stops;
        let mut current = self.focus;
        for _ in 0..stops.tab_stop_count() {
            let next = if forward {
                stops.next(current.as_ref())
            } else {
                stops.prev(current.as_ref())
            }?;
            if self.input_focus_is_admitted(&next) {
                return Some(next);
            }
            current = Some(next.id);
        }
        None
    }
}

struct ScopedInput {
    child: AnyElement,
    scope: WeakInputScope,
}
impl IntoElement for ScopedInput {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for ScopedInput {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
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
    ) {
        let node = window.next_frame.dispatch_tree.active_node_id().unwrap();
        window.next_frame.dispatch_tree.nodes[node.0].input_scope = Some(self.scope.clone());
        self.child.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        window.declare_input_scope(&self.scope);
        self.child.paint(window, cx);
    }
}
