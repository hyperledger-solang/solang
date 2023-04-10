// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{FunctionAttributes, Parameter, RetrieveType, Type};
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
    pub next_reachable: bool,
    pub statements: Vec<YulStatement>,
}

impl YulBlock {
    /// Returns if whatever follows the YulBlock is reachable
    pub fn is_next_reachable(&self) -> bool {
        self.statements.is_empty() || (!self.statements.is_empty() && self.next_reachable)
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum YulExpression {
    BoolLiteral(pt::Loc, bool, Type),
    NumberLiteral(pt::Loc, BigInt, Type),
    StringLiteral(pt::Loc, Vec<u8>, Type),
    YulLocalVariable(pt::Loc, Type, usize),
    SolidityLocalVariable(pt::Loc, Type, Option<StorageLocation>, usize),
    ConstantVariable(pt::Loc, Type, Option<usize>, usize),
    StorageVariable(pt::Loc, Type, usize, usize),
    BuiltInCall(pt::Loc, YulBuiltInFunction, Vec<YulExpression>),
    FunctionCall(pt::Loc, usize, Vec<YulExpression>, Arc<Vec<Parameter>>),
    SuffixAccess(pt::Loc, Box<YulExpression>, YulSuffix),
}

impl RetrieveType for YulExpression {
    fn ty(&self) -> Type {
        match self {
            YulExpression::BoolLiteral(_, _, ty)
            | YulExpression::NumberLiteral(_, _, ty)
            | YulExpression::StringLiteral(_, _, ty)
            | YulExpression::YulLocalVariable(_, ty, ..)
            | YulExpression::SolidityLocalVariable(_, ty, ..)
            | YulExpression::ConstantVariable(_, ty, ..)
            | YulExpression::StorageVariable(_, ty, ..) => ty.clone(),

            YulExpression::SuffixAccess(..) => Type::Uint(256),

            YulExpression::BuiltInCall(_, ty, ..) => {
                let prototype = ty.get_prototype_info();
                if prototype.no_returns == 1 {
                    Type::Uint(256)
                } else {
                    unreachable!("Expression does not have a type");
                }
            }

            YulExpression::FunctionCall(_, _, _, returns) => {
                if returns.len() == 1 {
                    returns[0].ty.clone()
                } else {
                    unreachable!("Expression does not have a type");
                }
            }
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
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
            | YulExpression::SuffixAccess(loc, ..)
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
    pub body: YulBlock,
    pub symtable: Symtable,
    pub parent_sol_func: Option<usize>,
    pub func_no: usize,
    pub called: bool,
    pub cfg_no: usize,
}

impl FunctionAttributes for YulFunction {
    fn get_symbol_table(&self) -> &Symtable {
        &self.symtable
    }

    fn get_parameters(&self) -> &Vec<Parameter> {
        &self.params
    }

    fn get_returns(&self) -> &Vec<Parameter> {
        &self.returns
    }
}

#[derive(Clone, Debug)]
pub enum YulStatement {
    FunctionCall(pt::Loc, bool, usize, Vec<YulExpression>),
    BuiltInCall(pt::Loc, bool, YulBuiltInFunction, Vec<YulExpression>),
    Block(Box<YulBlock>),
    VariableDeclaration(pt::Loc, bool, Vec<(usize, Type)>, Option<YulExpression>),
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

impl YulStatement {
    /// Returns if the current statement is reachable, i.e. there is a code path from the entry to
    /// it.
    pub fn is_reachable(&self) -> bool {
        match self {
            YulStatement::FunctionCall(_, reachable, ..)
            | YulStatement::BuiltInCall(_, reachable, ..)
            | YulStatement::VariableDeclaration(_, reachable, ..)
            | YulStatement::Assignment(_, reachable, ..)
            | YulStatement::IfBlock(_, reachable, ..)
            | YulStatement::Switch { reachable, .. }
            | YulStatement::For { reachable, .. }
            | YulStatement::Leave(_, reachable)
            | YulStatement::Break(_, reachable)
            | YulStatement::Continue(_, reachable) => *reachable,

            YulStatement::Block(block) => block.reachable,
        }
    }
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
            YulExpression::BuiltInCall(_, _, args) | YulExpression::FunctionCall(_, _, args, _) => {
                for arg in args {
                    arg.recurse(cx, f);
                }
            }
            YulExpression::SuffixAccess(_, expr, _) => {
                expr.recurse(cx, f);
            }

            _ => (),
        }
    }
}
