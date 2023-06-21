// SPDX-License-Identifier: Apache-2.0

use std::ops::Mul;

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

#[ignore = "trapped when transfer liquidity"]
#[tokio::test]
async fn mint() -> anyhow::Result<()> {
    let api = API::new().await?;

    let w = MockWorld::init(&api).await?;

    let min_liquidity: U256 = U256::from(1000_u32);

    w.token_0
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                let mut s = keccak_256(b"transfer(address,uint)")[..4].to_vec();
                w.pair.address.clone().unwrap().encode_to(&mut s);
                U256::from(10_u8).pow(18_u8.into()).encode_to(&mut s);

                s
            },
        )
        .await?;

    w.token_1
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                let mut s = keccak_256(b"transfer(address,uint)")[..4].to_vec();
                w.pair.address.clone().unwrap().encode_to(&mut s);
                U256::from(10_u8)
                    .pow(18_u8.into())
                    .mul(U256::from(4_u8))
                    .encode_to(&mut s);

                s
            },
        )
        .await?;

    let expected_liquidity = U256::from(10_u8).pow(18_u8.into()).mul(U256::from(2_u8));

    w.pair
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "mint",
                    [format!(
                        "0x{}",
                        hex::encode(sp_keyring::AccountKeyring::Alice.to_account_id())
                    )],
                )
                .unwrap()
            },
        )
        .await?;

    let total_supply = w
        .pair
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("totalSupply", []).unwrap(),
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(expected_liquidity, total_supply);

    let balance = w
        .pair
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(sp_keyring::AccountKeyring::Alice.to_account_id())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(balance, expected_liquidity - min_liquidity);

    let balance_0 = w
        .token_0
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(w.pair.address.as_ref().unwrap())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(balance_0, U256::from(10_u8).pow(18_u8.into()));

    let balance_1 = w
        .token_1
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(w.pair.address.as_ref().unwrap())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(
        balance_1,
        U256::from(10_u8).pow(18_u8.into()).mul(U256::from(4_u8))
    );

    let (r0, r1, block) = w
        .pair
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("getReserves", []).unwrap(),
        )
        .await
        .and_then(|v| <(u128, u128, u32)>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(U256::from(r0), U256::from(10_u8).pow(18_u8.into()));
    assert_eq!(
        U256::from(r1),
        U256::from(10_u8).pow(18_u8.into()).mul(U256::from(4_u8))
    );

    Ok(())
}

#[ignore = "trapped when adding liquidity"]
#[tokio::test]
async fn swap_token0() -> anyhow::Result<()> {
    let api = API::new().await?;

    let token0_amount = U256::from(10_u8)
        .pow(U256::from(18_u8))
        .mul(U256::from(5_u8));

    let token1_amount = U256::from(10_u8).pow(U256::from(19_u8));

    let w = MockWorld::init(&api).await?;

    let min_liquidity: U256 = U256::from(1000_u32);

    w.add_liquitity(&api, &token0_amount, &token1_amount)
        .await?;

    let swap_amount = U256::from(10_u8).pow(18_u8.into());
    let expected_output = U256::from_dec_str("1662497915624478906")?;

    w.token_0
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                let mut s = t
                    .encode::<_, String>(
                        "transfer",
                        [format!(
                            "0x{}",
                            hex::encode(w.pair.address.as_ref().unwrap())
                        )],
                    )
                    .unwrap();

                swap_amount.encode_to(&mut s);
                s
            },
        )
        .await?;

    w.pair
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                let mut s = t.encode::<_, String>("swap", []).unwrap();

                (
                    U256::zero(),
                    expected_output,
                    sp_keyring::AccountKeyring::Alice.to_account_id(),
                    "",
                )
                    .encode_to(&mut s);

                s
            },
        )
        .await?;

    let out = w
        .pair
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("getReserves", []).unwrap(),
        )
        .await
        .and_then(|v| {
            let t = &w.pair.transcoder;

            <(u128, u128, u32)>::decode(&mut &v[..]).map_err(Into::into)
        })?;

    assert_eq!(U256::from(out.0), token0_amount + swap_amount);
    assert_eq!(U256::from(out.1), token1_amount - expected_output);
    let bal_0 = w
        .token_0
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(w.pair.address.as_ref().unwrap())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(bal_0, token0_amount + swap_amount);

    let bal_1 = w
        .token_1
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(w.pair.address.as_ref().unwrap())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;
    assert_eq!(bal_1, token1_amount - expected_output);

    let supply_0 = w
        .token_0
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("totalSupply", []).unwrap(),
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    let supply_1 = w
        .token_1
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("totalSupply", []).unwrap(),
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    let wallet_balance_0 = w
        .token_0
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(sp_keyring::AccountKeyring::Alice.to_account_id())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    let wallet_balance_1 = w
        .token_1
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(sp_keyring::AccountKeyring::Alice.to_account_id())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(wallet_balance_0, supply_0 - token0_amount - swap_amount,);
    assert_eq!(wallet_balance_1, supply_1 - token1_amount + expected_output);

    Ok(())
}

#[ignore = "trapped when adding liquidity"]
#[tokio::test]
async fn swap_token1() -> anyhow::Result<()> {
    let api = API::new().await?;

    let token0_amount = U256::from(10_u8)
        .pow(U256::from(18_u8))
        .mul(U256::from(5_u8));

    let token1_amount = U256::from(10_u8).pow(U256::from(19_u8));

    let w = MockWorld::init(&api).await?;

    let min_liquidity: U256 = U256::from(1000_u32);

    w.add_liquitity(&api, &token0_amount, &token1_amount)
        .await?;

    let swap_amount = U256::from(10_u8).pow(18_u8.into());
    let expected_output = U256::from_dec_str("453305446940074565")?;

    w.token_1
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                let mut s = t
                    .encode::<_, String>(
                        "transfer",
                        [format!(
                            "0x{}",
                            hex::encode(w.pair.address.as_ref().unwrap())
                        )],
                    )
                    .unwrap();

                swap_amount.encode_to(&mut s);
                s
            },
        )
        .await?;

    w.pair
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                let mut s = t.encode::<_, String>("swap", []).unwrap();

                (
                    expected_output,
                    U256::zero(),
                    sp_keyring::AccountKeyring::Alice.to_account_id(),
                    "",
                )
                    .encode_to(&mut s);

                s
            },
        )
        .await?;

    let out = w
        .pair
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("getReserves", []).unwrap(),
        )
        .await
        .and_then(|v| <(u128, u128, u32)>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(U256::from(out.0), token0_amount - expected_output);
    assert_eq!(U256::from(out.1), token1_amount + swap_amount);
    let bal_0 = w
        .token_0
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(w.pair.address.as_ref().unwrap())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(bal_0, token0_amount - expected_output);

    let bal_1 = w
        .token_1
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(w.pair.address.as_ref().unwrap())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;
    assert_eq!(bal_1, token1_amount + swap_amount);

    let supply_0 = w
        .token_0
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("totalSupply", []).unwrap(),
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    let supply_1 = w
        .token_1
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("totalSupply", []).unwrap(),
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    let wallet_balance_0 = w
        .token_0
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(sp_keyring::AccountKeyring::Alice.to_account_id())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    let wallet_balance_1 = w
        .token_1
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(sp_keyring::AccountKeyring::Alice.to_account_id())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(wallet_balance_0, supply_0 - token0_amount + expected_output);
    assert_eq!(wallet_balance_1, supply_1 - token1_amount - swap_amount);

    Ok(())
}

#[ignore = "trapped when adding liquidity"]
#[tokio::test]
async fn burn() -> anyhow::Result<()> {
    let api = API::new().await?;

    let w = MockWorld::init(&api).await?;

    let token_amount = U256::from(10_u8).pow(18_u8.into()).mul(U256::from(3_u8));

    w.add_liquitity(&api, &token_amount, &token_amount).await?;

    w.pair
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                let mut s = t
                    .encode::<_, String>(
                        "transfer",
                        [format!(
                            "0x{}",
                            hex::encode(w.pair.address.as_ref().unwrap())
                        )],
                    )
                    .unwrap();

                (token_amount - U256::from(1000_u32)).encode_to(&mut s);
                s
            },
        )
        .await?;

    w.pair
        .call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "burn",
                    [format!(
                        "0x{}",
                        hex::encode(sp_keyring::AccountKeyring::Alice.to_account_id())
                    )],
                )
                .unwrap()
            },
        )
        .await?;

    let wallet_balance_0 = w
        .pair
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(sp_keyring::AccountKeyring::Alice.to_account_id())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(wallet_balance_0, U256::zero());

    let pair_supply = w
        .pair
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("totalSupply", []).unwrap(),
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(pair_supply, U256::from(1000_u32));

    let pair_balance_0 = w
        .token_0
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(w.pair.address.as_ref().unwrap())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(pair_balance_0, U256::from(1000_u32));

    let pair_balance_1 = w
        .token_1
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(w.pair.address.as_ref().unwrap())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(pair_balance_1, U256::from(1000_u32));

    let supply_0 = w
        .token_0
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("totalSupply", []).unwrap(),
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    let supply_1 = w
        .token_1
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("totalSupply", []).unwrap(),
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    let wallet_balance_0 = w
        .token_0
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(sp_keyring::AccountKeyring::Alice.to_account_id())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    let wallet_balance_1 = w
        .token_1
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode::<_, String>(
                    "balanceOf",
                    [format!(
                        "0x{}",
                        hex::encode(sp_keyring::AccountKeyring::Alice.to_account_id())
                    )],
                )
                .unwrap()
            },
        )
        .await
        .and_then(|v| <U256>::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(wallet_balance_0, supply_0 - U256::from(1000_u32));
    assert_eq!(wallet_balance_1, supply_1 - U256::from(1000_u32));

    Ok(())
}
struct MockWorld {
    factory: Contract,
    pair: Contract,
    token_0: Contract,
    token_1: Contract,
}

impl MockWorld {
    async fn init(api: &API) -> anyhow::Result<Self> {
        let mut factory = Contract::new("./contracts/UniswapV2Factory.contract")?;

        factory
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

        let mut pair = Contract::new("./contracts/UniswapV2Pair.contract")?;

        factory
            .upload_code(api, sp_keyring::AccountKeyring::Alice)
            .await?;

        let mut token_a = Contract::new("./contracts/UniswapV2ERC20.contract")?;
        token_a
            .deploy(
                api,
                sp_keyring::AccountKeyring::Alice,
                0,
                &|t: &ContractMessageTranscoder| {
                    let mut selector = t.encode::<_, String>("new", []).unwrap();

                    U256::from(10_u8).pow(22_u8.into()).encode_to(&mut selector);

                    selector
                },
            )
            .await?;

        let mut token_b = Contract::new("./contracts/UniswapV2ERC20.contract")?;
        token_b
            .deploy(
                api,
                sp_keyring::AccountKeyring::Alice,
                0,
                &|t: &ContractMessageTranscoder| {
                    let mut selector = t.encode::<_, String>("new", []).unwrap();

                    U256::from(10_u8).pow(22_u8.into()).encode_to(&mut selector);

                    selector
                },
            )
            .await?;

        factory
            .call(
                api,
                sp_keyring::AccountKeyring::Alice,
                0,
                &|t: &ContractMessageTranscoder| {
                    t.encode::<_, String>(
                        "createPair",
                        [
                            format!("0x{}", hex::encode(token_a.address.as_ref().unwrap())),
                            format!("0x{}", hex::encode(token_b.address.as_ref().unwrap())),
                        ],
                    )
                    .unwrap()
                },
            )
            .await?;

        let pair_addr = factory
            .try_call(
                api,
                sp_keyring::AccountKeyring::Alice,
                0,
                &|t: &ContractMessageTranscoder| {
                    t.encode::<_, String>(
                        "getPair",
                        [
                            format!("0x{}", hex::encode(token_a.address.as_ref().unwrap())),
                            format!("0x{}", hex::encode(token_b.address.as_ref().unwrap())),
                        ],
                    )
                    .unwrap()
                },
            )
            .await
            .and_then(|v| <AccountId32>::decode(&mut &v[..]).map_err(Into::into))?;

        pair = pair.from_addr(pair_addr)?;

        let token_0_addr = pair
            .try_call(
                api,
                sp_keyring::AccountKeyring::Alice,
                0,
                &|t: &ContractMessageTranscoder| t.encode::<_, String>("token0", []).unwrap(),
            )
            .await
            .and_then(|v| <AccountId32>::decode(&mut &v[..]).map_err(Into::into))?;

        let (token_0, token_1) = if *token_a.address.as_ref().unwrap() == token_0_addr {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };

        Ok(Self {
            factory,
            pair,
            token_0,
            token_1,
        })
    }

    async fn add_liquitity(
        &self,
        api: &API,
        amount_a: &U256,
        amount_b: &U256,
    ) -> anyhow::Result<()> {
        self.token_0
            .call(
                api,
                sp_keyring::AccountKeyring::Alice,
                0,
                &|t: &ContractMessageTranscoder| {
                    let mut s = keccak_256(b"transfer(address,uint)")[..4].to_vec();
                    self.pair.address.clone().unwrap().encode_to(&mut s);
                    amount_a.encode_to(&mut s);
                    s
                },
            )
            .await?;

        self.token_1
            .call(
                api,
                sp_keyring::AccountKeyring::Alice,
                0,
                &|t: &ContractMessageTranscoder| {
                    let mut s = keccak_256(b"transfer(address,uint)")[..4].to_vec();
                    self.pair.address.clone().unwrap().encode_to(&mut s);
                    amount_b.encode_to(&mut s);
                    s
                },
            )
            .await?;

        self.pair
            .call(
                api,
                sp_keyring::AccountKeyring::Alice,
                0,
                &|t: &ContractMessageTranscoder| {
                    let mut s = keccak_256(b"mint(address)")[..4].to_vec();
                    sp_keyring::AccountKeyring::Alice
                        .to_account_id()
                        .encode_to(&mut s);

                    s
                },
            )
            .await?;

        Ok(())
    }
}
