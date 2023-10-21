// SPDX-License-Identifier: Apache-2.0

use crate::codegen::vartable::Storage;
use crate::ssa_ir::expr::Operand;
use crate::ssa_ir::ssa_type::Type;
use indexmap::IndexMap;

#[derive(Debug)]
pub struct Var {
    id: usize,
    ty: Type,
    name: String,
    storage: Storage,
}

#[derive(Debug)]
pub struct Vartable {
    pub vars: IndexMap<usize, Var>,
    pub next_id: usize,
}

impl Var {
    pub(crate) fn new(id: usize, ty: Type, name: String, storage: Storage) -> Self {
        Self {
            id,
            ty,
            name,
            storage,
        }
    }
}

impl Vartable {
    pub(crate) fn get_type(&self, id: &usize) -> Result<&Type, String> {
        self.vars
            .get(id)
            .map(|var| &var.ty)
            .ok_or("Variable not found".to_string())
    }

    pub(crate) fn get_name(&self, id: &usize) -> Result<&str, String> {
        self.vars
            .get(id)
            .map(|var| var.name.as_str())
            .ok_or("Variable not found".to_string())
    }

    pub(crate) fn get_operand(&self, id: &usize) -> Result<Operand, String> {
        self.vars
            .get(id)
            .map(|var| Operand::Id { id: var.id })
            .ok_or("Variable not found".to_string())
    }

    pub(crate) fn new_temp(&mut self, ty: &Type) -> Operand {
        self.next_id += 1;

        let name = format!("temp.{}", self.next_id);
        let var = Var {
            id: self.next_id,
            ty: ty.clone(),
            name: name.clone(),
            storage: Storage::Local,
        };

        self.vars.insert(self.next_id, var);

        Operand::Id { id: self.next_id }
    }
}
