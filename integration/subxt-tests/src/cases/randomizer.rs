// SPDX-License-Identifier: Apache-2.0

use contract_transcode::ContractMessageTranscoder;
use parity_scale_codec::{Decode, Encode};
use sp_core::{hexdisplay::AsBytesRef, keccak_256};

use crate::{Contract, DeployContract, Execution, ReadContract, WriteContract, API};

#[ignore]
#[tokio::test]
async fn case() -> anyhow::Result<()> {
    let api = API::new().await?;

    let code = std::fs::read("./contracts/randomizer.wasm")?;

    let c = Contract::new("./contracts/randomizer.contract")?;

    let transcoder = &c.transcoder;

    let selector = transcoder.encode::<_, String>("new", [])?;

    let contract = DeployContract {
        caller: sp_keyring::AccountKeyring::Alice,
        selector,
        value: 0,
        code,
    }
    .execute(&api)
    .await?;

    let selector =
        transcoder.encode::<_, _>("get_random", [format!("{:?}", "01234567".as_bytes())])?;

    let rs = ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        value: 0,
        selector: selector.clone(),
    }
    .execute(&api)
    .await
    .and_then(|v| <[u8; 32]>::decode(&mut v.return_value.as_bytes_ref()).map_err(Into::into))?;

    WriteContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        value: 0,
        selector,
    }
    .execute(&api)
    .await?;

    let selector = transcoder.encode::<_, String>("value", [])?;

    let tx_rs = ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        value: 0,
        selector,
    }
    .execute(&api)
    .await
    .and_then(|v| <[u8; 32]>::decode(&mut v.return_value.as_bytes_ref()).map_err(Into::into))?;

    assert_ne!(rs, [0_u8; 32]);
    assert_ne!(tx_rs, [0_u8; 32]);
    assert_ne!(rs, tx_rs);

    Ok(())
}
