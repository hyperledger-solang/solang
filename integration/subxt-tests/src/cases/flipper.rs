// SPDX-License-Identifier: Apache-2.0

use contract_transcode::ContractMessageTranscoder;
use parity_scale_codec::{Decode, Encode};
use sp_core::hexdisplay::AsBytesRef;

use crate::{Contract, DeployContract, Execution, ReadContract, WriteContract, API};

#[tokio::test]
async fn case() -> anyhow::Result<()> {
    let api = API::new().await?;

    let mut contract = Contract::new("./contracts/flipper.contract")?;
    contract
        .upload_code(&api, sp_keyring::AccountKeyring::Alice)
        .await?;

    contract
        .deploy(
            &api,
            sp_keyring::AccountKeyring::Alice,
            10_u128.pow(16),
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("new", ["true".into()]).unwrap(),
        )
        .await?;

    // get value
    let init_value = contract
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("get", []).unwrap(),
        )
        .await
        .and_then(|v| <bool>::decode(&mut v.as_bytes_ref()).map_err(Into::into))?;

    assert!(init_value);

    // flip flipper
    contract
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("flip", []).unwrap(),
        )
        .await?;

    // get value
    let updated = contract
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("get", []).unwrap(),
        )
        .await
        .and_then(|v| <bool>::decode(&mut v.as_bytes_ref()).map_err(Into::into))?;

    assert!(!updated);

    Ok(())
}
