// SPDX-License-Identifier: Apache-2.0
use indexmap::IndexMap;

use crate::{
    codegen::vartable::Vars,
    ssa_ir::vartable::{Var, Vartable},
};

use super::Converter;

impl Converter<'_> {
    pub fn from_vars(&self, tab: &Vars) -> Result<Vartable, String> {
        let mut vars = IndexMap::new();
        let mut max_id = 0;
        for (id, var) in tab {
            vars.insert(
                *id,
                Var::new(
                    *id,
                    self.from_ast_type(&var.ty)?,
                    var.id.name.clone(),
                    var.storage.clone(),
                ),
            );
            if *id > max_id {
                max_id = *id;
            }
        }

        Ok(Vartable {
            vars,
            next_id: max_id + 1,
        })
    }
}
