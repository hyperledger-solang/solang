use contract_transcode::ContractMessageTranscoder;
use hex::FromHex;
use parity_scale_codec::{Decode, DecodeAll, Encode, Input};
use rand::Rng;
use sp_core::{
    crypto::{AccountId32, Ss58Codec},
    hexdisplay::AsBytesRef,
    keccak_256, U256,
};

use crate::{Contract, DeployContract, Execution, ReadContract, WriteContract, API};

#[tokio::test]
async fn setup() -> anyhow::Result<()> {
    let api = API::new().await?;

    let w = MockWorld::init(&api).await?;

    let rs = w
        .factory
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("feeTo", []).unwrap(),
        )
        .await
        .and_then(|v| <AccountId32>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(
        rs,
        AccountId32::from_string("5C4hrfjw9DjXZTzV3MwzrrAr9P1MJhSrvWGWqi1eSuyUpnhM")?
    );

    let rs = w
        .factory
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("feeToSetter", []).unwrap(),
        )
        .await
        .and_then(|v| <AccountId32>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(rs, sp_keyring::AccountKeyring::Alice.to_account_id());

    let rs = w
        .factory
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("allPairsLength", []).unwrap(),
        )
        .await
        .and_then(|v| <u8>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(rs, 0);

    Ok(())
}

#[tokio::test]
async fn test_pair() -> anyhow::Result<()> {
    let api = API::new().await?;

    let w = MockWorld::init(&api).await?;

    w.factory
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "createPair",
                    [
                        format!(
                            "0x{}",
                            hex::encode(
                                AccountId32::from_string(
                                    "5C4hrfjw9DjXZTzV3MwzrrAr9P1MJhSrvWGWqi1eSuyUv7BA"
                                )
                                .unwrap()
                            )
                        ),
                        format!(
                            "0x{}",
                            hex::encode(
                                AccountId32::from_string(
                                    "5C4hrfjw9DjXZTzV3MwzrrAr9P1MJhSrvWGWqi1eSuyV1W6M"
                                )
                                .unwrap()
                            )
                        ),
                    ],
                )
                .unwrap()
            },
        )
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_pair_reverse() -> anyhow::Result<()> {
    let api = API::new().await?;

    let w = MockWorld::init(&api).await?;

    w.factory
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "createPair",
                    [
                        format!(
                            "0x{}",
                            hex::encode(
                                AccountId32::from_string(
                                    "5C4hrfjw9DjXZTzV3MwzrrAr9P1MJhSrvWGWqi1eSuyV1W6M"
                                )
                                .unwrap()
                            )
                        ),
                        format!(
                            "0x{}",
                            hex::encode(
                                AccountId32::from_string(
                                    "5C4hrfjw9DjXZTzV3MwzrrAr9P1MJhSrvWGWqi1eSuyUv7BA"
                                )
                                .unwrap()
                            )
                        ),
                    ],
                )
                .unwrap()
            },
        )
        .await?;

    Ok(())
}

#[tokio::test]
async fn set_fee_to() -> anyhow::Result<()> {
    let api = API::new().await?;

    let w = MockWorld::init(&api).await?;

    w.factory
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "setFeeTo",
                    [format!(
                        "0x{}",
                        hex::encode(sp_keyring::AccountKeyring::Dave.to_account_id())
                    )],
                )
                .unwrap()
            },
        )
        .await?;

    let rs = w
        .factory
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("feeTo", []).unwrap(),
        )
        .await
        .and_then(|v| <AccountId32>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(rs, sp_keyring::AccountKeyring::Dave.to_account_id());

    Ok(())
}

#[tokio::test]
async fn set_fee_to_setter() -> anyhow::Result<()> {
    let api = API::new().await?;

    let w = MockWorld::init(&api).await?;

    w.factory
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "setFeeToSetter",
                    [format!(
                        "0x{}",
                        hex::encode(sp_keyring::AccountKeyring::Dave.to_account_id())
                    )],
                )
                .unwrap()
            },
        )
        .await?;

    let rs = w
        .factory
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("feeToSetter", []).unwrap(),
        )
        .await
        .and_then(|v| <AccountId32>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(rs, sp_keyring::AccountKeyring::Dave.to_account_id());

    Ok(())
}

struct MockWorld {
    factory: Contract,
}

impl MockWorld {
    async fn init(api: &API) -> anyhow::Result<Self> {
        let mut contract = Contract::new("./contracts/UniswapV2Factory.contract")?;

        Contract::new("./contracts/UniswapV2Pair.contract")?
            .upload_code(api, sp_keyring::AccountKeyring::Alice)
            .await?;

        contract
            .deploy(
                api,
                sp_keyring::AccountKeyring::Alice,
                10_u128.pow(16),
                &|t: &ContractMessageTranscoder| {
                    t.encode::<_, String>(
                        "new",
                        [format!(
                            "0x{}",
                            hex::encode(&sp_keyring::AccountKeyring::Alice.to_account_id())
                        )],
                    )
                    .unwrap()
                },
            )
            .await?;

        Ok(Self { factory: contract })
    }
}
