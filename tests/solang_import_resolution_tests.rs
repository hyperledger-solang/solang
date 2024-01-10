// SPDX-License-Identifier: Apache-2.0

use assert_cmd::Command;
use std::path::PathBuf;

fn solang_import_resolution_tests() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata")
        .join("solang_import_resolution_tests")
}

fn make_run(dir: &str) -> Command {
    let current_dir = solang_import_resolution_tests().join(dir);
    let mut cmd = Command::cargo_bin("solang").unwrap();
    cmd.current_dir(current_dir)
        .args(["compile", "--target", "solana"]);
    cmd
}

#[test]
fn import_test_03_ambiguous_imports_should_fail() {
    // Command 1
    let mut cmd = make_run("03_ambiguous_imports_should_fail");
    cmd.arg("contracts/Contract.sol")
        .arg("--importmap")
        .arg("lib=resources/node_modules/lib")
        .arg("--importpath")
        .arg("contracts")
        .arg("--importpath")
        .arg(".")
        .assert()
        .failure();
}
