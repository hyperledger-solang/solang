// SPDX-License-Identifier: Apache-2.0

use crate::cli::check_target_match;
use contract_extrinsics::ErrorVariant;

use serde_json::json;
use std::fmt::Debug;

use super::{prompt_confirm_transaction, CLIExtrinsicOpts};
use anyhow::{Context, Result};
use contract_build::name_value_println;
use contract_build::Verbosity;
use contract_extrinsics::{
    BalanceVariant, CallCommandBuilder, DefaultConfig, ExtrinsicOptsBuilder, StorageDeposit,
    TokenMetadata,
};
use subxt::Config;

#[derive(Debug, clap::Args)]
#[clap(name = "call", about = "Call a contract on Polkadot")]
pub struct PolkadotCallCommand {
    #[clap(
        name = "contract",
        long,
        help = "Specifies the address of the contract to call."
    )]
    contract: <DefaultConfig as Config>::AccountId,
    #[clap(
        long,
        short,
        help = "Specifies the name of the contract message to call."
    )]
    message: String,
    #[clap(long, num_args = 0.., help = "Specifies the arguments of the contract message to call.")]
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
    #[clap(long, help = "Specifies the maximum proof size for this call.")]
    proof_size: Option<u64>,
    #[clap(
        short('y'),
        long,
        help = "Specifies whether to skip the confirmation prompt."
    )]
    skip_confirm: bool,
}

impl PolkadotCallCommand {
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
        let exec = CallCommandBuilder::default()
            .contract(self.contract.clone())
            .message(self.message.clone())
            .args(self.args.clone())
            .extrinsic_opts(cli_options)
            .gas_limit(self.gas_limit)
            .proof_size(self.proof_size)
            .value(self.value.clone())
            .done()
            .await?;

        if !self.extrinsic_cli_opts.execute {
            let result = exec.call_dry_run().await?;
            match result.result {
                Ok(ref ret_val) => {
                    let value = exec
                        .transcoder()
                        .decode_message_return(exec.message(), &mut &ret_val.data[..])
                        .context(format!("Failed to decode return value {:?}", &ret_val))?;
                    if self.output_json() {
                        let json_object = json!({
                            "reverted": ret_val.did_revert(),
                            "data": value,
                            "gas_consumed": result.gas_consumed,
                            "gas_required": result.gas_required,
                            "storage_deposit": StorageDeposit::from(&result.storage_deposit),
                        })
                        .to_string();
                        println!("{}", json_object);
                    } else {
                        name_value_println!("Result", format!("{}", value));
                        name_value_println!("Reverted", format!("{:?}", ret_val.did_revert()));
                        println!("Execution of your call has NOT been completed.\n
                        To submit the transaction and execute the call on chain, please include -x/--execute flag.");
                    };
                }
                Err(ref err) => {
                    let metadata = exec.client().metadata();
                    let object = ErrorVariant::from_dispatch_error(err, &metadata)?;
                    return Err(object);
                }
            }
        } else {
            let gas_limit = exec.estimate_gas().await?;
            if !self.skip_confirm {
                prompt_confirm_transaction(|| {
                    println!("Call Summary:");
                    name_value_println!("Message", exec.message());
                    name_value_println!("Args", exec.args().join(" "));
                    name_value_println!("Gas limit", gas_limit.to_string());
                })?;
            }
            let token_metadata = TokenMetadata::query(exec.client()).await?;
            let display_events = exec.call(Some(gas_limit)).await?;
            let output = if self.output_json() {
                display_events.to_json()?
            } else {
                display_events.display_events(Verbosity::Default, &token_metadata)?
            };
            println!("{output}");
        }
        Ok(())
    }
}
