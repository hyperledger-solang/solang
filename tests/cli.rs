// SPDX-License-Identifier: Apache-2.0

use assert_cmd::Command;
use std::fs::File;

#[test]
fn create_output_dir() {
    let mut cmd = Command::cargo_bin("solang").unwrap();

    cmd.args(&[
        "examples/flipper.sol",
        "--target",
        "solana",
        "--output",
        "tests/create_me",
    ])
    .assert()
    .success();

    File::open("tests/create_me/flipper.abi").expect("should exist");

    let mut cmd = Command::cargo_bin("solang").unwrap();

    cmd.args(&[
        "examples/flipper.sol",
        "--target",
        "solana",
        "--output",
        "examples/flipper.sol",
    ])
    .assert()
    .failure();
}
