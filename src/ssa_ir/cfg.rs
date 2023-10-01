use std::sync::Arc;
use indexmap::IndexMap;
use solang_parser::pt::FunctionTy;
use crate::codegen::cfg::{ArrayLengthVars, ASTFunction};
use crate::sema::ast::{Parameter, Type};
use crate::ssa_ir::insn::Insn;

pub struct Var {
    id: usize,
    ty: Type,
    name: String
}

#[derive(Debug, Clone, Default)]
pub struct Block {
    pub name: String,
    pub instructions: Vec<Insn>,
}

pub struct Cfg {// FIXME: need some adjustments on the names and types
    pub name: String,
    pub function_no: ASTFunction,
    // TODO: define a new type for params?
    pub params: Arc<Vec<Parameter>>,
    pub returns: Arc<Vec<Parameter>>,
    pub vars: IndexMap<usize, Var>,
    pub blocks: Vec<Block>,

    // ...
    pub nonpayable: bool,
    pub public: bool,
    pub ty: FunctionTy,
    pub selector: Vec<u8>,
    current: usize,
    pub array_lengths_temps: ArrayLengthVars,
    pub modifier: Option<usize>,
}