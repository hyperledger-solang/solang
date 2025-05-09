//! This test is based on the following tutorial:
//! <https://stylus-by-example.org/getting_started/using_the_cli>
//!
//! It expects you to have a devnode running:
//! <https://docs.arbitrum.io/run-arbitrum-node/run-nitro-dev-node>
//!
//! It also expects `cargo-stylus` and `cast` to be installed:
//! - <https://github.com/OffchainLabs/cargo-stylus>
//! - <https://book.getfoundry.sh/cast/>
#![warn(clippy::pedantic)]

use assert_cmd::cargo::cargo_bin;
use std::{
    ffi::OsStr,
    fs::copy,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tempfile::tempdir;

const PRIVATE_KEY: &str = "0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659";

#[allow(clippy::too_many_lines)]
#[test]
fn counter() {
    let tempdir = tempdir().unwrap();
    let dir = &tempdir;

    let rust_toolchain_toml =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/rust-toolchain.toml");
    copy(rust_toolchain_toml, dir.path().join("rust-toolchain.toml")).unwrap();

    let counter_sol =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/counter.sol");

    command(
        dir,
        [
            cargo_bin("solang").to_str().unwrap(),
            "compile",
            &counter_sol.to_string_lossy(),
            "--target=stylus",
        ],
    );

    command(
        dir,
        ["cargo", "stylus", "check", "--wasm-file=Counter.wasm"],
    );

    let stdout = command(
        dir,
        [
            "cargo",
            "stylus",
            "deploy",
            "--wasm-file=Counter.wasm",
            "--endpoint=http://localhost:8547",
            "--private-key",
            PRIVATE_KEY,
            "--no-verify",
        ],
    );
    let address = stdout
        .lines()
        .find_map(|line| line.strip_prefix("deployed code at address: "))
        .unwrap();

    let stdout = command(
        dir,
        [
            "cast",
            "call",
            "--rpc-url=http://localhost:8547",
            "--private-key",
            PRIVATE_KEY,
            address,
            "number()(uint256)",
        ],
    );
    assert_eq!("0\n", stdout);

    command(
        dir,
        [
            "cast",
            "send",
            "--rpc-url=http://localhost:8547",
            "--private-key",
            PRIVATE_KEY,
            address,
            "increment()",
        ],
    );

    let stdout = command(
        dir,
        [
            "cast",
            "call",
            "--rpc-url=http://localhost:8547",
            "--private-key",
            PRIVATE_KEY,
            address,
            "number()(uint256)",
        ],
    );
    println!("{stdout}");
    // assert_eq!("1\n", stdout);

    command(
        dir,
        [
            "cast",
            "send",
            "--rpc-url=http://localhost:8547",
            "--private-key",
            PRIVATE_KEY,
            address,
            "setNumber(uint256)",
            "5",
        ],
    );

    let stdout = command(
        dir,
        [
            "cast",
            "call",
            "--rpc-url=http://localhost:8547",
            "--private-key",
            PRIVATE_KEY,
            address,
            "number()(uint256)",
        ],
    );
    assert_eq!("5\n", stdout);
}

fn command<I, S>(dir: impl AsRef<Path>, args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut args = args.into_iter();
    let mut command = Command::new(args.next().unwrap());
    command.args(args);
    command.current_dir(dir);
    command.stderr(Stdio::inherit());
    let output = command.output().unwrap();
    assert!(output.status.success(), "command failed: {command:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    strip_ansi_escapes::strip_str(stdout)
}
