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

#[tokio::test]
async fn shared_vm_keeps_eighty_modules_then_reclaims_and_reloads_them() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let vm = Vm::pure(Limits::default()).unwrap();
        for generation in 0..2 {
            let modules = (0..80).map(|index| {
                vm.load(format!("entry-{index}.mjs"), format!(
                    "const identity = [{generation}, {index}]; export function value() {{ return identity; }}"
                )).unwrap()
            }).collect::<Vec<_>>();
            for (index, module) in modules.iter().enumerate() {
                assert_eq!(call(module, "value").await, json!([generation, index]));
            }
            let snapshot = vm.statistics(false).await.unwrap();
            assert_eq!(snapshot.modules, 80);
            assert_eq!(snapshot.pending_calls, 0);
            for module in modules {
                let weak = module.downgrade();
                module.close().await.unwrap();
                assert!(module.call(vec!["value".into()], vec![]).await.is_err());
                drop(module);
                assert!(weak.upgrade().is_none());
            }
            assert_eq!(vm.statistics(true).await.unwrap().modules, 0);
        }
        vm.shutdown().await;
    }).await.expect("eighty small modules drain and reload without sleeps");
}
