use crate::ast::{Parameter, Type};
use crate::sema::symtable::Symtable;
use crate::sema::yul::builtin::YulBuiltInFunction;
use crate::sema::Recurse;
use num_bigint::BigInt;
use solang_parser::pt;
use solang_parser::pt::{CodeLocation, StorageLocation};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct InlineAssembly {
    pub loc: pt::Loc,
    pub body: Vec<YulStatement>,
    // (begin, end) offset for Namespace::yul_functions
    pub functions: std::ops::Range<usize>,
}

#[derive(Debug, Clone)]
pub struct YulBlock {
    pub loc: pt::Loc,
    pub reachable: bool,
    pub body: Vec<YulStatement>,
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
    pub params: Arc<Vec<Parameter>>,
    pub returns: Arc<Vec<Parameter>>,
    pub body: Vec<YulStatement>,
    pub symtable: Symtable,
    pub parent_sol_func: Option<usize>,
    pub func_no: usize,
    pub called: bool,
    pub cfg_no: usize,
}

#[derive(Clone, Debug)]
pub enum YulStatement {
    FunctionCall(pt::Loc, bool, usize, Vec<YulExpression>),
    BuiltInCall(pt::Loc, bool, YulBuiltInFunction, Vec<YulExpression>),
    Block(Box<YulBlock>),
    VariableDeclaration(pt::Loc, bool, Vec<usize>, Option<YulExpression>),
    Assignment(pt::Loc, bool, Vec<YulExpression>, YulExpression),
    IfBlock(pt::Loc, bool, YulExpression, Box<YulBlock>),
    Switch {
        loc: pt::Loc,
        reachable: bool,
        condition: YulExpression,
        cases: Vec<CaseBlock>,
        default: Option<YulBlock>,
    },
    For {
        loc: pt::Loc,
        reachable: bool,
        init_block: YulBlock,
        condition: YulExpression,
        post_block: YulBlock,
        execution_block: YulBlock,
    },
    Leave(pt::Loc, bool),
    Break(pt::Loc, bool),
    Continue(pt::Loc, bool),
}

#[derive(Debug, Clone)]
pub struct CaseBlock {
    pub loc: pt::Loc,
    pub condition: YulExpression,
    pub block: YulBlock,
}

impl Recurse for YulExpression {
    type ArgType = YulExpression;
    fn recurse<T>(&self, cx: &mut T, f: fn(expr: &YulExpression, ctx: &mut T) -> bool) {
        if !f(self, cx) {
            return;
        }
        match self {
            YulExpression::BuiltInCall(_, _, args) | YulExpression::FunctionCall(_, _, args) => {
                for arg in args {
                    arg.recurse(cx, f);
                }
            }
            YulExpression::MemberAccess(_, expr, _) => {
                expr.recurse(cx, f);
            }

            _ => (),
        }
    }
}
