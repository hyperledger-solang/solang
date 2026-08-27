// SPDX-License-Identifier: Apache-2.0

use crate::codegen::{
    cfg::{ASTFunction, ControlFlowGraph, Instr, InternalCallTy},
    encoding::{abi_decode, abi_encode},
    vartable::Vartable,
    Builtin, Expression, Options,
};
use crate::sema::ast::{Namespace, Parameter, Type, Type::Uint};
use num_bigint::{BigInt, Sign};
use solang_parser::pt::{FunctionTy, Loc::Codegen};

pub(crate) const RISCV_DISPATCH_CFG_NAME: &str = "riscv_dispatch";

/// r55 calldata dispatch. Unlike Polkadot, there's no separate deploy/call
/// export and no value-transfer check baked into the ABI here (r55 exposes
/// value via a syscall if needed, not via a dispatch arg). We handle both
/// constructor and regular function selectors in one dispatch table for now;
/// deploy vs. runtime binary separation happens at the `emit`/link stage.
pub(crate) fn function_dispatch(
    _contract_no: usize,
    all_cfg: &[ControlFlowGraph],
    ns: &mut Namespace,
    _opt: &Options,
) -> Vec<ControlFlowGraph> {
    vec![Dispatch::new(all_cfg, ns).build()]
}

struct Dispatch<'a> {
    start: usize,
    input_ptr: Expression,
    input_len: usize,
    vartab: Vartable,
    cfg: ControlFlowGraph,
    all_cfg: &'a [ControlFlowGraph],
    ns: &'a mut Namespace,
    selector_len: Box<Expression>,
}

fn new_cfg() -> ControlFlowGraph {
    let mut cfg = ControlFlowGraph::new(RISCV_DISPATCH_CFG_NAME.into(), ASTFunction::None);
    // single arg: pointer to the raw calldata bytes (already past the 8-byte
    // length prefix -- see emit/riscv/target.rs public_function_prelude).
    let input_ptr = Parameter {
        loc: Codegen,
        id: None,
        ty: Type::BufferPointer,
        ty_loc: None,
        indexed: false,
        readonly: true,
        infinite_size: false,
        recursive: false,
        annotation: None,
    };
    let mut input_len = input_ptr.clone();
    input_len.ty = Uint(32);
    cfg.params = vec![input_ptr, input_len].into();
    cfg
}

impl<'a> Dispatch<'a> {
    fn new(all_cfg: &'a [ControlFlowGraph], ns: &'a mut Namespace) -> Self {
        let mut vartab = Vartable::new(ns.next_id);
        let mut cfg = new_cfg();

        let input_len = vartab.temp_name("input_len", &Uint(32));
        cfg.add(
            &mut vartab,
            Instr::Set {
                loc: Codegen,
                res: input_len,
                expr: Expression::FunctionArg {
                    loc: Codegen,
                    ty: Uint(32),
                    arg_no: 1,
                },
            },
        );

        let input_ptr_var = vartab.temp_name("input_ptr", &Type::BufferPointer);
        cfg.add(
            &mut vartab,
            Instr::Set {
                loc: Codegen,
                res: input_ptr_var,
                expr: Expression::FunctionArg {
                    loc: Codegen,
                    ty: Type::BufferPointer,
                    arg_no: 0,
                },
            },
        );
        let input_ptr = Expression::Variable {
            loc: Codegen,
            ty: Type::BufferPointer,
            var_no: input_ptr_var,
        };

        // Selector is 4 bytes, matching r55's `u32::from_be_bytes(calldata[0..4])`.
        let selector_len: Box<Expression> = Expression::NumberLiteral {
            loc: Codegen,
            ty: Uint(32),
            value: 4.into(),
        }
        .into();
        let input_ptr = Expression::AdvancePointer {
            pointer: input_ptr.into(),
            bytes_offset: selector_len.clone(),
        };

        Self {
            start: cfg.new_basic_block("start_dispatch".into()),
            input_ptr,
            input_len,
            vartab,
            cfg,
            all_cfg,
            ns,
            selector_len,
        }
    }

    fn build(mut self) -> ControlFlowGraph {
        let cond = Expression::Less {
            loc: Codegen,
            signed: false,
            left: Expression::Variable {
                loc: Codegen,
                ty: Uint(32),
                var_no: self.input_len,
            }
            .into(),
            right: self.selector_len.clone(),
        };
        let invalid = self.cfg.new_basic_block("invalid_selector".into());
        self.add(Instr::BranchCond {
            cond,
            true_block: invalid,
            false_block: self.start,
        });

        // NOTE: r55 selectors are BIG-endian (`u32::from_be_bytes`), unlike
        // Polkadot which reads selectors little-endian via ReadFromBuffer.
        // Confirm ReadFromBuffer's endianness assumption in codegen/expression.rs
        // before trusting this -- you may need a dedicated big-endian read here,
        // or byte-swap the literal case values instead.
        let selector_ty = Uint(32);
        let cases = self
            .all_cfg
            .iter()
            .enumerate()
            .filter_map(|(func_no, func_cfg)| {
                if matches!(func_cfg.ty, FunctionTy::Function | FunctionTy::Constructor)
                    && func_cfg.public
                {
                    let selector = BigInt::from_bytes_be(Sign::Plus, &func_cfg.selector);
                    let case = Expression::NumberLiteral {
                        loc: Codegen,
                        ty: selector_ty.clone(),
                        value: selector,
                    };
                    Some((case, self.dispatch_case(func_no)))
                } else {
                    None
                }
            })
            .collect();

        self.cfg.set_basic_block(self.start);
        let selector_var = self.vartab.temp_name("selector", &selector_ty);
        self.add(Instr::Set {
            loc: Codegen,
            res: selector_var,
            expr: Expression::Builtin {
                loc: Codegen,
                tys: vec![selector_ty.clone()],
                kind: Builtin::ReadFromBuffer,
                args: vec![
                    Expression::FunctionArg {
                        loc: Codegen,
                        ty: Type::BufferPointer,
                        arg_no: 0,
                    },
                    Expression::NumberLiteral {
                        loc: Codegen,
                        ty: selector_ty.clone(),
                        value: 0.into(),
                    },
                ],
            },
        });
        let selector = Expression::Variable {
            loc: Codegen,
            ty: selector_ty,
            var_no: selector_var,
        };
        self.add(Instr::Switch {
            cond: selector,
            cases,
            default: invalid,
        });

        self.cfg.set_basic_block(invalid);
        // TODO: emit r55 Revert syscall (t0=4) here instead of a generic instr,
        // once you add an Instr/Expression path that lowers to `ecall`.
        self.add(Instr::AssertFailure { encoded_args: None });

        self.vartab.finalize(self.ns, &mut self.cfg);
        self.cfg
    }

    fn dispatch_case(&mut self, func_no: usize) -> usize {
        let case_bb = self.cfg.new_basic_block(format!("func_{func_no}_dispatch"));
        self.cfg.set_basic_block(case_bb);

        let cfg = &self.all_cfg[func_no];
        let mut args = vec![];
        if !cfg.params.is_empty() {
            let buf_len = Expression::Variable {
                loc: Codegen,
                ty: Uint(32),
                var_no: self.input_len,
            };
            let arg_len = Expression::Subtract {
                loc: Codegen,
                ty: Uint(32),
                overflowing: false,
                left: buf_len.into(),
                right: self.selector_len.clone(),
            };
            args = abi_decode(
                &Codegen,
                &self.input_ptr,
                &cfg.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
                self.ns,
                &mut self.vartab,
                &mut self.cfg,
                Some(Expression::Trunc {
                    loc: Codegen,
                    ty: Uint(32),
                    expr: arg_len.into(),
                }),
            );
        }

        let mut returns = Vec::with_capacity(cfg.returns.len());
        let mut return_tys = Vec::with_capacity(cfg.returns.len());
        let mut returns_expr = Vec::with_capacity(cfg.returns.len());
        for item in cfg.returns.iter() {
            let v = self.vartab.temp_anonymous(&item.ty);
            returns.push(v);
            return_tys.push(item.ty.clone());
            returns_expr.push(Expression::Variable {
                loc: Codegen,
                ty: item.ty.clone(),
                var_no: v,
            });
        }

        self.add(Instr::Call {
            res: returns,
            call: InternalCallTy::Static { cfg_no: func_no },
            args,
            return_tys,
        });

        if cfg.returns.is_empty() {
            let data_len = Expression::NumberLiteral {
                loc: Codegen,
                ty: Uint(32),
                value: 0.into(),
            };
            let data = Expression::AllocDynamicBytes {
                loc: Codegen,
                ty: Type::DynamicBytes,
                size: data_len.clone().into(),
                initializer: None,
            };
            self.add(Instr::ReturnData { data, data_len });
        } else {
            let (data, data_len) = abi_encode(
                &Codegen,
                returns_expr,
                self.ns,
                &mut self.vartab,
                &mut self.cfg,
                false,
            );
            self.add(Instr::ReturnData { data, data_len });
        }
        case_bb
    }

    fn add(&mut self, ins: Instr) {
        self.cfg.add(&mut self.vartab, ins);
    }
}
