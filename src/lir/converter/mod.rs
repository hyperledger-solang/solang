// SPDX-License-Identifier: Apache-2.0
use crate::codegen::cfg::BasicBlock;
use crate::lir::{Block, LIR};
use crate::{
    codegen::{
        self,
        cfg::{self, ControlFlowGraph},
    },
    sema::ast::{self, Namespace, Parameter, RetrieveType},
};

use super::lir_type::LIRType;
use super::{
    expressions::Operand, instructions::Instruction, lir_type::InternalCallTy, vartable::Vartable,
};

mod expression;
mod instruction;
mod lir_type;
mod vartable;

/// A Converter converts a control flow graph into a Lower Intermediate Representation
/// that has three-address code format.
pub struct Converter<'a> {
    /// a reference to the Namespace is used to retrieve useful information
    /// like enum types, address length, etc.
    ns: &'a Namespace,
    /// a reference to the ControlFlowGraph is used to retrieve the instructions.
    cfg: &'a ControlFlowGraph,
}

impl<'input> Converter<'input> {
    /// Create a new Converter with a reference to the Namespace and the ControlFlowGraph.
    pub fn new(ns: &'input Namespace, cfg: &'input ControlFlowGraph) -> Self {
        Self { ns, cfg }
    }

    /// get the selector length from the Namespace.
    pub fn fn_selector_length(&self) -> u8 {
        self.ns.target.selector_length()
    }

    /// get the address length from the Namespace.
    pub fn address_length(&self) -> usize {
        self.ns.address_length
    }

    /// get the value length from the Namespace.
    pub fn value_length(&self) -> usize {
        self.ns.value_length
    }

    /// if the expression is a variable or a literial value, return an Operand.
    /// otherwise, return None.
    pub fn to_operand(
        &self,
        expr: &codegen::Expression,
        vartable: &mut Vartable,
    ) -> Option<Operand> {
        match expr {
            codegen::Expression::NumberLiteral { ty, value, loc, .. } => {
                let ssa_ty = self.lower_ast_type(ty);
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

    /**<pre>
    This function will expand a single codegen::Expression into a list Instructions and an Operand.
    For example:
        celcius * 9 / 5 + 32;
    will be expanded into something like:
        int32 temp1 = celcius * 9;
        int32 temp2 = temp1 / 5;
        int32 temp3 = temp2 + 32;
    and temp3 will be returned as an Operand.
    </pre>
    */
    pub fn to_operand_and_insns(
        &self,
        expr: &codegen::Expression,
        vartable: &mut Vartable,
        result: &mut Vec<Instruction>,
    ) -> Operand {
        match self.to_operand(expr, vartable) {
            Some(op) => op,
            None => {
                let ast_ty = expr.ty();
                let tmp = vartable.new_temp(self.lower_ast_type(&ast_ty));
                self.lower_expression(&tmp, expr, vartable, result);
                tmp
            }
        }
    }

    /// this functio essentially does the same thing as to_operand_and_insns, but it returns a
    /// Option<Operand> instead of Operand, because the expression might be None.
    pub fn to_operand_option_and_insns(
        &self,
        expr: &Option<codegen::Expression>,
        vartable: &mut Vartable,
        result: &mut Vec<Instruction>,
    ) -> Option<Operand> {
        match expr {
            Some(expr) => {
                let tmp = self.to_operand_and_insns(expr, vartable, result);
                Some(tmp)
            }
            None => None,
        }
    }

    /// this function is similar to the to_operand_and_insns function,
    /// but it takes a `ast::StringLocation<codegen::Expression>`
    /// and returns a `ast::StringLocation<Operand>` instead.
    pub fn to_string_location_and_insns(
        &self,
        location: &ast::StringLocation<codegen::Expression>,
        vartable: &mut Vartable,
        result: &mut Vec<Instruction>,
    ) -> ast::StringLocation<Operand> {
        match location {
            ast::StringLocation::CompileTime(str) => {
                ast::StringLocation::CompileTime(str.clone()) as ast::StringLocation<Operand>
            }
            ast::StringLocation::RunTime(expr) => {
                let op = self.to_operand_and_insns(expr, vartable, result);
                ast::StringLocation::RunTime(Box::new(op))
            }
        }
    }

    /// this function is similar to the to_operand_and_insns function,
    /// but it takes a `ast::ExternalCallAccounts<codegen::Expression>`
    /// and returns a `ast::ExternalCallAccounts<Operand>` instead.
    pub fn to_external_call_accounts_and_insns(
        &self,
        accounts: &ast::ExternalCallAccounts<codegen::Expression>,
        vartable: &mut Vartable,
        result: &mut Vec<Instruction>,
    ) -> ast::ExternalCallAccounts<Operand> {
        match accounts {
            ast::ExternalCallAccounts::Present(accounts) => {
                let tmp = self.to_operand_and_insns(accounts, vartable, result);
                ast::ExternalCallAccounts::Present(tmp)
            }
            ast::ExternalCallAccounts::NoAccount => {
                ast::ExternalCallAccounts::NoAccount as ast::ExternalCallAccounts<Operand>
            }
            ast::ExternalCallAccounts::AbsentArgument => {
                ast::ExternalCallAccounts::AbsentArgument as ast::ExternalCallAccounts<Operand>
            }
        }
    }

    /// this function is similar to the to_operand_and_insns function,
    /// but it takes a `cfg::InternalCallTy`
    /// and returns a `InternalCallTy` instead.
    /// If the `cfg::InternalCallTy` is a `cfg::InternalCallTy::Dynamic`,
    /// then the `cfg::Expression` will be expanded into a list of `Instruction`s and an `Operand`.
    pub fn to_internal_call_ty_and_insns(
        &self,
        call: &cfg::InternalCallTy,
        vartable: &mut Vartable,
        result: &mut Vec<Instruction>,
    ) -> InternalCallTy {
        match call {
            cfg::InternalCallTy::Builtin { ast_func_no } => InternalCallTy::Builtin {
                ast_func_no: *ast_func_no,
            },
            cfg::InternalCallTy::Static { cfg_no } => InternalCallTy::Static { cfg_no: *cfg_no },
            cfg::InternalCallTy::Dynamic(expr) => {
                let tmp = self.to_operand_and_insns(expr, vartable, result);
                InternalCallTy::Dynamic(tmp)
            }
            cfg::InternalCallTy::HostFunction { name: sesa } => {
                InternalCallTy::HostFunction { name: sesa.clone() }
            }
        }
    }

    /// This is the entry point of the Converter.
    /// It will lower the control flow graph into a Lower Intermediate Representation `LIR`.
    pub fn get_lir(&self) -> LIR {
        let mut vartable = self.to_vartable(&self.cfg.vars);

        let blocks = self
            .cfg
            .blocks
            .iter()
            .map(|block| self.lower_basic_block(block, &mut vartable))
            .collect::<Vec<Block>>();

        let params = self
            .cfg
            .params
            .iter()
            .map(|p| self.to_lir_typed_parameter(p))
            .collect::<Vec<Parameter<LIRType>>>();

        let returns = self
            .cfg
            .returns
            .iter()
            .map(|p| self.to_lir_typed_parameter(p))
            .collect::<Vec<Parameter<LIRType>>>();

        LIR {
            name: self.cfg.name.clone(),
            function_no: self.cfg.function_no,
            params,
            returns,
            vartable,
            blocks,
            nonpayable: self.cfg.nonpayable,
            public: self.cfg.public,
            ty: self.cfg.ty,
            selector: self.cfg.selector.clone(),
        }
    }

    fn lower_basic_block(&self, basic_block: &BasicBlock, vartable: &mut Vartable) -> Block {
        let mut instructions = vec![];
        for insn in &basic_block.instr {
            self.lower_instr(insn, vartable, &mut instructions);
        }

        Block {
            name: basic_block.name.clone(),
            instructions,
        }
    }

    fn to_lir_typed_parameter(&self, param: &Parameter<ast::Type>) -> Parameter<LIRType> {
        Parameter {
            loc: param.loc,
            id: param.id.clone(),
            ty: self.lower_ast_type(&param.ty),
            ty_loc: param.ty_loc,
            indexed: param.indexed,
            readonly: param.readonly,
            infinite_size: param.infinite_size,
            recursive: param.recursive,
            annotation: param.annotation.clone(),
        }
    }
}
