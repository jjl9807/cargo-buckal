use std::process::Command;

use clap::Parser;

use crate::{
    buckify::flush_root,
    cache::BuckalCache,
    context::BuckalContext,
    utils::{check_buck2_package, ensure_buck2_installed, get_last_cache, section},
};

#[derive(Parser, Debug)]
pub struct UpdateArgs {
    #[clap(value_name = "SPEC", num_args = 1..)]
    packages: Vec<String>,
    #[arg(long, default_value = "false")]
    pub recursive: bool,
}

pub fn execute(args: &UpdateArgs) {
    // Ensure Buck2 is installed before proceeding
    if let Err(e) = ensure_buck2_installed() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // Check if the current directory is a valid Buck2 package
    check_buck2_package();

    // get last cache
    let last_cache = get_last_cache();

    let mut cargo_cmd = Command::new("cargo");
    cargo_cmd
        .arg("update")
        .args(&args.packages)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());
    if args.recursive {
        cargo_cmd.arg("--recursive");
    }

    // execute the cargo command
    let status = cargo_cmd.status().expect("Failed to execute command");
    if !status.success() {
        return;
    }

    section("Buckal Changelog");

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
