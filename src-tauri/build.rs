use std::path::PathBuf;

fn main() {
    // Build the Tauri application
    tauri_build::build();

    // Only Windows bundles the runner (other systems copy the app itself as the fake game)
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let resource_path = PathBuf::from("resources").join("quest-runner.exe");

    // Tell cargo to rerun if the resource file changes
    if resource_path.exists() {
        println!("cargo:rerun-if-changed={}", resource_path.display());
    } else {
        eprintln!("Warning: Runner not found at {}", resource_path.display());
        eprintln!("Make sure to run `pnpm sync:runner` first");
    }
}
