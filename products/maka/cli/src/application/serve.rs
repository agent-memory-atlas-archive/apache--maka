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

//! Compiled product availability for the shared private Host process.
use clap::Args;
use maka_host_runtime::{HostError, application::NativeRegistry};
#[derive(Args)]
pub(crate) struct Serve {}
pub(super) fn native() -> Result<NativeRegistry, HostError> {
    let mut native = NativeRegistry::default();
    maka_product_native::provide(&mut native)?;
    Ok(native)
}
pub(super) async fn run(_: Serve) -> Result<(), HostError> {
    maka_transport::transport::process::serve(
        native()?,
        Some(maka_host_js::factory(Default::default())),
    )
    .await
    .map_err(Into::into)
}
