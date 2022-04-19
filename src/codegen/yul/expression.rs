use crate::ast::{Function, Namespace, Type};
use crate::codegen;
use crate::codegen::cfg::ControlFlowGraph;
use crate::codegen::vartable::Vartable;
use crate::codegen::{Expression, Options};
use crate::sema::yul::ast;
use num_bigint::BigInt;
use solang_parser::pt::StorageLocation;

// TODO: This is a workaround to avoid compiler warnings during development.
#[allow(dead_code)]
pub(crate) fn expression(
    expr: &ast::YulExpression,
    contract_no: usize,
    func: Option<&Function>,
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
                func,
                ns,
                vartab,
                opt,
            )
        }
        ast::YulExpression::ConstantVariable(_, _, None, var_no) => codegen::expression(
            ns.constants[*var_no].initializer.as_ref().unwrap(),
            cfg,
            contract_no,
            func,
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

        // TODO: This is a workaround to avoid compiler errors
        _ => Expression::Poison,
    }
}
