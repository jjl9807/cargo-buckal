use std::{io, process::Command, str::FromStr};

use cargo_platform::Cfg;
use colored::Colorize;

use crate::buck2::Buck2Command;

pub fn check_buck2_installed() -> bool {
    Buck2Command::new()
        .arg("--help")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn prompt_buck2_installation() -> io::Result<bool> {
    println!();
    println!(
        "{} {}",
        "⚠️".yellow(),
        "Buck2 is not installed or not found in PATH.".yellow()
    );
    println!(
        "{} {}",
        "🔧".blue(),
        "Buck2 is required to use cargo buckal.".blue()
    );
    println!();
    println!(
        "{} {}",
        "📚".green(),
        "Please install Buck2 by following the official installation guide:".green()
    );
    println!(
        "   {}",
        "https://buck2.build/docs/getting_started/install/"
            .cyan()
            .underline()
    );
    println!();
    println!(
        "{} {}",
        "🔄".bright_blue(),
        "After installation, please run this command again.".bright_blue()
    );
    println!();

    Ok(false) // Always return false since we're not installing automatically
}

pub fn ensure_buck2_installed() -> io::Result<()> {
    if !check_buck2_installed() {
        prompt_buck2_installation()?;
        return Err(io::Error::other(
            "Buck2 is required but not installed. Please install Buck2 and try again.",
        ));
    }
    Ok(())
}

pub fn get_buck2_root() -> String {
    // This function should return the root directory of the Buck2 project.
    match Buck2Command::root().output() {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        Ok(output) => {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            String::new()
        }
        Err(e) => {
            eprintln!("Failed to execute buck2 command: {}", e);
            String::new()
        }
    }
}

pub fn check_buck2_package() {
    // This function checks if the current directory is a valid Buck2 package.
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let buck_file = cwd.join("BUCK");
    assert!(
        buck_file.exists(),
        "{}",
        format!("error: could not find `BUCK` in `{}`", cwd.display())
    );
}

pub fn get_target() -> String {
    let output = Command::new("rustc")
        .arg("-Vv")
        .output()
        .expect("rustc failed to run");
    let stdout = String::from_utf8(output.stdout).unwrap();
    for line in stdout.lines() {
        if let Some(line) = line.strip_prefix("host: ") {
            return String::from(line);
        }
    }
    panic!("Failed to find host: {stdout}");
}

pub fn get_cfgs() -> Vec<Cfg> {
    let output = Command::new("rustc")
        .arg("--print=cfg")
        .output()
        .expect("rustc failed to run");
    let stdout = String::from_utf8(output.stdout).unwrap();
    stdout
        .lines()
        .map(|line| Cfg::from_str(line).unwrap())
        .collect()
}
