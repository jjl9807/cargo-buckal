use std::process::{Command, Stdio};

use clap::Parser;

use crate::utils::ensure_buck2_installed;

#[derive(Parser, Debug)]
pub struct CleanArgs {}

pub fn execute(_args: &CleanArgs) {
    // Ensure Buck2 is installed before proceeding
    if let Err(e) = ensure_buck2_installed() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    let _status = Command::new("buck2")
        .arg("clean")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("Failed to execute command");
}
