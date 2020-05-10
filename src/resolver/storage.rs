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
    ns: &resolver::Namespace,
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

/// Resolve delete statement
pub fn delete(
    loc: &ast::Loc,
    var: &ast::Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let (var_expr, var_ty) = expression(var, cfg, contract_no, ns, vartab, errors)?;

    let tab = match vartab {
        &mut Some(ref mut tab) => tab,
        None => {
            errors.push(Output::error(
                *loc,
                "cannot use ‘delete’ in constant expression".to_string(),
            ));
            return Err(());
        }
    };

    if let resolver::Type::StorageRef(ty) = &var_ty {
        if ty.is_mapping() {
            errors.push(Output::error(
                *loc,
                "‘delete’ cannot be applied to mapping type".to_string(),
            ));
            return Err(());
        }

        cfg.writes_contract_storage = true;
        cfg.add(
            tab,
            Instr::ClearStorage {
                ty: ty.as_ref().clone(),
                storage: var_expr,
            },
        );
    } else {
        errors.push(Output::error(
            *loc,
            "argument to ‘delete’ should be storage reference".to_string(),
        ));
        return Err(());
    }

    Ok((Expression::Poison, resolver::Type::Undef))
}

/// Push() method on dynamic array in storage
pub fn array_push(
    loc: &ast::Loc,
    var_expr: Expression,
    func: &ast::Identifier,
    ty: &resolver::Type,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<Output>,
) -> Result<Vec<(Expression, resolver::Type)>, ()> {
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
    let slot_ty = resolver::Type::Uint(256);
    let length_pos = tab.temp_anonymous(&slot_ty);

    cfg.add(
        tab,
        Instr::Set {
            res: length_pos,
            expr: Expression::StorageLoad(*loc, slot_ty.clone(), Box::new(var_expr.clone())),
        },
    );

    let elem_ty = ty.storage_deref();

    let entry_pos = tab.temp_anonymous(&slot_ty);

    cfg.writes_contract_storage = true;
    cfg.add(
        tab,
        Instr::Set {
            res: entry_pos,
            expr: array_offset(
                loc,
                Expression::Keccak256(*loc, vec![(var_expr.clone(), slot_ty.clone())]),
                Expression::Variable(*loc, length_pos),
                elem_ty.clone(),
                ns,
            ),
        },
    );

    if args.len() == 1 {
        let (val_expr, val_ty) =
            expression(&args[0], cfg, contract_no, ns, &mut Some(tab), errors)?;

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
                ty: elem_ty.clone(),
                local: pos,
                storage: Expression::Variable(*loc, entry_pos),
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
                Box::new(Expression::Variable(*loc, length_pos)),
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

    if args.is_empty() {
        Ok(vec![(Expression::Variable(*loc, entry_pos), elem_ty)])
    } else {
        Ok(vec![(Expression::Poison, resolver::Type::Undef)])
    }
}

/// Pop() method on dynamic array in storage
pub fn array_pop(
    loc: &ast::Loc,
    var_expr: Expression,
    func: &ast::Identifier,
    ty: &resolver::Type,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<Output>,
) -> Result<Vec<(Expression, resolver::Type)>, ()> {
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

    if !args.is_empty() {
        errors.push(Output::error(
            func.loc,
            "method ‘pop()’ does not take any arguments".to_string(),
        ));
        return Err(());
    }

    // set array+length to val_expr
    let slot_ty = resolver::Type::Uint(256);
    let length_pos = tab.temp_anonymous(&slot_ty);

    cfg.add(
        tab,
        Instr::Set {
            res: length_pos,
            expr: Expression::StorageLoad(*loc, slot_ty.clone(), Box::new(var_expr.clone())),
        },
    );

    let empty_array = cfg.new_basic_block("empty_array".to_string());
    let has_elements = cfg.new_basic_block("has_elements".to_string());

    cfg.writes_contract_storage = true;
    cfg.add(
        tab,
        Instr::BranchCond {
            cond: Expression::Equal(
                *loc,
                Box::new(Expression::Variable(*loc, length_pos)),
                Box::new(Expression::NumberLiteral(*loc, 256, BigInt::zero())),
            ),
            true_: empty_array,
            false_: has_elements,
        },
    );

    cfg.set_basic_block(empty_array);
    cfg.add(tab, Instr::AssertFailure { expr: None });

    cfg.set_basic_block(has_elements);
    let new_length = tab.temp_anonymous(&slot_ty);

    cfg.add(
        tab,
        Instr::Set {
            res: new_length,
            expr: Expression::Subtract(
                *loc,
                Box::new(Expression::Variable(*loc, length_pos)),
                Box::new(Expression::NumberLiteral(*loc, 256, BigInt::one())),
            ),
        },
    );

    // The array element will be loaded before clearing. So, the return
    // type of pop() is the derefenced array dereference
    let elem_ty = ty.storage_deref().deref().clone();
    let entry_pos = tab.temp_anonymous(&slot_ty);

    cfg.add(
        tab,
        Instr::Set {
            res: entry_pos,
            expr: array_offset(
                loc,
                Expression::Keccak256(*loc, vec![(var_expr.clone(), slot_ty.clone())]),
                Expression::Variable(*loc, new_length),
                elem_ty.clone(),
                ns,
            ),
        },
    );

    let res_pos = tab.temp_anonymous(&elem_ty);

    cfg.add(
        tab,
        Instr::Set {
            res: res_pos,
            expr: Expression::StorageLoad(
                *loc,
                elem_ty.clone(),
                Box::new(Expression::Variable(*loc, entry_pos)),
            ),
        },
    );

    cfg.add(
        tab,
        Instr::ClearStorage {
            ty: elem_ty.clone(),
            storage: Expression::Variable(*loc, entry_pos),
        },
    );

    // set decrease length
    cfg.add(
        tab,
        Instr::SetStorage {
            ty: slot_ty,
            local: new_length,
            storage: var_expr,
        },
    );

    Ok(vec![(Expression::Variable(*loc, res_pos), elem_ty)])
}

/// Push() method on dynamic bytes in storage
pub fn bytes_push(
    loc: &ast::Loc,
    var_expr: Expression,
    func: &ast::Identifier,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<Output>,
) -> Result<Vec<(Expression, resolver::Type)>, ()> {
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

    cfg.writes_contract_storage = true;

    let val = match args.len() {
        0 => Expression::NumberLiteral(*loc, 8, BigInt::zero()),
        1 => {
            let (val_expr, val_ty) =
                expression(&args[0], cfg, contract_no, ns, &mut Some(tab), errors)?;

            cast(
                &args[0].loc(),
                val_expr,
                &val_ty,
                &resolver::Type::Bytes(1),
                true,
                ns,
                errors,
            )?
        }
        _ => {
            errors.push(Output::error(
                func.loc,
                "method ‘push()’ takes at most 1 argument".to_string(),
            ));
            return Err(());
        }
    };

    if args.is_empty() {
        Ok(vec![(
            Expression::StorageBytesPush(*loc, Box::new(var_expr), Box::new(val)),
            resolver::Type::Bytes(1),
        )])
    } else {
        Ok(vec![(
            Expression::StorageBytesPush(*loc, Box::new(var_expr), Box::new(val)),
            resolver::Type::Undef,
        )])
    }
}

/// Pop() method on dynamic bytes in storage
pub fn bytes_pop(
    loc: &ast::Loc,
    var_expr: Expression,
    func: &ast::Identifier,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    errors: &mut Vec<Output>,
) -> Result<Vec<(Expression, resolver::Type)>, ()> {
    cfg.writes_contract_storage = true;

    if !args.is_empty() {
        errors.push(Output::error(
            func.loc,
            "method ‘pop()’ does not take any arguments".to_string(),
        ));
        return Err(());
    }

    Ok(vec![(
        Expression::StorageBytesPop(*loc, Box::new(var_expr)),
        resolver::Type::Bytes(1),
    )])
}

/// Calculate storage subscript
pub fn mapping_subscript(
    loc: &ast::Loc,
    mapping: Expression,
    mapping_ty: &resolver::Type,
    index: &ast::Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let (key_ty, value_ty) = match mapping_ty.deref() {
        resolver::Type::Mapping(k, v) => (k, v),
        _ => unreachable!(),
    };

    let (index_expr, index_ty) = expression(index, cfg, contract_no, ns, vartab, errors)?;

    let index_expr = cast(
        &index.loc(),
        index_expr,
        &index_ty,
        key_ty,
        true,
        ns,
        errors,
    )?;

    let slot_ty = resolver::Type::Uint(256);

    let index_ty = if let resolver::Type::Enum(n) = index_ty {
        ns.enums[n].ty.clone()
    } else {
        index_ty
    };

    let slot = Expression::Keccak256(*loc, vec![(mapping, slot_ty), (index_expr, index_ty)]);

    Ok((slot, resolver::Type::StorageRef(value_ty.clone())))
}
