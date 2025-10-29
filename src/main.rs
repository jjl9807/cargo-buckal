mod buck;
mod buck2;
mod buckify;
mod bundles;
mod cache;
mod cli;
mod commands;
mod config;
mod context;
mod utils;

use std::sync::OnceLock;

use clap::Parser;

pub const RUST_CRATES_ROOT: &str = "third-party/rust/crates";
pub const BUCKAL_BUNDLES_REPO: &str = "buck2hub/buckal-bundles";
pub const DEFAULT_BUNDLE_HASH: &str = "f9c4f306b1aad816fa520fe361f4f03d28cd5b7b";

pub fn main() {
    let args = cli::Cli::parse();
    args.run();
}

pub fn build_version() -> &'static str {
    static VERSION_STRING: OnceLock<String> = OnceLock::new();
    VERSION_STRING.get_or_init(|| {
        let pkg_version = env!("CARGO_PKG_VERSION");
        let git_hash = option_env!("GIT_HASH").unwrap_or("unknown");
        let commit_date = option_env!("COMMIT_DATE").unwrap_or("unknown");
        format!("{} ({} {})", pkg_version, git_hash, commit_date)
    })
}

pub fn user_agent() -> &'static str {
    static USER_AGENT_STRING: OnceLock<String> = OnceLock::new();
    USER_AGENT_STRING.get_or_init(|| {
        let pkg_version = env!("CARGO_PKG_VERSION");
        format!("buckal/{}", pkg_version)
    })
}
