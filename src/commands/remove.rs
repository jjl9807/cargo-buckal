use std::process::Command;

use clap::Parser;

use crate::{
    buckify::flush_root,
    cache::BuckalCache,
    context::BuckalContext,
    utils::{UnwrapOrExit, check_buck2_package, ensure_prerequisites, get_last_cache, section},
};

#[derive(Parser, Debug)]
pub struct RemoveArgs {
    #[clap(value_name = "DEP_ID", num_args = 1..)]
    packages: Vec<String>,
    #[arg(long, default_value = "false")]
    pub dev: bool,
    #[arg(long, default_value = "false")]
    pub build: bool,
}

pub fn execute(args: &RemoveArgs) {
    // Ensure all prerequisites are installed before proceeding
    ensure_prerequisites().unwrap_or_exit();

    // Check if the current directory is a valid Buck2 package
    check_buck2_package().unwrap_or_exit();

    // get last cache
    let last_cache = get_last_cache();

    let mut cargo_cmd = Command::new("cargo");
    cargo_cmd
        .arg("remove")
        .args(&args.packages)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());
    if args.dev {
        cargo_cmd.arg("--dev");
    }
    if args.build {
        cargo_cmd.arg("--build");
    }

    // execute the cargo command
    let status = cargo_cmd
        .status()
        .unwrap_or_exit_ctx("failed to execute `cargo remove`");
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
    changes.apply(&ctx, false);

    // Flush the new cache
    new_cache.save();
}
