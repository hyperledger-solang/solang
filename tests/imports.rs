// SPDX-License-Identifier: Apache-2.0

use assert_cmd::Command;

#[test]
fn import_map_dup() {
    let mut cmd = Command::cargo_bin("solang").unwrap();
    let dup = cmd
        .args([
            "compile",
            "--target",
            "solana",
            "--importmap",
            "foo=tests",
            "--importmap",
            "foo=tests",
            "foo.sol",
        ])
        .env("exit", "1")
        .assert();

    let output = dup.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stderr: {stderr}");

    assert_eq!(
        stderr,
        "error: import path 'tests': duplicate mapping for 'foo'\n"
    );
}

#[test]
fn import_map_badpath() {
    let mut cmd = Command::cargo_bin("solang").unwrap();
    let badpath = cmd
        .args([
            "compile",
            "--target",
            "solana",
            "--importmap",
            "foo=/does/not/exist",
            "bar.sol",
        ])
        .env("exit", "1")
        .assert();

    let output = badpath.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stderr: {stderr}");

    assert!(stderr.contains("error: import path '/does/not/exist': "));
}

#[test]
fn import_map() {
    let mut cmd = Command::cargo_bin("solang").unwrap();
    let assert = cmd
        .args([
            "compile",
            "--target",
            "solana",
            "--importmap",
            "foo=imports/",
            "import_map.sol",
        ])
        .current_dir("tests/imports_testcases")
        .assert();

    let output = assert.get_output();

    assert_eq!(String::from_utf8_lossy(&output.stderr), "");

    let mut cmd = Command::cargo_bin("solang").unwrap();
    let badpath = cmd
        .args(["compile", "import_map.sol", "--target", "solana"])
        .current_dir("tests/imports_testcases")
        .assert();

    let output = badpath.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stderr: {stderr}");

    assert!(stderr.contains("file not found 'foo/bar.sol'"));
}

#[test]
fn import() {
    let mut cmd = Command::cargo_bin("solang").unwrap();
    let assert = cmd
        .args([
            "compile",
            "--target",
            "solana",
            "--importpath",
            "./imports_testcases/imports",
            "imports_testcases/import.sol",
        ])
        .current_dir("tests")
        .assert();

    let output = assert.get_output();

    assert_eq!(String::from_utf8_lossy(&output.stderr), "");

    let mut cmd = Command::cargo_bin("solang").unwrap();
    let badpath = cmd
        .args(["compile", "--target", "solana", "import.sol"])
        .current_dir("tests/imports_testcases")
        .assert();

    let output = badpath.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stderr: {stderr}");

    assert!(stderr.contains("file not found 'bar.sol'"));
}
