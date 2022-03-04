use crate::ast::Type;
use crate::sema::assembly::AssemblyStatement;
use crate::sema::symtable::Symtable;
use indexmap::IndexMap;
use solang_parser::pt;

#[derive(Debug, Clone)]
pub struct AssemblyFunction {
    pub loc: pt::Loc,
    pub name: String,
    pub params: Vec<AssemblyFunctionParameter>,
    pub returns: Vec<AssemblyFunctionParameter>,
    pub body: Vec<AssemblyStatement>,
    pub functions: IndexMap<String, AssemblyFunction>,
    pub symtable: Symtable,
}

#[derive(Debug, Clone)]
pub struct AssemblyFunctionParameter {
    pub loc: pt::Loc,
    pub name: pt::Identifier,
    pub ty: Type,
}
