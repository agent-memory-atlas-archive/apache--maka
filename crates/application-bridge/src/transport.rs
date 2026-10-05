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

//! Private, single-invocation channels across independent Application owners.
//! The trusted embedding authenticates each channel before selecting its Peer;
//! this module never treats caller observations or credentials in JSON as trust.
mod catalogue;
mod client;
pub mod ipc;
pub mod native;
mod policy;
pub mod process;
mod server;
mod wire;
pub use catalogue::{Catalogue, Description, Selection};
pub use client::Call;
pub use policy::{Endpoint, Peer, PeerPolicy};
pub use server::Server;
pub use wire::{Reply, Source};
