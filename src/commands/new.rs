use std::process::{Command, Stdio};

use clap::Parser;

use crate::RUST_CRATES_ROOT;

#[derive(Parser, Debug)]
pub struct NewArgs {
    pub path: String,
}

pub fn execute(args: &NewArgs) {
    let _status = Command::new("cargo")
        .arg("new")
        .arg(&args.path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("Failed to execute command");
    let _status = Command::new("buck2")
        .arg("init")
        .arg(&args.path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("Failed to execute command");
    std::fs::create_dir_all(format!("{}/{}", args.path, RUST_CRATES_ROOT))
        .expect("Failed to create directory");
}
