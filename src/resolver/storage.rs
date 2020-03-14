use num_bigint::BigInt;
use num_traits::FromPrimitive;
use num_traits::One;
use num_traits::Zero;

use super::cfg::{ControlFlowGraph, Instr, Vartable};
use super::expression::{cast, expression, Expression};
use output::Output;
use parser::ast;
use resolver;

/// Given a storage slot which is the start of the array, calculate the
/// offset of the array element. This function exists to avoid doing
/// 256 bit multiply if possible.
pub fn array_offset(
    loc: &ast::Loc,
    start: Expression,
    index: Expression,
    elem_ty: resolver::Type,
    ns: &resolver::Contract,
) -> Expression {
    let elem_size = elem_ty.storage_slots(ns);

    // the index needs to be cast to i256 and multiplied by the number
    // of slots for each element
    if elem_size == BigInt::one() {
        Expression::Add(*loc, Box::new(start), Box::new(index))
    } else if (elem_size.clone() & (elem_size.clone() - BigInt::one())) == BigInt::zero() {
        // elem_size is power of 2
        Expression::ShiftLeft(
            *loc,
            Box::new(start),
            Box::new(Expression::ShiftLeft(
                *loc,
                Box::new(index),
                Box::new(Expression::NumberLiteral(
                    *loc,
                    256,
                    BigInt::from_usize(elem_size.bits()).unwrap(),
                )),
            )),
        )
    } else {
        Expression::Add(
            *loc,
            Box::new(start),
            Box::new(Expression::Multiply(
                *loc,
                Box::new(index),
                Box::new(Expression::NumberLiteral(*loc, 256, elem_size)),
            )),
        )
    }
}

/// Push() method on dynamic array in storage
pub fn storage_array_push(
    loc: &ast::Loc,
    var_expr: Expression,
    func: &ast::Identifier,
    ty: &resolver::Type,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    ns: &resolver::Contract,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let tab = match vartab {
        &mut Some(ref mut tab) => tab,
        None => {
            errors.push(Output::error(
                *loc,
                format!("cannot call method ‘{}’ in constant expression", func.name),
            ));
            return Err(());
        }
    };

    if args.len() > 1 {
        errors.push(Output::error(
            func.loc,
            "method ‘push()’ takes at most 1 argument".to_string(),
        ));
        return Err(());
    }
    // set array+length to val_expr
    let slot_ty = resolver::Type::Primitive(ast::PrimitiveType::Uint(256));
    let slot_pos = tab.temp_anonymous(&slot_ty);

    cfg.add(
        tab,
        Instr::Set {
            res: slot_pos,
            expr: Expression::StorageLoad(*loc, slot_ty.clone(), Box::new(var_expr.clone())),
        },
    );

    let elem_ty = ty.array_deref();
    let storage = array_offset(
        loc,
        Expression::Keccak256(*loc, Box::new(var_expr.clone())),
        Expression::Variable(*loc, slot_pos),
        elem_ty.clone(),
        ns,
    );

    if args.len() == 1 {
        let (val_expr, val_ty) = expression(&args[0], cfg, ns, &mut Some(tab), errors)?;

        let pos = tab.temp_anonymous(&elem_ty);

        cfg.add(
            tab,
            Instr::Set {
                res: pos,
                expr: cast(
                    &args[0].loc(),
                    val_expr,
                    &val_ty,
                    &elem_ty.deref(),
                    true,
                    ns,
                    errors,
                )?,
            },
        );

        cfg.add(
            tab,
            Instr::SetStorage {
                ty: elem_ty,
                local: pos,
                storage,
            },
        );
    } else {
        cfg.add(
            tab,
            Instr::ClearStorage {
                ty: elem_ty,
                storage,
            },
        );
    }

    // increase length
    let new_length = tab.temp_anonymous(&slot_ty);

    cfg.add(
        tab,
        Instr::Set {
            res: new_length,
            expr: Expression::Add(
                *loc,
                Box::new(Expression::Variable(*loc, slot_pos)),
                Box::new(Expression::NumberLiteral(*loc, 256, BigInt::one())),
            ),
        },
    );

    cfg.add(
        tab,
        Instr::SetStorage {
            ty: slot_ty,
            local: new_length,
            storage: var_expr,
        },
    );

    Ok((Expression::Poison, resolver::Type::Undef))
}
