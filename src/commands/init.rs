use std::{fs::OpenOptions, io::Write, process::Command};

use clap::Parser;

use crate::{
    RUST_CRATES_ROOT,
    buck2::Buck2Command,
    utils::{UnwrapOrExit, ensure_prerequisites},
};

#[derive(Parser, Debug)]
pub struct InitArgs {
    #[arg(long, default_value = "false")]
    pub bin: bool,
    #[arg(long, default_value = "false")]
    pub lib: bool,
    #[arg(long)]
    pub edition: Option<String>,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long, default_value = "false", conflicts_with_all = ["bin", "lib", "edition", "name"])]
    pub root: bool,
}

pub fn execute(args: &InitArgs) {
    // Ensure all prerequisites are installed before proceeding
    ensure_prerequisites().unwrap_or_exit();

    if args.root {
        Buck2Command::init().arg("--git").execute().unwrap_or_exit();
        std::fs::create_dir_all(RUST_CRATES_ROOT)
            .unwrap_or_exit_ctx("failed to create third-party directory");
        let mut git_ignore = OpenOptions::new()
            .create(false)
            .append(true)
            .open(".gitignore")
            .unwrap_or_exit_ctx("failed to open `.gitignore` file");
        writeln!(git_ignore, "/.buckal").unwrap_or_exit_ctx("failed to write to `.gitignore` file");
    } else {
        let mut cargo_cmd = Command::new("cargo");
        cargo_cmd
            .arg("init")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
        if args.bin {
            cargo_cmd.arg("--bin");
        }
        if args.lib {
            cargo_cmd.arg("--lib");
        }
        if let Some(edition) = &args.edition {
            cargo_cmd.arg("--edition").arg(edition);
        }
        if let Some(name) = &args.name {
            cargo_cmd.arg("--name").arg(name);
        }

        // execute the cargo command
        let status = cargo_cmd
            .status()
            .unwrap_or_exit_ctx("failed to execute `cargo init`");
        if !status.success() {
            return;
        }

        let _buck =
            std::fs::File::create("BUCK").unwrap_or_exit_ctx("failed to create `BUCK` file");
    }
}
