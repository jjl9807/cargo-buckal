use std::process::Command;
use std::str;

fn main() {
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            str::from_utf8(&output.stdout)
                .ok()
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    let git_date = Command::new("git")
        .args(["log", "-1", "--format=%ad", "--date=short"])
        .output()
        .ok()
        .and_then(|output| {
            str::from_utf8(&output.stdout)
                .ok()
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=COMMIT_DATE={}", git_date);
}
