// SPDX-License-Identifier: Apache-2.0

use crate::{
    codegen::{
        cfg::{ASTFunction, ControlFlowGraph, Instr, InternalCallTy, ReturnCode},
        encoding::{abi_decode, abi_encode},
        expression::log_runtime_error,
        vartable::Vartable,
        Builtin, Expression, Options,
    },
    sema::ast::{Namespace, Parameter, Type, Type::Uint},
};
use num_bigint::{BigInt, Sign};
use solang_parser::pt::{FunctionTy, Loc::Codegen};

/// The dispatching algorithm consists of these steps:
/// 1. If the input is less than the expected selector length (default 4 bytes), fallback or receive.
/// 2. Match the function selector
///     - If no selector matches, fallback or receive.
///     - If the function is non-payable but the call features endowment, revert.
/// 3. ABI decode the arguments.
/// 4. Call the matching function.
/// 5. Return the result:
///     - On success, ABI encode the result (if any) and return.
///     - On failure, trap the contract.
///
/// We distinguish between fallback and receive:
/// - If there is no endowment, dispatch to fallback
/// - If there is endowment, dispatch to receive
pub(crate) fn function_dispatch(
    _contract_no: usize,
    all_cfg: &[ControlFlowGraph],
    ns: &mut Namespace,
    opt: &Options,
) -> ControlFlowGraph {
    Dispatch::new(all_cfg, ns, opt).build()
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
}

fn new_cfg(ns: &Namespace) -> ControlFlowGraph {
    let mut cfg = ControlFlowGraph::new("substrate_dispatch".into(), ASTFunction::None);
    let input_ptr = Parameter {
        loc: Codegen,
        id: None,
        ty: Type::BufferPointer,
        ty_loc: None,
        indexed: false,
        readonly: true,
        infinite_size: false,
        recursive: false,
    };
    let mut input_len = input_ptr.clone();
    input_len.ty = Uint(32);
    let mut value = input_ptr.clone();
    value.ty = Uint(8 * ns.value_length as u16);
    let mut selector_ptr = input_ptr.clone();
    selector_ptr.ty = Type::Ref(Uint(8 * ns.target.selector_length() as u16).into());
    cfg.params = vec![input_ptr, input_len, value, selector_ptr].into();
    cfg
}

impl<'a> Dispatch<'a> {
    /// Create a new `Dispatch` struct that has all the data needed for building the dispatch logic.
    fn new(all_cfg: &'a [ControlFlowGraph], ns: &'a mut Namespace, opt: &'a Options) -> Self {
        let mut vartab = Vartable::new(ns.next_id);
        let mut cfg = new_cfg(ns);

        // Read input length from args
        let input_len = vartab.temp_name("input_len", &Uint(32));
        cfg.add(
            &mut vartab,
            Instr::Set {
                loc: Codegen,
                res: input_len,
                expr: Expression::FunctionArg(Codegen, Uint(32), 1),
            },
        );

        // Read transferred value from args
        let value_ty = Uint(8 * ns.value_length as u16);
        let value = vartab.temp_name("value", &value_ty);
        cfg.add(
            &mut vartab,
            Instr::Set {
                loc: Codegen,
                res: value,
                expr: Expression::FunctionArg(Codegen, value_ty, 2),
            },
        );

        // Calculate input pointer offset
        let input_ptr_var = vartab.temp_name("input_ptr", &Type::BufferPointer);
        cfg.add(
            &mut vartab,
            Instr::Set {
                loc: Codegen,
                res: input_ptr_var,
                expr: Expression::FunctionArg(Codegen, Type::BufferPointer, 0),
            },
        );
        let input_ptr = Expression::Variable(Codegen, Type::BufferPointer, input_ptr_var);
        let selector_len: Box<Expression> =
            Expression::NumberLiteral(Codegen, Uint(32), ns.target.selector_length().into()).into();
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
        }
    }

    /// Build the dispatch logic into the returned control flow graph.
    fn build(mut self) -> ControlFlowGraph {
        // Go to fallback or receive if there is no selector in the call input
        let cond = Expression::Less {
            loc: Codegen,
            signed: false,
            left: Expression::Variable(Codegen, Uint(32), self.input_len).into(),
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
            .filter_map(|(msg_no, msg_cfg)| match msg_cfg.ty {
                FunctionTy::Function | FunctionTy::Constructor if msg_cfg.public => {
                    let selector = BigInt::from_bytes_le(Sign::Plus, &msg_cfg.selector);
                    let case = Expression::NumberLiteral(Codegen, selector_ty.clone(), selector);
                    Some((case, self.dispatch_case(msg_no)))
                }
                _ => None,
            })
            .collect();

        // Read selector
        self.cfg.set_basic_block(self.start);
        let selector_var = self.vartab.temp_name("selector", &selector_ty);
        self.add(Instr::Set {
            loc: Codegen,
            res: selector_var,
            expr: Expression::Builtin(
                Codegen,
                vec![selector_ty.clone()],
                Builtin::ReadFromBuffer,
                vec![
                    Expression::FunctionArg(Codegen, Type::BufferPointer, 0),
                    Expression::NumberLiteral(Codegen, selector_ty.clone(), 0.into()),
                ],
            ),
        });
        let selector = Expression::Variable(Codegen, selector_ty.clone(), selector_var);
        self.add(Instr::Store {
            dest: Expression::FunctionArg(Codegen, selector_ty.clone(), 3),
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

    /// Insert the dispatch logic for `msg_no`. `msg_no` may be a message or constructor.
    /// Returns the basic block number where the dispatch logic was inserted.
    fn dispatch_case(&mut self, msg_no: usize) -> usize {
        let case_bb = self.cfg.new_basic_block(format!("msg_{msg_no}_dispatch"));
        self.cfg.set_basic_block(case_bb);
        self.abort_if_value_transfer(msg_no);

        // Decode input data if necessary
        let cfg = &self.all_cfg[msg_no];
        let mut args = vec![];
        if !cfg.params.is_empty() {
            let buf_len = Expression::Variable(Codegen, Uint(32), self.input_len);
            let arg_len = Expression::Subtract(
                Codegen,
                Uint(32),
                false,
                buf_len.into(),
                self.selector_len.clone(),
            );
            args = abi_decode(
                &Codegen,
                &self.input_ptr,
                &cfg.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
                self.ns,
                &mut self.vartab,
                &mut self.cfg,
                Some(Expression::Trunc(Codegen, Uint(32), arg_len.into())),
            );
        }

        let mut returns: Vec<usize> = Vec::with_capacity(cfg.returns.len());
        let mut return_tys: Vec<Type> = Vec::with_capacity(cfg.returns.len());
        let mut returns_expr: Vec<Expression> = Vec::with_capacity(cfg.returns.len());
        for item in cfg.returns.iter() {
            let new_var = self.vartab.temp_anonymous(&item.ty);
            returns.push(new_var);
            return_tys.push(item.ty.clone());
            returns_expr.push(Expression::Variable(Codegen, item.ty.clone(), new_var));
        }

        self.add(Instr::Call {
            res: returns,
            call: InternalCallTy::Static { cfg_no: msg_no },
            args,
            return_tys,
        });

        if cfg.returns.is_empty() {
            let data_len = Expression::NumberLiteral(Codegen, Uint(32), 0.into());
            let data = Expression::AllocDynamicBytes(
                Codegen,
                Type::DynamicBytes,
                data_len.clone().into(),
                None,
            );
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

    /// Insert a trap into the cfg, if the message `msg_no` is not payable but received value anyways.
    /// Constructors always receive endowment.
    fn abort_if_value_transfer(&mut self, msg_no: usize) {
        if !self.all_cfg[msg_no].nonpayable || self.all_cfg[msg_no].ty == FunctionTy::Constructor {
            return;
        }

        let value_ty = Uint(self.ns.value_length as u16 * 8);
        let true_block = self.cfg.new_basic_block(format!("msg_{msg_no}_has_value"));
        let false_block = self.cfg.new_basic_block(format!("msg_{msg_no}_no_value"));
        self.add(Instr::BranchCond {
            cond: Expression::More {
                loc: Codegen,
                signed: false,
                left: Expression::Variable(Codegen, value_ty.clone(), self.value).into(),
                right: Expression::NumberLiteral(Codegen, value_ty, 0.into()).into(),
            },
            true_block,
            false_block,
        });

        self.cfg.set_basic_block(true_block);
        let function_name = self.all_cfg[msg_no].name.split("::").last().unwrap();
        log_runtime_error(
            self.opt.log_runtime_errors,
            &format!("runtime_error: non payable function {function_name} received value"),
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
        let fb_recv = self
            .all_cfg
            .iter()
            .enumerate()
            .fold([None, None], |mut acc, (no, cfg)| {
                match cfg.ty {
                    FunctionTy::Fallback if cfg.public => acc[0] = Some(no),
                    FunctionTy::Receive if cfg.public => acc[1] = Some(no),
                    _ => {}
                }
                acc
            });

        // No need to check value transferred; we will abort either way
        if fb_recv[0].is_none() && fb_recv[1].is_none() {
            return self.selector_invalid();
        }

        let value_ty = Uint(self.ns.value_length as u16 * 8);
        let fallback_block = self.cfg.new_basic_block("fallback".into());
        let receive_block = self.cfg.new_basic_block("receive".into());
        self.add(Instr::BranchCond {
            cond: Expression::More {
                loc: Codegen,
                signed: false,
                left: Expression::Variable(Codegen, value_ty.clone(), self.value).into(),
                right: Expression::NumberLiteral(Codegen, value_ty, 0.into()).into(),
            },
            true_block: receive_block,
            false_block: fallback_block,
        });

        self.cfg.set_basic_block(fallback_block);
        if let Some(cfg_no) = fb_recv[0] {
            self.add(Instr::Call {
                res: vec![],
                return_tys: vec![],
                call: InternalCallTy::Static { cfg_no },
                args: vec![],
            });
            let data_len = Expression::NumberLiteral(Codegen, Uint(32), 0.into());
            let data = Expression::AllocDynamicBytes(
                Codegen,
                Type::DynamicBytes,
                data_len.clone().into(),
                None,
            );
            self.add(Instr::ReturnData { data, data_len })
        } else {
            self.selector_invalid();
        }

        self.cfg.set_basic_block(receive_block);
        if let Some(cfg_no) = fb_recv[1] {
            self.add(Instr::Call {
                res: vec![],
                return_tys: vec![],
                call: InternalCallTy::Static { cfg_no },
                args: vec![],
            });
            let data_len = Expression::NumberLiteral(Codegen, Uint(32), 0.into());
            let data = Expression::AllocDynamicBytes(
                Codegen,
                Type::DynamicBytes,
                data_len.clone().into(),
                None,
            );
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
