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

// Returns a list of all `.sol` files in the given `dir` path.
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

// Attempts to compile the file at `path` for the `target`, returning the diagnostics on fail.
fn try_compile(path: &str, target: Target) -> Result<(), Diagnostics> {
    let mut cache = FileResolver::new();
    cache.set_file_contents(path, read_to_string(path).unwrap());
    let mut ns = solang::parse_and_resolve(OsStr::new(path), &mut cache, target);
    if ns.diagnostics.any_errors() {
        return Err(ns.diagnostics);
    }
    // Codegen should work too
    codegen(
        &mut ns,
        &Options {
            opt_level: OptimizationLevel::Default,
            ..Default::default()
        },
    );
    Ok(())
}

// Assert no compilation errors for for all `.sol` files found in `dir`.
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
