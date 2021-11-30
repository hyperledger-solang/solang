use num_bigint::BigInt;
use num_rational::BigRational;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
/// file no, start offset, end offset (in bytes)
pub struct Loc(pub usize, pub usize, pub usize);

impl Loc {
    pub fn begin(&self) -> Self {
        Loc(self.0, self.1, self.1)
    }

    pub fn end(&self) -> Self {
        Loc(self.0, self.2, self.2)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Identifier {
    pub loc: Loc,
    pub name: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct DocComment {
    pub offset: usize,
    pub tag: String,
    pub value: String,
}

#[derive(Debug, PartialEq)]
pub struct SourceUnit(pub Vec<SourceUnitPart>);

#[derive(Debug, PartialEq)]
pub enum SourceUnitPart {
    ContractDefinition(Box<ContractDefinition>),
    PragmaDirective(Vec<DocComment>, Identifier, StringLiteral),
    ImportDirective(Vec<DocComment>, Import),
    EnumDefinition(Box<EnumDefinition>),
    StructDefinition(Box<StructDefinition>),
    EventDefinition(Box<EventDefinition>),
    FunctionDefinition(Box<FunctionDefinition>),
    VariableDefinition(Box<VariableDefinition>),
    StraySemicolon(Loc),
}

#[derive(Debug, PartialEq)]
pub enum Import {
    Plain(StringLiteral),
    GlobalSymbol(StringLiteral, Identifier),
    Rename(StringLiteral, Vec<(Identifier, Option<Identifier>)>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    Address,
    AddressPayable,
    Payable,
    Bool,
    String,
    Int(u16),
    Uint(u16),
    Bytes(u8),
    Rational,
    DynamicBytes,
    Mapping(Loc, Box<Expression>, Box<Expression>),
    Function {
        params: Vec<(Loc, Option<Parameter>)>,
        attributes: Vec<FunctionAttribute>,
        returns: Vec<(Loc, Option<Parameter>)>,
        trailing_attributes: Vec<FunctionAttribute>,
    },
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, PartialEq, Clone)]
pub struct VariableDeclaration {
    pub loc: Loc,
    pub ty: Expression,
    pub storage: Option<StorageLocation>,
    pub name: Identifier,
}

#[derive(Debug, PartialEq)]
#[allow(clippy::vec_box)]
pub struct StructDefinition {
    pub doc: Vec<DocComment>,
    pub loc: Loc,
    pub name: Identifier,
    pub fields: Vec<VariableDeclaration>,
}

#[derive(Debug, PartialEq)]
pub enum ContractPart {
    StructDefinition(Box<StructDefinition>),
    EventDefinition(Box<EventDefinition>),
    EnumDefinition(Box<EnumDefinition>),
    VariableDefinition(Box<VariableDefinition>),
    FunctionDefinition(Box<FunctionDefinition>),
    StraySemicolon(Loc),
    Using(Box<Using>),
}

#[derive(Debug, PartialEq)]
pub struct Using {
    pub loc: Loc,
    pub library: Identifier,
    pub ty: Option<Expression>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ContractTy {
    Abstract(Loc),
    Contract(Loc),
    Interface(Loc),
    Library(Loc),
}

impl fmt::Display for ContractTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractTy::Abstract(_) => write!(f, "abstract contract"),
            ContractTy::Contract(_) => write!(f, "contract"),
            ContractTy::Interface(_) => write!(f, "interface"),
            ContractTy::Library(_) => write!(f, "library"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Base {
    pub loc: Loc,
    pub name: Identifier,
    pub args: Option<Vec<Expression>>,
}

#[derive(Debug, PartialEq)]
pub struct ContractDefinition {
    pub doc: Vec<DocComment>,
    pub loc: Loc,
    pub ty: ContractTy,
    pub name: Identifier,
    pub base: Vec<Base>,
    pub parts: Vec<ContractPart>,
}

#[derive(Debug, PartialEq)]
pub struct EventParameter {
    pub ty: Expression,
    pub loc: Loc,
    pub indexed: bool,
    pub name: Option<Identifier>,
}

#[derive(Debug, PartialEq)]
pub struct EventDefinition {
    pub doc: Vec<DocComment>,
    pub loc: Loc,
    pub name: Identifier,
    pub fields: Vec<EventParameter>,
    pub anonymous: bool,
}

#[derive(Debug, PartialEq)]
pub struct EnumDefinition {
    pub doc: Vec<DocComment>,
    pub loc: Loc,
    pub name: Identifier,
    pub values: Vec<Identifier>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum VariableAttribute {
    Visibility(Visibility),
    Constant(Loc),
    Immutable(Loc),
    Override(Loc),
}

#[derive(Debug, PartialEq)]
pub struct VariableDefinition {
    pub doc: Vec<DocComment>,
    pub loc: Loc,
    pub ty: Expression,
    pub attrs: Vec<VariableAttribute>,
    pub name: Identifier,
    pub initializer: Option<Expression>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct StringLiteral {
    pub loc: Loc,
    pub string: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct HexLiteral {
    pub loc: Loc,
    pub hex: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct NamedArgument {
    pub loc: Loc,
    pub name: Identifier,
    pub expr: Expression,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Unit {
    Seconds(Loc),
    Minutes(Loc),
    Hours(Loc),
    Days(Loc),
    Weeks(Loc),
    Wei(Loc),
    Szabo(Loc),
    Finney(Loc),
    Ether(Loc),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    PostIncrement(Loc, Box<Expression>),
    PostDecrement(Loc, Box<Expression>),
    New(Loc, Box<Expression>),
    ArraySubscript(Loc, Box<Expression>, Option<Box<Expression>>),
    MemberAccess(Loc, Box<Expression>, Identifier),
    FunctionCall(Loc, Box<Expression>, Vec<Expression>),
    FunctionCallBlock(Loc, Box<Expression>, Box<Statement>),
    NamedFunctionCall(Loc, Box<Expression>, Vec<NamedArgument>),
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
    RationalNumberLiteral(Loc, BigRational),
    HexNumberLiteral(Loc, String),
    StringLiteral(Vec<StringLiteral>),
    Type(Loc, Type),
    HexLiteral(Vec<HexLiteral>),
    AddressLiteral(Loc, String),
    Variable(Identifier),
    List(Loc, Vec<(Loc, Option<Parameter>)>),
    ArrayLiteral(Loc, Vec<Expression>),
    Unit(Loc, Box<Expression>, Unit),
    This(Loc),
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
            | Expression::FunctionCallBlock(loc, _, _)
            | Expression::NamedFunctionCall(loc, _, _)
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
            | Expression::RationalNumberLiteral(loc, _)
            | Expression::HexNumberLiteral(loc, _)
            | Expression::ArrayLiteral(loc, _)
            | Expression::List(loc, _)
            | Expression::Type(loc, _)
            | Expression::Unit(loc, _, _)
            | Expression::This(loc)
            | Expression::Variable(Identifier { loc, .. })
            | Expression::AddressLiteral(loc, _) => *loc,
            Expression::StringLiteral(v) => v[0].loc,
            Expression::HexLiteral(v) => v[0].loc,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    pub loc: Loc,
    pub ty: Expression,
    pub storage: Option<StorageLocation>,
    pub name: Option<Identifier>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Mutability {
    Pure(Loc),
    View(Loc),
    Constant(Loc),
    Payable(Loc),
}

impl fmt::Display for Mutability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mutability::Pure(_) => write!(f, "pure"),
            Mutability::Constant(_) | Mutability::View(_) => write!(f, "view"),
            Mutability::Payable(_) => write!(f, "payable"),
        }
    }
}

impl Mutability {
    pub fn loc(&self) -> Loc {
        match self {
            Mutability::Pure(loc)
            | Mutability::Constant(loc)
            | Mutability::View(loc)
            | Mutability::Payable(loc) => *loc,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Visibility {
    External(Option<Loc>),
    Public(Option<Loc>),
    Internal(Option<Loc>),
    Private(Option<Loc>),
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Public(_) => write!(f, "public"),
            Visibility::External(_) => write!(f, "external"),
            Visibility::Internal(_) => write!(f, "internal"),
            Visibility::Private(_) => write!(f, "private"),
        }
    }
}

impl Visibility {
    pub fn loc(&self) -> Option<Loc> {
        match self {
            Visibility::Public(loc)
            | Visibility::External(loc)
            | Visibility::Internal(loc)
            | Visibility::Private(loc) => *loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionAttribute {
    Mutability(Mutability),
    Visibility(Visibility),
    Virtual(Loc),
    Override(Loc, Vec<Identifier>),
    BaseOrModifier(Loc, Base),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FunctionTy {
    Constructor,
    Function,
    Fallback,
    Receive,
    Modifier,
}

impl fmt::Display for FunctionTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FunctionTy::Constructor => write!(f, "constructor"),
            FunctionTy::Function => write!(f, "function"),
            FunctionTy::Fallback => write!(f, "fallback"),
            FunctionTy::Receive => write!(f, "receive"),
            FunctionTy::Modifier => write!(f, "modifier"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FunctionDefinition {
    pub doc: Vec<DocComment>,
    pub loc: Loc,
    pub ty: FunctionTy,
    pub name: Option<Identifier>,
    pub name_loc: Loc,
    pub params: Vec<(Loc, Option<Parameter>)>,
    pub attributes: Vec<FunctionAttribute>,
    pub return_not_returns: Option<Loc>,
    pub returns: Vec<(Loc, Option<Parameter>)>,
    pub body: Option<Statement>,
}

#[derive(Debug, PartialEq, Clone)]
#[allow(clippy::large_enum_variant, clippy::type_complexity)]
pub enum Statement {
    Block {
        loc: Loc,
        unchecked: bool,
        statements: Vec<Statement>,
    },
    Assembly {
        loc: Loc,
        assembly: Vec<AssemblyStatement>,
    },
    Args(Loc, Vec<NamedArgument>),
    If(Loc, Expression, Box<Statement>, Option<Box<Statement>>),
    While(Loc, Expression, Box<Statement>),
    Expression(Loc, Expression),
    VariableDefinition(Loc, VariableDeclaration, Option<Expression>),
    For(
        Loc,
        Option<Box<Statement>>,
        Option<Box<Expression>>,
        Option<Box<Statement>>,
        Option<Box<Statement>>,
    ),
    DoWhile(Loc, Box<Statement>, Expression),
    Continue(Loc),
    Break(Loc),
    Return(Loc, Option<Expression>),
    Emit(Loc, Expression),
    Try(
        Loc,
        Expression,
        Option<(Vec<(Loc, Option<Parameter>)>, Box<Statement>)>,
        Option<Box<(Identifier, Parameter, Statement)>>,
        Box<(Option<Parameter>, Statement)>,
    ),
}

#[derive(Debug, PartialEq, Clone)]
pub enum AssemblyStatement {
    Assign(Loc, AssemblyExpression, AssemblyExpression),
    LetAssign(Loc, AssemblyExpression, AssemblyExpression),
    Expression(AssemblyExpression),
}

#[derive(Debug, PartialEq, Clone)]
pub enum AssemblyExpression {
    BoolLiteral(Loc, bool),
    NumberLiteral(Loc, BigInt),
    HexNumberLiteral(Loc, String),
    StringLiteral(StringLiteral),
    Variable(Identifier),
    Assign(Loc, Box<AssemblyExpression>, Box<AssemblyExpression>),
    LetAssign(Loc, Box<AssemblyExpression>, Box<AssemblyExpression>),
    Function(Loc, Box<AssemblyExpression>, Vec<AssemblyExpression>),
    Member(Loc, Box<AssemblyExpression>, Identifier),
    Subscript(Loc, Box<AssemblyExpression>, Box<AssemblyExpression>),
}

impl Statement {
    pub fn loc(&self) -> Loc {
        match self {
            Statement::Block { loc, .. }
            | Statement::Assembly { loc, .. }
            | Statement::Args(loc, _)
            | Statement::If(loc, _, _, _)
            | Statement::While(loc, _, _)
            | Statement::Expression(loc, _)
            | Statement::VariableDefinition(loc, _, _)
            | Statement::For(loc, _, _, _, _)
            | Statement::DoWhile(loc, _, _)
            | Statement::Continue(loc)
            | Statement::Break(loc)
            | Statement::Return(loc, _)
            | Statement::Emit(loc, _)
            | Statement::Try(loc, _, _, _, _) => *loc,
        }
    }
}
