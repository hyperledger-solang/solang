// SPDX-License-Identifier: Apache-2.0

use contract_extrinsics::ErrorVariant;
use serde_json::json;
use std::fmt::Debug;

use super::CLIExtrinsicOpts;
use crate::cli::check_target_match;
use anyhow::Result;
use contract_build::{name_value_println, Verbosity};
use contract_extrinsics::{ExtrinsicOptsBuilder, TokenMetadata, UploadCommandBuilder};

#[derive(Debug, clap::Args)]
#[clap(name = "upload", about = "Upload a contract on Polkadot")]
pub struct PolkadotUploadCommand {
    #[clap(flatten)]
    extrinsic_cli_opts: CLIExtrinsicOpts,
}

impl PolkadotUploadCommand {
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
        let exec = UploadCommandBuilder::default()
            .extrinsic_opts(cli_options)
            .done()
            .await?;

        let code_hash = exec.code().code_hash();

        if !self.extrinsic_cli_opts.execute {
            match exec.upload_code_rpc().await? {
                Ok(result) => {
                    if self.output_json() {
                        let json_object = json!({
                            "result": "Success",
                            "code_hash": result.code_hash,
                            "deposit": result.deposit
                        })
                        .to_string();
                        println!("{}", json_object);
                    } else {
                        name_value_println!("Result", "Success");
                        name_value_println!("Code hash", format!("{:?}", result.code_hash));
                        name_value_println!("Deposit", format!("{:?}", result.deposit));
                        println!("Execution of your upload call has NOT been completed.\n
                        To submit the transaction and execute the call on chain, please include -x/--execute flag.");
                    }
                }
                Err(err) => {
                    let metadata = exec.client().metadata();
                    let err = ErrorVariant::from_dispatch_error(&err, &metadata)?;
                    return Err(err);
                }
            }
        } else {
            let result = exec.upload_code().await?;
            let events = result.display_events;
            let output = if self.output_json() {
                events.to_json()?
            } else {
                let token_metadata = TokenMetadata::query(exec.client()).await?;
                events.display_events(Verbosity::Default, &token_metadata)?
            };
            println!("{output}");
            if let Some(code_stored) = result.code_stored {
                if self.output_json() {
                    let json_object = json!({
                        "code_hash": code_stored.code_hash,
                    })
                    .to_string();
                    println!("{}", json_object);
                } else {
                    name_value_println!("Code hash", format!("{:?}", code_stored.code_hash));
                }
            } else {
                let code_hash = hex::encode(code_hash);
                return Err(anyhow::anyhow!(
                    "This contract has already been uploaded. Code hash: 0x{code_hash}"
                )
                .into());
            }
        }
        Ok(())
    }
}
