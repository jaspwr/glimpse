use std::process::Command;

use execute::Execute;

pub fn execute_detached(name: String) -> Result<(), std::io::Error> {
    let mut command = Command::new("bash");
    command.arg("-c");
    command.arg(format!("{} & disown", name));
    command.execute()?;
    Ok(())
}

pub fn xdg_open(name: &String) -> Result<(), std::io::Error> {
    let mut command = std::process::Command::new("bash");
    command.arg("-c");
    command.arg(format!("xdg-open \"{}\" & disown", name.clone()));
    command.execute()?;
    Ok(())
}
