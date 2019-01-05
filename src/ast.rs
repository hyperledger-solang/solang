use num_bigint::BigInt;
use std::collections::HashMap;

#[derive(Debug,PartialEq)]
pub struct SourceUnit {
    pub name: String,
    pub parts: Vec<SourceUnitPart>
}

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
pub struct VariableDeclaration {
    pub typ: ElementaryTypeName,
    pub storage: StorageLocation,
    pub name: String
}

#[derive(Debug,PartialEq)]
pub struct StructDefinition {
    pub name: String,
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
    pub name: String,
    pub parts: Vec<ContractPart>,
}

#[derive(Debug,PartialEq)]
pub struct EventParameter {
    pub typ: ElementaryTypeName,
    pub indexed: bool,
    pub name: Option<String>,
}

#[derive(Debug,PartialEq)]
pub struct EventDefinition {
    pub name: String,
    pub fields: Vec<EventParameter>,
    pub anonymous: bool,
}

#[derive(Debug,PartialEq)]
pub struct EnumDefinition {
    pub name: String,
    pub values: Vec<String>,
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
    pub name: String,
    pub initializer: Option<Expression>,
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
pub struct Parameter {
    pub typ: ElementaryTypeName,
    pub storage: Option<StorageLocation>,
    pub name: Option<String>
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

        let a = SourceUnit{name: "".to_string(), parts: vec![
            SourceUnitPart::ContractDefinition(
                Box::new(ContractDefinition{typ: ContractType::Contract, name: "foo".to_string(), parts: vec![
                    ContractPart::StructDefinition(Box::new(StructDefinition{name: "Jurisdiction".to_string(), fields: vec![
                        Box::new(VariableDeclaration{
                            typ: ElementaryTypeName::Bool, storage: StorageLocation::Default, name: "exists".to_string()
                        }),
                        Box::new(VariableDeclaration{
                            typ: ElementaryTypeName::Uint(256), storage: StorageLocation::Default, name: "keyIdx".to_string()
                        }),
                        Box::new(VariableDeclaration{
                            typ: ElementaryTypeName::Bytes(2), storage: StorageLocation::Default, name: "country".to_string()
                        }),
                        Box::new(VariableDeclaration{
                            typ: ElementaryTypeName::Bytes(32), storage: StorageLocation::Default, name: "region".to_string()
                        })
                    ]})),
                    ContractPart::StateVariableDeclaration(Box::new(StateVariableDeclaration{
                        typ: ElementaryTypeName::String, attrs: vec![], name: "__abba_$".to_string(), initializer: None
                    })),
                    ContractPart::StateVariableDeclaration(Box::new(StateVariableDeclaration{
                        typ: ElementaryTypeName::Int(64), attrs: vec![], name: "$thing_102".to_string(), initializer: None
                    }))
            ]}))
        ]};

        assert_eq!(e, a);
    }
}
