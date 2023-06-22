// SPDX-License-Identifier: Apache-2.0

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::{node, Contract, DeployContract, Execution, ReadContract, API};
use contract_transcode::ContractMessageTranscoder;
use parity_scale_codec::{Decode, Encode};
use sp_core::hexdisplay::AsBytesRef;

#[tokio::test]
async fn case() -> anyhow::Result<()> {
    let api = API::new().await?;

    let mut contract = Contract::new("./contracts/builtins.contract")?;

    contract
        .deploy(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("new", []).unwrap(),
        )
        .await?;

    // check ripmed160
    let input_str = "Call me Ishmael.";

    let rv = contract
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode("hash_ripemd160", [format!("0x{}", hex::encode(input_str))])
                    .unwrap()
            },
        )
        .await?;

    let expected = hex::decode("0c8b641c461e3c7abbdabd7f12a8905ee480dadf")?;
    assert_eq!(rv, expected);

    // check sha256
    let rv = contract
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode("hash_sha256", [format!("0x{}", hex::encode(input_str))])
                    .unwrap()
            },
        )
        .await?;

    let expected = hex::decode("458f3ceeeec730139693560ecf66c9c22d9c7bc7dcb0599e8e10b667dfeac043")?;
    assert_eq!(rv, expected);

    // check keccak256
    let rv = contract
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode("hash_kecccak256", [format!("0x{}", hex::encode(input_str))])
                    .unwrap()
            },
        )
        .await?;

    let expected = hex::decode("823ad8e1757b879aac338f9a18542928c668e479b37e4a56f024016215c5928c")?;
    assert_eq!(rv, expected);

    // check timestamp
    let rv = contract
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("mr_now", []).unwrap(),
        )
        .await?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let decoded = u64::decode(&mut rv.as_bytes_ref())?;

    assert!(now.as_secs() >= decoded);
    assert!(now.as_secs() < decoded + 120);

    Ok(())
}
