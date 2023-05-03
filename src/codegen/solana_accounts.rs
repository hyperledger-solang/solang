// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ASTFunction, ControlFlowGraph, Instr, InternalCallTy};
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{Contract, Function, Mutability, Namespace, SolanaAccount};
use crate::sema::Recurse;
use base58::FromBase58;
use indexmap::IndexSet;
use num_bigint::{BigInt, Sign};
use num_traits::Zero;
use once_cell::sync::Lazy;
use solang_parser::pt::FunctionTy;
use std::collections::{HashMap, HashSet, VecDeque};

/// These are the accounts that we can collect from a contract and that Anchor will populate
/// automatically if their names match the source code description:
/// https://github.com/coral-xyz/anchor/blob/06c42327d4241e5f79c35bc5588ec0a6ad2fedeb/ts/packages/anchor/src/program/accounts-resolver.ts#L54-L60
static CLOCK_ACCOUNT: &str = "clock";
static SYSTEM_ACCOUNT: &str = "systemProgram";
static ASSOCIATED_TOKEN_PROGRAM: &str = "associatedTokenProgram";
static RENT_ACCOUNT: &str = "rent";
static TOKEN_PROGRAM_ID: &str = "tokenProgram";

/// We automatically include the following accounts in the IDL, but these are not
/// automatically populated
static DATA_ACCOUNT: &str = "dataAccount";
static WALLET_ACCOUNT: &str = "wallet";
static INSTRUCTION_ACCOUNT: &str = "SysvarInstruction";

/// If the public keys available in AVAILABLE_ACCOUNTS are hardcoded in a Solidity contract
/// for external calls, we can detect them and leverage Anchor's public key auto populate feature.
static AVAILABLE_ACCOUNTS: Lazy<HashMap<BigInt, &'static str>> = Lazy::new(|| {
    HashMap::from([
        (BigInt::zero(), SYSTEM_ACCOUNT),
        (
            BigInt::from_bytes_be(
                Sign::Plus,
                &"ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
                    .from_base58()
                    .unwrap(),
            ),
            ASSOCIATED_TOKEN_PROGRAM,
        ),
        (
            BigInt::from_bytes_be(
                Sign::Plus,
                &"SysvarRent111111111111111111111111111111111"
                    .from_base58()
                    .unwrap(),
            ),
            RENT_ACCOUNT,
        ),
        (
            BigInt::from_bytes_be(
                Sign::Plus,
                &"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
                    .from_base58()
                    .unwrap(),
            ),
            TOKEN_PROGRAM_ID,
        ),
        (
            BigInt::from_bytes_be(
                Sign::Plus,
                &"SysvarC1ock11111111111111111111111111111111"
                    .from_base58()
                    .unwrap(),
            ),
            CLOCK_ACCOUNT,
        ),
    ])
});

/// Retrieve a name from an account, according to Anchor's constant accounts map
/// https://github.com/coral-xyz/anchor/blob/06c42327d4241e5f79c35bc5588ec0a6ad2fedeb/ts/packages/anchor/src/program/accounts-resolver.ts#L54-L60
fn account_from_number(num: &BigInt) -> Option<&'static str> {
    AVAILABLE_ACCOUNTS.get(num).cloned()
}

/// Struct to save the recursion data when traversing all the CFG instructions
struct RecurseData<'a> {
    /// next_queue saves the set of functions we must check in the next iteration
    next_queue: IndexSet<(usize, usize)>,
    /// The number of the function we are currently traversing
    cfg_func_no: usize,
    /// The contract the function belongs to
    contract_no: usize,
    /// The quantity of accounts we have added the the hashmap 'accounts'
    accounts_added: usize,
    /// The number of the AST function we are currently traversing
    ast_no: usize,
    /// The namespace contracts
    contracts: &'a [Contract],
    /// The vector of functions from the contract
    functions: &'a [Function],
}

impl RecurseData<'_> {
    /// Add an account to the function's indexmap
    fn add_account(&mut self, account_name: String, account: SolanaAccount) {
        if self.functions[self.ast_no]
            .solana_accounts
            .borrow_mut()
            .insert(account_name, account)
            .is_none()
        {
            self.accounts_added += 1;
        }
    }

    /// Add the system account to the function's indexmap
    fn add_system_account(&mut self) {
        self.add_account(
            SYSTEM_ACCOUNT.to_string(),
            SolanaAccount {
                is_writer: false,
                is_signer: false,
            },
        );
    }
}

/// Collect the accounts this contract needs
pub(super) fn collect_accounts_from_contract(contract_no: usize, ns: &Namespace) {
    let mut visiting_queue: IndexSet<(usize, usize)> = IndexSet::new();

    for func_no in ns.contracts[contract_no].all_functions.keys() {
        if ns.functions[*func_no].is_public()
            && !matches!(
                ns.functions[*func_no].ty,
                FunctionTy::Fallback | FunctionTy::Receive | FunctionTy::Modifier
            )
        {
            let func = &ns.functions[*func_no];
            match &func.mutability {
                Mutability::Pure(_) => (),
                Mutability::View(_) => {
                    func.solana_accounts.borrow_mut().insert(
                        DATA_ACCOUNT.to_string(),
                        SolanaAccount {
                            is_writer: false,
                            is_signer: false,
                        },
                    );
                }
                _ => {
                    func.solana_accounts.borrow_mut().insert(
                        DATA_ACCOUNT.to_string(),
                        SolanaAccount {
                            is_writer: true,
                            /// With a @payer annotation, the account is created on-chain and needs a signer. The client
                            /// provides an address that does not exist yet, so SystemProgram.CreateAccount is called
                            /// on-chain.
                            ///
                            /// However, if a @seed is also provided, the program can sign for the account
                            /// with the seed using program derived address (pda) when SystemProgram.CreateAccount is called,
                            /// so no signer is required from the client.
                            is_signer: func.has_payer_annotation() && !func.has_seed_annotation(),
                        },
                    );
                }
            }
            if func.is_constructor() && func.has_payer_annotation() {
                func.solana_accounts.borrow_mut().insert(
                    WALLET_ACCOUNT.to_string(),
                    SolanaAccount {
                        is_signer: true,
                        is_writer: false,
                    },
                );
                func.solana_accounts.borrow_mut().insert(
                    SYSTEM_ACCOUNT.to_string(),
                    SolanaAccount {
                        is_signer: false,
                        is_writer: false,
                    },
                );
            }
        }
        visiting_queue.insert((
            contract_no,
            ns.contracts[contract_no].all_functions[func_no],
        ));
    }

    let mut recurse_data = RecurseData {
        next_queue: IndexSet::new(),
        cfg_func_no: 0,
        ast_no: 0,
        accounts_added: 0,
        contract_no,
        functions: &ns.functions,
        contracts: &ns.contracts,
    };

    let mut old_size: usize = 0;
    loop {
        for (contract_no, func_no) in &visiting_queue {
            if *func_no == usize::MAX {
                continue;
            }

            recurse_data.contract_no = *contract_no;
            recurse_data.cfg_func_no = *func_no;
            match &ns.contracts[*contract_no].cfg[*func_no].function_no {
                ASTFunction::SolidityFunction(ast_no) | ASTFunction::YulFunction(ast_no) => {
                    recurse_data.ast_no = *ast_no;
                }
                _ => (),
            }
            check_function(&ns.contracts[*contract_no].cfg[*func_no], &mut recurse_data);
        }

        // This is the convergence condition for this loop.
        // If we have not added new accounts to the map in this iteration and the queue for the
        // next iteration does not have any new element, we can stop.
        if old_size == recurse_data.accounts_added
            && visiting_queue.len() == recurse_data.next_queue.len()
        {
            break;
        }
        old_size = recurse_data.accounts_added;
        std::mem::swap(&mut visiting_queue, &mut recurse_data.next_queue);
        recurse_data.next_queue.clear();
    }
}

/// Collect the accounts in a function
fn check_function(cfg: &ControlFlowGraph, data: &mut RecurseData) {
    if cfg.blocks.is_empty() {
        return;
    }
    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut visited: HashSet<usize> = HashSet::new();
    queue.push_back(0);
    visited.insert(0);

    while let Some(cur_block) = queue.pop_front() {
        for instr in &cfg.blocks[cur_block].instr {
            check_instruction(instr, data);
        }
        // TODO: Block edges is an expensive function, we use it six times throughout the code,
        // perhaps we can just use the dag I calculate during cse.
        // Changes in constant folding would be necessary
        for edge in cfg.blocks[cur_block].edges() {
            if !visited.contains(&edge) {
                queue.push_back(edge);
                visited.insert(edge);
            }
        }
    }
}

/// Collect the accounts in an instruction
fn check_instruction(instr: &Instr, data: &mut RecurseData) {
    match instr {
        Instr::Print { expr }
        | Instr::LoadStorage { storage: expr, .. }
        | Instr::ClearStorage { storage: expr, .. }
        | Instr::BranchCond { cond: expr, .. }
        | Instr::PopStorage { storage: expr, .. }
        | Instr::SelfDestruct { recipient: expr }
        | Instr::Set { expr, .. } => {
            expr.recurse(data, check_expression);
        }
        Instr::Call { call, args, .. } => {
            if let InternalCallTy::Static { cfg_no } = call {
                // When we have an internal call, we analyse the current function again and the
                // function we are calling. This will guarantee convergence when there are
                // recursive function calls
                data.next_queue.insert((data.contract_no, *cfg_no));
                data.next_queue.insert((data.contract_no, data.cfg_func_no));
                match &data.contracts[data.contract_no].cfg[*cfg_no].function_no {
                    ASTFunction::SolidityFunction(ast_no) | ASTFunction::YulFunction(ast_no) => {
                        let accounts_to_add =
                            data.functions[*ast_no].solana_accounts.borrow().clone();
                        for (account_name, account) in accounts_to_add {
                            data.add_account(account_name, account);
                        }
                    }
                    _ => (),
                }
            } else if let InternalCallTy::Builtin { ast_func_no } = call {
                let name = &data.functions[*ast_func_no].name;
                if name == "create_program_address" {
                    data.add_system_account();
                }
            }

            for item in args {
                item.recurse(data, check_expression);
            }
        }
        Instr::Return { value } => {
            for item in value {
                item.recurse(data, check_expression);
            }
        }
        Instr::Branch { .. }
        | Instr::Nop
        | Instr::ReturnCode { .. }
        | Instr::PopMemory { .. }
        | Instr::Unimplemented { .. } => {}
        Instr::Store {
            dest,
            data: store_data,
        } => {
            dest.recurse(data, check_expression);
            store_data.recurse(data, check_expression);
        }

        Instr::AssertFailure { encoded_args } => {
            if let Some(args) = encoded_args {
                args.recurse(data, check_expression);
            }
        }

        Instr::ValueTransfer {
            address: expr1,
            value: expr2,
            ..
        }
        | Instr::ReturnData {
            data: expr1,
            data_len: expr2,
        }
        | Instr::SetStorage {
            value: expr1,
            storage: expr2,
            ..
        } => {
            expr1.recurse(data, check_expression);
            expr2.recurse(data, check_expression);
        }
        Instr::WriteBuffer {
            buf: expr_1,
            offset: expr_2,
            value: expr_3,
        }
        | Instr::MemCopy {
            source: expr_1,
            destination: expr_2,
            bytes: expr_3,
        }
        | Instr::SetStorageBytes {
            value: expr_1,
            storage: expr_2,
            offset: expr_3,
        } => {
            expr_1.recurse(data, check_expression);
            expr_2.recurse(data, check_expression);
            expr_3.recurse(data, check_expression);
        }
        Instr::PushStorage {
            value: opt_expr,
            storage: expr,
            ..
        } => {
            if let Some(opt_expr) = opt_expr {
                opt_expr.recurse(data, check_expression);
            }
            expr.recurse(data, check_expression);
        }
        Instr::PushMemory { value, .. } => {
            value.recurse(data, check_expression);
        }
        Instr::Constructor {
            encoded_args,
            value,
            gas,
            salt,
            address,
            seeds,
            accounts,
            ..
        } => {
            encoded_args.recurse(data, check_expression);
            if let Some(value) = value {
                value.recurse(data, check_expression);
            }
            gas.recurse(data, check_expression);
            if let Some(salt) = salt {
                salt.recurse(data, check_expression);
            }
            if let Some(address) = address {
                address.recurse(data, check_expression);
            }
            if let Some(seeds) = seeds {
                seeds.recurse(data, check_expression);
            }
            if let Some(accounts) = accounts {
                accounts.recurse(data, check_expression);
            }

            data.add_system_account();
        }
        Instr::ExternalCall {
            address,
            accounts,
            seeds,
            payload,
            value,
            gas,
            contract_function_no,
            ..
        } => {
            if let Some(address) = address {
                address.recurse(data, check_expression);
                if let Expression::NumberLiteral { value, .. } = address {
                    // Check if we can auto populate this account
                    if let Some(account) = account_from_number(value) {
                        data.add_account(
                            account.to_string(),
                            SolanaAccount {
                                is_signer: false,
                                is_writer: false,
                            },
                        );
                    }
                }
            }
            if let Some(accounts) = accounts {
                accounts.recurse(data, check_expression);
            }
            if let Some(seeds) = seeds {
                seeds.recurse(data, check_expression);
            }
            payload.recurse(data, check_expression);
            value.recurse(data, check_expression);
            gas.recurse(data, check_expression);
            // External calls always need the system account
            data.add_system_account();
            if let Some((contract_no, function_no)) = contract_function_no {
                let cfg_no = data.contracts[*contract_no].all_functions[function_no];
                let accounts_to_add = data.functions[*function_no]
                    .solana_accounts
                    .borrow()
                    .clone();
                for (account_name, account) in accounts_to_add {
                    data.add_account(account_name, account);
                }
                data.next_queue.insert((*contract_no, cfg_no));
                data.next_queue.insert((data.contract_no, data.cfg_func_no));
            }
        }
        Instr::EmitEvent {
            data: data_,
            topics,
            ..
        } => {
            data_.recurse(data, check_expression);

            for item in topics {
                item.recurse(data, check_expression);
            }
        }
        Instr::Switch { cond, cases, .. } => {
            cond.recurse(data, check_expression);
            for (expr, _) in cases {
                expr.recurse(data, check_expression);
            }
        }
    }
}

/// Collect accounts from this expression
fn check_expression(expr: &Expression, data: &mut RecurseData) -> bool {
    match expr {
        Expression::Builtin {
            kind: Builtin::Timestamp | Builtin::BlockNumber | Builtin::Slot,
            ..
        } => {
            data.add_account(
                CLOCK_ACCOUNT.to_string(),
                SolanaAccount {
                    is_signer: false,
                    is_writer: false,
                },
            );
        }
        Expression::Builtin {
            kind: Builtin::SignatureVerify,
            ..
        } => {
            data.add_account(
                INSTRUCTION_ACCOUNT.to_string(),
                SolanaAccount {
                    is_writer: false,
                    is_signer: false,
                },
            );
        }
        Expression::Builtin {
            kind: Builtin::Ripemd160 | Builtin::Keccak256 | Builtin::Sha256,
            ..
        } => {
            data.add_system_account();
        }

        _ => (),
    }

    true
}
