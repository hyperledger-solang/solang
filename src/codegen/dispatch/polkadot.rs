// SPDX-License-Identifier: Apache-2.0

use crate::{
    codegen::{
        cfg::{ASTFunction, ControlFlowGraph, Instr, InternalCallTy, ReturnCode},
        encoding::{abi_decode, abi_encode},
        revert::log_runtime_error,
        vartable::Vartable,
        Builtin, Expression, Options,
    },
    sema::ast::{Namespace, Parameter, Type, Type::Uint},
};
use num_bigint::{BigInt, Sign};
use solang_parser::pt::{FunctionTy, Loc::Codegen};
use std::fmt::{Display, Formatter, Result};

/// On Polkadot, contracts export  a `call` and a `deploy` function.
/// The `contracts` pallet will invoke `deploy` on contract instatiation,
/// and `call` on any contract calls after the instantiation.
///
/// On Ethereum, constructors do not exist on-chain; they are only executed once.
/// To cope with that model, we emit different code for the dispatcher,
/// depending on the exported function:
/// * On `deploy`, match only on constructor selectors
/// * On `call`, match only on selectors of externally callable functions
pub enum DispatchType {
    Deploy,
    Call,
}

impl Display for DispatchType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Deploy => f.write_str("polkadot_deploy_dispatch"),
            Self::Call => f.write_str("polkadot_call_dispatch"),
        }
    }
}

impl From<FunctionTy> for DispatchType {
    fn from(value: FunctionTy) -> Self {
        match value {
            FunctionTy::Constructor => Self::Deploy,
            FunctionTy::Function => Self::Call,
            _ => unreachable!("only constructors and functions have corresponding dispatch types"),
        }
    }
}

/// The dispatch algorithm consists of these steps:
/// 1. If the input is less than the expected selector length (default 4 bytes), fallback or receive.
/// 2. Match the function selector
///     - If no selector matches, fallback or receive.
///     - If the function is non-payable but the call features endowment, revert.
/// 3. ABI decode the arguments.
/// 4. Call the matching function.
/// 5. Return the result:
///     - On success, ABI encode the result (if any) and return.
///     - On failure, trap the contract.
pub(crate) fn function_dispatch(
    _contract_no: usize,
    all_cfg: &[ControlFlowGraph],
    ns: &mut Namespace,
    opt: &Options,
) -> Vec<ControlFlowGraph> {
    vec![
        Dispatch::new(all_cfg, ns, opt, FunctionTy::Constructor).build(),
        Dispatch::new(all_cfg, ns, opt, FunctionTy::Function).build(),
    ]
}

struct Dispatch<'a> {
    start: usize,
    input_len: usize,
    input_ptr: Expression,
    value: usize,
    vartab: Vartable,
    cfg: ControlFlowGraph,
    all_cfg: &'a [ControlFlowGraph],
    ns: &'a mut Namespace,
    selector_len: Box<Expression>,
    opt: &'a Options,
    ty: FunctionTy,
}

fn new_cfg(ns: &Namespace, ty: FunctionTy) -> ControlFlowGraph {
    let mut cfg = ControlFlowGraph::new(DispatchType::from(ty).to_string(), ASTFunction::None);
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
    let mut value = input_ptr.clone();
    value.ty = ns.value_type();
    let mut selector_ptr = input_ptr.clone();
    selector_ptr.ty = Type::Ref(Uint(8 * ns.target.selector_length() as u16).into());
    cfg.params = vec![input_ptr, input_len, value, selector_ptr].into();
    cfg
}

impl<'a> Dispatch<'a> {
    /// Create a new `Dispatch` struct that has all the data needed for building the dispatch logic.
    ///
    /// `ty` specifies whether to include constructors or functions.
    fn new(
        all_cfg: &'a [ControlFlowGraph],
        ns: &'a mut Namespace,
        opt: &'a Options,
        ty: FunctionTy,
    ) -> Self {
        let mut vartab = Vartable::new(ns.next_id);
        let mut cfg = new_cfg(ns, ty);

        // Read input length from args
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

        // Read transferred value from args
        let value = vartab.temp_name("value", &ns.value_type());
        cfg.add(
            &mut vartab,
            Instr::Set {
                loc: Codegen,
                res: value,
                expr: Expression::FunctionArg {
                    loc: Codegen,
                    ty: ns.value_type(),
                    arg_no: 2,
                },
            },
        );

        // Calculate input pointer offset
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
        let selector_len: Box<Expression> = Expression::NumberLiteral {
            loc: Codegen,
            ty: Uint(32),
            value: ns.target.selector_length().into(),
        }
        .into();
        let input_ptr = Expression::AdvancePointer {
            pointer: input_ptr.into(),
            bytes_offset: selector_len.clone(),
        };

        Self {
            start: cfg.new_basic_block("start_dispatch".into()),
            input_len,
            input_ptr,
            vartab,
            value,
            cfg,
            all_cfg,
            ns,
            selector_len,
            opt,
            ty,
        }
    }

    /// Build the dispatch logic into the returned control flow graph.
    fn build(mut self) -> ControlFlowGraph {
        // Go to fallback or receive if there is no selector in the call input
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
        let default = self.cfg.new_basic_block("fb_or_recv".into());
        self.add(Instr::BranchCond {
            cond,
            true_block: default,
            false_block: self.start,
        });

        // Build all cases
        let selector_ty = Uint(8 * self.ns.target.selector_length() as u16);
        let cases = self
            .all_cfg
            .iter()
            .enumerate()
            .filter_map(|(func_no, func_cfg)| {
                if func_cfg.ty == self.ty && func_cfg.public {
                    let selector = BigInt::from_bytes_le(Sign::Plus, &func_cfg.selector);
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

        // Read selector
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
            ty: selector_ty.clone(),
            var_no: selector_var,
        };
        self.add(Instr::Store {
            dest: Expression::FunctionArg {
                loc: Codegen,
                ty: selector_ty.clone(),
                arg_no: 3,
            },
            data: selector.clone(),
        });
        self.add(Instr::Switch {
            cond: selector,
            cases,
            default,
        });

        // Handle fallback or receive case
        self.cfg.set_basic_block(default);
        self.fallback_or_receive();

        self.vartab.finalize(self.ns, &mut self.cfg);
        self.cfg
    }

    /// Insert the dispatch logic for `func_no`. `func_no` may be a function or constructor.
    /// Returns the basic block number in which the dispatch logic was inserted.
    fn dispatch_case(&mut self, func_no: usize) -> usize {
        let case_bb = self.cfg.new_basic_block(format!("func_{func_no}_dispatch"));
        self.cfg.set_basic_block(case_bb);
        self.abort_if_value_transfer(func_no);

        // Decode input data if necessary
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

        let mut returns: Vec<usize> = Vec::with_capacity(cfg.returns.len());
        let mut return_tys: Vec<Type> = Vec::with_capacity(cfg.returns.len());
        let mut returns_expr: Vec<Expression> = Vec::with_capacity(cfg.returns.len());
        for item in cfg.returns.iter() {
            let new_var = self.vartab.temp_anonymous(&item.ty);
            returns.push(new_var);
            return_tys.push(item.ty.clone());
            returns_expr.push(Expression::Variable {
                loc: Codegen,
                ty: item.ty.clone(),
                var_no: new_var,
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
            self.add(Instr::ReturnData { data, data_len })
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

    /// Insert a trap into the cfg, if the function `func_no` is not payable but received value anyways.
    fn abort_if_value_transfer(&mut self, func_no: usize) {
        if !self.all_cfg[func_no].nonpayable {
            return;
        }

        let true_block = self
            .cfg
            .new_basic_block(format!("func_{func_no}_got_value"));
        let false_block = self.cfg.new_basic_block(format!("func_{func_no}_no_value"));
        self.add(Instr::BranchCond {
            cond: Expression::More {
                loc: Codegen,
                signed: false,
                left: Expression::Variable {
                    loc: Codegen,
                    ty: self.ns.value_type(),
                    var_no: self.value,
                }
                .into(),
                right: Expression::NumberLiteral {
                    loc: Codegen,
                    ty: self.ns.value_type(),
                    value: 0.into(),
                }
                .into(),
            },
            true_block,
            false_block,
        });

        self.cfg.set_basic_block(true_block);
        let function_name = self.all_cfg[func_no].name.split("::").last().unwrap();
        let function_type = self.all_cfg[func_no].ty;
        log_runtime_error(
            self.opt.log_runtime_errors,
            &format!("runtime_error: non payable {function_type} {function_name} received value"),
            Codegen,
            &mut self.cfg,
            &mut self.vartab,
            self.ns,
        );
        self.add(Instr::AssertFailure { encoded_args: None });

        self.cfg.set_basic_block(false_block);
    }

    /// Build calls to fallback or receive functions (if they are present in the contract).
    fn fallback_or_receive(&mut self) {
        let (fallback_cfg, receive_cfg) = self.all_cfg.iter().enumerate().fold(
            (None, None),
            |(mut fallback_cfg, mut receive_cfg), (no, cfg)| {
                match cfg.ty {
                    FunctionTy::Fallback if cfg.public => fallback_cfg = Some(no),
                    FunctionTy::Receive if cfg.public => receive_cfg = Some(no),
                    _ => {}
                }
                (fallback_cfg, receive_cfg)
            },
        );

        // No need to check value transferred; we will abort either way
        if (fallback_cfg.is_none() && receive_cfg.is_none()) || self.ty == FunctionTy::Constructor {
            return self.selector_invalid();
        }

        let fallback_block = self.cfg.new_basic_block("fallback".into());
        let receive_block = self.cfg.new_basic_block("receive".into());
        self.add(Instr::BranchCond {
            cond: Expression::More {
                loc: Codegen,
                signed: false,
                left: Expression::Variable {
                    loc: Codegen,
                    ty: self.ns.value_type(),
                    var_no: self.value,
                }
                .into(),
                right: Expression::NumberLiteral {
                    loc: Codegen,
                    ty: self.ns.value_type(),
                    value: 0.into(),
                }
                .into(),
            },
            true_block: receive_block,
            false_block: fallback_block,
        });

        self.cfg.set_basic_block(fallback_block);
        if let Some(cfg_no) = fallback_cfg {
            self.add(Instr::Call {
                res: vec![],
                return_tys: vec![],
                call: InternalCallTy::Static { cfg_no },
                args: vec![],
            });
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
            self.add(Instr::ReturnData { data, data_len })
        } else {
            self.selector_invalid();
        }

        self.cfg.set_basic_block(receive_block);
        if let Some(cfg_no) = receive_cfg {
            self.add(Instr::Call {
                res: vec![],
                return_tys: vec![],
                call: InternalCallTy::Static { cfg_no },
                args: vec![],
            });
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
            self.add(Instr::ReturnData { data, data_len })
        } else {
            self.selector_invalid()
        }
    }

    fn selector_invalid(&mut self) {
        let code = ReturnCode::FunctionSelectorInvalid;
        self.add(Instr::ReturnCode { code });
    }

    fn add(&mut self, ins: Instr) {
        self.cfg.add(&mut self.vartab, ins);
    }
}
