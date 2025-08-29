use std::{io, process::Command, str::FromStr};

use cargo_platform::Cfg;
use colored::Colorize;
use inquire::Select;

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
    
    let options = vec![
        "🚀 Install automatically (recommended)",
        "📖 Exit and show manual installation guide",
    ];
    
    let ans = Select::new("How would you like to install Buck2?", options)
        .prompt()
        .map_err(|e| io::Error::other(format!("Selection error: {}", e)))?;
    
    match ans {
        "🚀 Install automatically (recommended)" => {
            println!();
            println!("{} {}", "🚀".green(), "Installing Buck2 automatically...".green());
            
            if let Err(e) = install_buck2_automatically() {
                println!("{} {}: {}", "❌".red(), "Installation failed".red(), e);
                println!();
                show_manual_installation();
                return Ok(false);
            }
            
            println!("{} {}", "✅".green(), "Buck2 installation completed!".green());
            println!("{} {}", "🔍".blue(), "Verifying installation...".blue());
            
            // Check if installation was successful
            if check_buck2_installed() {
                println!("{} {}", "🎉".green(), "Buck2 is now available!".green());
                return Ok(true);
            } else {
                println!("{} {}", "⚠️".yellow(), "Buck2 installation completed but not found in PATH.".yellow());
                println!("{} {}", "💡".bright_blue(), "You may need to restart your terminal or source your shell profile.".bright_blue());
                return Ok(false);
            }
        }
        "📖 Exit and show manual installation guide" => {
            show_manual_installation();
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn install_buck2_automatically() -> io::Result<()> {
    println!("{} {}", "📦".cyan(), "Installing Rust nightly...".cyan());
    let status = Command::new("rustup")
        .args(["install", "nightly-2025-06-20"])
        .status()?;
    
    if !status.success() {
        return Err(io::Error::other("Failed to install Rust nightly"));
    }
    
    println!("{} {}", "📦".cyan(), "Installing Buck2 from GitHub...".cyan());
    let status = Command::new("cargo")
        .args([
            "+nightly-2025-06-20",
            "install",
            "--git",
            "https://github.com/facebook/buck2.git",
            "buck2"
        ])
        .status()?;
    
    if !status.success() {
        return Err(io::Error::other("Failed to install Buck2"));
    }
    
    Ok(())
}

fn show_manual_installation() {
    println!();
    println!("{} {}", "📖".green(), "Manual Buck2 Installation Guide".green().bold());
    println!();
    
    println!("{}", "Choose one of the following installation methods:".bright_magenta());
    println!();
    
    // Method 1: Cargo install
    println!("{}", "Method 1: Install via Cargo (Recommended)".cyan().bold());
    println!("{}", "1. Install Rust nightly (prerequisite)".cyan());
    println!("   {}", "rustup install nightly-2025-06-20".bright_white());
    println!();
    println!("{}", "2. Install Buck2 from GitHub".cyan());
    println!("   {}", "cargo +nightly-2025-06-20 install --git https://github.com/facebook/buck2.git buck2".bright_white());
    println!();
    println!("{}", "3. Add to your PATH (if not already)".cyan());
    println!("   {}", "# Add to your shell profile (~/.bashrc, ~/.zshrc, etc.)".bright_black());
    println!("   {}", "Linux/macOS:".bright_black());
    println!("   {}", "export PATH=$HOME/.cargo/bin:$PATH".bright_white());
    println!("   {}", "Windows PowerShell:".bright_black());
    println!("   {}", "$Env:PATH += \";$HOME\\.cargo\\bin\"".bright_white());
    println!();
    
    println!("{}", "─".repeat(60).bright_black());
    println!();
    
    // Method 2: Direct download
    println!("{}", "Method 2: Download Pre-built Binary".yellow().bold());
    println!("{}", "1. Download from GitHub releases".yellow());
    println!("   {}", "https://github.com/facebook/buck2/releases/tag/latest".bright_white().underline());
    println!();
    println!("{}", "2. Extract and place in your PATH".yellow());
    println!("   {}", "# Extract the downloaded file and move to a directory in your PATH".bright_black());
    println!("   {}", "# For example: /usr/local/bin (Linux/macOS) or C:\\bin (Windows)".bright_black());
    println!();
    
    println!("{}", "─".repeat(60).bright_black());
    println!();
    
    // Verification
    println!("{} {}", "✅".green(), "Verify Installation".green().bold());
    println!("   {}", "buck2 --help".bright_white());
    println!();
    
    println!("{} {}", "💡".bright_blue(), "Note: After installation, restart your terminal or source your shell profile.".bright_blue());
    println!();
    
    println!(
        "{} {}",
        "📚".bright_cyan(),
        "For detailed instructions and troubleshooting, refer to:".bright_cyan()
    );
    println!(
        "   {}",
        "https://buck2.build/docs/getting_started/install/"
            .cyan()
            .underline()
    );
    println!();
    
    println!("{} {}", "🔄".yellow(), "Once Buck2 is installed, run your cargo buckal command again.".yellow());
    println!();
}

pub fn ensure_buck2_installed() -> io::Result<()> {
    if !check_buck2_installed() {
        let installed = prompt_buck2_installation()?;
        if !installed {
            return Err(io::Error::other(
                "Buck2 is required but not installed. Please install Buck2 and try again.",
            ));
        }
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
