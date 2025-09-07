use std::{
    collections::{BTreeMap as Map, BTreeSet as Set, HashMap},
    path::PathBuf,
    vec,
};

use cargo_metadata::{
    DepKindInfo, DependencyKind, Node, Package, PackageId, Target, camino::Utf8PathBuf,
};
use fs_extra::dir::{CopyOptions, copy};
use itertools::Itertools;
use serde_json::Value;

use crate::{
    RUST_CRATES_ROOT,
    buck::{BuildscriptRun, CargoRule, CargoRustBinary, CargoRustLibrary, Glob, Load, Rule},
    buck2::Buck2Command,
    utils::{get_buck2_root, get_cfgs, get_target},
};

pub fn buckify_dep_node(node: &Node, packages_map: &HashMap<PackageId, Package>) -> Vec<Rule> {
    let package = packages_map.get(&node.id).unwrap().to_owned();
    let buckal_name = format!("{}-{}", package.name, package.version);

    // emit buck rules for lib target
    let mut buck_rules: Vec<Rule> = Vec::new();

    let manifest_dir = package.manifest_path.parent().unwrap().to_owned();
    let lib_target = package
        .targets
        .iter()
        .find(|t| {
            t.kind.contains(&cargo_metadata::TargetKind::Lib)
                || t.kind.contains(&cargo_metadata::TargetKind::CDyLib)
                || t.kind.contains(&cargo_metadata::TargetKind::DyLib)
                || t.kind.contains(&cargo_metadata::TargetKind::RLib)
                || t.kind.contains(&cargo_metadata::TargetKind::StaticLib)
                || t.kind.contains(&cargo_metadata::TargetKind::ProcMacro)
        })
        .expect("No library target found");

    let rust_library = emit_rust_library(
        &package,
        node,
        packages_map,
        lib_target,
        &manifest_dir,
        &buckal_name,
    );

    buck_rules.push(Rule::CargoRustLibrary(rust_library));

    // Check if the package has a build script
    let custom_build_target = package
        .targets
        .iter()
        .find(|t| t.kind.contains(&cargo_metadata::TargetKind::CustomBuild));

    if let Some(build_target) = custom_build_target {
        // create the build script rule
        let buildscript_build = emit_buildscript_build(
            buck_rules.last_mut().unwrap().as_cargo_rule_mut().unwrap(),
            build_target,
            &package,
            node,
            packages_map,
            &buckal_name,
            &manifest_dir,
        );
        buck_rules.push(Rule::CargoRustBinary(buildscript_build));

        // create the build script run rule
        let buildscript_run = emit_buildscript_run(&package, node, &buckal_name, build_target);
        buck_rules.push(Rule::BuildscriptRun(buildscript_run));
    }

    buck_rules
}

pub fn buckify_root_node(node: &Node, packages_map: &HashMap<PackageId, Package>) -> Vec<Rule> {
    let package = packages_map.get(&node.id).unwrap().to_owned();

    let bin_targets = package
        .targets
        .iter()
        .filter(|t| t.kind.contains(&cargo_metadata::TargetKind::Bin))
        .collect::<Vec<_>>();

    let lib_targets = package
        .targets
        .iter()
        .filter(|t| {
            t.kind.contains(&cargo_metadata::TargetKind::Lib)
                || t.kind.contains(&cargo_metadata::TargetKind::CDyLib)
                || t.kind.contains(&cargo_metadata::TargetKind::DyLib)
                || t.kind.contains(&cargo_metadata::TargetKind::RLib)
                || t.kind.contains(&cargo_metadata::TargetKind::StaticLib)
                || t.kind.contains(&cargo_metadata::TargetKind::ProcMacro)
        })
        .collect::<Vec<_>>();

    let mut buck_rules: Vec<Rule> = Vec::new();

    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let manifest_dir = Utf8PathBuf::from(cwd.to_str().unwrap());

    // emit buck rules for bin targets
    for bin_target in &bin_targets {
        let buckal_name = bin_target.name.to_owned();

        let rust_binary = emit_rust_binary(
            &package,
            node,
            packages_map,
            bin_target,
            &manifest_dir,
            &buckal_name,
        );

        buck_rules.push(Rule::CargoRustBinary(rust_binary));

        // Check if the package has a build script
        let custom_build_target = package
            .targets
            .iter()
            .find(|t| t.kind.contains(&cargo_metadata::TargetKind::CustomBuild));

        if let Some(build_target) = custom_build_target {
            // create the build script rule
            let buildscript_build = emit_buildscript_build(
                buck_rules.last_mut().unwrap().as_cargo_rule_mut().unwrap(),
                build_target,
                &package,
                node,
                packages_map,
                &buckal_name,
                &manifest_dir,
            );
            buck_rules.push(Rule::CargoRustBinary(buildscript_build));

            // create the build script run rule
            let buildscript_run = emit_buildscript_run(&package, node, &buckal_name, build_target);
            buck_rules.push(Rule::BuildscriptRun(buildscript_run));
        }
    }

    // emit buck rules for lib targets
    for lib_target in lib_targets {
        let buckal_name = if bin_targets.iter().any(|b| b.name == lib_target.name) {
            format!("lib{}", lib_target.name)
        } else {
            lib_target.name.to_owned()
        };

        let rust_library = emit_rust_library(
            &package,
            node,
            packages_map,
            lib_target,
            &manifest_dir,
            &buckal_name,
        );

        buck_rules.push(Rule::CargoRustLibrary(rust_library));

        // Check if the package has a build script
        let custom_build_target = package
            .targets
            .iter()
            .find(|t| t.kind.contains(&cargo_metadata::TargetKind::CustomBuild));

        if let Some(build_target) = custom_build_target {
            // create the build script build rule
            let buildscript_build = emit_buildscript_build(
                buck_rules.last_mut().unwrap().as_cargo_rule_mut().unwrap(),
                build_target,
                &package,
                node,
                packages_map,
                &buckal_name,
                &manifest_dir,
            );
            buck_rules.push(Rule::CargoRustBinary(buildscript_build));

            // create the build script run rule
            let buildscript_run = emit_buildscript_run(&package, node, &buckal_name, build_target);
            buck_rules.push(Rule::BuildscriptRun(buildscript_run));
        }
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
    cargo_env.insert("CARGO_PKG_AUTHORS".to_owned(), package.authors.join(":"));
    cargo_env.insert(
        "CARGO_PKG_DESCRIPTION".to_owned(),
        package.description.clone().unwrap_or_default(),
    );
    cargo_env.insert("CARGO_PKG_NAME".to_owned(), package.name.to_string());
    cargo_env.insert(
        "CARGO_PKG_REPOSITORY".to_owned(),
        package.repository.clone().unwrap_or_default(),
    );
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
    let loads: Vec<Rule> = vec![
        Rule::Load(Load {
            bzl: "@prelude//rust:cargo_buildscript.bzl".to_owned(),
            items: Set::from(["buildscript_run".to_owned()]),
        }),
        Rule::Load(Load {
            bzl: "@prelude//rust:cargo_package.bzl".to_owned(),
            items: Set::from(["cargo".to_owned()]),
        }),
    ];

    let loads_string = loads
        .iter()
        .map(serde_starlark::to_string)
        .map(Result::unwrap)
        .join("");

    let mut content = rules
        .iter()
        .map(serde_starlark::to_string)
        .map(Result::unwrap)
        .join("\n");

    content.insert(0, '\n');
    content.insert_str(0, &loads_string);
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

fn set_deps(
    rust_rule: &mut dyn CargoRule,
    node: &Node,
    packages_map: &HashMap<PackageId, Package>,
    is_build_script: bool,
) {
    for dep in &node.deps {
        if let Some(dep_package) = packages_map.get(&dep.pkg) {
            let dep_name = format!("{}-{}", dep_package.name, dep_package.version);
            if dep.dep_kinds.iter().any(|dk| {
                (dk.kind == DependencyKind::Normal
                    || is_build_script && dk.kind == DependencyKind::Build)
                    && check_dep_target(dk)
            }) {
                // Normal dependencies and build dependencies for `build.rs` on current arch
                if dep_package.source.is_none() {
                    // first-party dependency
                    let buck2_root = get_buck2_root();
                    if buck2_root.is_empty() {
                        return;
                    }
                    let buck2_root = PathBuf::from(buck2_root.trim());
                    let manifest_path = PathBuf::from(&dep_package.manifest_path);
                    let manifest_dir = manifest_path.parent().unwrap();
                    let relative = manifest_dir.strip_prefix(&buck2_root).ok();

                    if relative.is_none() {
                        eprintln!("error: Current directory is not inside the Buck2 project root.");
                        return;
                    }
                    let mut relative_path = relative.unwrap().to_string_lossy().into_owned();

                    if !relative_path.is_empty() {
                        relative_path += "/";
                    }

                    let target = format!("//{relative_path}...");

                    match Buck2Command::targets().arg(target).arg("--json").output() {
                        Ok(output) if output.status.success() => {
                            let json_str = String::from_utf8_lossy(&output.stdout);
                            let targets: Vec<Value> = serde_json::from_str(&json_str).unwrap();
                            let target_item = targets
                                .iter()
                                .find(|t| {
                                    t.get("buck.type")
                                        .and_then(|k| k.as_str())
                                        .map_or(false, |k| k.ends_with("rust_library"))
                                })
                                .expect("Failed to find rust library rule in BUCK file");
                            let buck_package = target_item
                                .get("buck.package")
                                .and_then(|n| n.as_str())
                                .expect("Failed to get target name")
                                .strip_prefix("root")
                                .unwrap();
                            let buck_name = target_item
                                .get("name")
                                .and_then(|n| n.as_str())
                                .expect("Failed to get target name");
                            rust_rule
                                .deps_mut()
                                .insert(format!("{buck_package}:{buck_name}"));
                        }
                        Ok(output) => {
                            panic!("{}", String::from_utf8_lossy(&output.stderr));
                        }
                        Err(e) => {
                            panic!("Failed to execute buck2 command: {}", e);
                        }
                    }
                } else {
                    // third-party dependency
                    rust_rule
                        .deps_mut()
                        .insert(format!("//{RUST_CRATES_ROOT}/{dep_name}:{dep_name}"));
                }
            }
        }
    }
}

/// Emit `cargo.rust_library` rule for the given lib target
fn emit_rust_library(
    package: &Package,
    node: &Node,
    packages_map: &HashMap<PackageId, Package>,
    lib_target: &Target,
    manifest_dir: &Utf8PathBuf,
    buckal_name: &String,
) -> CargoRustLibrary {
    let mut rust_library = CargoRustLibrary {
        name: buckal_name.clone(),
        srcs: Glob {
            include: Set::from(["**/**".to_owned()]),
            exclude: Set::from(["BUCK".to_owned()]),
        },
        crate_name: package.name.to_string(),
        edition: package.edition.to_string(),
        env: gen_cargo_env(&package),
        features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
        visibility: Set::from(["PUBLIC".to_owned()]),
        ..Default::default()
    };

    if lib_target
        .kind
        .contains(&cargo_metadata::TargetKind::ProcMacro)
    {
        rust_library.proc_macro = Some(true);
    }

    // Set the crate root path
    rust_library.crate_root = lib_target
        .src_path
        .to_owned()
        .strip_prefix(&manifest_dir)
        .expect("Failed to get library source path")
        .to_string();

    // Set dependencies
    set_deps(&mut rust_library, node, packages_map, false);

    rust_library
}

/// Emit `cargo.rust_binary` rule for the given bin target
fn emit_rust_binary(
    package: &Package,
    node: &Node,
    packages_map: &HashMap<PackageId, Package>,
    bin_target: &Target,
    manifest_dir: &Utf8PathBuf,
    buckal_name: &String,
) -> CargoRustBinary {
    let mut rust_binary = CargoRustBinary {
        name: buckal_name.clone(),
        srcs: Glob {
            include: Set::from(["**/**".to_owned()]),
            exclude: Set::from(["BUCK".to_owned()]),
        },
        crate_name: package.name.to_string(),
        edition: package.edition.to_string(),
        env: gen_cargo_env(&package),
        features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
        visibility: Set::from(["PUBLIC".to_owned()]),
        ..Default::default()
    };

    // Set the crate root path
    rust_binary.crate_root = bin_target
        .src_path
        .to_owned()
        .strip_prefix(&manifest_dir)
        .expect("Failed to get binary source path")
        .to_string();

    // Set dependencies
    set_deps(&mut rust_binary, node, packages_map, false);

    rust_binary
}

/// Emit `buildscript_build` rule for the given build target
fn emit_buildscript_build(
    rust_rule: &mut dyn CargoRule,
    build_target: &Target,
    package: &Package,
    node: &Node,
    packages_map: &HashMap<PackageId, Package>,
    buckal_name: &String,
    manifest_dir: &Utf8PathBuf,
) -> CargoRustBinary {
    // process the build script in rust_library
    rust_rule.env_mut().insert(
        "OUT_DIR".to_owned(),
        format!("$(location :{buckal_name}-build-script-run[out_dir])").to_owned(),
    );
    rust_rule
        .rustc_flags_mut()
        .insert(format!("@$(location :{buckal_name}-build-script-run[rustc_flags])").to_owned());

    // create the build script rule
    let mut buildscript_build = CargoRustBinary {
        name: format!("{}-{}", buckal_name, build_target.name),
        srcs: Glob {
            include: Set::from(["**/**".to_owned()]),
            exclude: Set::from(["BUCK".to_owned()]),
        },
        crate_name: build_target.name.to_owned(),
        edition: package.edition.to_string(),
        env: gen_cargo_env(&package),
        features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
        ..Default::default()
    };

    // Set the crate root path for the build script
    buildscript_build.crate_root = build_target
        .src_path
        .to_owned()
        .strip_prefix(&manifest_dir)
        .expect("Failed to get library source path")
        .to_string();

    // Set dependencies for the build script
    set_deps(&mut buildscript_build, node, packages_map, true);

    buildscript_build
}

/// Emit `buildscript_run` rule for the given build target
fn emit_buildscript_run(
    package: &Package,
    node: &Node,
    buckal_name: &String,
    build_target: &Target,
) -> BuildscriptRun {
    // create the build script run rule
    BuildscriptRun {
        name: format!("{buckal_name}-build-script-run"),
        package_name: package.name.to_string(),
        buildscript_rule: format!(":{}-{}", buckal_name, build_target.name),
        features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
        version: package.version.to_string(),
        local_manifest_dir: "**".to_owned(),
        ..Default::default()
    }
}
