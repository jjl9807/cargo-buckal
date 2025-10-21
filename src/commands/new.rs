use std::{
    fs::OpenOptions,
    io::Write,
    process::{Command, Stdio, exit},
};

use clap::Parser;
use ini::Ini;

use crate::{
    RUST_CRATES_ROOT,
    buck2::Buck2Command,
    buckal_error, buckal_log, buckal_note, extract_bundles,
    utils::{UnwrapOrExit, ensure_prerequisites},
};

#[derive(Parser, Debug)]
pub struct NewArgs {
    pub path: String,
    #[arg(long, default_value = "false")]
    pub bin: bool,
    #[arg(long, default_value = "false")]
    pub lib: bool,
    #[arg(long)]
    pub edition: Option<String>,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long, default_value = "false", conflicts_with_all = ["bin", "lib", "edition", "name"])]
    pub repo: bool,
    #[arg(long, default_value = "false", conflicts_with = "repo")]
    pub lite: bool,
}

pub fn execute(args: &NewArgs) {
    // Ensure all prerequisites are installed before proceeding
    ensure_prerequisites().unwrap_or_exit();

    // Use `cargo new` to initialize the directory
    let mut cargo_cmd = Command::new("cargo");
    cargo_cmd.arg("new").arg(&args.path);
    if args.bin {
        cargo_cmd.arg("--bin");
    }
    if args.lib {
        cargo_cmd.arg("--lib");
    }
    if let Some(edition) = &args.edition {
        cargo_cmd.arg("--edition").arg(edition);
    }
    if let Some(name) = &args.name {
        cargo_cmd.arg("--name").arg(name);
    }

    // Suppress output if `--repo` is set
    if args.repo {
        cargo_cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }

    // execute the cargo command
    let status = cargo_cmd
        .status()
        .unwrap_or_exit_ctx("failed to execute `cargo new`");
    if !status.success() {
        buckal_error!("failed to initialize directory");
        exit(1);
    }

    // If `--repo` is set, remove the generated `src` directory and `Cargo.toml`
    if args.repo {
        buckal_log!(
            "Creating",
            format!("buck2 repository named `{}`", args.path)
        );
        std::fs::remove_dir_all(format!("{}/src", args.path)).unwrap_or_exit();
        std::fs::remove_file(format!("{}/Cargo.toml", args.path)).unwrap_or_exit();
        buckal_note!(
            "You should manually configure a Cargo workspace before running `cargo buckal new <path>` to create packages."
        );
    }

    if args.repo || args.lite {
        // Init a new buck2 repo
        Buck2Command::init()
            .arg(&args.path)
            .execute()
            .unwrap_or_exit();
        std::fs::create_dir_all(format!("{}/{}", args.path, RUST_CRATES_ROOT))
            .unwrap_or_exit_ctx("failed to create third-party directory");
        let mut git_ignore = OpenOptions::new()
            .create(false)
            .append(true)
            .open(format!("{}/.gitignore", args.path))
            .unwrap_or_exit_ctx("failed to open `.gitignore` file");
        writeln!(git_ignore, "/buck-out")
            .unwrap_or_exit_ctx("failed to write to `.gitignore` file");
        writeln!(git_ignore, "/.buckal").unwrap_or_exit_ctx("failed to write to `.gitignore` file");

        // Configure the buckal cell in .buckconfig
        let cwd =
            std::env::current_dir().unwrap_or_exit_ctx("failed to get current working directory");
        let repo_path = cwd.join(&args.path);
        let mut buck_config = Ini::load_from_file(repo_path.join(".buckconfig"))
            .unwrap_or_exit_ctx("failed to parse .buckconfig");
        let cells = buck_config.section_mut(Some("cells")).unwrap();
        cells.insert("buckal", "buckal");
        buck_config
            .write_to_file(repo_path.join(".buckconfig"))
            .unwrap_or_exit_ctx("failed to write to .buckconfig file");

        // Extract bundled prelude files
        extract_bundles(&repo_path).unwrap_or_exit_ctx("failed to extract bundled files");
    } else {
        // Create a new buck2 cell
        let _buck = std::fs::File::create(format!("{}/BUCK", args.path))
            .unwrap_or_exit_ctx("failed to create `BUCK` file");
    }
}
