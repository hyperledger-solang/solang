// SPDX-License-Identifier: Apache-2.0

use crate::ssa_ir::vartable::Vartable;

use super::{expressions::Operand, ssa_type::Type};

pub mod printer;
pub mod expression;
pub mod instruction;

pub struct Printer {
    pub vartable: Box<Vartable>,
}

impl Printer {
    /// For testing purpose
    pub fn set_tmp_var(&mut self, id: usize, ty: &Type) {
        self.vartable.set_tmp(id, ty)
    }

    pub(crate) fn get_var_name(&self, id: &usize) -> Result<&str, String> {
        self.vartable.get_name(id)
    }

    pub(crate) fn get_var_type(&self, id: &usize) -> Result<&Type, String> {
        self.vartable.get_type(id)
    }

    pub(crate) fn get_var_operand(&self, id: &usize) -> Result<Operand, String> {
        self.vartable.get_operand(
            id,
            // the location is not important for printing
            solang_parser::pt::Loc::Codegen,
        )
    }
}
