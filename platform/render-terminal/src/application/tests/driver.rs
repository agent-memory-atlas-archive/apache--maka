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

// Legacy fixtures explicitly await the original owner before asserting commits.
// Responsiveness and ordering acceptance uses the asynchronous methods directly.
use super::super::*;
impl Presenter {
    fn activation_pending(&self) -> bool {
        self.deployment_status()
            .instances
            .iter()
            .any(|status| status.phase == maka_plugin_core::runtime::Phase::Loading)
    }
    pub(crate) fn settle(&mut self) -> maka_ui_runtime::Result<()> {
        let scheduler = self.application.scheduler().clone();
        scheduler.block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_secs(5),
                std::future::poll_fn(|cx| {
                    self.application.set_waker(cx.waker().clone());
                    self.pump()?;
                    // A closed surface cannot commit a replacement's candidate.
                    // The existing replacement owner must first adopt that root.
                    if self.surface.is_live()
                        && (self.application.ui_pending() || self.activation_pending())
                    {
                        std::task::Poll::Pending
                    } else {
                        std::task::Poll::Ready(Ok(()))
                    }
                }),
            )
            .await
            .expect("original UI preparation did not settle")
        })
    }
    pub(crate) fn input_settled(&mut self, event: Event) -> maka_ui_runtime::Result<()> {
        self.input(event)?;
        if self.surface.is_live() {
            self.settle()
        } else {
            Ok(())
        }
    }
    pub(crate) fn pump_settled(&mut self) -> maka_ui_runtime::Result<()> {
        self.settle()
    }
    pub(crate) fn replace_surface_settled(
        &mut self,
        props: Value,
    ) -> maka_ui_runtime::Result<bool> {
        let ready = self.try_replace_surface(props.clone())?;
        if ready {
            return Ok(true);
        }
        let scheduler = self.application.scheduler().clone();
        scheduler.block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_secs(5),
                std::future::poll_fn(|cx| {
                    self.application.set_waker(cx.waker().clone());
                    self.pump()?;
                    match self.try_replace_surface(props.clone()) {
                        Ok(false) => std::task::Poll::Pending,
                        result => std::task::Poll::Ready(result),
                    }
                }),
            )
            .await
            .expect("original replacement did not prepare")
        })
    }
    pub(crate) fn shutdown_settled(&mut self) -> maka_ui_runtime::Result<()> {
        if !self.can_shutdown() {
            return self.shutdown();
        }
        let scheduler = self.application.scheduler().clone();
        scheduler.block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_secs(5),
                std::future::poll_fn(|cx| {
                    self.application.set_waker(cx.waker().clone());
                    match self.shutdown() {
                        Err(Error::Core(maka_plugin_core::Error::CleanupPending)) => {
                            std::task::Poll::Pending
                        }
                        result => std::task::Poll::Ready(result),
                    }
                }),
            )
            .await
            .expect("original accepted work and modules did not close")
        })
    }
}
