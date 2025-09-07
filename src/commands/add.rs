use std::{
    collections::HashMap,
    process::{Command, Stdio},
};

use cargo_metadata::{MetadataCommand, camino::Utf8PathBuf};
use clap::Parser;

use crate::{
    buckify::{buckify_dep_node, buckify_root_node, gen_buck_content, vendor_package},
    utils::{check_buck2_package, ensure_buck2_installed},
};

#[derive(Parser, Debug)]
pub struct AddArgs {
    pub package: String,
}

pub fn execute(args: &AddArgs) {
    // Ensure Buck2 is installed before proceeding
    if let Err(e) = ensure_buck2_installed() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // Check if the current directory is a valid Buck2 package
    check_buck2_package();

    // execute the `cargo add` command to add a package dependency
    let status = Command::new("cargo")
        .arg("add")
        .arg(&args.package)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("Failed to execute command");
    if !status.success() {
        return;
    }

    // get cargo metadata
    let cargo_metadata = MetadataCommand::new().exec().unwrap();

    let packages_map = cargo_metadata
        .to_owned()
        .packages
        .into_iter()
        .map(|p| (p.id.to_owned(), p))
        .collect::<HashMap<_, _>>();
    let root = cargo_metadata.root_package().unwrap().to_owned();
    let resolve = cargo_metadata.resolve.unwrap();

    for node in resolve.nodes {
        let package = packages_map.get(&node.id).unwrap().to_owned();
        let is_root = node.id == root.id;

        if is_root {
            // process the root package
            let cwd = std::env::current_dir().expect("Failed to get current directory");
            let buck_path = Utf8PathBuf::from(cwd.to_str().unwrap()).join("BUCK");

            // Generate BUCK rules
            let buck_rules = buckify_root_node(&node, &packages_map);

            // Generate the BUCK file
            let buck_content = gen_buck_content(&buck_rules);
            std::fs::write(&buck_path, buck_content).expect("Failed to write BUCK file");
        } else {
            // process third-party package
            // Vendor package sources
            let vendor_path = vendor_package(&package, false);

            // Create BUCK file
            let buck_path = vendor_path.join("BUCK");
            std::fs::File::create(&buck_path).expect("Failed to create BUCK file");

            // Generate BUCK rules
            let buck_rules = buckify_dep_node(&node, &packages_map);

            // Generate the BUCK file
            let buck_content = gen_buck_content(&buck_rules);
            std::fs::write(&buck_path, buck_content).expect("Failed to write BUCK file");
        }
    }
}
