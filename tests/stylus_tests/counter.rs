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

use crate::{call, deploy, send, MUTEX};
use std::path::PathBuf;

#[allow(clippy::too_many_lines)]
#[test]
fn counter() {
    let _lock = MUTEX.lock();
    let (tempdir, address) = deploy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/counter.sol"),
        "Counter",
    )
    .unwrap();
    let dir = &tempdir;

    let stdout = call(dir, &address, ["number()(uint256)"]).unwrap();
    assert_eq!("0\n", stdout);

    send(dir, &address, ["increment()"]).unwrap();

    let stdout = call(dir, &address, ["number()(uint256)"]).unwrap();
    assert_eq!("1\n", stdout);

    send(dir, &address, ["setNumber(uint256)", "5"]).unwrap();

    let stdout = call(dir, &address, ["number()(uint256)"]).unwrap();
    assert_eq!("5\n", stdout);
}
