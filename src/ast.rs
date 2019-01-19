use num_bigint::BigInt;
use std::collections::HashMap;
use std::cell::Cell;

#[derive(Debug,PartialEq,Clone)]
pub struct Loc(
    pub usize,
    pub usize
);

#[derive(Debug,PartialEq)]
pub struct Identifier {
    pub loc: Loc,
    pub name: String
}

#[derive(Debug,PartialEq)]
pub struct SourceUnit {
    pub name: String,
    pub parts: Vec<SourceUnitPart>,
    pub resolved: bool
}

#[derive(Debug,PartialEq)]
pub enum SourceUnitPart {
    ContractDefinition(Box<ContractDefinition>),
    PragmaDirective(Identifier, String),
    ImportDirective(String)
}

#[derive(Debug,PartialEq,Clone,Copy)]
pub enum ElementaryTypeName {
    Address,
    Bool,
    String,
    Int(u16),
    Uint(u16),
    Bytes(u8),
    DynamicBytes,
    Any,
}

impl ElementaryTypeName {
    pub fn signed(&self) -> bool {
        match self {
            ElementaryTypeName::Int(_) => true,
            _ => false
        }
    }

    pub fn ordered(&self) -> bool {
        match self {
            ElementaryTypeName::Int(_) => true,
            ElementaryTypeName::Uint(_) => true,
            _ => false
        }
    }
}

#[derive(Debug,PartialEq)]
pub enum StorageLocation {
    Default,
    Memory,
    Storage,
    Calldata,
}

#[derive(Debug,PartialEq)]
pub struct VariableDeclaration {
    pub typ: ElementaryTypeName,
    pub storage: StorageLocation,
    pub name: Identifier
}

#[derive(Debug,PartialEq)]
pub struct StructDefinition {
    pub name: Identifier,
    pub fields: Vec<Box<VariableDeclaration>>
}

#[derive(Debug,PartialEq)]
pub enum ContractPart {
    StructDefinition(Box<StructDefinition>),
    EventDefinition(Box<EventDefinition>),
    EnumDefinition(Box<EnumDefinition>),
    StateVariableDeclaration(Box<StateVariableDeclaration>),
    FunctionDefinition(Box<FunctionDefinition>),
}

#[derive(Debug,PartialEq)]
pub enum ContractType {
    Contract,
    Interface,
    Library,
}

#[derive(Debug,PartialEq)]
pub struct ContractDefinition {
    pub typ: ContractType,
    pub name: Identifier,
    pub parts: Vec<ContractPart>,
}

#[derive(Debug,PartialEq)]
pub struct EventParameter {
    pub typ: ElementaryTypeName,
    pub indexed: bool,
    pub name: Option<Identifier>,
}

#[derive(Debug,PartialEq)]
pub struct EventDefinition {
    pub name: Identifier,
    pub fields: Vec<EventParameter>,
    pub anonymous: bool,
}

#[derive(Debug,PartialEq)]
pub struct EnumDefinition {
    pub name: Identifier,
    pub values: Vec<Identifier>,
}

#[derive(Debug,PartialEq)]
pub enum VariableAttribute {
    Public,
    Internal,
    Private,
    Constant
}

#[derive(Debug,PartialEq)]
pub struct StateVariableDeclaration {
    pub typ: ElementaryTypeName,
    pub attrs: Vec<VariableAttribute>,
    pub name: Identifier,
    pub initializer: Option<Expression>,
}

#[derive(Debug,PartialEq)]
pub enum Expression {
    PostIncrement(Box<Expression>),
    PostDecrement(Box<Expression>),
    New(ElementaryTypeName),
    IndexAccess(Box<Expression>, Box<Option<Expression>>),
    MemberAccess(Box<Expression>, Identifier),
    FunctionCall(Identifier, Vec<Expression>),
    Not(Box<Expression>),
    Complement(Box<Expression>),
    Delete(Box<Expression>),
    PreIncrement(Box<Expression>),
    PreDecrement(Box<Expression>),
    UnaryPlus(Box<Expression>),
    UnaryMinus(Box<Expression>),
    Power(Box<Expression>, Box<Expression>),
    Multiply(Box<Expression>, Box<Expression>),
    Divide(Box<Expression>, Box<Expression>),
    Modulo(Box<Expression>, Box<Expression>),
    Add(Box<Expression>, Box<Expression>),
    Subtract(Box<Expression>, Box<Expression>),
    ShiftLeft(Box<Expression>, Box<Expression>),
    ShiftRight(Box<Expression>, Box<Expression>),
    BitwiseAnd(Box<Expression>, Box<Expression>),
    BitwiseXor(Box<Expression>, Box<Expression>),
    BitwiseOr(Box<Expression>, Box<Expression>),
    Less(Box<Expression>, Box<Expression>),
    More(Box<Expression>, Box<Expression>),
    LessEqual(Box<Expression>, Box<Expression>),
    MoreEqual(Box<Expression>, Box<Expression>),
    Equal(Box<Expression>, Box<Expression>),
    NotEqual(Box<Expression>, Box<Expression>),
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Ternary(Box<Expression>, Box<Expression>, Box<Expression>),
    Assign(Box<Expression>, Box<Expression>),
    AssignOr(Box<Expression>, Box<Expression>),
    AssignAnd(Box<Expression>, Box<Expression>),
    AssignXor(Box<Expression>, Box<Expression>),
    AssignShiftLeft(Box<Expression>, Box<Expression>),
    AssignShiftRight(Box<Expression>, Box<Expression>),
    AssignAdd(Box<Expression>, Box<Expression>),
    AssignSubtract(Box<Expression>, Box<Expression>),
    AssignMultiply(Box<Expression>, Box<Expression>),
    AssignDivide(Box<Expression>, Box<Expression>),
    AssignModulo(Box<Expression>, Box<Expression>),
    BoolLiteral(bool),
    NumberLiteral(BigInt),
    StringLiteral(String),
    Variable(Cell<ElementaryTypeName>, Identifier),
}

impl Expression {
    pub fn visit(&self, f: &mut FnMut(&Expression)  -> Result<(), String>) -> Result<(), String> {
        f(self)?;

        match self {
            Expression::Not(e) |
            Expression::Complement(e) => e.visit(f),
            Expression::AssignShiftLeft(l, r) |
            Expression::AssignShiftRight(l, r) |
            Expression::AssignMultiply(l, r) |
            Expression::AssignModulo(l, r) |
            Expression::AssignXor(l, r) |
            Expression::AssignDivide(l, r) |
            Expression::AssignOr(l, r) |
            Expression::AssignAnd(l, r) |
            Expression::AssignAdd(l, r) |
            Expression::AssignSubtract(l, r) |
            Expression::Assign(l, r) |
            Expression::ShiftLeft(l, r) |
            Expression::ShiftRight(l, r) |
            Expression::Multiply(l, r) |
            Expression::Modulo(l, r) |
            Expression::Divide(l, r) |
            Expression::Subtract(l, r) |
            Expression::Add(l, r) |
            Expression::Equal(l, r) |
            Expression::Less(l, r) |
            Expression::LessEqual(l, r) |
            Expression::More(l, r) |
            Expression::MoreEqual(l, r) |
            Expression::NotEqual(l, r) => {
                l.visit(f)?;
                r.visit(f)
            },
            Expression::PreDecrement(e) |
            Expression::PostDecrement(e) |
            Expression::PreIncrement(e) |
            Expression::PostIncrement(e) => e.visit(f),
            _ => Ok(())
        }
    }

    pub fn written_vars(&self, set: &mut HashMap<String, ElementaryTypeName>) {
        self.visit(&mut |expr| {
            match expr {
                Expression::AssignShiftLeft(box Expression::Variable(t, s), _) |
                Expression::AssignShiftRight(box Expression::Variable(t, s), _) |
                Expression::AssignMultiply(box Expression::Variable(t, s), _) |
                Expression::AssignModulo(box Expression::Variable(t, s), _) |
                Expression::AssignXor(box Expression::Variable(t, s), _) |
                Expression::AssignDivide(box Expression::Variable(t, s), _) |
                Expression::AssignOr(box Expression::Variable(t, s), _) |
                Expression::AssignAnd(box Expression::Variable(t, s), _) |
                Expression::AssignAdd(box Expression::Variable(t, s), _) |
                Expression::AssignSubtract(box Expression::Variable(t, s), _) |
                Expression::Assign(box Expression::Variable(t, s), _) |
                Expression::PreDecrement(box Expression::Variable(t, s)) |
                Expression::PostDecrement(box Expression::Variable(t, s)) |
                Expression::PreIncrement(box Expression::Variable(t, s)) |
                Expression::PostIncrement(box Expression::Variable(t, s)) => {
                    set.insert(s.name.to_string(), t.get());
                },
                _ => ()
            };

            Ok(())
        }).unwrap();
    }
}

#[derive(Debug,PartialEq)]
pub struct Parameter {
    pub typ: ElementaryTypeName,
    pub storage: Option<StorageLocation>,
    pub name: Option<Identifier>
}

#[derive(Debug,PartialEq)]
pub enum StateMutability {
    Pure,
    View,
    Payable
}

#[derive(Debug,PartialEq)]
pub enum FunctionAttribute {
    StateMutability(StateMutability),
    External,
    Public,
    Internal,
    Private,
}

#[derive(Debug,PartialEq)]
pub struct FunctionDefinition {
    pub name: Option<Identifier>,
    pub params: Vec<Parameter>,
    pub attributes: Vec<FunctionAttribute>,
    pub returns: Vec<Parameter>,
    pub body: Statement,
    // annotated tree
    pub vartable: Option<HashMap<String, ElementaryTypeName>>,
}

#[derive(Debug,PartialEq)]
pub struct BlockStatement(
    pub Vec<Statement>
);

#[derive(Debug,PartialEq)]
pub enum Statement {
    BlockStatement(BlockStatement),
    If(Expression, Box<Statement>, Box<Option<Statement>>),
    While(Expression, Box<Statement>),
    PlaceHolder,
    Expression(Expression),
    VariableDefinition(Box<VariableDeclaration>, Option<Expression>),
    For(Box<Option<Statement>>, Box<Option<Expression>>, Box<Option<Statement>>, Box<Option<Statement>>),
    DoWhile(Box<Statement>, Expression),
    Continue,
    Break,
    Return(Option<Expression>),
    Throw,
    Emit(Identifier, Vec<Expression>),
    Empty
}

impl Statement {
    pub fn visit_stmt(&self, f: &mut FnMut(&Statement) -> Result<(), String>) -> Result<(), String> {
        f(self)?;

        match self {
            Statement::BlockStatement(BlockStatement(bs)) => {
                for i in bs {
                    i.visit_stmt(f)?;
                }
            },
            Statement::For(i, _, n, b) => {
                if let box Some(j) = i {
                    j.visit_stmt(f)?;
                }
                if let box Some(j) = n {
                    j.visit_stmt(f)?;
                }
                if let box Some(j) = b {
                    j.visit_stmt(f)?;
                }
            },
            Statement::While(_, b) => {
                b.visit_stmt(f)?;
            },
            Statement::DoWhile(b, _) => {
                b.visit_stmt(f)?;
            },
            Statement::If(_, then, _else) => {
                then.visit_stmt(f)?;
                if let box Some(b) = _else {
                    b.visit_stmt(f)?;
                }
            },
            _ => ()
        }

        Ok(())
    }

    pub fn visit_expr(&self, f: &mut FnMut(&Expression)  -> Result<(), String>) -> Result<(), String> {
        self.visit_stmt(&mut |s| {
            match s {
                Statement::Expression(e) => e.visit(f),
                Statement::If(e, _, _) => e.visit(f),
                Statement::While(e, _) => e.visit(f),
                Statement::DoWhile(_, e) => e.visit(f),
                Statement::For(_, box Some(e), _, _) => e.visit(f),
                _ => Ok(())
            }
        })
    }

    pub fn written_vars(&self, set: &mut HashMap<String, ElementaryTypeName>) {
        self.visit_expr(&mut |expr| {
            expr.written_vars(set);
            Ok(())
        }).unwrap();
    }
}

#[cfg(test)]
mod test {
    use solidity;
    use super::*;
    
    #[test]
    fn parse_test() {
        let e = solidity::SourceUnitParser::new()
                .parse("contract foo {
                    struct Jurisdiction {
                        bool exists;
                        uint keyIdx;
                        bytes2 country;
                        bytes32 region;
                    }
                    string __abba_$;
                    int64 $thing_102;
                }")
                .unwrap();

        let a = SourceUnit{name: "".to_string(), parts: vec![
            SourceUnitPart::ContractDefinition(
                Box::new(ContractDefinition{typ: ContractType::Contract, name: Identifier{loc: Loc(9, 12), name: "foo".to_string()}, parts: vec![
                    ContractPart::StructDefinition(Box::new(StructDefinition{name: Identifier{loc: Loc(42, 54), name: "Jurisdiction".to_string()}, fields: vec![
                        Box::new(VariableDeclaration{
                            typ: ElementaryTypeName::Bool, storage: StorageLocation::Default, name: Identifier{loc: Loc(86, 92), name: "exists".to_string()}
                        }),
                        Box::new(VariableDeclaration{
                            typ: ElementaryTypeName::Uint(256), storage: StorageLocation::Default, name: Identifier{loc: Loc(123, 129), name: "keyIdx".to_string()}
                        }),
                        Box::new(VariableDeclaration{
                            typ: ElementaryTypeName::Bytes(2), storage: StorageLocation::Default, name: Identifier{loc: Loc(162, 169), name: "country".to_string()}
                        }),
                        Box::new(VariableDeclaration{
                            typ: ElementaryTypeName::Bytes(32), storage: StorageLocation::Default, name: Identifier{loc: Loc(203, 209), name: "region".to_string()}
                        })
                    ]})),
                    ContractPart::StateVariableDeclaration(Box::new(StateVariableDeclaration{
                        typ: ElementaryTypeName::String, attrs: vec![], name: Identifier{loc: Loc(260, 268), name: "__abba_$".to_string()}, initializer: None
                    })),
                    ContractPart::StateVariableDeclaration(Box::new(StateVariableDeclaration{
                        typ: ElementaryTypeName::Int(64), attrs: vec![], name: Identifier{loc: Loc(296, 306), name: "$thing_102".to_string()}, initializer: None
                    }))
            ]}))
        ], resolved: false};

        assert_eq!(e, a);
    }
}
