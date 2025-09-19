use clap::Parser;

use crate::{buck2::Buck2Command, buckal_error, utils::ensure_prerequisites};

#[derive(Parser, Debug)]
pub struct CleanArgs {}

pub fn execute(_args: &CleanArgs) {
    // Ensure all prerequisites are installed before proceeding
    if let Err(e) = ensure_prerequisites() {
        buckal_error!(e);
        std::process::exit(1);
    }

    if let Err(e) = Buck2Command::clean().execute() {
        buckal_error!(format!("Failed to execute buck2 clean:\n  {}", e));
        std::process::exit(1);
    }
}
