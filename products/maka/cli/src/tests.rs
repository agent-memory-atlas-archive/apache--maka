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

// Both CLI test targets must preserve stack cleanup under the package's
// native linker configuration, including the large V8-bearing binary.
#[test]
fn caught_panic_runs_original_stack_cleanup() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Guard<'a>(&'a AtomicUsize);
    impl Drop for Guard<'_> {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    let drops = AtomicUsize::new(0);
    let result = std::panic::catch_unwind(|| {
        let _guard = Guard(&drops);
        panic!("CLI unwind probe");
    });
    let payload = result.expect_err("the original panic must reach its handler");
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"CLI unwind probe"));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
