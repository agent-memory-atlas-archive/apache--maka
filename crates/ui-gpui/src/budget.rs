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

//! Presenter-owned resource admission; no image identity or resource registry.
use maka_ui::image::ImageError;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct NativeBudget(Arc<Mutex<Usage>>);
#[derive(Debug)]
struct Usage {
    encoded: usize,
    pixels: usize,
    jobs: usize,
    limits: BudgetLimits,
}
#[derive(Clone, Copy, Debug)]
pub struct BudgetLimits {
    pub encoded_bytes: usize,
    pub pixel_bytes: usize,
    pub decode_jobs: usize,
}
impl Default for BudgetLimits {
    fn default() -> Self {
        Self {
            encoded_bytes: 64 * 1024 * 1024,
            pixel_bytes: 128 * 1024 * 1024,
            decode_jobs: 2,
        }
    }
}
impl Default for NativeBudget {
    fn default() -> Self {
        Self::new(BudgetLimits::default())
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum Kind {
    Encoded,
    Pixels,
    Decode,
}
#[derive(Debug)]
pub(crate) struct Permit {
    budget: NativeBudget,
    kind: Kind,
    bytes: usize,
}
impl NativeBudget {
    pub fn new(limits: BudgetLimits) -> Self {
        Self(Arc::new(Mutex::new(Usage {
            encoded: 0,
            pixels: 0,
            jobs: 0,
            limits,
        })))
    }
    pub(crate) fn reserve(&self, kind: Kind, bytes: usize) -> Result<Permit, ImageError> {
        let mut usage = self.0.lock().expect("native budget lock");
        let (used, limit) = match kind {
            Kind::Encoded => (usage.encoded, usage.limits.encoded_bytes),
            Kind::Pixels => (usage.pixels, usage.limits.pixel_bytes),
            Kind::Decode => (usage.jobs, usage.limits.decode_jobs),
        };
        let next = used
            .checked_add(bytes)
            .filter(|n| *n <= limit)
            .ok_or(ImageError::BudgetExhausted)?;
        match kind {
            Kind::Encoded => usage.encoded = next,
            Kind::Pixels => usage.pixels = next,
            Kind::Decode => usage.jobs = next,
        }
        Ok(Permit {
            budget: self.clone(),
            kind,
            bytes,
        })
    }
    #[cfg(test)]
    pub(crate) fn usage(&self) -> (usize, usize, usize) {
        let u = self.0.lock().unwrap();
        (u.encoded, u.pixels, u.jobs)
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        let mut usage = self.budget.0.lock().expect("native budget lock");
        let value = match self.kind {
            Kind::Encoded => &mut usage.encoded,
            Kind::Pixels => &mut usage.pixels,
            Kind::Decode => &mut usage.jobs,
        };
        *value = value
            .checked_sub(self.bytes)
            .expect("original native budget reservation");
    }
}
