// SPDX-License-Identifier: Apache-2.0

use crate::lir::expressions::Operand;
use indexmap::IndexMap;
use solang_parser::pt::Loc;

use super::lir_type::LIRType;

/// a constant prefix for temporary variables
pub const TEMP_PREFIX: &str = "temp.ssa_ir.";

/// The `Var` struct represents a variable in the Lower Intermediate Representation.
/// It contains the variable's unique identifier, its type, and its name.
#[derive(Debug, Clone)]
pub struct Var {
    /// The unique identifier of the variable.
    pub id: usize,
    /// The type of the variable.
    pub ty: LIRType,
    /// The name of the variable.
    pub name: String,
}

/// The `Vartable` struct represents a table of variables in the Lower Intermediate Representation.
/// It holds a map of variables, a map of function arguments, and the next variable identifier.
#[derive(Debug, Clone)]
pub struct Vartable {
    /// The map of variables
    /// that contains the variable's unique identifier, its type, and its name.
    pub vars: IndexMap<usize, Var>,
    /// The map of function arguments
    pub args: IndexMap</* arg no */ usize, /* var id */ usize>,
    /// The next variable identifier.
    pub next_id: usize,
}

impl Vartable {
    /// Get the type of a variable by its unique identifier.
    pub(crate) fn get_type(&self, id: &usize) -> &LIRType {
        self.vars
            .get(id)
            .map(|var| &var.ty)
            .ok_or(format!("Variable {} not found.", id))
            .unwrap()
    }

    /// Get the name of a variable by its unique identifier.
    pub(crate) fn get_name(&self, id: &usize) -> &str {
        self.vars
            .get(id)
            .map(|var| var.name.as_str())
            .ok_or(format!("Variable {} not found.", id))
            .unwrap()
    }

    /// Get the operand of a variable by its unique identifier.
    pub(crate) fn get_operand(&self, id: &usize, loc: Loc) -> Operand {
        self.vars
            .get(id)
            .map(|var| Operand::Id { id: var.id, loc })
            .ok_or(format!("Variable {} not found.", id))
            .unwrap()
    }

    /// Set a temporary variable by its unique identifier.
    pub fn set_tmp(&mut self, id: usize, ty: LIRType) {
        let var = Var {
            id,
            ty,
            name: format!("{}{}", TEMP_PREFIX, id),
        };
        self.next_id = self.next_id.max(id + 1);
        self.vars.insert(id, var);
    }

    /// Create a new temporary variable.
    pub(crate) fn new_temp(&mut self, ty: LIRType) -> Operand {
        let name = format!("{}{}", TEMP_PREFIX, self.next_id);
        let var = Var {
            id: self.next_id,
            ty,
            name: name.clone(),
        };
        self.vars.insert(self.next_id, var);
        let op = Operand::Id {
            id: self.next_id,
            loc: Loc::Codegen,
        };
        self.next_id += 1;
        op
    }

    /// Get the Operand of a function argument by its argument number.
    pub(crate) fn get_function_arg(&self, arg_no: usize, loc: Loc) -> Option<Operand> {
        match self.args.get(&arg_no) {
            Some(id) => {
                let op = self.get_operand(id, loc);
                Some(op)
            }
            None => None,
        }
    }

    /// Add a function argument to the Vartable.
    pub(crate) fn add_function_arg(&mut self, arg_no: usize, var_id: usize) {
        self.args.insert(arg_no, var_id);
    }
}
