extern crate lalrpop;
use std::process::Command;

fn main() {
    lalrpop::Configuration::new()
        .generate_in_source_tree()
        .emit_rerun_directives(true)
        .process()
        .unwrap();

    // note: add error checking yourself.
    let output = Command::new("git")
        .args(&["describe", "--tags"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
