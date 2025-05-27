use anyhow::{anyhow, Result};
use assert_cmd::cargo::cargo_bin;
use std::{
    env::var,
    ffi::OsStr,
    fs::copy,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
};
use tempfile::{tempdir, TempDir};

mod stylus_tests;

const PRIVATE_KEY: &str = "0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659";

// smoelius: Only one Stylus test can be run at a time.
static MUTEX: Mutex<()> = Mutex::new(());

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
