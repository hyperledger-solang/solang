// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::ASTFunction;
use crate::pt::FunctionTy;
use crate::ssa_ir::insn::Insn;
use crate::ssa_ir::ssa_type::Parameter;
use crate::ssa_ir::vartable::Vartable;
use std::sync::Arc;

#[derive(Debug)]
pub struct Cfg {
    pub name: String,
    pub function_no: ASTFunction,
    pub params: Arc<Vec<Parameter>>,
    pub returns: Arc<Vec<Parameter>>,
    pub vartable: Vartable,
    pub blocks: Vec<Block>,
    pub nonpayable: bool,
    pub public: bool,
    pub ty: FunctionTy,
    /// used to match the function in the contract
    pub selector: Vec<u8>,
}

#[derive(Debug)]
pub struct Block {
    pub name: String,
    pub instructions: Vec<Insn>,
}
