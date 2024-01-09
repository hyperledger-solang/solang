// SPDX-License-Identifier: Apache-2.0

use contract_transcode::ContractMessageTranscoder;
use parity_scale_codec::{Decode, Encode};
use sp_core::hexdisplay::AsBytesRef;

use crate::{
    free_balance_of, Contract, DeployContract, Execution, ReadContract, WriteContract, API,
};

#[tokio::test]
async fn case() -> anyhow::Result<()> {
    let api = API::new().await?;
    let code = std::fs::read("./contracts/destruct.wasm")?;

    let c = Contract::new("./contracts/destruct.contract")?;

    let transcoder = &c.transcoder;

    let selector = transcoder.encode::<_, String>("new", [])?;

    let deployed = DeployContract {
        caller: sp_keyring::AccountKeyring::Alice,
        selector,
        value: 0,
        code,
    }
    .execute(&api)
    .await?;

    let selector = transcoder.encode::<_, String>("hello", [])?;

    let rv = ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: deployed.contract_address.clone(),
        value: 0,
        selector,
    }
    .execute(&api)
    .await
    .and_then(|v| <String>::decode(&mut v.return_value.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(rv, "Hello");

    let dave_before =
        free_balance_of(&api, sp_keyring::AccountKeyring::Dave.to_account_id()).await?;
    let contract_before = free_balance_of(&api, deployed.contract_address.clone()).await?;

    let selector = transcoder.encode::<_, String>(
        "selfterminate",
        [format!(
            "0x{}",
            hex::encode(sp_keyring::AccountKeyring::Dave.to_account_id())
        )],
    )?;

    WriteContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: deployed.contract_address.clone(),
        selector,
        value: 0,
    }
    .execute(&api)
    .await?;

    let dave_after =
        free_balance_of(&api, sp_keyring::AccountKeyring::Dave.to_account_id()).await?;
    let contract_after = free_balance_of(&api, deployed.contract_address.clone()).await?;

    assert_eq!(contract_after, 0);

    let existential_deposit = 1000000000;
    assert_eq!(
        dave_after,
        dave_before + contract_before + existential_deposit
    );

    Ok(())
}
