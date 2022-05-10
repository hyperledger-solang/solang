use crate::ast::{Namespace, RetrieveType, Type};
use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::vartable::Vartable;
use crate::codegen::yul::expression::expression;
use crate::codegen::{Builtin, Expression, Options};
use crate::sema::expression::coerce_number;
use crate::sema::yul::ast;
use crate::sema::yul::builtin::YulBuiltInFunction;
use num_bigint::{BigInt, Sign};
use num_traits::FromPrimitive;
use solang_parser::pt;
use solang_parser::pt::Loc;

impl Expression {
    fn to_number_literal(&self) -> Expression {
        match self {
            Expression::BoolLiteral(loc, value) => {
                let val = if *value {
                    BigInt::from(1)
                } else {
                    BigInt::from(0)
                };
                Expression::NumberLiteral(*loc, Type::Uint(256), val)
            }
            Expression::BytesLiteral(loc, _, value) => {
                let val = BigInt::from_bytes_be(Sign::NoSign, value);
                Expression::NumberLiteral(*loc, Type::Uint(256), val)
            }
            _ => panic!("expression should not be converted into number literal"),
        }
    }
}

// TODO: Warn in sema that some builtins are not implemented/incompatible
pub(crate) fn process_builtin(
    loc: &pt::Loc,
    builtin_ty: &YulBuiltInFunction,
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
            Expression::Complement(*loc, exp.ty(), Box::new(exp))
        }

        YulBuiltInFunction::IsZero => {
            let left = expression(&args[0], contract_no, ns, vartab, cfg, opt);
            let right = Expression::NumberLiteral(Loc::Codegen, left.ty(), BigInt::from(0));

            Expression::Equal(*loc, Box::new(left), Box::new(right))
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
        | YulBuiltInFunction::Exp => {
            process_binary_arithmetic(loc, builtin_ty, args, contract_no, ns, vartab, cfg, opt)
        }

        YulBuiltInFunction::Byte => {
            byte_builtin(loc, args, contract_no, ns, cfg, vartab, opt)
        }

        YulBuiltInFunction::AddMod
        | YulBuiltInFunction::MulMod => {
            let left = expression(&args[0], contract_no, ns, vartab, cfg, opt);
            let right = expression(&args[1], contract_no, ns, vartab, cfg, opt);
            let (left, right) = equalize_types(left, right, ns);

            let main_expr = if matches!(builtin_ty, YulBuiltInFunction::AddMod) {
                Expression::Add(*loc, left.ty(), false, Box::new(left), Box::new(right))
            } else {
                Expression::Multiply(*loc, left.ty(), false, Box::new(left), Box::new(right))
            };

            let mod_arg = expression(&args[2], contract_no, ns, vartab, cfg, opt);
            let (mod_left, mod_right) = equalize_types(main_expr, mod_arg, ns);
            let codegen_expr = Expression::UnsignedModulo(*loc, mod_left.ty(), Box::new(mod_left), Box::new(mod_right.clone()));
            branch_if_zero(mod_right, codegen_expr, ns, cfg, vartab)
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
        | YulBuiltInFunction::ExtCodeSize
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
            unimplemented!("{} yul builtin not implemented", function_ty.name);
        }

        YulBuiltInFunction::Gas => {
            Expression::Builtin(*loc, vec![Type::Uint(256)], Builtin::Gasleft, vec![])
        }

        YulBuiltInFunction::Address => {
            Expression::Builtin(*loc, vec![Type::Uint(256)], Builtin::GetAddress, vec![])
        }

        YulBuiltInFunction::Balance => {
            let addr = expression(&args[0], contract_no, ns, vartab, cfg, opt).cast(&Type::Address(false), ns);
            Expression::Builtin(*loc, vec![Type::Uint(256)], Builtin::Balance,vec![addr])
        }

        YulBuiltInFunction::SelfBalance => {
            let addr = Expression::Builtin(*loc, vec![Type::Contract(contract_no)], Builtin::GetAddress,vec![]);
            Expression::Builtin(*loc, vec![Type::Uint(256)], Builtin::Balance, vec![addr])
        }

        YulBuiltInFunction::Caller => {
            Expression::Builtin(*loc, vec![Type::Uint(256)], Builtin::Sender, vec![])
        }

        YulBuiltInFunction::CallValue => {
            Expression::Builtin(*loc, vec![Type::Uint(256)], Builtin::Value, vec![])
        }

        YulBuiltInFunction::SelfDestruct => {
            let recipient = expression(&args[0], contract_no, ns, vartab, cfg, opt).cast(&Type::Address(true), ns);
            cfg.add(vartab, Instr::SelfDestruct { recipient });
            Expression::Poison
        }

        YulBuiltInFunction::Invalid => {
            cfg.add(vartab, Instr::AssertFailure { expr: None });
            Expression::Poison
        }

        YulBuiltInFunction::GasPrice => {
            Expression::Builtin(*loc, vec![Type::Uint(256)], Builtin::Gasprice, vec![])
        }

        YulBuiltInFunction::BlockHash => {
            // TODO: Will these returns types work?
            let arg = expression(&args[0], contract_no, ns, vartab, cfg, opt);
            Expression::Builtin(*loc, vec![Type::Uint(256)], Builtin::BlockHash, vec![arg])
        }

        YulBuiltInFunction::CoinBase => {
            Expression::Builtin(*loc, vec![Type::Uint(256)], Builtin::BlockCoinbase, vec![])
        }

        YulBuiltInFunction::Timestamp => {
            Expression::Builtin(*loc, vec![Type::Uint(256)], Builtin::Timestamp, vec![])
        }

        YulBuiltInFunction::Number => {
            Expression::Builtin(*loc, vec![Type::Uint(256)], Builtin::BlockNumber, vec![])
        }

        YulBuiltInFunction::Difficulty => {
            Expression::Builtin(*loc, vec![Type::Uint(256)], Builtin::BlockDifficulty, vec![])
        }

        YulBuiltInFunction::GasLimit => {
            Expression::Builtin(*loc, vec![Type::Uint(256)], Builtin::GasLimit, vec![])
        }
    }
}

fn process_binary_arithmetic(
    loc: &pt::Loc,
    builtin_ty: &YulBuiltInFunction,
    args: &[ast::YulExpression],
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
    opt: &Options,
) -> Expression {
    let left = expression(&args[0], contract_no, ns, vartab, cfg, opt);
    let right = expression(&args[1], contract_no, ns, vartab, cfg, opt);
    let (left, right) = equalize_types(left, right, ns);

    match builtin_ty {
        YulBuiltInFunction::Add => {
            Expression::Add(*loc, left.ty(), true, Box::new(left), Box::new(right))
        }
        YulBuiltInFunction::Sub => {
            Expression::Subtract(*loc, left.ty(), true, Box::new(left), Box::new(right))
        }
        YulBuiltInFunction::Mul => {
            Expression::Multiply(*loc, left.ty(), true, Box::new(left), Box::new(right))
        }
        YulBuiltInFunction::Div => {
            let expr =
                Expression::SignedDivide(*loc, left.ty(), Box::new(left), Box::new(right.clone()));
            branch_if_zero(right, expr, ns, cfg, vartab)
        }
        YulBuiltInFunction::SDiv => {
            let expr = Expression::UnsignedDivide(
                *loc,
                left.ty(),
                Box::new(left),
                Box::new(right.clone()),
            );
            branch_if_zero(right, expr, ns, cfg, vartab)
        }
        YulBuiltInFunction::Mod => {
            let expr = Expression::UnsignedModulo(
                *loc,
                left.ty(),
                Box::new(left),
                Box::new(right.clone()),
            );
            branch_if_zero(right, expr, ns, cfg, vartab)
        }
        YulBuiltInFunction::SMod => {
            let expr =
                Expression::SignedModulo(*loc, left.ty(), Box::new(left), Box::new(right.clone()));
            branch_if_zero(right, expr, ns, cfg, vartab)
        }
        YulBuiltInFunction::Exp => {
            Expression::Power(*loc, left.ty(), true, Box::new(left), Box::new(right))
        }
        YulBuiltInFunction::Lt => Expression::UnsignedLess(*loc, Box::new(left), Box::new(right)),
        YulBuiltInFunction::Gt => Expression::UnsignedMore(*loc, Box::new(left), Box::new(right)),
        YulBuiltInFunction::Slt => Expression::SignedLess(*loc, Box::new(left), Box::new(right)),
        YulBuiltInFunction::Sgt => Expression::SignedMore(*loc, Box::new(left), Box::new(right)),
        YulBuiltInFunction::Eq => Expression::Equal(*loc, Box::new(left), Box::new(right)),
        YulBuiltInFunction::And => {
            Expression::BitwiseAnd(*loc, left.ty(), Box::new(left), Box::new(right))
        }
        YulBuiltInFunction::Or => {
            Expression::BitwiseOr(*loc, left.ty(), Box::new(left), Box::new(right))
        }
        YulBuiltInFunction::Xor => {
            Expression::BitwiseXor(*loc, left.ty(), Box::new(left), Box::new(right))
        }
        YulBuiltInFunction::Shl => {
            Expression::ShiftLeft(*loc, left.ty(), Box::new(left), Box::new(right))
        }
        YulBuiltInFunction::Shr => {
            Expression::ShiftRight(*loc, left.ty(), Box::new(left), Box::new(right), false)
        }
        YulBuiltInFunction::Sar => {
            Expression::ShiftRight(*loc, left.ty(), Box::new(left), Box::new(right), true)
        }

        _ => panic!("This is not a binary arithmetic operation!"),
    }
}

fn equalize_types(
    mut right: Expression,
    mut left: Expression,
    ns: &Namespace,
) -> (Expression, Expression) {
    if !matches!(left, Expression::NumberLiteral(..)) {
        left = left.to_number_literal();
    }

    if !matches!(right, Expression::NumberLiteral(..)) {
        right = right.to_number_literal();
    }

    let left_ty = left.ty();
    let right_ty = right.ty();
    if left_ty != right_ty {
        let mut vec = Vec::new();
        let casted_type = coerce_number(
            &left_ty,
            &pt::Loc::Codegen,
            &right_ty,
            &pt::Loc::Codegen,
            false,
            false,
            ns,
            &mut vec,
        )
        .unwrap();

        left = left.cast(&casted_type, ns);
        right = right.cast(&casted_type, ns);
    }

    (left, right)
}

fn branch_if_zero(
    variable: Expression,
    codegen_expr: Expression,
    ns: &Namespace,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) -> Expression {
    let temp = vartab.temp_anonymous(&Type::Uint(256));
    let cond = Expression::Equal(
        Loc::Codegen,
        Box::new(variable.clone()),
        Box::new(Expression::NumberLiteral(
            Loc::Codegen,
            variable.ty(),
            BigInt::from(0),
        )),
    );

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
    vartab.new_dirty_tracker(ns.next_id);
    cfg.add(
        vartab,
        Instr::Set {
            loc: Loc::Codegen,
            res: temp,
            expr: Expression::NumberLiteral(Loc::Codegen, Type::Uint(256), BigInt::from(0)),
        },
    );
    cfg.add(vartab, Instr::Branch { block: endif });

    cfg.set_basic_block(else_);
    cfg.add(
        vartab,
        Instr::Set {
            loc: Loc::Codegen,
            res: temp,
            expr: codegen_expr,
        },
    );
    cfg.add(vartab, Instr::Branch { block: endif });
    cfg.set_phis(endif, vartab.pop_dirty_tracker());
    cfg.set_basic_block(endif);

    Expression::Variable(Loc::Codegen, Type::Uint(256), temp)
}

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
    let cond = Expression::MoreEqual(
        *loc,
        Box::new(offset.clone()),
        Box::new(Expression::NumberLiteral(
            *loc,
            Type::Uint(256),
            BigInt::from(32),
        )),
    );

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
    vartab.new_dirty_tracker(ns.next_id);
    cfg.add(
        vartab,
        Instr::Set {
            loc: Loc::Codegen,
            res: temp,
            expr: Expression::NumberLiteral(Loc::Codegen, Type::Uint(256), BigInt::from(0)),
        },
    );
    cfg.add(vartab, Instr::Branch { block: endif });

    cfg.set_basic_block(else_);

    // The follwing implementes the operation (arg[1] >> (8 * (31 - arg[0]))) & 0xff
    let op_31_sub_arg0 = Expression::Subtract(
        *loc,
        Type::Uint(256),
        false,
        Box::new(Expression::NumberLiteral(
            *loc,
            Type::Uint(256),
            BigInt::from(31),
        )),
        Box::new(offset),
    );
    let op_eight_times = Expression::ShiftRight(
        *loc,
        Type::Uint(256),
        Box::new(op_31_sub_arg0),
        Box::new(Expression::NumberLiteral(
            *loc,
            Type::Uint(256),
            BigInt::from(3),
        )),
        false,
    );
    let op_shift_right = Expression::ShiftRight(
        *loc,
        Type::Uint(256),
        Box::new(
            expression(&args[1], contract_no, ns, vartab, cfg, opt).cast(&Type::Uint(256), ns),
        ),
        Box::new(op_eight_times),
        false,
    );
    let masked_result = Expression::BitwiseAnd(
        *loc,
        Type::Uint(256),
        Box::new(op_shift_right),
        Box::new(Expression::NumberLiteral(
            *loc,
            Type::Uint(256),
            BigInt::from_u8(255).unwrap(),
        )),
    );

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

    Expression::Variable(Loc::Codegen, Type::Uint(256), temp)
}
