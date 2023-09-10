// SPDX-License-Identifier: Apache-2.0

pub mod call;
pub mod instantiate;
pub mod remove;
pub mod upload;

pub(crate) use self::{
    call::PolkadotCallCommand, instantiate::PolkadotInstantiateCommand,
    remove::PolkadotRemoveCommand, upload::PolkadotUploadCommand,
};
use crate::PathBuf;
use anyhow::anyhow;
use anyhow::Result;
use contract_extrinsics::BalanceVariant;
use std::io::{self, Write};
pub use subxt::PolkadotConfig as DefaultConfig;

/// Common CLI options for any extrinsic execution.
#[derive(Clone, Debug, clap::Args)]
pub struct CLIExtrinsicOpts {
    #[clap(
        value_parser,
        help = "Specifies the path to a contract wasm file, .contract bundle, or .json metadata file."
    )]
    file: PathBuf,
    #[clap(
        name = "url",
        long,
        value_parser,
        default_value = "ws://localhost:9944",
        help = "Specifies the websockets URL for the substrate node directly."
    )]
    url: url::Url,
    #[clap(
        value_enum,
        name = "network",
        long,
        conflicts_with = "url",
        help = "Specifies the network name."
    )]
    network: Option<Network>,
    #[clap(
        name = "suri",
        long,
        short,
        help = "Specifies the secret key URI used for deploying the contract. For example:\n
    For a development account: //Alice\n
    With a password: //Alice///SECRET_PASSWORD"
    )]
    suri: String,
    #[clap(
        short('x'),
        long,
        help = "Specifies whether to submit the extrinsic for execution."
    )]
    execute: bool,
    #[clap(
        long,
        help = "Specifies the maximum amount of balance that can be charged from the caller to pay for the storage consumed."
    )]
    storage_deposit_limit: Option<BalanceVariant>,
    #[clap(long, help = "Specifies whether to export the call output in JSON.")]
    output_json: bool,
}

/// Available networks.
#[derive(clap::ValueEnum, Clone, Debug)]
enum Network {
    Rococo,
    PhalaPoC5,
    AstarShiden,
    AstarShibuya,
    Astar,
    AlephZeroTestnet,
    AlephZero,
    T3RNT0RN,
    PendulumTestnet,
}

impl CLIExtrinsicOpts {
    /// Returns the URL for the substrate node.
    /// If the network is specified, it returns the URL for the network.
    /// Otherwise, it returns the URL specified by the user.
    pub fn url(&self) -> url::Url {
        if let Some(net) = &self.network {
            match net {
                Network::Rococo => {
                    url::Url::parse("wss://rococo-contracts-rpc.polkadot.io").unwrap()
                }
                Network::PhalaPoC5 => url::Url::parse("wss://poc5.phala.network/ws").unwrap(),
                Network::AstarShiden => url::Url::parse("wss://rpc.shiden.astar.network").unwrap(),
                Network::AstarShibuya => {
                    url::Url::parse("wss://rpc.shibuya.astar.network").unwrap()
                }
                Network::Astar => url::Url::parse("wss://rpc.astar.network").unwrap(),
                Network::AlephZeroTestnet => url::Url::parse("wss://ws.test.azero.dev").unwrap(),
                Network::AlephZero => url::Url::parse("wss://ws.azero.dev").unwrap(),
                Network::T3RNT0RN => url::Url::parse("wss://ws.t0rn.io").unwrap(),
                Network::PendulumTestnet => {
                    url::Url::parse("wss://rpc-foucoco.pendulumchain.tech").unwrap()
                }
            }
        } else {
            self.url.clone()
        }
    }
}

/// Prompt the user to confirm transaction.
pub fn prompt_confirm_transaction<F: FnOnce()>(summary: F) -> Result<()> {
    summary();
    println!("Are you sure you want to submit this transaction? (Y/n): ");

    let mut choice = String::new();
    io::stdout().flush()?;
    io::stdin().read_line(&mut choice)?;
    match choice.trim().to_lowercase().as_str() {
        "y" | "" => Ok(()),
        "n" => Err(anyhow!("Transaction not submitted")),
        _ => Err(anyhow!("Invalid choice")),
    }
}
