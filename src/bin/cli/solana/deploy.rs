// SPDX-License-Identifier: Apache-2.0

use crate::cli::check_target_match;
use std::{process::exit, str::FromStr, time::Duration};

use solana_cli::{
    cli::{
        process_command, CliCommand, CliCommandInfo, CliConfig, DEFAULT_CONFIRM_TX_TIMEOUT_SECONDS,
        DEFAULT_RPC_TIMEOUT_SECONDS,
    },
    program::ProgramCliCommand,
};
use solana_cli_config::{Config, CONFIG_FILE};
use solana_cli_output::OutputFormat;
use solana_rpc_client_api::config::RpcSendTransactionConfig;
use solana_sdk::{commitment_config::CommitmentConfig, signer::keypair::read_keypair_file};

#[derive(Clone, Debug, clap::Args)]
#[clap(name = "deploy", about = "Deploy a program to Solana")]
pub struct SolanaDeploy {
    #[clap(help = "Specifies the path to the program file to deploy (.so)")]
    program_location: String,
    #[clap(long, help = "Specifies whether to export the output in JSON format")]
    output_json: bool,
    #[clap(
        short('v'),
        long,
        conflicts_with = "output_json",
        help = "Specifies whether to display verbose program deployment information"
    )]
    verbose: bool,
}

impl SolanaDeploy {
    /// Handle the deployment of a Solana program
    ///
    /// This function is responsible for managing the deployment process,
    /// including checking the current directory, parsing command-line arguments,
    /// configuring settings, and executing the deployment command. It also handles
    /// loading the necessary configuration and signers, defining output formats,
    /// and processing the deployment command using the provided configuration.
    pub fn handle(&self) {
        // Make sure the command is run in the correct directory
        // Fails if the command is run in a Solang Polkadot project directory
        if !check_target_match("solana", None).unwrap() {
            exit(1);
        }

        // Parse the command line arguments
        let verbose = self.verbose;
        let output_json = self.output_json;
        let file_name = self.program_location.as_str();

        // Get the path to the configuration file (default location)
        let config_file = CONFIG_FILE.as_ref().unwrap();

        // Load configuration settings from a file or use defaults if the file is not found
        let config = Config::load(config_file).unwrap_or_default();

        // Create a CLI command for program deployment and define signers
        let CliCommandInfo { command, signers } = CliCommandInfo {
            command: CliCommand::Program(ProgramCliCommand::Deploy {
                program_location: Some(file_name.to_string()),
                program_signer_index: None,
                program_pubkey: None,
                buffer_signer_index: None,
                buffer_pubkey: None,
                upgrade_authority_signer_index: 0,
                is_final: false,
                max_len: None,
                allow_excessive_balance: false,
                skip_fee_check: false,
            }),
            // Load signer keypair from the file specified in the configuration
            signers: vec![read_keypair_file(&config.keypair_path).unwrap().into()],
        };

        // Parse the commitment level from the configuration file
        let commitment = CommitmentConfig::from_str(&config.commitment).ok().unwrap();

        // Determine the output format (JSON or Display)
        let output_format = match output_json {
            true => OutputFormat::Json,
            false => {
                if verbose {
                    OutputFormat::DisplayVerbose
                } else {
                    OutputFormat::Display
                }
            }
        };

        // Create a new configuration with modified settings
        let config = CliConfig {
            command,
            json_rpc_url: config.json_rpc_url,
            websocket_url: config.websocket_url,
            signers: signers.iter().map(|s| s.as_ref()).collect(),
            keypair_path: config.keypair_path,
            rpc_client: None,
            rpc_timeout: Duration::from_secs(DEFAULT_RPC_TIMEOUT_SECONDS.parse::<u64>().unwrap()),
            verbose,
            output_format,
            commitment,
            send_transaction_config: RpcSendTransactionConfig {
                preflight_commitment: Some(commitment.commitment),
                ..RpcSendTransactionConfig::default()
            },
            confirm_transaction_initial_timeout: Duration::from_secs(
                DEFAULT_CONFIRM_TX_TIMEOUT_SECONDS.parse::<u64>().unwrap(),
            ),
            address_labels: config.address_labels,
            use_quic: true,
        };

        // Process the deployment command with the updated configuration
        let result = process_command(&config).unwrap();
        println!("{result}");
    }
}
