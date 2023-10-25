// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::Instr;
use crate::codegen::dispatch::solana::SOLANA_DISPATCH_CFG_NAME;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{
    ArrayLength, Contract, ExternalCallAccounts, Function, Namespace, StructType, Type,
};
use crate::sema::solana_accounts::BuiltinAccounts;
use num_bigint::BigInt;
use solang_parser::pt::Loc;
use std::collections::{HashSet, VecDeque};

/// This function walks over the CFG and automates the account management, so developers do not need
/// to do so. For instance, when calling 'new construct{address: addr}()', we construct the correct
/// AccountMeta array with all the accounts the constructor needs.
pub(crate) fn manage_contract_accounts(contract_no: usize, ns: &mut Namespace) {
    let contract_functions = ns.contracts[contract_no].functions.clone();
    let mut constructor_no = None;
    for function_no in &contract_functions {
        if ns.functions[*function_no].is_constructor() {
            constructor_no = Some(*function_no);
        }
        let cfg_no = ns.contracts[contract_no]
            .all_functions
            .get(function_no)
            .copied()
            .unwrap();
        traverse_cfg(
            &mut ns.contracts,
            contract_no,
            &ns.functions,
            cfg_no,
            *function_no,
        );
    }

    if let Some(constructor) = constructor_no {
        let dispatch = ns.contracts[contract_no]
            .cfg
            .iter()
            .position(|cfg| cfg.name == SOLANA_DISPATCH_CFG_NAME)
            .expect("dispatch CFG is always generated");
        traverse_cfg(
            &mut ns.contracts,
            contract_no,
            &ns.functions,
            dispatch,
            constructor,
        );
    }
}

/// This function walks over the CFG to process its instructions for the account management.
fn traverse_cfg(
    contracts: &mut [Contract],
    contract_no: usize,
    functions: &[Function],
    cfg_no: usize,
    ast_no: usize,
) {
    if contracts[contract_no].cfg[cfg_no].blocks.is_empty() {
        return;
    }

    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut visited: HashSet<usize> = HashSet::new();
    queue.push_back(0);
    visited.insert(0);

    while let Some(cur_block) = queue.pop_front() {
        for instr_no in 0..contracts[contract_no].cfg[cfg_no].blocks[cur_block]
            .instr
            .len()
        {
            process_instruction(
                cfg_no,
                instr_no,
                cur_block,
                functions,
                contracts,
                ast_no,
                contract_no,
            );
        }

        for edge in contracts[contract_no].cfg[cfg_no].blocks[cur_block].successors() {
            if !visited.contains(&edge) {
                queue.push_back(edge);
                visited.insert(edge);
            }
        }
    }
}

/// This function processes the instruction, creating the AccountMeta array when possible.
/// Presently, we only check the Instr::Constructor, but more will come later.
fn process_instruction(
    cfg_no: usize,
    instr_no: usize,
    block_no: usize,
    functions: &[Function],
    contracts: &mut [Contract],
    ast_no: usize,
    contract_no: usize,
) {
    let instr = &mut contracts[contract_no].cfg[cfg_no].blocks[block_no].instr[instr_no];
    match instr {
        Instr::Constructor {
            accounts,
            constructor_no: Some(func_no),
            contract_no,
            ..
        }
        | Instr::ExternalCall {
            accounts,
            contract_function_no: Some((contract_no, func_no)),
            ..
        } => {
            if !accounts.is_absent() {
                return;
            }

            let mut account_metas: Vec<Expression> = Vec::new();
            let constructor_func = &functions[*func_no];
            for (name, account) in constructor_func.solana_accounts.borrow().iter() {
                let name_to_index = if name == BuiltinAccounts::DataAccount {
                    format!("{}_dataAccount", contracts[*contract_no].id)
                } else {
                    name.clone()
                };

                let account_index = functions[ast_no]
                    .solana_accounts
                    .borrow()
                    .get_index_of(&name_to_index)
                    .unwrap();
                let ptr_to_address = accounts_vector_key_at_index(account_index);
                account_metas.push(account_meta_literal(
                    ptr_to_address,
                    account.is_signer,
                    account.is_writer,
                ));
            }

            let metas_vector = Expression::ArrayLiteral {
                loc: Loc::Codegen,
                ty: Type::Array(
                    Box::new(Type::Struct(StructType::AccountMeta)),
                    vec![ArrayLength::Fixed(BigInt::from(account_metas.len()))],
                ),
                dimensions: vec![account_metas.len() as u32],
                values: account_metas,
            };
            *accounts = ExternalCallAccounts::Present(metas_vector);
        }
        Instr::Constructor {
            contract_no,
            constructor_no: None,
            accounts,
            ..
        } => {
            let name_to_index = format!("{}_dataAccount", contracts[*contract_no].id);
            let account_index = functions[ast_no]
                .solana_accounts
                .borrow()
                .get_index_of(&name_to_index)
                .unwrap();
            let ptr_to_address = accounts_vector_key_at_index(account_index);
            let account_metas = vec![account_meta_literal(ptr_to_address, false, true)];
            let metas_vector = Expression::ArrayLiteral {
                loc: Loc::Codegen,
                ty: Type::Array(
                    Box::new(Type::Struct(StructType::AccountMeta)),
                    vec![ArrayLength::Fixed(BigInt::from(account_metas.len()))],
                ),
                dimensions: vec![1],
                values: account_metas,
            };
            *accounts = ExternalCallAccounts::Present(metas_vector);
        }
        Instr::AccountAccess { loc, name, var_no } => {
            // This could have been an Expression::AccountAccess if we had a three-address form.
            // The amount of code necessary to traverse all Instructions and all expressions recursively
            // (Expressions form a tree) makes the usage of Expression::AccountAccess too burdensome.

            // Alternatively, we can create a codegen::Expression::AccountAccess when we have the
            // new SSA IR complete.
            let account_index = functions[ast_no]
                .solana_accounts
                .borrow()
                .get_index_of(name)
                .unwrap();
            let expr = index_accounts_vector(account_index);

            *instr = Instr::Set {
                loc: *loc,
                res: *var_no,
                expr,
            };
        }
        _ => (),
    }
}

/// This function automates the process of retrieving 'tx.accounts[index].key'.
pub(crate) fn accounts_vector_key_at_index(index: usize) -> Expression {
    let payer_info = index_accounts_vector(index);

    retrieve_key_from_account_info(payer_info)
}

/// This function retrieves the account key from the AccountInfo struct.
/// The argument should be of type 'Type::Ref(Type::Struct(StructType::AccountInfo))'.
pub(crate) fn retrieve_key_from_account_info(account_info: Expression) -> Expression {
    let address = Expression::StructMember {
        loc: Loc::Codegen,
        ty: Type::Ref(Box::new(Type::Ref(Box::new(Type::Address(false))))),
        expr: Box::new(account_info),
        member: 0,
    };

    Expression::Load {
        loc: Loc::Codegen,
        ty: Type::Ref(Box::new(Type::Address(false))),
        expr: Box::new(address),
    }
}

/// This function automates the process of retrieving 'tx.accounts[index]'.
fn index_accounts_vector(index: usize) -> Expression {
    let accounts_vector = Expression::Builtin {
        loc: Loc::Codegen,
        tys: vec![Type::Array(
            Box::new(Type::Struct(StructType::AccountInfo)),
            vec![ArrayLength::Dynamic],
        )],
        kind: Builtin::Accounts,
        args: vec![],
    };

    Expression::Subscript {
        loc: Loc::Codegen,
        ty: Type::Ref(Box::new(Type::Struct(StructType::AccountInfo))),
        array_ty: Type::Array(
            Box::new(Type::Struct(StructType::AccountInfo)),
            vec![ArrayLength::Dynamic],
        ),
        expr: Box::new(accounts_vector),
        index: Box::new(Expression::NumberLiteral {
            loc: Loc::Codegen,
            ty: Type::Uint(32),
            value: BigInt::from(index),
        }),
    }
}

/// This function creates an AccountMeta struct literal.
pub(crate) fn account_meta_literal(
    address: Expression,
    is_signer: bool,
    is_writer: bool,
) -> Expression {
    Expression::StructLiteral {
        loc: Loc::Codegen,
        ty: Type::Struct(StructType::AccountMeta),
        values: vec![
            address,
            Expression::BoolLiteral {
                loc: Loc::Codegen,
                value: is_writer,
            },
            Expression::BoolLiteral {
                loc: Loc::Codegen,
                value: is_signer,
            },
        ],
    }
}
