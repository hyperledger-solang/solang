// SPDX-License-Identifier: Apache-2.0

use crate::lir::vartable::Vartable;

use super::{expressions::Operand, ssa_type::Type};

pub mod printer;
pub mod expression;
pub mod instruction;

pub struct Printer {
    vartable: Box<Vartable>,
}

impl Printer {

    pub fn new(vartable: Box<Vartable>) -> Self {
        Self {
            vartable,
        }
    }

    pub(crate) fn get_var_name(&self, id: &usize) -> &str {
        self.vartable.get_name(id)
    }

    pub(crate) fn get_var_type(&self, id: &usize) -> &Type {
        self.vartable.get_type(id)
    }

    pub(crate) fn get_var_operand(&self, id: &usize) -> Operand {
        self.vartable.get_operand(
            id,
            // the location is not important for printing
            solang_parser::pt::Loc::Codegen,
        )
    }
}
