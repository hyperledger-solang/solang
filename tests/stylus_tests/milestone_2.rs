//! This test expects you to have a devnode running:
//! <https://docs.arbitrum.io/run-arbitrum-node/run-nitro-dev-node>
//!
//! It also expects `cargo-stylus` and `cast` to be installed:
//! - <https://github.com/OffchainLabs/cargo-stylus>
//! - <https://book.getfoundry.sh/cast/>
#![warn(clippy::pedantic)]

use crate::{call, deploy, MUTEX};
use std::path::PathBuf;

#[test]
fn milestone_2() {
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
        ["test()(uint64,uint256,address,uint256,uint256,uint256,uint256)"],
    )
    .unwrap();
    println!("{}", label(&stdout));

    stdout = call(dir, &address, ["test2()(uint256,uint256)"]).unwrap();
    println!("{}", stdout);
}

fn label(stdout: &str) -> String {
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
