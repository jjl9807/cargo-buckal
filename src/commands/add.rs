use std::{
    collections::{BTreeSet as Set, HashMap},
    process::{Command, Stdio},
};

use cargo_metadata::{DependencyKind, MetadataCommand, camino::Utf8PathBuf};
use clap::Parser;
use fs_extra::dir::{CopyOptions, copy};
use itertools::Itertools;

use crate::{
    RUST_CRATES_ROOT,
    buck::{BuildscriptRun, CargoRustBinary, CargoRustLibrary, Glob, Load, Rule},
    utils::get_buck2_root,
};

#[derive(Parser, Debug)]
pub struct AddArgs {
    pub package: String,
}

pub fn execute(args: &AddArgs) {
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

        // TODO: process cargo environment variables

        if is_root {
            // process the root package
            // emit buck rules for bin target
            // assert that the plugin is running in the root package
            // Cargo.toml and BUCK file should exist in current directory
            let buckal_name = package.name.to_string();
            let cwd = std::env::current_dir().expect("Failed to get current directory");
            let src_path = Utf8PathBuf::from(cwd.to_str().unwrap());
            let buck_path = src_path.join("BUCK");

            // START: Generate the BUCK file content
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
                srcs: Glob {
                    // TODO: change `include` to ["**/*.rs"] after buck file parser finished
                    include: Set::from(["**/*.*".to_owned()]),
                    exclude: Set::from([
                        "LICENSE*".to_owned(),
                        "Cargo.*".to_owned(),
                        "BUCK".to_owned(),
                    ]),
                },
                crate_name: package.name.to_string(),
                edition: package.edition.to_string(),
                features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
                visibility: Set::from(["PUBLIC".to_owned()]),
                ..Default::default()
            };

            // Set the crate root path
            let bin_target = package
                .targets
                .iter()
                .find(|t| t.kind.contains(&cargo_metadata::TargetKind::Bin))
                .expect("No binary target found");
            let bin_src = bin_target
                .src_path
                .to_owned()
                .strip_prefix(&src_path)
                .expect("Failed to get binary source path")
                .to_string();
            rust_binary.crate_root = bin_src;

            // Set dependencies
            let dep_prefix = format!("//{}/", RUST_CRATES_ROOT);
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
                            .insert(format!("{}{}:{}", dep_prefix, dep_name, dep_name));
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
                    format!("$(location :{}-build-script-run[out_dir])", buckal_name).to_owned(),
                );
                rust_binary.rustc_flags.insert(
                    format!(
                        "@$(location :{}-build-script-run[rustc_flags])",
                        buckal_name
                    )
                    .to_owned(),
                );
                buck_rules.push(Rule::CargoRustBinary(rust_binary));

                // create the build script rule
                let mut buildscript_build = CargoRustBinary {
                    name: format!("{}-{}", buckal_name, build_target.name),
                    srcs: Glob {
                        // TODO: change `include` to ["**/*.rs", "build.rs"] after buck file parser finished
                        include: Set::from(["**/*.*".to_owned()]),
                        exclude: Set::from([
                            "LICENSE*".to_owned(),
                            "Cargo.*".to_owned(),
                            "BUCK".to_owned(),
                        ]),
                    },
                    crate_name: build_target.name.to_owned(),
                    edition: package.edition.to_string(),
                    features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
                    ..Default::default()
                };

                // Set the crate root path for the build script
                let build_src = build_target
                    .src_path
                    .to_owned()
                    .strip_prefix(src_path)
                    .expect("Failed to get library source path")
                    .to_string();
                buildscript_build.crate_root = build_src;

                // Set dependencies for the build script
                for dep in node.deps {
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
                                .insert(format!("{}{}:{}", dep_prefix, dep_name, dep_name));
                        }
                    }
                }
                buck_rules.push(Rule::CargoRustBinary(buildscript_build));

                // create the build script run rule
                let buildscript_run = BuildscriptRun {
                    name: format!("{}-build-script-run", buckal_name),
                    package_name: package.name.to_string(),
                    buildscript_rule: format!(":{}-{}", buckal_name, build_target.name),
                    features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
                    version: package.version.to_string(),
                    ..Default::default()
                };
                buck_rules.push(Rule::BuildscriptRun(buildscript_run));
            } else {
                // No build script, all rules done!
                buck_rules.push(Rule::CargoRustBinary(rust_binary));
            }

            // generate the BUCK file
            let mut buck_file = buck_rules
                .iter()
                .map(serde_starlark::to_string)
                .map(Result::unwrap)
                .join("\n");

            buck_file.insert_str(0, "# @generated by `cargo buck`\n\n");
            std::fs::write(&buck_path, buck_file).expect("Failed to write BUCK file");
        } else {
            // process third-party packages
            // Vendor the package sources to `third-party/rust/crates/<package_name>-<version>`
            let manifest_path = package.manifest_path.clone();
            let src_path = manifest_path.parent().unwrap().to_owned();
            let target_dir_path = Utf8PathBuf::from(get_buck2_root()).join(RUST_CRATES_ROOT);
            copy(&src_path, &target_dir_path, &CopyOptions::new())
                .expect("Failed to copy package sources");

            // Create the BUCK file for the package
            let buckal_name = format!("{}-{}", package.name, package.version);
            let buck_path = target_dir_path.join(format!("{}/BUCK", buckal_name));
            std::fs::File::create(&buck_path).expect("Failed to create BUCK file");

            // START: Generate the BUCK file content
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
                srcs: Glob {
                    // TODO: change `include` to ["**/*.rs"] after buck file parser finished
                    include: Set::from(["**/*.*".to_owned()]),
                    exclude: Set::from([
                        "LICENSE*".to_owned(),
                        "Cargo.*".to_owned(),
                        "BUCK".to_owned(),
                    ]),
                },
                crate_name: package.name.to_string(),
                edition: package.edition.to_string(),
                features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
                visibility: Set::from(["PUBLIC".to_owned()]),
                ..Default::default()
            };

            // Set the crate root path
            let lib_target = package
                .targets
                .iter()
                .find(|t| t.kind.contains(&cargo_metadata::TargetKind::Lib))
                .expect("No library target found");
            let lib_src = lib_target
                .src_path
                .to_owned()
                .strip_prefix(&src_path)
                .expect("Failed to get library source path")
                .to_string();
            rust_library.crate_root = lib_src;

            // Set dependencies
            let dep_prefix = format!("//{}/", RUST_CRATES_ROOT);
            for dep in &node.deps {
                if let Some(dep_package) = packages_map.get(&dep.pkg) {
                    let dep_name = format!("{}-{}", dep_package.name, dep_package.version);
                    if dep
                        .dep_kinds
                        .iter()
                        .any(|dk| dk.kind == DependencyKind::Normal)
                    {
                        // Normal dependencies
                        rust_library
                            .deps
                            .insert(format!("{}{}:{}", dep_prefix, dep_name, dep_name));
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
                    format!("$(location :{}-build-script-run[out_dir])", buckal_name).to_owned(),
                );
                rust_library.rustc_flags.insert(
                    format!(
                        "@$(location :{}-build-script-run[rustc_flags])",
                        buckal_name
                    )
                    .to_owned(),
                );
                buck_rules.push(Rule::CargoRustLibrary(rust_library));

                // create the build script rule
                let mut buildscript_build = CargoRustBinary {
                    name: format!("{}-{}", buckal_name, build_target.name),
                    srcs: Glob {
                        // TODO: change `include` to ["**/*.rs", "build.rs"] after buck file parser finished
                        include: Set::from(["**/*.*".to_owned()]),
                        exclude: Set::from([
                            "LICENSE*".to_owned(),
                            "Cargo.*".to_owned(),
                            "BUCK".to_owned(),
                        ]),
                    },
                    crate_name: build_target.name.to_owned(),
                    edition: package.edition.to_string(),
                    features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
                    ..Default::default()
                };

                // Set the crate root path for the build script
                let build_src = build_target
                    .src_path
                    .to_owned()
                    .strip_prefix(src_path)
                    .expect("Failed to get library source path")
                    .to_string();
                buildscript_build.crate_root = build_src;

                // Set dependencies for the build script
                for dep in node.deps {
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
                                .insert(format!("{}{}:{}", dep_prefix, dep_name, dep_name));
                        }
                    }
                }
                buck_rules.push(Rule::CargoRustBinary(buildscript_build));

                // create the build script run rule
                let buildscript_run = BuildscriptRun {
                    name: format!("{}-build-script-run", buckal_name),
                    package_name: package.name.to_string(),
                    buildscript_rule: format!(":{}-{}", buckal_name, build_target.name),
                    features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
                    version: package.version.to_string(),
                    ..Default::default()
                };
                buck_rules.push(Rule::BuildscriptRun(buildscript_run));
            } else {
                // No build script, all rules done!
                buck_rules.push(Rule::CargoRustLibrary(rust_library));
            }

            // generate the BUCK file
            let mut buck_file = buck_rules
                .iter()
                .map(serde_starlark::to_string)
                .map(Result::unwrap)
                .join("\n");

            buck_file.insert_str(0, "# @generated by `cargo buck`\n\n");
            std::fs::write(&buck_path, buck_file).expect("Failed to write BUCK file");
        }
    }
}
