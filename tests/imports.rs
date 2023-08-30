// SPDX-License-Identifier: Apache-2.0

use assert_cmd::Command;

#[test]
fn import_map_dup() {
    let mut cmd = Command::cargo_bin("solang").unwrap();
    let run = cmd
        .args([
            "compile",
            "--target",
            "solana",
            "--importmap",
            "foo=tests",
            "--importmap",
            "foo=tests2",
            "dummy.sol",
        ])
        .current_dir("tests/imports_testcases")
        .assert()
        .success();
    let output = run.get_output();

    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "warning: mapping 'foo' to 'tests' is overwritten\n"
    );
}

#[test]
fn import_map_badpath() {
    let mut cmd = Command::cargo_bin("solang").unwrap();
    let run = cmd
        .args([
            "compile",
            "--target",
            "solana",
            "--importmap",
            "foo=/does/not/exist",
            "dummy.sol",
        ])
        .current_dir("tests/imports_testcases")
        .assert()
        .success();

    let output = run.get_output();

    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
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
        .assert()
        .success();

    let output = assert.get_output();

    assert_eq!(String::from_utf8_lossy(&output.stderr), "");

    let mut cmd = Command::cargo_bin("solang").unwrap();
    let badpath = cmd
        .args(["compile", "import_map.sol", "--target", "solana"])
        .current_dir("tests/imports_testcases")
        .assert()
        .failure();

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
        .assert()
        .success();

    let output = assert.get_output();

    assert_eq!(String::from_utf8_lossy(&output.stderr), "");

    let mut cmd = Command::cargo_bin("solang").unwrap();
    let badpath = cmd
        .args(["compile", "--target", "solana", "import.sol"])
        .current_dir("tests/imports_testcases")
        .assert()
        .failure();

    let output = badpath.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stderr: {stderr}");

    assert!(stderr.contains("file not found 'bar.sol'"));
}

#[test]
fn contract_name_defined_twice() {
    let mut cmd = Command::cargo_bin("solang").unwrap();

    let ok = cmd
        .args(["compile", "--target", "solana", "bar.sol", "rel.sol"])
        .current_dir("tests/imports_testcases/imports")
        .assert()
        .success();

    let output = ok.get_output();

    assert_eq!(String::from_utf8_lossy(&output.stderr), "");

    let mut cmd = Command::cargo_bin("solang").unwrap();

    let not_ok = cmd
        .args([
            "compile",
            "--target",
            "solana",
            "relative_import.sol",
            "rel.sol",
        ])
        .current_dir("tests/imports_testcases/imports")
        .assert()
        .failure();

    let output = not_ok.get_output();
    let err = String::from_utf8_lossy(&output.stderr);

    println!("{}", err);

    // The error contains the absolute paths, so we cannot assert the whole string
    assert!(err.starts_with("error: contract rel defined at "));
    assert!(err.contains("relative_import.sol:1:1-6:2 and "));
    assert!(err.ends_with("rel.sol:2:1-16\n"));
}

#[test]
fn bad_escape() {
    let mut cmd = Command::cargo_bin("solang").unwrap();

    let not_ok = cmd
        .args([
            "compile",
            "--target",
            "solana",
            "tests/imports_testcases/bad_escape.sol",
        ])
        .assert()
        .failure();

    let output = not_ok.get_output();
    let err = String::from_utf8_lossy(&output.stderr);

    println!("{}", err);

    // The error contains the absolute paths, so we cannot assert the whole string
    assert!(err.contains(": \\x escape should be followed by two hex digits"));
    #[cfg(windows)]
    assert!(err.contains(": string is not a valid filename"));
    #[cfg(not(windows))]
    assert!(err.contains(": file not found 'bar�.sol'"));
}

// Ensure that .\ and ..\ are not interpreted as relative paths on Unix/MacOS
// Note Windows allows these as relative paths, but we do not.
#[test]
fn backslash_path() {
    let mut cmd = Command::cargo_bin("solang").unwrap();

    let not_ok = cmd
        .args([
            "compile",
            "--target",
            "solana",
            "tests/imports_testcases/imports/bar_backslash.sol",
            "--importpath",
            "tests/imports_testcases/imports",
        ])
        .assert();
    #[cfg(windows)]
    let not_ok = not_ok.success();

    #[cfg(not(windows))]
    let not_ok = not_ok.failure();

    let output = not_ok.get_output();
    let err = String::from_utf8_lossy(&output.stderr);

    println!("{}", err);

    #[cfg(windows)]
    assert!(err.is_empty());

    #[cfg(not(windows))]
    assert!(err.contains(": file not found '.\\relative_import.sol'"));
    #[cfg(not(windows))]
    assert!(err.contains(": file not found '..\\import.sol'"));
}

#[test]
fn found_two_files() {
    let mut cmd = Command::cargo_bin("solang").unwrap();
    let run = cmd
        .args([
            "compile",
            "--target",
            "solana",
            "--importpath",
            "imports",
            "-I",
            "imports",
            "--importpath",
            "meh",
            "-I",
            "meh",
            "import.sol",
        ])
        .current_dir("tests/imports_testcases")
        .assert()
        .failure();
    let output = run.get_output();
    let error = String::from_utf8_lossy(&output.stderr);
    println!("{error}");

    assert!(error.contains("error: import paths 'imports', 'meh' specifed more than once"));

    let mut cmd = Command::cargo_bin("solang").unwrap();
    let run = cmd
        .args([
            "compile",
            "--target",
            "solana",
            "--importpath",
            "imports",
            "-I",
            "imports2",
            "import.sol",
        ])
        .current_dir("tests/imports_testcases")
        .assert()
        .failure();
    let output = run.get_output();

    let error = String::from_utf8_lossy(&output.stderr);

    println!("{error}");

    assert!(error.contains(": found multiple files matching 'bar.sol': '"));
    #[cfg(windows)]
    {
        assert!(error.contains("\\tests\\imports_testcases\\imports\\bar.sol', '"));
        assert!(error.contains("\\tests\\imports_testcases\\imports2\\bar.sol'"));
    }
    #[cfg(not(windows))]
    {
        assert!(error.contains("/tests/imports_testcases/imports/bar.sol', '"));
        assert!(error.contains("/tests/imports_testcases/imports2/bar.sol'"));
    }
}
