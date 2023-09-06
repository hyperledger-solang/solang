// SPDX-License-Identifier: Apache-2.0

use contract_extrinsics::ErrorVariant;
use std::fmt::Debug;

use super::{display_dry_run_result_warning, CLIExtrinsicOpts};
use crate::cli::check_target_match;
use anyhow::Result;
use contract_build::name_value_println;
use contract_extrinsics::{Balance, ExtrinsicOptsBuilder, TokenMetadata, UploadCommandBuilder};

#[derive(Debug, clap::Args)]
#[clap(name = "upload", about = "Upload a contract's code on Polkadot")]
pub struct PolkadotUploadCommand {
    #[clap(flatten)]
    extrinsic_cli_opts: CLIExtrinsicOpts,
    /// Export the call output in JSON format.
    #[clap(long, conflicts_with = "verbose")]
    output_json: bool,
}

impl PolkadotUploadCommand {
    /// Returns whether to export the call output in JSON format.
    pub fn output_json(&self) -> bool {
        self.output_json
    }

    pub async fn handle(&self) -> Result<(), ErrorVariant> {
        check_target_match("polkadot");
        let extrinsic_opts = ExtrinsicOptsBuilder::default()
            .file(Some(self.extrinsic_cli_opts.file.clone()))
            .url(self.extrinsic_cli_opts.url.clone())
            .suri(self.extrinsic_cli_opts.suri.clone())
            .storage_deposit_limit(self.extrinsic_cli_opts.storage_deposit_limit.clone())
            .done();
        let upload_exec = UploadCommandBuilder::default()
            .extrinsic_opts(extrinsic_opts)
            .done()
            .await?;

        let code_hash = upload_exec.code().code_hash();

        if !self.extrinsic_cli_opts.execute {
            match upload_exec.upload_code_rpc().await? {
                Ok(result) => {
                    let upload_result = UploadDryRunResult {
                        result: String::from("Success!"),
                        code_hash: format!("{:?}", result.code_hash),
                        deposit: result.deposit,
                    };
                    if self.output_json() {
                        println!("{}", upload_result.to_json()?);
                    } else {
                        upload_result.print();
                        display_dry_run_result_warning("upload");
                    }
                }
                Err(err) => {
                    let metadata = upload_exec.client().metadata();
                    let err = ErrorVariant::from_dispatch_error(&err, &metadata)?;
                    if self.output_json() {
                        return Err(err);
                    } else {
                        name_value_println!("Result", err);
                    }
                }
            }
        } else {
            let upload_result = upload_exec.upload_code().await?;
            let display_events = upload_result.display_events;
            let output = if self.output_json() {
                display_events.to_json()?
            } else {
                let token_metadata = TokenMetadata::query(upload_exec.client()).await?;
                display_events
                    .display_events(self.extrinsic_cli_opts.verbosity()?, &token_metadata)?
            };
            println!("{output}");
            if let Some(code_stored) = upload_result.code_stored {
                let upload_result = CodeHashResult {
                    code_hash: format!("{:?}", code_stored.code_hash),
                };
                if self.output_json() {
                    println!("{}", upload_result.to_json()?);
                } else {
                    upload_result.print();
                }
            } else {
                let code_hash = hex::encode(code_hash);
                return Err(anyhow::anyhow!(
                    "This contract has already been uploaded with code hash: 0x{code_hash}"
                )
                .into());
            }
        }
        Ok(())
    }
}

#[derive(serde::Serialize)]
pub struct CodeHashResult {
    pub code_hash: String,
}

impl CodeHashResult {
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn print(&self) {
        name_value_println!("Code hash", format!("{:?}", self.code_hash));
    }
}

#[derive(serde::Serialize)]
pub struct UploadDryRunResult {
    pub result: String,
    pub code_hash: String,
    pub deposit: Balance,
}

impl UploadDryRunResult {
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn print(&self) {
        name_value_println!("Result", self.result);
        name_value_println!("Code hash", format!("{:?}", self.code_hash));
        name_value_println!("Deposit", format!("{:?}", self.deposit));
    }
}
