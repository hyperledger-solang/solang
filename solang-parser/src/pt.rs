use std::fmt;

use num_bigint::BigInt;
use num_rational::BigRational;

use crate::lexer::CommentType;

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Hash, Clone, Copy)]
/// file no, start offset, end offset (in bytes)
pub enum Loc {
    Builtin,
    CommandLine,
    Implicit,
    Codegen,
    File(usize, usize, usize),
}

/// Structs can implement this trait to easily return their loc
pub trait CodeLocation {
    fn loc(&self) -> Loc;
}

/// Structs should implement this trait to return an optional location
pub trait OptionalCodeLocation {
    fn loc(&self) -> Option<Loc>;
}

impl Loc {
    #[must_use]
    pub fn begin_range(&self) -> Self {
        match self {
            Loc::File(file_no, start, _) => Loc::File(*file_no, *start, *start),
            loc => *loc,
        }
    }

    #[must_use]
    pub fn end_range(&self) -> Self {
        match self {
            Loc::File(file_no, _, end) => Loc::File(*file_no, *end, *end),
            loc => *loc,
        }
    }

    pub fn file_no(&self) -> usize {
        match self {
            Loc::File(file_no, _, _) => *file_no,
            _ => unreachable!(),
        }
    }

    pub fn start(&self) -> usize {
        match self {
            Loc::File(_, start, _) => *start,
            _ => unreachable!(),
        }
    }

    pub fn end(&self) -> usize {
        match self {
            Loc::File(_, _, end) => *end,
            _ => unreachable!(),
        }
    }

    pub fn use_end_from(&mut self, other: &Loc) {
        match (self, other) {
            (Loc::File(_, _, end), Loc::File(_, _, other_end)) => {
                *end = *other_end;
            }
            _ => unreachable!(),
        }
    }

    pub fn use_start_from(&mut self, other: &Loc) {
        match (self, other) {
            (Loc::File(_, start, _), Loc::File(_, other_start, _)) => {
                *start = *other_start;
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Identifier {
    pub loc: Loc,
    pub name: String,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Comment {
    Line(Loc, String),
    Block(Loc, String),
}

#[derive(Debug, PartialEq, Clone)]
pub enum DocComment {
    Line { comment: SingleDocComment },
    Block { comments: Vec<SingleDocComment> },
}

impl DocComment {
    pub fn comments(&self) -> Vec<&SingleDocComment> {
        match self {
            DocComment::Line { comment } => vec![comment],
            DocComment::Block { comments } => comments.iter().collect(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct SingleDocComment {
    pub offset: usize,
    pub tag: String,
    pub value: String,
}

#[derive(Debug, PartialEq)]
pub struct SourceUnit(pub Vec<SourceUnitPart>);

#[derive(Debug, PartialEq)]
pub enum SourceUnitPart {
    ContractDefinition(Box<ContractDefinition>),
    PragmaDirective(Loc, Vec<DocComment>, Identifier, StringLiteral),
    ImportDirective(Vec<DocComment>, Import),
    EnumDefinition(Box<EnumDefinition>),
    StructDefinition(Box<StructDefinition>),
    EventDefinition(Box<EventDefinition>),
    ErrorDefinition(Box<ErrorDefinition>),
    FunctionDefinition(Box<FunctionDefinition>),
    VariableDefinition(Box<VariableDefinition>),
    StraySemicolon(Loc),
}

#[derive(Debug, PartialEq)]
pub enum Import {
    Plain(StringLiteral, Loc),
    GlobalSymbol(StringLiteral, Identifier, Loc),
    Rename(StringLiteral, Vec<(Identifier, Option<Identifier>)>, Loc),
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

impl CodeLocation for StorageLocation {
    fn loc(&self) -> Loc {
        match self {
            StorageLocation::Memory(l)
            | StorageLocation::Storage(l)
            | StorageLocation::Calldata(l) => *l,
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
    ErrorDefinition(Box<ErrorDefinition>),
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
pub struct ErrorParameter {
    pub ty: Expression,
    pub loc: Loc,
    pub name: Option<Identifier>,
}

#[derive(Debug, PartialEq)]
pub struct ErrorDefinition {
    pub doc: Vec<DocComment>,
    pub loc: Loc,
    pub name: Identifier,
    pub fields: Vec<ErrorParameter>,
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
    ArraySlice(
        Loc,
        Box<Expression>,
        Option<Box<Expression>>,
        Option<Box<Expression>>,
    ),
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

impl CodeLocation for Expression {
    fn loc(&self) -> Loc {
        match self {
            Expression::PostIncrement(loc, _)
            | Expression::PostDecrement(loc, _)
            | Expression::New(loc, _)
            | Expression::ArraySubscript(loc, ..)
            | Expression::ArraySlice(loc, ..)
            | Expression::MemberAccess(loc, ..)
            | Expression::FunctionCall(loc, ..)
            | Expression::FunctionCallBlock(loc, ..)
            | Expression::NamedFunctionCall(loc, ..)
            | Expression::Not(loc, _)
            | Expression::Complement(loc, _)
            | Expression::Delete(loc, _)
            | Expression::PreIncrement(loc, _)
            | Expression::PreDecrement(loc, _)
            | Expression::UnaryPlus(loc, _)
            | Expression::UnaryMinus(loc, _)
            | Expression::Power(loc, ..)
            | Expression::Multiply(loc, ..)
            | Expression::Divide(loc, ..)
            | Expression::Modulo(loc, ..)
            | Expression::Add(loc, ..)
            | Expression::Subtract(loc, ..)
            | Expression::ShiftLeft(loc, ..)
            | Expression::ShiftRight(loc, ..)
            | Expression::BitwiseAnd(loc, ..)
            | Expression::BitwiseXor(loc, ..)
            | Expression::BitwiseOr(loc, ..)
            | Expression::Less(loc, ..)
            | Expression::More(loc, ..)
            | Expression::LessEqual(loc, ..)
            | Expression::MoreEqual(loc, ..)
            | Expression::Equal(loc, ..)
            | Expression::NotEqual(loc, ..)
            | Expression::And(loc, ..)
            | Expression::Or(loc, ..)
            | Expression::Ternary(loc, ..)
            | Expression::Assign(loc, ..)
            | Expression::AssignOr(loc, ..)
            | Expression::AssignAnd(loc, ..)
            | Expression::AssignXor(loc, ..)
            | Expression::AssignShiftLeft(loc, ..)
            | Expression::AssignShiftRight(loc, ..)
            | Expression::AssignAdd(loc, ..)
            | Expression::AssignSubtract(loc, ..)
            | Expression::AssignMultiply(loc, ..)
            | Expression::AssignDivide(loc, ..)
            | Expression::AssignModulo(loc, ..)
            | Expression::BoolLiteral(loc, _)
            | Expression::NumberLiteral(loc, _)
            | Expression::RationalNumberLiteral(loc, _)
            | Expression::HexNumberLiteral(loc, _)
            | Expression::ArrayLiteral(loc, _)
            | Expression::List(loc, _)
            | Expression::Type(loc, _)
            | Expression::Unit(loc, ..)
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

impl CodeLocation for Mutability {
    fn loc(&self) -> Loc {
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

impl OptionalCodeLocation for Visibility {
    fn loc(&self) -> Option<Loc> {
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
        dialect: Option<StringLiteral>,
        block: AssemblyBlock,
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
    Revert(Loc, Option<Identifier>, Vec<Expression>),
    Emit(Loc, Expression),
    Try(
        Loc,
        Expression,
        Option<(Vec<(Loc, Option<Parameter>)>, Box<Statement>)>,
        Vec<CatchClause>,
    ),
    DocComment(Loc, CommentType, String),
}

#[derive(Debug, PartialEq, Clone)]
pub enum CatchClause {
    Simple(Loc, Option<Parameter>, Statement),
    Named(Loc, Identifier, Parameter, Statement),
}

#[derive(Debug, PartialEq, Clone)]
pub enum AssemblyStatement {
    Assign(Loc, Vec<AssemblyExpression>, AssemblyExpression),
    VariableDeclaration(
        Loc,
        Vec<AssemblyTypedIdentifier>,
        Option<AssemblyExpression>,
    ),
    If(Loc, AssemblyExpression, AssemblyBlock),
    For(AssemblyFor),
    Switch(AssemblySwitch),
    Leave(Loc),
    Break(Loc),
    Continue(Loc),
    Block(AssemblyBlock),
    FunctionDefinition(Box<AssemblyFunctionDefinition>),
    FunctionCall(Box<AssemblyFunctionCall>),
}
#[derive(PartialEq, Clone, Debug)]
pub struct AssemblySwitch {
    pub loc: Loc,
    pub condition: AssemblyExpression,
    pub cases: Vec<AssemblySwitchOptions>,
    pub default: Option<AssemblySwitchOptions>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct AssemblyFor {
    pub loc: Loc,
    pub init_block: AssemblyBlock,
    pub condition: AssemblyExpression,
    pub post_block: AssemblyBlock,
    pub execution_block: AssemblyBlock,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AssemblyBlock {
    pub loc: Loc,
    pub statements: Vec<AssemblyStatement>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum AssemblyExpression {
    BoolLiteral(Loc, bool, Option<Identifier>),
    NumberLiteral(Loc, BigInt, Option<Identifier>),
    HexNumberLiteral(Loc, String, Option<Identifier>),
    HexStringLiteral(HexLiteral, Option<Identifier>),
    StringLiteral(StringLiteral, Option<Identifier>),
    Variable(Identifier),
    FunctionCall(Box<AssemblyFunctionCall>),
    Member(Loc, Box<AssemblyExpression>, Identifier),
}

#[derive(Debug, PartialEq, Clone)]
pub struct AssemblyTypedIdentifier {
    pub loc: Loc,
    pub id: Identifier,
    pub ty: Option<Identifier>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AssemblyFunctionDefinition {
    pub loc: Loc,
    pub id: Identifier,
    pub params: Vec<AssemblyTypedIdentifier>,
    pub returns: Vec<AssemblyTypedIdentifier>,
    pub body: AssemblyBlock,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AssemblyFunctionCall {
    pub loc: Loc,
    pub id: Identifier,
    pub arguments: Vec<AssemblyExpression>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum AssemblySwitchOptions {
    Case(Loc, AssemblyExpression, AssemblyBlock),
    Default(Loc, AssemblyBlock),
}

impl CodeLocation for AssemblySwitchOptions {
    fn loc(&self) -> Loc {
        match self {
            AssemblySwitchOptions::Case(loc, ..) | AssemblySwitchOptions::Default(loc, ..) => *loc,
        }
    }
}

impl CodeLocation for Statement {
    fn loc(&self) -> Loc {
        match self {
            Statement::Block { loc, .. }
            | Statement::Assembly { loc, .. }
            | Statement::Args(loc, ..)
            | Statement::If(loc, ..)
            | Statement::While(loc, ..)
            | Statement::Expression(loc, ..)
            | Statement::VariableDefinition(loc, ..)
            | Statement::For(loc, ..)
            | Statement::DoWhile(loc, ..)
            | Statement::Continue(loc)
            | Statement::Break(loc)
            | Statement::Return(loc, ..)
            | Statement::Revert(loc, ..)
            | Statement::Emit(loc, ..)
            | Statement::Try(loc, ..)
            | Statement::DocComment(loc, ..) => *loc,
        }
    }
}

impl AssemblyStatement {
    pub fn loc(&self) -> Loc {
        match self {
            AssemblyStatement::Assign(loc, ..)
            | AssemblyStatement::VariableDeclaration(loc, ..)
            | AssemblyStatement::If(loc, ..)
            | AssemblyStatement::Leave(loc, ..)
            | AssemblyStatement::Break(loc, ..)
            | AssemblyStatement::Continue(loc, ..) => *loc,

            AssemblyStatement::Block(block) => block.loc,

            AssemblyStatement::FunctionDefinition(func_def) => func_def.loc,

            AssemblyStatement::FunctionCall(func_call) => func_call.loc,

            AssemblyStatement::For(for_struct) => for_struct.loc,
            AssemblyStatement::Switch(switch_struct) => switch_struct.loc,
        }
    }
}
