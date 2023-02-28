// SPDX-License-Identifier: Apache-2.0

use crate::codegen::encoding::create_encoder;
use crate::codegen::{
    cfg::{ASTFunction, ControlFlowGraph, Instr, InternalCallTy, ReturnCode},
    solana_deploy::solana_deploy,
    vartable::Vartable,
    Builtin, Expression, Options,
};
use crate::{
    sema::ast::{Namespace, StructType, Type},
    Target,
};
use num_bigint::{BigInt, Sign};
use num_traits::Zero;
use solang_parser::{pt, pt::Loc};

use super::encoding::abi_encode;

/// Create the dispatch for the Solana target
pub(super) fn function_dispatch(
    contract_no: usize,
    all_cfg: &[ControlFlowGraph],
    ns: &mut Namespace,
    opt: &Options,
) -> ControlFlowGraph {
    let mut vartab = Vartable::new(ns.next_id);
    let mut cfg = ControlFlowGraph::new("solang_dispatch".into(), ASTFunction::None);

    let switch_block = cfg.new_basic_block("switch".to_string());
    let no_function_matched = cfg.new_basic_block("no_function_matched".to_string());

    let argsdata_var = vartab.temp_name("input", &Type::BufferPointer);
    let argslen_var = vartab.temp_name("input_len", &Type::Uint(64));

    let sol_params =
        Expression::FunctionArg(Loc::Codegen, Type::Struct(StructType::SolParameters), 0);

    // ty:bufferptr argsdata_var = load ty:ref(ty:bufferptr) (structmember ty:ref(ty:bufferptr) (funcarg ty:struct(solparam), 2))
    cfg.add(
        &mut vartab,
        Instr::Set {
            res: argsdata_var,
            loc: Loc::Codegen,
            expr: Expression::Load(
                Loc::Codegen,
                Type::BufferPointer,
                Expression::StructMember(
                    Loc::Codegen,
                    Type::Ref(Type::BufferPointer.into()),
                    sol_params.clone().into(),
                    2,
                )
                .into(),
            ),
        },
    );

    let argsdata = Expression::Variable(Loc::Codegen, Type::BufferPointer, argsdata_var);

    // ty:uint64 argslen_var = load ref(ty:uint64) (structmember ref(ty:uin64) (funcarg ty:struct(solparam), 3))
    cfg.add(
        &mut vartab,
        Instr::Set {
            res: argslen_var,
            loc: Loc::Codegen,
            expr: Expression::Load(
                Loc::Codegen,
                Type::Uint(64),
                Expression::StructMember(
                    Loc::Codegen,
                    Type::Ref(Type::Uint(64).into()),
                    sol_params.into(),
                    3,
                )
                .into(),
            ),
        },
    );

    let argslen = Expression::Variable(Loc::Codegen, Type::Uint(64), argslen_var);

    let not_fallback = Expression::MoreEqual(
        Loc::Codegen,
        argslen.clone().into(),
        Expression::NumberLiteral(Loc::Codegen, Type::Uint(64), BigInt::from(8u8)).into(),
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

    let fid = Expression::Builtin(
        Loc::Codegen,
        vec![Type::Uint(64)],
        Builtin::ReadFromBuffer,
        vec![
            argsdata.clone(),
            Expression::NumberLiteral(Loc::Codegen, Type::Uint(64), BigInt::zero()),
        ],
    );

    let argsdata = Expression::AdvancePointer {
        pointer: Box::new(argsdata),
        bytes_offset: Box::new(Expression::NumberLiteral(
            Loc::Codegen,
            Type::Uint(32),
            BigInt::from(8u8),
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
            BigInt::from(8u8),
        )),
    );

    let magic = vartab.temp_name("magic", &Type::Uint(32));

    cfg.add(
        &mut vartab,
        Instr::LoadStorage {
            res: magic,
            ty: Type::Uint(32),
            storage: Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), 0.into()),
        },
    );

    let mut cases = Vec::new();

    for (cfg_no, func_cfg) in all_cfg.iter().enumerate() {
        if !func_cfg.public {
            continue;
        }

        let entry = if func_cfg.ty == pt::FunctionTy::Function {
            add_function_dispatch_case(
                cfg_no,
                func_cfg,
                magic,
                &argsdata,
                argslen.clone(),
                contract_no,
                ns,
                &mut vartab,
                &mut cfg,
            )
        } else if func_cfg.ty == pt::FunctionTy::Constructor {
            add_constructor_dispatch_case(
                contract_no,
                cfg_no,
                &argsdata,
                argslen.clone(),
                func_cfg,
                ns,
                &mut vartab,
                &mut cfg,
                opt,
            )
        } else {
            continue;
        };

        cases.push((
            Expression::NumberLiteral(
                Loc::Codegen,
                Type::Uint(64),
                BigInt::from_bytes_le(Sign::Plus, &func_cfg.selector),
            ),
            entry,
        ));
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
fn add_function_dispatch_case(
    cfg_no: usize,
    func_cfg: &ControlFlowGraph,
    magic: usize,
    argsdata: &Expression,
    argslen: Expression,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
) -> usize {
    let entry = cfg.new_basic_block(format!("function_cfg_{cfg_no}"));
    cfg.set_basic_block(entry);

    let needs_account = if let ASTFunction::SolidityFunction(func_no) = func_cfg.function_no {
        !ns.functions[func_no].is_pure()
    } else {
        true
    };

    if needs_account {
        // check for magic in data account, to see if data account is initialized
        let magic_ok = cfg.new_basic_block("magic_ok".into());
        let magic_bad = cfg.new_basic_block("magic_bad".into());

        cfg.add(
            vartab,
            Instr::BranchCond {
                cond: Expression::Equal(
                    Loc::Codegen,
                    Expression::Variable(Loc::Codegen, Type::Uint(32), magic).into(),
                    Expression::NumberLiteral(
                        Loc::Codegen,
                        Type::Uint(32),
                        ns.contracts[contract_no].selector().into(),
                    )
                    .into(),
                ),
                true_block: magic_ok,
                false_block: magic_bad,
            },
        );

        cfg.set_basic_block(magic_bad);

        cfg.add(
            vartab,
            Instr::ReturnCode {
                code: ReturnCode::InvalidDataError,
            },
        );

        cfg.set_basic_block(magic_ok);
    }

    let truncated_len = Expression::Trunc(Loc::Codegen, Type::Uint(32), Box::new(argslen));

    let tys = func_cfg
        .params
        .iter()
        .map(|e| e.ty.clone())
        .collect::<Vec<Type>>();
    let encoder = create_encoder(ns, false);
    let decoded = encoder.abi_decode(
        &Loc::Codegen,
        argsdata,
        &tys,
        ns,
        vartab,
        cfg,
        Some(truncated_len),
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
        let (data, data_len) = abi_encode(&Loc::Codegen, returns_expr, ns, vartab, cfg, false);
        let zext_len = Expression::ZeroExt(Loc::Codegen, Type::Uint(64), Box::new(data_len));
        cfg.add(
            vartab,
            Instr::ReturnData {
                data,
                data_len: zext_len,
            },
        );
    } else {
        // TODO: On Solana, we could elide setting the return data if this function calls no external functions
        // and replace this with a simple Instr::Return, which does not set any return data.
        //
        // The return data buffer is empty when Solana VM first executes a program, but if another program is
        // called via CPI then that program may set return data. We must clear this buffer, else return data
        // from the CPI callee will be visible to this program's callee.
        cfg.add(
            vartab,
            Instr::ReturnData {
                data: Expression::AllocDynamicBytes(
                    Loc::Codegen,
                    Type::DynamicBytes,
                    Expression::NumberLiteral(Loc::Codegen, Type::Uint(32), 0.into()).into(),
                    None,
                ),
                data_len: Expression::NumberLiteral(Loc::Codegen, Type::Uint(64), 0.into()),
            },
        );
    }

    entry
}

/// Create the dispatch for a contract constructor. This case creates a new function in
/// the CFG because we want to use the abi decoding implementation from codegen.
fn add_constructor_dispatch_case(
    contract_no: usize,
    cfg_no: usize,
    argsdata: &Expression,
    argslen: Expression,
    func_cfg: &ControlFlowGraph,
    ns: &mut Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
    opt: &Options,
) -> usize {
    let entry = cfg.new_basic_block(format!("constructor_cfg_{cfg_no}"));
    cfg.set_basic_block(entry);

    let mut returns: Vec<Expression> = Vec::new();

    if !func_cfg.params.is_empty() {
        let tys = func_cfg
            .params
            .iter()
            .map(|e| e.ty.clone())
            .collect::<Vec<Type>>();
        let encoder = create_encoder(ns, false);
        let truncated_len = Expression::Trunc(Loc::Codegen, Type::Uint(32), Box::new(argslen));
        returns = encoder.abi_decode(
            &Loc::Codegen,
            argsdata,
            &tys,
            ns,
            vartab,
            cfg,
            Some(truncated_len),
        );
    }

    if ns.target == Target::Solana {
        if let ASTFunction::SolidityFunction(function_no) = func_cfg.function_no {
            let func = &ns.functions[function_no];

            solana_deploy(func, &returns, contract_no, vartab, cfg, ns, opt);
        } else if let Some((func, _)) = &ns.contracts[contract_no].default_constructor {
            solana_deploy(func, &returns, contract_no, vartab, cfg, ns, opt);
        } else {
            unreachable!();
        }
    }

    // Call storage initializer
    cfg.add(
        vartab,
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
        vartab,
        Instr::Call {
            res: vec![],
            return_tys: vec![],
            call: InternalCallTy::Static { cfg_no },
            args: returns,
        },
    );

    cfg.add(
        vartab,
        Instr::ReturnCode {
            code: ReturnCode::Success,
        },
    );

    entry
}
