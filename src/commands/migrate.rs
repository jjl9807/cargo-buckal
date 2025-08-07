use std::collections::{HashMap, HashSet, VecDeque};

use cargo_metadata::{MetadataCommand, Node, camino::Utf8PathBuf};
use clap::Parser;

use crate::{
    buckify::{
        buckify_dep_node, buckify_root_node, check_dep_target, gen_buck_content, vendor_package,
    },
    utils::check_buck2_package,
};

#[derive(Parser, Debug)]
pub struct MigrateArgs {
    /// generate minimal BUCK rules
    #[arg(long, default_value = "false")]
    pub minimal: bool,
    /// override existing source files
    #[arg(long, default_value = "false")]
    #[clap(name = "override")]
    pub _override: bool,
}

pub fn execute(args: &MigrateArgs) {
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
    let root_node = nodes_map.get(&root.id).expect("Root node not found");
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let buck_path = Utf8PathBuf::from(cwd.to_str().unwrap()).join("BUCK");

    // Generate BUCK rules
    let buck_rules = buckify_root_node(root_node, &packages_map, args.minimal);

    // Generate the BUCK file
    let buck_content = gen_buck_content(&buck_rules);
    std::fs::write(&buck_path, buck_content).expect("Failed to write BUCK file");

    // Process dep nodes
    let mut queue: VecDeque<Node> = VecDeque::new();
    let mut visited: HashSet<&str> = HashSet::new();

    for dep in &root_node.deps {
        if let Some(node) = nodes_map.get(&dep.pkg) {
            queue.push_back(node.clone());
        }
    }

    while let Some(node) = queue.pop_front() {
        let package = packages_map.get(&node.id).unwrap().to_owned();
        println!("Processing package: {}", package.name);

        // Vendor package sources
        let vendor_path = vendor_package(&package, args._override);

        // Create BUCK file
        let buck_path = vendor_path.join("BUCK");
        std::fs::File::create(&buck_path).expect("Failed to create BUCK file");

        // Generate BUCK rules
        let buck_rules = buckify_dep_node(&node, &packages_map, args.minimal);

        // Generate the BUCK file
        let buck_content = gen_buck_content(&buck_rules);
        std::fs::write(&buck_path, buck_content).expect("Failed to write BUCK file");

        for dep in &node.deps {
            if !dep.dep_kinds.iter().any(check_dep_target) {
                continue; // Skip dependencies that do not match the target
            }
            if let Some(next_node) = nodes_map.get(&dep.pkg)
                && visited.insert(&next_node.id.repr)
            {
                queue.push_back(next_node.clone());
            }
        }
    }
}
