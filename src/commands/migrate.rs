use clap::Parser;

use crate::{
    buckify::flush_root,
    bundles::fetch_buckal_cell,
    cache::BuckalCache,
    context::BuckalContext,
    utils::{UnwrapOrExit, check_buck2_package, ensure_prerequisites},
};

#[derive(Parser, Debug)]
pub struct MigrateArgs {
    /// Do not use cached data from previous runs
    #[clap(long, name = "no-cache")]
    pub no_cache: bool,
    /// Merge manual edits with generated content
    #[clap(long)]
    pub merge: bool,
    /// Fetch latest bundles from remote repository
    #[clap(long)]
    pub fetch: bool,
    /// Process first-party crates separately
    #[clap(long)]
    pub separate: bool,
}

pub fn execute(args: &MigrateArgs) {
    // Ensure all prerequisites are installed before proceeding
    ensure_prerequisites().unwrap_or_exit();

    // Check if the current directory is a valid Buck2 package
    check_buck2_package().unwrap_or_exit();

    // get cargo metadata and generate context
    let mut ctx = BuckalContext::new();
    ctx.no_merge = !args.merge;
    ctx.separate = args.separate;

    // Fetch latest bundles if requested
    if args.fetch {
        let cwd = std::env::current_dir().unwrap_or_exit();
        fetch_buckal_cell(&cwd).unwrap_or_exit();
    }

    // Process the root node
    flush_root(&ctx);

    // Process dep nodes
    let last_cache = if args.no_cache || BuckalCache::load().is_err() {
        BuckalCache::new_empty()
    } else {
        BuckalCache::load().unwrap_or_exit_ctx("failed to load existing cache")
    };
    let new_cache = BuckalCache::new(&ctx.nodes_map);
    let changes = new_cache.diff(&last_cache);

    // Apply changes to BUCK files
    changes.apply(&ctx);

    // Flush the new cache
    new_cache.save();
}
