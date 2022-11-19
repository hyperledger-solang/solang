// SPDX-License-Identifier: Apache-2.0

use rayon::prelude::*;
use solang::{
    codegen::{codegen, OptimizationLevel, Options},
    file_resolver::FileResolver,
    sema::diagnostics::Diagnostics,
    Target,
};
use std::{
    ffi::OsStr,
    fs::{read_dir, read_to_string},
};

/// Poppulates a file resolver with all imports that could be used by some example.
fn file_resolver(target: Target) -> FileResolver {
    let mut result = FileResolver::new();
    result.set_file_contents(
        "examples/user.sol",
        r##"
        struct User { string name; uint count; }
        function clear_count(User memory user) {
            user.count = 0;
        }
        using {clear_count} for User global;"##
            .into(),
    );
    if let Target::Solana = target {
        result.set_file_contents(
                "examples/solana/bobcat.sol",
                r##"
                anchor_anchor constant bobcat = anchor_anchor(address'z7FbDfQDfucxJz5o8jrGLgvSbdoeSqX5VrxBb5TVjHq');
                interface anchor_anchor {
                    function pounce() selector=hex"afaf6d1f0d989bed" view external returns(int64);
                }"##.into(),
            );
    }
    result
}

/// Returns a list of all `.sol` files in the given `dir` path.
fn get_source_files(dir: &str) -> Vec<String> {
    read_dir(dir)
        .unwrap()
        .into_iter()
        .filter_map(|entry| {
            let e = entry.unwrap();
            if let (true, Some(ext)) = (e.path().is_file(), e.path().extension()) {
                if ext == "sol" {
                    return Some(e.path().display().to_string());
                }
            }
            None
        })
        .collect()
}

/// Attempts to compile the file at `path` for the `target`, returning the diagnostics on fail.
fn try_compile(path: &str, target: Target) -> Result<(), Diagnostics> {
    let mut cache = file_resolver(target);
    cache.set_file_contents(path, read_to_string(path).unwrap());
    let mut ns = solang::parse_and_resolve(OsStr::new(path), &mut cache, target);
    if ns.diagnostics.any_errors() {
        return Err(ns.diagnostics);
    }
    // Codegen should work too
    codegen(
        &mut ns,
        &Options {
            dead_storage: true,
            constant_folding: true,
            strength_reduce: true,
            vector_to_slice: true,
            math_overflow_check: true,
            common_subexpression_elimination: true,
            opt_level: OptimizationLevel::Default,
            ..Default::default()
        },
    );
    Ok(())
}

/// Assert no compilation errors for all `.sol` files found in `dir`.
fn assert_compile(dir: &str, target: Target) {
    let errors = get_source_files(dir)
        .par_iter()
        .filter_map(|path| try_compile(path, target).err().map(|msg| (path, msg)))
        .map(|(path, msg)| println!("{} failed: {:#?}", path, msg))
        .count();
    assert_eq!(0, errors);
}

#[test]
fn substrate_general() {
    assert_compile("examples/", Target::default_substrate())
}

#[test]
fn substrate_specific() {
    assert_compile("examples/substrate/", Target::default_substrate())
}

#[test]
fn solana_general() {
    assert_compile("examples/", Target::Solana)
}

#[test]
fn solana_specific() {
    assert_compile("examples/solana", Target::Solana)
}
