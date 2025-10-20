use clap::Parser;
use ini::Ini;

use crate::{
    buckify::flush_root,
    cache::BuckalCache,
    context::BuckalContext,
    extract_bundles,
    utils::{UnwrapOrExit, check_buck2_package, ensure_prerequisites, get_buck2_root},
};

#[derive(Parser, Debug)]
pub struct MigrateArgs {
    #[clap(long, name = "no-cache")]
    pub no_cache: bool,
    #[clap(long, name = "no-merge")]
    pub no_merge: bool,
    /// overwrite bundled prelude files
    #[clap(long)]
    pub redist: bool,
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
    ctx.no_merge = args.no_merge;
    ctx.separate = args.separate;

    // Extract bundled prelude files if `--redist` is set
    if args.redist {
        let buck2_root = get_buck2_root().unwrap_or_exit_ctx("failed to get Buck2 project root");
        if buck2_root.join("buckal").exists() {
            std::fs::remove_dir_all(buck2_root.join("buckal"))
                .unwrap_or_exit_ctx("failed to overwrite custom prelude files");
        }
        extract_bundles(buck2_root.as_std_path())
            .unwrap_or_exit_ctx("failed to overwrite custom prelude files");

        let mut buck_config = Ini::load_from_file(buck2_root.join(".buckconfig"))
            .unwrap_or_exit_ctx("failed to parse .buckconfig");
        let cells = buck_config.section_mut(Some("cells")).unwrap();
        if cells.contains_key("buckal") {
            cells.remove("buckal");
        }
        cells.insert("buckal".to_string(), "buckal".to_string());
        buck_config
            .write_to_file(buck2_root.join(".buckconfig"))
            .unwrap_or_exit_ctx("failed to update .buckconfig with 'buckal' cell entry");
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
