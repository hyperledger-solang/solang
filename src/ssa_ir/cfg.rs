use std::sync::Arc;
use solang_parser::pt::FunctionTy;
use crate::codegen::cfg::ASTFunction;
use crate::sema::ast::Parameter;
use crate::ssa_ir::insn::Insn;
use crate::ssa_ir::vartable::Vartable;

#[derive(Debug)]
pub struct Cfg {// FIXME: need some adjustments on the params and types
    pub name: String,
    pub function_no: ASTFunction,
    // TODO: define a new type for params?
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
    pub instructions: Vec<Insn>
}