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

    pub fn to_string(&self) -> String {
        match self {
            ElementaryTypeName::Address => "address".to_string(),
            ElementaryTypeName::Bool => "bool".to_string(),
            ElementaryTypeName::String => "string".to_string(),
            ElementaryTypeName::Int(n) => format!("int{}", n),
            ElementaryTypeName::Uint(n) => format!("uint{}", n),
            ElementaryTypeName::Bytes(n) => format!("bytes{}", n),
            ElementaryTypeName::DynamicBytes => "bytes".to_string(),
            ElementaryTypeName::Any => panic!("any")
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
    pub ty: Cell<ElementaryTypeName>,
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
    PostIncrement(Loc, Box<Expression>),
    PostDecrement(Loc, Box<Expression>),
    New(Loc, ElementaryTypeName),
    IndexAccess(Loc, Box<Expression>, Box<Option<Expression>>),
    MemberAccess(Loc, Box<Expression>, Identifier),
    FunctionCall(Loc, Identifier, Vec<Expression>),
    Not(Loc, Box<Expression>),
    Complement(Loc, Box<Expression>),
    Delete(Loc, Box<Expression>),
    PreIncrement(Loc, Box<Expression>),
    PreDecrement(Loc, Box<Expression>),
    UnaryPlus(Loc, Box<Expression>),
    UnaryMinus(Loc, Box<Expression>),
    Power(Loc, Box<Expression>, Box<Expression>),
    Multiply(Loc, Box<Expression>, Box<Expression>),
    Divide(Loc, Box<Expression>, Box<Expression>),
    Modulo(Loc, Box<Expression>, Box<Expression>),
    Add(Loc, Box<Expression>, Box<Expression>),
    Subtract(Loc, Box<Expression>, Box<Expression>),
    ShiftLeft(Loc, Box<Expression>, Box<Expression>),
    ShiftRight(Loc, Box<Expression>, Box<Expression>),
    BitwiseAnd(Loc, Box<Expression>, Box<Expression>),
    BitwiseXor(Loc, Box<Expression>, Box<Expression>),
    BitwiseOr(Loc, Box<Expression>, Box<Expression>),
    Less(Loc, Box<Expression>, Box<Expression>),
    More(Loc, Box<Expression>, Box<Expression>),
    LessEqual(Loc, Box<Expression>, Box<Expression>),
    MoreEqual(Loc, Box<Expression>, Box<Expression>),
    Equal(Loc, Box<Expression>, Box<Expression>),
    NotEqual(Loc, Box<Expression>, Box<Expression>),
    And(Loc, Box<Expression>, Box<Expression>),
    Or(Loc, Box<Expression>, Box<Expression>),
    Ternary(Loc, Box<Expression>, Box<Expression>, Box<Expression>),
    Assign(Loc, Box<Expression>, Box<Expression>),
    AssignOr(Loc, Box<Expression>, Box<Expression>),
    AssignAnd(Loc, Box<Expression>, Box<Expression>),
    AssignXor(Loc, Box<Expression>, Box<Expression>),
    AssignShiftLeft(Loc, Box<Expression>, Box<Expression>),
    AssignShiftRight(Loc, Box<Expression>, Box<Expression>),
    AssignAdd(Loc, Box<Expression>, Box<Expression>),
    AssignSubtract(Loc, Box<Expression>, Box<Expression>),
    AssignMultiply(Loc, Box<Expression>, Box<Expression>),
    AssignDivide(Loc, Box<Expression>, Box<Expression>),
    AssignModulo(Loc, Box<Expression>, Box<Expression>),
    BoolLiteral(Loc, bool),
    NumberLiteral(Loc, BigInt),
    StringLiteral(Loc, String),
    Variable(Cell<ElementaryTypeName>, Identifier),
}

impl Expression {
    pub fn visit(&self, f: &mut FnMut(&Expression)  -> Result<(), ()>) -> Result<(), ()> {
        f(self)?;

        match self {
            Expression::Not(_, e) |
            Expression::Complement(_, e) => e.visit(f),
            Expression::AssignShiftLeft(_, l, r) |
            Expression::AssignShiftRight(_, l, r) |
            Expression::AssignMultiply(_, l, r) |
            Expression::AssignModulo(_, l, r) |
            Expression::AssignXor(_, l, r) |
            Expression::AssignDivide(_, l, r) |
            Expression::AssignOr(_, l, r) |
            Expression::AssignAnd(_, l, r) |
            Expression::AssignAdd(_, l, r) |
            Expression::AssignSubtract(_, l, r) |
            Expression::Assign(_, l, r) |
            Expression::ShiftLeft(_, l, r) |
            Expression::ShiftRight(_, l, r) |
            Expression::Multiply(_, l, r) |
            Expression::Modulo(_, l, r) |
            Expression::Divide(_, l, r) |
            Expression::Subtract(_, l, r) |
            Expression::Add(_, l, r) |
            Expression::Equal(_, l, r) |
            Expression::Less(_, l, r) |
            Expression::LessEqual(_, l, r) |
            Expression::More(_, l, r) |
            Expression::MoreEqual(_, l, r) |
            Expression::NotEqual(_, l, r) => {
                l.visit(f)?;
                r.visit(f)
            },
            Expression::PreDecrement(_, e) |
            Expression::PostDecrement(_, e) |
            Expression::PreIncrement(_, e) |
            Expression::PostIncrement(_, e) => e.visit(f),
            _ => Ok(())
        }
    }

    pub fn loc(&self) -> Loc {
        match self {
            Expression::PostIncrement(loc, _) |
            Expression::PostDecrement(loc, _) |
            Expression::New(loc, _) |
            Expression::IndexAccess(loc, _, _) |
            Expression::MemberAccess(loc, _, _) |
            Expression::FunctionCall(loc, _, _) |
            Expression::Not(loc, _) |
            Expression::Complement(loc, _) |
            Expression::Delete(loc, _) |
            Expression::PreIncrement(loc, _) |
            Expression::PreDecrement(loc, _) |
            Expression::UnaryPlus(loc, _) |
            Expression::UnaryMinus(loc, _) |
            Expression::Power(loc, _, _) |
            Expression::Multiply(loc, _, _) |
            Expression::Divide(loc, _, _) |
            Expression::Modulo(loc, _, _) |
            Expression::Add(loc, _, _) |
            Expression::Subtract(loc, _, _) |
            Expression::ShiftLeft(loc, _, _) |
            Expression::ShiftRight(loc, _, _) |
            Expression::BitwiseAnd(loc, _, _) |
            Expression::BitwiseXor(loc, _, _) |
            Expression::BitwiseOr(loc, _, _) |
            Expression::Less(loc, _, _) |
            Expression::More(loc, _, _) |
            Expression::LessEqual(loc, _, _) |
            Expression::MoreEqual(loc, _, _) |
            Expression::Equal(loc, _, _) |
            Expression::NotEqual(loc, _, _) |
            Expression::And(loc, _, _) |
            Expression::Or(loc, _, _) |
            Expression::Ternary(loc, _, _, _) |
            Expression::Assign(loc, _, _) |
            Expression::AssignOr(loc, _, _) |
            Expression::AssignAnd(loc, _, _) |
            Expression::AssignXor(loc, _, _) |
            Expression::AssignShiftLeft(loc, _, _) |
            Expression::AssignShiftRight(loc, _, _) |
            Expression::AssignAdd(loc, _, _) |
            Expression::AssignSubtract(loc, _, _) |
            Expression::AssignMultiply(loc, _, _) |
            Expression::AssignDivide(loc, _, _) |
            Expression::AssignModulo(loc, _, _) |
            Expression::BoolLiteral(loc, _) |
            Expression::NumberLiteral(loc, _) |
            Expression::StringLiteral(loc, _) |
            Expression::Variable(_, Identifier{ loc, name: _ })  => loc.clone()
        }
    }

    pub fn written_vars(&self, set: &mut HashMap<String, ElementaryTypeName>) {
        self.visit(&mut |expr| {
            match expr {
                Expression::AssignShiftLeft(_, box Expression::Variable(t, s), _) |
                Expression::AssignShiftRight(_, box Expression::Variable(t, s), _) |
                Expression::AssignMultiply(_, box Expression::Variable(t, s), _) |
                Expression::AssignModulo(_, box Expression::Variable(t, s), _) |
                Expression::AssignXor(_, box Expression::Variable(t, s), _) |
                Expression::AssignDivide(_, box Expression::Variable(t, s), _) |
                Expression::AssignOr(_, box Expression::Variable(t, s), _) |
                Expression::AssignAnd(_, box Expression::Variable(t, s), _) |
                Expression::AssignAdd(_, box Expression::Variable(t, s), _) |
                Expression::AssignSubtract(_, box Expression::Variable(t, s), _) |
                Expression::Assign(_, box Expression::Variable(t, s), _) |
                Expression::PreDecrement(_, box Expression::Variable(t, s)) |
                Expression::PostDecrement(_, box Expression::Variable(t, s)) |
                Expression::PreIncrement(_, box Expression::Variable(t, s)) |
                Expression::PostIncrement(_, box Expression::Variable(t, s)) => {
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
    Return(Loc, Option<Expression>),
    Throw,
    Emit(Identifier, Vec<Expression>),
    Empty
}

impl Statement {
    pub fn visit_stmt(&self, f: &mut FnMut(&Statement) -> Result<(), ()>) -> Result<(), ()> {
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

    pub fn visit_expr(&self, f: &mut FnMut(&Expression)  -> Result<(), ()>) -> Result<(), ()> {
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
