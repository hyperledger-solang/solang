use assert_cmd::Command;

#[test]
fn import_map_dup() {
    let mut cmd = Command::cargo_bin("solang").unwrap();
    let dup = cmd
        .args(&[
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

    println!("stderr: {}", stderr);

    assert_eq!(
        stderr,
        "error: import path ‘tests’: duplicate mapping for ‘foo’\n"
    );
}

#[test]
fn import_map_badpath() {
    let mut cmd = Command::cargo_bin("solang").unwrap();
    let badpath = cmd
        .args(&["--importmap", "foo=/does/not/exist", "bar.sol"])
        .env("exit", "1")
        .assert();

    let output = badpath.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stderr: {}", stderr);

    assert!(stderr.contains("error: import path ‘/does/not/exist’: "));
}

#[test]
fn import_map() {
    let mut cmd = Command::cargo_bin("solang").unwrap();
    let assert = cmd
        .args(&["--importmap", "foo=imports/", "import_map.sol"])
        .current_dir("tests/imports_testcases")
        .assert();

    let output = assert.get_output();

    assert_eq!(String::from_utf8_lossy(&output.stderr), "");

    let mut cmd = Command::cargo_bin("solang").unwrap();
    let badpath = cmd
        .args(&["import_map.sol"])
        .current_dir("tests/imports_testcases")
        .assert();

    let output = badpath.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stderr: {}", stderr);

    assert!(stderr.contains("import_map.sol:1:8-21: error: file not found ‘foo/bar.sol’"));
}

#[test]
fn import() {
    let mut cmd = Command::cargo_bin("solang").unwrap();
    let assert = cmd
        .args(&["--importpath", "imports", "import.sol"])
        .current_dir("tests/imports_testcases")
        .assert();

    let output = assert.get_output();

    assert_eq!(String::from_utf8_lossy(&output.stderr), "");

    let mut cmd = Command::cargo_bin("solang").unwrap();
    let badpath = cmd
        .args(&["import.sol"])
        .current_dir("tests/imports_testcases")
        .assert();

    let output = badpath.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stderr: {}", stderr);

    assert!(stderr.contains("error: file not found ‘bar.sol’"));
}
