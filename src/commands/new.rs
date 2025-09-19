use std::{fs::OpenOptions, io::Write, process::Command};

use clap::Parser;

use crate::{RUST_CRATES_ROOT, buck2::Buck2Command, buckal_error, utils::ensure_prerequisites};

#[derive(Parser, Debug)]
pub struct NewArgs {
    pub path: String,
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

pub fn execute(args: &NewArgs) {
    // Ensure all prerequisites are installed before proceeding
    if let Err(e) = ensure_prerequisites() {
        buckal_error!(e);
        std::process::exit(1);
    }

    if args.root {
        // Init a new buck2 repo
        if let Err(e) = Buck2Command::init().arg(&args.path).arg("--git").execute() {
            buckal_error!(format!("failed to execute buck2 init:\n  {}", e));
            std::process::exit(1);
        }
        std::fs::create_dir_all(format!("{}/{}", args.path, RUST_CRATES_ROOT))
            .expect("Failed to create directory");
        let mut git_ignore = OpenOptions::new()
            .create(false)
            .write(true)
            .append(true)
            .open(format!("{}/.gitignore", args.path))
            .expect("Failed to open .gitignore file");
        writeln!(git_ignore, "/.buckal").expect("Failed to write to .gitignore file");
    } else {
        // Create a new cargo package/buck2 cell
        let mut cargo_cmd = Command::new("cargo");
        cargo_cmd
            .arg("new")
            .arg(&args.path)
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
        let status = cargo_cmd.status().expect("Failed to execute command");
        if !status.success() {
            return;
        }

        let _buck = std::fs::File::create(format!("{}/BUCK", args.path))
            .expect("Failed to create BUCK file");
    }
}
