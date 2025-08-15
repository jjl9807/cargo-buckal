use std::collections::{BTreeMap as Map, BTreeSet as Set, HashMap};

use cargo_metadata::{DepKindInfo, DependencyKind, Node, Package, PackageId, camino::Utf8PathBuf};
use fs_extra::dir::{CopyOptions, copy};
use itertools::Itertools;

use crate::{
    RUST_CRATES_ROOT,
    buck::{BuildscriptRun, CargoRustBinary, CargoRustLibrary, Load, Rule},
    utils::{get_buck2_root, get_cfgs, get_target},
};

pub fn buckify_dep_node(
    node: &Node,
    packages_map: &HashMap<PackageId, Package>,
    is_minimal: bool,
) -> Vec<Rule> {
    let package = packages_map.get(&node.id).unwrap().to_owned();
    let buckal_name = format!("{}-{}", package.name, package.version);

    // emit buck rules for lib target
    let mut buck_rules: Vec<Rule> = Vec::new();
    buck_rules.push(Rule::Load(Load {
        bzl: "@prelude//rust:cargo_buildscript.bzl".to_owned(),
        items: Set::from(["buildscript_run".to_owned()]),
    }));
    buck_rules.push(Rule::Load(Load {
        bzl: "@prelude//rust:cargo_package.bzl".to_owned(),
        items: Set::from(["cargo".to_owned()]),
    }));

    let mut rust_library = CargoRustLibrary {
        name: buckal_name.clone(),
        crate_name: package.name.to_string(),
        edition: package.edition.to_string(),
        features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
        visibility: Set::from(["PUBLIC".to_owned()]),
        ..Default::default()
    };

    if is_minimal {
        // If minimal, only include nessary fields
        rust_library.srcs.include = Set::from(["**/*.rs".to_owned()]);
    } else {
        // Include all possible fields
        rust_library.srcs.include = Set::from(["**/**".to_owned()]);
        rust_library.srcs.exclude = Set::from(["BUCK".to_owned()]);
        rust_library.env = gen_cargo_env(&package);
    }

    // Set the crate root path
    let manifest_dir = package.manifest_path.parent().unwrap().to_owned();
    let lib_target = package
        .targets
        .iter()
        .find(|t| {
            t.kind.contains(&cargo_metadata::TargetKind::Lib)
                || t.kind.contains(&cargo_metadata::TargetKind::ProcMacro)
        })
        .expect("No library target found");
    if lib_target
        .kind
        .contains(&cargo_metadata::TargetKind::ProcMacro)
    {
        rust_library.proc_macro = Some(true);
    }
    rust_library.crate_root = lib_target
        .src_path
        .to_owned()
        .strip_prefix(&manifest_dir)
        .expect("Failed to get library source path")
        .to_string();

    // Set dependencies
    let dep_prefix = format!("//{RUST_CRATES_ROOT}/");
    for dep in &node.deps {
        if let Some(dep_package) = packages_map.get(&dep.pkg) {
            let dep_name = format!("{}-{}", dep_package.name, dep_package.version);
            if dep
                .dep_kinds
                .iter()
                .any(|dk| dk.kind == DependencyKind::Normal && check_dep_target(dk))
            {
                // Normal dependencies on current arch
                rust_library
                    .deps
                    .insert(format!("{dep_prefix}{dep_name}:{dep_name}"));
            }
        }
    }

    // Check if the package has a build script
    let custom_build_target = package
        .targets
        .iter()
        .find(|t| t.kind.contains(&cargo_metadata::TargetKind::CustomBuild));

    if let Some(build_target) = custom_build_target {
        // process the build script in rust_library
        rust_library.env.insert(
            "OUT_DIR".to_owned(),
            format!("$(location :{buckal_name}-build-script-run[out_dir])").to_owned(),
        );
        rust_library.rustc_flags.insert(
            format!("@$(location :{buckal_name}-build-script-run[rustc_flags])").to_owned(),
        );
        buck_rules.push(Rule::CargoRustLibrary(rust_library));

        // create the build script rule
        let mut buildscript_build = CargoRustBinary {
            name: format!("{}-{}", buckal_name, build_target.name),
            crate_name: build_target.name.to_owned(),
            edition: package.edition.to_string(),
            features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
            ..Default::default()
        };

        if is_minimal {
            // If minimal, only include nessary fields
            buildscript_build.srcs.include =
                Set::from(["**/*.rs".to_owned(), "build.rs".to_owned()]);
        } else {
            // Include all possible fields
            buildscript_build.srcs.include = Set::from(["**/**".to_owned()]);
            buildscript_build.srcs.exclude = Set::from(["BUCK".to_owned()]);
            buildscript_build.env = gen_cargo_env(&package);
        }

        // Set the crate root path for the build script
        buildscript_build.crate_root = build_target
            .src_path
            .to_owned()
            .strip_prefix(&manifest_dir)
            .expect("Failed to get library source path")
            .to_string();

        // Set dependencies for the build script
        for dep in &node.deps {
            if let Some(dep_package) = packages_map.get(&dep.pkg) {
                let dep_name = format!("{}-{}", dep_package.name, dep_package.version);
                if dep
                    .dep_kinds
                    .iter()
                    .any(|dk| dk.kind == DependencyKind::Build)
                {
                    // Build dependencies
                    buildscript_build
                        .deps
                        .insert(format!("{dep_prefix}{dep_name}:{dep_name}"));
                }
            }
        }
        buck_rules.push(Rule::CargoRustBinary(buildscript_build));

        // create the build script run rule
        let buildscript_run = BuildscriptRun {
            name: format!("{buckal_name}-build-script-run"),
            package_name: package.name.to_string(),
            buildscript_rule: format!(":{}-{}", buckal_name, build_target.name),
            features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
            version: package.version.to_string(),
            local_manifest_dir: "**".to_owned(),
            ..Default::default()
        };
        buck_rules.push(Rule::BuildscriptRun(buildscript_run));
    } else {
        // No build script, all rules done!
        buck_rules.push(Rule::CargoRustLibrary(rust_library));
    }

    buck_rules
}

pub fn buckify_root_node(
    node: &Node,
    packages_map: &HashMap<PackageId, Package>,
    is_minimal: bool,
) -> Vec<Rule> {
    let package = packages_map.get(&node.id).unwrap().to_owned();
    let buckal_name = package.name.to_string();

    // emit buck rules for bin target
    let mut buck_rules: Vec<Rule> = Vec::new();
    buck_rules.push(Rule::Load(Load {
        bzl: "@prelude//rust:cargo_buildscript.bzl".to_owned(),
        items: Set::from(["buildscript_run".to_owned()]),
    }));
    buck_rules.push(Rule::Load(Load {
        bzl: "@prelude//rust:cargo_package.bzl".to_owned(),
        items: Set::from(["cargo".to_owned()]),
    }));

    let mut rust_binary = CargoRustBinary {
        name: buckal_name.clone(),
        crate_name: package.name.to_string(),
        edition: package.edition.to_string(),
        features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
        visibility: Set::from(["PUBLIC".to_owned()]),
        ..Default::default()
    };

    if is_minimal {
        // If minimal, only include nessary fields
        rust_binary.srcs.include = Set::from(["**/*.rs".to_owned()]);
    } else {
        // Include all possible fields
        rust_binary.srcs.include = Set::from(["**/**".to_owned()]);
        rust_binary.srcs.exclude = Set::from(["BUCK".to_owned()]);
        rust_binary.env = gen_cargo_env(&package);
    }

    // Set the crate root path
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let manifest_dir = Utf8PathBuf::from(cwd.to_str().unwrap());
    let bin_target = package
        .targets
        .iter()
        .find(|t| t.kind.contains(&cargo_metadata::TargetKind::Bin))
        .expect("No binary target found");
    rust_binary.crate_root = bin_target
        .src_path
        .to_owned()
        .strip_prefix(&manifest_dir)
        .expect("Failed to get binary source path")
        .to_string();

    // Set dependencies
    let dep_prefix = format!("//{RUST_CRATES_ROOT}/");
    for dep in &node.deps {
        if let Some(dep_package) = packages_map.get(&dep.pkg) {
            let dep_name = format!("{}-{}", dep_package.name, dep_package.version);
            if dep
                .dep_kinds
                .iter()
                .any(|dk| dk.kind == DependencyKind::Normal)
            {
                // Normal dependencies
                rust_binary
                    .deps
                    .insert(format!("{dep_prefix}{dep_name}:{dep_name}"));
            }
        }
    }

    // Check if the package has a build script
    let custom_build_target = package
        .targets
        .iter()
        .find(|t| t.kind.contains(&cargo_metadata::TargetKind::CustomBuild));

    if let Some(build_target) = custom_build_target {
        // process the build script in rust_binary
        rust_binary.env.insert(
            "OUT_DIR".to_owned(),
            format!("$(location :{buckal_name}-build-script-run[out_dir])").to_owned(),
        );
        rust_binary.rustc_flags.insert(
            format!("@$(location :{buckal_name}-build-script-run[rustc_flags])").to_owned(),
        );
        buck_rules.push(Rule::CargoRustBinary(rust_binary));

        // create the build script rule
        let mut buildscript_build = CargoRustBinary {
            name: format!("{}-{}", buckal_name, build_target.name),
            crate_name: build_target.name.to_owned(),
            edition: package.edition.to_string(),
            features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
            ..Default::default()
        };

        if is_minimal {
            // If minimal, only include nessary fields
            buildscript_build.srcs.include =
                Set::from(["**/*.rs".to_owned(), "build.rs".to_owned()]);
        } else {
            // Include all possible fields
            buildscript_build.srcs.include = Set::from(["**/**".to_owned()]);
            buildscript_build.srcs.exclude = Set::from(["BUCK".to_owned()]);
            buildscript_build.env = gen_cargo_env(&package);
        }

        // Set the crate root path for the build script
        buildscript_build.crate_root = build_target
            .src_path
            .to_owned()
            .strip_prefix(&manifest_dir)
            .expect("Failed to get library source path")
            .to_string();

        // Set dependencies for the build script
        for dep in &node.deps {
            if let Some(dep_package) = packages_map.get(&dep.pkg) {
                let dep_name = format!("{}-{}", dep_package.name, dep_package.version);
                if dep
                    .dep_kinds
                    .iter()
                    .any(|dk| dk.kind == DependencyKind::Build)
                {
                    // Build dependencies
                    buildscript_build
                        .deps
                        .insert(format!("{dep_prefix}{dep_name}:{dep_name}"));
                }
            }
        }
        buck_rules.push(Rule::CargoRustBinary(buildscript_build));

        // create the build script run rule
        let buildscript_run = BuildscriptRun {
            name: format!("{buckal_name}-build-script-run"),
            package_name: package.name.to_string(),
            buildscript_rule: format!(":{}-{}", buckal_name, build_target.name),
            features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
            version: package.version.to_string(),
            local_manifest_dir: "**".to_owned(),
            ..Default::default()
        };
        buck_rules.push(Rule::BuildscriptRun(buildscript_run));
    } else {
        // No build script, all rules done!
        buck_rules.push(Rule::CargoRustBinary(rust_binary));
    }

    buck_rules
}

pub fn vendor_package(package: &Package, is_override: bool) -> Utf8PathBuf {
    // Vendor the package sources to `third-party/rust/crates/<package_name>-<version>`
    let manifest_path = package.manifest_path.clone();
    let src_path = manifest_path.parent().unwrap().to_owned();
    let target_path = Utf8PathBuf::from(get_buck2_root()).join(format!(
        "{RUST_CRATES_ROOT}/{}-{}",
        package.name, package.version
    ));
    if !target_path.exists() {
        std::fs::create_dir_all(&target_path).expect("Failed to create target directory");
    }
    let copy_options = CopyOptions {
        skip_exist: !is_override,
        overwrite: is_override,
        content_only: true,
        ..Default::default()
    };
    copy(&src_path, &target_path, &copy_options).expect("Failed to copy package sources");

    target_path
}

pub fn gen_cargo_env(package: &Package) -> Map<String, String> {
    // Generate cargo environment variables
    let mut cargo_env: Map<String, String> = Map::new();
    cargo_env.insert("CARGO_CRATE_NAME".to_owned(), package.name.to_string());
    cargo_env.insert("CARGO_MANIFEST_DIR".to_owned(), ".".to_owned());
    cargo_env.insert("CARGO_PKG_NAME".to_owned(), package.name.to_string());
    cargo_env.insert("CARGO_PKG_VERSION".to_owned(), package.version.to_string());
    cargo_env.insert(
        "CARGO_PKG_VERSION_MAJOR".to_owned(),
        package.version.major.to_string(),
    );
    cargo_env.insert(
        "CARGO_PKG_VERSION_MINOR".to_owned(),
        package.version.minor.to_string(),
    );
    cargo_env.insert(
        "CARGO_PKG_VERSION_PATCH".to_owned(),
        package.version.patch.to_string(),
    );
    cargo_env.insert(
        "CARGO_PKG_VERSION_PRE".to_owned(),
        package.version.pre.to_string(),
    );
    if let Some(links) = &package.links {
        cargo_env.insert("CARGO_PKG_LINKS".to_owned(), links.to_string());
    }

    cargo_env
}

pub fn gen_buck_content(rules: &[Rule]) -> String {
    let mut content = rules
        .iter()
        .map(serde_starlark::to_string)
        .map(Result::unwrap)
        .join("\n");

    content.insert_str(0, "# @generated by `cargo buckal`\n\n");
    content
}

pub fn check_dep_target(dk: &DepKindInfo) -> bool {
    if dk.target.is_none() {
        return true; // No target specified
    }

    let platform = dk.target.as_ref().unwrap();
    let target = get_target();
    let cfgs = get_cfgs();

    platform.matches(&target, &cfgs[..])
}
