// SPDX-License-Identifier: Apache-2.0
use indexmap::IndexMap;

use crate::{
    codegen::vartable::Vars,
    lir::vartable::{Var, Vartable},
};

use super::Converter;

impl Converter<'_> {
    pub fn from_vars(&self, tab: &Vars) -> Vartable {
        let mut vars = IndexMap::new();
        let mut max_id = 0;
        for (id, var) in tab {
            vars.insert(
                *id,
                Var::new(
                    *id,
                    self.from_ast_type(&var.ty),
                    var.id.name.clone(),
                ),
            );
            max_id = max_id.max(*id);
        }

        Vartable {
            vars,
            args: IndexMap::new(),
            next_id: max_id + 1,
        }
    }
}
