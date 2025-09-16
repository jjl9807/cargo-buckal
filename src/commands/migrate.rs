use std::collections::HashMap;

use cargo_metadata::{MetadataCommand, camino::Utf8PathBuf};
use clap::Parser;
use regex::Regex;

use crate::{
    buck::{parse_buck_file, patch_buck_rules},
    buckify::{buckify_dep_node, buckify_root_node, gen_buck_content, vendor_package},
    cache::BuckalCache,
    utils::{check_buck2_package, ensure_buck2_installed, get_vendor_dir},
};

#[derive(Parser, Debug)]
pub struct MigrateArgs {
    /// override existing source files
    #[arg(long, default_value = "false")]
    #[clap(name = "override")]
    pub _override: bool,
}

pub fn execute(args: &MigrateArgs) {
    // Ensure Buck2 is installed before proceeding
    if let Err(e) = ensure_buck2_installed() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // Check if the current directory is a valid Buck2 package
    check_buck2_package();

    // get cargo metadata
    let cargo_metadata = MetadataCommand::new().exec().unwrap();

    let root = cargo_metadata.root_package().unwrap().to_owned();
    let packages_map = cargo_metadata
        .packages
        .into_iter()
        .map(|p| (p.id.to_owned(), p))
        .collect::<HashMap<_, _>>();
    let resolve = cargo_metadata.resolve.unwrap();
    let nodes_map = resolve
        .nodes
        .into_iter()
        .map(|n| (n.id.to_owned(), n))
        .collect::<HashMap<_, _>>();

    // Process the root node
    println!("Processing package: {}@{}", root.name, root.version);
    let root_node = nodes_map.get(&root.id).expect("Root node not found");
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let buck_path = Utf8PathBuf::from(cwd.to_str().unwrap()).join("BUCK");

    // Generate BUCK rules
    let buck_rules = buckify_root_node(root_node, &packages_map);

    // Generate the BUCK file
    let buck_content = gen_buck_content(&buck_rules);
    std::fs::write(&buck_path, buck_content).expect("Failed to write BUCK file");

    // Process dep nodes
    let old_cache = BuckalCache::load();
    let new_cache = BuckalCache::new(&nodes_map);
    let changes = new_cache.diff(&old_cache);

    for id in changes.added.iter().chain(changes.changed.iter()) {
        // Skip root package
        if id == &root.id {
            continue;
        }

        if let Some(node) = nodes_map.get(id) {
            let package = packages_map.get(id).unwrap();

            // Skip local packages
            if package.source.is_none() {
                continue;
            }

            println!("Processing package: {}@{}", package.name, package.version);

            // Vendor package sources
            let vendor_path = vendor_package(package, args._override);

            // Generate BUCK rules
            let mut buck_rules = buckify_dep_node(node, &packages_map);

            // Patch BUCK Rules
            let buck_path = vendor_path.join("BUCK");
            if buck_path.exists() && !args._override {
                let existing_rules =
                    parse_buck_file(&buck_path).expect("Failed to parse existing BUCK file");
                patch_buck_rules(&existing_rules, &mut buck_rules);
            } else {
                std::fs::File::create(&buck_path).expect("Failed to create BUCK file");
            }

            // Generate the BUCK file
            let buck_content = gen_buck_content(&buck_rules);
            std::fs::write(&buck_path, buck_content).expect("Failed to write BUCK file");
        }
    }

    let re = Regex::new(r"^([^+#]+)\+([^#]+)#([^@]+)@([^+#]+)(?:\+(.+))?$")
        .expect("error creating regex");

    for id in changes.removed.iter() {
        let caps = re.captures(&id.repr).expect("Failed to parse package ID");
        let name = &caps[3];
        let version = &caps[4];

        println!("Removing package: {}@{}", name, version);
        let vendor_dir = get_vendor_dir(name, version);
        if vendor_dir.exists() {
            std::fs::remove_dir_all(&vendor_dir).expect("Failed to remove vendor directory");
        }
        if let Some(package_dir) = vendor_dir.parent()
            && package_dir.read_dir().unwrap().next().is_none()
        {
            std::fs::remove_dir_all(package_dir)
                .expect("Failed to remove empty package directory");
        }
    }

    // Flush the new cache
    new_cache.save();
}
