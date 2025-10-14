//! This test expects you to have a devnode running:
//! <https://docs.arbitrum.io/run-arbitrum-node/run-nitro-dev-node>
//!
//! It also expects `cargo-stylus` and `cast` to be installed:
//! - <https://github.com/OffchainLabs/cargo-stylus>
//! - <https://book.getfoundry.sh/cast/>
#![warn(clippy::pedantic)]

use crate::{deploy, send, MUTEX};
use std::path::PathBuf;

#[test]
fn value() {
    let _lock = MUTEX.lock();
    let (tempdir, address) = deploy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/value.sol"),
        "C",
    )
    .unwrap();
    let dir = &tempdir;

    let stdout = send(dir, &address, ["test()", "--value=1000000000000000000"]).unwrap();
    println!("{}", &stdout);
}
