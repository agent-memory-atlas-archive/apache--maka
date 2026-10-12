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

use std::{os::unix::fs::PermissionsExt, process::Command};

#[test]
fn desktop_executes_the_fixed_sibling_with_original_arguments_and_exit_status() {
    let temporary = tempfile::tempdir().unwrap();
    let executable = temporary.path().join("maka");
    std::fs::copy(env!("CARGO_BIN_EXE_maka"), &executable).unwrap();
    let desktop = temporary.path().join("maka-desktop");
    std::fs::write(
        &desktop,
        "#!/bin/sh\nprintf '%s\\n' \"$$\" \"$@\"\nexit 73\n",
    )
    .unwrap();
    std::fs::set_permissions(&desktop, std::fs::Permissions::from_mode(0o700)).unwrap();
    let arguments = [
        "--root",
        "a root 中文",
        "--deployment",
        "--help",
        "--",
        "trailing",
    ];
    let child = Command::new(&executable)
        .arg("desktop")
        .args(arguments)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let pid = child.id();
    let result = child.wait_with_output().unwrap();
    assert_eq!(result.status.code(), Some(73));
    let output = String::from_utf8(result.stdout).unwrap();
    let mut lines = output.lines();
    assert_eq!(lines.next().unwrap(), pid.to_string());
    assert_eq!(lines.collect::<Vec<_>>(), arguments);
    let decoy = temporary.path().join("path");
    std::fs::create_dir(&decoy).unwrap();
    std::fs::rename(&desktop, decoy.join("maka-desktop")).unwrap();
    let result = Command::new(&executable)
        .args(["desktop", "--help"])
        .env("PATH", &decoy)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("maka-desktop"));
    let help = Command::new(&executable).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("desktop"));
}
