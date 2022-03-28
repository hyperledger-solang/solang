use crate::ast::Type;
use crate::sema::assembly::builtin::AssemblyBuiltInFunction;
use crate::sema::symtable::Symtable;
use num_bigint::BigInt;
use solang_parser::pt;
use solang_parser::pt::{CodeLocation, StorageLocation};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct InlineAssembly {
    pub loc: pt::Loc,
    pub body: Vec<(AssemblyStatement, bool)>,
    pub functions: Vec<AssemblyFunction>,
}

#[derive(Debug, Clone)]
pub struct AssemblyBlock {
    pub loc: pt::Loc,
    pub body: Vec<(AssemblyStatement, bool)>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum AssemblyExpression {
    BoolLiteral(pt::Loc, bool, Type),
    NumberLiteral(pt::Loc, BigInt, Type),
    StringLiteral(pt::Loc, Vec<u8>, Type),
    AssemblyLocalVariable(pt::Loc, Type, usize),
    SolidityLocalVariable(pt::Loc, Type, Option<StorageLocation>, usize),
    ConstantVariable(pt::Loc, Type, Option<usize>, usize),
    StorageVariable(pt::Loc, Type, usize, usize),
    BuiltInCall(pt::Loc, AssemblyBuiltInFunction, Vec<AssemblyExpression>),
    FunctionCall(pt::Loc, usize, Vec<AssemblyExpression>),
    MemberAccess(pt::Loc, Box<AssemblyExpression>, AssemblySuffix),
}

#[derive(PartialEq, Debug, Clone)]
pub enum AssemblySuffix {
    Offset,
    Slot,
    Length,
    Selector,
    Address,
}

impl ToString for AssemblySuffix {
    fn to_string(&self) -> String {
        let name = match self {
            AssemblySuffix::Offset => "offset",
            AssemblySuffix::Slot => "slot",
            AssemblySuffix::Length => "length",
            AssemblySuffix::Selector => "selector",
            AssemblySuffix::Address => "address",
        };

        name.to_string()
    }
}

impl CodeLocation for AssemblyExpression {
    fn loc(&self) -> pt::Loc {
        match self {
            AssemblyExpression::BoolLiteral(loc, ..)
            | AssemblyExpression::NumberLiteral(loc, ..)
            | AssemblyExpression::StringLiteral(loc, ..)
            | AssemblyExpression::AssemblyLocalVariable(loc, ..)
            | AssemblyExpression::SolidityLocalVariable(loc, ..)
            | AssemblyExpression::ConstantVariable(loc, ..)
            | AssemblyExpression::StorageVariable(loc, ..)
            | AssemblyExpression::BuiltInCall(loc, ..)
            | AssemblyExpression::MemberAccess(loc, ..)
            | AssemblyExpression::FunctionCall(loc, ..) => *loc,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssemblyFunction {
    pub loc: pt::Loc,
    pub name: String,
    pub params: Arc<Vec<AssemblyFunctionParameter>>,
    pub returns: Arc<Vec<AssemblyFunctionParameter>>,
    pub body: Vec<(AssemblyStatement, bool)>,
    pub symtable: Symtable,
    pub called: bool,
}

#[derive(Debug, Clone)]
pub struct AssemblyFunctionParameter {
    pub loc: pt::Loc,
    pub id: pt::Identifier,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub enum AssemblyStatement {
    FunctionCall(pt::Loc, usize, Vec<AssemblyExpression>),
    BuiltInCall(pt::Loc, AssemblyBuiltInFunction, Vec<AssemblyExpression>),
    Block(Box<AssemblyBlock>),
    VariableDeclaration(pt::Loc, Vec<usize>, Option<AssemblyExpression>),
    Assignment(pt::Loc, Vec<AssemblyExpression>, AssemblyExpression),
    IfBlock(pt::Loc, AssemblyExpression, Box<AssemblyBlock>),
    Switch {
        loc: pt::Loc,
        condition: AssemblyExpression,
        cases: Vec<CaseBlock>,
        default: Option<AssemblyBlock>,
    },
    For {
        loc: pt::Loc,
        init_block: AssemblyBlock,
        condition: AssemblyExpression,
        post_block: AssemblyBlock,
        execution_block: AssemblyBlock,
    },
    Leave(pt::Loc),
    Break(pt::Loc),
    Continue(pt::Loc),
}

#[derive(Debug, Clone)]
pub struct CaseBlock {
    pub loc: pt::Loc,
    pub condition: AssemblyExpression,
    pub block: AssemblyBlock,
}
