use std::time::Duration;

use crate::{free_balance_of, node, Contract, WriteContract};
use contract_transcode::ContractMessageTranscoder;
use hex::FromHex;
use parity_scale_codec::{Decode, Encode};
use sp_core::{hexdisplay::AsBytesRef, keccak_256, KeccakHasher, H256};

use crate::{DeployContract, Execution, ReadContract, API};

#[tokio::test]
async fn case() -> anyhow::Result<()> {
    let api = API::new().await?;

    let mut contract = Contract::new("./contracts/balances.contract")?;

    contract
        .deploy(
            &api,
            sp_keyring::AccountKeyring::Alice,
            10_u128.pow(7),
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("new", []).unwrap(),
        )
        .await?;

    let contract_balance_rpc = free_balance_of(&api, contract.address.clone().unwrap()).await?;

    let rv = contract
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("get_balance", []).unwrap(),
        )
        .await?;

    let contract_balance = <u128>::decode(&mut rv.as_bytes_ref())?;
    assert!(contract_balance == contract_balance_rpc);

    contract
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            10_u128.pow(3),
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("pay_me", []).unwrap(),
        )
        .await?;

    let contract_balance_after = free_balance_of(&api, contract.address.clone().unwrap()).await?;
    assert_eq!(contract_balance + 10_u128.pow(3), contract_balance_after);

    let dave = sp_keyring::AccountKeyring::Dave;
    let dave_balance_rpc = free_balance_of(&api, dave.to_account_id()).await?;

    contract
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "transfer",
                    [
                        format!("{:?}", dave.to_raw_public()),
                        format!("{}", 20000_u128),
                    ],
                )
                .unwrap()
            },
        )
        .await?;

    let dave_balance_rpc_after = free_balance_of(&api, dave.to_account_id()).await?;

    assert_eq!(dave_balance_rpc_after, dave_balance_rpc + 20000_u128);

    contract
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "send",
                    [
                        format!("{:?}", dave.to_raw_public()),
                        format!("{}", 10000_u128),
                    ],
                )
                .unwrap()
            },
        )
        .await?;

    let dave_balance_rpc_after2 =
        free_balance_of(&api, sp_keyring::AccountKeyring::Dave.to_account_id()).await?;

    assert_eq!(dave_balance_rpc_after + 10000_u128, dave_balance_rpc_after2);

    Ok(())
}
