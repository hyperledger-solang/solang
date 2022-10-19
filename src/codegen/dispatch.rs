// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ASTFunction, ControlFlowGraph, Instr, InternalCallTy, ReturnCode};
use crate::codegen::vartable::Vartable;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{ArrayLength, Namespace, Parameter, RetrieveType, StructType, Type};
use num_bigint::{BigInt, Sign};
use num_traits::{ToPrimitive, Zero};
use solang_parser::pt;
use solang_parser::pt::{FunctionTy, Loc};
use std::sync::Arc;

/// Create the dispatch for the Solana target
pub(super) fn function_dispatch(
    contract_no: usize,
    all_cfg: &[ControlFlowGraph],
    ns: &mut Namespace,
) -> ControlFlowGraph {
    let mut vartab = Vartable::new(ns.next_id);
    let mut cfg = ControlFlowGraph::new(
        format!("dispatch_{}", ns.contracts[contract_no].name),
        ASTFunction::None,
    );

    cfg.params = Arc::new(vec![
        Parameter {
            loc: Loc::Codegen,
            id: None,
            ty: Type::BufferPointer,
            ty_loc: None,
            indexed: false,
            readonly: false,
            recursive: false,
        },
        Parameter {
            loc: Loc::Codegen,
            id: None,
            ty: Type::Uint(64),
            ty_loc: None,
            indexed: false,
            readonly: false,
            recursive: false,
        },
    ]);

    let switch_block = cfg.new_basic_block("switch".to_string());
    let no_function_matched = cfg.new_basic_block("no_function_matched".to_string());

    let not_fallback = Expression::MoreEqual(
        Loc::Codegen,
        Box::new(Expression::FunctionArg(Loc::Codegen, Type::Uint(64), 1)),
        Box::new(Expression::NumberLiteral(
            Loc::Codegen,
            Type::Uint(64),
            BigInt::from(4u8),
        )),
    );

    cfg.add(
        &mut vartab,
        Instr::BranchCond {
            cond: not_fallback,
            true_block: switch_block,
            false_block: no_function_matched,
        },
    );
    cfg.set_basic_block(switch_block);
    let argsdata = Expression::FunctionArg(Loc::Codegen, Type::BufferPointer, 0);
    let argslen = Expression::FunctionArg(Loc::Codegen, Type::Uint(64), 1);
    let fid = Expression::Builtin(
        Loc::Codegen,
        vec![Type::Uint(32)],
        Builtin::ReadFromBuffer,
        vec![
            argsdata.clone(),
            Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), BigInt::zero()),
        ],
    );

    let argsdata = Expression::AdvancePointer {
        pointer: Box::new(argsdata),
        bytes_offset: Box::new(Expression::NumberLiteral(
            Loc::Codegen,
            Type::Uint(32),
            BigInt::from(4u8),
        )),
    };
    let argslen = Expression::Subtract(
        Loc::Codegen,
        Type::Uint(64),
        false,
        Box::new(argslen),
        Box::new(Expression::NumberLiteral(
            Loc::Codegen,
            Type::Uint(64),
            BigInt::from(4u8),
        )),
    );

    let mut cases = Vec::new();
    for (cfg_no, func_cfg) in all_cfg.iter().enumerate() {
        if func_cfg.ty != pt::FunctionTy::Function || !func_cfg.public {
            continue;
        }

        add_dispatch_case(
            cfg_no,
            func_cfg,
            &argsdata,
            argslen.clone(),
            &mut cases,
            &mut vartab,
            &mut cfg,
        );
    }

    cfg.set_basic_block(switch_block);

    cfg.add(
        &mut vartab,
        Instr::Switch {
            cond: fid,
            cases,
            default: no_function_matched,
        },
    );

    cfg.set_basic_block(no_function_matched);

    let fallback = all_cfg
        .iter()
        .enumerate()
        .find(|(_, cfg)| cfg.public && cfg.ty == pt::FunctionTy::Fallback);

    let receive = all_cfg
        .iter()
        .enumerate()
        .find(|(_, cfg)| cfg.public && cfg.ty == pt::FunctionTy::Receive);

    if fallback.is_none() && receive.is_none() {
        cfg.add(
            &mut vartab,
            Instr::ReturnCode {
                code: ReturnCode::FunctionSelectorInvalid,
            },
        );

        vartab.finalize(ns, &mut cfg);

        return cfg;
    }

    match fallback {
        Some((cfg_no, _)) => {
            cfg.add(
                &mut vartab,
                Instr::Call {
                    res: vec![],
                    return_tys: vec![],
                    args: vec![],
                    call: InternalCallTy::Static { cfg_no },
                },
            );

            cfg.add(
                &mut vartab,
                Instr::ReturnCode {
                    code: ReturnCode::Success,
                },
            );
        }
        None => {
            cfg.add(
                &mut vartab,
                Instr::ReturnCode {
                    code: ReturnCode::InvalidDataError,
                },
            );
        }
    }

    vartab.finalize(ns, &mut cfg);

    cfg
}

/// Add the dispatch for function given a matched selector
fn add_dispatch_case(
    cfg_no: usize,
    func_cfg: &ControlFlowGraph,
    argsdata: &Expression,
    argslen: Expression,
    cases: &mut Vec<(Expression, usize)>,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) {
    let bb = cfg.new_basic_block(format!("function_cfg_{}", cfg_no));
    cfg.set_basic_block(bb);

    let truncated_len = Expression::Trunc(Loc::Codegen, Type::Uint(32), Box::new(argslen));

    let mut vars: Vec<usize> = Vec::with_capacity(func_cfg.params.len());
    let mut decoded: Vec<Expression> = Vec::with_capacity(func_cfg.params.len());
    for item in func_cfg.params.iter() {
        let new_var = vartab.temp_anonymous(&item.ty);
        vars.push(new_var);
        decoded.push(Expression::Variable(Loc::Codegen, item.ty.clone(), new_var));
    }
    cfg.add(
        vartab,
        Instr::AbiDecode {
            res: vars,
            selector: None,
            exception_block: None,
            tys: (*func_cfg.params).clone(),
            data: argsdata.clone(),
            data_len: Some(truncated_len),
        },
    );

    let mut returns: Vec<usize> = Vec::with_capacity(func_cfg.returns.len());
    let mut return_tys: Vec<Type> = Vec::with_capacity(func_cfg.returns.len());
    let mut returns_expr: Vec<Expression> = Vec::with_capacity(func_cfg.returns.len());
    for item in func_cfg.returns.iter() {
        let new_var = vartab.temp_anonymous(&item.ty);
        returns.push(new_var);
        return_tys.push(item.ty.clone());
        returns_expr.push(Expression::Variable(Loc::Codegen, item.ty.clone(), new_var));
    }

    cfg.add(
        vartab,
        Instr::Call {
            res: returns,
            call: InternalCallTy::Static { cfg_no },
            args: decoded,
            return_tys,
        },
    );

    if !func_cfg.returns.is_empty() {
        let data = Expression::AbiEncode {
            loc: Loc::Codegen,
            tys: returns_expr.iter().map(|e| e.ty()).collect::<Vec<Type>>(),
            packed: vec![],
            args: returns_expr,
        };
        let data_len = Expression::Builtin(
            Loc::Codegen,
            vec![Type::Uint(32)],
            Builtin::ArrayLength,
            vec![data.clone()],
        );
        let zext_len = Expression::ZeroExt(Loc::Codegen, Type::Uint(64), Box::new(data_len));
        cfg.add(
            vartab,
            Instr::ReturnData {
                data,
                data_len: zext_len,
            },
        );
    }

    cfg.add(vartab, Instr::Return { value: vec![] });

    cases.push((
        Expression::NumberLiteral(
            Loc::Codegen,
            Type::Uint(32),
            BigInt::from_bytes_le(Sign::Plus, &*func_cfg.selector),
        ),
        bb,
    ));
}

/// Create the dispatch for a contract constructor. This case needs creates a new function in
/// the CFG because we want to use de abi decoding implementation from codegen.
pub(super) fn constructor_dispatch(
    contract_no: usize,
    constructor_cfg_no: usize,
    all_cfg: &[ControlFlowGraph],
    ns: &mut Namespace,
) -> ControlFlowGraph {
    let mut vartab = Vartable::new(ns.next_id);
    let mut func_name = format!("constructor_dispatch_{}", all_cfg[constructor_cfg_no].name);
    for params in all_cfg[constructor_cfg_no].params.iter() {
        func_name.push_str(format!("_{}", params.ty.to_string(ns)).as_str());
    }
    let mut cfg = ControlFlowGraph::new(func_name, ASTFunction::None);
    cfg.ty = FunctionTy::Function;
    cfg.public = all_cfg[constructor_cfg_no].public;

    cfg.params = Arc::new(vec![
        Parameter {
            loc: Loc::Codegen,
            id: None,
            ty: Type::BufferPointer,
            ty_loc: None,
            indexed: false,
            readonly: false,
            recursive: false,
        },
        Parameter {
            loc: Loc::Codegen,
            id: None,
            ty: Type::Uint(64),
            ty_loc: None,
            indexed: false,
            readonly: false,
            recursive: false,
        },
    ]);

    let mut res: Vec<usize> = Vec::with_capacity(all_cfg[constructor_cfg_no].params.len());
    let mut returns: Vec<Expression> = Vec::with_capacity(all_cfg[constructor_cfg_no].params.len());
    for item in all_cfg[constructor_cfg_no].params.iter() {
        let new_var = vartab.temp_anonymous(&item.ty);
        res.push(new_var);
        returns.push(Expression::Variable(Loc::Codegen, item.ty.clone(), new_var));
    }

    let data = Expression::FunctionArg(Loc::Codegen, Type::BufferPointer, 0);
    let data_len = Expression::FunctionArg(Loc::Codegen, Type::Uint(64), 1);

    if !res.is_empty() {
        cfg.add(
            &mut vartab,
            Instr::AbiDecode {
                res,
                selector: None,
                exception_block: None,
                tys: (*all_cfg[constructor_cfg_no].params).clone(),
                data,
                data_len: Some(data_len),
            },
        );
    }

    // Make sure that the data account is large enough
    let account_length = Expression::Builtin(
        Loc::Codegen,
        vec![Type::Uint(32)],
        Builtin::ArrayLength,
        vec![Expression::StructMember(
            Loc::Codegen,
            Type::DynamicBytes,
            Box::new(Expression::Subscript(
                Loc::Codegen,
                Type::Struct(StructType::AccountInfo),
                Type::Array(
                    Box::new(Type::Struct(StructType::AccountInfo)),
                    vec![ArrayLength::Dynamic],
                ),
                Box::new(Expression::Builtin(
                    Loc::Codegen,
                    vec![Type::Array(
                        Box::new(Type::Struct(StructType::AccountInfo)),
                        vec![ArrayLength::Dynamic],
                    )],
                    Builtin::Accounts,
                    vec![],
                )),
                Box::new(Expression::NumberLiteral(
                    Loc::Codegen,
                    Type::Uint(32),
                    BigInt::zero(),
                )),
            )),
            2,
        )],
    );

    let is_enough = Expression::MoreEqual(
        Loc::Codegen,
        Box::new(account_length),
        Box::new(Expression::NumberLiteral(
            Loc::Codegen,
            Type::Uint(32),
            ns.contracts[contract_no].fixed_layout_size.clone(),
        )),
    );

    let enough = cfg.new_basic_block("enough".into());
    let not_enough = cfg.new_basic_block("not_enough".into());

    cfg.add(
        &mut vartab,
        Instr::BranchCond {
            cond: is_enough,
            true_block: enough,
            false_block: not_enough,
        },
    );

    cfg.set_basic_block(not_enough);

    cfg.add(
        &mut vartab,
        Instr::ReturnCode {
            code: ReturnCode::AccountDataToSmall,
        },
    );

    cfg.set_basic_block(enough);

    // Write contract magic to offset 0
    cfg.add(
        &mut vartab,
        Instr::SetStorage {
            ty: Type::Uint(32),
            value: Expression::NumberLiteral(
                pt::Loc::Codegen,
                Type::Uint(64),
                BigInt::from(ns.contracts[contract_no].selector()),
            ),
            storage: Expression::NumberLiteral(pt::Loc::Codegen, Type::Uint(64), BigInt::zero()),
        },
    );

    // Calculate heap offset
    let fixed_fields_size = ns.contracts[contract_no]
        .fixed_layout_size
        .to_u64()
        .unwrap();

    let heap_offset = (fixed_fields_size + 7) & !7;

    // Write heap offset to 12
    cfg.add(
        &mut vartab,
        Instr::SetStorage {
            ty: Type::Uint(32),
            value: Expression::NumberLiteral(
                pt::Loc::Codegen,
                Type::Uint(64),
                BigInt::from(heap_offset),
            ),
            storage: Expression::NumberLiteral(pt::Loc::Codegen, Type::Uint(64), BigInt::from(12)),
        },
    );

    // Call storage initializer
    cfg.add(
        &mut vartab,
        Instr::Call {
            res: vec![],
            return_tys: vec![],
            call: InternalCallTy::Static {
                cfg_no: ns.contracts[contract_no].initializer.unwrap(),
            },
            args: vec![],
        },
    );

    cfg.add(
        &mut vartab,
        Instr::Call {
            res: vec![],
            return_tys: vec![],
            call: InternalCallTy::Static {
                cfg_no: constructor_cfg_no,
            },
            args: returns,
        },
    );

    cfg.add(
        &mut vartab,
        Instr::ReturnCode {
            code: ReturnCode::Success,
        },
    );

    vartab.finalize(ns, &mut cfg);

    cfg
}
