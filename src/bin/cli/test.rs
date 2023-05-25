// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]

mod tests {
    use crate::{Cli, Commands};
    use clap::{CommandFactory, Parser};

    #[test]
    fn test() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parse_compile_options() {
        let mut command_vec: Vec<&str> = "solang compile flipper.sol --target substrate --value-length=31 --address-length=33 --no-dead-storage --no-constant-folding --no-strength-reduce --no-vector-to-slice --no-cse -O aggressive".split(' ').collect();
        let mut sesa = Cli::parse_from(command_vec);

        if let Commands::Compile(compile_args) = sesa.command {
            assert_eq!(compile_args.package.input, vec!["flipper.sol"]);
            assert_eq!(compile_args.target_arg.name, "substrate");
            assert_eq!(compile_args.target_arg.address_length.unwrap(), 33_u64);
            assert_eq!(compile_args.target_arg.value_length.unwrap(), 31_u64);
            assert!(!compile_args.optimizations.common_subexpression_elimination,);
            assert!(!compile_args.optimizations.constant_folding);
            assert!(!compile_args.optimizations.dead_storage);
            assert!(!compile_args.optimizations.vector_to_slice);
            assert!(!compile_args.optimizations.strength_reduce);
            assert_eq!(compile_args.optimizations.opt_level, "aggressive");
        }

        command_vec = "solang compile flipper.sol --target substrate --no-log-runtime-errors --no-prints --no-log-api-return-codes -g --release".split(' ').collect();
        sesa = Cli::parse_from(command_vec);

        if let Commands::Compile(compile_args) = sesa.command {
            assert!(compile_args.debug_features.generate_debug_info);
            assert!(!compile_args.debug_features.log_api_return_codes);
            assert!(!compile_args.debug_features.log_prints);
            assert!(!compile_args.debug_features.log_runtime_errors);
            assert!(compile_args.debug_features.release);
        }
    }
}
