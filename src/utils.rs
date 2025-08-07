use std::{process::Command, str::FromStr};

use cargo_platform::Cfg;

pub fn get_buck2_root() -> String {
    // This function should return the root directory of the Buck2 project.
    let output = Command::new("buck2")
        .arg("root")
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        String::new()
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
