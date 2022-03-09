use crate::sema::assembly::functions::AssemblyFunction;
use indexmap::IndexMap;
use solang_parser::pt;

mod builtin;
mod expression;
mod functions;
mod tests;
mod types;

// TODO: Functions can be called anywhere inside the block.
#[derive(Debug, Clone)]
pub struct AssemblyBlock {
    pub loc: pt::Loc,
    pub body: Vec<AssemblyStatement>,
    pub functions: IndexMap<String, AssemblyFunction>,
}

#[derive(Debug, Clone)]
pub enum AssemblyStatement {
    AssemblyBlock(AssemblyBlock),
}
