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

#[path = "counter/support.rs"]
mod support;
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::path::absolute(PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("output directory required")?,
    ))?;
    std::fs::create_dir(&output)?;
    let (input, packages) = if std::env::args().nth(2).as_deref() == Some("editor") {
        support::editor_fixture()
    } else if std::env::args().nth(2).as_deref() == Some("native") {
        support::native_fixture()
    } else if std::env::args().nth(2).as_deref() == Some("composed") {
        support::composed_fixture(support::CHILD_UPDATE, support::PARENT_EVENT)
    } else {
        support::fixture("return frame({count:state.count+1,revision:state.revision+1});")
    };
    for package in packages {
        package.export_to(&output.join(format!("{}.maka-plugin", package.manifest().id)))?;
    }
    std::fs::write(
        output.join("input.json"),
        serde_json::to_vec_pretty(&input)?,
    )?;
    println!("{}", output.display());
    Ok(())
}
