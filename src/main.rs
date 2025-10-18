mod buck;
mod buck2;
mod buckify;
mod cache;
mod cli;
mod commands;
mod config;
mod context;
mod utils;

use clap::Parser;

pub const RUST_CRATES_ROOT: &str = "third-party/rust/crates";

pub fn main() {
    let args = cli::Cli::parse();
    args.run();
}

pub fn build_version() -> &'static str {
    use std::sync::OnceLock;
    static VERSION_STRING: OnceLock<String> = OnceLock::new();
    VERSION_STRING.get_or_init(|| {
        let pkg_version = env!("CARGO_PKG_VERSION");
        let git_hash = option_env!("GIT_HASH").unwrap_or("unknown");
        let commit_date = option_env!("COMMIT_DATE").unwrap_or("unknown");
        format!("{} ({} {})", pkg_version, git_hash, commit_date)
    })
}
