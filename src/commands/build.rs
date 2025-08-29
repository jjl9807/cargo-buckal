use std::path::PathBuf;

use clap::Parser;

use crate::{
    buck2::Buck2Command,
    utils::{ensure_buck2_installed, get_buck2_root},
};

#[derive(Parser, Debug)]
pub struct BuildArgs {
    /// Use verbose output (-vv very verbose output)
    #[arg(short, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

pub fn execute(args: &BuildArgs) {
    // Ensure Buck2 is installed before proceeding
    if let Err(e) = ensure_buck2_installed() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // Get the root directory of the Buck2 project
    let buck2_root = get_buck2_root();
    if buck2_root.is_empty() {
        return;
    }
    let buck2_root = PathBuf::from(buck2_root.trim());
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let relative = cwd.strip_prefix(&buck2_root).ok();

    if relative.is_none() {
        eprintln!("error: Current directory is not inside the Buck2 project root.");
        return;
    }
    let mut relative_path = relative.unwrap().to_string_lossy().into_owned();

    if !relative_path.is_empty() {
        relative_path += "/";
    }

    if args.verbose > 2 {
        eprintln!("error: Maximum verbosity!");
        return;
    }

    let target = format!("//{relative_path}...");
    let result = Buck2Command::build(&target)
        .verbosity(args.verbose)
        .status();

    match result {
        Ok(status) if status.success() => {}
        Ok(_) => {
            eprintln!("Buck2 build failed");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Failed to execute buck2 build: {}", e);
            std::process::exit(1);
        }
    }
}
