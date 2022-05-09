use std::fmt::{self, Display};

use num_bigint::BigInt;
use num_rational::BigRational;

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

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum CommentType {
    Line,
    Block,
}

#[derive(Debug, PartialEq, Clone)]
pub struct DocComment {
    pub loc: Loc,
    pub ty: CommentType,
    pub comment: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct SourceUnit(pub Vec<SourceUnitPart>);

#[derive(Debug, PartialEq, Clone)]
pub enum SourceUnitPart {
    ContractDefinition(Box<ContractDefinition>),
    PragmaDirective(Loc, Identifier, StringLiteral),
    ImportDirective(Import),
    EnumDefinition(Box<EnumDefinition>),
    StructDefinition(Box<StructDefinition>),
    EventDefinition(Box<EventDefinition>),
    ErrorDefinition(Box<ErrorDefinition>),
    FunctionDefinition(Box<FunctionDefinition>),
    VariableDefinition(Box<VariableDefinition>),
    TypeDefinition(Box<TypeDefinition>),
    DocComment(DocComment),
    Using(Box<Using>),
    StraySemicolon(Loc),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Import {
    Plain(StringLiteral, Loc),
    GlobalSymbol(StringLiteral, Identifier, Loc),
    Rename(StringLiteral, Vec<(Identifier, Option<Identifier>)>, Loc),
}

pub type ParameterList = Vec<(Loc, Option<Parameter>)>;

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
        returns: Option<(ParameterList, Vec<FunctionAttribute>)>,
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

#[derive(Debug, PartialEq, Clone)]
#[allow(clippy::vec_box)]
pub struct StructDefinition {
    pub loc: Loc,
    pub name: Identifier,
    pub fields: Vec<VariableDeclaration>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ContractPart {
    StructDefinition(Box<StructDefinition>),
    EventDefinition(Box<EventDefinition>),
    EnumDefinition(Box<EnumDefinition>),
    ErrorDefinition(Box<ErrorDefinition>),
    VariableDefinition(Box<VariableDefinition>),
    FunctionDefinition(Box<FunctionDefinition>),
    TypeDefinition(Box<TypeDefinition>),
    StraySemicolon(Loc),
    Using(Box<Using>),
    DocComment(DocComment),
}

#[derive(Debug, PartialEq, Clone)]
pub enum UsingList {
    Library(Expression),
    Functions(Vec<Expression>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Using {
    pub loc: Loc,
    pub list: UsingList,
    pub ty: Option<Expression>,
    pub global: Option<Identifier>,
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
    pub name: Expression,
    pub args: Option<Vec<Expression>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ContractDefinition {
    pub loc: Loc,
    pub ty: ContractTy,
    pub name: Identifier,
    pub base: Vec<Base>,
    pub parts: Vec<ContractPart>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct EventParameter {
    pub ty: Expression,
    pub loc: Loc,
    pub indexed: bool,
    pub name: Option<Identifier>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct EventDefinition {
    pub loc: Loc,
    pub name: Identifier,
    pub fields: Vec<EventParameter>,
    pub anonymous: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ErrorParameter {
    pub ty: Expression,
    pub loc: Loc,
    pub name: Option<Identifier>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ErrorDefinition {
    pub loc: Loc,
    pub name: Identifier,
    pub fields: Vec<ErrorParameter>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct EnumDefinition {
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

#[derive(Debug, PartialEq, Clone)]
pub struct VariableDefinition {
    pub loc: Loc,
    pub ty: Expression,
    pub attrs: Vec<VariableAttribute>,
    pub name: Identifier,
    pub initializer: Option<Expression>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TypeDefinition {
    pub loc: Loc,
    pub name: Identifier,
    pub ty: Expression,
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
    Gwei(Loc),
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
    List(Loc, ParameterList),
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

impl Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Variable(id) => write!(f, "{}", id.name),
            Expression::MemberAccess(_, e, id) => write!(f, "{}.{}", e, id.name),
            _ => unimplemented!(),
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
    Immutable(Loc),
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

#[derive(Debug, PartialEq, Clone)]
pub struct FunctionDefinition {
    pub loc: Loc,
    pub ty: FunctionTy,
    pub name: Option<Identifier>,
    pub name_loc: Loc,
    pub params: ParameterList,
    pub attributes: Vec<FunctionAttribute>,
    pub return_not_returns: Option<Loc>,
    pub returns: ParameterList,
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
        block: YulBlock,
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
    Revert(Loc, Option<Expression>, Vec<Expression>),
    RevertNamedArgs(Loc, Option<Expression>, Vec<NamedArgument>),
    Emit(Loc, Expression),
    Try(
        Loc,
        Expression,
        Option<(ParameterList, Box<Statement>)>,
        Vec<CatchClause>,
    ),
    DocComment(DocComment),
}

#[derive(Debug, PartialEq, Clone)]
pub enum CatchClause {
    Simple(Loc, Option<Parameter>, Statement),
    Named(Loc, Identifier, Parameter, Statement),
}

#[derive(Debug, PartialEq, Clone)]
pub enum YulStatement {
    Assign(Loc, Vec<YulExpression>, YulExpression),
    VariableDeclaration(Loc, Vec<YulTypedIdentifier>, Option<YulExpression>),
    If(Loc, YulExpression, YulBlock),
    For(YulFor),
    Switch(YulSwitch),
    Leave(Loc),
    Break(Loc),
    Continue(Loc),
    Block(YulBlock),
    FunctionDefinition(Box<YulFunctionDefinition>),
    FunctionCall(Box<YulFunctionCall>),
}
#[derive(PartialEq, Clone, Debug)]
pub struct YulSwitch {
    pub loc: Loc,
    pub condition: YulExpression,
    pub cases: Vec<YulSwitchOptions>,
    pub default: Option<YulSwitchOptions>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct YulFor {
    pub loc: Loc,
    pub init_block: YulBlock,
    pub condition: YulExpression,
    pub post_block: YulBlock,
    pub execution_block: YulBlock,
}

#[derive(Debug, PartialEq, Clone)]
pub struct YulBlock {
    pub loc: Loc,
    pub statements: Vec<YulStatement>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum YulExpression {
    BoolLiteral(Loc, bool, Option<Identifier>),
    NumberLiteral(Loc, BigInt, Option<Identifier>),
    HexNumberLiteral(Loc, String, Option<Identifier>),
    HexStringLiteral(HexLiteral, Option<Identifier>),
    StringLiteral(StringLiteral, Option<Identifier>),
    Variable(Identifier),
    FunctionCall(Box<YulFunctionCall>),
    Member(Loc, Box<YulExpression>, Identifier),
}

#[derive(Debug, PartialEq, Clone)]
pub struct YulTypedIdentifier {
    pub loc: Loc,
    pub id: Identifier,
    pub ty: Option<Identifier>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct YulFunctionDefinition {
    pub loc: Loc,
    pub id: Identifier,
    pub params: Vec<YulTypedIdentifier>,
    pub returns: Vec<YulTypedIdentifier>,
    pub body: YulBlock,
}

#[derive(Debug, PartialEq, Clone)]
pub struct YulFunctionCall {
    pub loc: Loc,
    pub id: Identifier,
    pub arguments: Vec<YulExpression>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum YulSwitchOptions {
    Case(Loc, YulExpression, YulBlock),
    Default(Loc, YulBlock),
}

impl CodeLocation for YulSwitchOptions {
    fn loc(&self) -> Loc {
        match self {
            YulSwitchOptions::Case(loc, ..) | YulSwitchOptions::Default(loc, ..) => *loc,
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
            | Statement::RevertNamedArgs(loc, ..)
            | Statement::Emit(loc, ..)
            | Statement::Try(loc, ..) => *loc,
            Statement::DocComment(comment) => comment.loc,
        }
    }
}

impl YulStatement {
    pub fn loc(&self) -> Loc {
        match self {
            YulStatement::Assign(loc, ..)
            | YulStatement::VariableDeclaration(loc, ..)
            | YulStatement::If(loc, ..)
            | YulStatement::Leave(loc, ..)
            | YulStatement::Break(loc, ..)
            | YulStatement::Continue(loc, ..) => *loc,

            YulStatement::Block(block) => block.loc,

            YulStatement::FunctionDefinition(func_def) => func_def.loc,

            YulStatement::FunctionCall(func_call) => func_call.loc,

            YulStatement::For(for_struct) => for_struct.loc,
            YulStatement::Switch(switch_struct) => switch_struct.loc,
        }
    }
}
