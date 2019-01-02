use num_bigint::BigInt;
use std::collections::HashMap;

#[derive(Debug,PartialEq)]
pub struct SourceUnit(
    pub String,
    pub Vec<SourceUnitPart>
);

#[derive(Debug,PartialEq)]
pub enum SourceUnitPart {
    ContractDefinition(Box<ContractDefinition>),
    PragmaDirective(String, String),
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

#[derive(Debug,PartialEq)]
pub enum StorageLocation {
    Default,
    Memory,
    Storage,
    Calldata,
}

#[derive(Debug,PartialEq)]
pub struct VariableDeclaration(
    pub ElementaryTypeName, 
    pub StorageLocation,
    pub String
);

#[derive(Debug,PartialEq)]
pub struct StructDefinition(
    String,
    Vec<Box<VariableDeclaration>>
);

impl StructDefinition {
    pub fn new(n: String, v: Vec<Box<VariableDeclaration>>) -> StructDefinition {
        StructDefinition(n, v)
    }
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
pub struct ContractDefinition(
    pub ContractType,
    pub String,
    pub Vec<ContractPart>,
);

#[derive(Debug,PartialEq)]
pub struct EventParameter(
    ElementaryTypeName,
    bool,
    Option<String>,
);

impl EventParameter {
    pub fn new(t: ElementaryTypeName, indexed: bool, n: Option<String>) -> EventParameter {
        EventParameter(t, indexed, n)
    }
}

#[derive(Debug,PartialEq)]
pub struct EventDefinition(
    String,
    Vec<EventParameter>,
    bool,
);

impl EventDefinition {
    pub fn new(n: String, v: Vec<EventParameter>, anonymous: bool) -> EventDefinition {
        EventDefinition(n, v, anonymous)
    }
}

#[derive(Debug,PartialEq)]
pub struct EnumDefinition(
    String,
    Vec<String>,
);

impl EnumDefinition {
    pub fn new(n: String, v: Vec<String>) -> EnumDefinition {
        EnumDefinition(n, v)
    }
}

#[derive(Debug,PartialEq)]
pub enum VariableAttribute {
    Public,
    Internal,
    Private,
    Constant
}

#[derive(Debug,PartialEq)]
pub struct StateVariableDeclaration(
    ElementaryTypeName,
    Vec<VariableAttribute>,
    String,
    Option<Expression>
);

impl StateVariableDeclaration {
    pub fn new(e: ElementaryTypeName, a: Vec<VariableAttribute>, i: String, n: Option<Expression>) -> StateVariableDeclaration {
        StateVariableDeclaration(e, a, i, n)
    }
}

#[derive(Debug,PartialEq)]
pub enum Expression {
    PostIncrement(Box<Expression>),
    PostDecrement(Box<Expression>),
    New(ElementaryTypeName),
    IndexAccess(Box<Expression>, Box<Option<Expression>>),
    MemberAccess(Box<Expression>, String),
    FunctionCall(String, Vec<Expression>),
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
    Variable(String),
}

#[derive(Debug,PartialEq)]
pub struct Parameter(
    pub ElementaryTypeName,
    pub Option<StorageLocation>,
    pub Option<String>
);

impl Parameter {
    pub fn new(e: ElementaryTypeName, s: Option<StorageLocation>, i: Option<String>) -> Parameter {
        Parameter(e, s, i)
    }
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
    pub name: Option<String>,
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
    Emit(String, Vec<Expression>),
    Empty
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

        let a = SourceUnit("".to_string(), vec![
            SourceUnitPart::ContractDefinition(
                Box::new(ContractDefinition(ContractType::Contract, "foo".to_string(), vec![
                    ContractPart::StructDefinition(Box::new(StructDefinition("Jurisdiction".to_string(), vec![
                        Box::new(VariableDeclaration(ElementaryTypeName::Bool, StorageLocation::Default, "exists".to_string())),
                        Box::new(VariableDeclaration(ElementaryTypeName::Uint(256), StorageLocation::Default, "keyIdx".to_string())),
                        Box::new(VariableDeclaration(ElementaryTypeName::Bytes(2), StorageLocation::Default, "country".to_string())),
                        Box::new(VariableDeclaration(ElementaryTypeName::Bytes(32), StorageLocation::Default, "region".to_string()))
                    ]))),
                    ContractPart::StateVariableDeclaration(Box::new(StateVariableDeclaration(ElementaryTypeName::String, vec![], "__abba_$".to_string(), None))),
                    ContractPart::StateVariableDeclaration(Box::new(StateVariableDeclaration(ElementaryTypeName::Int(64), vec![], "$thing_102".to_string(), None)))
            ])))
        ]);

        assert_eq!(e, a);
    }
}
