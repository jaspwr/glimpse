// Glimpse - GNU/Linux launcher and file search utility.
// Copyright (C) 2024 https://github.com/jaspwr

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{io::Write, process::Command};

use execute::Execute;

pub fn execute_detached(name: String) -> Result<(), std::io::Error> {
    let mut command = Command::new("bash");
    command.arg("-c");
    command.arg(format!("{} & disown", name));
    command.execute()?;
    Ok(())
}

pub fn write_clipboard(s: &str) -> Result<(), std::io::Error> {
    let mut command = Command::new("xclip");
    command.arg("-sel");
    command.arg("clip");
    command.arg("-r");

    command.stdin(std::process::Stdio::piped());
    let mut child = command.spawn()?;
    child.stdin.as_mut().unwrap().write_all(s.as_bytes())?;
    child.wait()?;
    Ok(())
}

pub fn xdg_open(name: &String) -> Result<(), std::io::Error> {
    let mut command = Command::new("bash");
    command.arg("-c");
    command.arg(format!("xdg-open \"{}\" & disown", name.clone()));
    command.execute()?;
    Ok(())
}
