// SPDX-License-Identifier: Apache-2.0

pub(crate) mod soroban;

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::events::polkadot::PolkadotEventEmitter;
use crate::codegen::events::solana::SolanaEventEmitter;
use crate::codegen::expression::expression;
use crate::codegen::interface::{EventEmitter, TargetCodegen};
use crate::codegen::solana_accounts::account_collection::collect_accounts_from_contract;
use crate::codegen::solana_accounts::account_management::manage_contract_accounts;
use crate::codegen::storage::{
    array_pop, array_push, storage_slots_array_pop, storage_slots_array_push,
};
use crate::codegen::vartable::Vartable;
use crate::codegen::{dispatch, polkadot, Expression, Options};
use crate::sema::ast;
use crate::sema::ast::{
    CallTy, ExternalCallAccounts, Function, Namespace, RetrieveType, StructType, Type,
};
use crate::Target;
use num_bigint::BigInt;
use num_traits::Zero;
use solang_parser::pt::{self, Loc};

use self::soroban::SorobanTarget;

pub(crate) fn make_target(ns: &Namespace) -> Box<dyn TargetCodegen> {
    match &ns.target {
        Target::Soroban => Box::new(SorobanTarget),
        Target::Solana => Box::new(SolanaTarget),
        Target::Polkadot { .. } => Box::new(PolkadotTarget { is_evm: false }),
        Target::EVM => Box::new(PolkadotTarget { is_evm: true }),
    }
}

pub(crate) struct SolanaTarget;
pub(crate) struct PolkadotTarget {
    pub(crate) is_evm: bool,
}

impl TargetCodegen for SolanaTarget {
    fn function_dispatch(
        &self,
        contract_no: usize,
        all_cfg: &mut [ControlFlowGraph],
        ns: &mut Namespace,
        opt: &Options,
    ) -> Vec<ControlFlowGraph> {
        vec![dispatch::solana::function_dispatch(
            contract_no,
            all_cfg,
            ns,
            opt,
            self,
        )]
    }

    fn post_process_program(&self, ns: &mut Namespace, _opt: &Options) {
        for contract_no in 0..ns.contracts.len() {
            if ns.contracts[contract_no].instantiable {
                let diag = collect_accounts_from_contract(contract_no, ns);
                ns.diagnostics.extend(diag);
            }
        }

        for contract_no in 0..ns.contracts.len() {
            if ns.contracts[contract_no].instantiable {
                manage_contract_accounts(contract_no, ns);
            }
        }
    }

    fn storage_array_length_is_inline(&self) -> bool {
        true
    }

    fn selector_hash_algorithm(&self) -> ast::Builtin {
        ast::Builtin::Sha256
    }

    fn initial_storage_slot(&self) -> BigInt {
        BigInt::from(crate::codegen::SOLANA_FIRST_OFFSET)
    }

    fn align_storage_slot(&self, mut slot: BigInt, ty: &Type, ns: &Namespace) -> BigInt {
        let alignment = ty.align_of(ns);
        let offset = slot.clone() % alignment;
        if offset > BigInt::zero() {
            slot += alignment - offset;
        }
        slot
    }

    fn storage_array_push(
        &self,
        loc: &Loc,
        args: &[ast::Expression],
        cfg: &mut ControlFlowGraph,
        contract_no: usize,
        func: Option<&Function>,
        ns: &Namespace,
        vartab: &mut Vartable,
        opt: &Options,
    ) -> Expression {
        // Solana stores dynamic arrays as flat slots.
        array_push(loc, args, cfg, contract_no, func, ns, vartab, opt, self)
    }

    fn storage_array_pop(
        &self,
        loc: &Loc,
        args: &[ast::Expression],
        return_ty: &Type,
        cfg: &mut ControlFlowGraph,
        contract_no: usize,
        func: Option<&Function>,
        ns: &Namespace,
        vartab: &mut Vartable,
        opt: &Options,
    ) -> Expression {
        array_pop(
            loc,
            args,
            return_ty,
            cfg,
            contract_no,
            func,
            ns,
            vartab,
            opt,
            self,
        )
    }

    fn event_emitter<'a>(
        &self,
        loc: &pt::Loc,
        event_no: usize,
        args: &'a [ast::Expression],
        ns: &'a Namespace,
    ) -> Box<dyn EventEmitter + 'a> {
        Box::new(SolanaEventEmitter {
            loc: *loc,
            args,
            ns,
            event_no,
        })
    }

    fn lower_storage_struct_member(
        &self,
        loc: &Loc,
        var_expr: Expression,
        struct_ty: &StructType,
        field_no: usize,
        ns: &Namespace,
        _cfg: &mut ControlFlowGraph,
        _vartab: &mut Vartable,
    ) -> Expression {
        let offset = struct_ty.definition(ns).storage_offsets[field_no].clone();
        Expression::Add {
            loc: *loc,
            ty: ns.storage_type(),
            overflowing: true,
            left: Box::new(var_expr),
            right: Box::new(Expression::NumberLiteral {
                loc: *loc,
                ty: ns.storage_type(),
                value: offset,
            }),
        }
    }
}

impl TargetCodegen for PolkadotTarget {
    fn function_dispatch(
        &self,
        contract_no: usize,
        all_cfg: &mut [ControlFlowGraph],
        ns: &mut Namespace,
        opt: &Options,
    ) -> Vec<ControlFlowGraph> {
        dispatch::polkadot::function_dispatch(contract_no, all_cfg, ns, opt)
    }

    fn storage_array_push(
        &self,
        loc: &Loc,
        args: &[ast::Expression],
        cfg: &mut ControlFlowGraph,
        contract_no: usize,
        func: Option<&Function>,
        ns: &Namespace,
        vartab: &mut Vartable,
        opt: &Options,
    ) -> Expression {
        // `bytes` uses the flat-slot path; typed arrays use hashed slots.
        if args[0].ty().is_storage_bytes() {
            array_push(loc, args, cfg, contract_no, func, ns, vartab, opt, self)
        } else {
            storage_slots_array_push(loc, args, cfg, contract_no, func, ns, vartab, opt, self)
        }
    }

    fn storage_array_pop(
        &self,
        loc: &Loc,
        args: &[ast::Expression],
        return_ty: &Type,
        cfg: &mut ControlFlowGraph,
        contract_no: usize,
        func: Option<&Function>,
        ns: &Namespace,
        vartab: &mut Vartable,
        opt: &Options,
    ) -> Expression {
        if args[0].ty().is_storage_bytes() {
            array_pop(
                loc,
                args,
                return_ty,
                cfg,
                contract_no,
                func,
                ns,
                vartab,
                opt,
                self,
            )
        } else {
            storage_slots_array_pop(
                loc,
                args,
                return_ty,
                cfg,
                contract_no,
                func,
                ns,
                vartab,
                opt,
                self,
            )
        }
    }

    fn event_emitter<'a>(
        &self,
        _loc: &pt::Loc,
        event_no: usize,
        args: &'a [ast::Expression],
        ns: &'a Namespace,
    ) -> Box<dyn EventEmitter + 'a> {
        Box::new(PolkadotEventEmitter { args, ns, event_no })
    }

    fn default_gas_builtin(&self) -> BigInt {
        if self.is_evm {
            BigInt::from(i64::MAX)
        } else {
            BigInt::zero()
        }
    }

    fn lower_print_expr(&self, expr: Expression) -> Expression {
        if self.is_evm {
            expr
        } else {
            crate::codegen::expression::add_prefix_and_delimiter_to_print(expr)
        }
    }

    fn lower_mapping_subscript(
        &self,
        loc: &Loc,
        elem_ty: &Type,
        array_ty: &Type,
        array: Expression,
        index: Expression,
    ) -> Expression {
        if self.is_evm {
            Expression::Subscript {
                loc: *loc,
                ty: elem_ty.clone(),
                array_ty: array_ty.clone(),
                expr: Box::new(array),
                index: Box::new(index),
            }
        } else {
            Expression::Keccak256 {
                loc: *loc,
                ty: array_ty.clone(),
                exprs: vec![array, index],
            }
        }
    }

    fn lower_builtin(
        &self,
        loc: &Loc,
        builtin: ast::Builtin,
        args: &[ast::Expression],
        cfg: &mut ControlFlowGraph,
        contract_no: usize,
        func: Option<&Function>,
        ns: &Namespace,
        vartab: &mut Vartable,
        opt: &Options,
    ) -> Option<Expression> {
        match builtin {
            ast::Builtin::Gasprice if self.is_evm && args.len() == 1 => {
                Some(crate::codegen::expression::builtin_evm_gasprice(
                    loc,
                    args,
                    cfg,
                    contract_no,
                    func,
                    ns,
                    vartab,
                    opt,
                    self,
                ))
            }
            ast::Builtin::PayableSend => {
                Some(self.payable_send(loc, args, cfg, contract_no, func, ns, vartab, opt))
            }
            ast::Builtin::PayableTransfer => {
                Some(self.payable_transfer(loc, args, cfg, contract_no, func, ns, vartab, opt))
            }
            _ => None,
        }
    }

    fn lower_storage_struct_member(
        &self,
        loc: &Loc,
        var_expr: Expression,
        struct_ty: &StructType,
        field_no: usize,
        ns: &Namespace,
        _cfg: &mut ControlFlowGraph,
        _vartab: &mut Vartable,
    ) -> Expression {
        // Polkadot/EVM lay struct fields out in consecutive storage slots.
        let offset: BigInt = struct_ty.definition(ns).fields[..field_no]
            .iter()
            .filter(|field| !field.infinite_size)
            .map(|field| field.ty.storage_slots(ns))
            .sum();
        Expression::Add {
            loc: *loc,
            ty: ns.storage_type(),
            overflowing: true,
            left: Box::new(var_expr),
            right: Box::new(Expression::NumberLiteral {
                loc: *loc,
                ty: ns.storage_type(),
                value: offset,
            }),
        }
    }
}

impl PolkadotTarget {
    /// Lower `address.send(value)` (`Builtin::PayableSend`). EVM routes through an external
    /// call with an empty payload; Polkadot emits a `ValueTransfer` and checks the return code.
    fn payable_send(
        &self,
        loc: &Loc,
        args: &[ast::Expression],
        cfg: &mut ControlFlowGraph,
        contract_no: usize,
        func: Option<&Function>,
        ns: &Namespace,
        vartab: &mut Vartable,
        opt: &Options,
    ) -> Expression {
        let address = expression(&args[0], cfg, contract_no, func, ns, vartab, opt, self);
        let value = expression(&args[1], cfg, contract_no, func, ns, vartab, opt, self);
        let success = vartab.temp(
            &pt::Identifier {
                loc: *loc,
                name: "success".to_owned(),
            },
            &Type::Uint(32),
        );

        // Ethereum can only transfer via external call
        if self.is_evm {
            cfg.add(
                vartab,
                Instr::ExternalCall {
                    loc: *loc,
                    success: Some(success),
                    address: Some(address),
                    accounts: ExternalCallAccounts::AbsentArgument,
                    seeds: None,
                    payload: Expression::AllocDynamicBytes {
                        loc: *loc,
                        ty: Type::DynamicBytes,
                        size: Box::new(Expression::NumberLiteral {
                            loc: *loc,
                            ty: Type::Uint(32),
                            value: BigInt::from(0),
                        }),
                        initializer: Some(vec![]),
                    },
                    value,
                    gas: Expression::NumberLiteral {
                        loc: *loc,
                        ty: Type::Uint(64),
                        value: BigInt::from(i64::MAX),
                    },
                    callty: CallTy::Regular,
                    contract_function_no: None,
                    flags: None,
                },
            );
            return Expression::Variable {
                loc: *loc,
                ty: Type::Bool,
                var_no: success,
            };
        }

        cfg.add(
            vartab,
            Instr::ValueTransfer {
                success: Some(success),
                address,
                value,
            },
        );

        polkadot::check_transfer_ret(loc, success, cfg, ns, opt, vartab, false).unwrap()
    }

    /// Lower `address.transfer(value)` (`Builtin::PayableTransfer`). EVM routes through an
    /// external call; Polkadot emits a `ValueTransfer` and reverts on a non-zero return code.
    fn payable_transfer(
        &self,
        loc: &Loc,
        args: &[ast::Expression],
        cfg: &mut ControlFlowGraph,
        contract_no: usize,
        func: Option<&Function>,
        ns: &Namespace,
        vartab: &mut Vartable,
        opt: &Options,
    ) -> Expression {
        let address = expression(&args[0], cfg, contract_no, func, ns, vartab, opt, self);
        let value = expression(&args[1], cfg, contract_no, func, ns, vartab, opt, self);
        if self.is_evm {
            // Ethereum can only transfer via external call
            cfg.add(
                vartab,
                Instr::ExternalCall {
                    loc: *loc,
                    success: None,
                    accounts: ExternalCallAccounts::AbsentArgument,
                    seeds: None,
                    address: Some(address),
                    payload: Expression::AllocDynamicBytes {
                        loc: *loc,
                        ty: Type::DynamicBytes,
                        size: Box::new(Expression::NumberLiteral {
                            loc: *loc,
                            ty: Type::Uint(32),
                            value: BigInt::from(0),
                        }),
                        initializer: Some(vec![]),
                    },
                    value,
                    gas: Expression::NumberLiteral {
                        loc: *loc,
                        ty: Type::Uint(64),
                        value: BigInt::from(i64::MAX),
                    },
                    callty: CallTy::Regular,
                    contract_function_no: None,
                    flags: None,
                },
            );
            return Expression::Poison;
        }

        let success = vartab.temp_name("success", &Type::Uint(32));
        cfg.add(
            vartab,
            Instr::ValueTransfer {
                success: Some(success),
                address,
                value,
            },
        );

        polkadot::check_transfer_ret(loc, success, cfg, ns, opt, vartab, true);

        Expression::Poison
    }
}
