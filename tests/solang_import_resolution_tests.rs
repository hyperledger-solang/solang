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
fn import_test_01_solang_remap_target() {
    // Command 1
    let mut cmd = make_run("01_solang_remap_target");
    let run = cmd.arg("contracts/Contract.sol").env("exit", "1").assert();

    let stderr = String::from_utf8_lossy(&run.get_output().stderr);
    println!("stderr: {stderr}");
    assert!(stderr.contains("file not found 'lib/Lib.sol'"));
    assert!(stderr.contains("'Lib' not foun"));
    assert!(stderr.contains("'x' not found"));

    // Command 2
    let mut cmd = make_run("01_solang_remap_target");
    let run = cmd
        .arg("contracts/Contract.sol")
        .arg("--importmap")
        .arg("lib=node_modules/lib")
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&run.get_output().stderr);
    println!("stderr: {stderr}");
    assert!(stderr.contains("file not found 'lib/Lib.sol'"));
    assert!(stderr.contains("'Lib' not foun"));
    assert!(stderr.contains("'x' not found"));

    // Command 3
    let mut cmd = make_run("01_solang_remap_target");
    let run = cmd
        .arg("contracts/Contract.sol")
        .arg("--importmap")
        .arg("lib=node_modules/lib")
        .arg("--importpath")
        .arg(".")
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&run.get_output().stderr);
    println!("stderr: {stderr}");
    assert!(stderr.contains("file not found 'lib/Lib.sol'"));
    assert!(stderr.contains("'Lib' not foun"));
    assert!(stderr.contains("'x' not found"));

    // Command 4
    let mut cmd = make_run("01_solang_remap_target");
    let run = cmd
        .arg("contracts/Contract.sol")
        .arg("--importmap")
        .arg("lib=node_modules/lib")
        .arg("--importpath")
        .arg(".")
        .arg("--importpath")
        .arg("resources/node_modules")
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&run.get_output().stderr);
    println!("stderr: {stderr}");
    assert!(stderr.contains("file not found 'lib/Lib.sol'"));
    assert!(stderr.contains("'Lib' not foun"));
    assert!(stderr.contains("'x' not found"));

    // Command 5
    let mut cmd = make_run("01_solang_remap_target");
    cmd.arg("contracts/Contract.sol")
        .arg("--importmap")
        .arg("lib=node_modules/lib")
        .arg("--importpath")
        .arg(".")
        .arg("--importpath")
        .arg("resources")
        .assert()
        .success();
}

#[test]
fn import_test_02_solang_incorrect_direct_imports() {
    // Command 1
    let mut cmd = make_run("02_solang_incorrect_direct_imports");
    cmd.arg("contracts/Contract.sol").env("exit", "0").assert();

    // Command 2
    let mut cmd = make_run("02_solang_incorrect_direct_imports");
    cmd.arg("contracts/Contract.sol")
        .arg("--importpath")
        .arg(".")
        .assert()
        .success();
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

#[test]
fn import_test_04_multiple_path_map_segments() {
    // Command 1
    let mut cmd = make_run("04_multiple_map_path_segments");
    cmd.arg("contracts/Contract.sol")
        .arg("--importmap")
        .arg("lib/nested=resources/node_modules/lib/nested")
        .arg("--importpath")
        .arg(".")
        .assert()
        .success();
}

#[test]
fn import_test_06_redundant_remaps() {
    // Command 1
    let mut cmd = make_run("06_redundant_remaps");
    cmd.arg("contracts/Contract.sol")
        .arg("--importmap")
        .arg("node_modules=resources/node_modules")
        .arg("--importmap")
        .arg("node_modules=node_modules")
        .arg("--importpath")
        .arg("resources")
        .assert()
        .success();

    let mut cmd = make_run("06_redundant_remaps");
    cmd.arg("contracts/Contract.sol")
        .arg("--importmap")
        .arg("node_modules=node_modules")
        .arg("--importmap")
        .arg("node_modules=resources/node_modules")
        .arg("--importpath")
        .arg("resources")
        .assert()
        .failure();

    let mut cmd = make_run("06_redundant_remaps");
    cmd.arg("contracts/Contract.sol")
        .arg("--importmap")
        .arg("node_modules=node_modules")
        .arg("--importmap")
        .arg("node_modules=resources/node_modules")
        .arg("--importmap")
        .arg("node_modules=node_modules")
        .arg("--importpath")
        .arg("resources")
        .assert()
        .success();
}
