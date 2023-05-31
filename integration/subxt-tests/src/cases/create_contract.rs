use std::str::FromStr;

use contract_transcode::ContractMessageTranscoder;
use hex::FromHex;
use parity_scale_codec::{Decode, Encode};
use sp_core::{crypto::AccountId32, hexdisplay::AsBytesRef, H256};

use crate::{
    free_balance_of, node, Contract, DeployContract, Execution, ReadContract, ReadLayout,
    WriteContract, API,
};

#[tokio::test]
async fn case() -> anyhow::Result<()> {
    let api = API::new().await?;

    let mut c_creator = Contract::new("./contracts/creator.contract")?;

    c_creator
        .deploy(
            &api,
            sp_keyring::AccountKeyring::Alice,
            10_u128.pow(16),
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("new", []).unwrap(),
        )
        .await?;

    let mut c_child = Contract::new("./contracts/child_create_contract.contract")?;
    c_child
        .upload_code(&api, sp_keyring::AccountKeyring::Alice)
        .await?;

    c_creator
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0_u128,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("create_child", []).unwrap(),
        )
        .await?;

    let rv = c_creator
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0_u128,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("call_child", []).unwrap(),
        )
        .await
        .and_then(|v| String::decode(&mut v.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(rv, "child");

    let child_addr = c_creator
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0_u128,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("c", []).unwrap(),
        )
        .await
        .and_then(|v| <AccountId32>::decode(&mut v.as_bytes_ref()).map_err(Into::into))?;

    c_child.address.replace(child_addr.clone());
    let child_balance_rpc = free_balance_of(&api, child_addr).await?;
    assert!(child_balance_rpc != 0);
    let creator_balance_rpc = free_balance_of(&api, c_creator.address.unwrap()).await?;
    assert!(creator_balance_rpc < 10_u128.pow(16));

    let rv = c_child
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0_u128,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("say_my_name", []).unwrap(),
        )
        .await
        .and_then(|v| String::decode(&mut v.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(rv, "child");

    Ok(())
}
