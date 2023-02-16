// SPDX-License-Identifier: Apache-2.0

use assert_cmd::Command;
use std::fs::File;
use tempfile::TempDir;

#[test]
fn create_output_dir() {
    let mut cmd = Command::cargo_bin("solang").unwrap();

    let tmp = TempDir::new_in("tests").unwrap();

    let test1 = tmp.path().join("test1");

    cmd.args([
        "compile",
        "examples/flipper.sol",
        "--target",
        "solana",
        "--output",
    ])
    .arg(test1.clone())
    .assert()
    .success();

    File::open(test1.join("flipper.json")).expect("should exist");
    File::open(test1.join("flipper.so")).expect("should exist");

    let mut cmd = Command::cargo_bin("solang").unwrap();

    let test2 = tmp.path().join("test2");
    let test2_meta = tmp.path().join("test2_meta");

    cmd.args([
        "compile",
        "examples/flipper.sol",
        "--target",
        "solana",
        "--contract",
        "flipper",
        "--output",
    ])
    .arg(test2.clone())
    .arg("--output-meta")
    .arg(test2_meta.clone())
    .assert()
    .success();

    File::open(test2.join("flipper.so")).expect("should exist");
    File::open(test2_meta.join("flipper.json")).expect("should exist");

    let mut cmd = Command::cargo_bin("solang").unwrap();

    cmd.args([
        "compile",
        "examples/flipper.sol",
        "--target",
        "solana",
        "--output",
        "examples/flipper.sol",
    ])
    .assert()
    .failure();

    let mut cmd = Command::cargo_bin("solang").unwrap();

    let test3 = tmp.path().join("test3");

    cmd.args([
        "compile",
        "examples/flipper.sol",
        "--target",
        "solana",
        "--contract",
        "flapper,flipper", // not just flipper
        "--output",
    ])
    .arg(test3.clone())
    .assert()
    .failure();

    // nothing should have been created because flapper does not exist
    assert!(!test3.exists());
}
