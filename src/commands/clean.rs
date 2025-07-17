use std::process::{Command, Stdio};

use clap::Parser;

#[derive(Parser, Debug)]
pub struct CleanArgs {}

pub fn execute(_args: &CleanArgs) {
    let _status = Command::new("buck2")
        .arg("clean")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("Failed to execute command");
}
