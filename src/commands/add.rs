use std::process::{Command, Stdio};

use clap::Parser;

use crate::{
    buckal_error,
    buckify::flush_root,
    cache::BuckalCache,
    context::BuckalContext,
    utils::{check_buck2_package, ensure_prerequisites, get_last_cache, section},
};

#[derive(Parser, Debug)]
pub struct AddArgs {
    pub package: String,
    #[arg(long, short = 'F')]
    pub features: Option<String>,
    #[arg(long)]
    pub rename: Option<String>,
    #[arg(long, default_value = "false")]
    pub dev: bool,
    #[arg(long, default_value = "false")]
    pub build: bool,
}

pub fn execute(args: &AddArgs) {
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

    // get last cache
    let last_cache = get_last_cache();

    let mut cargo_cmd = Command::new("cargo");
    cargo_cmd
        .arg("add")
        .arg(&args.package)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if let Some(features) = &args.features {
        cargo_cmd.arg("--features").arg(features);
    }
    if let Some(rename) = &args.rename {
        cargo_cmd.arg("--rename").arg(rename);
    }
    if args.dev {
        cargo_cmd.arg("--dev");
    }
    if args.build {
        cargo_cmd.arg("--build");
    }

    // execute the cargo command
    let status = cargo_cmd.status().expect("Failed to execute command");
    if !status.success() {
        return;
    }

    section("Buckal Console");

    // get cargo metadata and generate context
    let ctx = BuckalContext::new();

    // Process the root node
    flush_root(&ctx);

    let new_cache = BuckalCache::new(&ctx.nodes_map);
    let changes = new_cache.diff(&last_cache);

    // Apply changes to BUCK files
    changes.apply(&ctx);

    // Flush the new cache
    new_cache.save();
}
