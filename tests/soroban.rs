// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "soroban")]
pub mod soroban_testcases;

use solang::codegen::Options;
use solang::file_resolver::FileResolver;
use solang::{compile, Target};
use soroban_sdk::testutils::Logs;
use soroban_sdk::{vec, Address, Env, Symbol, Val};
use std::ffi::OsStr;

// TODO: register accounts, related balances, events, etc.
pub struct SorobanEnv {
    env: Env,
    contracts: Vec<Address>,
}

/// Compile a Solidity contract and return the compiled WASM blob.
pub fn build_solidity(src: &str) -> Vec<u8> {
    let tmp_file = OsStr::new("test.sol");
    let mut cache = FileResolver::default();
    cache.set_file_contents(tmp_file.to_str().unwrap(), src.to_string());
    let opt = inkwell::OptimizationLevel::Default;
    let target = Target::Soroban;
    let (wasm, ns) = compile(
        tmp_file,
        &mut cache,
        target,
        &Options {
            opt_level: opt.into(),
            log_runtime_errors: true,
            log_prints: true,
            #[cfg(feature = "wasm_opt")]
            wasm_opt: Some(contract_build::OptimizationPasses::Z),
            soroban_version: None,
            ..Default::default()
        },
        std::vec!["unknown".to_string()],
        "0.0.1",
    );
    ns.print_diagnostics_in_plain(&cache, false);
    assert!(!wasm.is_empty());
    let wasm_blob = wasm[0].0.clone();
    wasm_blob
}

impl SorobanEnv {
    /// Create a new Soroban environment.
    pub fn new() -> Self {
        Self {
            env: Env::default(),
            contracts: Vec::new(),
        }
    }

    /// Create a new Soroban environment with a contract.
    pub fn new_with_contract(
        contract_wasm: Vec<u8>,
        constructor_args: soroban_sdk::Vec<Val>,
    ) -> Self {
        let mut env = Self::new();
        env.register_contract(contract_wasm, constructor_args);
        env
    }

    /// Register a contract given its WASM blob and constructor arguments.
    pub fn register_contract(
        &mut self,
        contract_wasm: Vec<u8>,
        constructor_args: soroban_sdk::Vec<Val>,
    ) -> Address {
        let addr = self
            .env
            .register(contract_wasm.as_slice(), constructor_args);
        self.contracts.push(addr.clone());
        addr
    }

    /// Invoke a contract and return the result.
    pub fn invoke_contract(&self, addr: &Address, function_name: &str, args: Vec<Val>) -> Val {
        let func = Symbol::new(&self.env, function_name);
        let mut args_soroban = vec![&self.env];
        for arg in args {
            args_soroban.push_back(arg)
        }
        println!("args_soroban: {:?}", args_soroban);
        self.env.invoke_contract(addr, &func, args_soroban)
    }

    /// Invoke a contract and expect an error. Returns the logs.
    pub fn invoke_contract_expect_error(
        &self,
        addr: &Address,
        function_name: &str,
        args: Vec<Val>,
    ) -> Vec<String> {
        let func = Symbol::new(&self.env, function_name);
        let mut args_soroban = vec![&self.env];
        for arg in args {
            args_soroban.push_back(arg)
        }

        let _ = self
            .env
            .try_invoke_contract::<Val, Val>(addr, &func, args_soroban);

        self.env.logs().all()
    }
}

impl Default for SorobanEnv {
    fn default() -> Self {
        Self::new()
    }
}
