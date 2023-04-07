// SPDX-License-Identifier: Apache-2.0

use crate::codegen::expression::{assert_failure, log_runtime_error};
use crate::codegen::{
    cfg::{ControlFlowGraph, Instr},
    vartable::Vartable,
    yul::expression::expression,
    {Builtin, Expression, Options},
};
use crate::sema::ast::{Namespace, RetrieveType, Type};
use crate::sema::{
    diagnostics::Diagnostics,
    expression::integers::coerce_number,
    yul::{ast, builtin::YulBuiltInFunction},
};
use num_bigint::BigInt;
use num_traits::{FromPrimitive, Zero};
use solang_parser::pt;

impl Expression {
    fn to_number_literal(&self) -> Expression {
        match self {
            Expression::BoolLiteral { loc, value } => {
                let val = if *value {
                    BigInt::from(1)
                } else {
                    BigInt::from(0)
                };
                Expression::NumberLiteral {
                    loc: *loc,
                    ty: Type::Uint(256),
                    value: val,
                }
            }
            _ => panic!("expression should not be converted into number literal"),
        }
    }
}

/// Transfrom YUL builtin functions into CFG instructions
pub(crate) fn process_builtin(
    loc: &pt::Loc,
    builtin_ty: YulBuiltInFunction,
    args: &[ast::YulExpression],
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
    opt: &Options,
) -> Expression {
    match builtin_ty {
        YulBuiltInFunction::Not => {
            let exp = expression(&args[0], contract_no, ns, vartab, cfg, opt);
            Expression::BitwiseNot { loc: *loc, ty: exp.ty(), expr: Box::new(exp) }
        }

        YulBuiltInFunction::IsZero => {
            let left = expression(&args[0], contract_no, ns, vartab, cfg, opt);
            let right = Expression::NumberLiteral { loc: pt::Loc::Codegen, ty: left.ty(), value: BigInt::from(0) };

            Expression::Equal { loc: *loc, left: Box::new(left), right: Box::new(right) }
        }

        YulBuiltInFunction::Add
        | YulBuiltInFunction::Sub
        | YulBuiltInFunction::Mul
        | YulBuiltInFunction::Div
        | YulBuiltInFunction::SDiv
        | YulBuiltInFunction::Mod
        | YulBuiltInFunction::SMod
        | YulBuiltInFunction::Lt
        | YulBuiltInFunction::Gt
        | YulBuiltInFunction::Slt
        | YulBuiltInFunction::Sgt
        | YulBuiltInFunction::Eq
        | YulBuiltInFunction::And
        | YulBuiltInFunction::Or
        | YulBuiltInFunction::Xor
        | YulBuiltInFunction::Shl
        | YulBuiltInFunction::Shr
        | YulBuiltInFunction::Sar
        | YulBuiltInFunction::Exp
        | YulBuiltInFunction::AddMod
        | YulBuiltInFunction::MulMod => {
            process_arithmetic(loc, builtin_ty, args, contract_no, ns, vartab, cfg, opt)
        }

        YulBuiltInFunction::Byte => {
            byte_builtin(loc, args, contract_no, ns, cfg, vartab, opt)
        }

        YulBuiltInFunction::SignExtend
        | YulBuiltInFunction::Keccak256
        | YulBuiltInFunction::Pop
        | YulBuiltInFunction::Pc
        | YulBuiltInFunction::ChainId
        | YulBuiltInFunction::BaseFee
        // Memory functions: need to convert between number to pointer type
        | YulBuiltInFunction::MLoad
        | YulBuiltInFunction::MStore
        | YulBuiltInFunction::MStore8
        | YulBuiltInFunction::MSize
        // Storage function: need to think about how to deal with pointer size and the size of chunk to load
        | YulBuiltInFunction::SStore
        | YulBuiltInFunction::SLoad
        // Calldata functions: the same problems with other memory functions
        | YulBuiltInFunction::CallDataLoad
        | YulBuiltInFunction::CallDataSize
        | YulBuiltInFunction::CallDataCopy
        // Functions that manage code memory
        | YulBuiltInFunction::CodeSize
        | YulBuiltInFunction::CodeCopy
        | YulBuiltInFunction::ExtCodeCopy
        | YulBuiltInFunction::ExtCodeHash
        // Functions that manage return data
        | YulBuiltInFunction::ReturnDataSize
        | YulBuiltInFunction::ReturnDataCopy
        // Functions that manage contracts
        | YulBuiltInFunction::Create
        | YulBuiltInFunction::Create2
        | YulBuiltInFunction::Call
        | YulBuiltInFunction::CallCode
        | YulBuiltInFunction::DelegateCall
        | YulBuiltInFunction::StaticCall
        // Return and revert also load from memory, so we first need to solve mload and mstore builtins
        | YulBuiltInFunction::Return
        | YulBuiltInFunction::Stop // Stop is the same as return(0, 0)
        | YulBuiltInFunction::Revert
        // Log functions
        | YulBuiltInFunction::Log0
        | YulBuiltInFunction::Log1
        | YulBuiltInFunction::Log2
        | YulBuiltInFunction::Log3
        | YulBuiltInFunction::Log4
        // origin is the same as tx.origin and is not implemented
        | YulBuiltInFunction::Origin
        => {
            let function_ty = builtin_ty.get_prototype_info();
            unreachable!("{} yul builtin not implemented", function_ty.name);
        }

        YulBuiltInFunction::Gas => {
            Expression::Builtin { loc: *loc, tys: vec![Type::Uint(64)], builtin: Builtin::Gasleft, args: vec![] }
        }

        YulBuiltInFunction::Address => {
            Expression::Builtin { loc: *loc, tys: vec![Type::Address(false)], builtin: Builtin::GetAddress, args: vec![] }
        }

        YulBuiltInFunction::Balance => {
            let addr = expression(&args[0], contract_no, ns, vartab, cfg, opt).cast(&Type::Address(false), ns);
            Expression::Builtin { loc: *loc, tys: vec![Type::Value], builtin: Builtin::Balance, args: vec![addr] }
        }

        YulBuiltInFunction::SelfBalance => {
            let addr = Expression::Builtin { loc: *loc, tys: vec![Type::Contract(contract_no)], builtin: Builtin::GetAddress, args: vec![] };
            Expression::Builtin { loc: *loc, tys: vec![Type::Value], builtin: Builtin::Balance, args: vec![addr] }
        }

        YulBuiltInFunction::Caller => {
            Expression::Builtin { loc: *loc, tys: vec![Type::Address(true)], builtin: Builtin::Sender, args: vec![] }
        }

        YulBuiltInFunction::CallValue => {
            Expression::Builtin { loc: *loc, tys: vec![Type::Value], builtin: Builtin::Value, args: vec![] }
        }

        YulBuiltInFunction::SelfDestruct => {
            let recipient = expression(&args[0], contract_no, ns, vartab, cfg, opt).cast(&Type::Address(true), ns);
            cfg.add(vartab, Instr::SelfDestruct { recipient });
            Expression::Poison
        }

        YulBuiltInFunction::Invalid => {
            log_runtime_error(opt.log_runtime_errors,  "reached invalid instruction", *loc, cfg,
            vartab,
            ns);
            assert_failure(loc, None, ns, cfg, vartab);
            Expression::Poison
        }

        YulBuiltInFunction::GasPrice => {
            Expression::Builtin { loc: *loc, tys: vec![Type::Uint(64)], builtin: Builtin::Gasprice, args: vec![] }
        }

        YulBuiltInFunction::ExtCodeSize => {
            let address = expression(&args[0], contract_no, ns, vartab, cfg, opt).cast(&Type::Address(false), ns);
            Expression::Builtin { loc: *loc, tys: vec![Type::Uint(32)], builtin: Builtin::ExtCodeSize, args: vec![address] }
        }

        YulBuiltInFunction::BlockHash => {
            let arg = expression(&args[0], contract_no, ns, vartab, cfg, opt).cast(&Type::Uint(64), ns);
            Expression::Builtin { loc: *loc, tys: vec![Type::Uint(256)], builtin: Builtin::BlockHash, args: vec![arg] }
        }

        YulBuiltInFunction::CoinBase => {
            Expression::Builtin { loc: *loc, tys: vec![Type::Address(false)], builtin: Builtin::BlockCoinbase, args: vec![] }
        }

        YulBuiltInFunction::Timestamp => {
            Expression::Builtin { loc: *loc, tys: vec![Type::Uint(64)], builtin: Builtin::Timestamp, args: vec![] }
        }

        YulBuiltInFunction::Number => {
            Expression::Builtin { loc: *loc, tys: vec![Type::Uint(64)], builtin: Builtin::BlockNumber, args: vec![] }
        }

        YulBuiltInFunction::Difficulty => {
            Expression::Builtin { loc: *loc, tys: vec![Type::Uint(256)], builtin: Builtin::BlockDifficulty, args: vec![] }
        }

        YulBuiltInFunction::GasLimit => {
            Expression::Builtin { loc: *loc, tys: vec![Type::Uint(64)], builtin: Builtin::GasLimit, args: vec![] }
        }
    }
}

/// Process arithmetic operations
fn process_arithmetic(
    loc: &pt::Loc,
    builtin_ty: YulBuiltInFunction,
    args: &[ast::YulExpression],
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
    opt: &Options,
) -> Expression {
    let left = expression(&args[0], contract_no, ns, vartab, cfg, opt);
    let right = expression(&args[1], contract_no, ns, vartab, cfg, opt);

    let left = cast_to_number(left, ns);
    let right = cast_to_number(right, ns);

    let (left, right) = equalize_types(left, right, ns);

    match builtin_ty {
        YulBuiltInFunction::Add => Expression::Add {
            loc: *loc,
            ty: left.ty(),
            unchecked: true,
            left: Box::new(left),
            right: Box::new(right),
        },
        YulBuiltInFunction::Sub => Expression::Subtract {
            loc: *loc,
            ty: left.ty(),
            unchecked: true,
            left: Box::new(left),
            right: Box::new(right),
        },
        YulBuiltInFunction::Mul => Expression::Multiply {
            loc: *loc,
            ty: left.ty(),
            unchecked: true,
            left: Box::new(left),
            right: Box::new(right),
        },
        YulBuiltInFunction::Div => {
            let expr = Expression::UnsignedDivide {
                loc: *loc,
                ty: left.ty(),
                left: Box::new(left),
                right: Box::new(right.clone()),
            };
            branch_if_zero(right, expr, cfg, vartab)
        }
        YulBuiltInFunction::SDiv => {
            let expr = Expression::SignedDivide {
                loc: *loc,
                ty: left.ty(),
                left: Box::new(left),
                right: Box::new(right.clone()),
            };
            branch_if_zero(right, expr, cfg, vartab)
        }
        YulBuiltInFunction::Mod => {
            let expr = Expression::UnsignedModulo {
                loc: *loc,
                ty: left.ty(),
                left: Box::new(left),
                right: Box::new(right.clone()),
            };
            branch_if_zero(right, expr, cfg, vartab)
        }
        YulBuiltInFunction::SMod => {
            let expr = Expression::SignedModulo {
                loc: *loc,
                ty: left.ty(),
                left: Box::new(left),
                right: Box::new(right.clone()),
            };
            branch_if_zero(right, expr, cfg, vartab)
        }
        YulBuiltInFunction::Exp => Expression::Power {
            loc: *loc,
            ty: left.ty(),
            unchecked: true,
            base: Box::new(left),
            exp: Box::new(right),
        },
        YulBuiltInFunction::Lt => Expression::Less {
            loc: *loc,
            signed: false,
            left: Box::new(left),
            right: Box::new(right),
        },
        YulBuiltInFunction::Gt => Expression::More {
            loc: *loc,
            signed: false,
            left: Box::new(left),
            right: Box::new(right),
        },
        YulBuiltInFunction::Slt => Expression::Less {
            loc: *loc,
            signed: true,
            left: Box::new(left),
            right: Box::new(right),
        },
        YulBuiltInFunction::Sgt => Expression::More {
            loc: *loc,
            signed: true,
            left: Box::new(left),
            right: Box::new(right),
        },
        YulBuiltInFunction::Eq => Expression::Equal {
            loc: *loc,
            left: Box::new(left),
            right: Box::new(right),
        },
        YulBuiltInFunction::And => Expression::BitwiseAnd {
            loc: *loc,
            ty: left.ty(),
            left: Box::new(left),
            right: Box::new(right),
        },
        YulBuiltInFunction::Or => Expression::BitwiseOr {
            loc: *loc,
            ty: left.ty(),
            left: Box::new(left),
            right: Box::new(right),
        },
        YulBuiltInFunction::Xor => Expression::BitwiseXor {
            loc: *loc,
            ty: left.ty(),
            left: Box::new(left),
            right: Box::new(right),
        },
        // For bit shifting, the syntax is the following: shr(x, y) shifts right y by x bits.
        YulBuiltInFunction::Shl => Expression::ShiftLeft {
            loc: *loc,
            ty: left.ty(),
            left: Box::new(right),
            right: Box::new(left),
        },
        YulBuiltInFunction::Shr => Expression::ShiftRight {
            loc: *loc,
            ty: left.ty(),
            left: Box::new(right),
            right: Box::new(left),
            signed: false,
        },
        YulBuiltInFunction::Sar => Expression::ShiftRight {
            loc: *loc,
            ty: left.ty(),
            left: Box::new(right),
            right: Box::new(left),
            signed: true,
        },

        YulBuiltInFunction::AddMod | YulBuiltInFunction::MulMod => {
            let modulo_operand = expression(&args[2], contract_no, ns, vartab, cfg, opt);
            let (_, equalized_modulo) = equalize_types(left.clone(), modulo_operand.clone(), ns);
            let builtin = if builtin_ty == YulBuiltInFunction::AddMod {
                Builtin::AddMod
            } else {
                Builtin::MulMod
            };
            let codegen_expr = Expression::Builtin {
                loc: *loc,
                tys: vec![left.ty()],
                builtin,
                args: vec![right, left, equalized_modulo],
            };
            branch_if_zero(modulo_operand, codegen_expr, cfg, vartab)
        }

        _ => panic!("This is not a binary arithmetic operation!"),
    }
}

/// Arithmetic operations work on numbers, so addresses and pointers need to be
/// converted to integers
fn cast_to_number(expr: Expression, ns: &Namespace) -> Expression {
    let ty = expr.ty();

    if !ty.is_contract_storage() && ty.is_reference_type(ns) {
        Expression::Cast {
            loc: pt::Loc::Codegen,
            ty: Type::Uint(ns.target.ptr_size()),
            expr: expr.into(),
        }
    } else if ty.is_address() {
        Expression::Cast {
            loc: pt::Loc::Codegen,
            ty: Type::Uint((ns.address_length * 8) as u16),
            expr: expr.into(),
        }
    } else {
        expr
    }
}

/// This function matches the type between the right and left hand sides of operations
fn equalize_types(
    mut left: Expression,
    mut right: Expression,
    ns: &Namespace,
) -> (Expression, Expression) {
    if matches!(
        left,
        Expression::BytesLiteral { .. } | Expression::BoolLiteral { .. }
    ) {
        left = left.to_number_literal();
    }

    if matches!(
        right,
        Expression::BytesLiteral { .. } | Expression::BoolLiteral { .. }
    ) {
        right = right.to_number_literal();
    }

    let left_ty = left.ty();
    let right_ty = right.ty();
    if left_ty != right_ty {
        let mut diagnostics = Diagnostics::default();
        let casted_type = coerce_number(
            &left_ty,
            &pt::Loc::Codegen,
            &right_ty,
            &pt::Loc::Codegen,
            false,
            false,
            ns,
            &mut diagnostics,
        )
        .unwrap();

        left = left.cast(&casted_type, ns);
        right = right.cast(&casted_type, ns);
    }

    (left, right)
}

/// In some Yul functions, we need to branch if the argument is zero.
/// e.g. 'x := div(y, 0)'. Division by zero returns 0 in Yul.
fn branch_if_zero(
    variable: Expression,
    codegen_expr: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) -> Expression {
    let temp = vartab.temp_anonymous(&Type::Uint(256));
    let cond = Expression::Equal {
        loc: pt::Loc::Codegen,
        left: Box::new(variable.clone()),
        right: Box::new(Expression::NumberLiteral {
            loc: pt::Loc::Codegen,
            ty: variable.ty(),
            value: BigInt::zero(),
        }),
    };

    let then = cfg.new_basic_block("then".to_string());
    let else_ = cfg.new_basic_block("else".to_string());
    let endif = cfg.new_basic_block("endif".to_string());
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond,
            true_block: then,
            false_block: else_,
        },
    );

    cfg.set_basic_block(then);
    vartab.new_dirty_tracker();
    cfg.add(
        vartab,
        Instr::Set {
            loc: pt::Loc::Codegen,
            res: temp,
            expr: Expression::NumberLiteral {
                loc: pt::Loc::Codegen,
                ty: Type::Uint(256),
                value: BigInt::from(0),
            },
        },
    );
    cfg.add(vartab, Instr::Branch { block: endif });

    cfg.set_basic_block(else_);
    cfg.add(
        vartab,
        Instr::Set {
            loc: pt::Loc::Codegen,
            res: temp,
            expr: codegen_expr,
        },
    );
    cfg.add(vartab, Instr::Branch { block: endif });
    cfg.set_phis(endif, vartab.pop_dirty_tracker());
    cfg.set_basic_block(endif);

    Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(256),
        var_no: temp,
    }
}

/// This function implements the byte builtin
fn byte_builtin(
    loc: &pt::Loc,
    args: &[ast::YulExpression],
    contract_no: usize,
    ns: &Namespace,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    opt: &Options,
) -> Expression {
    let offset = expression(&args[0], contract_no, ns, vartab, cfg, opt).cast(&Type::Uint(256), ns);
    let cond = Expression::MoreEqual {
        loc: *loc,
        signed: false,
        left: Box::new(offset.clone()),
        right: Box::new(Expression::NumberLiteral {
            loc: *loc,
            ty: Type::Uint(256),
            value: BigInt::from(32),
        }),
    };

    let temp = vartab.temp_anonymous(&Type::Uint(256));

    let then = cfg.new_basic_block("then".to_string());
    let else_ = cfg.new_basic_block("else".to_string());
    let endif = cfg.new_basic_block("endif".to_string());

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond,
            true_block: then,
            false_block: else_,
        },
    );

    cfg.set_basic_block(then);
    vartab.new_dirty_tracker();
    cfg.add(
        vartab,
        Instr::Set {
            loc: pt::Loc::Codegen,
            res: temp,
            expr: Expression::NumberLiteral {
                loc: pt::Loc::Codegen,
                ty: Type::Uint(256),
                value: BigInt::zero(),
            },
        },
    );
    cfg.add(vartab, Instr::Branch { block: endif });

    cfg.set_basic_block(else_);

    // The following implements the operation (arg[1] >> (8 * (31 - arg[0]))) & 0xff
    let op_31_sub_arg0 = Expression::Subtract {
        loc: *loc,
        ty: Type::Uint(256),
        unchecked: false,
        left: Box::new(Expression::NumberLiteral {
            loc: *loc,
            ty: Type::Uint(256),
            value: BigInt::from(31),
        }),
        right: Box::new(offset),
    };
    let op_eight_times = Expression::ShiftLeft {
        loc: *loc,
        ty: Type::Uint(256),
        left: Box::new(op_31_sub_arg0),
        right: Box::new(Expression::NumberLiteral {
            loc: *loc,
            ty: Type::Uint(256),
            value: BigInt::from(3),
        }),
    };
    let op_shift_right = Expression::ShiftRight {
        loc: *loc,
        ty: Type::Uint(256),
        left: Box::new(
            expression(&args[1], contract_no, ns, vartab, cfg, opt).cast(&Type::Uint(256), ns),
        ),
        right: Box::new(op_eight_times),
        signed: false,
    };
    let masked_result = Expression::BitwiseAnd {
        loc: *loc,
        ty: Type::Uint(256),
        left: Box::new(op_shift_right),
        right: Box::new(Expression::NumberLiteral {
            loc: *loc,
            ty: Type::Uint(256),
            value: BigInt::from_u8(255).unwrap(),
        }),
    };

    cfg.add(
        vartab,
        Instr::Set {
            loc: *loc,
            res: temp,
            expr: masked_result,
        },
    );
    cfg.add(vartab, Instr::Branch { block: endif });

    cfg.set_phis(endif, vartab.pop_dirty_tracker());
    cfg.set_basic_block(endif);

    Expression::Variable {
        loc: pt::Loc::Codegen,
        ty: Type::Uint(256),
        var_no: temp,
    }
}
