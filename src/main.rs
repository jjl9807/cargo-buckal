mod buck;
mod buck2;
mod buckify;
mod cli;
mod commands;
mod config;
mod utils;
mod cache;

use clap::Parser;

pub const RUST_CRATES_ROOT: &str = "third-party/rust/crates";

pub fn main() {
    let args = cli::Cli::parse();
    args.run();
}
