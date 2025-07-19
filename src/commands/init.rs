use std::process::{Command, Stdio};

use clap::Parser;

use crate::RUST_CRATES_ROOT;

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
    std::fs::create_dir_all(RUST_CRATES_ROOT).expect("Failed to create directory");
}
