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

use std::{
    ffi::OsString,
    process::{Command, ExitCode},
};

#[derive(clap::Args)]
pub(super) struct Args {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    arguments: Vec<OsString>,
}

pub(super) fn bootstrap() -> Option<ExitCode> {
    (std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("desktop"))).then(|| {
        match launch(std::env::args_os().skip(2)) {
            Ok(exit) => exit,
            Err(error) => {
                eprintln!("desktop startup: {error}");
                ExitCode::FAILURE
            }
        }
    })
}

impl Args {
    pub(super) fn run(self) -> std::io::Result<ExitCode> {
        launch(self.arguments)
    }
}

fn launch(arguments: impl IntoIterator<Item = OsString>) -> std::io::Result<ExitCode> {
    let executable = std::env::current_exe()?;
    let desktop = executable.with_file_name(if cfg!(windows) {
        "maka-desktop.exe"
    } else {
        "maka-desktop"
    });
    let mut command = Command::new(&desktop);
    command.args(arguments);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        Err(command.exec()).map_err(|error| {
            std::io::Error::new(error.kind(), format!("{}: {error}", desktop.display()))
        })
    }
    #[cfg(windows)]
    {
        let status = command.status()?;
        // ExitCode only exposes a portable u8 constructor. Preserve the native
        // Windows exit status after the frontend and its owned workers finish.
        std::process::exit(status.code().unwrap_or(1));
    }
}
