use clap::Parser;

use crate::{
    buckal_error,
    buckify::flush_root,
    cache::BuckalCache,
    context::BuckalContext,
    utils::{check_buck2_package, ensure_prerequisites},
};

#[derive(Parser, Debug)]
pub struct MigrateArgs {}

pub fn execute(_args: &MigrateArgs) {
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

    // get cargo metadata and generate context
    let ctx = BuckalContext::new();

    // Process the root node
    flush_root(&ctx);

    // Process dep nodes
    let old_cache = BuckalCache::load();
    let new_cache = BuckalCache::new(&ctx.nodes_map);
    let changes = new_cache.diff(&old_cache);

    // Apply changes to BUCK files
    changes.apply(&ctx);

    // Flush the new cache
    new_cache.save();
}
