use clap::Parser;

use crate::build_version;

#[derive(Parser, Debug)]
pub struct VersionArgs {}

pub fn execute(_args: &VersionArgs) {
    println!("buckal {}", build_version());
}
