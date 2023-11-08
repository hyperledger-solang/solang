// SPDX-License-Identifier: Apache-2.0
use crate::{
    codegen::{
        self,
        cfg::{self, ControlFlowGraph},
    },
    sema::ast::{self, Namespace, RetrieveType},
};

use super::{
    expressions::Operand,
    instructions::Instruction,
    ssa_type::{InternalCallTy, Type},
    vartable::Vartable,
};

mod converter;
mod expression;
mod instruction;
mod ssa_type;
mod vartable;

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

    pub fn convert_user_type(&self, user_ty: &ast::Type) -> Result<Type, String> {
        // clone happens here because function unwrap_user_type takes ownership
        let real_ty = user_ty.clone().unwrap_user_type(self.ns);
        self.from_ast_type(&real_ty)
    }

    pub fn convert_enum_type(&self, enum_no: usize) -> Result<Type, String> {
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

    pub fn to_operand(
        &self,
        expr: &codegen::Expression,
        vartable: &mut Vartable,
    ) -> Option<Operand> {
        match expr {
            codegen::Expression::NumberLiteral { ty, value, loc, .. } => {
                let ssa_ty = self.from_ast_type(ty).unwrap();
                Some(Operand::new_number_literal(value, ssa_ty, *loc))
            }
            codegen::Expression::BoolLiteral { value, loc, .. } => {
                Some(Operand::new_bool_literal(*value, *loc))
            }
            codegen::Expression::Variable { loc, var_no, .. } => {
                Some(Operand::new_id(*var_no, *loc))
            }
            codegen::Expression::FunctionArg { loc, arg_no, .. } => {
                vartable.get_function_arg(*arg_no, *loc)
            }
            _ => None,
        }
    }

    pub fn to_operand_and_insns(
        &self,
        expr: &codegen::Expression,
        vartable: &mut Vartable,
        mut result: &mut Vec<Instruction>,
    ) -> Result<Operand, String> {
        match self.to_operand(expr, vartable) {
            Some(op) => Ok(op),
            None => {
                let tmp = vartable.new_temp(&self.from_ast_type(&expr.ty())?);
                self.lowering_expression(&tmp, expr, vartable, &mut result)?;
                Ok(tmp)
            }
        }
    }

    pub fn to_operand_option_and_insns(
        &self,
        expr: &Option<codegen::Expression>,
        vartable: &mut Vartable,
        result: &mut Vec<Instruction>,
    ) -> Result<Option<Operand>, String> {
        match expr {
            Some(address) => {
                let tmp = self.to_operand_and_insns(address, vartable, result)?;
                Ok(Some(tmp))
            }
            None => Ok(None),
        }
    }

    pub fn to_string_location_and_insns(
        &self,
        location: &ast::StringLocation<codegen::Expression>,
        vartable: &mut Vartable,
        result: &mut Vec<Instruction>,
    ) -> Result<ast::StringLocation<Operand>, String> {
        match location {
            ast::StringLocation::CompileTime(str) => {
                Ok(ast::StringLocation::CompileTime(str.clone()) as ast::StringLocation<Operand>)
            }
            ast::StringLocation::RunTime(expr) => {
                let op = self.to_operand_and_insns(expr, vartable, result)?;
                Ok(ast::StringLocation::RunTime(Box::new(op)))
            }
        }
    }

    pub fn to_external_call_accounts_and_insns(
        &self,
        accounts: &ast::ExternalCallAccounts<codegen::Expression>,
        vartable: &mut Vartable,
        result: &mut Vec<Instruction>,
    ) -> Result<ast::ExternalCallAccounts<Operand>, String> {
        match accounts {
            ast::ExternalCallAccounts::Present(accounts) => {
                let tmp = self.to_operand_and_insns(accounts, vartable, result)?;
                Ok(ast::ExternalCallAccounts::Present(tmp))
            }
            ast::ExternalCallAccounts::NoAccount => {
                Ok(ast::ExternalCallAccounts::NoAccount as ast::ExternalCallAccounts<Operand>)
            }
            ast::ExternalCallAccounts::AbsentArgument => {
                Ok(ast::ExternalCallAccounts::AbsentArgument as ast::ExternalCallAccounts<Operand>)
            }
        }
    }

    pub fn to_internal_call_ty_and_insns(
        &self,
        call: &cfg::InternalCallTy,
        vartable: &mut Vartable,
        result: &mut Vec<Instruction>,
    ) -> Result<InternalCallTy, String> {
        match call {
            cfg::InternalCallTy::Builtin { ast_func_no } => Ok(InternalCallTy::Builtin {
                ast_func_no: *ast_func_no,
            }),
            cfg::InternalCallTy::Static { cfg_no } => {
                Ok(InternalCallTy::Static { cfg_no: *cfg_no })
            }
            cfg::InternalCallTy::Dynamic(expr) => {
                let tmp = self.to_operand_and_insns(expr, vartable, result)?;
                Ok(InternalCallTy::Dynamic(tmp))
            }
        }
    }
}
