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
        "examples/solana/flipper.sol",
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

    let assert = cmd
        .args([
            "compile",
            "examples/solana/flipper.sol",
            "--target",
            "solana",
            "--contract",
            "flipper",
            "--contract-authors",
            "itchy and cratchy",
            "--output",
        ])
        .arg(test2.clone())
        .arg("--output-meta")
        .arg(test2_meta.clone())
        .assert()
        .success();

    File::open(test2.join("flipper.so")).expect("should exist");
    File::open(test2_meta.join("flipper.json")).expect("should exist");

    let output = assert.get_output();

    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "warning: the `authors` flag will be ignored for Solana target\n"
    );

    let mut cmd = Command::cargo_bin("solang").unwrap();

    cmd.args([
        "compile",
        "examples/solana/flipper.sol",
        "--target",
        "solana",
        "--output",
        "examples/solana/flipper.sol",
    ])
    .assert()
    .failure();

    let mut cmd = Command::cargo_bin("solang").unwrap();

    let test3 = tmp.path().join("test3");

    cmd.args([
        "compile",
        "examples/solana/flipper.sol",
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

#[test]
fn basic_compilation_from_toml() {
    let mut new_cmd = Command::cargo_bin("solang").unwrap();
    let tmp = TempDir::new_in("tests").unwrap();

    let solana_test = tmp.path().join("solana_test");

    //solang new --target solana
    new_cmd
        .arg("new")
        .arg(solana_test.clone())
        .args(["--target", "solana"])
        .assert()
        .success();
    File::open(solana_test.join("flipper.sol")).expect("should exist");
    File::open(solana_test.join("solang.toml")).expect("should exist");

    // compile flipper using config file
    let mut compile_cmd = Command::cargo_bin("solang").unwrap();

    compile_cmd
        .args(["compile"])
        .current_dir(solana_test)
        .assert()
        .success();

    let polkadot_test = tmp.path().join("polkadot_test");
    let _new_cmd = Command::cargo_bin("solang")
        .unwrap()
        .arg("new")
        .arg(polkadot_test.clone())
        .args(["--target", "solana"])
        .assert()
        .success();

    compile_cmd.current_dir(polkadot_test).assert().success();
}
