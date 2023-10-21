// SPDX-License-Identifier: Apache-2.0

use crate::{sema::ast, ssa_ir::ssa_type::Parameter};

use super::Converter;

impl Converter<'_> {
    pub fn from_ast_parameter(&self, param: &ast::Parameter) -> Result<Parameter, String> {
        let ty = self.from_ast_type(&param.ty)?;
        Ok(Parameter {
            loc: param.loc,
            id: param.id.clone(),
            ty,
            ty_loc: param.ty_loc,
            indexed: param.indexed,
            readonly: param.readonly,
            infinite_size: param.infinite_size,
            recursive: param.recursive,
            annotation: param.annotation.clone(),
        })
    }
}
