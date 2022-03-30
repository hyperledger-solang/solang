use crate::ast::Type;
use crate::sema::symtable::Symtable;
use crate::sema::yul::builtin::YulBuiltInFunction;
use num_bigint::BigInt;
use solang_parser::pt;
use solang_parser::pt::{CodeLocation, StorageLocation};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct InlineAssembly {
    pub loc: pt::Loc,
    pub body: Vec<(YulStatement, bool)>,
    pub functions: Vec<YulFunction>,
}

#[derive(Debug, Clone)]
pub struct YulBlock {
    pub loc: pt::Loc,
    pub body: Vec<(YulStatement, bool)>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum YulExpression {
    BoolLiteral(pt::Loc, bool, Type),
    NumberLiteral(pt::Loc, BigInt, Type),
    StringLiteral(pt::Loc, Vec<u8>, Type),
    YulLocalVariable(pt::Loc, Type, usize),
    SolidityLocalVariable(pt::Loc, Type, Option<StorageLocation>, usize),
    ConstantVariable(pt::Loc, Type, Option<usize>, usize),
    StorageVariable(pt::Loc, Type, usize, usize),
    BuiltInCall(pt::Loc, YulBuiltInFunction, Vec<YulExpression>),
    FunctionCall(pt::Loc, usize, Vec<YulExpression>),
    MemberAccess(pt::Loc, Box<YulExpression>, YulSuffix),
}

#[derive(PartialEq, Debug, Clone)]
pub enum YulSuffix {
    Offset,
    Slot,
    Length,
    Selector,
    Address,
}

impl ToString for YulSuffix {
    fn to_string(&self) -> String {
        let name = match self {
            YulSuffix::Offset => "offset",
            YulSuffix::Slot => "slot",
            YulSuffix::Length => "length",
            YulSuffix::Selector => "selector",
            YulSuffix::Address => "address",
        };

        name.to_string()
    }
}

impl CodeLocation for YulExpression {
    fn loc(&self) -> pt::Loc {
        match self {
            YulExpression::BoolLiteral(loc, ..)
            | YulExpression::NumberLiteral(loc, ..)
            | YulExpression::StringLiteral(loc, ..)
            | YulExpression::YulLocalVariable(loc, ..)
            | YulExpression::SolidityLocalVariable(loc, ..)
            | YulExpression::ConstantVariable(loc, ..)
            | YulExpression::StorageVariable(loc, ..)
            | YulExpression::BuiltInCall(loc, ..)
            | YulExpression::MemberAccess(loc, ..)
            | YulExpression::FunctionCall(loc, ..) => *loc,
        }
    }
}

#[derive(Debug, Clone)]
pub struct YulFunction {
    pub loc: pt::Loc,
    pub name: String,
    pub params: Arc<Vec<YulFunctionParameter>>,
    pub returns: Arc<Vec<YulFunctionParameter>>,
    pub body: Vec<(YulStatement, bool)>,
    pub symtable: Symtable,
    pub called: bool,
}

#[derive(Debug, Clone)]
pub struct YulFunctionParameter {
    pub loc: pt::Loc,
    pub id: pt::Identifier,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub enum YulStatement {
    FunctionCall(pt::Loc, usize, Vec<YulExpression>),
    BuiltInCall(pt::Loc, YulBuiltInFunction, Vec<YulExpression>),
    Block(Box<YulBlock>),
    VariableDeclaration(pt::Loc, Vec<usize>, Option<YulExpression>),
    Assignment(pt::Loc, Vec<YulExpression>, YulExpression),
    IfBlock(pt::Loc, YulExpression, Box<YulBlock>),
    Switch {
        loc: pt::Loc,
        condition: YulExpression,
        cases: Vec<CaseBlock>,
        default: Option<YulBlock>,
    },
    For {
        loc: pt::Loc,
        init_block: YulBlock,
        condition: YulExpression,
        post_block: YulBlock,
        execution_block: YulBlock,
    },
    Leave(pt::Loc),
    Break(pt::Loc),
    Continue(pt::Loc),
}

#[derive(Debug, Clone)]
pub struct CaseBlock {
    pub loc: pt::Loc,
    pub condition: YulExpression,
    pub block: YulBlock,
}
