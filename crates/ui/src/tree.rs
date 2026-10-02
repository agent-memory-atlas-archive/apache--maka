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
use std::{io::Write, time::Instant};

/// Count JSON without allocating a serialized copy. Both adapters have the
/// same call boundary; reject oversized input before schema evaluation/cloning.
pub(super) fn check_call_bytes(value: &impl serde::Serialize) -> Result<()> {
    struct Limit(usize);
    impl Write for Limit {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 += bytes.len();
            if self.0 > 1024 * 1024 {
                return Err(std::io::Error::other("UI call exceeds 1 MiB"));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Limit(0), value).map_err(|error| Error::Invalid(error.to_string()))
}

#[derive(Clone)]
struct Child {
    binding: String,
    member: u32,
    mount: Mount,
}

pub(super) struct Mounted<V: ?Sized> {
    mount: Mount,
    pub frame: Frame<Presentation<V>>,
    props: Value,
    children: BTreeMap<String, Child>,
    bytes: usize,
}

/// A composed input is one transaction, not a sequence of published child
/// frames. Newly created mounts cannot be observed by a renderer until commit.
struct Update<V: ?Sized> {
    changed: BTreeMap<Uuid, Mounted<V>>,
    created: Vec<Mount>,
    removed: Vec<Mount>,
    deadline: Instant,
    bytes: usize,
    touched: usize,
}
impl<V: ?Sized> Update<V> {
    fn new(slice: std::time::Duration) -> Self {
        Self {
            changed: BTreeMap::new(),
            created: Vec::new(),
            removed: Vec::new(),
            deadline: Instant::now() + slice,
            bytes: 0,
            touched: 0,
        }
    }
    fn charge(
        &mut self,
        frame: &Frame<Presentation<V>>,
        props: &Value,
        depth: usize,
    ) -> Result<usize> {
        self.touched += 1;
        if depth > 32 || self.touched > 64 {
            return Err(Error::Invalid(
                "component update exceeds mount/depth limits".into(),
            ));
        }
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
            &(&frame.state, &frame.window, frame.revision, props),
        )
        .map_err(|error| Error::Invalid(error.to_string()))?;
        match &frame.view {
            Presentation::Tree(node) => serde_json::to_writer(&mut counter, node),
            Presentation::Native(view) => serde_json::to_writer(&mut counter, view.schemas()),
        }
        .map_err(|error| Error::Invalid(error.to_string()))?;
        let bytes = counter.0 - self.bytes;
        self.bytes = counter.0;
        Ok(bytes)
    }
}

impl<V: ?Sized + 'static> Application<V> {
    pub fn mount_launch(&mut self, props: Value) -> Result<Mount> {
        let mount = self.runtime.mount_launch()?;
        let mut update = Update::new(self.slice);
        update.created.push(mount.clone());
        let result = (|| {
            self.check_props(&mount, &props)?;
            let frame = self.invoke(
                &mount,
                "mount",
                std::slice::from_ref(&props),
                update.deadline,
            )?;
            Self::check_initial(&frame)?;
            self.validate_surface(&mount, &frame)?;
            self.stage(&mount, props, frame, 0, &mut update)?;
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
    ) -> Result<()> {
        let mut update = Update::new(self.slice);
        let current = self
            .views
            .get(&mount.id())
            .ok_or_else(|| Error::Invalid("mount has no committed frame".into()))?;
        if current.frame.revision != revision {
            return Err(Error::Invalid("stale or undeclared UI event".into()));
        }
        check_call_bytes(&(&current.frame.state, &current.props, key, &event))?;
        current.frame.view.validate_input(key, &event)?;
        let args = [
            current.frame.state.clone(),
            json!({"key":key,"event":event}),
            current.props.clone(),
        ];
        let props = current.props.clone();
        let window = current.frame.window.clone();
        let depth = self.mount_depth(mount)?;
        let result = (|| {
            let next = self.invoke(mount, "event", &args, update.deadline)?;
            Self::check_next(revision, &window, &next)?;
            self.stage(mount, props, next, depth, &mut update)?;
            self.commit(&mut update)
        })();
        if let Err(error) = result {
            self.rollback(&update)?;
            return Err(error);
        }
        Ok(())
    }

    fn check_initial(frame: &Frame<Presentation<V>>) -> Result<()> {
        if frame.revision != 0 {
            return Err(Error::Invalid("initial UI revision must be zero".into()));
        }
        Ok(())
    }
    fn check_next(
        revision: u64,
        window: &Option<Window>,
        frame: &Frame<Presentation<V>>,
    ) -> Result<()> {
        if revision.checked_add(1) != Some(frame.revision) {
            return Err(Error::Invalid("UI revision must advance once".into()));
        }
        if &frame.window != window {
            return Err(Error::Invalid(
                "window configuration is fixed for this mount".into(),
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
    fn provider(&self, mount: &Mount) -> Result<Activation> {
        self.runtime
            .mount_statuses()
            .into_iter()
            .find(|status| status.mount == *mount)
            .map(|status| status.provider)
            .ok_or(maka_plugin_core::Error::Retired.into())
    }
    fn mount_depth(&self, mount: &Mount) -> Result<usize> {
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
    fn mount_chain(&self, mount: &Mount) -> Result<Vec<Mount>> {
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
    fn stage(
        &mut self,
        mount: &Mount,
        props: Value,
        frame: Frame<Presentation<V>>,
        depth: usize,
        update: &mut Update<V>,
    ) -> Result<()> {
        let bytes = update.charge(&frame, &props, depth)?;
        let mut previous = self
            .views
            .get(&mount.id())
            .map(|view| view.children.clone())
            .unwrap_or_default();
        let mut children = BTreeMap::new();
        let mut desired = Vec::new();
        frame.view.components(&mut desired);
        for (key, binding, member, child_props) in desired {
            let old = previous.remove(key);
            let retained = old.as_ref().filter(|child| {
                child.binding == binding
                    && child.member == member
                    && !self.is_retiring(&child.mount)
            });
            let child_mount = if let Some(old) = retained {
                let child = &self.views[&old.mount.id()];
                if child.props != *child_props {
                    let revision = child.frame.revision;
                    let window = child.frame.window.clone();
                    let args = [child.frame.state.clone(), child_props.clone()];
                    self.check_props(&old.mount, child_props)?;
                    let next = self.invoke(&old.mount, "update", &args, update.deadline)?;
                    Self::check_next(revision, &window, &next)?;
                    self.stage(&old.mount, child_props.clone(), next, depth + 1, update)?;
                }
                old.mount.clone()
            } else {
                if let Some(old) = old {
                    update.removed.push(old.mount);
                }
                let provider = self.provider(mount)?;
                let capabilities = self.runtime.bind(&provider, binding)?;
                let capability = capabilities
                    .get(member as usize)
                    .ok_or_else(|| Error::Invalid("component binding member unavailable".into()))?;
                let child = self.runtime.mount_component(capability, Some(mount))?;
                update.created.push(child.clone());
                self.check_props(&child, child_props)?;
                let next = self.invoke(
                    &child,
                    "mount",
                    std::slice::from_ref(child_props),
                    update.deadline,
                )?;
                Self::check_initial(&next)?;
                if next.window.is_some() {
                    return Err(Error::Invalid(
                        "embedded component cannot create a window".into(),
                    ));
                }
                self.stage(&child, child_props.clone(), next, depth + 1, update)?;
                child
            };
            children.insert(
                key.into(),
                Child {
                    binding: binding.into(),
                    member,
                    mount: child_mount,
                },
            );
        }
        update
            .removed
            .extend(previous.into_values().map(|child| child.mount));
        update.changed.insert(
            mount.id(),
            Mounted {
                mount: mount.clone(),
                frame,
                props,
                children,
                bytes,
            },
        );
        Ok(())
    }
    fn commit(&mut self, update: &mut Update<V>) -> Result<()> {
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
        if count + update.changed.len() > 512 || bytes + update.bytes > 32 * 1024 * 1024 {
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
        for mount in &update.removed {
            self.runtime.retire_mount(mount)?;
        }
        self.views.append(&mut update.changed);
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
