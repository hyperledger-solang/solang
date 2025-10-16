//! This file contains two functions similar to ones from `cargo-stylus`'s codebase, which is
//! distributed under the terms of both the MIT license and the Apache License (Version 2.0).

use anyhow::Context;

// smoelius: `compress_wasm` is based on the function of the same name here:
// https://github.com/OffchainLabs/cargo-stylus/blob/5c520876d54594d9ca93cf017cb966075b4f4ca5/main/src/project.rs#L381
pub fn compress_wasm(wasm: &[u8]) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    use brotli2::read::BrotliEncoder;
    use std::io::Read;

    /// Maximum brotli compression level used for Stylus contracts.
    const BROTLI_COMPRESSION_LEVEL: u32 = 11;

    /// EOF prefix used in Stylus compressed WASMs on-chain
    const EOF_PREFIX_NO_DICT: &str = "EFF00000";

    let mut compressor = BrotliEncoder::new(&*wasm, BROTLI_COMPRESSION_LEVEL);
    let mut compressed_bytes = vec![];
    compressor
        .read_to_end(&mut compressed_bytes)
        .context("failed to compress WASM bytes")?;

    let mut contract_code = hex::decode(EOF_PREFIX_NO_DICT).unwrap();
    contract_code.extend(compressed_bytes);

    Ok((wasm.to_vec(), contract_code))
}

// smoelius: `contract_deployment_calldata` is based on the function of the same name here:
// https://github.com/OffchainLabs/cargo-stylus/blob/5c520876d54594d9ca93cf017cb966075b4f4ca5/main/src/deploy/mod.rs#L305
pub fn contract_deployment_calldata(code: &[u8]) -> Vec<u8> {
    let code_len: [u8; 32] = u256_from_usize(code.len());
    let mut deploy: Vec<u8> = vec![];
    deploy.push(0x7f); // PUSH32
    deploy.extend(code_len);
    deploy.push(0x80); // DUP1
    deploy.push(0x60); // PUSH1
    deploy.push(42 + 1); // prelude + version
    deploy.push(0x60); // PUSH1
    deploy.push(0x00);
    deploy.push(0x39); // CODECOPY
    deploy.push(0x60); // PUSH1
    deploy.push(0x00);
    deploy.push(0xf3); // RETURN
    deploy.push(0x00); // version
    deploy.extend(code);
    deploy
}

fn u256_from_usize(x: usize) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[32 - std::mem::size_of_val(&x)..].copy_from_slice(&x.to_be_bytes());
    bytes
}
