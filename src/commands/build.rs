use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use clap::Parser;

use crate::utils::get_buck2_root;

#[derive(Parser, Debug)]
pub struct BuildArgs {
    /// Use verbose output (-vv very verbose output)
    #[arg(short, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

pub fn execute(args: &BuildArgs) {
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

    let mut buck2_build_cmd = Command::new("buck2");
    buck2_build_cmd
        .arg("build")
        .arg(format!("//{relative_path}..."))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    match args.verbose {
        0 => {}
        1 => {
            buck2_build_cmd.arg("-v=3");
        }
        2 => {
            buck2_build_cmd.arg("-v=4");
        }
        _ => {
            eprintln!("error: Maximum verbosity!");
            return;
        }
    }
    let _status = buck2_build_cmd.status().expect("Failed to execute command");
}
