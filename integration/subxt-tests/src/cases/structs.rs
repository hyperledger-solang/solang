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

    let code = std::fs::read("./contracts/structs.wasm")?;

    let c = Contract::new("./contracts/structs.contract")?;

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

    let selector = transcoder.encode::<_, String>("set_foo1", [])?;

    WriteContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        selector,
        value: 0,
    }
    .execute(&api)
    .await?;

    let selector = transcoder.encode::<_, String>("get_both_foos", [])?;

    #[derive(Encode, Decode, Eq, PartialEq, Debug)]
    enum enum_bar {
        bar1,
        bar2,
        bar3,
        bar4,
    }

    #[derive(Encode, Decode, Eq, PartialEq, Debug)]
    struct struct_foo {
        f1: enum_bar,
        f2: Vec<u8>,
        f3: i64,
        f4: [u8; 3],
        f5: String,
        f6: inner_foo,
    }

    #[derive(Encode, Decode, Eq, PartialEq, Debug)]
    struct inner_foo {
        in1: bool,
        in2: String,
    }

    let rs = ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        selector,
        value: 0,
    }
    .execute(&api)
    .await
    .and_then(|v| {
        <(struct_foo, struct_foo)>::decode(&mut &v.return_value[..]).map_err(Into::into)
    })?;

    assert_eq!(rs,
     (
        struct_foo { f1: enum_bar::bar2, f2: hex::decode("446f6e277420636f756e7420796f757220636869636b656e73206265666f72652074686579206861746368")?, f3: -102, f4: <_>::from_hex("edaeda")?, f5: "You can't have your cake and eat it too".into(), f6: inner_foo { in1: true, in2: "There are other fish in the sea".into() } },
        struct_foo { f1: enum_bar::bar1, f2: vec![], f3: 0, f4: <_>::from_hex("000000")?, f5: String::new(), f6:inner_foo { in1: false, in2: "".into()} } 
     )
    );

    // TODO: find a way to generate signature with enum input
    let mut selector = hex::decode("9c408762").unwrap();

    let mut input = struct_foo {
        f1: enum_bar::bar2,
        f2: hex::decode("b52b073595ccb35eaebb87178227b779")?,
        f3: -123112321,
        f4: <_>::from_hex("123456")?,
        f5: "Barking up the wrong tree".into(),
        f6: inner_foo {
            in1: true,
            in2: "Drive someone up the wall".into(),
        },
    };

    input.encode_to(&mut selector);
    "nah".encode_to(&mut selector);

    input.f6.in2 = "nah".into();

    WriteContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        selector,
        value: 0,
    }
    .execute(&api)
    .await?;

    let selector = transcoder.encode::<_, _>("get_foo", ["false"])?;

    let rs = ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        selector,
        value: 0,
    }
    .execute(&api)
    .await
    .and_then(|v| <struct_foo>::decode(&mut &v.return_value[..]).map_err(Into::into))?;

    assert_eq!(rs, input);

    let selector = transcoder.encode::<_, String>("get_both_foos", [])?;

    let rs = ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        selector,
        value: 0,
    }
    .execute(&api)
    .await
    .and_then(|v| {
        <(struct_foo, struct_foo)>::decode(&mut &v.return_value[..]).map_err(Into::into)
    })?;

    assert_eq!(rs,
     (
        struct_foo { f1: enum_bar::bar2, f2: hex::decode("446f6e277420636f756e7420796f757220636869636b656e73206265666f72652074686579206861746368")?, f3: -102, f4: <_>::from_hex("edaeda")?, f5: "You can't have your cake and eat it too".into(), f6: inner_foo { in1: true, in2: "There are other fish in the sea".into() } },
        input

     )
    );

    let selector = transcoder.encode::<_, _>("delete_foo", ["true"])?;

    WriteContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        selector,
        value: 0,
    }
    .execute(&api)
    .await?;

    let selector = transcoder.encode::<_, _>("get_foo", ["false"])?;

    let rs = ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        selector,
        value: 0,
    }
    .execute(&api)
    .await
    .and_then(|v| <struct_foo>::decode(&mut &v.return_value[..]).map_err(Into::into))?;

    assert_eq!(
        rs,
        struct_foo {
            f1: enum_bar::bar2,
            f2: hex::decode("b52b073595ccb35eaebb87178227b779")?,
            f3: -123112321,
            f4: <_>::from_hex("123456")?,
            f5: "Barking up the wrong tree".into(),
            f6: inner_foo {
                in1: true,
                in2: "nah".into()
            }
        }
    );

    let selector = transcoder.encode::<_, String>("struct_literal", [])?;

    WriteContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        selector,
        value: 0,
    }
    .execute(&api)
    .await?;

    let selector = transcoder.encode::<_, _>("get_foo", ["true"])?;

    let rs = ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: contract.contract_address.clone(),
        selector,
        value: 0,
    }
    .execute(&api)
    .await
    .and_then(|v| <struct_foo>::decode(&mut &v.return_value[..]).map_err(Into::into))?;

    assert_eq!(
        rs,
        struct_foo {
            f1: enum_bar::bar4,
            f2: hex::decode(
                "537570657263616c6966726167696c697374696365787069616c69646f63696f7573"
            )?,
            f3: 64927,
            f4: <_>::from_hex("e282ac")?,
            f5: "Antidisestablishmentarianism".into(),
            f6: inner_foo {
                in1: true,
                in2: "Pseudopseudohypoparathyroidism".into()
            }
        }
    );

    Ok(())
}
