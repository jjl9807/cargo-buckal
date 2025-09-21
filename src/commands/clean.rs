use clap::Parser;

use crate::{
    buck2::Buck2Command,
    utils::{UnwrapOrExit, check_buck2_package, ensure_prerequisites},
};

#[derive(Parser, Debug)]
pub struct CleanArgs {}

pub fn execute(_args: &CleanArgs) {
    // Ensure all prerequisites are installed before proceeding
    ensure_prerequisites().unwrap_or_exit();

    // Check if the current directory is a valid Buck2 package
    check_buck2_package().unwrap_or_exit();

    Buck2Command::clean().execute().unwrap_or_exit();
}
