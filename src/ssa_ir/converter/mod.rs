// SPDX-License-Identifier: Apache-2.0
use crate::{
    codegen::cfg::ControlFlowGraph,
    sema::ast::{self, Namespace},
};

use super::ssa_type::Type;

mod block_converter;
mod vartable_converter;
mod cfg_converter;
mod expr_converter;
mod insn_converter;
mod parameter_converter;
mod type_converter;

pub struct Converter<'a> {
    // namespace
    ns: &'a Namespace,
    cfg: &'a ControlFlowGraph,
}

impl<'input> Converter<'input> {
    pub fn new(ns: &'input Namespace, cfg: &'input ControlFlowGraph) -> Self {
        Self { ns, cfg }
    }

    pub fn fn_selector_length(&self) -> u8 {
        self.ns.target.selector_length()
    }

    pub fn address_length(&self) -> usize {
        self.ns.address_length
    }

    pub fn unwrap_user_type(&self, user_ty: &ast::Type) -> Result<Type, String> {
        // clone happens here because function unwrap_user_type takes ownership
        let real_ty = user_ty.clone().unwrap_user_type(self.ns);
        self.from_ast_type(&real_ty)
    }

    pub fn get_enum_type(&self, enum_no: usize) -> Result<Type, String> {
        let ty = &self.ns.enums[enum_no].ty;
        self.from_ast_type(ty)
    }

    pub fn value_length(&self) -> usize {
        self.ns.value_length
    }

    pub fn get_ast_type_by_id(&self, id: &usize) -> Result<ast::Type, String> {
        match self.cfg.vars.get(id) {
            Some(var) => Ok(var.ty.clone()),
            None => Err(format!("Cannot find type for id {}", id)),
        }
    }
}
