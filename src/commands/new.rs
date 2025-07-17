use std::process::{Command, Stdio};

use clap::Parser;

#[derive(Parser, Debug)]
pub struct NewArgs {
    pub path: String,
}

pub fn execute(args: &NewArgs) {
    let _status = Command::new("cargo")
        .arg("new")
        .arg(&args.path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("Failed to execute command");
    let _status = Command::new("buck2")
        .arg("init")
        .arg(&args.path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("Failed to execute command");
    std::fs::create_dir_all(format!("{}/third-party", args.path))
        .expect("Failed to create directory");
    std::fs::File::create(format!("{}/third-party/BUCK", args.path))
        .expect("Failed to create BUCK file in third-party directory");
}
