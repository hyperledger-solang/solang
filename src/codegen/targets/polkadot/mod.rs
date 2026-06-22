// SPDX-License-Identifier: Apache-2.0

pub(crate) mod dispatch;
pub(crate) mod encoding;
pub(crate) mod events;
pub(crate) mod return_code;
pub(crate) mod try_catch;

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::expression::{expression, load_storage};
use crate::codegen::interface::{EventEmitter, TargetCodegen};
use crate::codegen::storage::{
    array_pop, array_push, storage_slots_array_pop, storage_slots_array_push,
};
use crate::codegen::vartable::Vartable;
use crate::codegen::{Expression, Options};
use crate::sema::ast;
use crate::sema::ast::{
    CallTy, ExternalCallAccounts, Function, Namespace, RetrieveType, StructType, Type,
};
use num_bigint::BigInt;
use solang_parser::pt::{self, Loc};

use self::events::PolkadotEventEmitter;

pub(crate) struct PolkadotTarget;

impl TargetCodegen for PolkadotTarget {
    fn function_dispatch(
        &self,
        contract_no: usize,
        all_cfg: &mut [ControlFlowGraph],
        ns: &mut Namespace,
        opt: &Options,
    ) -> Vec<ControlFlowGraph> {
        dispatch::function_dispatch(contract_no, all_cfg, ns, opt)
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

    // Polkadot hashes (array, index) with keccak256 to form the storage key.
    // All other targets use a direct Subscript (trait default).
    fn lower_mapping_subscript(
        &self,
        loc: &Loc,
        _elem_ty: &Type,
        array_ty: &Type,
        array: Expression,
        index: Expression,
    ) -> Expression {
        Expression::Keccak256 {
            loc: *loc,
            ty: array_ty.clone(),
            exprs: vec![array, index],
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

    // Polkadot prepends a `print:`/`,\n` delimiter; EVM and others pass through (trait default).
    fn lower_print_expr(&self, expr: Expression) -> Expression {
        crate::codegen::expression::add_prefix_and_delimiter_to_print(expr)
    }

    fn lower_storage_array_length(
        &self,
        loc: &Loc,
        _ty: &Type,
        array: Expression,
        _elem_ty: &Type,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
        ns: &Namespace,
    ) -> Expression {
        load_storage(loc, &ns.storage_type(), array, cfg, vartab, None, ns, self)
    }
}

impl PolkadotTarget {
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
        cfg.add(
            vartab,
            Instr::ValueTransfer {
                success: Some(success),
                address,
                value,
            },
        );
        return_code::check_transfer_ret(loc, success, cfg, ns, opt, vartab, false).unwrap()
    }

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
        let success = vartab.temp_name("success", &Type::Uint(32));
        cfg.add(
            vartab,
            Instr::ValueTransfer {
                success: Some(success),
                address,
                value,
            },
        );
        return_code::check_transfer_ret(loc, success, cfg, ns, opt, vartab, true);
        Expression::Poison
    }
}

pub(crate) struct EvmTarget(pub(crate) PolkadotTarget);

impl TargetCodegen for EvmTarget {
    fn default_gas_builtin(&self) -> BigInt {
        BigInt::from(i64::MAX)
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
            ast::Builtin::Gasprice if args.len() == 1 => {
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

    fn function_dispatch(
        &self,
        contract_no: usize,
        all_cfg: &mut [ControlFlowGraph],
        ns: &mut Namespace,
        opt: &Options,
    ) -> Vec<ControlFlowGraph> {
        self.0.function_dispatch(contract_no, all_cfg, ns, opt)
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
        self.0
            .storage_array_push(loc, args, cfg, contract_no, func, ns, vartab, opt)
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
        self.0.storage_array_pop(
            loc,
            args,
            return_ty,
            cfg,
            contract_no,
            func,
            ns,
            vartab,
            opt,
        )
    }

    fn event_emitter<'a>(
        &self,
        loc: &pt::Loc,
        event_no: usize,
        args: &'a [ast::Expression],
        ns: &'a Namespace,
    ) -> Box<dyn EventEmitter + 'a> {
        self.0.event_emitter(loc, event_no, args, ns)
    }

    fn lower_storage_struct_member(
        &self,
        loc: &Loc,
        var_expr: Expression,
        struct_ty: &StructType,
        field_no: usize,
        ns: &Namespace,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
    ) -> Expression {
        self.0
            .lower_storage_struct_member(loc, var_expr, struct_ty, field_no, ns, cfg, vartab)
    }

    fn lower_storage_array_length(
        &self,
        loc: &Loc,
        ty: &Type,
        array: Expression,
        elem_ty: &Type,
        cfg: &mut ControlFlowGraph,
        vartab: &mut Vartable,
        ns: &Namespace,
    ) -> Expression {
        self.0
            .lower_storage_array_length(loc, ty, array, elem_ty, cfg, vartab, ns)
    }
}

impl EvmTarget {
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
        Expression::Variable {
            loc: *loc,
            ty: Type::Bool,
            var_no: success,
        }
    }

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
        Expression::Poison
    }
}
