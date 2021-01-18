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

#[derive(Debug)]
enum Test {
    Check(String),
    Fail(String),
    Rewind,
}

fn testcase(path: PathBuf) {
    // find the args to run.
    println!("testcase: {}", path.display());

    let file = File::open(&path).unwrap();
    let reader = BufReader::new(file);
    let mut command_line: Option<String> = None;
    let mut checks = Vec::new();
    let mut fails = Vec::new();
    for line in reader.lines() {
        let line = line.unwrap();
        if let Some(args) = line.strip_prefix("// RUN: ") {
            assert_eq!(command_line, None);

            command_line = Some(String::from(args));
        } else if let Some(check) = line.strip_prefix("// CHECK:") {
            checks.push(Test::Check(check.trim().to_string()));
        } else if let Some(fail) = line.strip_prefix("// FAIL:") {
            fails.push(Test::Fail(fail.trim().to_string()));
        } else if let Some(check) = line.strip_prefix("// BEGIN-CHECK:") {
            checks.push(Test::Rewind);
            checks.push(Test::Check(check.trim().to_string()));
        }
    }

    let args = command_line.expect("cannot find RUN: line");
    assert_ne!(checks.len() + fails.len(), 0);

    let mut cmd = Command::cargo_bin("solang").unwrap();
    let assert = cmd
        .args(args.split_whitespace())
        .arg(format!("{}", path.canonicalize().unwrap().display()))
        .assert();
    let output = assert.get_output();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut current_check = 0;
    let mut current_fail = 0;
    let mut current_line = 0;
    let lines: Vec<&str> = stdout.split('\n').chain(stderr.split('\n')).collect();

    while current_line < lines.len() {
        let line = lines[current_line];

        match checks.get(current_check) {
            Some(Test::Check(needle)) => {
                if line.find(needle).is_some() {
                    current_check += 1;
                }
            }
            Some(Test::Rewind) => {
                current_line = 0;
                current_check += 1;
                continue;
            }
            _ => (),
        }

        match fails.get(current_fail) {
            Some(Test::Fail(needle)) => {
                if line.find(needle).is_some() {
                    current_fail += 1;
                }
            }
            _ => (),
        }

        current_line += 1;
    }

    if current_check < checks.len() {
        println!("OUTPUT: \n===8<===8<===\n{}===8<===8<===\n", stdout);

        panic!("NOT FOUND CHECK: {:?}", checks[current_check]);
    } else if current_fail < fails.len() {
        println!("STDERR: \n===8<===8<===\n{}===8<===8<===\n", stderr);

        panic!("NOT FOUND FAIL: {:?}", fails[current_check]);
    }
}
