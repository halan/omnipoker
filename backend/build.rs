use std::path::Path;
use std::process::Command;

fn main() {
    let frontend_dir = Path::new("../frontend");

    let status = Command::new("trunk")
        .args(&["build", "--release"])
        .current_dir(&frontend_dir)
        .status()
        .expect("Failed to compile the frontend");

    if !status.success() {
        panic!("Failed to compile the frontend");
    }
}
