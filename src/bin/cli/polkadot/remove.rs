// SPDX-License-Identifier: Apache-2.0

use contract_extrinsics::ErrorVariant;
use std::fmt::Debug;

use super::CLIExtrinsicOpts;
use crate::cli::check_target_match;
use anyhow::Result;
use contract_build::{name_value_println, Verbosity};
use contract_extrinsics::{
    parse_code_hash, DefaultConfig, ExtrinsicOptsBuilder, RemoveCommandBuilder, TokenMetadata,
};
use subxt::Config;

#[derive(Debug, clap::Args)]
#[clap(name = "remove", about = "Remove a contract on Polkadot")]
pub struct PolkadotRemoveCommand {
    #[clap(long, value_parser = parse_code_hash, help = "Specifies the code hash to remove.")]
    code_hash: Option<<DefaultConfig as Config>::Hash>,
    #[clap(flatten)]
    extrinsic_cli_opts: CLIExtrinsicOpts,
}

impl PolkadotRemoveCommand {
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
        let remove_exec = RemoveCommandBuilder::default()
            .code_hash(self.code_hash)
            .extrinsic_opts(cli_options)
            .done()
            .await?;
        let remove_result = remove_exec.remove_code().await?;
        let display_events = remove_result.display_events;
        let output = if self.output_json() {
            display_events.to_json()?
        } else {
            let token_metadata = TokenMetadata::query(remove_exec.client()).await?;
            display_events.display_events(Verbosity::Default, &token_metadata)?
        };
        println!("{output}");
        if let Some(code_removed) = remove_result.code_removed {
            let remove_result = code_removed.code_hash;

            if self.output_json() {
                println!("{}", &remove_result);
            } else {
                name_value_println!("Code hash", format!("{remove_result:?}"));
            }
            Ok(())
        } else {
            let error_code_hash = hex::encode(remove_exec.final_code_hash());
            Err(anyhow::anyhow!("Error removing the code: {}", error_code_hash).into())
        }
    }
}
