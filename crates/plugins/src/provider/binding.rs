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

use super::{Definition, Error};
use crate::{
    contributions::{Catalog, Contribution},
    fiber::CallGuard,
};

pub use maka_runtime::provider::Identity;

#[derive(Clone)]
pub struct Binding {
    identity: Identity,
    contribution: Contribution<Definition>,
}
impl Binding {
    pub fn resolve(identity: &Identity, catalog: &Catalog) -> Result<Self, Error> {
        identity.validate().map_err(Error::Invalid)?;
        let contribution = catalog
            .snapshot::<Definition>(&identity.scope)
            .entries
            .remove(&identity.name)
            .ok_or(Error::Unavailable)?;
        let binding = Self::new(identity.name.clone(), contribution)?;
        if binding.identity != *identity {
            return Err(Error::Unavailable);
        }
        Ok(binding)
    }

    pub fn new(name: String, contribution: Contribution<Definition>) -> Result<Self, Error> {
        let owner = contribution
            .owner
            .identity()
            .map_err(|_| Error::Unavailable)?;
        let identity = Identity {
            package_id: owner.package_id,
            entry_id: owner.entry_id,
            scope: owner.scope,
            name,
        };
        identity.validate().map_err(Error::Invalid)?;
        Ok(Self {
            identity,
            contribution,
        })
    }
    pub fn definition(&self) -> &Definition {
        &self.contribution.value
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    pub async fn prepare(&self, request: super::Resolve) -> Result<super::Model, Error> {
        let _call = self.admit()?;
        self.definition().prepare(request).await
    }

    pub async fn authorize(
        &self,
        connection: super::Connection,
        credential: Option<super::authentication::Credential>,
        session_id: String,
    ) -> Result<crate::model::Credentials, Error> {
        let _call = self.admit()?;
        self.definition()
            .authorize(connection, credential, session_id)
            .await
    }

    pub async fn authenticate(
        &self,
        request: super::authentication::Authenticate,
        context: super::Context,
    ) -> Result<super::authentication::Credential, Error> {
        self.prepare_authenticate(request, context)?.run().await
    }

    pub fn prepare_authenticate(
        &self,
        request: super::authentication::Authenticate,
        context: super::Context,
    ) -> Result<super::AuthenticationCall, Error> {
        self.definition()
            .validate_authentication(&request, &context)?;
        Ok(super::AuthenticationCall {
            call: self.admit()?,
            implementation: self.definition().implementation.clone(),
            request,
            context,
        })
    }

    pub async fn refresh(
        &self,
        connection: super::Connection,
        credential: super::authentication::Credential,
        context: super::Context,
    ) -> Result<super::authentication::Credential, Error> {
        self.prepare_refresh(connection, credential)?
            .run(context)
            .await
    }

    /// Admit before Host persists a single-use grant claim. Retirement after
    /// this point cannot turn an unstarted callback into an uncertain exchange.
    pub fn prepare_refresh(
        &self,
        connection: super::Connection,
        credential: super::authentication::Credential,
    ) -> Result<RefreshCall, Error> {
        self.definition()
            .validate_configuration(&connection.configuration)?;
        credential.validate().map_err(Error::Invalid)?;
        Ok(RefreshCall {
            call: Some(self.admit()?),
            implementation: self.definition().implementation.clone(),
            connection,
            credential,
        })
    }

    pub fn admit(&self) -> Result<CallGuard, Error> {
        self.contribution.admit().map_err(|_| Error::Unavailable)
    }
    pub fn source(&self) -> Result<maka_runtime::composition::SourceRevision, Error> {
        let owner = self
            .contribution
            .owner
            .identity()
            .map_err(|_| Error::Unavailable)?;
        Ok(maka_runtime::composition::SourceRevision {
            kind: maka_runtime::composition::SourceKind::ModelProvider,
            name: self.identity.name.clone(),
            package_id: owner.package_id,
            entry_id: owner.entry_id,
            activation: owner.activation,
            revision: self.contribution.registration_id().to_string(),
        })
    }
}

/// A single admitted refresh. It owns the provider instance until settlement.
pub struct RefreshCall {
    pub(super) call: Option<CallGuard>,
    pub(super) implementation: std::sync::Arc<dyn super::Provider>,
    pub(super) connection: super::Connection,
    pub(super) credential: super::authentication::Credential,
}

impl RefreshCall {
    pub async fn run(
        self,
        context: super::Context,
    ) -> Result<super::authentication::Credential, Error> {
        let Self {
            call,
            implementation,
            connection,
            credential,
        } = self;
        let _call = call;
        let credential = implementation
            .refresh(connection, credential, context)
            .await?;
        credential.validate().map_err(Error::Invalid)?;
        Ok(credential)
    }
}
