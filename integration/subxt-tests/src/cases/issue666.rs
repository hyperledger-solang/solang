// SPDX-License-Identifier: Apache-2.0

use crate::{Contract, API};

#[tokio::test]
async fn case() -> anyhow::Result<()> {
    let api = API::new().await?;
    let mut c_flipper = Contract::new("contracts/Flip.contract")?;
    c_flipper
        .deploy(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("new", [])
                .expect("unable to find selector")
        })
        .await?;

    let mut c_inc = Contract::new("./contracts/Inc.contract")?;

    // flip on Flip
    c_flipper
        .call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("flip", [])
                .expect("unable to find selector")
        })
        .await?;

    c_inc
        .deploy(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>(
                "new",
                [format!(
                    "0x{}",
                    hex::encode(c_flipper.address.clone().unwrap())
                )],
            )
            .expect("unable to find selector")
        })
        .await?;
    // superFlip on Inc
    c_inc
        .call(&api, sp_keyring::AccountKeyring::Alice, 0, |t| {
            t.encode::<_, String>("superFlip", [])
                .expect("unable to find selector")
        })
        .await?;
    Ok(())
}
