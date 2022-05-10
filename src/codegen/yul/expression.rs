use crate::ast::{Namespace, Type};
use crate::codegen;
use crate::codegen::cfg::{ControlFlowGraph, Instr, InternalCallTy};
use crate::codegen::vartable::Vartable;
use crate::codegen::yul::builtin::process_builtin;
use crate::codegen::{Builtin, Expression, Options};
use crate::sema::yul::ast;
use crate::sema::yul::ast::YulSuffix;
use num_bigint::BigInt;
use solang_parser::pt;
use solang_parser::pt::{Loc, StorageLocation};

pub(crate) fn expression(
    expr: &ast::YulExpression,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
    opt: &Options,
) -> Expression {
    match expr {
        ast::YulExpression::BoolLiteral(loc, value, ty) => {
            if matches!(ty, Type::Bool) {
                return Expression::BoolLiteral(*loc, *value);
            }

            // If the user has coerced a type for bool, it is a number literal.
            let num = if *value {
                BigInt::from(1)
            } else {
                BigInt::from(0)
            };

            Expression::NumberLiteral(*loc, ty.clone(), num)
        }
        ast::YulExpression::NumberLiteral(loc, value, ty) => {
            Expression::NumberLiteral(*loc, ty.clone(), value.clone())
        }
        ast::YulExpression::StringLiteral(loc, value, ty) => {
            Expression::BytesLiteral(*loc, ty.clone(), value.clone())
        }
        ast::YulExpression::YulLocalVariable(loc, ty, var_no) => {
            Expression::Variable(*loc, ty.clone(), *var_no)
        }
        ast::YulExpression::ConstantVariable(_, _, Some(var_contract_no), var_no) => {
            codegen::expression(
                ns.contracts[*var_contract_no].variables[*var_no]
                    .initializer
                    .as_ref()
                    .unwrap(),
                cfg,
                contract_no,
                None,
                ns,
                vartab,
                opt,
            )
        }
        ast::YulExpression::ConstantVariable(_, _, None, var_no) => codegen::expression(
            ns.constants[*var_no].initializer.as_ref().unwrap(),
            cfg,
            contract_no,
            None,
            ns,
            vartab,
            opt,
        ),
        ast::YulExpression::StorageVariable(..)
        | ast::YulExpression::SolidityLocalVariable(_, _, Some(StorageLocation::Storage(_)), ..) => {
            panic!("Storage variables cannot be accessed without suffixed in yul");
        }
        ast::YulExpression::SolidityLocalVariable(loc, ty, _, var_no) => {
            Expression::Variable(*loc, ty.clone(), *var_no)
        }
        ast::YulExpression::MemberAccess(loc, expr, suffix) => {
            process_member_access(loc, expr, suffix, contract_no, vartab, cfg, ns, opt)
        }
        ast::YulExpression::FunctionCall(_, function_no, args, _) => {
            let mut returns =
                process_function_call(*function_no, args, contract_no, vartab, cfg, ns, opt);
            assert_eq!(returns.len(), 1);
            returns.remove(0)
        }

        ast::YulExpression::BuiltInCall(loc, builtin_ty, args) => {
            process_builtin(loc, builtin_ty, args, contract_no, ns, vartab, cfg, opt)
        }
    }
}

fn process_member_access(
    loc: &pt::Loc,
    expr: &ast::YulExpression,
    suffix: &YulSuffix,
    contract_no: usize,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
    ns: &Namespace,
    opt: &Options,
) -> Expression {
    match suffix {
        YulSuffix::Slot => match expr {
            ast::YulExpression::StorageVariable(_, _, contract_no, var_no) => {
                // TODO: Check if defaulting type to Uint(256) is going to work
                return ns.contracts[*contract_no].get_storage_slot(
                    *contract_no,
                    *var_no,
                    ns,
                    Some(Type::Uint(256)),
                );
            }
            ast::YulExpression::SolidityLocalVariable(
                loc,
                _,
                Some(StorageLocation::Storage(_)),
                var_no,
            ) => {
                // TODO: Check if defaulting type to Uint(256) is going to work
                return Expression::Variable(*loc, Type::Uint(256), *var_no);
            }

            _ => (),
        },
        YulSuffix::Offset => match expr {
            ast::YulExpression::StorageVariable(..)
            | ast::YulExpression::SolidityLocalVariable(
                _,
                _,
                Some(StorageLocation::Storage(_)),
                ..,
            ) => {
                // TODO: Check if defaulting type to Uint(256) is going to work
                return Expression::NumberLiteral(Loc::Codegen, Type::Uint(256), BigInt::from(0));
            }

            ast::YulExpression::SolidityLocalVariable(
                _,
                Type::Array(_, ref dims),
                Some(StorageLocation::Calldata(_)),
                var_no,
            ) => {
                if dims.last().unwrap().is_none() {
                    // TODO: Check if defaulting type to Uint(256) is going to work
                    return Expression::Variable(*loc, Type::Uint(256), *var_no);
                }
            }

            _ => (),
        },

        YulSuffix::Length => {
            if let ast::YulExpression::SolidityLocalVariable(
                _,
                Type::Array(_, ref dims),
                Some(StorageLocation::Calldata(_)),
                _,
            ) = expr
            {
                // TODO: Check if defaulting type to Uint(256) is going to work
                if dims.last().unwrap().is_none() {
                    return Expression::Builtin(
                        *loc,
                        vec![Type::Uint(256)],
                        Builtin::ArrayLength,
                        vec![expression(expr, contract_no, ns, vartab, cfg, opt)],
                    );
                }
            }
        }

        YulSuffix::Selector | YulSuffix::Address => {
            if let ast::YulExpression::SolidityLocalVariable(_, Type::ExternalFunction { .. }, ..) =
                expr
            {
                // TODO: Check if defaulting type to Uint(256) is going to work
                return Expression::Builtin(
                    *loc,
                    vec![Type::Uint(256)],
                    if matches!(suffix, YulSuffix::Selector) {
                        Builtin::FunctionSelector
                    } else {
                        Builtin::ExternalFunctionAddress
                    },
                    vec![expression(expr, contract_no, ns, vartab, cfg, opt)],
                );
            }
        }
    }

    unreachable!("Expression does not support suffixes");
}

pub(crate) fn process_function_call(
    function_no: usize,
    args: &[ast::YulExpression],
    contract_no: usize,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
    ns: &Namespace,
    opt: &Options,
) -> Vec<Expression> {
    let args = args
        .iter()
        .map(|a| expression(a, contract_no, ns, vartab, cfg, opt))
        .collect();

    let cfg_no = ns.yul_functions[function_no].cfg_no;

    if ns.yul_functions[function_no].returns.is_empty() {
        cfg.add(
            vartab,
            Instr::Call {
                res: Vec::new(),
                return_tys: Vec::new(),
                call: InternalCallTy::Static(cfg_no),
                args,
            },
        );

        return vec![Expression::Poison];
    }

    let mut res = Vec::new();
    let mut returns = Vec::new();
    let mut return_tys = Vec::new();

    for ret in &*ns.yul_functions[function_no].returns {
        let id = pt::Identifier {
            loc: ret.loc,
            name: ret.name_as_str().to_owned(),
        };

        let temp_pos = vartab.temp(&id, &ret.ty);
        return_tys.push(ret.ty.clone());
        res.push(temp_pos);
        returns.push(Expression::Variable(id.loc, ret.ty.clone(), temp_pos));
    }

    cfg.add(
        vartab,
        Instr::Call {
            res,
            call: InternalCallTy::Static(cfg_no),
            args,
            return_tys,
        },
    );

    returns
}
