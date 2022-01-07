use super::symtable::Symtable;
use crate::codegen::cfg::ControlFlowGraph;
pub use crate::parser::diagnostics::*;
use crate::parser::pt;
use crate::Target;
use num_bigint::BigInt;
use num_rational::BigRational;
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    path::PathBuf,
};
use tiny_keccak::{Hasher, Keccak};

#[derive(PartialEq, Clone, Eq, Hash, Debug)]
pub enum Type {
    Address(bool),
    Bool,
    Int(u16),
    Uint(u16),
    Rational,
    Bytes(u8),
    DynamicBytes,
    String,
    Array(Box<Type>, Vec<Option<BigInt>>),
    Enum(usize),
    Struct(usize),
    Mapping(Box<Type>, Box<Type>),
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
    /// There is no way to declare value in Solidity (should there be?)
    Value,
    Void,
    Unreachable,
    /// DynamicBytes and String are lowered to a vector.
    Slice,
}

#[derive(PartialEq, Clone, Debug)]
pub struct StructDecl {
    pub tags: Vec<Tag>,
    pub name: String,
    pub loc: Option<pt::Loc>,
    pub contract: Option<String>,
    pub fields: Vec<Parameter>,
    // List of offsets of the fields, last entry is the offset for the struct overall size
    pub offsets: Vec<BigInt>,
    // Same, but now in storage
    pub storage_offsets: Vec<BigInt>,
}

#[derive(PartialEq, Clone, Debug)]
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
    /// Make the struct name into a string for printing. The enum can be declared either
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
    pub values: HashMap<String, (pt::Loc, usize)>,
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

#[derive(PartialEq, Clone, Debug)]
pub struct Parameter {
    pub loc: pt::Loc,
    pub name: String,
    // The name can empty (e.g. in an event field or unnamed parameter/return)
    pub name_loc: Option<pt::Loc>,
    pub ty: Type,
    pub ty_loc: pt::Loc,
    pub indexed: bool,
    pub readonly: bool,
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
    pub params: Vec<Parameter>,
    pub returns: Vec<Parameter>,
    // constructor arguments for base contracts, only present on constructors
    pub bases: BTreeMap<usize, (pt::Loc, usize, Vec<Expression>)>,
    // modifiers for functions
    pub modifiers: Vec<Expression>,
    pub is_virtual: bool,
    /// Is this function an acccesor function created by a public variable
    pub is_accessor: bool,
    pub is_override: Option<(pt::Loc, Vec<usize>)>,
    /// Was the function declared with a body
    pub has_body: bool,
    /// The resolved body (if any)
    pub body: Vec<Statement>,
    pub symtable: Symtable,
    // What events are emitted by the body of this function
    pub emits_events: Vec<usize>,
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

        Function {
            tags,
            loc,
            name,
            contract_no,
            ty,
            signature,
            mutability,
            visibility,
            params,
            returns,
            bases: BTreeMap::new(),
            modifiers: Vec::new(),
            is_virtual: false,
            is_accessor: false,
            has_body: false,
            is_override: None,
            body: Vec::new(),
            symtable: Symtable::new(),
            emits_events: Vec::new(),
        }
    }

    /// Generate selector for this function
    pub fn selector(&self) -> u32 {
        let mut res = [0u8; 32];

        let mut hasher = Keccak::v256();
        hasher.update(self.signature.as_bytes());
        hasher.finalize(&mut res);

        u32::from_be_bytes([res[0], res[1], res[2], res[3]])
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

    /// Return a unique string for this function which is a valid llvm symbol
    pub fn llvm_symbol(&self, ns: &Namespace) -> String {
        let mut sig = self.name.to_owned();

        if !self.params.is_empty() {
            sig.push_str("__");

            for (i, p) in self.params.iter().enumerate() {
                if i > 0 {
                    sig.push('_');
                }

                sig.push_str(&p.ty.to_llvm_string(ns));
            }
        }

        sig
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
            pt::Type::Mapping(_, _, _) => unimplemented!(),
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

#[derive(Clone, PartialEq)]
pub enum Symbol {
    Enum(pt::Loc, usize),
    Function(Vec<(pt::Loc, usize)>),
    Variable(pt::Loc, Option<usize>, usize),
    Struct(pt::Loc, usize),
    Event(Vec<(pt::Loc, usize)>),
    Contract(pt::Loc, usize),
    Import(pt::Loc, usize),
}

impl Symbol {
    pub fn loc(&self) -> &pt::Loc {
        match self {
            Symbol::Enum(loc, _) => loc,
            Symbol::Function(funcs) => &funcs[0].0,
            Symbol::Variable(loc, _, _) => loc,
            Symbol::Struct(loc, _) => loc,
            Symbol::Event(events) => &events[0].0,
            Symbol::Contract(loc, _) => loc,
            Symbol::Import(loc, _) => loc,
        }
    }

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
}

/// Any Solidity file, either the main file or anything that was imported
#[derive(Clone)]
pub struct File {
    /// The on-disk filename
    pub path: PathBuf,
    /// Used for offset to line-column conversions
    pub line_starts: Vec<usize>,
    /// Indicates the file number in FileResolver.files
    pub cache_no: usize,
}

/// When resolving a Solidity file, this holds all the resolved items
pub struct Namespace {
    pub target: Target,
    pub files: Vec<File>,
    pub enums: Vec<EnumDecl>,
    pub structs: Vec<StructDecl>,
    pub events: Vec<EventDecl>,
    pub contracts: Vec<Contract>,
    /// All functions
    pub functions: Vec<Function>,
    /// Global constants
    pub constants: Vec<Variable>,
    /// address length in bytes
    pub address_length: usize,
    /// value length in bytes
    pub value_length: usize,
    pub diagnostics: Vec<Diagnostic>,
    /// There is a separate namespace for functions and non-functions
    pub function_symbols: HashMap<(usize, Option<usize>, String), Symbol>,
    /// Symbol key is file_no, contract, identifier
    pub variable_symbols: HashMap<(usize, Option<usize>, String), Symbol>,
    // each variable in the symbol table should have a unique number
    pub next_id: usize,
    /// For a variable reference at a location, give the constant value
    /// This for use by the language server to show the value of a variable at a location
    pub var_constants: HashMap<pt::Loc, Expression>,
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

pub struct Contract {
    pub tags: Vec<Tag>,
    pub loc: pt::Loc,
    pub ty: pt::ContractTy,
    pub name: String,
    pub bases: Vec<Base>,
    pub using: Vec<(usize, Option<Type>)>,
    pub layout: Vec<Layout>,
    pub fixed_layout_size: BigInt,
    pub functions: Vec<usize>,
    pub all_functions: BTreeMap<usize, usize>,
    pub virtual_functions: HashMap<String, usize>,
    pub variables: Vec<Variable>,
    // List of contracts this contract instantiates
    pub creates: Vec<usize>,
    // List of events this contract produces
    pub sends_events: Vec<usize>,
    pub initializer: Option<usize>,
    pub default_constructor: Option<(Function, usize)>,
    pub cfg: Vec<ControlFlowGraph>,
    pub code: Vec<u8>,
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

#[derive(PartialEq, Clone, Debug)]
pub enum Expression {
    FunctionArg(pt::Loc, Type, usize),
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
    StorageLoad(pt::Loc, Type, Box<Expression>),
    ZeroExt(pt::Loc, Type, Box<Expression>),
    SignExt(pt::Loc, Type, Box<Expression>),
    Trunc(pt::Loc, Type, Box<Expression>),
    Cast(pt::Loc, Type, Box<Expression>),
    BytesCast(pt::Loc, Type, Type, Box<Expression>),

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

    Ternary(
        pt::Loc,
        Type,
        Box<Expression>,
        Box<Expression>,
        Box<Expression>,
    ),
    Subscript(pt::Loc, Type, Type, Box<Expression>, Box<Expression>),
    StructMember(pt::Loc, Type, Box<Expression>, usize),

    AllocDynamicArray(pt::Loc, Type, Box<Expression>, Option<Vec<u8>>),
    StorageArrayLength {
        loc: pt::Loc,
        ty: Type,
        array: Box<Expression>,
        elem_ty: Type,
    },
    StringCompare(pt::Loc, StringLocation, StringLocation),
    StringConcat(pt::Loc, Type, StringLocation, StringLocation),

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
        value: Option<Box<Expression>>,
        gas: Option<Box<Expression>>,
    },
    ExternalFunctionCallRaw {
        loc: pt::Loc,
        ty: CallTy,
        address: Box<Expression>,
        args: Box<Expression>,
        value: Option<Box<Expression>>,
        gas: Option<Box<Expression>>,
    },
    Constructor {
        loc: pt::Loc,
        contract_no: usize,
        constructor_no: Option<usize>,
        args: Vec<Expression>,
        gas: Option<Box<Expression>>,
        value: Option<Box<Expression>>,
        salt: Option<Box<Expression>>,
        space: Option<Box<Expression>>,
    },
    FormatString(pt::Loc, Vec<(FormatArg, Expression)>),
    Builtin(pt::Loc, Vec<Type>, Builtin, Vec<Expression>),
    InterfaceId(pt::Loc, usize),
    List(pt::Loc, Vec<Expression>),
    // The remaining types are only generated during codegen
    Keccak256(pt::Loc, Type, Vec<Expression>),
    ReturnData(pt::Loc),
    AbiEncode {
        loc: pt::Loc,
        tys: Vec<Type>,
        packed: Vec<Expression>,
        args: Vec<Expression>,
    },
    InternalFunctionCfg(usize),
    Undefined(Type),
    Poison,
}

impl Expression {
    /// Recurse over expression and copy each element through a filter. This allows the optimizer passes to create
    /// copies of expressions while modifying the results slightly
    #[must_use]
    pub fn copy_filter<T, F>(&self, ctx: &mut T, filter: F) -> Expression
    where
        F: Fn(&Expression, &mut T) -> Expression,
    {
        filter(
            &match self {
                Expression::StructLiteral(loc, ty, args) => Expression::StructLiteral(
                    *loc,
                    ty.clone(),
                    args.iter().map(|e| filter(e, ctx)).collect(),
                ),
                Expression::ArrayLiteral(loc, ty, lengths, args) => Expression::ArrayLiteral(
                    *loc,
                    ty.clone(),
                    lengths.clone(),
                    args.iter().map(|e| filter(e, ctx)).collect(),
                ),
                Expression::ConstArrayLiteral(loc, ty, lengths, args) => {
                    Expression::ConstArrayLiteral(
                        *loc,
                        ty.clone(),
                        lengths.clone(),
                        args.iter().map(|e| filter(e, ctx)).collect(),
                    )
                }
                Expression::Add(loc, ty, unchecked, left, right) => Expression::Add(
                    *loc,
                    ty.clone(),
                    *unchecked,
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::Subtract(loc, ty, unchecked, left, right) => Expression::Subtract(
                    *loc,
                    ty.clone(),
                    *unchecked,
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::Multiply(loc, ty, unchecked, left, right) => Expression::Multiply(
                    *loc,
                    ty.clone(),
                    *unchecked,
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::Divide(loc, ty, left, right) => Expression::Divide(
                    *loc,
                    ty.clone(),
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::Power(loc, ty, unchecked, left, right) => Expression::Power(
                    *loc,
                    ty.clone(),
                    *unchecked,
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::BitwiseOr(loc, ty, left, right) => Expression::BitwiseOr(
                    *loc,
                    ty.clone(),
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::BitwiseAnd(loc, ty, left, right) => Expression::BitwiseAnd(
                    *loc,
                    ty.clone(),
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::BitwiseXor(loc, ty, left, right) => Expression::BitwiseXor(
                    *loc,
                    ty.clone(),
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::ShiftLeft(loc, ty, left, right) => Expression::ShiftLeft(
                    *loc,
                    ty.clone(),
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::ShiftRight(loc, ty, left, right, sign_extend) => {
                    Expression::ShiftRight(
                        *loc,
                        ty.clone(),
                        Box::new(filter(left, ctx)),
                        Box::new(filter(right, ctx)),
                        *sign_extend,
                    )
                }
                Expression::Load(loc, ty, expr) => {
                    Expression::Load(*loc, ty.clone(), Box::new(filter(expr, ctx)))
                }
                Expression::StorageLoad(loc, ty, expr) => {
                    Expression::StorageLoad(*loc, ty.clone(), Box::new(filter(expr, ctx)))
                }
                Expression::ZeroExt(loc, ty, expr) => {
                    Expression::ZeroExt(*loc, ty.clone(), Box::new(filter(expr, ctx)))
                }
                Expression::SignExt(loc, ty, expr) => {
                    Expression::SignExt(*loc, ty.clone(), Box::new(filter(expr, ctx)))
                }
                Expression::Trunc(loc, ty, expr) => {
                    Expression::Trunc(*loc, ty.clone(), Box::new(filter(expr, ctx)))
                }
                Expression::Cast(loc, ty, expr) => {
                    Expression::Cast(*loc, ty.clone(), Box::new(filter(expr, ctx)))
                }
                Expression::BytesCast(loc, ty, from, expr) => Expression::BytesCast(
                    *loc,
                    ty.clone(),
                    from.clone(),
                    Box::new(filter(expr, ctx)),
                ),
                Expression::PreIncrement(loc, ty, unchecked, expr) => Expression::PreIncrement(
                    *loc,
                    ty.clone(),
                    *unchecked,
                    Box::new(filter(expr, ctx)),
                ),
                Expression::PreDecrement(loc, ty, unchecked, expr) => Expression::PreDecrement(
                    *loc,
                    ty.clone(),
                    *unchecked,
                    Box::new(filter(expr, ctx)),
                ),
                Expression::PostIncrement(loc, ty, unchecked, expr) => Expression::PostIncrement(
                    *loc,
                    ty.clone(),
                    *unchecked,
                    Box::new(filter(expr, ctx)),
                ),
                Expression::PostDecrement(loc, ty, unchecked, expr) => Expression::PostDecrement(
                    *loc,
                    ty.clone(),
                    *unchecked,
                    Box::new(filter(expr, ctx)),
                ),
                Expression::Assign(loc, ty, left, right) => Expression::Assign(
                    *loc,
                    ty.clone(),
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::More(loc, left, right) => Expression::More(
                    *loc,
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::Less(loc, left, right) => Expression::Less(
                    *loc,
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::MoreEqual(loc, left, right) => Expression::MoreEqual(
                    *loc,
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::LessEqual(loc, left, right) => Expression::LessEqual(
                    *loc,
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::Equal(loc, left, right) => Expression::Equal(
                    *loc,
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::NotEqual(loc, left, right) => Expression::NotEqual(
                    *loc,
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::Not(loc, expr) => Expression::Not(*loc, Box::new(filter(expr, ctx))),
                Expression::Complement(loc, ty, expr) => {
                    Expression::Complement(*loc, ty.clone(), Box::new(filter(expr, ctx)))
                }
                Expression::UnaryMinus(loc, ty, expr) => {
                    Expression::UnaryMinus(*loc, ty.clone(), Box::new(filter(expr, ctx)))
                }
                Expression::Ternary(loc, ty, cond, left, right) => Expression::Ternary(
                    *loc,
                    ty.clone(),
                    Box::new(filter(cond, ctx)),
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::Subscript(loc, elem_ty, array_ty, left, right) => {
                    Expression::Subscript(
                        *loc,
                        elem_ty.clone(),
                        array_ty.clone(),
                        Box::new(filter(left, ctx)),
                        Box::new(filter(right, ctx)),
                    )
                }
                Expression::StructMember(loc, ty, expr, field) => {
                    Expression::StructMember(*loc, ty.clone(), Box::new(filter(expr, ctx)), *field)
                }
                Expression::AllocDynamicArray(loc, ty, expr, initializer) => {
                    Expression::AllocDynamicArray(
                        *loc,
                        ty.clone(),
                        Box::new(filter(expr, ctx)),
                        initializer.clone(),
                    )
                }
                Expression::StorageArrayLength {
                    loc,
                    ty,
                    array,
                    elem_ty,
                } => Expression::StorageArrayLength {
                    loc: *loc,
                    ty: ty.clone(),
                    array: Box::new(filter(array, ctx)),
                    elem_ty: elem_ty.clone(),
                },
                Expression::StringCompare(loc, left, right) => Expression::StringCompare(
                    *loc,
                    match left {
                        StringLocation::CompileTime(_) => left.clone(),
                        StringLocation::RunTime(expr) => {
                            StringLocation::RunTime(Box::new(filter(expr, ctx)))
                        }
                    },
                    match right {
                        StringLocation::CompileTime(_) => right.clone(),
                        StringLocation::RunTime(expr) => {
                            StringLocation::RunTime(Box::new(filter(expr, ctx)))
                        }
                    },
                ),
                Expression::StringConcat(loc, ty, left, right) => Expression::StringConcat(
                    *loc,
                    ty.clone(),
                    match left {
                        StringLocation::CompileTime(_) => left.clone(),
                        StringLocation::RunTime(expr) => {
                            StringLocation::RunTime(Box::new(filter(expr, ctx)))
                        }
                    },
                    match right {
                        StringLocation::CompileTime(_) => right.clone(),
                        StringLocation::RunTime(expr) => {
                            StringLocation::RunTime(Box::new(filter(expr, ctx)))
                        }
                    },
                ),
                Expression::Or(loc, left, right) => Expression::Or(
                    *loc,
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::And(loc, left, right) => Expression::And(
                    *loc,
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::ExternalFunction {
                    loc,
                    ty,
                    address,
                    function_no,
                } => Expression::ExternalFunction {
                    loc: *loc,
                    ty: ty.clone(),
                    address: Box::new(filter(address, ctx)),
                    function_no: *function_no,
                },
                Expression::InternalFunctionCall {
                    loc,
                    returns,
                    function,
                    args,
                } => Expression::InternalFunctionCall {
                    loc: *loc,
                    returns: returns.clone(),
                    function: Box::new(filter(function, ctx)),
                    args: args.iter().map(|e| filter(e, ctx)).collect(),
                },
                Expression::ExternalFunctionCall {
                    loc,
                    returns,
                    function,
                    args,
                    value,
                    gas,
                } => Expression::ExternalFunctionCall {
                    loc: *loc,
                    returns: returns.clone(),
                    function: Box::new(filter(function, ctx)),
                    args: args.iter().map(|e| filter(e, ctx)).collect(),
                    value: value.as_ref().map(|value| Box::new(filter(value, ctx))),
                    gas: gas.as_ref().map(|gas| Box::new(filter(gas, ctx))),
                },
                Expression::ExternalFunctionCallRaw {
                    loc,
                    ty,
                    address,
                    args,
                    value,
                    gas,
                } => Expression::ExternalFunctionCallRaw {
                    loc: *loc,
                    ty: ty.clone(),
                    address: Box::new(filter(address, ctx)),
                    args: Box::new(filter(args, ctx)),
                    value: value.as_ref().map(|value| Box::new(filter(value, ctx))),
                    gas: gas.as_ref().map(|gas| Box::new(filter(gas, ctx))),
                },
                Expression::Constructor {
                    loc,
                    contract_no,
                    constructor_no,
                    args,
                    gas,
                    value,
                    salt,
                    space,
                } => Expression::Constructor {
                    loc: *loc,
                    contract_no: *contract_no,
                    constructor_no: *constructor_no,
                    args: args.iter().map(|e| filter(e, ctx)).collect(),
                    value: value.as_ref().map(|e| Box::new(filter(e, ctx))),
                    gas: gas.as_ref().map(|e| Box::new(filter(e, ctx))),
                    salt: salt.as_ref().map(|e| Box::new(filter(e, ctx))),
                    space: space.as_ref().map(|e| Box::new(filter(e, ctx))),
                },
                Expression::Keccak256(loc, ty, args) => {
                    let args = args.iter().map(|e| filter(e, ctx)).collect();

                    Expression::Keccak256(*loc, ty.clone(), args)
                }
                Expression::FormatString(loc, args) => {
                    let args = args.iter().map(|(f, e)| (*f, filter(e, ctx))).collect();

                    Expression::FormatString(*loc, args)
                }
                Expression::Builtin(loc, tys, builtin, args) => {
                    let args = args.iter().map(|e| filter(e, ctx)).collect();

                    Expression::Builtin(*loc, tys.clone(), *builtin, args)
                }
                Expression::AbiEncode {
                    loc,
                    tys,
                    packed,
                    args,
                } => {
                    let packed = packed.iter().map(|e| filter(e, ctx)).collect();
                    let args = args.iter().map(|e| filter(e, ctx)).collect();

                    Expression::AbiEncode {
                        loc: *loc,
                        tys: tys.clone(),
                        packed,
                        args,
                    }
                }
                _ => self.clone(),
            },
            ctx,
        )
    }

    /// recurse over the expression
    pub fn recurse<T>(&self, cx: &mut T, f: fn(expr: &Expression, ctx: &mut T) -> bool) {
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
                | Expression::ZeroExt(_, _, expr)
                | Expression::SignExt(_, _, expr)
                | Expression::Trunc(_, _, expr)
                | Expression::Cast(_, _, expr)
                | Expression::BytesCast(_, _, _, expr)
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

                Expression::Ternary(_, _, cond, left, right) => {
                    cond.recurse(cx, f);
                    left.recurse(cx, f);
                    right.recurse(cx, f);
                }
                Expression::Subscript(_, _, _, left, right) => {
                    left.recurse(cx, f);
                    right.recurse(cx, f);
                }
                Expression::StructMember(_, _, expr, _) => expr.recurse(cx, f),

                Expression::AllocDynamicArray(_, _, expr, _) => expr.recurse(cx, f),
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
                    value,
                    gas,
                    ..
                } => {
                    for e in args {
                        e.recurse(cx, f);
                    }
                    function.recurse(cx, f);
                    if let Some(value) = value {
                        value.recurse(cx, f);
                    }
                    if let Some(gas) = gas {
                        gas.recurse(cx, f);
                    }
                }
                Expression::ExternalFunctionCallRaw {
                    address,
                    args,
                    value,
                    gas,
                    ..
                } => {
                    args.recurse(cx, f);
                    address.recurse(cx, f);
                    if let Some(value) = value {
                        value.recurse(cx, f);
                    }
                    if let Some(gas) = gas {
                        gas.recurse(cx, f);
                    }
                }
                Expression::Constructor {
                    args,
                    value,
                    gas,
                    salt,
                    ..
                } => {
                    for e in args {
                        e.recurse(cx, f);
                    }
                    if let Some(value) = value {
                        value.recurse(cx, f);
                    }
                    if let Some(gas) = gas {
                        gas.recurse(cx, f);
                    }
                    if let Some(salt) = salt {
                        salt.recurse(cx, f);
                    }
                }
                Expression::Builtin(_, _, _, exprs)
                | Expression::List(_, exprs)
                | Expression::Keccak256(_, _, exprs) => {
                    for e in exprs {
                        e.recurse(cx, f);
                    }
                }
                Expression::AbiEncode { packed, args, .. } => {
                    for e in packed {
                        e.recurse(cx, f);
                    }
                    for e in args {
                        e.recurse(cx, f);
                    }
                }
                _ => (),
            }
        }
    }

    /// Return the location for this expression
    pub fn loc(&self) -> pt::Loc {
        match self {
            Expression::FunctionArg(loc, _, _)
            | Expression::BoolLiteral(loc, _)
            | Expression::BytesLiteral(loc, _, _)
            | Expression::CodeLiteral(loc, _, _)
            | Expression::NumberLiteral(loc, _, _)
            | Expression::RationalNumberLiteral(loc, _, _)
            | Expression::StructLiteral(loc, _, _)
            | Expression::ArrayLiteral(loc, _, _, _)
            | Expression::ConstArrayLiteral(loc, _, _, _)
            | Expression::Add(loc, _, _, _, _)
            | Expression::Subtract(loc, _, _, _, _)
            | Expression::Multiply(loc, _, _, _, _)
            | Expression::Divide(loc, _, _, _)
            | Expression::Modulo(loc, _, _, _)
            | Expression::Power(loc, _, _, _, _)
            | Expression::BitwiseOr(loc, _, _, _)
            | Expression::BitwiseAnd(loc, _, _, _)
            | Expression::BitwiseXor(loc, _, _, _)
            | Expression::ShiftLeft(loc, _, _, _)
            | Expression::ShiftRight(loc, _, _, _, _)
            | Expression::Variable(loc, _, _)
            | Expression::ConstantVariable(loc, _, _, _)
            | Expression::StorageVariable(loc, _, _, _)
            | Expression::Load(loc, _, _)
            | Expression::StorageLoad(loc, _, _)
            | Expression::ZeroExt(loc, _, _)
            | Expression::SignExt(loc, _, _)
            | Expression::Trunc(loc, _, _)
            | Expression::Cast(loc, _, _)
            | Expression::BytesCast(loc, _, _, _)
            | Expression::More(loc, _, _)
            | Expression::Less(loc, _, _)
            | Expression::MoreEqual(loc, _, _)
            | Expression::LessEqual(loc, _, _)
            | Expression::Equal(loc, _, _)
            | Expression::NotEqual(loc, _, _)
            | Expression::Not(loc, _)
            | Expression::Complement(loc, _, _)
            | Expression::UnaryMinus(loc, _, _)
            | Expression::Ternary(loc, _, _, _, _)
            | Expression::Subscript(loc, ..)
            | Expression::StructMember(loc, _, _, _)
            | Expression::Or(loc, _, _)
            | Expression::AllocDynamicArray(loc, _, _, _)
            | Expression::StorageArrayLength { loc, .. }
            | Expression::StringCompare(loc, _, _)
            | Expression::StringConcat(loc, _, _, _)
            | Expression::Keccak256(loc, _, _)
            | Expression::ReturnData(loc)
            | Expression::InternalFunction { loc, .. }
            | Expression::ExternalFunction { loc, .. }
            | Expression::InternalFunctionCall { loc, .. }
            | Expression::ExternalFunctionCall { loc, .. }
            | Expression::ExternalFunctionCallRaw { loc, .. }
            | Expression::Constructor { loc, .. }
            | Expression::PreIncrement(loc, _, _, _)
            | Expression::PreDecrement(loc, _, _, _)
            | Expression::PostIncrement(loc, _, _, _)
            | Expression::PostDecrement(loc, _, _, _)
            | Expression::Builtin(loc, _, _, _)
            | Expression::Assign(loc, _, _, _)
            | Expression::List(loc, _)
            | Expression::FormatString(loc, _)
            | Expression::AbiEncode { loc, .. }
            | Expression::InterfaceId(loc, ..)
            | Expression::And(loc, _, _) => *loc,
            Expression::InternalFunctionCfg(_) | Expression::Undefined(_) | Expression::Poison => {
                unreachable!()
            }
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
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

#[derive(PartialEq, Clone, Debug)]
pub enum StringLocation {
    CompileTime(Vec<u8>),
    RunTime(Box<Expression>),
}

#[derive(PartialEq, Clone, Copy, Debug)]
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
    TombstoneDeposit,
    AbiDecode,
    AbiEncode,
    AbiEncodePacked,
    AbiEncodeWithSelector,
    AbiEncodeWithSignature,
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
    Accounts,
}

#[derive(PartialEq, Clone, Debug)]
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
    VariableDecl(pt::Loc, usize, Parameter, Option<Expression>),
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
}

#[derive(Clone, Debug)]
pub struct TryCatch {
    pub expr: Expression,
    pub returns: Vec<(Option<usize>, Parameter)>,
    pub ok_stmt: Vec<Statement>,
    pub error: Option<(Option<usize>, Parameter, Vec<Statement>)>,
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

impl DestructureField {
    pub fn loc(&self) -> Option<pt::Loc> {
        match self {
            DestructureField::None => None,
            DestructureField::Expression(e) => Some(e.loc()),
            DestructureField::VariableDecl(_, p) => Some(p.loc),
        }
    }
}

impl Statement {
    /// recurse over the statement
    pub fn recurse<T>(&self, cx: &mut T, f: fn(stmt: &Statement, ctx: &mut T) -> bool) {
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

                    if let Some((_, _, error)) = &try_catch.error {
                        for stmt in error {
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

    /// Shorthand for checking underscore
    pub fn is_underscore(&self) -> bool {
        matches!(&self, Statement::Underscore(_))
    }

    pub fn reachable(&self) -> bool {
        match self {
            Statement::Block { statements, .. } => statements.iter().all(|s| s.reachable()),
            Statement::Underscore(_)
            | Statement::Destructure(_, _, _)
            | Statement::VariableDecl(_, _, _, _) => true,
            Statement::If(_, reachable, _, _, _)
            | Statement::While(_, reachable, _, _)
            | Statement::DoWhile(_, reachable, _, _)
            | Statement::Expression(_, reachable, _) => *reachable,
            Statement::Emit { .. } => true,
            Statement::Delete(_, _, _) => true,
            Statement::Continue(_) | Statement::Break(_) | Statement::Return(_, _) => false,
            Statement::For { reachable, .. } | Statement::TryCatch(_, reachable, _) => *reachable,
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct Tag {
    pub tag: String,
    pub no: usize,
    pub value: String,
}
