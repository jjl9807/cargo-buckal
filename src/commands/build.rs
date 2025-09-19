use std::path::PathBuf;

use clap::Parser;

use crate::{
    buck2::Buck2Command,
    buckal_error,
    utils::{check_buck2_package, ensure_prerequisites, get_buck2_root},
};

#[derive(Parser, Debug)]
pub struct BuildArgs {
    /// Use verbose output (-vv very verbose output)
    #[arg(short, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

pub fn execute(args: &BuildArgs) {
    // Ensure all prerequisites are installed before proceeding
    if let Err(e) = ensure_prerequisites() {
        buckal_error!(e);
        std::process::exit(1);
    }

    // Check if the current directory is a valid Buck2 package
    if let Err(e) = check_buck2_package() {
        buckal_error!(e);
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
        buckal_error!("current directory is not inside the Buck2 project root");
        return;
    }
    let mut relative_path = relative.unwrap().to_string_lossy().into_owned();

    if !relative_path.is_empty() {
        relative_path += "/";
    }

    if args.verbose > 2 {
        buckal_error!("maximum verbosity");
        return;
    }

    let target = format!("//{relative_path}...");
    let result = Buck2Command::build(&target)
        .verbosity(args.verbose)
        .status();

    match result {
        Ok(status) if status.success() => {}
        Ok(_) => {
            buckal_error!("buck2 build failed");
            std::process::exit(1);
        }
        Err(e) => {
            buckal_error!(format!("failed to execute buck2 build:\n  {}", e));
            std::process::exit(1);
        }
    }
}
