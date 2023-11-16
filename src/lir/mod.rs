// SPDX-License-Identifier: Apache-2.0

pub mod converter;
pub mod expressions;
pub mod instructions;
pub mod lir_type;
pub mod printer;
pub mod vartable;

use crate::codegen::cfg::ASTFunction;
use crate::lir::instructions::Instruction;
use crate::lir::vartable::Vartable;
use crate::pt::FunctionTy;
use crate::sema::ast::Parameter;

use self::lir_type::Type;

#[derive(Debug)]
pub struct LIR {
    pub name: String,
    pub function_no: ASTFunction,
    pub params: Vec<Parameter<Type>>,
    pub returns: Vec<Parameter<Type>>,
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
    pub instructions: Vec<Instruction>,
}
