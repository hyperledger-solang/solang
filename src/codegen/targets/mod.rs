// SPDX-License-Identifier: Apache-2.0

pub(crate) mod soroban;

use crate::codegen::cfg::ControlFlowGraph;
use crate::codegen::interface::TargetCodegen;
use crate::codegen::solana_accounts::account_collection::collect_accounts_from_contract;
use crate::codegen::solana_accounts::account_management::manage_contract_accounts;
use crate::codegen::storage::{
    array_pop, array_push, storage_slots_array_pop, storage_slots_array_push,
};
use crate::codegen::vartable::Vartable;
use crate::codegen::{dispatch, Expression, Options};
use crate::sema::ast;
use crate::sema::ast::{Function, Namespace, RetrieveType, Type};
use crate::Target;
use solang_parser::pt::Loc;

use self::soroban::SorobanTarget;

pub(crate) fn make_target(ns: &Namespace) -> Box<dyn TargetCodegen> {
    match &ns.target {
        Target::Soroban => Box::new(SorobanTarget),
        Target::Solana => Box::new(SolanaTarget),
        // EVM reuses the Polkadot codegen path — intentional, not a gap.
        Target::Polkadot { .. } | Target::EVM => Box::new(PolkadotTarget),
    }
}

pub(crate) struct SolanaTarget;
pub(crate) struct PolkadotTarget;

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
}
