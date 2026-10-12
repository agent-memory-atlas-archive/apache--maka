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

use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    Error,
    composition::{self, Configuration, Input, InstanceId, Node, Override, PackageUse, Target},
    contract::{Availability, Export, Import, Interface, Transport},
    package::{EntryDeclaration, EntryTarget, PackageRef, PackageSelection, Packages},
};

/// Remote declarations are observations supplied by the execution adapter.
/// They establish compatibility, never caller authority or runtime readiness.
pub type RemoteExports = BTreeMap<RemoteCapability, Export>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteCapability {
    pub deployment: String,
    pub instance: InstanceId,
    pub export: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Provider {
    Local {
        instance: InstanceId,
        export: String,
    },
    Remote {
        capability: RemoteCapability,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Binding {
    pub provider: Provider,
    pub declaration: Export,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub identity: InstanceId,
    pub path: Vec<String>,
    pub parent: Option<InstanceId>,
    pub package: PackageRef,
    pub disabled: bool,
    pub configuration: Value,
    pub entry: Option<EntryDeclaration>,
    pub bindings: BTreeMap<String, Vec<Binding>>,
    pub exports: BTreeMap<String, Binding>,
}

/// A checked projection; callers retain Input as the sole editable authority.
/// Static checking says nothing about effect authorization or live exports.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    instances: BTreeMap<InstanceId, Instance>,
    launch: Option<Binding>,
    recovery: Option<Binding>,
}

impl Plan {
    pub fn prepare(
        input: &Input,
        packages: &Packages,
        remote: &RemoteExports,
    ) -> Result<Self, Error> {
        let target = input.environment.entry_target()?;
        composition::validate_nodes(&input.instances)?;
        for root in [&input.launch, &input.recovery].into_iter().flatten() {
            root.validate()?;
        }
        let mut builder = Builder {
            packages,
            target,
            nodes: BTreeMap::new(),
            aliases: BTreeMap::new(),
        };
        builder.expand(&input.instances, &[], None, &BTreeMap::new(), &[], false)?;
        let resolver = Resolver {
            nodes: &builder.nodes,
            aliases: &builder.aliases,
            remote,
        };
        let mut instances = BTreeMap::new();
        for (id, pending) in &builder.nodes {
            let mut bindings = BTreeMap::new();
            let mut exports = BTreeMap::new();
            if !pending.instance.disabled {
                for name in pending.node.bindings.keys() {
                    if !pending.imports.contains_key(name) {
                        return Err(Error::Composition(format!(
                            "undeclared import at {:?}: {name}",
                            pending.instance.path
                        )));
                    }
                }
                for name in pending.imports.keys() {
                    bindings.insert(
                        name.clone(),
                        resolver.binding(id, name, &mut BTreeSet::new())?,
                    );
                }
                for name in pending.exports.keys() {
                    exports.insert(
                        name.clone(),
                        resolver.export(id, name, &mut BTreeSet::new())?,
                    );
                }
            }
            let mut instance = pending.instance.clone();
            instance.bindings = bindings;
            instance.exports = exports;
            instances.insert(id.clone(), instance);
        }
        let resolve_root = |target: &composition::Target| {
            let bindings = resolver.target(target, &[], None, &mut BTreeSet::new())?;
            let [binding] = bindings.as_slice() else {
                return Err(Error::Composition("root must select one component".into()));
            };
            if !matches!(
                binding.declaration.contract.interface,
                Interface::Component { .. }
            ) || binding.declaration.availability != Availability::Required
            {
                return Err(Error::Contract(
                    "root requires an available component declaration".into(),
                ));
            }
            Ok(binding.clone())
        };
        let launch = input.launch.as_ref().map(resolve_root).transpose()?;
        let recovery = input.recovery.as_ref().map(resolve_root).transpose()?;
        validate_dependencies(&instances)?;
        Ok(Self {
            instances,
            launch,
            recovery,
        })
    }

    pub fn instances(&self) -> &BTreeMap<InstanceId, Instance> {
        &self.instances
    }

    pub fn launch(&self) -> Option<&Binding> {
        self.launch.as_ref()
    }

    pub fn recovery(&self) -> Option<&Binding> {
        self.recovery.as_ref()
    }

    pub fn root(&self, kind: composition::RootKind) -> Option<&Binding> {
        match kind {
            composition::RootKind::Launch => self.launch(),
            composition::RootKind::Recovery => self.recovery(),
        }
    }
}

enum PendingExport {
    Actual(Export),
    Forward(Target),
}

struct Pending {
    instance: Instance,
    node: Node,
    imports: BTreeMap<String, Import>,
    exports: BTreeMap<String, PendingExport>,
}

struct Builder<'a> {
    packages: &'a Packages,
    target: EntryTarget,
    nodes: BTreeMap<InstanceId, Pending>,
    aliases: BTreeMap<Vec<String>, InstanceId>,
}

impl Builder<'_> {
    fn expand(
        &mut self,
        nodes: &BTreeMap<String, Node>,
        namespace: &[String],
        parent: Option<&InstanceId>,
        parameters: &BTreeMap<String, Value>,
        overrides: &[Override],
        ancestor_disabled: bool,
    ) -> Result<(), Error> {
        composition::validate_nodes(nodes)?;
        for patch in overrides {
            if !nodes.contains_key(&patch.path[0]) {
                return Err(Error::Composition(format!(
                    "override target disappeared: {:?}",
                    patch.path
                )));
            }
        }
        for (alias, original) in nodes {
            if self.nodes.len() >= 4096 || namespace.len() >= 32 {
                return Err(Error::Composition(
                    "composition exceeds expansion limits".into(),
                ));
            }
            let mut node = original.clone();
            let mut child_overrides = BTreeMap::new();
            for patch in &node.overrides {
                child_overrides.insert(patch.path.clone(), patch.clone());
            }
            for patch in overrides.iter().filter(|patch| patch.path[0] == *alias) {
                if patch.path.len() == 1 {
                    if let Some(package) = &patch.package {
                        node.package = package.clone().into();
                    }
                    if let Some(use_package) = patch.use_package {
                        node.use_package = use_package;
                    }
                    for (field, value) in &patch.parameters {
                        node.parameters.parameters.remove(field);
                        node.parameters.values.insert(field.clone(), value.clone());
                    }
                    for (field, value) in &patch.configuration {
                        node.configuration.parameters.remove(field);
                        node.configuration
                            .values
                            .insert(field.clone(), value.clone());
                    }
                    node.bindings.extend(patch.bindings.clone());
                    if let Some(disabled) = patch.disabled {
                        node.disabled = disabled;
                    }
                } else {
                    let mut relative = patch.clone();
                    relative.path.remove(0);
                    let combined =
                        child_overrides
                            .entry(relative.path.clone())
                            .or_insert_with(|| Override {
                                path: relative.path.clone(),
                                configuration: BTreeMap::new(),
                                bindings: BTreeMap::new(),
                                disabled: None,
                                ..Override::default()
                            });
                    combined.configuration.extend(relative.configuration);
                    combined.bindings.extend(relative.bindings);
                    combined.parameters.extend(relative.parameters);
                    if relative.package.is_some() {
                        combined.package = relative.package;
                    }
                    if relative.use_package.is_some() {
                        combined.use_package = relative.use_package;
                    }
                    if relative.disabled.is_some() {
                        combined.disabled = relative.disabled;
                    }
                }
            }
            let child_overrides: Vec<_> = child_overrides.into_values().collect();
            node.validate()?;
            let mut identity = parent.map_or_else(Vec::new, |parent| parent.0.clone());
            identity.push(node.key);
            let identity = InstanceId(identity);
            identity.validate()?;
            let mut path = namespace.to_vec();
            path.push(alias.clone());
            let package = match &node.package {
                PackageSelection::Pinned { package } => package.clone(),
                PackageSelection::Current => parent
                    .map(|parent| self.nodes[parent].instance.package.clone())
                    .ok_or_else(|| {
                        Error::Composition(
                            "deployment roots require a pinned package revision".into(),
                        )
                    })?,
            };
            let declaration = self.packages.get(&package).ok_or_else(|| {
                Error::Composition(format!(
                    "missing pinned package: {}@{}",
                    package.id, package.revision
                ))
            })?;
            let configuration = Value::Object(
                node.configuration
                    .resolve(parameters)?
                    .into_iter()
                    .collect(),
            );
            let disabled = ancestor_disabled || node.disabled;
            let (entry, imports, exports) = match node.use_package {
                PackageUse::Entry => {
                    if !child_overrides.is_empty() || node.parameters != Configuration::default() {
                        return Err(Error::Composition(
                            "executable instance cannot own template parameters or child overrides"
                                .into(),
                        ));
                    }
                    let entry = declaration.entries.get(&self.target).ok_or_else(|| {
                        Error::Composition(format!(
                            "package {} has no {:?} entry",
                            package.id, self.target
                        ))
                    })?;
                    crate::validate_value(&entry.configuration, &configuration)?;
                    let exports = entry
                        .exports
                        .iter()
                        .map(|(name, export)| (name.clone(), PendingExport::Actual(export.clone())))
                        .collect();
                    (Some(entry.clone()), entry.imports.clone(), exports)
                }
                PackageUse::Composition => {
                    if configuration != serde_json::json!({}) {
                        return Err(Error::Composition("structural instances use template parameters, not executable configuration".into()));
                    }
                    let template = declaration.composition.as_ref().ok_or_else(|| {
                        Error::Composition(format!("package {} has no composition", package.id))
                    })?;
                    (
                        None,
                        template.imports.clone(),
                        template
                            .exports
                            .iter()
                            .map(|(name, target)| {
                                (name.clone(), PendingExport::Forward(target.clone()))
                            })
                            .collect(),
                    )
                }
            };
            self.aliases.insert(path.clone(), identity.clone());
            self.nodes.insert(
                identity.clone(),
                Pending {
                    instance: Instance {
                        identity: identity.clone(),
                        path: path.clone(),
                        parent: parent.cloned(),
                        package: package.clone(),
                        disabled,
                        configuration,
                        entry,
                        bindings: BTreeMap::new(),
                        exports: BTreeMap::new(),
                    },
                    node: node.clone(),
                    imports,
                    exports,
                },
            );
            if node.use_package == PackageUse::Composition {
                let template = declaration.composition.as_ref().unwrap();
                let mut supplied = node.parameters.resolve(parameters)?;
                for name in supplied.keys() {
                    if !template.parameters.contains_key(name) {
                        return Err(Error::Composition(format!(
                            "undeclared template parameter: {name}"
                        )));
                    }
                }
                for (name, parameter) in &template.parameters {
                    if !supplied.contains_key(name) {
                        supplied.insert(
                            name.clone(),
                            parameter.default.clone().ok_or_else(|| {
                                Error::Composition(format!("missing template parameter: {name}"))
                            })?,
                        );
                    }
                    crate::validate_value(&parameter.schema, &supplied[name])?;
                }
                self.expand(
                    &template.children,
                    &path,
                    Some(&identity),
                    &supplied,
                    &child_overrides,
                    disabled,
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Resolving {
    Import(InstanceId, String),
    Export(InstanceId, String),
}

struct Resolver<'a> {
    nodes: &'a BTreeMap<InstanceId, Pending>,
    aliases: &'a BTreeMap<Vec<String>, InstanceId>,
    remote: &'a RemoteExports,
}

impl Resolver<'_> {
    fn binding(
        &self,
        instance: &InstanceId,
        name: &str,
        stack: &mut BTreeSet<Resolving>,
    ) -> Result<Vec<Binding>, Error> {
        let key = Resolving::Import(instance.clone(), name.into());
        if stack.len() >= 128 || !stack.insert(key.clone()) {
            return Err(Error::Composition("cyclic import forwarding".into()));
        }
        let pending = &self.nodes[instance];
        if pending.instance.disabled {
            return Err(Error::Composition(
                "binding to a disabled structural instance".into(),
            ));
        }
        let import = pending
            .imports
            .get(name)
            .ok_or_else(|| Error::Composition(format!("undeclared imported binding: {name}")))?;
        let namespace = &pending.instance.path[..pending.instance.path.len() - 1];
        let mut bindings = Vec::new();
        for target in pending.node.bindings.get(name).into_iter().flatten() {
            bindings.extend(self.target(
                target,
                namespace,
                pending.instance.parent.as_ref(),
                stack,
            )?);
            if bindings.len() > 4096 {
                return Err(Error::Composition("expanded binding exceeds limits".into()));
            }
        }
        let unique: BTreeSet<_> = bindings.iter().map(|binding| &binding.provider).collect();
        if unique.len() != bindings.len() {
            return Err(Error::Composition(format!(
                "duplicate provider in binding: {name}"
            )));
        }
        for binding in &bindings {
            if !import.contract.accepts(&binding.declaration.contract) {
                return Err(Error::Contract(format!(
                    "import {name} does not accept its bound export"
                )));
            }
        }
        let guaranteed = bindings
            .iter()
            .filter(|binding| binding.declaration.availability == Availability::Required)
            .count();
        if !import.cardinality.accepts(bindings.len(), guaranteed) {
            return Err(Error::Contract(format!(
                "import cardinality is not satisfied: {name}"
            )));
        }
        stack.remove(&key);
        Ok(bindings)
    }

    fn export(
        &self,
        instance: &InstanceId,
        name: &str,
        stack: &mut BTreeSet<Resolving>,
    ) -> Result<Binding, Error> {
        let key = Resolving::Export(instance.clone(), name.into());
        if stack.len() >= 128 || !stack.insert(key.clone()) {
            return Err(Error::Composition("cyclic export forwarding".into()));
        }
        let pending = &self.nodes[instance];
        if pending.instance.disabled {
            return Err(Error::Composition(
                "binding selects a disabled provider".into(),
            ));
        }
        let binding = match pending
            .exports
            .get(name)
            .ok_or_else(|| Error::Composition(format!("missing export: {name}")))?
        {
            PendingExport::Actual(declaration) => Binding {
                provider: Provider::Local {
                    instance: instance.clone(),
                    export: name.into(),
                },
                declaration: declaration.clone(),
            },
            PendingExport::Forward(target) => {
                let targets = self.target(target, &pending.instance.path, Some(instance), stack)?;
                let [binding] = targets.as_slice() else {
                    return Err(Error::Composition(
                        "structural export must forward exactly one capability".into(),
                    ));
                };
                binding.clone()
            }
        };
        stack.remove(&key);
        Ok(binding)
    }

    fn target(
        &self,
        target: &Target,
        namespace: &[String],
        import_owner: Option<&InstanceId>,
        stack: &mut BTreeSet<Resolving>,
    ) -> Result<Vec<Binding>, Error> {
        match target {
            Target::Instance { path, export } => {
                let mut absolute = namespace.to_vec();
                absolute.extend(path.iter().cloned());
                let instance = self.aliases.get(&absolute).ok_or_else(|| {
                    Error::Composition(format!("missing provider instance: {absolute:?}"))
                })?;
                Ok(vec![self.export(instance, export, stack)?])
            }
            Target::Import { name } => {
                let owner = import_owner.ok_or_else(|| {
                    Error::Composition("root input cannot refer to an enclosing import".into())
                })?;
                self.binding(owner, name, stack)
            }
            Target::Remote {
                deployment,
                instance,
                export,
            } => {
                let capability = RemoteCapability {
                    deployment: deployment.clone(),
                    instance: instance.clone(),
                    export: export.clone(),
                };
                let declaration = self.remote.get(&capability).ok_or_else(|| {
                    Error::Composition("remote capability has no observed declaration".into())
                })?;
                declaration.contract.validate()?;
                if declaration.contract.transport != Transport::Remote {
                    return Err(Error::Contract(
                        "local capability cannot cross deployments".into(),
                    ));
                }
                Ok(vec![Binding {
                    provider: Provider::Remote { capability },
                    declaration: declaration.clone(),
                }])
            }
        }
    }
}

fn validate_dependencies(instances: &BTreeMap<InstanceId, Instance>) -> Result<(), Error> {
    fn visit<'a>(
        id: &'a InstanceId,
        instances: &'a BTreeMap<InstanceId, Instance>,
        path: &mut BTreeSet<&'a InstanceId>,
        done: &mut BTreeSet<&'a InstanceId>,
    ) -> Result<(), Error> {
        if done.contains(id) {
            return Ok(());
        }
        if path.len() >= 4096 || !path.insert(id) {
            return Err(Error::Composition("cyclic capability dependency".into()));
        }
        for binding in instances[id].bindings.values().flatten() {
            if let Provider::Local { instance, .. } = &binding.provider {
                visit(instance, instances, path, done)?;
            }
        }
        path.remove(id);
        done.insert(id);
        Ok(())
    }
    let mut done = BTreeSet::new();
    for id in instances.keys() {
        visit(id, instances, &mut BTreeSet::new(), &mut done)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
