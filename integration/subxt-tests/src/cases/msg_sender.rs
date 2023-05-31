use contract_transcode::ContractMessageTranscoder;
use parity_scale_codec::{Decode, Encode, Input};
use sp_core::{crypto::AccountId32, hexdisplay::AsBytesRef};

use crate::{Contract, DeployContract, Execution, ReadContract, WriteContract, API};

#[tokio::test]
async fn case() -> anyhow::Result<()> {
    let api = API::new().await?;

    // mytoken
    let mytoken_code = std::fs::read("./contracts/mytoken.wasm")?;
    let mytoken_event_code = std::fs::read("./contracts/mytokenEvent.wasm")?;

    let c_mytoken = Contract::new("./contracts/mytoken.contract")?;
    let t_mytoken = &c_mytoken.transcoder;

    let c_mytoken_evt = Contract::new("./contracts/mytokenEvent.contract")?;
    let t_mytoken_evt = &c_mytoken_evt.transcoder;

    let selector = t_mytoken.encode::<_, String>("new", [])?;

    // let selector = build_selector("861731d5", None)?;

    let mytoken = DeployContract {
        caller: sp_keyring::AccountKeyring::Alice,
        selector,
        value: 0,
        code: mytoken_code,
    }
    .execute(&api)
    .await?;

    // read test
    let selector = t_mytoken.encode::<_, String>(
        "test",
        [
            format!(
                "0x{}",
                hex::encode(sp_keyring::AccountKeyring::Alice.to_account_id())
            ),
            "true".into(),
        ],
    )?;

    let addr = ReadContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: mytoken.contract_address.clone(),
        value: 0,
        selector,
    }
    .execute(&api)
    .await
    .and_then(|v| <AccountId32>::decode(&mut v.return_value.as_bytes_ref()).map_err(Into::into))?;

    assert_eq!(addr, sp_keyring::AccountKeyring::Alice.to_account_id());

    // mytokenEvent
    let selector = t_mytoken_evt.encode::<_, String>("new", [])?;

    let mytoken_event = DeployContract {
        caller: sp_keyring::AccountKeyring::Alice,
        selector,
        value: 0,
        code: mytoken_event_code,
    }
    .execute(&api)
    .await?;

    // call test
    let selector = t_mytoken_evt.encode::<_, String>("test", [])?;

    let output = WriteContract {
        caller: sp_keyring::AccountKeyring::Alice,
        contract_address: mytoken_event.contract_address.clone(),
        selector,
        value: 0,
    }
    .execute(&api)
    .await?;

    assert_eq!(output.events.len(), 1);

    let evt = &output.events[0];

    let mut evt_buffer = evt.data.as_slice();

    let topic_id = evt_buffer.read_byte()?;

    assert_eq!(topic_id, 0);

    let addr = <AccountId32>::decode(&mut evt_buffer)?;

    assert_eq!(addr, sp_keyring::AccountKeyring::Alice.to_account_id());

    Ok(())
}
