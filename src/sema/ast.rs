// SPDX-License-Identifier: Apache-2.0

use super::symtable::Symtable;
use crate::abi::anchor::discriminator;
use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::diagnostics::Diagnostics;
use crate::sema::yul::ast::{InlineAssembly, YulFunction};
use crate::sema::Recurse;
use crate::{codegen, Target};
use indexmap::IndexMap;
use num_bigint::BigInt;
use num_rational::BigRational;
use once_cell::unsync::OnceCell;
pub use solang_parser::diagnostics::*;
use solang_parser::pt;
use solang_parser::pt::{CodeLocation, FunctionTy, OptionalCodeLocation};
use std::collections::HashSet;
use std::sync::Arc;
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    path::PathBuf,
};
use tiny_keccak::{Hasher, Keccak};

#[derive(PartialEq, Eq, Clone, Hash, Debug)]
pub enum Type {
    Address(bool),
    Bool,
    Int(u16),
    Uint(u16),
    Rational,
    Bytes(u8),
    DynamicBytes,
    String,
    Array(Box<Type>, Vec<ArrayLength>),
    /// The usize is an index into enums in the namespace
    Enum(usize),
    /// The usize is an index into contracts in the namespace
    Struct(StructType),
    Mapping(Box<Type>, Box<Type>),
    /// The usize is an index into contracts in the namespace
    Contract(usize),
    Ref(Box<Type>),
    /// Reference to storage, first bool is true for immutables
    StorageRef(bool, Box<Type>),
    InternalFunction {
        mutability: Mutability,
        params: Vec<Type>,
        returns: Vec<Type>,
    },
    ExternalFunction {
        mutability: Mutability,
        params: Vec<Type>,
        returns: Vec<Type>,
    },
    /// User type definitions, e.g. `type Foo is int128;`. The usize
    /// is an index into user_types in the namespace.
    UserType(usize),
    /// There is no way to declare value in Solidity (should there be?)
    Value,
    Void,
    Unreachable,
    /// DynamicBytes and String are lowered to a vector.
    Slice(Box<Type>),
    /// We could not resolve this type
    Unresolved,
    /// When we advance a pointer, it cannot be any of the previous types.
    /// e.g. Type::Bytes is a pointer to struct.vector. When we advance it, it is a pointer
    /// to latter's data region.
    BufferPointer,
    /// The function selector (or discriminator) type is 4 bytes on Substrate and 8 bytes on Solana
    FunctionSelector,
}

#[derive(PartialEq, Eq, Clone, Hash, Debug)]
pub enum ArrayLength {
    Fixed(BigInt),
    Dynamic,
    /// Fixed length arrays, any length permitted. This is useful for when we
    /// do not want dynamic length, but want to permit any length. For example
    /// the create_program_address() call takes any number of seeds as its
    /// first argument, and we don't want to allocate a dynamic array for
    /// this parameter as this would be wasteful to allocate a vector for
    /// this argument.
    AnyFixed,
}

impl ArrayLength {
    /// Get the length, if fixed
    pub fn array_length(&self) -> Option<&BigInt> {
        match self {
            ArrayLength::Fixed(len) => Some(len),
            _ => None,
        }
    }
}

pub trait RetrieveType {
    /// Return the type for this expression. This assumes the expression has a single value,
    /// panics will occur otherwise
    fn ty(&self) -> Type;
}

impl Type {
    pub fn get_type_size(&self) -> u16 {
        match self {
            Type::Int(n) | Type::Uint(n) => *n,
            Type::Bool => 1,
            _ => unimplemented!("size of type not known"),
        }
    }

    pub fn unwrap_user_type(self, ns: &Namespace) -> Type {
        if let Type::UserType(type_no) = self {
            ns.user_types[type_no].ty.clone()
        } else {
            self
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Copy, Hash)]
pub enum StructType {
    UserDefined(usize),
    AccountInfo,
    AccountMeta,
    ExternalFunction,
    SolParameters,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct StructDecl {
    pub tags: Vec<Tag>,
    pub name: String,
    pub loc: pt::Loc,
    pub contract: Option<String>,
    pub fields: Vec<Parameter>,
    // List of offsets of the fields, last entry is the offset for the struct overall size
    pub offsets: Vec<BigInt>,
    // Same, but now in storage
    pub storage_offsets: Vec<BigInt>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct EventDecl {
    pub tags: Vec<Tag>,
    pub name: String,
    pub loc: pt::Loc,
    pub contract: Option<usize>,
    pub fields: Vec<Parameter>,
    pub signature: String,
    pub anonymous: bool,
    pub used: bool,
}

impl EventDecl {
    pub fn symbol_name(&self, ns: &Namespace) -> String {
        match &self.contract {
            Some(c) => format!("{}.{}", ns.contracts[*c].name, self.name),
            None => self.name.to_string(),
        }
    }
}

impl fmt::Display for StructDecl {
    /// Make the struct name into a string for printing. The struct can be declared either
    /// inside or outside a contract.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.contract {
            Some(c) => write!(f, "{}.{}", c, self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

pub struct EnumDecl {
    pub tags: Vec<Tag>,
    pub name: String,
    pub contract: Option<String>,
    pub loc: pt::Loc,
    pub ty: Type,
    pub values: IndexMap<String, pt::Loc>,
}

impl fmt::Display for EnumDecl {
    /// Make the enum name into a string for printing. The enum can be declared either
    /// inside or outside a contract.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.contract {
            Some(c) => write!(f, "{}.{}", c, self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Parameter {
    pub loc: pt::Loc,
    /// The name can empty (e.g. in an event field or unnamed parameter/return)
    pub id: Option<pt::Identifier>,
    pub ty: Type,
    /// Yul function parameters may not have a type identifier
    pub ty_loc: Option<pt::Loc>,
    /// Event fields may indexed, which means they are sent to the log
    pub indexed: bool,
    /// Some builtin structs have readonly fields
    pub readonly: bool,
    /// A struct may contain itself which make the struct infinite size in
    /// memory. This boolean specifies which field introduces the recursion.
    pub recursive: bool,
}

impl Parameter {
    pub fn name_as_str(&self) -> &str {
        if let Some(name) = &self.id {
            name.name.as_str()
        } else {
            ""
        }
    }
}

#[derive(PartialEq, Eq, Clone, Hash, Debug)]
pub enum Mutability {
    Payable(pt::Loc),
    Nonpayable(pt::Loc),
    View(pt::Loc),
    Pure(pt::Loc),
}

impl Mutability {
    pub fn is_default(&self) -> bool {
        matches!(self, Mutability::Nonpayable(_))
    }
}

impl fmt::Display for Mutability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mutability::Pure(_) => write!(f, "pure"),
            Mutability::View(_) => write!(f, "view"),
            Mutability::Nonpayable(_) => write!(f, "nonpayable"),
            Mutability::Payable(_) => write!(f, "payable"),
        }
    }
}

pub struct Function {
    pub tags: Vec<Tag>,
    /// The location of the prototype (not body)
    pub loc: pt::Loc,
    pub name: String,
    pub contract_no: Option<usize>,
    pub ty: pt::FunctionTy,
    pub signature: String,
    pub mutability: Mutability,
    pub visibility: pt::Visibility,
    pub params: Arc<Vec<Parameter>>,
    pub returns: Arc<Vec<Parameter>>,
    /// Constructor arguments for base contracts, only present on constructors
    pub bases: BTreeMap<usize, (pt::Loc, usize, Vec<Expression>)>,
    /// Modifiers for functions
    pub modifiers: Vec<Expression>,
    pub is_virtual: bool,
    /// Is this function an acccesor function created by a public variable
    pub is_accessor: bool,
    pub is_override: Option<(pt::Loc, Vec<usize>)>,
    /// The selector (known as discriminator on Solana/Anchor)
    pub selector: Option<(pt::Loc, Vec<u8>)>,
    /// Was the function declared with a body
    pub has_body: bool,
    /// The resolved body (if any)
    pub body: Vec<Statement>,
    pub symtable: Symtable,
    /// What events are emitted by the body of this function
    pub emits_events: Vec<usize>,
    /// For overloaded functions this is the mangled (unique) name.
    pub mangled_name: String,
    /// Solana constructors may have seeds specified using @seed tags
    pub annotations: Vec<ConstructorAnnotation>,
    /// Which contracts should we use the mangled name in?
    pub mangled_name_contracts: HashSet<usize>,
}

pub enum ConstructorAnnotation {
    Seed(Expression),
    Payer(Expression),
    Space(Expression),
    Bump(Expression),
}

impl CodeLocation for ConstructorAnnotation {
    fn loc(&self) -> pt::Loc {
        match self {
            ConstructorAnnotation::Seed(expr) => expr.loc(),
            ConstructorAnnotation::Payer(expr) => expr.loc(),
            ConstructorAnnotation::Space(expr) => expr.loc(),
            ConstructorAnnotation::Bump(expr) => expr.loc(),
        }
    }
}

/// This trait provides a single interface for fetching paramenters, returns and the symbol table
/// for both yul and solidity functions
pub trait FunctionAttributes {
    fn get_symbol_table(&self) -> &Symtable;
    fn get_parameters(&self) -> &Vec<Parameter>;
    fn get_returns(&self) -> &Vec<Parameter>;
}

impl FunctionAttributes for Function {
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

impl Function {
    pub fn new(
        loc: pt::Loc,
        name: String,
        contract_no: Option<usize>,
        tags: Vec<Tag>,
        ty: pt::FunctionTy,
        mutability: Option<pt::Mutability>,
        visibility: pt::Visibility,
        params: Vec<Parameter>,
        returns: Vec<Parameter>,
        ns: &Namespace,
    ) -> Self {
        let signature = match ty {
            pt::FunctionTy::Fallback => String::from("@fallback"),
            pt::FunctionTy::Receive => String::from("@receive"),
            _ => ns.signature(&name, &params),
        };

        let mutability = match mutability {
            None => Mutability::Nonpayable(loc),
            Some(pt::Mutability::Payable(loc)) => Mutability::Payable(loc),
            Some(pt::Mutability::Pure(loc)) => Mutability::Pure(loc),
            Some(pt::Mutability::View(loc)) => Mutability::View(loc),
            Some(pt::Mutability::Constant(loc)) => Mutability::View(loc),
        };

        let mangled_name = signature
            .replace('(', "_")
            .replace(')', "")
            .replace(',', "_")
            .replace("[]", "Array")
            .replace('[', "Array")
            .replace(']', "");

        Function {
            tags,
            loc,
            name,
            contract_no,
            ty,
            signature,
            mutability,
            visibility,
            params: Arc::new(params),
            returns: Arc::new(returns),
            bases: BTreeMap::new(),
            modifiers: Vec::new(),
            selector: None,
            is_virtual: false,
            is_accessor: false,
            has_body: false,
            is_override: None,
            body: Vec::new(),
            symtable: Symtable::new(),
            emits_events: Vec::new(),
            mangled_name,
            annotations: Vec::new(),
            mangled_name_contracts: HashSet::new(),
        }
    }

    /// Generate selector for this function
    pub fn selector(&self, ns: &Namespace, contract_no: &usize) -> Vec<u8> {
        if let Some((_, selector)) = &self.selector {
            selector.clone()
        } else if ns.target == Target::Solana {
            match self.ty {
                FunctionTy::Constructor => discriminator("global", "new"),
                _ => {
                    let discriminator_image = if self.mangled_name_contracts.contains(contract_no) {
                        &self.mangled_name
                    } else {
                        &self.name
                    };
                    discriminator("global", discriminator_image.as_str())
                }
            }
        } else {
            let mut res = [0u8; 32];

            let mut hasher = Keccak::v256();
            hasher.update(self.signature.as_bytes());
            hasher.finalize(&mut res);

            res[..4].to_vec()
        }
    }

    /// Is this a constructor
    pub fn is_constructor(&self) -> bool {
        self.ty == pt::FunctionTy::Constructor
    }

    /// Does this function have the payable state
    pub fn is_payable(&self) -> bool {
        matches!(self.mutability, Mutability::Payable(_))
    }

    /// Is this function accessable externally
    pub fn is_public(&self) -> bool {
        matches!(
            self.visibility,
            pt::Visibility::Public(_) | pt::Visibility::External(_)
        )
    }

    /// Is this function accessable only from same contract
    pub fn is_private(&self) -> bool {
        matches!(self.visibility, pt::Visibility::Private(_))
    }

    /// Print the function type, contract name, and name
    pub fn print_name(&self, ns: &Namespace) -> String {
        if let Some(contract_no) = &self.contract_no {
            format!(
                "{} {}.{}",
                self.ty, ns.contracts[*contract_no].name, self.name
            )
        } else {
            format!("{} {}", self.ty, self.name)
        }
    }
}

impl From<&pt::Type> for Type {
    fn from(p: &pt::Type) -> Type {
        match p {
            pt::Type::Bool => Type::Bool,
            pt::Type::Address => Type::Address(false),
            pt::Type::AddressPayable => Type::Address(true),
            pt::Type::Payable => Type::Address(true),
            pt::Type::Int(n) => Type::Int(*n),
            pt::Type::Uint(n) => Type::Uint(*n),
            pt::Type::Bytes(n) => Type::Bytes(*n),
            pt::Type::String => Type::String,
            pt::Type::Rational => Type::Rational,
            pt::Type::DynamicBytes => Type::DynamicBytes,
            // needs special casing
            pt::Type::Function { .. } => unimplemented!(),
            pt::Type::Mapping(..) => unimplemented!(),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct UserTypeDecl {
    pub tags: Vec<Tag>,
    pub loc: pt::Loc,
    pub name: String,
    pub ty: Type,
    pub contract: Option<String>,
}

impl fmt::Display for UserTypeDecl {
    /// Make the user type name into a string for printing. The user type can
    /// be declared either inside or outside a contract.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.contract {
            Some(c) => write!(f, "{}.{}", c, self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

pub struct Variable {
    pub tags: Vec<Tag>,
    pub name: String,
    pub loc: pt::Loc,
    pub ty: Type,
    pub visibility: pt::Visibility,
    pub constant: bool,
    pub immutable: bool,
    pub initializer: Option<Expression>,
    pub assigned: bool,
    pub read: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Symbol {
    Enum(pt::Loc, usize),
    Function(Vec<(pt::Loc, usize)>),
    Variable(pt::Loc, Option<usize>, usize),
    Struct(pt::Loc, StructType),
    Event(Vec<(pt::Loc, usize)>),
    Contract(pt::Loc, usize),
    Import(pt::Loc, usize),
    UserType(pt::Loc, usize),
}

impl CodeLocation for Symbol {
    fn loc(&self) -> pt::Loc {
        match self {
            Symbol::Enum(loc, _)
            | Symbol::Variable(loc, ..)
            | Symbol::Struct(loc, _)
            | Symbol::Contract(loc, _)
            | Symbol::Import(loc, _)
            | Symbol::UserType(loc, _) => *loc,
            Symbol::Event(items) | Symbol::Function(items) => items[0].0,
        }
    }
}

impl Symbol {
    /// Is this symbol for an event
    pub fn is_event(&self) -> bool {
        matches!(self, Symbol::Event(_))
    }

    /// Does this symbol have an accessor function
    pub fn has_accessor(&self, ns: &Namespace) -> bool {
        if let Symbol::Variable(_, Some(contract_no), var_no) = self {
            matches!(
                ns.contracts[*contract_no].variables[*var_no].visibility,
                pt::Visibility::Public(_)
            )
        } else {
            false
        }
    }

    /// Is this a private symbol
    pub fn is_private_variable(&self, ns: &Namespace) -> bool {
        match self {
            Symbol::Variable(_, Some(contract_no), var_no) => {
                let visibility = &ns.contracts[*contract_no].variables[*var_no].visibility;

                matches!(visibility, pt::Visibility::Private(_))
            }
            _ => false,
        }
    }
}

/// Any Solidity file, either the main file or anything that was imported
#[derive(Clone, Debug)]
pub struct File {
    /// The on-disk filename
    pub path: PathBuf,
    /// Used for offset to line-column conversions
    pub line_starts: Vec<usize>,
    /// Indicates the file number in FileResolver.files
    pub cache_no: Option<usize>,
}

/// When resolving a Solidity file, this holds all the resolved items
pub struct Namespace {
    pub target: Target,
    pub files: Vec<File>,
    pub enums: Vec<EnumDecl>,
    pub structs: Vec<StructDecl>,
    pub events: Vec<EventDecl>,
    pub contracts: Vec<Contract>,
    /// Global using declarations
    pub using: Vec<Using>,
    /// All type declarations
    pub user_types: Vec<UserTypeDecl>,
    /// All functions
    pub functions: Vec<Function>,
    /// Yul functions
    pub yul_functions: Vec<YulFunction>,
    /// Global constants
    pub constants: Vec<Variable>,
    /// address length in bytes
    pub address_length: usize,
    /// value length in bytes
    pub value_length: usize,
    pub diagnostics: Diagnostics,
    /// There is a separate namespace for functions and non-functions
    pub function_symbols: HashMap<(usize, Option<usize>, String), Symbol>,
    /// Symbol key is file_no, contract, identifier
    pub variable_symbols: HashMap<(usize, Option<usize>, String), Symbol>,
    // each variable in the symbol table should have a unique number
    pub next_id: usize,
    /// For a variable reference at a location, give the constant value
    /// This for use by the language server to show the value of a variable at a location
    pub var_constants: HashMap<pt::Loc, codegen::Expression>,
    /// Overrides for hover in the language server
    pub hover_overrides: HashMap<pt::Loc, String>,
}

pub struct Layout {
    pub slot: BigInt,
    pub contract_no: usize,
    pub var_no: usize,
    pub ty: Type,
}

pub struct Base {
    pub loc: pt::Loc,
    pub contract_no: usize,
    pub constructor: Option<(usize, Vec<Expression>)>,
}

pub struct Using {
    pub list: UsingList,
    pub ty: Option<Type>,
    pub file_no: Option<usize>,
}

pub enum UsingList {
    Library(usize),
    Functions(Vec<usize>),
}

pub struct Contract {
    pub tags: Vec<Tag>,
    pub loc: pt::Loc,
    pub ty: pt::ContractTy,
    pub name: String,
    pub bases: Vec<Base>,
    pub using: Vec<Using>,
    pub layout: Vec<Layout>,
    pub fixed_layout_size: BigInt,
    pub functions: Vec<usize>,
    pub all_functions: BTreeMap<usize, usize>,
    pub virtual_functions: HashMap<String, usize>,
    pub yul_functions: Vec<usize>,
    pub variables: Vec<Variable>,
    /// List of contracts this contract instantiates
    pub creates: Vec<usize>,
    /// List of events this contract may emit
    pub emits_events: Vec<usize>,
    pub initializer: Option<usize>,
    pub default_constructor: Option<(Function, usize)>,
    pub cfg: Vec<ControlFlowGraph>,
    /// Compiled program. Only available after emit.
    pub code: OnceCell<Vec<u8>>,
    /// Can the contract be instantiated, i.e. not abstract, no errors, etc.
    pub instantiable: bool,
    /// CFG number of this contract's dispatch function
    pub dispatch_no: usize,
    /// Account of deployed program code on Solana
    pub program_id: Option<Vec<u8>>,
}

impl Contract {
    // Is this a concrete contract, which can be instantiated
    pub fn is_concrete(&self) -> bool {
        matches!(self.ty, pt::ContractTy::Contract(_))
    }

    // Is this an interface
    pub fn is_interface(&self) -> bool {
        matches!(self.ty, pt::ContractTy::Interface(_))
    }

    // Is this an library
    pub fn is_library(&self) -> bool {
        matches!(self.ty, pt::ContractTy::Library(_))
    }

    /// Does the constructor require arguments. Should be false is there is no constructor
    pub fn constructor_needs_arguments(&self, ns: &Namespace) -> bool {
        self.have_constructor(ns) && self.no_args_constructor(ns).is_none()
    }

    /// Does the contract have a constructor defined
    pub fn have_constructor(&self, ns: &Namespace) -> bool {
        self.functions
            .iter()
            .any(|func_no| ns.functions[*func_no].is_constructor())
    }

    /// Return the constructor with no arguments
    pub fn no_args_constructor(&self, ns: &Namespace) -> Option<usize> {
        self.functions
            .iter()
            .find(|func_no| {
                let func = &ns.functions[**func_no];

                func.is_constructor() && func.params.is_empty()
            })
            .cloned()
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Expression {
    BoolLiteral(pt::Loc, bool),
    BytesLiteral(pt::Loc, Type, Vec<u8>),
    CodeLiteral(pt::Loc, usize, bool),
    NumberLiteral(pt::Loc, Type, BigInt),
    RationalNumberLiteral(pt::Loc, Type, BigRational),
    StructLiteral(pt::Loc, Type, Vec<Expression>),
    ArrayLiteral(pt::Loc, Type, Vec<u32>, Vec<Expression>),
    ConstArrayLiteral(pt::Loc, Type, Vec<u32>, Vec<Expression>),
    Add(pt::Loc, Type, bool, Box<Expression>, Box<Expression>),
    Subtract(pt::Loc, Type, bool, Box<Expression>, Box<Expression>),
    Multiply(pt::Loc, Type, bool, Box<Expression>, Box<Expression>),
    Divide(pt::Loc, Type, Box<Expression>, Box<Expression>),
    Modulo(pt::Loc, Type, Box<Expression>, Box<Expression>),
    Power(pt::Loc, Type, bool, Box<Expression>, Box<Expression>),
    BitwiseOr(pt::Loc, Type, Box<Expression>, Box<Expression>),
    BitwiseAnd(pt::Loc, Type, Box<Expression>, Box<Expression>),
    BitwiseXor(pt::Loc, Type, Box<Expression>, Box<Expression>),
    ShiftLeft(pt::Loc, Type, Box<Expression>, Box<Expression>),
    ShiftRight(pt::Loc, Type, Box<Expression>, Box<Expression>, bool),
    Variable(pt::Loc, Type, usize),
    ConstantVariable(pt::Loc, Type, Option<usize>, usize),
    StorageVariable(pt::Loc, Type, usize, usize),
    Load(pt::Loc, Type, Box<Expression>),
    GetRef(pt::Loc, Type, Box<Expression>),
    StorageLoad(pt::Loc, Type, Box<Expression>),
    ZeroExt {
        loc: pt::Loc,
        to: Type,
        expr: Box<Expression>,
    },
    SignExt {
        loc: pt::Loc,
        to: Type,
        expr: Box<Expression>,
    },
    Trunc {
        loc: pt::Loc,
        to: Type,
        expr: Box<Expression>,
    },
    CheckingTrunc {
        loc: pt::Loc,
        to: Type,
        expr: Box<Expression>,
    },
    Cast {
        loc: pt::Loc,
        to: Type,
        expr: Box<Expression>,
    },
    BytesCast {
        loc: pt::Loc,
        from: Type,
        to: Type,
        expr: Box<Expression>,
    },
    PreIncrement(pt::Loc, Type, bool, Box<Expression>),
    PreDecrement(pt::Loc, Type, bool, Box<Expression>),
    PostIncrement(pt::Loc, Type, bool, Box<Expression>),
    PostDecrement(pt::Loc, Type, bool, Box<Expression>),
    Assign(pt::Loc, Type, Box<Expression>, Box<Expression>),

    More(pt::Loc, Box<Expression>, Box<Expression>),
    Less(pt::Loc, Box<Expression>, Box<Expression>),
    MoreEqual(pt::Loc, Box<Expression>, Box<Expression>),
    LessEqual(pt::Loc, Box<Expression>, Box<Expression>),
    Equal(pt::Loc, Box<Expression>, Box<Expression>),
    NotEqual(pt::Loc, Box<Expression>, Box<Expression>),

    Not(pt::Loc, Box<Expression>),
    Complement(pt::Loc, Type, Box<Expression>),
    UnaryMinus(pt::Loc, Type, Box<Expression>),

    ConditionalOperator(
        pt::Loc,
        Type,
        Box<Expression>,
        Box<Expression>,
        Box<Expression>,
    ),
    Subscript(pt::Loc, Type, Type, Box<Expression>, Box<Expression>),
    StructMember(pt::Loc, Type, Box<Expression>, usize),

    AllocDynamicBytes(pt::Loc, Type, Box<Expression>, Option<Vec<u8>>),
    StorageArrayLength {
        loc: pt::Loc,
        ty: Type,
        array: Box<Expression>,
        elem_ty: Type,
    },
    StringCompare(
        pt::Loc,
        StringLocation<Expression>,
        StringLocation<Expression>,
    ),
    StringConcat(
        pt::Loc,
        Type,
        StringLocation<Expression>,
        StringLocation<Expression>,
    ),

    Or(pt::Loc, Box<Expression>, Box<Expression>),
    And(pt::Loc, Box<Expression>, Box<Expression>),
    InternalFunction {
        loc: pt::Loc,
        ty: Type,
        function_no: usize,
        signature: Option<String>,
    },
    ExternalFunction {
        loc: pt::Loc,
        ty: Type,
        address: Box<Expression>,
        function_no: usize,
    },
    InternalFunctionCall {
        loc: pt::Loc,
        returns: Vec<Type>,
        function: Box<Expression>,
        args: Vec<Expression>,
    },
    ExternalFunctionCall {
        loc: pt::Loc,
        returns: Vec<Type>,
        function: Box<Expression>,
        args: Vec<Expression>,
        call_args: CallArgs,
    },
    ExternalFunctionCallRaw {
        loc: pt::Loc,
        ty: CallTy,
        address: Box<Expression>,
        args: Box<Expression>,
        call_args: CallArgs,
    },
    Constructor {
        loc: pt::Loc,
        contract_no: usize,
        constructor_no: Option<usize>,
        args: Vec<Expression>,
        call_args: CallArgs,
    },
    FormatString(pt::Loc, Vec<(FormatArg, Expression)>),
    Builtin(pt::Loc, Vec<Type>, Builtin, Vec<Expression>),
    InterfaceId(pt::Loc, usize),
    List(pt::Loc, Vec<Expression>),
}

#[derive(PartialEq, Eq, Clone, Default, Debug)]
pub struct CallArgs {
    pub gas: Option<Box<Expression>>,
    pub salt: Option<Box<Expression>>,
    pub value: Option<Box<Expression>>,
    pub address: Option<Box<Expression>>,
    pub accounts: Option<Box<Expression>>,
    pub seeds: Option<Box<Expression>>,
}

impl Recurse for CallArgs {
    type ArgType = Expression;
    fn recurse<T>(&self, cx: &mut T, f: fn(expr: &Expression, ctx: &mut T) -> bool) {
        if let Some(gas) = &self.gas {
            f(gas, cx);
        }
        if let Some(salt) = &self.salt {
            f(salt, cx);
        }
        if let Some(value) = &self.value {
            f(value, cx);
        }
        if let Some(accounts) = &self.accounts {
            f(accounts, cx);
        }
    }
}

impl Recurse for Expression {
    type ArgType = Expression;
    fn recurse<T>(&self, cx: &mut T, f: fn(expr: &Expression, ctx: &mut T) -> bool) {
        if f(self, cx) {
            match self {
                Expression::StructLiteral(_, _, exprs)
                | Expression::ArrayLiteral(_, _, _, exprs)
                | Expression::ConstArrayLiteral(_, _, _, exprs) => {
                    for e in exprs {
                        e.recurse(cx, f);
                    }
                }
                Expression::Add(_, _, _, left, right)
                | Expression::Subtract(_, _, _, left, right)
                | Expression::Multiply(_, _, _, left, right)
                | Expression::Divide(_, _, left, right)
                | Expression::Modulo(_, _, left, right)
                | Expression::Power(_, _, _, left, right)
                | Expression::BitwiseOr(_, _, left, right)
                | Expression::BitwiseAnd(_, _, left, right)
                | Expression::BitwiseXor(_, _, left, right)
                | Expression::ShiftLeft(_, _, left, right)
                | Expression::ShiftRight(_, _, left, right, _) => {
                    left.recurse(cx, f);
                    right.recurse(cx, f);
                }
                Expression::Load(_, _, expr)
                | Expression::StorageLoad(_, _, expr)
                | Expression::ZeroExt { expr, .. }
                | Expression::SignExt { expr, .. }
                | Expression::Trunc { expr, .. }
                | Expression::Cast { expr, .. }
                | Expression::BytesCast { expr, .. }
                | Expression::PreIncrement(_, _, _, expr)
                | Expression::PreDecrement(_, _, _, expr)
                | Expression::PostIncrement(_, _, _, expr)
                | Expression::PostDecrement(_, _, _, expr) => expr.recurse(cx, f),

                Expression::Assign(_, _, left, right)
                | Expression::More(_, left, right)
                | Expression::Less(_, left, right)
                | Expression::MoreEqual(_, left, right)
                | Expression::LessEqual(_, left, right)
                | Expression::Equal(_, left, right)
                | Expression::NotEqual(_, left, right) => {
                    left.recurse(cx, f);
                    right.recurse(cx, f);
                }
                Expression::Not(_, expr)
                | Expression::Complement(_, _, expr)
                | Expression::UnaryMinus(_, _, expr) => expr.recurse(cx, f),

                Expression::ConditionalOperator(_, _, cond, left, right) => {
                    cond.recurse(cx, f);
                    left.recurse(cx, f);
                    right.recurse(cx, f);
                }
                Expression::Subscript(_, _, _, left, right) => {
                    left.recurse(cx, f);
                    right.recurse(cx, f);
                }
                Expression::StructMember(_, _, expr, _) => expr.recurse(cx, f),

                Expression::AllocDynamicBytes(_, _, expr, _) => expr.recurse(cx, f),
                Expression::StorageArrayLength { array, .. } => array.recurse(cx, f),
                Expression::StringCompare(_, left, right)
                | Expression::StringConcat(_, _, left, right) => {
                    if let StringLocation::RunTime(expr) = left {
                        expr.recurse(cx, f);
                    }
                    if let StringLocation::RunTime(expr) = right {
                        expr.recurse(cx, f);
                    }
                }
                Expression::Or(_, left, right) | Expression::And(_, left, right) => {
                    left.recurse(cx, f);
                    right.recurse(cx, f);
                }
                Expression::InternalFunctionCall { function, args, .. } => {
                    function.recurse(cx, f);

                    for e in args {
                        e.recurse(cx, f);
                    }
                }
                Expression::ExternalFunction { address, .. } => {
                    address.recurse(cx, f);
                }
                Expression::ExternalFunctionCall {
                    function,
                    args,
                    call_args,
                    ..
                } => {
                    for e in args {
                        e.recurse(cx, f);
                    }
                    function.recurse(cx, f);
                    call_args.recurse(cx, f);
                }
                Expression::ExternalFunctionCallRaw {
                    address,
                    args,
                    call_args,
                    ..
                } => {
                    args.recurse(cx, f);
                    address.recurse(cx, f);
                    call_args.recurse(cx, f);
                }
                Expression::Constructor {
                    args, call_args, ..
                } => {
                    for e in args {
                        e.recurse(cx, f);
                    }
                    call_args.recurse(cx, f);
                }
                Expression::Builtin(_, _, _, exprs) | Expression::List(_, exprs) => {
                    for e in exprs {
                        e.recurse(cx, f);
                    }
                }
                _ => (),
            }
        }
    }
}

impl CodeLocation for Expression {
    fn loc(&self) -> pt::Loc {
        match self {
            Expression::BoolLiteral(loc, _)
            | Expression::BytesLiteral(loc, ..)
            | Expression::CodeLiteral(loc, ..)
            | Expression::NumberLiteral(loc, ..)
            | Expression::RationalNumberLiteral(loc, ..)
            | Expression::StructLiteral(loc, ..)
            | Expression::ArrayLiteral(loc, ..)
            | Expression::ConstArrayLiteral(loc, ..)
            | Expression::Add(loc, ..)
            | Expression::Subtract(loc, ..)
            | Expression::Multiply(loc, ..)
            | Expression::Divide(loc, ..)
            | Expression::Modulo(loc, ..)
            | Expression::Power(loc, ..)
            | Expression::BitwiseOr(loc, ..)
            | Expression::BitwiseAnd(loc, ..)
            | Expression::BitwiseXor(loc, ..)
            | Expression::ShiftLeft(loc, ..)
            | Expression::ShiftRight(loc, ..)
            | Expression::Variable(loc, ..)
            | Expression::ConstantVariable(loc, ..)
            | Expression::StorageVariable(loc, ..)
            | Expression::Load(loc, ..)
            | Expression::GetRef(loc, ..)
            | Expression::StorageLoad(loc, ..)
            | Expression::ZeroExt { loc, .. }
            | Expression::SignExt { loc, .. }
            | Expression::Trunc { loc, .. }
            | Expression::CheckingTrunc { loc, .. }
            | Expression::Cast { loc, .. }
            | Expression::BytesCast { loc, .. }
            | Expression::More(loc, ..)
            | Expression::Less(loc, ..)
            | Expression::MoreEqual(loc, ..)
            | Expression::LessEqual(loc, ..)
            | Expression::Equal(loc, ..)
            | Expression::NotEqual(loc, ..)
            | Expression::Not(loc, _)
            | Expression::Complement(loc, ..)
            | Expression::UnaryMinus(loc, ..)
            | Expression::ConditionalOperator(loc, ..)
            | Expression::Subscript(loc, ..)
            | Expression::StructMember(loc, ..)
            | Expression::Or(loc, ..)
            | Expression::AllocDynamicBytes(loc, ..)
            | Expression::StorageArrayLength { loc, .. }
            | Expression::StringCompare(loc, ..)
            | Expression::StringConcat(loc, ..)
            | Expression::InternalFunction { loc, .. }
            | Expression::ExternalFunction { loc, .. }
            | Expression::InternalFunctionCall { loc, .. }
            | Expression::ExternalFunctionCall { loc, .. }
            | Expression::ExternalFunctionCallRaw { loc, .. }
            | Expression::Constructor { loc, .. }
            | Expression::PreIncrement(loc, ..)
            | Expression::PreDecrement(loc, ..)
            | Expression::PostIncrement(loc, ..)
            | Expression::PostDecrement(loc, ..)
            | Expression::Builtin(loc, ..)
            | Expression::Assign(loc, ..)
            | Expression::List(loc, _)
            | Expression::FormatString(loc, _)
            | Expression::InterfaceId(loc, ..)
            | Expression::And(loc, ..) => *loc,
        }
    }
}

impl CodeLocation for Statement {
    fn loc(&self) -> pt::Loc {
        match self {
            Statement::Block { loc, .. }
            | Statement::VariableDecl(loc, ..)
            | Statement::If(loc, ..)
            | Statement::While(loc, ..)
            | Statement::For { loc, .. }
            | Statement::DoWhile(loc, ..)
            | Statement::Expression(loc, ..)
            | Statement::Delete(loc, ..)
            | Statement::Destructure(loc, ..)
            | Statement::Continue(loc, ..)
            | Statement::Break(loc, ..)
            | Statement::Return(loc, ..)
            | Statement::Emit { loc, .. }
            | Statement::TryCatch(loc, ..)
            | Statement::Underscore(loc, ..) => *loc,
            Statement::Assembly(..) => pt::Loc::Codegen,
        }
    }
}

impl CodeLocation for Instr {
    fn loc(&self) -> pt::Loc {
        match self {
            Instr::Set { loc, expr, .. } => match loc {
                pt::Loc::File(_, _, _) => *loc,
                _ => expr.loc(),
            },
            Instr::Call { args, .. } if args.is_empty() => pt::Loc::Codegen,
            Instr::Call { args, .. } => args[0].loc(),
            Instr::Return { value } if value.is_empty() => pt::Loc::Codegen,
            Instr::Return { value } => value[0].loc(),
            Instr::EmitEvent { data, .. } if data.is_empty() => pt::Loc::Codegen,
            Instr::EmitEvent { data, .. } => data[0].loc(),
            Instr::BranchCond { cond, .. } => cond.loc(),
            Instr::Store { dest, .. } => dest.loc(),
            Instr::SetStorageBytes { storage, .. }
            | Instr::PushStorage { storage, .. }
            | Instr::PopStorage { storage, .. }
            | Instr::LoadStorage { storage, .. }
            | Instr::ClearStorage { storage, .. } => storage.loc(),
            Instr::ExternalCall { value, .. } | Instr::SetStorage { value, .. } => value.loc(),
            Instr::PushMemory { value, .. } => value.loc(),
            Instr::Constructor { gas, .. } => gas.loc(),
            Instr::ValueTransfer { address, .. } => address.loc(),
            Instr::AbiDecode { data, .. } => data.loc(),
            Instr::SelfDestruct { recipient } => recipient.loc(),
            Instr::WriteBuffer { buf, .. } => buf.loc(),
            Instr::Print { expr } => expr.loc(),
            Instr::MemCopy {
                source,
                destination,
                ..
            } => match source.loc() {
                pt::Loc::File(_, _, _) => source.loc(),
                _ => destination.loc(),
            },
            Instr::Switch { cond, .. } => cond.loc(),
            Instr::ReturnData { data, .. } => data.loc(),
            Instr::Branch { .. }
            | Instr::Unreachable
            | Instr::ReturnCode { .. }
            | Instr::Nop
            | Instr::AssertFailure { .. }
            | Instr::PopMemory { .. } => pt::Loc::Codegen,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug, Eq)]
pub enum FormatArg {
    StringLiteral,
    Default,
    Binary,
    Hex,
}

impl fmt::Display for FormatArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatArg::StringLiteral => write!(f, ""),
            FormatArg::Default => write!(f, ""),
            FormatArg::Binary => write!(f, ":b"),
            FormatArg::Hex => write!(f, ":x"),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum StringLocation<T> {
    CompileTime(Vec<u8>),
    RunTime(Box<T>),
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Builtin {
    GetAddress,
    Balance,
    PayableSend,
    PayableTransfer,
    ArrayPush,
    ArrayPop,
    ArrayLength,
    Assert,
    Print,
    Revert,
    Require,
    SelfDestruct,
    Keccak256,
    Ripemd160,
    Sha256,
    Blake2_128,
    Blake2_256,
    Gasleft,
    BlockCoinbase,
    BlockDifficulty,
    GasLimit,
    BlockNumber,
    Slot,
    ProgramId,
    Timestamp,
    Calldata,
    Sender,
    Signature,
    Value,
    Gasprice,
    Origin,
    BlockHash,
    Random,
    MinimumBalance,
    AbiDecode,
    AbiEncode,
    AbiEncodePacked,
    AbiEncodeWithSelector,
    AbiEncodeWithSignature,
    AbiEncodeCall,
    MulMod,
    AddMod,
    ExternalFunctionAddress,
    FunctionSelector,
    SignatureVerify,
    ReadInt8,
    ReadInt16LE,
    ReadInt32LE,
    ReadInt64LE,
    ReadInt128LE,
    ReadInt256LE,
    ReadUint8,
    ReadUint16LE,
    ReadUint32LE,
    ReadUint64LE,
    ReadUint128LE,
    ReadUint256LE,
    ReadAddress,
    WriteInt8,
    WriteInt16LE,
    WriteInt32LE,
    WriteInt64LE,
    WriteInt128LE,
    WriteInt256LE,
    WriteUint8,
    WriteUint16LE,
    WriteUint32LE,
    WriteUint64LE,
    WriteUint128LE,
    WriteUint256LE,
    WriteAddress,
    WriteString,
    WriteBytes,
    Accounts,
    UserTypeWrap,
    UserTypeUnwrap,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum CallTy {
    Regular,
    Delegate,
    Static,
}

impl fmt::Display for CallTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallTy::Regular => write!(f, "regular"),
            CallTy::Static => write!(f, "static"),
            CallTy::Delegate => write!(f, "delegate"),
        }
    }
}

#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Statement {
    Block {
        loc: pt::Loc,
        unchecked: bool,
        statements: Vec<Statement>,
    },
    VariableDecl(pt::Loc, usize, Parameter, Option<Arc<Expression>>),
    If(pt::Loc, bool, Expression, Vec<Statement>, Vec<Statement>),
    While(pt::Loc, bool, Expression, Vec<Statement>),
    For {
        loc: pt::Loc,
        reachable: bool,
        init: Vec<Statement>,
        cond: Option<Expression>,
        next: Vec<Statement>,
        body: Vec<Statement>,
    },
    DoWhile(pt::Loc, bool, Vec<Statement>, Expression),
    Expression(pt::Loc, bool, Expression),
    Delete(pt::Loc, Type, Expression),
    Destructure(pt::Loc, Vec<DestructureField>, Expression),
    Continue(pt::Loc),
    Break(pt::Loc),
    Return(pt::Loc, Option<Expression>),
    Emit {
        loc: pt::Loc,
        event_no: usize,
        event_loc: pt::Loc,
        args: Vec<Expression>,
    },
    TryCatch(pt::Loc, bool, TryCatch),
    Underscore(pt::Loc),
    Assembly(InlineAssembly, bool),
}

#[derive(Clone, Debug)]
pub struct TryCatch {
    pub expr: Expression,
    pub returns: Vec<(Option<usize>, Parameter)>,
    pub ok_stmt: Vec<Statement>,
    pub errors: Vec<(Option<usize>, Parameter, Vec<Statement>)>,
    pub catch_param: Option<Parameter>,
    pub catch_param_pos: Option<usize>,
    pub catch_stmt: Vec<Statement>,
}

#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum DestructureField {
    None,
    Expression(Expression),
    VariableDecl(usize, Parameter),
}

impl OptionalCodeLocation for DestructureField {
    fn loc(&self) -> Option<pt::Loc> {
        match self {
            DestructureField::None => None,
            DestructureField::Expression(e) => Some(e.loc()),
            DestructureField::VariableDecl(_, p) => Some(p.loc),
        }
    }
}

impl Recurse for Statement {
    type ArgType = Statement;
    fn recurse<T>(&self, cx: &mut T, f: fn(stmt: &Statement, ctx: &mut T) -> bool) {
        if f(self, cx) {
            match self {
                Statement::Block { statements, .. } => {
                    for stmt in statements {
                        stmt.recurse(cx, f);
                    }
                }
                Statement::If(_, _, _, then_stmt, else_stmt) => {
                    for stmt in then_stmt {
                        stmt.recurse(cx, f);
                    }

                    for stmt in else_stmt {
                        stmt.recurse(cx, f);
                    }
                }
                Statement::For {
                    init, next, body, ..
                } => {
                    for stmt in init {
                        stmt.recurse(cx, f);
                    }

                    for stmt in body {
                        stmt.recurse(cx, f);
                    }

                    for stmt in next {
                        stmt.recurse(cx, f);
                    }
                }
                Statement::While(_, _, _, body) => {
                    for stmt in body {
                        stmt.recurse(cx, f);
                    }
                }
                Statement::DoWhile(_, _, body, _) => {
                    for stmt in body {
                        stmt.recurse(cx, f);
                    }
                }
                Statement::TryCatch(_, _, try_catch) => {
                    for stmt in &try_catch.ok_stmt {
                        stmt.recurse(cx, f);
                    }

                    for error_stmt in &try_catch.errors {
                        for stmt in &error_stmt.2 {
                            stmt.recurse(cx, f);
                        }
                    }

                    for stmt in &try_catch.catch_stmt {
                        stmt.recurse(cx, f);
                    }
                }
                _ => (),
            }
        }
    }
}

impl Statement {
    /// Shorthand for checking underscore
    pub fn is_underscore(&self) -> bool {
        matches!(&self, Statement::Underscore(_))
    }

    pub fn reachable(&self) -> bool {
        match self {
            Statement::Block { statements, .. } => statements.iter().all(|s| s.reachable()),
            Statement::Underscore(_)
            | Statement::Destructure(..)
            | Statement::VariableDecl(..)
            | Statement::Emit { .. }
            | Statement::Delete(..) => true,

            Statement::Continue(_) | Statement::Break(_) | Statement::Return(..) => false,

            Statement::If(_, reachable, ..)
            | Statement::While(_, reachable, ..)
            | Statement::DoWhile(_, reachable, ..)
            | Statement::Expression(_, reachable, _)
            | Statement::For { reachable, .. }
            | Statement::TryCatch(_, reachable, _)
            | Statement::Assembly(_, reachable) => *reachable,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Tag {
    pub loc: pt::Loc,
    pub tag: String,
    pub no: usize,
    pub value: String,
}
