//! This test expects you to have a devnode running:
//! <https://docs.arbitrum.io/run-arbitrum-node/run-nitro-dev-node>
//!
//! It also expects `cargo-stylus` and `cast` to be installed:
//! - <https://github.com/OffchainLabs/cargo-stylus>
//! - <https://book.getfoundry.sh/cast/>
#![warn(clippy::pedantic)]

use crate::{call, deploy, send, MUTEX};
use std::{io::Write, path::PathBuf};

#[test]
fn milestone_2() {
    writeln!(
        std::io::stderr(),
        "If you run the `milestone_2` test twice, it will fail the second time because the \
         contract `Greeter` cannot be activated twice.",
    )
    .unwrap();

    let _lock = MUTEX.lock();
    let (tempdir, address) = deploy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/milestone_2.sol"),
        "C",
        true,
    )
    .unwrap();
    let dir = &tempdir;

    let mut stdout = call(
        dir,
        &address,
        ["test_block()(uint64,uint256,address,uint256,uint256,uint256,uint256)"],
    )
    .unwrap();
    println!("{}", label_test_block_output(&stdout));

    stdout = call(dir, &address, ["test_tstore()(uint256,uint256)"]).unwrap();
    println!("{}", label_test_tstore_output(&stdout));

    stdout = send(
        dir,
        &address,
        ["test_create1()", "--value=1000000000000000000"],
    )
    .unwrap();
    println!("{}", stdout);

    stdout = send(
        dir,
        &address,
        ["test_create2()", "--value=1000000000000000000"],
    )
    .unwrap();
    println!("{}", stdout);

    stdout = send(
        dir,
        &address,
        ["test_value_sender()", "--value=1000000000000000000"],
    )
    .unwrap();
    println!("{}", stdout);

    let line = stdout
        .lines()
        .find(|line| line.starts_with("logs"))
        .unwrap();
    // smoelius: keccak256("Reason(uint256)") = 0xa8142743f8f70a4c26f3691cf4ed59718381fb2f18070ec52be1f1022d855557
    // 0x0de0b6b3a7640000 = 1000000000000000000
    assert!(line.contains(r#""topics":["0xa8142743f8f70a4c26f3691cf4ed59718381fb2f18070ec52be1f1022d855557"],"data":"0x0000000000000000000000000000000000000000000000000de0b6b3a7640000""#));
}

fn label_test_block_output(stdout: &str) -> String {
    const LABELS: &[&str] = &[
        "gasleft",
        "basefee",
        "coinbase",
        "gaslimit",
        "number",
        "timestamp",
        "chainid",
    ];
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(LABELS.len(), lines.len());
    LABELS
        .iter()
        .zip(lines)
        .map(|(label, line)| format!("{label} = {line}\n"))
        .collect()
}

fn label_test_tstore_output(stdout: &str) -> String {
    const LABELS: &[&str] = &["sload", "tload"];
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(LABELS.len(), lines.len());
    LABELS
        .iter()
        .zip(lines)
        .map(|(label, line)| format!("{label} = {line}\n"))
        .collect()
}
