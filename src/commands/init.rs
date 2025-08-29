use std::{
    fs::OpenOptions,
    io::Write,
    process::{Command, Stdio},
};

use clap::Parser;

use crate::{RUST_CRATES_ROOT, utils::ensure_buck2_installed};

#[derive(Parser, Debug)]
pub struct InitArgs {}

pub fn execute(_args: &InitArgs) {
    // Ensure Buck2 is installed before proceeding
    if let Err(e) = ensure_buck2_installed() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

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

    let mut git_ignore = OpenOptions::new()
        .create(false)
        .append(true)
        .open(".gitignore")
        .expect("Failed to open .gitignore file");
    writeln!(git_ignore, "/buck-out").expect("Failed to write to .gitignore file");
}
