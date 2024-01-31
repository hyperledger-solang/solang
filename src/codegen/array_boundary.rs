// SPDX-License-Identifier: Apache-2.0

use super::vartable::Vartable;
use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::Expression;
use crate::sema::ast::Type;
use solang_parser::pt::Loc;

/// This function is called whenever an assignment statement of an array is encountered. We have to ensure
/// that the variable number of the array is tied to the variable number of the correct temporary variable.
/// We have two cases:
/// Case 1: If the right hand side is an array, the left array keeps track of the temp variable of the right array (if a temp variable exists)
/// Case 2: If reallocation of the left array is done (with a new expression), we create a new temp variable and the left side tracks it.
/// If that's the case, we return an AllocDynamicArray expression with the size being the temp variable to avoid repetitive expressions in the cfg.
pub(crate) fn handle_array_assign(
    right: Expression,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    pos: usize,
) -> Expression {
    if let Expression::AllocDynamicBytes {
        loc,
        ty: ty @ Type::Array(..),
        size,
        initializer,
    } = right
    {
        // If we re-allocate the pointer, create a new temp variable to hold the new array length
        let temp_res = vartab.temp_name("array_length", &Type::Uint(32));

        cfg.add(
            vartab,
            Instr::Set {
                loc: Loc::Codegen,
                res: temp_res,
                expr: *size,
            },
        );

        cfg.array_lengths_temps.insert(pos, temp_res);

        Expression::AllocDynamicBytes {
            loc,
            ty,
            size: Box::new(Expression::Variable {
                loc: Loc::Codegen,
                ty: Type::Uint(32),
                var_no: temp_res,
            }),
            initializer,
        }
    } else {
        if let Expression::Variable {
            var_no: right_res, ..
        } = &right
        {
            // If we have initialized a temp var for this var
            if cfg.array_lengths_temps.contains_key(right_res) {
                let to_update = cfg.array_lengths_temps[right_res];

                cfg.array_lengths_temps.insert(pos, to_update);
            } else {
                // If the right hand side doesn't have a temp, it must be a function parameter or a struct member.
                cfg.array_lengths_temps.swap_remove(&pos);
            }
        }

        right.clone()
    }
}
