use std::collections::HashMap;

use cargo_metadata::{MetadataCommand, Node, Package, PackageId};

pub struct BuckalContext {
    pub root: Package,
    pub nodes_map: HashMap<PackageId, Node>,
    pub packages_map: HashMap<PackageId, Package>,
}

impl BuckalContext {
    pub fn new() -> Self {
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

        Self {
            root,
            nodes_map,
            packages_map,
        }
    }
}
