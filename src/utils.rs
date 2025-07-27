use std::process::Command;

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
