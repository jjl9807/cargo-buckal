mod buck;
mod buckify;
mod cli;
mod commands;
mod utils;

use clap::Parser;

pub const RUST_CRATES_ROOT: &str = "third-party/rust/crates";

pub fn main() {
    let args = cli::Cli::parse();
    args.run();
}
