use clap::Parser;

use crate::{
    buck2::Buck2Command,
    buckal_error,
    utils::{UnwrapOrExit, check_buck2_package, ensure_prerequisites, get_buck2_root},
};

#[derive(Parser, Debug)]
pub struct BuildArgs {
    /// Build optimized artifacts with the release profile
    #[arg(short, long)]
    pub release: bool,
    /// Use verbose output (-vv very verbose output)
    #[arg(short, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

pub fn execute(args: &BuildArgs) {
    // Ensure all prerequisites are installed before proceeding
    ensure_prerequisites().unwrap_or_exit();

    // Check if the current directory is a valid Buck2 package
    check_buck2_package().unwrap_or_exit();

    // Get the root directory of the Buck2 project
    let buck2_root = get_buck2_root().unwrap_or_exit_ctx("failed to get Buck2 project root");
    let cwd = std::env::current_dir().unwrap_or_exit_ctx("failed to get current directory");
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
    let mut buck2_cmd = Buck2Command::build(&target).verbosity(args.verbose);
    if args.release {
        buck2_cmd = buck2_cmd.arg("-m").arg("release");
    }
    let result = buck2_cmd.status();

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
