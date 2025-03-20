use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    Command::new("cargo")
        .current_dir("guest/")
        .arg("component")
        .arg("build")
        .arg("--release")
        .output()
        .expect("failed to execute process");
}
