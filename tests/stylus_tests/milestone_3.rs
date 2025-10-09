//! This test expects you to have a devnode running:
//! <https://docs.arbitrum.io/run-arbitrum-node/run-nitro-dev-node>
//!
//! It also expects `cargo-stylus` and `cast` to be installed:
//! - <https://github.com/OffchainLabs/cargo-stylus>
//! - <https://book.getfoundry.sh/cast/>
#![warn(clippy::pedantic)]

use crate::{call, deploy, send, MUTEX};
use std::path::PathBuf;
use tiny_keccak::{Hasher, Keccak};

#[test]
fn milestone_3() {
    let _lock = MUTEX.lock();
    let (tempdir, address) = deploy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/milestone_3.sol"),
        "C",
    )
    .unwrap();
    let dir = &tempdir;

    let stdout = send(dir, &address, ["accept_donation()", "--value=1000"]).unwrap();
    println!("{}", &stdout);

    let stdout = call(
        dir,
        &address,
        [
            "test()(uint256,bytes32,bytes32,uint256)",
            "--gas-price=100000000",
        ],
    )
    .unwrap();
    let labeled_stdout = label(&stdout);
    println!("{}", &labeled_stdout);

    let balance = get(&labeled_stdout, "balance").unwrap();
    assert_eq!(1000, u64::from_str_radix(balance, 10).unwrap());

    let codehash = get(&labeled_stdout, "codehash")
        .map(|s| s.strip_prefix("0x"))
        .flatten()
        .unwrap();

    let stdout = call(dir, &address, ["getCode()"]).unwrap();
    let len_prefixed_code = stdout.strip_prefix("0x").unwrap();
    let len = usize::from_str_radix(&len_prefixed_code[..64], 16).unwrap();
    let code = hex::decode(&len_prefixed_code[64..].trim_end()).unwrap();
    assert_eq!(len, code.len());
    let digest = keccak256(&code);
    assert_eq!(codehash, hex::encode(digest));

    let gasprice = get(&labeled_stdout, "gasprice").unwrap();
    let i = gasprice
        .bytes()
        .position(|c| c.is_ascii_whitespace())
        .unwrap_or_else(|| gasprice.len());
    assert_eq!(100000000, u64::from_str_radix(&gasprice[..i], 10).unwrap());

    call(dir, &address, ["test_addmod()"]).unwrap();

    call(dir, &address, ["test_mulmod()"]).unwrap();

    call(dir, &address, ["test_div()"]).unwrap();

    call(dir, &address, ["test_mod()"]).unwrap();

    call(dir, &address, ["test_power()"]).unwrap();
}

fn label(stdout: &str) -> String {
    const LABELS: &[&str] = &["balance", "codehash", "manual_codehash", "gasprice"];
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(LABELS.len(), lines.len());
    LABELS
        .iter()
        .zip(lines)
        .map(|(label, line)| format!("{label} = {line}\n"))
        .collect()
}

fn get<'a>(stdout: &'a str, label: &str) -> Option<&'a str> {
    let prefix = format!("{label} = ");
    stdout.lines().find_map(|line| line.strip_prefix(&prefix))
}

fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(&input);
    hasher.finalize(&mut output);
    output
}
