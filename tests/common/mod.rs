use std::{env, process::Command};

pub fn command() -> Command {
    // Create full path to binary
    let path = env::current_exe().expect("Could not get path to current executable.");
    let path = path.parent().expect("Path parent not found.");
    let mut path = path.parent().expect("Path parent not found.").to_owned();
    path.push(env!("CARGO_PKG_NAME"));
    path.set_extension(env::consts::EXE_EXTENSION);
    Command::new(path.into_os_string())
}
