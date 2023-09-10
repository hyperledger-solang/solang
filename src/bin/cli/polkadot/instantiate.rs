// SPDX-License-Identifier: Apache-2.0

use super::{prompt_confirm_transaction, CLIExtrinsicOpts};
use crate::cli::check_target_match;
use anyhow::Result;
use contract_build::{name_value_println, util::decode_hex, Verbosity};
use contract_extrinsics::ErrorVariant;
use contract_extrinsics::{
    BalanceVariant, DisplayEvents, ExtrinsicOptsBuilder, InstantiateCommandBuilder,
};
use sp_core::Bytes;
use std::fmt::Debug;

#[derive(Debug, clap::Args)]
#[clap(name = "instantiate", about = "Instantiate a contract on Polkadot")]
pub struct PolkadotInstantiateCommand {
    #[clap(
        name = "constructor",
        long,
        default_value = "new",
        help = "Specifies the name of the contract constructor to call."
    )]
    constructor: String,
    #[clap(long, num_args = 0.., help = "Specifies the arguments of the contract constructor to call.")]
    args: Vec<String>,
    #[clap(flatten)]
    extrinsic_cli_opts: CLIExtrinsicOpts,
    #[clap(
        name = "value",
        long,
        default_value = "0",
        help = "Specifies the value to be transferred as part of the call."
    )]
    value: BalanceVariant,
    #[clap(
        name = "gas",
        long,
        help = "Specifies the maximum amount of gas to be used for this command."
    )]
    gas_limit: Option<u64>,
    #[clap(
        long,
        help = "Specifies the maximum proof size for this instantiation."
    )]
    proof_size: Option<u64>,
    #[clap(long, value_parser = parse_hex_bytes, help = "Specifies a salt used in the address derivation of the new contract.")]
    salt: Option<Bytes>,
    #[clap(
        short('y'),
        long,
        help = "Specifies whether to skip the confirmation prompt."
    )]
    skip_confirm: bool,
}

/// Parse hex encoded bytes.
fn parse_hex_bytes(input: &str) -> Result<Bytes> {
    let bytes = decode_hex(input)?;
    Ok(bytes.into())
}

impl PolkadotInstantiateCommand {
    /// Returns whether to export the call output in JSON format.
    pub fn output_json(&self) -> bool {
        self.extrinsic_cli_opts.output_json
    }

    pub async fn handle(&self) -> Result<(), ErrorVariant> {
        check_target_match("polkadot").unwrap();
        let cli_options = ExtrinsicOptsBuilder::default()
            .file(Some(self.extrinsic_cli_opts.file.clone()))
            .url(self.extrinsic_cli_opts.url().clone())
            .suri(self.extrinsic_cli_opts.suri.clone())
            .storage_deposit_limit(self.extrinsic_cli_opts.storage_deposit_limit.clone())
            .done();
        let exec = InstantiateCommandBuilder::default()
            .constructor(self.constructor.clone())
            .args(self.args.clone())
            .extrinsic_opts(cli_options)
            .value(self.value.clone())
            .gas_limit(self.gas_limit)
            .proof_size(self.proof_size)
            .salt(self.salt.clone())
            .done()
            .await?;

        if !self.extrinsic_cli_opts.execute {
            let result = exec.instantiate_dry_run().await?;
            match exec.decode_instantiate_dry_run(&result).await {
                Ok(dry_run_result) => {
                    if self.output_json() {
                        println!("{}", dry_run_result.to_json()?);
                    } else {
                        name_value_println!("Result", format!("{}", &dry_run_result.result));
                        name_value_println!("Reverted", format!("{:?}", &dry_run_result.reverted));
                        name_value_println!("Contract", &dry_run_result.contract);
                        name_value_println!(
                            "Gas consumed",
                            &dry_run_result.gas_consumed.to_string()
                        );
                        println!("Execution of your instantiate call has NOT been completed.\n
                        To submit the transaction and execute the call on chain, please include -x/--execute flag.");
                    }
                    Ok(())
                }
                Err(object) => Err(object),
            }
        } else {
            let gas_limit = exec.estimate_gas().await?;
            if !self.skip_confirm {
                prompt_confirm_transaction(|| {
                    println!("Instantiation Summary:");
                    name_value_println!("Constructor", exec.args().constructor());
                    name_value_println!("Args", exec.args().raw_args().join(" "));
                    name_value_println!("Gas limit", gas_limit.to_string());
                })?;
            }
            let instantiate_result = exec.instantiate(Some(gas_limit)).await?;
            let events = DisplayEvents::from_events(
                &instantiate_result.result,
                Some(exec.transcoder()),
                &exec.client().metadata(),
            )?;
            let contract_address = instantiate_result.contract_address.to_string();
            if self.output_json() {
                let display_instantiate_result = InstantiateResult {
                    code_hash: instantiate_result.code_hash.map(|ch| format!("{ch:?}")),
                    contract: contract_address,
                    events,
                };
                println!("{}", display_instantiate_result.to_json()?)
            } else {
                println!(
                    "{}",
                    events
                        .display_events(Verbosity::Default, &instantiate_result.token_metadata)?
                );
                if let Some(code_hash) = instantiate_result.code_hash {
                    name_value_println!("Code hash", format!("{code_hash:?}"));
                }
                name_value_println!("Contract", contract_address);
            };
            Ok(())
        }
    }
}

#[derive(serde::Serialize)]
pub struct InstantiateResult {
    pub contract: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_hash: Option<String>,
    pub events: DisplayEvents,
}

impl InstantiateResult {
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}
