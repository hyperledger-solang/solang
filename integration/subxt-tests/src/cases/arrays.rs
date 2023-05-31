use crate::{Contract, DeployContract, Execution, ReadContract, WriteContract, API};
use contract_transcode::{ContractMessageTranscoder, Value};
use hex::FromHex;

use parity_scale_codec::{Decode, Encode};
use rand::{seq::SliceRandom, thread_rng, Rng};
use sp_core::{crypto::AccountId32, hexdisplay::AsBytesRef};

#[ignore]
#[tokio::test]
async fn case() -> anyhow::Result<()> {
    let api = API::new().await?;

    let mut contract = Contract::new("./contracts/arrays.contract")?;

    contract
        .deploy(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| t.encode::<_, String>("new", []).unwrap(),
        )
        .await?;

    let mut users = Vec::new();

    for i in 0..3 {
        let rnd_addr = rand::random::<[u8; 32]>();

        let name = format!("name{i}");

        let id = u32::from_be_bytes(rand::random::<[u8; 4]>());
        let mut perms = Vec::<String>::new();

        let mut j: f64 = 0.0;
        while j < rand::thread_rng().gen_range(0.0..=3.0) {
            j += 1.0;

            let p = rand::thread_rng().gen_range(0..8);
            perms.push(format!("Perm{}", p + 1));
        }

        contract
            .call(
                &api,
                sp_keyring::AccountKeyring::Alice,
                0,
                &|t: &ContractMessageTranscoder| {
                    t.encode(
                        "addUser",
                        [
                            id.to_string(),
                            format!("0x{}", hex::encode(rnd_addr)),
                            format!("\"{}\"", name.clone()),
                            format!("[{}]", perms.join(",")),
                        ],
                    )
                    .unwrap()
                },
            )
            .await?;

        users.push((name, rnd_addr, id, perms));
    }

    let (name, addr, id, perms) = users.choose(&mut thread_rng()).unwrap();

    let output = contract
        .try_call(
            &api,
            sp_keyring::AccountKeyring::Alice,
            0,
            &|t: &ContractMessageTranscoder| {
                t.encode("getUserById", [format!("\"{id}\"")]).unwrap()
            },
        )
        .await?;

    let (name, addr, id, perms) =
        <(String, AccountId32, u64, Vec<u8>)>::decode(&mut output.as_bytes_ref())?;

    if !perms.is_empty() {
        let p = perms.choose(&mut thread_rng()).unwrap();

        let output = contract
            .try_call(
                &api,
                sp_keyring::AccountKeyring::Alice,
                0,
                &|t: &ContractMessageTranscoder| {
                    t.encode(
                        "hasPermission",
                        [format!("\"{id}\""), format!("Perm{}", p + 1)],
                    )
                    .unwrap()
                },
            )
            .await?;

        let has_permission = <bool>::decode(&mut output.as_bytes_ref())?;
        assert!(has_permission);
    }

    Ok(())
}
