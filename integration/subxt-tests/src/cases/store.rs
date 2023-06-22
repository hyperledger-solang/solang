// SPDX-License-Identifier: Apache-2.0

use contract_transcode::ContractMessageTranscoder;
use hex::FromHex;
use parity_scale_codec::{Decode, DecodeAll, Encode};
use rand::Rng;
use sp_core::{hexdisplay::AsBytesRef, keccak_256, U256};

use crate::{Contract, DeployContract, Execution, ReadContract, WriteContract, API};

#[tokio::test]
async fn case() -> anyhow::Result<()> {
    let api = API::new().await?;

    let code = std::fs::read("./contracts/store.wasm")?;

    let c = Contract::new("./contracts/store.contract")?;

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

    let selector = transcoder.encode::<_, String>("get_values1", [])?;

    let res = ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        value: 0,
        selector,
    }
    .execute(&api)
    .await
    .and_then(|e| {
        <(u64, u32, i16, U256)>::decode(&mut e.return_value.as_bytes_ref()).map_err(Into::into)
    })?;

    assert_eq!(res, (0, 0, 0, U256::zero()));

    #[derive(Encode, Decode, PartialEq, Eq, Debug)]
    enum Bar {
        Bar1,
        Bar2,
        Bar3,
        Bar4,
    }

    let selector = transcoder.encode::<_, String>("get_values2", [])?;

    let res = ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        value: 0,
        selector,
    }
    .execute(&api)
    .await
    .and_then(|e| {
        <(U256, String, Vec<u8>, [u8; 4], Bar)>::decode(&mut e.return_value.as_bytes_ref())
            .map_err(Into::into)
    })?;

    assert_eq!(
        res,
        (
            U256::zero(),
            "".into(),
            hex::decode("b00b1e")?,
            <_>::from_hex("00000000")?,
            Bar::Bar1
        )
    );

    let selector = transcoder.encode::<_, String>("set_values", [])?;

    WriteContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        value: 0,
        selector,
    }
    .execute(&api)
    .await?;

    let selector = transcoder.encode::<_, String>("get_values1", [])?;

    let res = ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        value: 0,
        selector,
    }
    .execute(&api)
    .await
    .and_then(|e| {
        <(u64, u32, i16, U256)>::decode(&mut e.return_value.as_bytes_ref()).map_err(Into::into)
    })?;

    assert_eq!(
        res,
        (
            u64::from_be_bytes(<[u8; 8]>::from_hex("ffffffffffffffff")?),
            3671129839,
            32766,
            U256::from_big_endian(&<[u8; 32]>::from_hex(
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
            )?)
        )
    );

    let selector = transcoder.encode::<_, String>("get_values2", [])?;

    let res = ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        value: 0,
        selector,
    }
    .execute(&api)
    .await
    .and_then(|e| {
        <(U256, String, Vec<u8>, [u8; 4], Bar)>::decode(&mut e.return_value.as_bytes_ref())
            .map_err(Into::into)
    })?;

    assert_eq!(
        res,
        (
            U256::from_dec_str("102")?,
            "the course of true love never did run smooth".into(),
            hex::decode("b00b1e")?,
            <_>::from_hex("41424344")?,
            Bar::Bar2
        )
    );

    let selector = transcoder.encode::<_, String>("do_ops", [])?;

    WriteContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        value: 0,
        selector,
    }
    .execute(&api)
    .await?;

    let selector = transcoder.encode::<_, String>("get_values1", [])?;

    let res = ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        value: 0,
        selector,
    }
    .execute(&api)
    .await
    .and_then(|e| {
        <(u64, u32, i16, U256)>::decode(&mut e.return_value.as_bytes_ref()).map_err(Into::into)
    })?;

    assert_eq!(
        res,
        (
            1,
            65263,
            32767,
            U256::from_big_endian(&<[u8; 32]>::from_hex(
                "7ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe"
            )?)
        )
    );

    let selector = transcoder.encode::<_, String>("get_values2", [])?;

    let res = ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        value: 0,
        selector,
    }
    .execute(&api)
    .await
    .and_then(|e| {
        <(U256, String, Vec<u8>, [u8; 4], Bar)>::decode(&mut e.return_value.as_bytes_ref())
            .map_err(Into::into)
    })?;

    assert_eq!(
        res,
        (
            U256::from_dec_str("61200")?,
            "".into(),
            hex::decode("b0ff1e")?,
            <_>::from_hex("61626364")?,
            Bar::Bar4
        )
    );

    let selector = transcoder.encode::<_, String>("push_zero", [])?;

    WriteContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        value: 0,
        selector,
    }
    .execute(&api)
    .await?;

    let mut bs = "0xb0ff1e00".to_string();

    for _ in 0..20 {
        let selector = transcoder.encode::<_, String>("get_bs", [])?;

        let res = ReadContract {
            caller: sp_keyring::AccountKeyring::Alice,
            contract_address: contract.contract_address.clone(),
            value: 0,
            selector,
        }
        .execute(&api)
        .await
        .and_then(|e| <Vec<u8>>::decode(&mut e.return_value.as_bytes_ref()).map_err(Into::into))?;

        assert_eq!(res, hex::decode(&bs[2..])?);

        if bs.len() <= 4 || rand::thread_rng().gen_range(0.0_f32..1.0_f32) >= 0.5 {
            // left pad random u8 in hex
            let val = format!("{:02x}", rand::random::<u8>());

            let selector = transcoder.encode::<_, _>("push", [format!("0x{}", val)])?;

            WriteContract {
                caller: sp_keyring::AccountKeyring::Alice,
                contract_address: contract.contract_address.clone(),
                value: 0,
                selector,
            }
            .execute(&api)
            .await?;

            bs += &val;
        } else {
            let selector = transcoder.encode::<_, String>("pop", [])?;

            WriteContract {
                caller: sp_keyring::AccountKeyring::Alice,
                contract_address: contract.contract_address.clone(),
                value: 0,
                selector,
            }
            .execute(&api)
            .await?;

            bs = bs[0..bs.len() - 2].to_string();
        }
    }

    Ok(())
}
