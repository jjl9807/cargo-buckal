use std::{
    borrow::Cow,
    collections::{BTreeSet as Set, HashMap},
    path::PathBuf,
    vec,
};

use cargo_metadata::{
    DepKindInfo, DependencyKind, Node, Package, PackageId, Target, camino::Utf8PathBuf,
};
use itertools::Itertools;
use regex::Regex;
use serde_json::Value;

use crate::{
    RUST_CRATES_ROOT,
    buck::{
        BuildscriptRun, CargoManifest, FileGroup, Glob, HttpArchive, Load, Rule, RustBinary,
        RustLibrary, RustRule, parse_buck_file, patch_buck_rules,
    },
    buck2::Buck2Command,
    buckal_log,
    cache::{BuckalChange, ChangeType},
    context::BuckalContext,
    utils::{UnwrapOrExit, get_buck2_root, get_cfgs, get_target, get_vendor_dir},
};

pub fn buckify_dep_node(node: &Node, ctx: &BuckalContext) -> Vec<Rule> {
    let package = ctx.packages_map.get(&node.id).unwrap().to_owned();

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

    let http_archive = emit_http_archive(&package, ctx);
    buck_rules.push(Rule::HttpArchive(http_archive));

    let cargo_manifest = emit_cargo_manifest(&package);
    buck_rules.push(Rule::CargoManifest(cargo_manifest));

    let rust_library = emit_rust_library(
        &package,
        node,
        &ctx.packages_map,
        lib_target,
        &manifest_dir,
        &package.name,
    );

    buck_rules.push(Rule::RustLibrary(rust_library));

    // Check if the package has a build script
    let custom_build_target = package
        .targets
        .iter()
        .find(|t| t.kind.contains(&cargo_metadata::TargetKind::CustomBuild));

    if let Some(build_target) = custom_build_target {
        // Patch the rust_library rule to support build scripts
        for rule in &mut buck_rules {
            if let Some(rust_rule) = rule.as_rust_rule_mut() {
                patch_with_buildscript(rust_rule, build_target, &package);
            }
        }

        // create the build script rule
        let buildscript_build = emit_buildscript_build(
            build_target,
            &package,
            node,
            &ctx.packages_map,
            &manifest_dir,
        );
        buck_rules.push(Rule::RustBinary(buildscript_build));

        // create the build script run rule
        let buildscript_run = emit_buildscript_run(&package, node, &ctx.packages_map, build_target);
        buck_rules.push(Rule::BuildscriptRun(buildscript_run));
    }

    buck_rules
}

pub fn buckify_root_node(node: &Node, ctx: &BuckalContext) -> Vec<Rule> {
    let package = ctx.packages_map.get(&node.id).unwrap().to_owned();

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

    let manifest_dir = package.manifest_path.parent().unwrap().to_owned();

    // emit filegroup rule for vendor
    let filegroup = emit_filegroup(&package);
    buck_rules.push(Rule::FileGroup(filegroup));

    let cargo_manifest = emit_cargo_manifest(&package);
    buck_rules.push(Rule::CargoManifest(cargo_manifest));

    // emit buck rules for bin targets
    for bin_target in &bin_targets {
        let buckal_name = bin_target.name.to_owned();

        let mut rust_binary = emit_rust_binary(
            &package,
            node,
            &ctx.packages_map,
            bin_target,
            &manifest_dir,
            &buckal_name,
        );

        if lib_targets.iter().any(|l| l.name == bin_target.name) {
            // Cargo allows `main.rs` to use items from `lib.rs` via the crate's own name by default.
            rust_binary.deps_mut().insert(format!(":lib{}", bin_target.name));
        }

        buck_rules.push(Rule::RustBinary(rust_binary));
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
            &ctx.packages_map,
            lib_target,
            &manifest_dir,
            &buckal_name,
        );

        buck_rules.push(Rule::RustLibrary(rust_library));
    }

    // Check if the package has a build script
    let custom_build_target = package
        .targets
        .iter()
        .find(|t| t.kind.contains(&cargo_metadata::TargetKind::CustomBuild));

    if let Some(build_target) = custom_build_target {
        // Patch the rust_library and rust_binary rules to support build scripts
        for rule in &mut buck_rules {
            if let Some(rust_rule) = rule.as_rust_rule_mut() {
                patch_with_buildscript(rust_rule, build_target, &package);
            }
        }

        // create the build script rule
        let buildscript_build = emit_buildscript_build(
            build_target,
            &package,
            node,
            &ctx.packages_map,
            &manifest_dir,
        );
        buck_rules.push(Rule::RustBinary(buildscript_build));

        // create the build script run rule
        let buildscript_run = emit_buildscript_run(&package, node, &ctx.packages_map, build_target);
        buck_rules.push(Rule::BuildscriptRun(buildscript_run));
    }

    buck_rules
}

pub fn vendor_package(package: &Package) -> Utf8PathBuf {
    // Vendor the package sources to `third-party/rust/crates/<package_name>/<version>`
    let vendor_dir = get_vendor_dir(&package.name, &package.version.to_string())
        .unwrap_or_exit_ctx("failed to get vendor directory");
    if !vendor_dir.exists() {
        std::fs::create_dir_all(&vendor_dir).expect("Failed to create target directory");
    }

    vendor_dir
}

pub fn gen_buck_content(rules: &[Rule]) -> String {
    let loads: Vec<Rule> = vec![
        Rule::Load(Load {
            bzl: "@buckal//:cargo_manifest.bzl".to_owned(),
            items: Set::from(["cargo_manifest".to_owned()]),
        }),
        Rule::Load(Load {
            bzl: "@buckal//:wrapper.bzl".to_owned(),
            items: Set::from([
                "buildscript_run".to_owned(),
                "rust_binary".to_owned(),
                "rust_library".to_owned(),
            ]),
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
    rust_rule: &mut dyn RustRule,
    node: &Node,
    packages_map: &HashMap<PackageId, Package>,
    is_build_script: bool,
) {
    for dep in &node.deps {
        if let Some(dep_package) = packages_map.get(&dep.pkg) {
            let dep_package_name = dep_package.name.to_string();
            if dep.dep_kinds.iter().any(|dk| {
                (!is_build_script && dk.kind == DependencyKind::Normal
                    || is_build_script && dk.kind == DependencyKind::Build)
                    && check_dep_target(dk)
            }) {
                // Normal dependencies and build dependencies for `build.rs` on current arch
                if dep_package.source.is_none() {
                    // first-party dependency
                    let buck2_root =
                        get_buck2_root().unwrap_or_exit_ctx("failed to get buck2 root");
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
                                        .is_some_and(|k| k.ends_with("rust_library"))
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

                            if dep.name != dep_package_name.replace("-", "_") {
                                // renamed dependency
                                rust_rule.named_deps_mut().insert(
                                    dep.name.clone(),
                                    format!("{buck_package}:{buck_name}"),
                                );
                            } else {
                                rust_rule
                                    .deps_mut()
                                    .insert(format!("{buck_package}:{buck_name}"));
                            }
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
                    if dep.name != dep_package_name.replace("-", "_") {
                        // renamed dependency
                        rust_rule.named_deps_mut().insert(
                            dep.name.clone(),
                            format!(
                                "//{RUST_CRATES_ROOT}/{}/{}:{}",
                                dep_package.name, dep_package.version, dep_package.name
                            ),
                        );
                    } else {
                        rust_rule.deps_mut().insert(format!(
                            "//{RUST_CRATES_ROOT}/{}/{}:{}",
                            dep_package.name, dep_package.version, dep_package.name
                        ));
                    }
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
    buckal_name: &str,
) -> RustLibrary {
    let mut rust_library = RustLibrary {
        name: buckal_name.to_owned(),
        srcs: Set::from([get_vendor_target(package)]),
        crate_name: lib_target.name.to_owned().replace("-", "_"),
        edition: package.edition.to_string(),
        features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
        rustc_flags: Set::from([format!(
            "@$(location :{}-manifest[env_flags])",
            package.name
        )]),
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
    rust_library.crate_root = format!(
        "vendor/{}",
        lib_target
            .src_path
            .to_owned()
            .strip_prefix(manifest_dir)
            .expect("Failed to get library source path")
    );

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
    buckal_name: &str,
) -> RustBinary {
    let mut rust_binary = RustBinary {
        name: buckal_name.to_owned(),
        srcs: Set::from([get_vendor_target(package)]),
        crate_name: bin_target.name.to_owned().replace("-", "_"),
        edition: package.edition.to_string(),
        features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
        rustc_flags: Set::from([format!(
            "@$(location :{}-manifest[env_flags])",
            package.name
        )]),
        visibility: Set::from(["PUBLIC".to_owned()]),
        ..Default::default()
    };

    // Set the crate root path
    rust_binary.crate_root = format!(
        "vendor/{}",
        bin_target
            .src_path
            .to_owned()
            .strip_prefix(manifest_dir)
            .expect("Failed to get binary source path")
    );

    // Set dependencies
    set_deps(&mut rust_binary, node, packages_map, false);

    rust_binary
}

/// Emit `buildscript_build` rule for the given build target
fn emit_buildscript_build(
    build_target: &Target,
    package: &Package,
    node: &Node,
    packages_map: &HashMap<PackageId, Package>,
    manifest_dir: &Utf8PathBuf,
) -> RustBinary {
    // create the build script rule
    let mut buildscript_build = RustBinary {
        name: format!("{}-{}", package.name, build_target.name),
        srcs: Set::from([get_vendor_target(package)]),
        crate_name: build_target.name.to_owned().replace("-", "_"),
        edition: package.edition.to_string(),
        features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
        rustc_flags: Set::from([format!(
            "@$(location :{}-manifest[env_flags])",
            package.name
        )]),
        ..Default::default()
    };

    // Set the crate root path for the build script
    buildscript_build.crate_root = format!(
        "vendor/{}",
        build_target
            .src_path
            .to_owned()
            .strip_prefix(manifest_dir)
            .expect("Failed to get library source path")
    );

    // Set dependencies for the build script
    set_deps(&mut buildscript_build, node, packages_map, true);

    buildscript_build
}

/// Emit `buildscript_run` rule for the given build target
fn emit_buildscript_run(
    package: &Package,
    node: &Node,
    packages_map: &HashMap<PackageId, Package>,
    build_target: &Target,
) -> BuildscriptRun {
    // create the build script run rule
    let build_name = get_build_name(&build_target.name);
    let mut buildscript_run = BuildscriptRun {
        name: format!("{}-{}-run", package.name, build_name),
        package_name: package.name.to_string(),
        buildscript_rule: format!(":{}-{}", package.name, build_target.name),
        env_srcs: Set::from([format!(":{}-manifest[env_dict]", package.name)]),
        features: Set::from_iter(node.features.iter().map(|f| f.to_string())),
        version: package.version.to_string(),
        manifest_dir: format!(":{}-vendor", package.name),
        visibility: Set::from(["PUBLIC".to_owned()]),
        ..Default::default()
    };

    // Set environment variables from dependencies
    // See https://doc.rust-lang.org/cargo/reference/build-scripts.html#the-links-manifest-key
    for dep in &node.deps {
        if let Some(dep_package) = packages_map.get(&dep.pkg)
            && dep_package.links.is_some()
            && dep
                .dep_kinds
                .iter()
                .any(|dk| dk.kind == DependencyKind::Normal && check_dep_target(dk))
        {
            // Only normal dependencies with The links Manifest Key for current arch are considered
            let custom_build_target_dep = dep_package
                .targets
                .iter()
                .find(|t| t.kind.contains(&cargo_metadata::TargetKind::CustomBuild));
            if let Some(build_target_dep) = custom_build_target_dep {
                let build_name_dep = get_build_name(&build_target_dep.name);
                buildscript_run.env_srcs.insert(format!(
                    "//{RUST_CRATES_ROOT}/{}/{}:{}-{build_name_dep}-run[metadata]",
                    dep_package.name, dep_package.version, dep_package.name
                ));
            } else {
                panic!(
                    "Dependency {} has links key but no build script target",
                    dep_package.name
                );
            }
        }
    }

    buildscript_run
}

/// Patch the given `rust_library` or `rust_binary` rule to support build scripts
fn patch_with_buildscript(rust_rule: &mut dyn RustRule, build_target: &Target, package: &Package) {
    let build_name = get_build_name(&build_target.name);
    rust_rule.env_mut().insert(
        "OUT_DIR".to_owned(),
        format!("$(location :{}-{build_name}-run[out_dir])", package.name).to_owned(),
    );
    rust_rule.rustc_flags_mut().insert(
        format!(
            "@$(location :{}-{build_name}-run[rustc_flags])",
            package.name
        )
        .to_owned(),
    );
}

/// Emit `http_archive` rule for the given package
fn emit_http_archive(package: &Package, ctx: &BuckalContext) -> HttpArchive {
    let vendor_name = format!("{}-vendor", package.name);
    let url = format!(
        "https://static.crates.io/crates/{}/{}-{}.crate",
        package.name, package.name, package.version
    );
    let buckal_name = format!("{}-{}", package.name, package.version);
    let checksum = ctx
        .checksums_map
        .get(&format!("{}-{}", package.name, package.version))
        .unwrap();

    HttpArchive {
        name: vendor_name,
        urls: Set::from([url]),
        sha256: checksum.to_string(),
        _type: "tar.gz".to_owned(),
        strip_prefix: buckal_name,
        out: Some("vendor".to_owned()),
    }
}

/// Emit `filegroup` rule for the given package
fn emit_filegroup(package: &Package) -> FileGroup {
    let vendor_name = format!("{}-vendor", package.name);
    FileGroup {
        name: vendor_name,
        srcs: Glob {
            include: Set::from(["**/**".to_owned()]),
            ..Default::default()
        },
        out: Some("vendor".to_owned()),
    }
}

// Emit `cargo_manifest` rule for the given package
fn emit_cargo_manifest(package: &Package) -> CargoManifest {
    CargoManifest {
        name: format!("{}-manifest", package.name),
        vendor: get_vendor_target(package),
    }
}

fn get_build_name(s: &str) -> Cow<'_, str> {
    if let Some(stripped) = s.strip_suffix("-build") {
        Cow::Owned(stripped.to_string())
    } else {
        Cow::Borrowed(s)
    }
}

fn get_vendor_target(package: &Package) -> String {
    format!(":{}-vendor", package.name)
}

impl BuckalChange {
    pub fn apply(&self, ctx: &BuckalContext) {
        // This function applies changes to the BUCK files of detected packages in the cache diff, but skips the root package.
        let re = Regex::new(r"^([^+#]+)\+([^#]+)#([^@]+)@([^+#]+)(?:\+(.+))?$")
            .expect("error creating regex");

        for (id, change_type) in &self.changes {
            match change_type {
                ChangeType::Added | ChangeType::Changed => {
                    // Skip root package
                    if id == &ctx.root.id {
                        continue;
                    }

                    if let Some(node) = ctx.nodes_map.get(id) {
                        let package = ctx.packages_map.get(id).unwrap();

                        if ctx.separate && package.source.is_none() {
                            // Skip first-party packages if `--separate` is set
                            continue;
                        }

                        buckal_log!(
                            if let ChangeType::Added = change_type {
                                "Adding"
                            } else {
                                "Flushing"
                            },
                            format!("{} v{}", package.name, package.version)
                        );

                        // Vendor package sources
                        let vendor_dir = if package.source.is_none() {
                            package.manifest_path.parent().unwrap().to_owned()
                        } else {
                            vendor_package(package)
                        };

                        // Generate BUCK rules
                        let mut buck_rules = if package.source.is_none() {
                            buckify_root_node(node, ctx)
                        } else {
                            buckify_dep_node(node, ctx)
                        };

                        // Patch BUCK Rules
                        let buck_path = vendor_dir.join("BUCK");
                        if buck_path.exists() {
                            // Skip merging manual changes if `--no-merge` is set
                            if !ctx.no_merge {
                                let existing_rules = parse_buck_file(&buck_path)
                                    .expect("Failed to parse existing BUCK file");
                                patch_buck_rules(&existing_rules, &mut buck_rules);
                            }
                        } else {
                            std::fs::File::create(&buck_path).expect("Failed to create BUCK file");
                        }

                        // Generate the BUCK file
                        let buck_content = gen_buck_content(&buck_rules);
                        std::fs::write(&buck_path, buck_content)
                            .expect("Failed to write BUCK file");
                    }
                }
                ChangeType::Removed => {
                    let caps = re.captures(&id.repr).expect("Failed to parse package ID");
                    let name = &caps[3];
                    let version = &caps[4];

                    buckal_log!("Removing", format!("{} v{}", name, version));
                    let vendor_dir = get_vendor_dir(name, version)
                        .unwrap_or_exit_ctx("failed to get vendor directory");
                    if vendor_dir.exists() {
                        std::fs::remove_dir_all(&vendor_dir)
                            .expect("Failed to remove vendor directory");
                    }
                    if let Some(package_dir) = vendor_dir.parent()
                        && package_dir.exists()
                        && package_dir.read_dir().unwrap().next().is_none()
                    {
                        std::fs::remove_dir_all(package_dir)
                            .expect("Failed to remove empty package directory");
                    }
                }
            }
        }
    }
}

pub fn flush_root(ctx: &BuckalContext) {
    buckal_log!(
        "Flushing",
        format!("{} v{}", ctx.root.name, ctx.root.version)
    );
    let root_node = ctx
        .nodes_map
        .get(&ctx.root.id)
        .expect("Root node not found");
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let buck_path = Utf8PathBuf::from(cwd.to_str().unwrap()).join("BUCK");

    // Generate BUCK rules
    let buck_rules = buckify_root_node(root_node, ctx);

    // Generate the BUCK file
    let buck_content = gen_buck_content(&buck_rules);
    std::fs::write(&buck_path, buck_content).expect("Failed to write BUCK file");
}
