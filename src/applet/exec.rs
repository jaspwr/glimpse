use std::{process::Command, io::Write};

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
