// SPDX-License-Identifier: Apache-2.0

//! Contains `codegen` helpers for the Polkadot target.

use solang_parser::pt::Loc;

use crate::{
    codegen::{
        cfg::{ControlFlowGraph, Instr},
        revert::log_runtime_error,
        vartable::Vartable,
        Expression, Options,
    },
    sema::ast::{Namespace, Type},
};

// When using the seal api, we use our own scratch buffer.
pub const SCRATCH_SIZE: u32 = 32 * 1024;

/// Helper to handle error cases from external function and constructor calls.
pub(crate) struct RetCodeCheck {
    pub success: usize,
    pub revert: usize,
    pub error_no_data: usize,
    msg: &'static str,
    loc: Loc,
}

#[derive(Default)]
pub(crate) struct RetCodeCheckBuilder<Success = ()> {
    loc: Loc,
    msg: &'static str,
    success_var: Success,
}

impl RetCodeCheckBuilder {
    pub(crate) fn loc(mut self, loc: Loc) -> Self {
        self.loc = loc;
        self
    }

    pub(crate) fn msg(mut self, msg: &'static str) -> Self {
        self.msg = msg;
        self
    }

    pub(crate) fn success_var(self, success_var: usize) -> RetCodeCheckBuilder<usize> {
        RetCodeCheckBuilder {
            loc: self.loc,
            msg: self.msg,
            success_var,
        }
    }
}

impl RetCodeCheckBuilder<usize> {
    pub(crate) fn insert(self, cfg: &mut ControlFlowGraph, vartab: &mut Vartable) -> RetCodeCheck {
        let cond = Expression::Variable {
            loc: self.loc,
            ty: Type::Uint(32),
            var_no: self.success_var,
        };
        let ret = RetCodeCheck {
            success: cfg.new_basic_block("ret_success".into()),
            revert: cfg.new_basic_block("ret_bubble".into()),
            error_no_data: cfg.new_basic_block("ret_no_data".into()),
            msg: self.msg,
            loc: self.loc,
        };
        let cases = vec![
            (
                Expression::NumberLiteral {
                    loc: self.loc,
                    ty: Type::Uint(32),
                    value: 0.into(),
                },
                ret.success,
            ),
            (
                Expression::NumberLiteral {
                    loc: self.loc,
                    ty: Type::Uint(32),
                    value: 2.into(),
                },
                ret.revert,
            ),
        ];
        let ins = Instr::Switch {
            cond,
            cases,
            default: ret.error_no_data,
        };
        cfg.add(vartab, ins);

        ret
    }
}

impl RetCodeCheck {
    /// Handles all cases from the [RetBlock] accordingly.
    /// On success, nothing is done and the execution continues at the success block.
    /// If the callee reverted and output was supplied, it will be bubble up.
    /// Otherwise, a revert without data will be inserted.
    pub(crate) fn handle_cases(
        &self,
        cfg: &mut ControlFlowGraph,
        ns: &Namespace,
        opt: &Options,
        vartab: &mut Vartable,
    ) {
        cfg.set_basic_block(self.error_no_data);
        log_runtime_error(opt.log_runtime_errors, self.msg, self.loc, cfg, vartab, ns);
        cfg.add(vartab, Instr::AssertFailure { encoded_args: None });

        cfg.set_basic_block(self.revert);
        log_runtime_error(opt.log_runtime_errors, self.msg, self.loc, cfg, vartab, ns);
        let encoded_args = Expression::ReturnData { loc: self.loc }.into();
        cfg.add(vartab, Instr::AssertFailure { encoded_args });

        cfg.set_basic_block(self.success);
    }
}

/// Check the return code of `transfer`.
///
/// If `bubble_up` is set to true, this will revert the contract execution on failure.
/// Otherwise, the expression comparing the return code against `0` is returned.
pub(super) fn check_transfer_ret(
    loc: &Loc,
    success: usize,
    cfg: &mut ControlFlowGraph,
    ns: &Namespace,
    opt: &Options,
    vartab: &mut Vartable,
    bubble_up: bool,
) -> Option<Expression> {
    let ret_code = Expression::Variable {
        loc: *loc,
        ty: Type::Uint(32),
        var_no: success,
    };
    let ret_ok = Expression::NumberLiteral {
        loc: *loc,
        ty: Type::Uint(32),
        value: 0.into(),
    };
    let cond = Expression::Equal {
        loc: *loc,
        left: ret_code.into(),
        right: ret_ok.into(),
    };

    if !bubble_up {
        return Some(cond);
    }

    let success_block = cfg.new_basic_block("transfer_success".into());
    let fail_block = cfg.new_basic_block("transfer_fail".into());
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond,
            true_block: success_block,
            false_block: fail_block,
        },
    );

    cfg.set_basic_block(fail_block);
    let msg = "value transfer failure";
    log_runtime_error(opt.log_runtime_errors, msg, *loc, cfg, vartab, ns);
    cfg.add(vartab, Instr::AssertFailure { encoded_args: None });

    cfg.set_basic_block(success_block);

    None
}
