// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use contract_transcode::ContractMessageTranscoder;
use hex::FromHex;
use num_bigint::{BigInt, BigUint, Sign};
use parity_scale_codec::{Decode, Encode, Input};
use sp_core::{crypto::AccountId32, hexdisplay::AsBytesRef, keccak_256, KeccakHasher, H256, U256};
use sp_runtime::{assert_eq_error_rate, scale_info::TypeInfo};
use sp_runtime::{traits::One, MultiAddress};

use crate::{Contract, DeployContract, Execution, ReadContract, WriteContract, API};

async fn query<T: Decode>(api: &API, addr: &AccountId32, selector: &[u8]) -> anyhow::Result<T> {
    ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: addr.clone(),
        value: 0,
        selector: selector.to_vec(),
    }
    .execute(api)
    .await
    .and_then(|v| T::decode(&mut v.return_value.as_bytes_ref()).map_err(Into::into))
}

#[tokio::test]
async fn case() -> anyhow::Result<()> {
    let api = API::new().await?;

    let mut c = Contract::new("contracts/primitives.contract")?;

    c.deploy(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
        t.encode::<_, String>("new", [])
            .expect("unable to find selector")
    })
    .await?;

    // test res
    #[derive(Encode, Decode)]
    enum Oper {
        Add,
        Sub,
        Mul,
        Div,
        Mod,
        Pow,
        Shl,
        Shr,
        Or,
        And,
        Xor,
    }

    let is_mul = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("is_mul", ["mul".into()])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| bool::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    assert!(is_mul);

    let return_div = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("return_div", [])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| Oper::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    if let Oper::Div = return_div {
    } else {
        panic!("not div");
    }

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("op_i64", ["add".into(), "1000".into(), "4100".into()])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| i64::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(res, 5100);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("op_i64", ["sub".into(), "1000".into(), "4100".into()])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| i64::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(res, -3100);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("op_i64", ["mul".into(), "1000".into(), "4100".into()])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| i64::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, 4100000);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("op_i64", ["div".into(), "1000".into(), "10".into()])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| i64::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, 100);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("op_i64", ["mod".into(), "1000".into(), "99".into()])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| i64::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(res, 10);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("op_i64", ["shl".into(), "-1000".into(), "8".into()])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| i64::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(res, -256000);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("op_i64", ["shr".into(), "-1000".into(), "8".into()])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| i64::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, -4);

    // op_u64
    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("op_u64", ["add".into(), "1000".into(), "4100".into()])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| u64::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(res, 5100);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("op_u64", ["sub".into(), "1000".into(), "4100".into()])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| u64::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(res, 18446744073709548516);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>(
                "op_u64",
                ["mul".into(), "123456789".into(), "123456789".into()],
            )
            .expect("unable to find selector")
        })
        .await
        .and_then(|rv| u64::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(res, 15241578750190521);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("op_u64", ["div".into(), "123456789".into(), "100".into()])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| u64::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, 1234567);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("op_u64", ["mod".into(), "123456789".into(), "100".into()])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| u64::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, 89);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("op_u64", ["pow".into(), "3".into(), "7".into()])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| u64::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, 2187);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("op_u64", ["shl".into(), "1000".into(), "8".into()])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| u64::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, 256000);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("op_u64", ["shr".into(), "1000".into(), "8".into()])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| u64::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, 3);

    // op_i256
    // TODO: currently contract-transcode doesn't support encoding/decoding of I256 type so we'll need  to encode it manually

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            // TODO: currently contract-transcode expects equal number of legal input args to generate the correct selector,
            // since i256 is not supported by contract-metadata we need to manually supply the selector and encode its inputs
            let mut sel = hex::decode("d6435f25").expect("unable to decode selector");

            Oper::Add.encode_to(&mut sel);
            U256::from_dec_str("1000")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            U256::from_dec_str("4100")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            sel
        })
        .await
        .and_then(|rv| U256::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(res, U256::from(5100_u128));

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            // TODO: currently contract-transcode expects equal number of legal input args to generate the correct selector,
            // since i256 is not supported by contract-metadata we need to manually supply the selector and encode its inputs
            let mut sel = hex::decode("d6435f25").expect("unable to decode selector");

            Oper::Sub.encode_to(&mut sel);
            U256::from_dec_str("1000")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            U256::from_dec_str("4100")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            sel
        })
        .await
        .and_then(|rv| U256::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    // use two's compliment to get negative value in
    assert_eq!(res, !U256::from(3100_u128) + U256::one());

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            // TODO: currently contract-transcode expects equal number of legal input args to generate the correct selector,
            // since i256 is not supported by contract-metadata we need to manually supply the selector and encode its inputs
            let mut sel = hex::decode("d6435f25").expect("unable to decode selector");

            Oper::Mul.encode_to(&mut sel);
            U256::from_dec_str("1000")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            U256::from_dec_str("4100")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            sel
        })
        .await
        .and_then(|rv| U256::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(res, U256::from_dec_str("4100000")?);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            // TODO: currently contract-transcode expects equal number of legal input args to generate the correct selector,
            // since i256 is not supported by contract-metadata we need to manually supply the selector and encode its inputs
            let mut sel = hex::decode("d6435f25").expect("unable to decode selector");

            Oper::Div.encode_to(&mut sel);
            U256::from_dec_str("1000")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            U256::from_dec_str("10")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            sel
        })
        .await
        .and_then(|rv| U256::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(res, U256::from_dec_str("100")?);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            // TODO: currently contract-transcode expects equal number of legal input args to generate the correct selector,
            // since i256 is not supported by contract-metadata we need to manually supply the selector and encode its inputs
            let mut sel = hex::decode("d6435f25").expect("unable to decode selector");

            Oper::Mod.encode_to(&mut sel);
            U256::from_dec_str("1000")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            U256::from_dec_str("99")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            sel
        })
        .await
        .and_then(|rv| U256::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, U256::from_dec_str("10")?);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            // TODO: currently contract-transcode expects equal number of legal input args to generate the correct selector,
            // since i256 is not supported by contract-metadata we need to manually supply the selector and encode its inputs
            let mut sel = hex::decode("d6435f25").expect("unable to decode selector");

            Oper::Shl.encode_to(&mut sel);
            (!U256::from_dec_str("10000000000000").unwrap() + U256::one()).encode_to(&mut sel);

            U256::from_dec_str("8")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            sel
        })
        .await
        .and_then(|rv| U256::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, !U256::from_dec_str("2560000000000000")? + U256::one());

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            // TODO: currently contract-transcode expects equal number of legal input args to generate the correct selector,
            // since i256 is not supported by contract-metadata we need to manually supply the selector and encode its inputs
            let mut sel = hex::decode("d6435f25").expect("unable to decode selector");

            Oper::Shr.encode_to(&mut sel);
            (!U256::from_dec_str("10000000000000").unwrap() + U256::one()).encode_to(&mut sel);

            U256::from_dec_str("8")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            sel
        })
        .await
        .and_then(|rv| U256::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(res, !U256::from(39062500000_i64) + U256::one());

    // op_u256
    // TODO: currently U256 from string is not supported by contract-transcode, we'll need to encode it manually

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            // TODO: currently contract-transcode expects equal number of legal input args to generate the correct selector,
            // since i256 is not supported by contract-metadata we need to manually supply the selector and encode its inputs
            let mut sel = hex::decode("b446eacd").expect("unable to decode selector");

            Oper::Add.encode_to(&mut sel);
            U256::from_dec_str("1000")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            U256::from_dec_str("4100")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            sel
        })
        .await
        .and_then(|rv| U256::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, U256::from(5100_u128));

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            // TODO: currently contract-transcode expects equal number of legal input args to generate the correct selector,
            // since i256 is not supported by contract-metadata we need to manually supply the selector and encode its inputs
            let mut sel = hex::decode("b446eacd").expect("unable to decode selector");

            Oper::Sub.encode_to(&mut sel);
            U256::from_dec_str("1000")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            U256::from_dec_str("4100")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            sel
        })
        .await
        .and_then(|rv| U256::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, !U256::from(3100_u128) + U256::one());

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            // TODO: currently contract-transcode expects equal number of legal input args to generate the correct selector,
            // since i256 is not supported by contract-metadata we need to manually supply the selector and encode its inputs
            let mut sel = hex::decode("b446eacd").expect("unable to decode selector");

            Oper::Mul.encode_to(&mut sel);
            U256::from_dec_str("123456789")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            U256::from_dec_str("123456789")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            sel
        })
        .await
        .and_then(|rv| U256::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, U256::from(15241578750190521_u128));

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            // TODO: currently contract-transcode expects equal number of legal input args to generate the correct selector,
            // since i256 is not supported by contract-metadata we need to manually supply the selector and encode its inputs
            let mut sel = hex::decode("b446eacd").expect("unable to decode selector");

            Oper::Div.encode_to(&mut sel);
            U256::from_dec_str("123456789")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            U256::from_dec_str("100")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            sel
        })
        .await
        .and_then(|rv| U256::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, U256::from(1234567_u128));

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            // TODO: currently contract-transcode expects equal number of legal input args to generate the correct selector,
            // since i256 is not supported by contract-metadata we need to manually supply the selector and encode its inputs
            let mut sel = hex::decode("b446eacd").expect("unable to decode selector");

            Oper::Mod.encode_to(&mut sel);
            U256::from_dec_str("123456789")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            U256::from_dec_str("100")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            sel
        })
        .await
        .and_then(|rv| U256::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, U256::from(89_u64));

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            // TODO: currently contract-transcode expects equal number of legal input args to generate the correct selector,
            // since i256 is not supported by contract-metadata we need to manually supply the selector and encode its inputs
            let mut sel = hex::decode("b446eacd").expect("unable to decode selector");

            Oper::Pow.encode_to(&mut sel);
            U256::from_dec_str("123456789")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            U256::from_dec_str("9")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            sel
        })
        .await
        .and_then(|rv| U256::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(
        res.to_string(),
        "6662462759719942007440037531362779472290810125440036903063319585255179509"
    );

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            // TODO: currently contract-transcode expects equal number of legal input args to generate the correct selector,
            // since i256 is not supported by contract-metadata we need to manually supply the selector and encode its inputs
            let mut sel = hex::decode("b446eacd").expect("unable to decode selector");

            Oper::Shl.encode_to(&mut sel);
            U256::from_dec_str("10000000000000")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            U256::from_dec_str("8")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            sel
        })
        .await
        .and_then(|rv| U256::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, U256::from(2560000000000000_u128));

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |_| {
            // TODO: currently contract-transcode expects equal number of legal input args to generate the correct selector,
            // since i256 is not supported by contract-metadata we need to manually supply the selector and encode its inputs
            let mut sel = hex::decode("b446eacd").expect("unable to decode selector");

            Oper::Shr.encode_to(&mut sel);
            U256::from_dec_str("10000000000000")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            U256::from_dec_str("8")
                .map(|o| o.encode_to(&mut sel))
                .expect("unable to encode to selector");
            sel
        })
        .await
        .and_then(|rv| U256::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, U256::from(39062500000_u128));

    // test bytesN
    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("return_u8_6", [])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| <[u8; 6]>::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(hex::encode(res), "414243444546");

    // test bytesS
    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, _>("op_u8_5_shift", ["shl", "0xdeadcafe59", "8"])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| <[u8; 5]>::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(hex::encode(res), "adcafe5900");

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, _>("op_u8_5_shift", ["shr", "0xdeadcafe59", "8"])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| <[u8; 5]>::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(hex::encode(res), "00deadcafe");

    // opU85
    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, _>("op_u8_5", ["or", "0xdeadcafe59", "0x0000000006"])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| <[u8; 5]>::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(hex::encode(res), "deadcafe5f");

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, _>("op_u8_5", ["and", "0xdeadcafe59", "0x00000000ff"])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| <[u8; 5]>::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(hex::encode(res), "0000000059");

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, _>("op_u8_5", ["xor", "0xdeadcafe59", "0x00000000ff"])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| <[u8; 5]>::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(hex::encode(res), "deadcafea6");

    // test bytes14
    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, _>(
                "op_u8_14_shift",
                ["shl", "0xdeadcafe123456789abcdefbeef7", "9"],
            )
            .expect("unable to find selector")
        })
        .await
        .and_then(|rv| <[u8; 14]>::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(hex::encode(res), "5b95fc2468acf13579bdf7ddee00");

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, _>(
                "op_u8_14_shift",
                ["shr", "0xdeadcafe123456789abcdefbeef7", "9"],
            )
            .expect("unable to find selector")
        })
        .await
        .and_then(|rv| <[u8; 14]>::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(hex::encode(res), "006f56e57f091a2b3c4d5e6f7df7");

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, _>(
                "op_u8_14",
                [
                    "or",
                    "0xdeadcafe123456789abcdefbeef7",
                    "0x0000060000000000000000000000",
                ],
            )
            .expect("unable to find selector")
        })
        .await
        .and_then(|rv| <[u8; 14]>::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(hex::encode(res), "deadcefe123456789abcdefbeef7");

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, _>(
                "op_u8_14",
                [
                    "and",
                    "0xdeadcafe123456789abcdefbeef7",
                    "0x000000000000000000ff00000000",
                ],
            )
            .expect("unable to find selector")
        })
        .await
        .and_then(|rv| <[u8; 14]>::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(hex::encode(res), "000000000000000000bc00000000");

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, _>(
                "op_u8_14",
                [
                    "xor",
                    "0xdeadcafe123456789abcdefbeef7",
                    "0xff00000000000000000000000000",
                ],
            )
            .expect("unable to find selector")
        })
        .await
        .and_then(|rv| <[u8; 14]>::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(hex::encode(res), "21adcafe123456789abcdefbeef7");

    // test addressPassthrough
    let default_acc =
        AccountId32::from_str("5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sjeZ").unwrap();

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, _>(
                "address_passthrough",
                [format!("0x{}", hex::encode(&default_acc))],
            )
            .expect("unable to find selector")
        })
        .await
        .and_then(|rv| AccountId32::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, default_acc);

    let alice = sp_keyring::AccountKeyring::Alice.to_account_id();

    let dave = sp_keyring::AccountKeyring::Dave.to_account_id();

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, _>(
                "address_passthrough",
                [format!("0x{}", hex::encode(&alice))],
            )
            .expect("unable to find selector")
        })
        .await
        .and_then(|rv| AccountId32::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, alice);

    let res = c
        .try_call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, _>("address_passthrough", [format!("0x{}", hex::encode(&dave))])
                .expect("unable to find selector")
        })
        .await
        .and_then(|rv| AccountId32::decode(&mut rv.as_bytes_ref()).map_err(Into::into))?;
    assert_eq!(res, dave);

    Ok(())
}
