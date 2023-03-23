use crate::{
    lexer::LexicalError,
    pt::{self, Loc},
};

/// Returns a shared reference to the code location, if it exists.
pub trait OptionalCodeLocation {
    /// Optionally returns a shared reference to the code location of `self`.
    fn loc_opt(&self) -> Option<&Loc>;
}

impl<T: CodeLocation> OptionalCodeLocation for T {
    fn loc_opt(&self) -> Option<&Loc> {
        Some(self.loc())
    }
}

impl<T: CodeLocation> OptionalCodeLocation for Option<T> {
    fn loc_opt(&self) -> Option<&Loc> {
        self.as_ref().map(CodeLocation::loc)
    }
}

/// Returns a shared reference to the code location.
pub trait CodeLocation {
    /// Returns a shared reference to the code location of `self`.
    fn loc(&self) -> &Loc;
}

impl CodeLocation for Loc {
    fn loc(&self) -> &Loc {
        self
    }
}

impl<T: CodeLocation> CodeLocation for &'_ T {
    fn loc(&self) -> &Loc {
        (*self).loc()
    }
}

impl<T: CodeLocation> CodeLocation for [T] {
    fn loc(&self) -> &Loc {
        self.first().map(CodeLocation::loc).unwrap_or(Loc::DEFAULT)
    }
}

impl<T: CodeLocation> CodeLocation for Vec<T> {
    fn loc(&self) -> &Loc {
        self.as_slice().loc()
    }
}

macro_rules! impl_for_structs {
    ($($t:ty),+ $(,)?) => {
        $(
            impl CodeLocation for $t {
                fn loc(&self) -> &Loc {
                    &self.loc
                }
            }
        )+
    };
}

// all structs except for SourceUnit
impl_for_structs!(
    pt::Annotation,
    pt::Base,
    pt::ContractDefinition,
    pt::EnumDefinition,
    pt::ErrorDefinition,
    pt::ErrorParameter,
    pt::EventDefinition,
    pt::EventParameter,
    pt::FunctionDefinition,
    pt::HexLiteral,
    pt::Identifier,
    pt::IdentifierPath,
    pt::NamedArgument,
    pt::Parameter,
    pt::StringLiteral,
    pt::StructDefinition,
    pt::TypeDefinition,
    pt::Using,
    pt::UsingFunction,
    pt::VariableDeclaration,
    pt::VariableDefinition,
    pt::YulBlock,
    pt::YulFor,
    pt::YulFunctionCall,
    pt::YulFunctionDefinition,
    pt::YulSwitch,
    pt::YulTypedIdentifier,
);

macro_rules! impl_for_enums {
    ($(
        $t:ty: match $s:ident {
            $($m:tt)*
        }
    )+) => {
        $(
            impl CodeLocation for $t {
                fn loc(&$s) -> &Loc {
                    match $s {
                        $($m)*
                    }
                }
            }
        )+
    };
}

// all enums except for SourceUnit, Type, UserDefinedOperator and FunctionTy
impl_for_enums! {
    pt::CatchClause: match self {
        Self::Simple(l, ..)
        | Self::Named(l, ..) => l,
    }

    pt::Comment: match self {
        Self::Line(l, ..)
        | Self::Block(l, ..)
        | Self::DocLine(l, ..)
        | Self::DocBlock(l, ..) => l,
    }

    pt::ContractPart: match self {
        Self::StructDefinition(l, ..) => l.loc(),
        Self::EventDefinition(l, ..) => l.loc(),
        Self::EnumDefinition(l, ..) => l.loc(),
        Self::ErrorDefinition(l, ..) => l.loc(),
        Self::VariableDefinition(l, ..) => l.loc(),
        Self::FunctionDefinition(l, ..) => l.loc(),
        Self::TypeDefinition(l, ..) => l.loc(),
        Self::Annotation(l, ..) => l.loc(),
        Self::Using(l, ..) => l.loc(),
        Self::StraySemicolon(l, ..) => l,
    }

    pt::ContractTy: match self {
        Self::Abstract(l, ..)
        | Self::Contract(l, ..)
        | Self::Library(l, ..)
        | Self::Interface(l, ..) => l,
    }

    pt::Expression: match self {
        Self::StringLiteral(l, ..) => l.loc(),
        Self::HexLiteral(l, ..) => l.loc(),
        Self::Variable(l, ..) => l.loc(),
        Self::PostIncrement(l, ..)
        | Self::PostDecrement(l, ..)
        | Self::New(l, ..)
        | Self::Parenthesis(l, ..)
        | Self::ArraySubscript(l, ..)
        | Self::ArraySlice(l, ..)
        | Self::MemberAccess(l, ..)
        | Self::FunctionCall(l, ..)
        | Self::FunctionCallBlock(l, ..)
        | Self::NamedFunctionCall(l, ..)
        | Self::Not(l, ..)
        | Self::Complement(l, ..)
        | Self::Delete(l, ..)
        | Self::PreIncrement(l, ..)
        | Self::PreDecrement(l, ..)
        | Self::UnaryPlus(l, ..)
        | Self::Negate(l, ..)
        | Self::Power(l, ..)
        | Self::Multiply(l, ..)
        | Self::Divide(l, ..)
        | Self::Modulo(l, ..)
        | Self::Add(l, ..)
        | Self::Subtract(l, ..)
        | Self::ShiftLeft(l, ..)
        | Self::ShiftRight(l, ..)
        | Self::BitwiseAnd(l, ..)
        | Self::BitwiseXor(l, ..)
        | Self::BitwiseOr(l, ..)
        | Self::Less(l, ..)
        | Self::More(l, ..)
        | Self::LessEqual(l, ..)
        | Self::MoreEqual(l, ..)
        | Self::Equal(l, ..)
        | Self::NotEqual(l, ..)
        | Self::And(l, ..)
        | Self::Or(l, ..)
        | Self::ConditionalOperator(l, ..)
        | Self::Assign(l, ..)
        | Self::AssignOr(l, ..)
        | Self::AssignAnd(l, ..)
        | Self::AssignXor(l, ..)
        | Self::AssignShiftLeft(l, ..)
        | Self::AssignShiftRight(l, ..)
        | Self::AssignAdd(l, ..)
        | Self::AssignSubtract(l, ..)
        | Self::AssignMultiply(l, ..)
        | Self::AssignDivide(l, ..)
        | Self::AssignModulo(l, ..)
        | Self::BoolLiteral(l, ..)
        | Self::NumberLiteral(l, ..)
        | Self::RationalNumberLiteral(l, ..)
        | Self::HexNumberLiteral(l, ..)
        | Self::ArrayLiteral(l, ..)
        | Self::List(l, ..)
        | Self::Type(l, ..)
        | Self::This(l, ..)
        | Self::AddressLiteral(l, ..) => l,
    }

    pt::FunctionAttribute: match self {
        Self::Mutability(l) => l.loc(),
        Self::Visibility(l) => l.loc(),
        Self::Virtual(l, ..)
        | Self::Immutable(l, ..)
        | Self::Override(l, ..,)
        | Self::BaseOrModifier(l, ..)
        | Self::Error(l, ..) => l,
    }

    pt::Import: match self {
        Self::GlobalSymbol(.., l)
        | Self::Plain(.., l)
        | Self::Rename(.., l) => l,
    }

    pt::Mutability: match self {
        Self::Constant(l, ..)
        | Self::Payable(l, ..)
        | Self::Pure(l, ..)
        | Self::View(l, ..) => l,
    }

    pt::SourceUnitPart: match self {
        Self::PragmaDirective(l, ..) => l,
        Self::ImportDirective(l, ..) => l.loc(),
        Self::ContractDefinition(l, ..) => l.loc(),
        Self::EnumDefinition(l, ..) => l.loc(),
        Self::StructDefinition(l, ..) => l.loc(),
        Self::EventDefinition(l, ..) => l.loc(),
        Self::ErrorDefinition(l, ..) => l.loc(),
        Self::FunctionDefinition(l, ..) => l.loc(),
        Self::VariableDefinition(l, ..) => l.loc(),
        Self::TypeDefinition(l, ..) => l.loc(),
        Self::Annotation(l, ..) => l.loc(),
        Self::Using(l, ..) => l.loc(),
        Self::StraySemicolon(l) => l,
    }

    pt::Statement: match self {
        Self::Block { loc: l, .. }
        | Self::Assembly { loc: l, .. }
        | Self::Args(l, ..)
        | Self::If(l, ..)
        | Self::While(l, ..)
        | Self::Expression(l, ..)
        | Self::VariableDefinition(l, ..)
        | Self::For(l, ..)
        | Self::DoWhile(l, ..)
        | Self::Continue(l, ..)
        | Self::Break(l, ..)
        | Self::Return(l, ..)
        | Self::Revert(l, ..)
        | Self::RevertNamedArgs(l, ..)
        | Self::Emit(l, ..)
        | Self::Try(l, ..)
        | Self::Error(l, ..) => l,
    }

    pt::StorageLocation: match self {
        Self::Calldata(l, ..)
        | Self::Memory(l, ..)
        | Self::Storage(l, ..) => l,
    }

    pt::UsingList: match self {
        Self::Library(l, ..) => l.loc(),
        Self::Functions(l, ..) => l.loc(),
        Self::Error => panic!("an error occurred"),
    }

    pt::VariableAttribute: match self {
        Self::Visibility(l, ..) => l.loc(),
        Self::Constant(l, ..)
        | Self::Immutable(l, ..)
        | Self::Override(l, ..) => l,
    }

    pt::Visibility: match self {
        Self::External(l, ..)
        | Self::Internal(l, ..)
        | Self::Private(l, ..)
        | Self::Public(l, ..) => l.as_ref().unwrap_or(Loc::DEFAULT),
    }

    pt::YulExpression: match self {
        Self::HexStringLiteral(l, ..) => l.loc(),
        Self::StringLiteral(l, ..) => l.loc(),
        Self::Variable(l, ..) => l.loc(),
        Self::FunctionCall(l, ..) => l.loc(),
        Self::BoolLiteral(l, ..)
        | Self::NumberLiteral(l, ..)
        | Self::HexNumberLiteral(l, ..)
        | Self::SuffixAccess(l, ..) => l,
    }

    pt::YulStatement: match self {
        Self::Block(l, ..) => l.loc(),
        Self::FunctionDefinition(l, ..) => l.loc(),
        Self::FunctionCall(l, ..) => l.loc(),
        Self::For(l, ..) => l.loc(),
        Self::Switch(l, ..) => l.loc(),
        Self::Assign(l, ..)
        | Self::VariableDeclaration(l, ..)
        | Self::If(l, ..)
        | Self::Leave(l, ..)
        | Self::Break(l, ..)
        | Self::Continue(l, ..)
        | Self::Error(l, ..) => l,
    }

    pt::YulSwitchOptions: match self {
        Self::Case(l, ..)
        | Self::Default(l, ..) => l,
    }

    // other
    LexicalError: match self {
        Self::EndOfFileInComment(l)
        | Self::EndOfFileInString(l)
        | Self::EndofFileInHex(l)
        | Self::MissingNumber(l)
        | Self::InvalidCharacterInHexLiteral(l, _)
        | Self::UnrecognisedToken(l, _)
        | Self::ExpectedFrom(l, _)
        | Self::MissingExponent(l) => l,
    }
}
