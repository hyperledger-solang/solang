use contract_transcode::ContractMessageTranscoder;
use hex::FromHex;

use parity_scale_codec::{Decode, DecodeAll, Encode, Input};
use rand::Rng;
use sp_core::{crypto::AccountId32, hexdisplay::AsBytesRef, keccak_256, U256};

use crate::{Contract, DeployContract, Execution, ReadContract, WriteContract, API};

#[tokio::test]
async fn setup() -> anyhow::Result<()> {
    let api = API::new().await?;

    let w = MockWorld::init(&api).await?;

    let rs = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("name", [])
                .expect("unable to find selector")
        })
        .await
        .and_then(|v| String::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(rs, "Uniswap V2");

    let rs = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("symbol", [])
                .expect("unable to find selector")
        })
        .await
        .and_then(|v| String::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(rs, "UNI-V2");

    let rs = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("decimals", [])
                .expect("unable to find selector")
        })
        .await
        .and_then(|v| u8::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(rs, 18);

    let rs = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("totalSupply", [])
                .expect("unable to find selector")
        })
        .await
        .and_then(|v| U256::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(
        rs,
        U256::from_dec_str("10000")? * U256::from(10).pow(18_u8.into())
    );
    let rs = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("balanceOf", [format!("0x{}", hex::encode(&w.alice))])
                .expect("unable to find selector")
        })
        .await
        .and_then(|v| U256::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(
        rs,
        U256::from_dec_str("10000")? * U256::from(10).pow(18_u8.into())
    );

    let rs = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("DOMAIN_SEPARATOR", [])
                .expect("unable to find selector")
        })
        .await
        .and_then(|v| <[u8; 32]>::decode(&mut &v[..]).map_err(Into::into))?;

    let expected = [
        keccak_256(
            "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
                .as_bytes(),
        )
        .to_vec(),
        keccak_256("Uniswap V2".as_bytes()).to_vec(),
        keccak_256("1".as_bytes()).to_vec(),
        hex::decode("0100000000000000000000000000000000000000000000000000000000000000")?,
        AsRef::<[u8; 32]>::as_ref(&w.token_addr).to_vec(),
    ]
    .concat();

    let expected = keccak_256(&expected[..]);
    assert_eq!(rs, expected);

    let rs = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("PERMIT_TYPEHASH", [])
                .expect("unable to find selector")
        })
        .await
        .and_then(|v| <[u8; 32]>::decode(&mut &v[..]).map_err(Into::into))?;
    assert_eq!(
        rs,
        <[u8; 32]>::from_hex("6e71edae12b1b97f4d1f60370fef10105fa2faae0126114a169c64845d6126c9")?
    );

    Ok(())
}

struct MockWorld {
    alice: AccountId32,
    dave: AccountId32,
    token_addr: AccountId32,
    contract: Contract,
}

#[tokio::test]
async fn approve() -> anyhow::Result<()> {
    let api = API::new().await?;

    let w = MockWorld::init(&api).await?;

    w.contract
        .call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            let mut sel = keccak_256(b"approve(address,uint256)")[..4].to_vec();
            w.dave.encode_to(&mut sel);
            U256::from(10_u128.pow(18)).encode_to(&mut sel);
            sel
        })
        .await?;

    let rs = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>(
                "allowance",
                [
                    format!("0x{}", hex::encode(&w.alice)),
                    format!("0x{}", hex::encode(&w.dave)),
                ],
            )
            .expect("unable to find selector")
        })
        .await
        .and_then(|v| U256::decode(&mut &v[..]).map_err(Into::into))?;
    assert_eq!(rs, U256::from(10_u128.pow(18)));

    Ok(())
}

#[tokio::test]
async fn transfer() -> anyhow::Result<()> {
    let api = API::new().await?;

    let w = MockWorld::init(&api).await?;

    w.contract
        .call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            let mut sel = keccak_256(b"transfer(address,uint256)")[..4].to_vec();
            w.dave.encode_to(&mut sel);
            U256::from(10_u128.pow(18)).encode_to(&mut sel);
            sel
        })
        .await?;

    let alice_balance = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("balanceOf", [format!("0x{}", hex::encode(&w.alice))])
                .expect("unable to find selector")
        })
        .await
        .and_then(|v| U256::decode(&mut &v[..]).map_err(Into::into))?;

    let dave_balance = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("balanceOf", [format!("0x{}", hex::encode(&w.dave))])
                .expect("unable to find selector")
        })
        .await
        .and_then(|v| U256::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(
        alice_balance,
        U256::from_dec_str("10000")? * U256::from(10).pow(18_u8.into())
            - U256::from(10_u128.pow(18))
    );

    assert_eq!(dave_balance, U256::from(10_u128.pow(18)));

    Ok(())
}

#[tokio::test]
async fn transfer_from() -> anyhow::Result<()> {
    let api = API::new().await?;

    let w = MockWorld::init(&api).await?;

    w.contract
        .call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            let mut sel = keccak_256(b"approve(address,uint256)")[..4].to_vec();
            w.dave.encode_to(&mut sel);
            U256::from(10_u128.pow(18)).encode_to(&mut sel);
            sel
        })
        .await?;

    w.contract
        .call(&api, sp_keyring::AccountKeyring::Dave, 0, |_| {
            let mut sel = keccak_256(b"transferFrom(address,address,uint256)")[..4].to_vec();
            w.alice.encode_to(&mut sel);
            w.dave.encode_to(&mut sel);
            U256::from(10_u128.pow(18)).encode_to(&mut sel);

            sel
        })
        .await?;

    let rs = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>(
                "allowance",
                [
                    format!("0x{}", hex::encode(&w.alice)),
                    format!("0x{}", hex::encode(&w.dave)),
                ],
            )
            .expect("unable to find selector")
        })
        .await
        .and_then(|v| U256::decode(&mut &v[..]).map_err(Into::into))?;
    assert_eq!(rs, 0_u8.into());

    let alice_balance = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("balanceOf", [format!("0x{}", hex::encode(&w.alice))])
                .expect("unable to find selector")
        })
        .await
        .and_then(|v| U256::decode(&mut &v[..]).map_err(Into::into))?;
    assert_eq!(
        alice_balance,
        U256::from_dec_str("10000")? * U256::from(10).pow(18_u8.into())
            - U256::from(10_u128.pow(18))
    );

    let dave_balance = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("balanceOf", [format!("0x{}", hex::encode(&w.dave))])
                .expect("unable to find selector")
        })
        .await
        .and_then(|v| U256::decode(&mut &v[..]).map_err(Into::into))?;
    assert_eq!(
        alice_balance,
        U256::from_dec_str("10000")? * U256::from(10).pow(18_u8.into())
            - U256::from(10_u128.pow(18))
    );

    assert_eq!(dave_balance, U256::from(10_u128.pow(18)));

    Ok(())
}

#[tokio::test]
async fn transfer_from_max() -> anyhow::Result<()> {
    let api = API::new().await?;

    let w = MockWorld::init(&api).await?;

    w.contract
        .call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            let mut sel = keccak_256(b"approve(address,uint256)")[..4].to_vec();
            w.dave.encode_to(&mut sel);
            U256::MAX.encode_to(&mut sel);
            sel
        })
        .await?;

    w.contract
        .call(&api, sp_keyring::AccountKeyring::Dave, 0, |_| {
            let mut sel = keccak_256(b"transferFrom(address,address,uint256)")[..4].to_vec();
            w.alice.encode_to(&mut sel);
            w.dave.encode_to(&mut sel);
            U256::from(10_u128.pow(18)).encode_to(&mut sel);

            sel
        })
        .await?;

    let rs = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>(
                "allowance",
                [
                    format!("0x{}", hex::encode(&w.alice)),
                    format!("0x{}", hex::encode(&w.dave)),
                ],
            )
            .expect("unable to find selector")
        })
        .await
        .and_then(|v| U256::decode(&mut &v[..]).map_err(Into::into))?;

    assert_eq!(rs, U256::MAX);

    let alice_balance = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("balanceOf", [format!("0x{}", hex::encode(&w.alice))])
                .expect("unable to find selector")
        })
        .await
        .and_then(|v| U256::decode(&mut &v[..]).map_err(Into::into))?;
    assert_eq!(
        alice_balance,
        U256::from_dec_str("10000")? * U256::from(10).pow(18_u8.into())
            - U256::from(10_u128.pow(18))
    );

    let dave_balance = w
        .contract
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("balanceOf", [format!("0x{}", hex::encode(&w.dave))])
                .expect("unable to find selector")
        })
        .await
        .and_then(|v| U256::decode(&mut &v[..]).map_err(Into::into))?;
    assert_eq!(
        alice_balance,
        U256::from_dec_str("10000")? * U256::from(10).pow(18_u8.into())
            - U256::from(10_u128.pow(18))
    );

    assert_eq!(dave_balance, U256::from(10_u128.pow(18)));

    Ok(())
}

impl MockWorld {
    async fn init(api: &API) -> anyhow::Result<Self> {
        let alice: AccountId32 = sp_keyring::AccountKeyring::Alice.to_account_id();
        let dave: AccountId32 = sp_keyring::AccountKeyring::Dave.to_account_id();

        let mut c = Contract::new("./contracts/ERC20.contract")?;

        c.deploy(api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            let mut sel = hex::decode("5816c425").expect("unable to decode");
            U256::from_dec_str("10000")
                .map(|t| t * U256::from(10).pow(18_u8.into()))
                .unwrap()
                .encode_to(&mut sel);
            sel
        })
        .await?;

        Ok(Self {
            alice,
            dave,
            token_addr: c.address.clone().unwrap(),
            contract: c,
        })
    }
}
