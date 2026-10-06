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

use super::*;
use std::{
    collections::{BTreeSet, VecDeque},
    io::Write,
    time::Instant,
};

/// Count JSON without allocating a serialized copy. Both adapters have the
/// same call boundary; reject oversized input before schema evaluation/cloning.
pub(super) fn check_call_bytes(value: &impl serde::Serialize) -> Result<()> {
    call_bytes(value).map(|_| ())
}

pub(super) fn call_bytes(value: &impl serde::Serialize) -> Result<usize> {
    struct Limit(usize);
    impl Write for Limit {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_add(bytes.len())
                .ok_or_else(|| std::io::Error::other("UI byte overflow"))?;
            if self.0 > 1024 * 1024 {
                return Err(std::io::Error::other("UI call exceeds 1 MiB"));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut limit = Limit(0);
    serde_json::to_writer(&mut limit, value).map_err(|error| Error::Invalid(error.to_string()))?;
    Ok(limit.0)
}

#[derive(Clone)]
pub(super) struct Child {
    binding: String,
    member: u32,
    pub mount: Mount,
    on: BTreeMap<String, String>,
}

#[derive(Clone)]
struct Subscription {
    parent: Mount,
    slot: String,
    on: BTreeMap<String, String>,
}
struct Route {
    parent: Mount,
    child: Mount,
    slot: String,
    handler: String,
}
struct Notification {
    route: Route,
    event: Emission,
    input_admission: bool,
}

pub(super) struct Mounted<V: ?Sized> {
    mount: Mount,
    pub frame: Frame<NativeView<V>>,
    pub props: Value,
    pub children: BTreeMap<String, Child>,
    pub(super) bytes: usize,
    pub(super) subscriptions: BTreeMap<String, actions::subscriptions::Demand>,
}

/// A composed input is one transaction, not a sequence of published child
/// frames. Newly created mounts cannot be observed by a renderer until commit.
pub(super) struct Update<V: ?Sized> {
    changed: BTreeMap<Uuid, Mounted<V>>,
    created: Vec<Mount>,
    removed: BTreeMap<Uuid, Mount>,
    events: VecDeque<Notification>,
    emitted: usize,
    pub deadline: Instant,
    bytes: usize,
    touched: usize,
    actions: Vec<actions::Prepared<V>>,
    decision: InputDecision,
}
impl<V: ?Sized> Update<V> {
    fn new(slice: std::time::Duration) -> Self {
        Self {
            changed: BTreeMap::new(),
            created: Vec::new(),
            removed: BTreeMap::new(),
            events: VecDeque::new(),
            emitted: 0,
            deadline: Instant::now() + slice,
            bytes: 0,
            touched: 0,
            actions: Vec::new(),
            decision: InputDecision::default(),
        }
    }
    pub fn step(&mut self, depth: usize) -> Result<()> {
        self.touched += 1;
        if depth > 32 || self.touched > 64 {
            return Err(Error::Invalid(
                "component update exceeds mount/depth limits".into(),
            ));
        }
        Ok(())
    }
    pub fn charge_intent(&mut self, value: &impl serde::Serialize) -> Result<()> {
        // Every intent is already bounded by the per-call output limit.
        let size = serde_json::to_vec(value)
            .map_err(|e| Error::Invalid(e.to_string()))?
            .len();
        self.bytes = self
            .bytes
            .checked_add(size)
            .ok_or_else(|| Error::Invalid("work byte overflow".into()))?;
        if self.bytes > 4 * 1024 * 1024 {
            return Err(Error::Invalid("component work exceeds 4 MiB".into()));
        }
        Ok(())
    }
    fn charge(
        &mut self,
        frame: &Frame<NativeView<V>>,
        props: &Value,
        events: &[Emission],
        subscriptions: &[actions::Action],
    ) -> Result<usize> {
        struct Counter(usize);
        impl Write for Counter {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0 += bytes.len();
                if self.0 > 4 * 1024 * 1024 {
                    return Err(std::io::Error::other("component update exceeds 4 MiB"));
                }
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut counter = Counter(self.bytes);
        serde_json::to_writer(
            &mut counter,
            &(&frame.state, &frame.surface, frame.revision, props),
        )
        .map_err(|error| Error::Invalid(error.to_string()))?;
        serde_json::to_writer(&mut counter, &frame.view.metadata())
            .map_err(|error| Error::Invalid(error.to_string()))?;
        serde_json::to_writer(&mut counter, subscriptions)
            .map_err(|error| Error::Invalid(error.to_string()))?;
        let bytes = counter.0 - self.bytes;
        serde_json::to_writer(&mut counter, events)
            .map_err(|error| Error::Invalid(error.to_string()))?;
        self.bytes = counter.0;
        Ok(bytes)
    }
    fn emit(
        &mut self,
        mount: &Mount,
        subscription: Option<Subscription>,
        events: Vec<Emission>,
        input_admission: bool,
    ) -> Result<()> {
        self.emitted += events.len();
        if self.emitted > 64 {
            return Err(Error::Invalid("composed component emission limit".into()));
        }
        if let Some(subscription) = subscription {
            for event in events {
                if let Some(handler) = subscription.on.get(&event.name) {
                    self.events.push_back(Notification {
                        route: Route {
                            parent: subscription.parent.clone(),
                            child: mount.clone(),
                            slot: subscription.slot.clone(),
                            handler: handler.clone(),
                        },
                        event,
                        input_admission,
                    });
                }
            }
        }
        Ok(())
    }
}

impl<V: ?Sized + 'static> Application<V> {
    pub fn mount_launch(&mut self, props: Value) -> Result<Mount> {
        let mount = self.runtime.mount_launch()?;
        let mut update = Update::new(self.slice);
        update.created.push(mount.clone());
        let result = (|| {
            self.check_props(&mount, &props)?;
            update.step(0)?;
            let invocation = self.invoke(
                &mount,
                "mount",
                std::slice::from_ref(&props),
                update.deadline,
                false,
            )?;
            Self::check_initial(&invocation.output.frame)?;
            self.validate_surface(&mount, &invocation.output.frame)?;
            self.stage(&mount, props, invocation, None, 0, &mut update)?;
            self.deliver(&mut update)?;
            self.commit(&mut update)
        })();
        if let Err(error) = result {
            self.rollback(&update)?;
            return Err(error);
        }
        Ok(mount)
    }

    /// A child receives input against its own revision and state. A parent input
    /// may stage props updates/new children, but cannot partially publish them.
    pub fn dispatch(
        &mut self,
        mount: &Mount,
        revision: u64,
        key: &str,
        event: Value,
    ) -> Result<InputDecision> {
        let current = self
            .views
            .get(&mount.id())
            .ok_or_else(|| Error::Invalid("mount has no committed frame".into()))?;
        if current.frame.revision != revision {
            return Err(Error::Invalid("stale or undeclared UI event".into()));
        }
        check_call_bytes(&(&current.frame.state, &current.props, key, &event))?;
        current.frame.view.validate_input(key, &event)?;
        self.dispatch_origin(
            mount,
            super::abi::ComponentInput::Raw {
                key: key.into(),
                event,
            },
        )
    }
    pub(super) fn dispatch_action_result(
        &mut self,
        mount: &Mount,
        key: &str,
        event: Value,
    ) -> Result<()> {
        self.dispatch_origin(
            mount,
            super::abi::ComponentInput::Action {
                key: key.into(),
                event,
            },
        )
        .map(|_| ())
    }
    fn dispatch_origin(
        &mut self,
        mount: &Mount,
        event: super::abi::ComponentInput,
    ) -> Result<InputDecision> {
        let event =
            serde_json::to_value(event).map_err(|error| Error::Invalid(error.to_string()))?;
        let mut update = Update::new(self.slice);
        let current = self
            .views
            .get(&mount.id())
            .ok_or_else(|| Error::Invalid("mount has no committed frame".into()))?;
        check_call_bytes(&(&current.frame.state, &current.props, &event))?;
        let args = [current.frame.state.clone(), event, current.props.clone()];
        let props = current.props.clone();
        let revision = current.frame.revision;
        let surface = current.frame.surface.clone();
        let depth = self.mount_depth(mount)?;
        let subscription = self.subscription(mount, &update)?;
        let result = (|| {
            update.step(depth)?;
            let next = self.invoke(mount, "event", &args, update.deadline, false)?;
            Self::check_next(revision, &surface, &next.output.frame)?;
            self.stage(mount, props, next, subscription, depth, &mut update)?;
            self.deliver(&mut update)?;
            self.commit(&mut update)?;
            Ok(update.decision)
        })();
        match result {
            Ok(decision) => Ok(decision),
            Err(error) => {
                self.rollback(&update)?;
                Err(error)
            }
        }
    }

    fn check_initial(frame: &Frame<NativeView<V>>) -> Result<()> {
        if frame.revision != 0 {
            return Err(Error::Invalid("initial UI revision must be zero".into()));
        }
        Ok(())
    }
    fn check_next(
        revision: u64,
        surface: &Option<Surface>,
        frame: &Frame<NativeView<V>>,
    ) -> Result<()> {
        if revision.checked_add(1) != Some(frame.revision) {
            return Err(Error::Invalid("UI revision must advance once".into()));
        }
        if &frame.surface != surface {
            return Err(Error::Invalid(
                "surface configuration is fixed for this mount".into(),
            ));
        }
        Ok(())
    }
    fn check_props(&mut self, mount: &Mount, value: &Value) -> Result<()> {
        check_call_bytes(value)?;
        let execution = self.runtime.admit_mount(mount)?;
        let Export::Component { props, .. } = execution.value() else {
            unreachable!()
        };
        let result = props
            .validate(value)
            .map_err(|error| Error::Invalid(error.to_string()));
        self.runtime.settle(&execution, Ok(()))?;
        result
    }
    pub(super) fn provider(&self, mount: &Mount) -> Result<Activation> {
        self.runtime
            .mount_statuses()
            .into_iter()
            .find(|status| status.mount == *mount)
            .map(|status| status.provider)
            .ok_or(maka_plugin_core::Error::Retired.into())
    }
    pub(super) fn mount_depth(&self, mount: &Mount) -> Result<usize> {
        Ok(self.mount_chain(mount)?.len() - 1)
    }
    /// The owning root comes from the mount parent chain, not display reachability.
    /// Detached mounts remain attributed to their original native surface.
    pub fn mount_root(&self, mount: &Mount) -> Result<Mount> {
        Ok(self
            .mount_chain(mount)?
            .pop()
            .expect("nonempty mount chain"))
    }
    pub(super) fn mount_chain(&self, mount: &Mount) -> Result<Vec<Mount>> {
        let parents: BTreeMap<_, _> = self
            .runtime
            .mount_statuses()
            .into_iter()
            .map(|status| (status.mount.id(), (status.mount, status.parent)))
            .collect();
        let mut current = mount;
        let mut chain = vec![mount.clone()];
        loop {
            match parents.get(&current.id()) {
                Some((owner, Some(parent))) if owner == current => {
                    chain.push(parent.clone());
                    current = parent;
                }
                Some((owner, None)) if owner == current => return Ok(chain),
                _ => return Err(maka_plugin_core::Error::Retired.into()),
            }
        }
    }
    pub(super) fn current<'a>(
        &'a self,
        mount: &Mount,
        update: &'a Update<V>,
    ) -> Result<&'a Mounted<V>> {
        update
            .changed
            .get(&mount.id())
            .or_else(|| self.views.get(&mount.id()))
            .filter(|view| view.mount == *mount && !update.removed.contains_key(&mount.id()))
            .ok_or_else(|| Error::Invalid("component is absent from the transaction".into()))
    }
    fn subscription(&self, mount: &Mount, update: &Update<V>) -> Result<Option<Subscription>> {
        let status = self
            .runtime
            .mount_statuses()
            .into_iter()
            .find(|status| status.mount == *mount)
            .ok_or(maka_plugin_core::Error::Retired)?;
        let Some(parent) = status.parent else {
            return Ok(None);
        };
        let view = self.current(&parent, update)?;
        Ok(view
            .children
            .iter()
            .find(|(_, child)| child.mount == *mount)
            .map(|(slot, child)| Subscription {
                parent,
                slot: slot.clone(),
                on: child.on.clone(),
            }))
    }
    fn detach(&self, mount: &Mount, update: &mut Update<V>) {
        let statuses = self.runtime.mount_statuses();
        update.removed.insert(mount.id(), mount.clone());
        loop {
            let before = update.removed.len();
            for status in &statuses {
                if status
                    .parent
                    .as_ref()
                    .is_some_and(|parent| update.removed.contains_key(&parent.id()))
                {
                    update
                        .removed
                        .insert(status.mount.id(), status.mount.clone());
                }
            }
            if before == update.removed.len() {
                break;
            }
        }
        update
            .changed
            .retain(|id, _| !update.removed.contains_key(id));
        for action in &mut update.actions {
            action.invalidate(|e| {
                !update.removed.contains_key(&e.parent.id())
                    && !update.removed.contains_key(&e.child.id())
            });
        }
        update.events.retain(|notification| {
            !update.removed.contains_key(&notification.route.parent.id())
                && !update.removed.contains_key(&notification.route.child.id())
        });
    }
    fn check_subscription(&mut self, mount: &Mount, on: &BTreeMap<String, String>) -> Result<()> {
        let execution = self.runtime.admit_mount(mount)?;
        let Export::Component { events, .. } = execution.value() else {
            unreachable!()
        };
        let valid = on.keys().all(|name| events.contains_key(name));
        self.runtime.settle(&execution, Ok(()))?;
        if valid {
            Ok(())
        } else {
            Err(Error::Invalid(
                "subscription to undeclared component event".into(),
            ))
        }
    }
    fn stage(
        &mut self,
        mount: &Mount,
        props: Value,
        invocation: Invocation<V>,
        subscription: Option<Subscription>,
        depth: usize,
        update: &mut Update<V>,
    ) -> Result<()> {
        let Invocation {
            output,
            input_admission,
        } = invocation;
        if let Some(decision) = output.decision {
            // All validated synchronous observers can veto; no observer can
            // undo an earlier veto. This result escapes only after full commit.
            update.decision.prevent_default |= decision.prevent_default;
            update.decision.stop_propagation |= decision.stop_propagation;
        }
        let bytes = update.charge(&output.frame, &props, &output.events, &output.subscriptions)?;
        let previous_subscriptions = self
            .current(mount, update)
            .map(|v| v.subscriptions.clone())
            .unwrap_or_default();
        let declarations = output.subscriptions;
        update.charge_intent(&output.actions)?;
        update.charge_intent(&output.decision)?;
        let actions = output.actions;
        // No one-shot feedback can be confused with a second request or a
        // subscription. Distinct keys permit independent concurrent requests.
        let mut keys = std::collections::BTreeSet::new();
        for action in &actions {
            if !keys.insert(&action.key)
                || declarations.iter().any(|s| s.key == action.key)
                || previous_subscriptions.contains_key(&action.key)
            {
                return Err(Error::Invalid("action feedback key already in use".into()));
            }
        }
        for action in actions.iter().chain(&declarations) {
            if self.actions.key_pending(mount, &action.key)
                || update
                    .actions
                    .iter()
                    .any(|a| !a.subscription && a.consumer == *mount && a.key == action.key)
            {
                return Err(Error::Invalid("action feedback key already in use".into()));
            }
        }
        update.emit(mount, subscription, output.events, input_admission)?;
        let frame = output.frame;
        let mut previous = update
            .changed
            .get(&mount.id())
            .or_else(|| self.views.get(&mount.id()))
            .map(|view| view.children.clone())
            .unwrap_or_default();
        let mut children = BTreeMap::new();
        let mut desired = Vec::new();
        frame.view.components(&mut desired);
        for reference in desired {
            let old = previous.remove(reference.key);
            let retained = old.as_ref().filter(|child| {
                child.binding == reference.binding
                    && child.member == reference.member
                    && !self.is_retiring(&child.mount)
                    && !update.removed.contains_key(&child.mount.id())
            });
            let subscription = Some(Subscription {
                parent: mount.clone(),
                slot: reference.key.into(),
                on: reference.on.clone(),
            });
            let child_mount = if let Some(old) = retained {
                self.check_subscription(&old.mount, reference.on)?;
                let child = self.current(&old.mount, update)?;
                if child.props != *reference.props {
                    let revision = child.frame.revision;
                    let surface = child.frame.surface.clone();
                    let args = [child.frame.state.clone(), reference.props.clone()];
                    self.check_props(&old.mount, reference.props)?;
                    update.step(depth + 1)?;
                    let next = self.invoke(&old.mount, "update", &args, update.deadline, false)?;
                    Self::check_next(revision, &surface, &next.output.frame)?;
                    self.stage(
                        &old.mount,
                        reference.props.clone(),
                        next,
                        subscription,
                        depth + 1,
                        update,
                    )?;
                }
                old.mount.clone()
            } else {
                if let Some(old) = old {
                    self.detach(&old.mount, update);
                }
                let provider = self.provider(mount)?;
                let capabilities = self.runtime.bind(&provider, reference.binding)?;
                let capability = capabilities
                    .get(reference.member as usize)
                    .ok_or_else(|| Error::Invalid("component binding member unavailable".into()))?;
                let child = self.runtime.mount_component(capability, Some(mount))?;
                update.created.push(child.clone());
                self.check_subscription(&child, reference.on)?;
                self.check_props(&child, reference.props)?;
                update.step(depth + 1)?;
                let next = self.invoke(
                    &child,
                    "mount",
                    std::slice::from_ref(reference.props),
                    update.deadline,
                    false,
                )?;
                Self::check_initial(&next.output.frame)?;
                if next.output.frame.surface.is_some() {
                    return Err(Error::Invalid(
                        "embedded component cannot create a surface".into(),
                    ));
                }
                self.stage(
                    &child,
                    reference.props.clone(),
                    next,
                    subscription,
                    depth + 1,
                    update,
                )?;
                child
            };
            children.insert(
                reference.key.into(),
                Child {
                    binding: reference.binding.into(),
                    member: reference.member,
                    mount: child_mount,
                    on: reference.on.clone(),
                },
            );
        }
        for child in previous.into_values() {
            self.detach(&child.mount, update);
        }
        // Revoke queued routes permanently at each edge diff. Later restoring
        // the same subscription cannot resurrect a previously detached event.
        for action in &mut update.actions {
            action.invalidate(|e| {
                !update.removed.contains_key(&e.parent.id())
                    && !update.removed.contains_key(&e.child.id())
            });
        }
        update.events.retain(|notification| {
            notification.route.parent != *mount
                || children.get(&notification.route.slot).is_some_and(|child| {
                    child.mount == notification.route.child
                        && child.on.get(&notification.event.name)
                            == Some(&notification.route.handler)
                })
        });
        for action in &mut update.actions {
            action.invalidate(|e| {
                e.parent != *mount || children.get(&e.slot).is_some_and(|c| c.mount == e.child)
            });
        }
        update.changed.insert(
            mount.id(),
            Mounted {
                mount: mount.clone(),
                frame,
                props,
                children,
                bytes,
                subscriptions: BTreeMap::new(),
            },
        );
        for action in actions {
            if self.actions.len() + update.actions.len() >= 64 {
                return Err(Error::Invalid("pending action limit".into()));
            }
            let prepared = self.prepare_action(mount, action, update, false)?;
            update.actions.push(prepared);
        }
        let mut demands = BTreeMap::new();
        for action in declarations {
            if demands.contains_key(&action.key) {
                return Err(Error::Invalid("duplicate subscription key".into()));
            }
            let demand = match self
                .views
                .get(&mount.id())
                .and_then(|view| view.subscriptions.get(&action.key))
                .filter(|old| old.action == action)
                .or_else(|| {
                    previous_subscriptions
                        .get(&action.key)
                        .filter(|old| old.action == action)
                }) {
                Some(old) => old.clone(),
                None => {
                    if self.actions.len() + update.actions.len() >= 64 {
                        return Err(Error::Invalid("pending action limit".into()));
                    }
                    let prepared = self.prepare_action(mount, action.clone(), update, true)?;
                    let demand = actions::subscriptions::Demand {
                        action: action.clone(),
                        request: prepared.id,
                    };
                    update.actions.push(prepared);
                    demand
                }
            };
            demands.insert(action.key.clone(), demand);
        }
        update
            .changed
            .get_mut(&mount.id())
            .expect("staged owner")
            .subscriptions = demands;

        Ok(())
    }
    fn deliver(&mut self, update: &mut Update<V>) -> Result<()> {
        while let Some(notification) = update.events.pop_front() {
            let route = notification.route;
            let current = self.current(&route.parent, update)?;
            if !current.children.get(&route.slot).is_some_and(|child| {
                child.mount == route.child
                    && child.on.get(&notification.event.name) == Some(&route.handler)
            }) {
                continue;
            }
            let args = [
                current.frame.state.clone(),
                serde_json::to_value(super::abi::ComponentInput::Component {
                    key: route.handler.clone(),
                    child: route.slot.clone(),
                    name: notification.event.name,
                    event: notification.event.value,
                })
                .map_err(|error| Error::Invalid(error.to_string()))?,
                current.props.clone(),
            ];
            let revision = current.frame.revision;
            let props = current.props.clone();
            let surface = current.frame.surface.clone();
            let depth = self.mount_depth(&route.parent)?;
            let subscription = self.subscription(&route.parent, update)?;
            update.step(depth)?;
            let invocation = self.invoke(
                &route.parent,
                "event",
                &args,
                update.deadline,
                notification.input_admission,
            )?;
            Self::check_next(revision, &surface, &invocation.output.frame)?;
            self.stage(
                &route.parent,
                props,
                invocation,
                subscription,
                depth,
                update,
            )?;
        }
        Ok(())
    }
    fn discard_private(&mut self, update: &mut Update<V>) -> Result<()> {
        let private: Vec<_> = update
            .created
            .iter()
            .rev()
            .filter(|mount| update.removed.contains_key(&mount.id()))
            .cloned()
            .collect();
        for mount in private {
            self.runtime.retire_mount(&mount)?;
            // Never published, never exposed to a renderer, no native effects.
            self.runtime.complete_mount(&mount, Ok(()))?;
            update.created.retain(|created| *created != mount);
            update.removed.remove(&mount.id());
        }
        Ok(())
    }
    fn validate_render_depth(&self, update: &Update<V>) -> Result<()> {
        let Some(limit) = self.render_depth_limit else {
            return Ok(());
        };
        // Walk the proposed whole tree, including retained ancestors and children.
        // Removed/retiring roots are absent from admission. Their already
        // validated frames can remain in custody during native cleanup, which
        // may have completed some children before their root.
        // Hidden slots retain custody but draw nothing.
        let admitted: BTreeSet<_> = self
            .runtime
            .mount_statuses()
            .into_iter()
            .filter(|status| status.phase != Phase::Retiring)
            .map(|status| status.mount.id())
            .collect();
        let mut roots = BTreeMap::new();
        for view in self.views.values().chain(update.changed.values()) {
            if view.frame.surface.is_some()
                && admitted.contains(&view.mount.id())
                && !update.removed.contains_key(&view.mount.id())
            {
                roots.insert(view.mount.id(), view.mount.clone());
            }
        }
        let mut pending: Vec<_> = roots.into_values().map(|mount| (mount, 0u16)).collect();
        while let Some((mount, enclosing)) = pending.pop() {
            if Instant::now() >= update.deadline {
                return Err(Error::Invalid(
                    "composed UI update deadline exceeded".into(),
                ));
            }
            let mounted = self.current(&mount, update)?;
            let view = &mounted.frame.view;
            let total = enclosing
                .checked_add(view.render_depth)
                .ok_or_else(|| Error::Invalid("native render depth overflow".into()))?;
            if total > limit {
                return Err(Error::Invalid("composed native render depth limit".into()));
            }
            for slot in view.visible_slots() {
                let cost = view.slot_render_depth.get(&slot.key).copied().unwrap_or(0);
                let child = mounted
                    .children
                    .get(&slot.key)
                    .ok_or_else(|| Error::Invalid("native slot has no proposed child".into()))?;
                pending.push((child.mount.clone(), enclosing + cost));
            }
        }
        Ok(())
    }
    fn commit(&mut self, update: &mut Update<V>) -> Result<()> {
        self.validate_render_depth(update)?;
        self.discard_private(update)?;
        if Instant::now() >= update.deadline {
            return Err(Error::Invalid(
                "composed UI update deadline exceeded".into(),
            ));
        }
        let retained = self
            .views
            .iter()
            .filter(|(id, _)| !update.changed.contains_key(id));
        let (count, bytes) = retained.fold((0, 0), |(count, bytes), (_, view)| {
            (count + 1, bytes + view.bytes)
        });
        let changed_bytes: usize = update.changed.values().map(|view| view.bytes).sum();
        if count + update.changed.len() > 512
            || bytes + changed_bytes + self.value_bytes.values().sum::<usize>() > 32 * 1024 * 1024
        {
            return Err(Error::Invalid(
                "UI state limit reached; confirm retired mount cleanup".into(),
            ));
        }
        // No async/native execution may interleave with this owner-thread commit.
        // Check every changed owner before changing phases or visible state.
        for view in update.changed.values() {
            let execution = self.runtime.admit_mount(&view.mount)?;
            self.runtime.settle(&execution, Ok(()))?;
        }
        // This is the commit decision, after every potentially costly precheck.
        // No expired composed update may change the visible tree or lifetimes.
        if Instant::now() >= update.deadline {
            return Err(Error::Invalid(
                "composed UI update deadline exceeded".into(),
            ));
        }
        for mount in &update.created {
            self.runtime.publish_mount(mount)?;
        }
        for mount in update.removed.values() {
            self.runtime.retire_mount(mount)?;
        }
        self.views.append(&mut update.changed);
        self.invalidate_pending_captures();
        update.actions.retain(|action| {
            !action.subscription
                || self.subscription_wanted(&action.consumer, &action.key, action.id)
        });
        self.actions.queue.retain(|action| {
            !action.subscription
                || self
                    .views
                    .get(&action.consumer.id())
                    .and_then(|view| view.subscriptions.get(&action.key))
                    .is_some_and(|demand| demand.request == action.id)
        });
        self.actions.queue.extend(update.actions.drain(..));
        self.sync_subscription_demand();
        Ok(())
    }
    fn rollback(&mut self, update: &Update<V>) -> Result<()> {
        for mount in update.created.iter().rev() {
            self.runtime.retire_mount(mount)?;
            // These never entered committed UI state, dispatched native effects,
            // or escaped to a renderer. There are no native resources to close.
            self.runtime.complete_mount(mount, Ok(()))?;
        }
        Ok(())
    }
    /// Read the committed child identity. The renderer uses this exact mount,
    /// not a namespace or an implicit global component lookup.
    pub fn child_mount(&self, parent: &Mount, key: &str) -> Result<&Mount> {
        self.views
            .get(&parent.id())
            .filter(|view| view.mount == *parent)
            .and_then(|view| view.children.get(key))
            .map(|child| &child.mount)
            .ok_or_else(|| Error::Invalid("component child is not committed".into()))
    }
    /// Read the exact committed display subtree, including retired components
    /// still displayed pending recovery. Detached mounts are absent.
    pub fn mounted_subtree(&self, root: &Mount) -> Result<Vec<Mount>> {
        let mut mounts = Vec::new();
        let mut pending = vec![root.clone()];
        while let Some(mount) = pending.pop() {
            let view = self
                .views
                .get(&mount.id())
                .filter(|view| view.mount == mount)
                .ok_or_else(|| Error::Invalid("component subtree is not committed".into()))?;
            pending.extend(view.children.values().map(|child| child.mount.clone()));
            mounts.push(mount);
        }
        Ok(mounts)
    }
    /// Removed/faulted mounts remain fenced until the renderer supplies real
    /// resource close acknowledgements. Leaves precede parents for settlement.
    pub fn retiring_mounts(&self) -> Vec<Mount> {
        let statuses = self.runtime.mount_statuses();
        let parents: BTreeMap<_, _> = statuses
            .iter()
            .map(|status| (status.mount.id(), status.parent.as_ref().map(Mount::id)))
            .collect();
        let mut mounts: Vec<_> = statuses
            .into_iter()
            .filter(|status| status.phase == Phase::Retiring)
            .map(|status| status.mount)
            .collect();
        mounts.sort_by_key(|mount| {
            let mut depth = 0;
            let mut id = mount.id();
            while let Some(Some(parent)) = parents.get(&id) {
                depth += 1;
                id = *parent;
            }
            std::cmp::Reverse(depth)
        });
        mounts
    }
}
