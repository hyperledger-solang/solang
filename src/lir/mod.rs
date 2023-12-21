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

use self::lir_type::LIRType;

/// The `LIR` struct represents the Lower Intermediate Representation of a function,
/// which uses three-address code for instructions.
#[derive(Debug)]
pub struct LIR {
    /// The name of the function.
    pub name: String,
    /// The unique identifier of the function.
    pub function_no: ASTFunction,
    /// The parameters of the function, with their types.
    pub params: Vec<Parameter<LIRType>>,
    /// The return values of the function, with their types.
    pub returns: Vec<Parameter<LIRType>>,
    /// A table of variables used in the function.
    pub vartable: Vartable,
    /// The blocks of instructions in the function.
    pub blocks: Vec<Block>,
    /// A flag indicating whether the function is non-payable.
    pub nonpayable: bool,
    /// A flag indicating whether the function is public.
    pub public: bool,
    /// The type of the function (e.g., constructor, fallback, etc.).
    pub ty: FunctionTy,
    /// Used to match the function in the contract
    pub selector: Vec<u8>,
}

/// A block of instructions in the Lower Intermediate Representation.
#[derive(Debug)]
pub struct Block {
    /// The name of the block.
    pub name: String,
    /// The instructions in the block.
    pub instructions: Vec<Instruction>,
}
