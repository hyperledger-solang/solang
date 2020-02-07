use super::lexer::LexicalError;
use num_bigint::BigInt;
use std::fmt;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Loc(pub usize, pub usize);

#[derive(Debug, PartialEq, Clone)]
pub struct Identifier {
    pub loc: Loc,
    pub name: String,
}

#[derive(Debug, PartialEq)]
pub struct SourceUnit(pub Vec<SourceUnitPart>);

#[derive(Debug, PartialEq)]
pub enum SourceUnitPart {
    ContractDefinition(Box<ContractDefinition>),
    PragmaDirective(Identifier, StringLiteral),
    ImportDirective(StringLiteral),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PrimitiveType {
    Address,
    Bool,
    String,
    Int(u16),
    Uint(u16),
    Bytes(u8),
    DynamicBytes,
}

impl PrimitiveType {
    pub fn signed(self) -> bool {
        match self {
            PrimitiveType::Int(_) => true,
            _ => false,
        }
    }

    pub fn ordered(self) -> bool {
        match self {
            PrimitiveType::Int(_) => true,
            PrimitiveType::Uint(_) => true,
            _ => false,
        }
    }

    pub fn bits(self) -> u16 {
        match self {
            PrimitiveType::Address => 160,
            PrimitiveType::Bool => 1,
            PrimitiveType::Int(n) => n,
            PrimitiveType::Uint(n) => n,
            PrimitiveType::Bytes(n) => n as u16 * 8,
            _ => panic!("{} not fixed size", self.to_string()),
        }
    }
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveType::Address => write!(f, "address"),
            PrimitiveType::Bool => write!(f, "bool"),
            PrimitiveType::String => write!(f, "string"),
            PrimitiveType::Int(n) => write!(f, "int{}", n),
            PrimitiveType::Uint(n) => write!(f, "uint{}", n),
            PrimitiveType::Bytes(n) => write!(f, "bytes{}", n),
            PrimitiveType::DynamicBytes => write!(f, "bytes"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    Primitive(PrimitiveType, Vec<Option<(Loc, BigInt)>>),
    Unresolved(Identifier, Vec<Option<(Loc, BigInt)>>),
}

#[derive(Debug, PartialEq)]
pub enum StorageLocation {
    Memory(Loc),
    Storage(Loc),
    Calldata(Loc),
}

impl StorageLocation {
    pub fn loc(&self) -> &Loc {
        match self {
            StorageLocation::Memory(l) => l,
            StorageLocation::Storage(l) => l,
            StorageLocation::Calldata(l) => l,
        }
    }
}

impl fmt::Display for StorageLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageLocation::Memory(_) => write!(f, "memory"),
            StorageLocation::Storage(_) => write!(f, "storage"),
            StorageLocation::Calldata(_) => write!(f, "calldata"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct VariableDeclaration {
    pub typ: Type,
    pub storage: Option<StorageLocation>,
    pub name: Identifier,
}

#[derive(Debug, PartialEq)]
#[allow(clippy::vec_box)]
pub struct StructDefinition {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub fields: Vec<VariableDeclaration>,
}

#[derive(Debug, PartialEq)]
pub enum ContractPart {
    StructDefinition(Box<StructDefinition>),
    EventDefinition(Box<EventDefinition>),
    EnumDefinition(Box<EnumDefinition>),
    ContractVariableDefinition(Box<ContractVariableDefinition>),
    FunctionDefinition(Box<FunctionDefinition>),
}

#[derive(Debug, PartialEq)]
pub enum ContractType {
    Contract,
    Interface,
    Library,
}

#[derive(Debug, PartialEq)]
pub struct ContractDefinition {
    pub doc: Vec<String>,
    pub loc: Loc,
    pub ty: ContractType,
    pub name: Identifier,
    pub parts: Vec<ContractPart>,
}

#[derive(Debug, PartialEq)]
pub struct EventParameter {
    pub typ: Type,
    pub indexed: bool,
    pub name: Option<Identifier>,
}

#[derive(Debug, PartialEq)]
pub struct EventDefinition {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub fields: Vec<EventParameter>,
    pub anonymous: bool,
}

#[derive(Debug, PartialEq)]
pub struct EnumDefinition {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub values: Vec<Identifier>,
}

#[derive(Debug, PartialEq)]
pub enum VariableAttribute {
    Visibility(Visibility),
    Constant(Loc),
}

#[derive(Debug, PartialEq)]
pub struct ContractVariableDefinition {
    pub doc: Vec<String>,
    pub loc: Loc,
    pub ty: Type,
    pub attrs: Vec<VariableAttribute>,
    pub name: Identifier,
    pub initializer: Option<Expression>,
}

#[derive(Debug, PartialEq)]
pub struct StringLiteral {
    pub loc: Loc,
    pub string: String,
}

#[derive(Debug, PartialEq)]
pub struct HexLiteral {
    pub loc: Loc,
    pub hex: String,
}

#[derive(Debug, PartialEq)]
pub enum Expression {
    PostIncrement(Loc, Box<Expression>),
    PostDecrement(Loc, Box<Expression>),
    New(Loc, Type),
    ArraySubscript(Loc, Box<Expression>, Option<Box<Expression>>),
    MemberAccess(Loc, Identifier, Identifier),
    FunctionCall(Loc, Type, Vec<Expression>),
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
    AddressLiteral(Loc, String),
    StringLiteral(Vec<StringLiteral>),
    HexLiteral(Vec<HexLiteral>),
    Variable(Identifier),
    ArrayLiteral(Loc, Vec<Expression>),
}

impl Expression {
    pub fn loc(&self) -> Loc {
        match self {
            Expression::PostIncrement(loc, _)
            | Expression::PostDecrement(loc, _)
            | Expression::New(loc, _)
            | Expression::ArraySubscript(loc, _, _)
            | Expression::MemberAccess(loc, _, _)
            | Expression::FunctionCall(loc, _, _)
            | Expression::Not(loc, _)
            | Expression::Complement(loc, _)
            | Expression::Delete(loc, _)
            | Expression::PreIncrement(loc, _)
            | Expression::PreDecrement(loc, _)
            | Expression::UnaryPlus(loc, _)
            | Expression::UnaryMinus(loc, _)
            | Expression::Power(loc, _, _)
            | Expression::Multiply(loc, _, _)
            | Expression::Divide(loc, _, _)
            | Expression::Modulo(loc, _, _)
            | Expression::Add(loc, _, _)
            | Expression::Subtract(loc, _, _)
            | Expression::ShiftLeft(loc, _, _)
            | Expression::ShiftRight(loc, _, _)
            | Expression::BitwiseAnd(loc, _, _)
            | Expression::BitwiseXor(loc, _, _)
            | Expression::BitwiseOr(loc, _, _)
            | Expression::Less(loc, _, _)
            | Expression::More(loc, _, _)
            | Expression::LessEqual(loc, _, _)
            | Expression::MoreEqual(loc, _, _)
            | Expression::Equal(loc, _, _)
            | Expression::NotEqual(loc, _, _)
            | Expression::And(loc, _, _)
            | Expression::Or(loc, _, _)
            | Expression::Ternary(loc, _, _, _)
            | Expression::Assign(loc, _, _)
            | Expression::AssignOr(loc, _, _)
            | Expression::AssignAnd(loc, _, _)
            | Expression::AssignXor(loc, _, _)
            | Expression::AssignShiftLeft(loc, _, _)
            | Expression::AssignShiftRight(loc, _, _)
            | Expression::AssignAdd(loc, _, _)
            | Expression::AssignSubtract(loc, _, _)
            | Expression::AssignMultiply(loc, _, _)
            | Expression::AssignDivide(loc, _, _)
            | Expression::AssignModulo(loc, _, _)
            | Expression::BoolLiteral(loc, _)
            | Expression::NumberLiteral(loc, _)
            | Expression::AddressLiteral(loc, _)
            | Expression::ArrayLiteral(loc, _)
            | Expression::Variable(Identifier { loc, .. }) => *loc,
            Expression::StringLiteral(v) => v[0].loc,
            Expression::HexLiteral(v) => v[0].loc,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Parameter {
    pub typ: Type,
    pub storage: Option<StorageLocation>,
    pub name: Option<Identifier>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum StateMutability {
    Pure(Loc),
    View(Loc),
    Payable(Loc),
}

impl StateMutability {
    pub fn to_string(&self) -> &'static str {
        match self {
            StateMutability::Pure(_) => "pure",
            StateMutability::View(_) => "view",
            StateMutability::Payable(_) => "payable",
        }
    }

    pub fn loc(&self) -> Loc {
        match self {
            StateMutability::Pure(loc)
            | StateMutability::View(loc)
            | StateMutability::Payable(loc) => *loc,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Visibility {
    External(Loc),
    Public(Loc),
    Internal(Loc),
    Private(Loc),
}

impl Visibility {
    pub fn to_string(&self) -> &'static str {
        match self {
            Visibility::Public(_) => "public",
            Visibility::External(_) => "external",
            Visibility::Internal(_) => "internal",
            Visibility::Private(_) => "private",
        }
    }

    pub fn loc(&self) -> Loc {
        match self {
            Visibility::Public(loc)
            | Visibility::External(loc)
            | Visibility::Internal(loc)
            | Visibility::Private(loc) => *loc,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum FunctionAttribute {
    StateMutability(StateMutability),
    Visibility(Visibility),
}

#[derive(Debug, PartialEq)]
pub struct FunctionDefinition {
    pub doc: Vec<String>,
    pub loc: Loc,
    pub constructor: bool,
    pub name: Option<Identifier>,
    pub params: Vec<Parameter>,
    pub attributes: Vec<FunctionAttribute>,
    pub returns: Vec<Parameter>,
    pub body: Statement,
}

#[derive(Debug, PartialEq)]
pub struct BlockStatement(pub Vec<Statement>);

#[derive(Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Statement {
    BlockStatement(BlockStatement),
    If(Expression, Box<Statement>, Option<Box<Statement>>),
    While(Expression, Box<Statement>),
    PlaceHolder,
    Expression(Expression),
    VariableDefinition(VariableDeclaration, Option<Expression>),
    For(
        Option<Box<Statement>>,
        Option<Box<Expression>>,
        Option<Box<Statement>>,
        Option<Box<Statement>>,
    ),
    DoWhile(Box<Statement>, Expression),
    Continue,
    Break,
    Return(Loc, Vec<Expression>),
    Throw,
    Emit(Identifier, Vec<Expression>),
    Empty,
}

impl Statement {
    pub fn loc(&self) -> Loc {
        // FIXME add to parser
        Loc(0, 0)
    }
}

// An array type can look like foo[2], if foo is an enum type. The lalrpop parses
// this as an expression, so we need to convert it to Type and check there are
// no unexpected expressions types.
pub fn expr_to_type(expr: Expression) -> Result<Type, LexicalError> {
    let mut dimensions = Vec::new();

    let mut expr = expr;

    loop {
        expr = match expr {
            Expression::ArraySubscript(_, r, None) => {
                dimensions.push(None);

                *r
            }
            Expression::ArraySubscript(_, r, Some(index)) => {
                let loc = index.loc();
                dimensions.push(match *index {
                    Expression::NumberLiteral(_, n) => Some((loc, n)),
                    _ => {
                        return Err(LexicalError::UnexpectedExpressionArrayDimension(
                            loc.0, loc.1,
                        ))
                    }
                });
                *r
            }
            Expression::Variable(id) => return Ok(Type::Unresolved(id, dimensions)),
            _ => {
                let loc = expr.loc();
                return Err(LexicalError::NonIdentifierInTypeName(loc.0, loc.1));
            }
        }
    }
}
