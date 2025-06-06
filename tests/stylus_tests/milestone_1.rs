//! This test expects you to have a devnode running:
//! <https://docs.arbitrum.io/run-arbitrum-node/run-nitro-dev-node>
//!
//! It also expects `cargo-stylus` and `cast` to be installed:
//! - <https://github.com/OffchainLabs/cargo-stylus>
//! - <https://book.getfoundry.sh/cast/>
#![warn(clippy::pedantic)]

use crate::{call, deploy, send, MUTEX};
use std::path::PathBuf;

#[test]
fn milestone_1() {
    let _lock = MUTEX.lock();
    let (tempdir, address) = deploy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/milestone_1.sol"),
        "C",
    )
    .unwrap();
    let dir = &tempdir;

    send(dir, &address, ["test()"]).unwrap();

    let stdout = call(dir, &address, ["get()(uint256)"]).unwrap();
    assert_eq!("3\n", stdout);
}
