// SPDX-License-Identifier: Apache-2.0

use assert_cmd::Command;
use std::fs::{remove_dir, remove_file, File};

#[test]
fn create_output_dir() {
    let mut cmd = Command::cargo_bin("solang").unwrap();

    cmd.args([
        "compile",
        "examples/flipper.sol",
        "--target",
        "solana",
        "--output",
        "tests/create_me",
    ])
    .assert()
    .success();

    File::open("tests/create_me/flipper.json").expect("should exist");
    File::open("tests/create_me/flipper.so").expect("should exist");

    remove_file("tests/create_me/flipper.json").unwrap();
    remove_file("tests/create_me/flipper.so").unwrap();
    remove_dir("tests/create_me").unwrap();

    let mut cmd = Command::cargo_bin("solang").unwrap();

    cmd.args([
        "compile",
        "examples/flipper.sol",
        "--target",
        "solana",
        "--output",
        "tests/create_me",
        "--output-meta",
        "tests/create_me_meta",
    ])
    .assert()
    .success();

    File::open("tests/create_me/flipper.so").expect("should exist");
    File::open("tests/create_me_meta/flipper.json").expect("should exist");

    remove_file("tests/create_me/flipper.so").unwrap();
    remove_dir("tests/create_me").unwrap();
    remove_file("tests/create_me_meta/flipper.json").unwrap();
    remove_dir("tests/create_me_meta").unwrap();

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
}
