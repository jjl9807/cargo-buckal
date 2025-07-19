use std::{
    fs::OpenOptions,
    io::Write,
    process::{Command, Stdio},
};

use clap::Parser;

use crate::RUST_CRATES_ROOT;

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
    std::fs::create_dir_all(format!("{}/{}", args.path, RUST_CRATES_ROOT))
        .expect("Failed to create directory");

    let mut git_ignore = OpenOptions::new()
        .create(false)
        .write(true)
        .append(true)
        .open(format!("{}/.gitignore", args.path))
        .expect("Failed to open .gitignore file");
    writeln!(git_ignore, "/buck-out").expect("Failed to write to .gitignore file");
}
