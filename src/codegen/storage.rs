use num_bigint::BigInt;
use num_traits::FromPrimitive;
use num_traits::One;
use num_traits::Zero;

use super::cfg::{ControlFlowGraph, Instr, Vartable};
use super::expression::expression;
use crate::parser::pt;
use crate::sema::ast::{Expression, Namespace, Type};

/// Given a storage slot which is the start of the array, calculate the
/// offset of the array element. This function exists to avoid doing
/// 256 bit multiply if possible.
pub fn array_offset(
    loc: &pt::Loc,
    start: Expression,
    index: Expression,
    elem_ty: Type,
    ns: &Namespace,
) -> Expression {
    let elem_size = elem_ty.storage_slots(ns);
    let slot_ty = ns.storage_type();

    // the index needs to be cast to i256 and multiplied by the number
    // of slots for each element
    if elem_size == BigInt::one() {
        Expression::Add(*loc, slot_ty, Box::new(start), Box::new(index))
    } else if (elem_size.clone() & (elem_size.clone() - BigInt::one())) == BigInt::zero() {
        // elem_size is power of 2
        Expression::ShiftLeft(
            *loc,
            slot_ty.clone(),
            Box::new(start),
            Box::new(Expression::ShiftLeft(
                *loc,
                slot_ty.clone(),
                Box::new(index),
                Box::new(Expression::NumberLiteral(
                    *loc,
                    slot_ty,
                    BigInt::from_u64(elem_size.bits()).unwrap(),
                )),
            )),
        )
    } else {
        Expression::Add(
            *loc,
            slot_ty.clone(),
            Box::new(start),
            Box::new(Expression::Multiply(
                *loc,
                slot_ty.clone(),
                Box::new(index),
                Box::new(Expression::NumberLiteral(*loc, slot_ty, elem_size)),
            )),
        )
    }
}

/// Push() method on dynamic array in storage
pub fn array_push(
    loc: &pt::Loc,
    args: &[Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
) -> Expression {
    // set array+length to val_expr
    let slot_ty = ns.storage_type();
    let length_pos = vartab.temp_anonymous(&slot_ty);

    let var_expr = expression(&args[0], cfg, contract_no, ns, vartab);

    cfg.add(
        vartab,
        Instr::Set {
            res: length_pos,
            expr: Expression::StorageLoad(*loc, slot_ty.clone(), Box::new(var_expr.clone())),
        },
    );

    let elem_ty = args[0].ty().storage_array_elem();

    let entry_pos = vartab.temp_anonymous(&slot_ty);

    cfg.add(
        vartab,
        Instr::Set {
            res: entry_pos,
            expr: array_offset(
                loc,
                Expression::Keccak256(*loc, slot_ty.clone(), vec![var_expr.clone()]),
                Expression::Variable(*loc, slot_ty.clone(), length_pos),
                elem_ty.clone(),
                ns,
            ),
        },
    );

    if args.len() == 2 {
        let value = expression(&args[1], cfg, contract_no, ns, vartab);

        cfg.add(
            vartab,
            Instr::SetStorage {
                ty: elem_ty.clone(),
                value,
                storage: Expression::Variable(*loc, slot_ty.clone(), entry_pos),
            },
        );
    }

    // increase length
    let new_length = Expression::Add(
        *loc,
        slot_ty.clone(),
        Box::new(Expression::Variable(*loc, slot_ty.clone(), length_pos)),
        Box::new(Expression::NumberLiteral(
            *loc,
            slot_ty.clone(),
            BigInt::one(),
        )),
    );

    cfg.add(
        vartab,
        Instr::SetStorage {
            ty: slot_ty,
            value: new_length,
            storage: var_expr,
        },
    );

    if args.len() == 1 {
        Expression::Variable(*loc, elem_ty, entry_pos)
    } else {
        Expression::Poison
    }
}

/// Pop() method on dynamic array in storage
pub fn array_pop(
    loc: &pt::Loc,
    args: &[Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
) -> Expression {
    // set array+length to val_expr
    let slot_ty = ns.storage_type();
    let length_ty = ns.storage_type();
    let length_pos = vartab.temp_anonymous(&slot_ty);

    let ty = args[0].ty();
    let var_expr = expression(&args[0], cfg, contract_no, ns, vartab);

    cfg.add(
        vartab,
        Instr::Set {
            res: length_pos,
            expr: Expression::StorageLoad(*loc, length_ty.clone(), Box::new(var_expr.clone())),
        },
    );

    let empty_array = cfg.new_basic_block("empty_array".to_string());
    let has_elements = cfg.new_basic_block("has_elements".to_string());

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: Expression::Equal(
                *loc,
                Box::new(Expression::Variable(*loc, length_ty.clone(), length_pos)),
                Box::new(Expression::NumberLiteral(
                    *loc,
                    length_ty.clone(),
                    BigInt::zero(),
                )),
            ),
            true_: empty_array,
            false_: has_elements,
        },
    );

    cfg.set_basic_block(empty_array);
    cfg.add(vartab, Instr::AssertFailure { expr: None });

    cfg.set_basic_block(has_elements);
    let new_length = vartab.temp_anonymous(&slot_ty);

    cfg.add(
        vartab,
        Instr::Set {
            res: new_length,
            expr: Expression::Subtract(
                *loc,
                length_ty.clone(),
                Box::new(Expression::Variable(*loc, length_ty.clone(), length_pos)),
                Box::new(Expression::NumberLiteral(*loc, length_ty, BigInt::one())),
            ),
        },
    );

    // The array element will be loaded before clearing. So, the return
    // type of pop() is the derefenced array dereference
    let elem_ty = ty.storage_array_elem().deref_any().clone();
    let entry_pos = vartab.temp_anonymous(&slot_ty);

    cfg.add(
        vartab,
        Instr::Set {
            res: entry_pos,
            expr: array_offset(
                loc,
                Expression::Keccak256(*loc, slot_ty.clone(), vec![var_expr.clone()]),
                Expression::Variable(*loc, slot_ty.clone(), new_length),
                elem_ty.clone(),
                ns,
            ),
        },
    );

    let res_pos = vartab.temp_anonymous(&elem_ty);

    cfg.add(
        vartab,
        Instr::Set {
            res: res_pos,
            expr: Expression::StorageLoad(
                *loc,
                elem_ty.clone(),
                Box::new(Expression::Variable(*loc, elem_ty.clone(), entry_pos)),
            ),
        },
    );

    cfg.add(
        vartab,
        Instr::ClearStorage {
            ty: elem_ty.clone(),
            storage: Expression::Variable(*loc, slot_ty.clone(), entry_pos),
        },
    );

    // set decrease length
    cfg.add(
        vartab,
        Instr::SetStorage {
            ty: slot_ty.clone(),
            value: Expression::Variable(*loc, slot_ty, new_length),
            storage: var_expr,
        },
    );

    Expression::Variable(*loc, elem_ty, res_pos)
}

/// Push() method on dynamic bytes in storage
pub fn bytes_push(
    loc: &pt::Loc,
    args: &[Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
) -> Expression {
    let var_expr = expression(&args[0], cfg, contract_no, ns, vartab);

    if args.len() > 1 {
        let val = expression(&args[1], cfg, contract_no, ns, vartab);

        Expression::StorageBytesPush(*loc, Box::new(var_expr), Box::new(val))
    } else {
        Expression::StorageBytesPush(
            *loc,
            Box::new(var_expr),
            Box::new(Expression::NumberLiteral(
                *loc,
                Type::Bytes(1),
                BigInt::zero(),
            )),
        )
    }
}

/// Pop() method on dynamic bytes in storage
pub fn bytes_pop(
    loc: &pt::Loc,
    args: &[Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
) -> Expression {
    let var_expr = expression(&args[0], cfg, contract_no, ns, vartab);

    Expression::StorageBytesPop(*loc, Box::new(var_expr))
}
