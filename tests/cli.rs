// SPDX-License-Identifier: Apache-2.0

use assert_cmd::Command;
use std::{env::set_current_dir, fs::File};
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

    cmd.args([
        "compile",
        "examples/solana/flipper.sol",
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

    println!("path:  =>> {}", solana_test.display());
    //solang new --target solana
    new_cmd
        .arg("new")
        .arg(solana_test.clone())
        .args(["--target", "solana"])
        .assert()
        .success();

    let flipper_file = File::open(solana_test.join("flipper.sol")).expect("should exist");
    let config_file = File::open(solana_test.join("Solang.toml")).expect("should exist");

    // compile flipper using config file
    let mut compile_cmd = Command::cargo_bin("solang").unwrap();

    //assert!(set_current_dir(solana_test).is_ok());

    compile_cmd
        .args(["compile", "--configuration-file"])
        .current_dir(solana_test)
        .assert()
        .success();

    let substrate_test = tmp.path().join("substrate_test");
    let new_cmd = Command::cargo_bin("solang")
        .unwrap()
        .arg("new")
        .arg(substrate_test.clone())
        .args(["--target", "solana"])
        .assert()
        .success();

    //assert!(set_current_dir(substrate_test).is_ok());

    compile_cmd.current_dir(substrate_test).assert().success();
}

fn incorrect_toml_error_handling() {}
