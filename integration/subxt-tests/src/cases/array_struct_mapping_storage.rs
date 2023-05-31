use contract_transcode::ContractMessageTranscoder;
use parity_scale_codec::{Decode, Encode};
use sp_core::{hexdisplay::AsBytesRef, U256};

use crate::{Contract, DeployContract, Execution, ReadContract, WriteContract, API};

#[tokio::test]
async fn case() -> anyhow::Result<()> {
    let api = API::new().await?;

    let mut contract = Contract::new("./contracts/array_struct_mapping_storage.contract")?;

    contract
        .deploy(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("new", []).unwrap(),
        )
        .await?;

    contract
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, _>("setNumber", ["2147483647"]).unwrap(),
        )
        .await?;

    let b_push = |t: &ContractMessageTranscoder| t.encode::<_, String>("push", []).unwrap();

    contract
        .call(&api, sp_keyring::AccountKeyring::Alice, 0, &b_push)
        .await?;

    contract
        .call(&api, sp_keyring::AccountKeyring::Alice, 0, &b_push)
        .await?;

    for array_no in 0..2 {
        for i in 0..10 {
            contract
                .call(
                    &api,
                    sp_keyring::AccountKeyring::Alice,
                    0,
                    &|t: &ContractMessageTranscoder| {
                        t.encode::<_, _>(
                            "set",
                            [
                                format!("{}", array_no),
                                format!("{}", 102 + i + array_no * 500),
                                format!("{}", 300331 + i),
                            ],
                        )
                        .unwrap()
                    },
                )
                .await?;
        }
    }

    for array_no in 0..2 {
        for i in 0..10 {
            let rs = contract
                .try_call(
                    &api,
                    sp_keyring::AccountKeyring::Alice,
                    0,
                    &|t: &ContractMessageTranscoder| {
                        t.encode::<_, _>(
                            "get",
                            [
                                format!("{}", array_no),
                                format!("{}", 102 + i + array_no * 500),
                            ],
                        )
                        .unwrap()
                    },
                )
                .await
                .and_then(|v| <U256>::decode(&mut v.as_bytes_ref()).map_err(Into::into))?;

            assert_eq!(rs, U256::from(300331_u128 + i));
        }
    }

    contract
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, _>("rm", [format!("{}", 0), format!("{}", 104)])
                    .unwrap()
            },
        )
        .await?;

    for i in 0..10 {
        let rs = contract
            .try_call(
                &api,
                sp_keyring::AccountKeyring::Alice,
                0,
                &|t: &ContractMessageTranscoder| {
                    t.encode::<_, _>("get", [format!("{}", 0), format!("{}", 102 + i)])
                        .unwrap()
                },
            )
            .await
            .and_then(|v| <U256>::decode(&mut v.as_bytes_ref()).map_err(Into::into))?;

        if i != 2 {
            assert_eq!(rs, U256::from(300331_u128 + i));
        } else {
            assert_eq!(rs, U256::zero());
        }
    }

    let rs = contract
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("number", []).unwrap(),
        )
        .await
        .and_then(|v| <i64>::decode(&mut v.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(rs, 2147483647);

    Ok(())
}
