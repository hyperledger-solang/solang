use anyhow::{anyhow, Result};
use assert_cmd::cargo::cargo_bin;
use regex::Regex;
use std::{
    env::var,
    ffi::OsStr,
    fs::{copy, read_to_string},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
};
use tempfile::{tempdir, TempDir};
use walkdir::WalkDir;

mod stylus_tests;

const PRIVATE_KEY: &str = "0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659";

// smoelius: Only one Stylus test can be run at a time.
static MUTEX: Mutex<()> = Mutex::new(());

fn test(required: &[&str], forbidden: &[&str]) {
    let _lock = MUTEX.lock();
    let required = required
        .iter()
        .map(|s| Regex::new(&format!(r"\<{s}\>")).unwrap())
        .collect::<Vec<_>>();
    let forbidden = forbidden
        .iter()
        .map(|s| Regex::new(&format!(r"\<{s}\>")).unwrap())
        .collect::<Vec<_>>();
    let contract_re = Regex::new(r"\<contract ([A-Za-z_0-9]+)\>").unwrap();
    let argless_function_re = Regex::new(r"\<function ([A-Za-z_0-9]+)\(\)").unwrap();
    for result in WalkDir::new("testdata/solidity/test/libsolidity/semanticTests") {
        let entry = result.unwrap();
        let path = entry.path();
        if !path.is_file() || path.extension() != Some(OsStr::new("sol")) {
            continue;
        }
        let contents = read_to_string(path).unwrap();
        if !required.iter().all(|re| re.is_match(&contents)) {
            continue;
        }
        if forbidden.iter().any(|re| re.is_match(&contents)) {
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

fn deploy(path: impl AsRef<Path>, contract: &str) -> Result<(TempDir, String)> {
    let tempdir = tempdir().unwrap();
    let dir = &tempdir;

    let rust_toolchain_toml =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/rust-toolchain.toml");
    copy(rust_toolchain_toml, dir.path().join("rust-toolchain.toml")).unwrap();

    command(
        dir,
        [
            cargo_bin("solang").to_str().unwrap(),
            "compile",
            &path.as_ref().to_string_lossy(),
            "--target=stylus",
            // smoelius: The default LLVM optimization level can cause public functions to be
            // inlined into the dispatch function.
            "-O=less",
        ],
    )?;

    command(
        dir,
        [
            "cargo",
            "stylus",
            "check",
            &format!("--wasm-file={contract}.wasm"),
        ],
    )
    .unwrap();

    let stdout = command(
        dir,
        [
            "cargo",
            "stylus",
            "deploy",
            &format!("--wasm-file={contract}.wasm"),
            "--endpoint=http://localhost:8547",
            "--private-key",
            PRIVATE_KEY,
            "--no-verify",
        ],
    )
    .unwrap();

    let address = stdout
        .lines()
        .find_map(|line| line.strip_prefix("deployed code at address: "))
        .unwrap();

    Ok((tempdir, address.to_owned()))
}

pub fn call<I, S>(dir: impl AsRef<Path>, address: &str, args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    cast(dir, CastSubcommand::Call, address, args)
}

pub fn send<I, S>(dir: impl AsRef<Path>, address: &str, args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    cast(dir, CastSubcommand::Send, address, args)
}

enum CastSubcommand {
    Call,
    Send,
}

impl CastSubcommand {
    fn as_str(&self) -> &str {
        match self {
            CastSubcommand::Call => "call",
            CastSubcommand::Send => "send",
        }
    }
}

fn cast<I, S>(dir: impl AsRef<Path>, subcommand: CastSubcommand, address: &str, args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let iter = [
        "cast",
        subcommand.as_str(),
        "--rpc-url=http://localhost:8547",
        "--private-key",
        PRIVATE_KEY,
        &address,
    ]
    .into_iter()
    .map(OsStr::new);

    let other = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect::<Vec<_>>();

    command(dir, iter.chain(other.iter().map(|s| s.as_os_str()))).unwrap()
}

fn command<I, S>(dir: impl AsRef<Path>, args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut args = args.into_iter();
    let mut command = Command::new(args.next().unwrap());
    command.args(args);
    command.current_dir(dir);
    if enabled("VERBOSE") {
        command.stderr(Stdio::inherit());
    }
    let output = command.output().unwrap();
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).unwrap();
        Ok(strip_ansi_escapes::strip_str(stdout))
    } else {
        Err(anyhow!("command failed: {command:?}"))
    }
}

pub fn enabled(key: &str) -> bool {
    var(key).is_ok_and(|value| value != "0")
}
