// SPDX-License-Identifier: Apache-2.0

pub(crate) mod accounts;
pub(super) mod deploy;
pub(crate) mod dispatch;
pub(crate) mod encoding;
mod events;

use self::accounts::account_collection::collect_accounts_from_contract;
use self::accounts::account_management::manage_contract_accounts;
use self::events::SolanaEventEmitter;
use crate::codegen::cfg::ControlFlowGraph;
use crate::codegen::interface::{EventEmitter, TargetCodegen};
use crate::codegen::storage::{array_pop, array_push};
use crate::codegen::vartable::Vartable;
use crate::codegen::{Expression, Options};
use crate::sema::ast::{self, Function, Namespace, StructType, Type};
use num_bigint::BigInt;
use num_traits::Zero;
use solang_parser::pt::{self, Loc};

pub(crate) struct SolanaTarget;

impl TargetCodegen for SolanaTarget {
    fn function_dispatch(
        &self,
        contract_no: usize,
        all_cfg: &mut [ControlFlowGraph],
        ns: &mut Namespace,
        opt: &Options,
    ) -> Vec<ControlFlowGraph> {
        vec![dispatch::function_dispatch(
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

    fn lower_storage_array_length(
        &self,
        loc: &Loc,
        ty: &Type,
        array: Expression,
        elem_ty: &Type,
        _cfg: &mut ControlFlowGraph,
        _vartab: &mut Vartable,
        _ns: &Namespace,
    ) -> Expression {
        Expression::StorageArrayLength {
            loc: *loc,
            ty: ty.clone(),
            array: Box::new(array),
            elem_ty: elem_ty.clone(),
        }
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
