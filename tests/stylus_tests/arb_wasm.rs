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
fn arb_wasm() {
    let _lock = MUTEX.lock();

    let (tempdir, address_c) = deploy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/arb_wasm.sol"),
        "C",
        true,
    )
    .unwrap();
    let dir_c = &tempdir;

    let (tempdir, address_d) = deploy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration/stylus/arb_wasm.sol"),
        "D",
        false,
    )
    .unwrap();
    let dir_d = &tempdir;

    let stdout = send(
        dir_c,
        &address_c,
        [
            "forwardActivateProgram(address,address)",
            &format!("0x{:0>40x}", 0x71),
            &address_d,
            "--value=1000000000000000000",
        ],
    )
    .unwrap();
    println!("{}", &stdout);

    let stdout = call(dir_d, &address_d, ["greet()"]).unwrap();
    println!("{}", &stdout);
}
