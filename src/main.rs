mod cli;
mod commands;
mod utils;

use clap::Parser;

pub fn main() {
    let args = cli::Cli::parse();
    args.run();
}