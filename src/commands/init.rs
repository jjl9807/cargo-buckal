use std::process::{Command, Stdio};

use clap::Parser;

#[derive(Parser, Debug)]
pub struct InitArgs {}

pub fn execute(_args: &InitArgs) {
    let _status = Command::new("cargo")
        .arg("init")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("Failed to execute command");
    let _status = Command::new("buck2")
        .arg("init")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("Failed to execute command");
    std::fs::create_dir("third-party").expect("Failed to create directory");
    std::fs::File::create("third-party/BUCK")
        .expect("Failed to create BUCK file in third-party directory");
}
