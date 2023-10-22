// SPDX-License-Identifier: Apache-2.0
use crate::{
    codegen::{
        self,
        cfg::{self, ControlFlowGraph},
    },
    sema::ast::{self, Namespace, RetrieveType},
};

use super::{
    expr::Operand,
    insn::Insn,
    ssa_type::{InternalCallTy, Type},
    typechecker::TypeChecker,
    vartable::Vartable,
};

mod cfg_converter;
mod expr_converter;
mod insn_converter;
mod type_converter;
mod vartable_converter;

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

    pub fn as_operand(&self, expr: &codegen::Expression) -> Option<Operand> {
        match expr {
            codegen::Expression::NumberLiteral { ty, value, loc, .. } => {
                let ssa_ty = self.from_ast_type(ty).unwrap();
                Some(Operand::new_number_literal(value, ssa_ty, loc.clone()))
            }
            codegen::Expression::BoolLiteral { value, loc, .. } => {
                Some(Operand::new_bool_literal(*value, loc.clone()))
            }
            codegen::Expression::Variable { loc, ty, var_no } => {
                let var_ty = self.get_ast_type_by_id(var_no).unwrap();
                TypeChecker::assert_ty_eq(&var_ty, ty).unwrap();
                Some(Operand::new_id(var_no.clone(), loc.clone()))
            }
            _ => None,
        }
    }

    pub fn as_operand_and_insns(
        &self,
        expr: &codegen::Expression,
        vartable: &mut Vartable,
    ) -> Result<(Operand, Vec<Insn>), String> {
        match self.as_operand(expr) {
            Some(op) => Ok((op, vec![])),
            None => {
                let tmp = vartable.new_temp(&self.from_ast_type(&expr.ty())?);
                let dest_insns = self.from_expression(&tmp, expr, vartable)?;
                Ok((tmp, dest_insns))
            }
        }
    }

    pub fn as_operand_option_and_insns(
        &self,
        expr: &Option<codegen::Expression>,
        vartable: &mut Vartable,
    ) -> Result<(Option<Operand>, Vec<Insn>), String> {
        match expr {
            Some(address) => {
                let (tmp, expr_insns) = self.as_operand_and_insns(address, vartable)?;
                Ok((Some(tmp), expr_insns))
            }
            None => Ok((None, vec![])),
        }
    }

    pub fn as_string_location_and_insns(
        &self,
        location: &ast::StringLocation<codegen::Expression>,
        vartable: &mut Vartable,
    ) -> Result<(ast::StringLocation<Operand>, Vec<Insn>), String> {
        match location {
            ast::StringLocation::CompileTime(str) => Ok((
                ast::StringLocation::CompileTime(str.clone()) as ast::StringLocation<Operand>,
                vec![],
            )),
            ast::StringLocation::RunTime(expr) => {
                let (op, insns) = self.as_operand_and_insns(expr, vartable)?;
                Ok((ast::StringLocation::RunTime(Box::new(op)), insns))
            }
        }
    }

    pub fn as_external_call_accounts_and_insns(
        &self,
        accounts: &ast::ExternalCallAccounts<codegen::Expression>,
        vartable: &mut Vartable,
    ) -> Result<(ast::ExternalCallAccounts<Operand>, Vec<Insn>), String> {
        match accounts {
            ast::ExternalCallAccounts::Present(accounts) => {
                let (tmp, expr_insns) = self.as_operand_and_insns(&accounts, vartable)?;
                Ok((ast::ExternalCallAccounts::Present(tmp), expr_insns))
            }
            ast::ExternalCallAccounts::NoAccount => Ok((
                ast::ExternalCallAccounts::NoAccount as ast::ExternalCallAccounts<Operand>,
                vec![],
            )),
            ast::ExternalCallAccounts::AbsentArgument => Ok((
                ast::ExternalCallAccounts::AbsentArgument as ast::ExternalCallAccounts<Operand>,
                vec![],
            )),
        }
    }

    pub fn as_internal_call_ty_and_insns(
        &self,
        call: &cfg::InternalCallTy,
        vartable: &mut Vartable,
    ) -> Result<(InternalCallTy, Vec<Insn>), String> {
        match call {
            cfg::InternalCallTy::Builtin { ast_func_no } => Ok((
                InternalCallTy::Builtin {
                    ast_func_no: *ast_func_no,
                },
                vec![],
            )),
            cfg::InternalCallTy::Static { cfg_no } => {
                Ok((InternalCallTy::Static { cfg_no: *cfg_no }, vec![]))
            }
            cfg::InternalCallTy::Dynamic(expr) => {
                let (tmp, expr_insns) = self.as_operand_and_insns(expr, vartable)?;
                Ok((InternalCallTy::Dynamic(tmp), expr_insns))
            }
        }
    }
}
