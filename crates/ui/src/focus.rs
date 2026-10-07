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

//! Native one-shot focus on an input's eligibility transition. Clipping does
//! not change eligibility; only actual presentation consumes a pending offer.
//! 原生输入的单次聚焦机会：结构可见与可用性决定资格，视口裁剪不重新触发。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusOnReveal {
    #[default]
    Inactive,
    Pending,
    Presented,
}
impl FocusOnReveal {
    pub fn reconcile(&mut self, eligible: bool) {
        if !eligible {
            *self = Self::Inactive;
        } else if *self == Self::Inactive {
            *self = Self::Pending;
        }
    }
    pub fn pending(&self) -> bool {
        *self == Self::Pending
    }
    pub fn presented(&mut self) -> bool {
        match self {
            Self::Pending => {
                *self = Self::Presented;
                true
            }
            Self::Inactive | Self::Presented => false,
        }
    }
}
