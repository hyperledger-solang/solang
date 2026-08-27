// SPDX-License-Identifier: Apache-2.0

use crate::codegen::{
    cfg::{ASTFunction, ControlFlowGraph, Instr, InternalCallTy},
    encoding::{abi_decode, abi_encode},
    vartable::Vartable,
    Builtin, Expression, Options,
};
use crate::sema::ast::{Namespace, Type, Type::Uint};
use num_bigint::{BigInt, Sign};
use solang_parser::pt::{FunctionTy, Loc::Codegen};

pub(crate) const DISPATCH_CFG_NAME: &str = "_start";

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
    vartab: Vartable,
    cfg: ControlFlowGraph,
    all_cfg: &'a [ControlFlowGraph],
    ns: &'a mut Namespace,
    calldata_len_var: usize,
    payload_ptr_var: usize,
}

fn new_cfg() -> ControlFlowGraph {
    let mut cfg = ControlFlowGraph::new(DISPATCH_CFG_NAME.into(), ASTFunction::None);
    cfg.params = vec![].into();
    cfg
}

impl<'a> Dispatch<'a> {
    fn new(all_cfg: &'a [ControlFlowGraph], ns: &'a mut Namespace) -> Self {
        let mut vartab = Vartable::new(ns.next_id);
        let mut cfg = new_cfg();

        // read calldata length from 0x80000000 (8 bytes).
        let length_addr = Expression::NumberLiteral {
            loc: Codegen,
            ty: Uint(64),
            value: 0x80000000u64.into(),
        };
        let length_ptr = Expression::Cast {
            loc: Codegen,
            ty: Type::BufferPointer,
            expr: Box::new(length_addr),
        };
        let length_val = Expression::Load {
            loc: Codegen,
            ty: Uint(64),
            expr: Box::new(length_ptr),
        };

        let len_var = vartab.temp_name("calldata_len", &Uint(64));
        cfg.add(
            &mut vartab,
            Instr::Set {
                loc: Codegen,
                res: len_var,
                expr: length_val,
            },
        );

        // payload starts at 0x80000008.
        let payload_addr = Expression::NumberLiteral {
            loc: Codegen,
            ty: Uint(64),
            value: 0x80000008u64.into(),
        };
        let payload_ptr = Expression::Cast {
            loc: Codegen,
            ty: Type::BufferPointer,
            expr: Box::new(payload_addr),
        };
        let payload_ptr_var = vartab.temp_name("payload_ptr", &Type::BufferPointer);
        cfg.add(
            &mut vartab,
            Instr::Set {
                loc: Codegen,
                res: payload_ptr_var,
                expr: payload_ptr,
            },
        );

        Self {
            start: cfg.new_basic_block("start_dispatch".into()),
            vartab,
            cfg,
            all_cfg,
            ns,
            calldata_len_var: len_var,
            payload_ptr_var,
        }
    }

    fn build(mut self) -> ControlFlowGraph {
        // check if calldata length >= 4 bytes.
        let cond = Expression::Less {
            loc: Codegen,
            signed: false,
            left: Expression::Variable {
                loc: Codegen,
                ty: Uint(64),
                var_no: self.calldata_len_var,
            }
            .into(),
            right: Expression::NumberLiteral {
                loc: Codegen,
                ty: Uint(64),
                value: 4u64.into(),
            }
            .into(),
        };
        let invalid = self.cfg.new_basic_block("invalid_selector".into());
        self.add(Instr::BranchCond {
            cond,
            true_block: invalid,
            false_block: self.start,
        });

        // read selector (first 4 bytes of payload) via Builtin::ReadFromBuffer.
        let selector_ty = Uint(32);
        let selector_var = self.vartab.temp_name("selector", &selector_ty);
        self.cfg.set_basic_block(self.start);
        self.add(Instr::Set {
            loc: Codegen,
            res: selector_var,
            expr: Expression::Builtin {
                loc: Codegen,
                tys: vec![selector_ty.clone()],
                kind: Builtin::ReadFromBuffer,
                args: vec![
                    Expression::Variable {
                        loc: Codegen,
                        ty: Type::BufferPointer,
                        var_no: self.payload_ptr_var,
                    },
                    Expression::NumberLiteral {
                        loc: Codegen,
                        ty: selector_ty.clone(),
                        value: 0u64.into(),
                    },
                ],
            },
        });
        let selector = Expression::Variable {
            loc: Codegen,
            ty: selector_ty.clone(),
            var_no: selector_var,
        };

        // build switch cases.
        let cases = self
            .all_cfg
            .iter()
            .enumerate()
            .filter_map(|(func_no, func_cfg)| {
                if matches!(func_cfg.ty, FunctionTy::Function | FunctionTy::Constructor)
                    && func_cfg.public
                {
                    let selector_bytes = &func_cfg.selector;
                    let selector_val = BigInt::from_bytes_be(Sign::Plus, selector_bytes);
                    let case_expr = Expression::NumberLiteral {
                        loc: Codegen,
                        ty: selector_ty.clone(),
                        value: selector_val,
                    };
                    Some((case_expr, self.dispatch_case(func_no)))
                } else {
                    None
                }
            })
            .collect();

        self.cfg.set_basic_block(self.start);

        self.add(Instr::Switch {
            cond: selector,
            cases,
            default: invalid,
        });

        self.cfg.set_basic_block(invalid);
        // for now, just an assertfailure.
        self.add(Instr::AssertFailure { encoded_args: None });

        self.vartab.finalize(self.ns, &mut self.cfg);
        self.cfg
    }

    fn dispatch_case(&mut self, func_no: usize) -> usize {
        let case_bb = self
            .cfg
            .new_basic_block(format!("func_{}_dispatch", func_no));
        self.cfg.set_basic_block(case_bb);

        let cfg = &self.all_cfg[func_no];
        let mut args = vec![];
        if !cfg.params.is_empty() {
            // Prepare argument decoding.
            let len_var = self.calldata_len_var;
            let buf_len = Expression::Variable {
                loc: Codegen,
                ty: Uint(64),
                var_no: len_var,
            };
            let selector_len_expr = Expression::NumberLiteral {
                loc: Codegen,
                ty: Uint(64),
                value: 4u64.into(),
            };
            let arg_len = Expression::Subtract {
                loc: Codegen,
                ty: Uint(64),
                overflowing: false,
                left: buf_len.clone().into(),
                right: selector_len_expr.clone().into(),
            };
            let payload_ptr_var = self.payload_ptr_var;
            let payload_ptr_expr = Expression::Variable {
                loc: Codegen,
                ty: Type::BufferPointer,
                var_no: payload_ptr_var,
            };
            // advance pointer by 4 bytes.
            let arg_ptr = Expression::AdvancePointer {
                pointer: payload_ptr_expr.into(),
                bytes_offset: selector_len_expr.into(),
            };

            args = abi_decode(
                &Codegen,
                &arg_ptr,
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

        // prepare return variables.
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

        // encode and return.
        if cfg.returns.is_empty() {
            let data_len = Expression::NumberLiteral {
                loc: Codegen,
                ty: Uint(32),
                value: 0u64.into(),
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
