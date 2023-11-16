// SPDX-License-Identifier: Apache-2.0

use crate::lir::expressions::Operand;
use crate::lir::lir_type::Type;
use crate::sema::ast;
use indexmap::IndexMap;
use solang_parser::pt::Loc;

/// a constant prefix for temporary variables
pub const TEMP_PREFIX: &str = "temp.ssa_ir.";

#[derive(Debug, Clone)]
pub struct Var {
    pub id: usize,
    pub ty: Type,
    pub ast_ty: ast::Type,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Vartable {
    pub vars: IndexMap<usize, Var>,
    pub args: IndexMap</* arg no */ usize, /* var id */ usize>,
    pub next_id: usize,
}

impl Vartable {
    pub(crate) fn get_type(&self, id: &usize) -> &Type {
        self.vars
            .get(id)
            .map(|var| &var.ty)
            .ok_or(format!("Variable {} not found.", id))
            .unwrap()
    }

    pub(crate) fn get_name(&self, id: &usize) -> &str {
        self.vars
            .get(id)
            .map(|var| var.name.as_str())
            .ok_or(format!("Variable {} not found.", id))
            .unwrap()
    }

    pub(crate) fn get_operand(&self, id: &usize, loc: Loc) -> Operand {
        self.vars
            .get(id)
            .map(|var| Operand::Id { id: var.id, loc })
            .ok_or(format!("Variable {} not found.", id))
            .unwrap()
    }

    pub fn set_tmp(&mut self, id: usize, ty: Type, ast_ty: ast::Type) {
        let var = Var {
            id,
            ty,
            ast_ty,
            name: format!("{}{}", TEMP_PREFIX, id),
        };
        self.next_id = self.next_id.max(id + 1);
        self.vars.insert(id, var);
    }

    pub(crate) fn new_temp(&mut self, ty: Type, ast_ty: ast::Type) -> Operand {
        let name = format!("{}{}", TEMP_PREFIX, self.next_id);
        let var = Var {
            id: self.next_id,
            ty,
            ast_ty,
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

    pub(crate) fn get_function_arg(&self, arg_no: usize, loc: Loc) -> Option<Operand> {
        match self.args.get(&arg_no) {
            Some(id) => {
                let op = self.get_operand(id, loc);
                Some(op)
            }
            None => None,
        }
    }

    pub(crate) fn add_function_arg(&mut self, arg_no: usize, var_id: usize) {
        self.args.insert(arg_no, var_id);
    }
}
