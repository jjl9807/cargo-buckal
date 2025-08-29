use clap::Parser;

use crate::{buck2::Buck2Command, utils::ensure_buck2_installed};

#[derive(Parser, Debug)]
pub struct CleanArgs {}

pub fn execute(_args: &CleanArgs) {
    // Ensure Buck2 is installed before proceeding
    if let Err(e) = ensure_buck2_installed() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = Buck2Command::clean().execute() {
        eprintln!("Failed to execute buck2 clean: {}", e);
        std::process::exit(1);
    }
}
