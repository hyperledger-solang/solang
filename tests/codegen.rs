use assert_cmd::Command;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::{fs, path::PathBuf};

#[test]
fn testcases() {
    let ext = OsString::from("sol");
    for entry in fs::read_dir("./tests/codegen_testcases/").unwrap() {
        let path = entry.unwrap().path();

        if path.is_file() && path.extension() == Some(&ext) {
            testcase(path);
        }
    }
}

fn testcase(path: PathBuf) {
    // find the args to run.
    println!("testcase: {}", path.display());

    let file = File::open(&path).unwrap();
    let reader = BufReader::new(file);
    let mut command_line: Option<String> = None;
    let mut checks = Vec::new();
    for line in reader.lines() {
        let line = line.unwrap();
        if let Some(args) = line.strip_prefix("// RUN: ") {
            assert_eq!(command_line, None);

            command_line = Some(String::from(args));
        } else if let Some(check) = line.strip_prefix("// CHECK:") {
            checks.push(check.trim().to_string());
        }
    }

    let args = command_line.expect("cannot find RUN: line");
    assert_ne!(checks.len(), 0);

    let mut cmd = Command::cargo_bin("solang").unwrap();
    let assert = cmd
        .args(args.split_whitespace())
        .arg(format!("{}", path.canonicalize().unwrap().display()))
        .assert()
        .success();
    let output = assert.get_output();

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut check_done = 0;

    for line in stdout.split('\n') {
        // have we done all checks
        if check_done < checks.len() && line.find(&checks[check_done]).is_some() {
            check_done += 1;
        }
    }

    if check_done < checks.len() {
        panic!("NOT FOUND CHECK: {}", checks[check_done]);
    }
}
