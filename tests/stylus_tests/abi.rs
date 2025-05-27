#![warn(clippy::pedantic)]

use crate::{call, deploy, MUTEX};
use regex::Regex;
use std::{ffi::OsStr, fs::read_to_string, path::PathBuf};
use walkdir::WalkDir;

#[test]
fn abi() {
    let _lock = MUTEX.lock();
    let abi_re = Regex::new(r"\<abi\>").unwrap();
    let assembly_re = Regex::new(r"\<assembly\>").unwrap();
    let assert_re = Regex::new(r"\<assert\>").unwrap();
    let contract_re = Regex::new(r"\<contract ([A-Za-z_0-9]+)\>").unwrap();
    let argless_function_re = Regex::new(r"\<function ([A-Za-z_0-9]+)\(\)").unwrap();
    for result in WalkDir::new("testdata/solidity/test/libsolidity/semanticTests") {
        let entry = result.unwrap();
        let path = entry.path();
        if !path.is_file() || path.extension() != Some(OsStr::new("sol")) {
            continue;
        }
        let contents = read_to_string(path).unwrap();
        if !abi_re.is_match(&contents) || !assert_re.is_match(&contents) {
            continue;
        }
        if assembly_re.is_match(&contents) {
            continue;
        }
        let contracts = contract_re
            .captures_iter(&contents)
            .map(|captures| {
                assert_eq!(2, captures.len());
                captures.get(1).unwrap().as_str()
            })
            .collect::<Vec<_>>();
        let [contract] = contracts[..] else {
            eprintln!(
                "Skipping `{}` as it contains {} contracts",
                path.display(),
                contracts.len()
            );
            continue;
        };
        let argless_functions = argless_function_re
            .captures_iter(&contents)
            .map(|captures| {
                assert_eq!(2, captures.len());
                captures.get(1).unwrap().as_str()
            })
            .collect::<Vec<_>>();
        if argless_functions.is_empty() {
            eprintln!(
                "Skipping `{}` as it contains no argless functions",
                path.display(),
            );
            continue;
        }

        eprintln!("Deploying `{}`", path.display());

        let (tempdir, address) = match deploy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path),
            contract,
        ) {
            Ok((tempdir, address)) => (tempdir, address),
            Err(error) => {
                eprintln!("Failed to deploy `{}`: {error:?}", path.display());
                continue;
            }
        };
        let dir = &tempdir;

        for function in argless_functions {
            eprintln!("Testing `{function}`");
            call(dir, &address, &[&format!("{function}()")]);
        }
    }
}
