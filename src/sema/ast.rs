use super::symtable::Symtable;
use codegen::cfg::ControlFlowGraph;
use num_bigint::BigInt;
use parser::pt;
use std::collections::HashMap;
use std::fmt;
use tiny_keccak::{Hasher, Keccak};
use Target;

#[derive(PartialEq, Clone, Debug)]
pub enum Type {
    Address(bool),
    Bool,
    Int(u16),
    Uint(u16),
    Bytes(u8),
    DynamicBytes,
    String,
    Array(Box<Type>, Vec<Option<BigInt>>),
    Enum(usize),
    Struct(usize),
    Mapping(Box<Type>, Box<Type>),
    Contract(usize),
    Ref(Box<Type>),
    StorageRef(Box<Type>),
    /// There is no way to declare this type in Solidity
    Value,
    Void,
    Unreachable,
}

#[derive(PartialEq, Clone, Debug)]
pub struct StructDecl {
    pub tags: Vec<Tag>,
    pub name: String,
    pub loc: pt::Loc,
    pub contract: Option<String>,
    pub fields: Vec<Parameter>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct EventDecl {
    pub tags: Vec<Tag>,
    pub name: String,
    pub loc: pt::Loc,
    pub contract: Option<String>,
    pub fields: Vec<Parameter>,
    pub signature: String,
    pub anonymous: bool,
}

impl fmt::Display for EventDecl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.contract {
            Some(c) => write!(f, "{}.{}", c, self.name),
            None => write!(f, "{}", self.name),
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
}

pub struct Function {
    pub tags: Vec<Tag>,
    pub loc: pt::Loc,
    pub name: String,
    pub ty: pt::FunctionTy,
    pub signature: String,
    pub mutability: Option<pt::StateMutability>,
    pub visibility: pt::Visibility,
    pub params: Vec<Parameter>,
    pub returns: Vec<Parameter>,
    // constructor arguments for base contracts, only present on constructors
    pub bases: HashMap<usize, (pt::Loc, usize, Vec<Expression>)>,
    // modifiers for functions
    pub modifiers: Vec<Expression>,
    pub is_virtual: bool,
    pub is_override: Option<(pt::Loc, Vec<usize>)>,
    pub body: Vec<Statement>,
    pub symtable: Symtable,
}

impl Function {
    pub fn new(
        loc: pt::Loc,
        name: String,
        tags: Vec<Tag>,
        ty: pt::FunctionTy,
        mutability: Option<pt::StateMutability>,
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

        Function {
            tags,
            loc,
            name,
            ty,
            signature,
            mutability,
            visibility,
            params,
            returns,
            bases: HashMap::new(),
            modifiers: Vec::new(),
            is_virtual: false,
            is_override: None,
            body: Vec::new(),
            symtable: Symtable::new(),
        }
    }

    /// Generate selector for this function
    pub fn selector(&self) -> u32 {
        let mut res = [0u8; 32];

        let mut hasher = Keccak::v256();
        hasher.update(self.signature.as_bytes());
        hasher.finalize(&mut res);

        u32::from_le_bytes([res[0], res[1], res[2], res[3]])
    }

    /// Is this a constructor
    pub fn is_constructor(&self) -> bool {
        self.ty == pt::FunctionTy::Constructor
    }

    /// Does this function have the payable state
    pub fn is_payable(&self) -> bool {
        matches!(self.mutability, Some(pt::StateMutability::Payable(_)))
    }

    /// Is this function accessable externally
    pub fn is_public(&self) -> bool {
        matches!(self.visibility,
            pt::Visibility::Public(_) | pt::Visibility::External(_))
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

                fn type_to_wasm_name(ty: &Type, ns: &Namespace) -> String {
                    match ty {
                        Type::Bool => "bool".to_string(),
                        Type::Address(_) => "address".to_string(),
                        Type::Int(n) => format!("int{}", n),
                        Type::Uint(n) => format!("uint{}", n),
                        Type::Bytes(n) => format!("bytes{}", n),
                        Type::DynamicBytes => "bytes".to_string(),
                        Type::String => "string".to_string(),
                        Type::Enum(i) => format!("{}", ns.enums[*i]),
                        Type::Struct(i) => format!("{}", ns.structs[*i]),
                        Type::Array(ty, len) => format!(
                            "{}{}",
                            type_to_wasm_name(ty, ns),
                            len.iter()
                                .map(|r| match r {
                                    None => ":".to_string(),
                                    Some(r) => format!(":{}", r),
                                })
                                .collect::<String>()
                        ),
                        Type::Mapping(k, v) => format!(
                            "mapping:{}:{}",
                            type_to_wasm_name(k, ns),
                            type_to_wasm_name(v, ns)
                        ),
                        Type::Contract(i) => ns.contracts[*i].name.to_owned(),
                        Type::Ref(r) => type_to_wasm_name(r, ns),
                        Type::StorageRef(r) => type_to_wasm_name(r, ns),
                        _ => unreachable!(),
                    }
                }

                sig.push_str(&type_to_wasm_name(&p.ty, ns));
            }
        }

        sig
    }

    /// State mutability as string
    pub fn print_mutability(&self) -> String {
        match &self.mutability {
            None => "nonpayable".to_string(),
            Some(m) => format!("{}", m),
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
            pt::Type::DynamicBytes => Type::DynamicBytes,
            // needs special casing
            pt::Type::Mapping(_, _, _) => unimplemented!(),
        }
    }
}

pub enum ContractVariableType {
    Storage,
    Constant,
}

pub struct ContractVariable {
    pub tags: Vec<Tag>,
    pub name: String,
    pub loc: pt::Loc,
    pub ty: Type,
    pub visibility: pt::Visibility,
    pub var: ContractVariableType,
    pub initializer: Option<Expression>,
}

impl ContractVariable {
    pub fn is_storage(&self) -> bool {
        matches!(self.var, ContractVariableType::Storage)
    }
}

#[derive(Clone, PartialEq)]
pub enum Symbol {
    Enum(pt::Loc, usize),
    Function(Vec<pt::Loc>),
    Variable(pt::Loc, usize, usize),
    Struct(pt::Loc, usize),
    Event(Vec<(pt::Loc, usize)>),
    Contract(pt::Loc, usize),
    Import(pt::Loc, usize),
}

impl Symbol {
    pub fn loc(&self) -> &pt::Loc {
        match self {
            Symbol::Enum(loc, _) => loc,
            Symbol::Function(funcs) => &funcs[0],
            Symbol::Variable(loc, _, _) => loc,
            Symbol::Struct(loc, _) => loc,
            Symbol::Event(events) => &events[0].0,
            Symbol::Contract(loc, _) => loc,
            Symbol::Import(loc, _) => loc,
        }
    }
}

/// When resolving a Solidity file, this holds all the resolved items
pub struct Namespace {
    pub target: Target,
    pub files: Vec<String>,
    pub enums: Vec<EnumDecl>,
    pub structs: Vec<StructDecl>,
    pub events: Vec<EventDecl>,
    pub contracts: Vec<Contract>,
    /// address length in bytes
    pub address_length: usize,
    /// value length in bytes
    pub value_length: usize,
    pub diagnostics: Vec<Diagnostic>,
    /// Symbol key is file_no, contract, identifier
    pub symbols: HashMap<(usize, Option<usize>, String), Symbol>,
    // each variable in the symbol table should have a unique number
    pub next_id: usize,
}

pub struct Layout {
    pub slot: BigInt,
    pub contract_no: usize,
    pub var_no: usize,
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
    // list of libraries used by this contract
    pub libraries: Vec<usize>,
    pub using: Vec<(usize, Option<Type>)>,
    pub layout: Vec<Layout>,
    pub functions: Vec<Function>,
    pub all_functions: HashMap<(usize, usize), usize>,
    pub virtual_functions: HashMap<String, (usize, usize)>,
    pub variables: Vec<ContractVariable>,
    // List of contracts this contract instantiates
    pub creates: Vec<usize>,
    // List of events this contract produces
    pub sends_events: Vec<usize>,
    pub initializer: Option<usize>,
    pub default_constructor: Option<(Function, usize)>,
    pub cfg: Vec<ControlFlowGraph>,
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

    /// Get the storage slot for a variable, possibly from base contract
    pub fn get_storage_slot(&self, var_contract_no: usize, var_no: usize) -> Expression {
        if let Some(layout) = self
            .layout
            .iter()
            .find(|l| l.contract_no == var_contract_no && l.var_no == var_no)
        {
            Expression::NumberLiteral(pt::Loc(0, 0, 0), Type::Uint(256), layout.slot.clone())
        } else {
            panic!("get_storage_slot called on non-storage variable");
        }
    }

    /// Does the constructor require arguments. Should be false is there is no constructor
    pub fn constructor_needs_arguments(&self) -> bool {
        self.have_constructor() && self.no_args_constructor().is_none()
    }

    /// Does the contract have a constructor defined
    pub fn have_constructor(&self) -> bool {
        self.functions.iter().any(|f| f.is_constructor())
    }

    /// Return the constructor with no arguments
    pub fn no_args_constructor(&self) -> Option<usize> {
        self.functions
            .iter()
            .position(|f| f.is_constructor() && f.params.is_empty())
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum Expression {
    FunctionArg(pt::Loc, Type, usize),
    BoolLiteral(pt::Loc, bool),
    BytesLiteral(pt::Loc, Type, Vec<u8>),
    CodeLiteral(pt::Loc, usize, bool),
    NumberLiteral(pt::Loc, Type, BigInt),
    StructLiteral(pt::Loc, Type, Vec<Expression>),
    ArrayLiteral(pt::Loc, Type, Vec<u32>, Vec<Expression>),
    ConstArrayLiteral(pt::Loc, Type, Vec<u32>, Vec<Expression>),
    Add(pt::Loc, Type, Box<Expression>, Box<Expression>),
    Subtract(pt::Loc, Type, Box<Expression>, Box<Expression>),
    Multiply(pt::Loc, Type, Box<Expression>, Box<Expression>),
    UDivide(pt::Loc, Type, Box<Expression>, Box<Expression>),
    SDivide(pt::Loc, Type, Box<Expression>, Box<Expression>),
    UModulo(pt::Loc, Type, Box<Expression>, Box<Expression>),
    SModulo(pt::Loc, Type, Box<Expression>, Box<Expression>),
    Power(pt::Loc, Type, Box<Expression>, Box<Expression>),
    BitwiseOr(pt::Loc, Type, Box<Expression>, Box<Expression>),
    BitwiseAnd(pt::Loc, Type, Box<Expression>, Box<Expression>),
    BitwiseXor(pt::Loc, Type, Box<Expression>, Box<Expression>),
    ShiftLeft(pt::Loc, Type, Box<Expression>, Box<Expression>),
    ShiftRight(pt::Loc, Type, Box<Expression>, Box<Expression>, bool),
    Variable(pt::Loc, Type, usize),
    ConstantVariable(pt::Loc, Type, usize, usize),
    StorageVariable(pt::Loc, Type, usize, usize),
    Load(pt::Loc, Type, Box<Expression>),
    StorageLoad(pt::Loc, Type, Box<Expression>),
    ZeroExt(pt::Loc, Type, Box<Expression>),
    SignExt(pt::Loc, Type, Box<Expression>),
    Trunc(pt::Loc, Type, Box<Expression>),
    Cast(pt::Loc, Type, Box<Expression>),
    BytesCast(pt::Loc, Type, Type, Box<Expression>),

    PreIncrement(pt::Loc, Type, Box<Expression>),
    PreDecrement(pt::Loc, Type, Box<Expression>),
    PostIncrement(pt::Loc, Type, Box<Expression>),
    PostDecrement(pt::Loc, Type, Box<Expression>),
    Assign(pt::Loc, Type, Box<Expression>, Box<Expression>),

    UMore(pt::Loc, Box<Expression>, Box<Expression>),
    ULess(pt::Loc, Box<Expression>, Box<Expression>),
    UMoreEqual(pt::Loc, Box<Expression>, Box<Expression>),
    ULessEqual(pt::Loc, Box<Expression>, Box<Expression>),
    SMore(pt::Loc, Box<Expression>, Box<Expression>),
    SLess(pt::Loc, Box<Expression>, Box<Expression>),
    SMoreEqual(pt::Loc, Box<Expression>, Box<Expression>),
    SLessEqual(pt::Loc, Box<Expression>, Box<Expression>),
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
    ArraySubscript(pt::Loc, Type, Box<Expression>, Box<Expression>),
    StructMember(pt::Loc, Type, Box<Expression>, usize),

    AllocDynamicArray(pt::Loc, Type, Box<Expression>, Option<Vec<u8>>),
    DynamicArrayLength(pt::Loc, Box<Expression>),
    DynamicArraySubscript(pt::Loc, Type, Box<Expression>, Box<Expression>),
    DynamicArrayPush(pt::Loc, Box<Expression>, Type, Box<Expression>),
    DynamicArrayPop(pt::Loc, Box<Expression>, Type),
    StorageBytesSubscript(pt::Loc, Box<Expression>, Box<Expression>),
    StorageBytesPush(pt::Loc, Box<Expression>, Box<Expression>),
    StorageBytesPop(pt::Loc, Box<Expression>),
    StorageBytesLength(pt::Loc, Box<Expression>),
    StringCompare(pt::Loc, StringLocation, StringLocation),
    StringConcat(pt::Loc, Type, StringLocation, StringLocation),

    Or(pt::Loc, Box<Expression>, Box<Expression>),
    And(pt::Loc, Box<Expression>, Box<Expression>),
    InternalFunctionCall {
        loc: pt::Loc,
        returns: Vec<Type>,
        contract_no: usize,
        function_no: usize,
        signature: Option<String>,
        args: Vec<Expression>,
    },
    ExternalFunctionCall {
        loc: pt::Loc,
        returns: Vec<Type>,
        contract_no: usize,
        function_no: usize,
        address: Box<Expression>,
        args: Vec<Expression>,
        value: Box<Expression>,
        gas: Box<Expression>,
    },
    ExternalFunctionCallRaw {
        loc: pt::Loc,
        ty: CallTy,
        address: Box<Expression>,
        args: Box<Expression>,
        value: Box<Expression>,
        gas: Box<Expression>,
    },
    Constructor {
        loc: pt::Loc,
        contract_no: usize,
        constructor_no: Option<usize>,
        args: Vec<Expression>,
        gas: Box<Expression>,
        value: Option<Box<Expression>>,
        salt: Option<Box<Expression>>,
    },
    Keccak256(pt::Loc, Type, Vec<Expression>),

    ReturnData(pt::Loc),
    GetAddress(pt::Loc, Type),
    Balance(pt::Loc, Type, Box<Expression>),
    Builtin(pt::Loc, Vec<Type>, Builtin, Vec<Expression>),
    List(pt::Loc, Vec<Expression>),
    Poison,
}

impl Expression {
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
                Expression::Add(_, _, left, right)
                | Expression::Subtract(_, _, left, right)
                | Expression::Multiply(_, _, left, right)
                | Expression::UDivide(_, _, left, right)
                | Expression::SDivide(_, _, left, right)
                | Expression::UModulo(_, _, left, right)
                | Expression::SModulo(_, _, left, right)
                | Expression::Power(_, _, left, right)
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
                | Expression::PreIncrement(_, _, expr)
                | Expression::PreDecrement(_, _, expr)
                | Expression::PostIncrement(_, _, expr)
                | Expression::PostDecrement(_, _, expr) => expr.recurse(cx, f),

                Expression::Assign(_, _, left, right)
                | Expression::UMore(_, left, right)
                | Expression::ULess(_, left, right)
                | Expression::UMoreEqual(_, left, right)
                | Expression::ULessEqual(_, left, right)
                | Expression::SMore(_, left, right)
                | Expression::SLess(_, left, right)
                | Expression::SMoreEqual(_, left, right)
                | Expression::SLessEqual(_, left, right)
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
                Expression::ArraySubscript(_, _, left, right) => {
                    left.recurse(cx, f);
                    right.recurse(cx, f);
                }
                Expression::StructMember(_, _, expr, _) => expr.recurse(cx, f),

                Expression::AllocDynamicArray(_, _, expr, _)
                | Expression::DynamicArrayLength(_, expr) => expr.recurse(cx, f),
                Expression::DynamicArraySubscript(_, _, left, right)
                | Expression::StorageBytesSubscript(_, left, right)
                | Expression::StorageBytesPush(_, left, right)
                | Expression::DynamicArrayPush(_, left, _, right) => {
                    left.recurse(cx, f);
                    right.recurse(cx, f);
                }
                Expression::StorageBytesPop(_, expr)
                | Expression::StorageBytesLength(_, expr)
                | Expression::DynamicArrayPop(_, expr, _) => expr.recurse(cx, f),
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
                Expression::InternalFunctionCall { args, .. } => {
                    for e in args {
                        e.recurse(cx, f);
                    }
                }
                Expression::ExternalFunctionCall {
                    address,
                    args,
                    value,
                    gas,
                    ..
                } => {
                    for e in args {
                        e.recurse(cx, f);
                    }
                    address.recurse(cx, f);
                    value.recurse(cx, f);
                    gas.recurse(cx, f);
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
                    value.recurse(cx, f);
                    gas.recurse(cx, f);
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
                    gas.recurse(cx, f);
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
                Expression::Balance(_, _, expr) => expr.recurse(cx, f),
                _ => (),
            }
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum StringLocation {
    CompileTime(Vec<u8>),
    RunTime(Box<Expression>),
}

#[derive(PartialEq, Clone, Debug)]
pub enum Builtin {
    PayableSend,
    PayableTransfer,
    ArrayPush,
    ArrayPop,
    BytesPush,
    BytesPop,
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
    Return(pt::Loc, Vec<Expression>),
    Emit {
        loc: pt::Loc,
        event_no: usize,
        event_loc: pt::Loc,
        args: Vec<Expression>,
    },
    TryCatch {
        loc: pt::Loc,
        reachable: bool,
        expr: Expression,
        returns: Vec<(Option<usize>, Parameter)>,
        ok_stmt: Vec<Statement>,
        error: Option<(Option<usize>, Parameter, Vec<Statement>)>,
        catch_param: Parameter,
        catch_param_pos: Option<usize>,
        catch_stmt: Vec<Statement>,
    },
    Underscore(pt::Loc),
}

#[derive(Clone, Debug)]
pub enum DestructureField {
    None,
    Expression(Expression),
    VariableDecl(usize, Parameter),
}

impl Statement {
    /// recurse over the statement
    pub fn recurse<T>(&mut self, cx: &mut T, f: fn(stmt: &mut Statement, ctx: &mut T) -> bool) {
        if f(self, cx) {
            match self {
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
                Statement::TryCatch {
                    ok_stmt,
                    catch_stmt,
                    error,
                    ..
                } => {
                    for stmt in ok_stmt {
                        stmt.recurse(cx, f);
                    }

                    if let Some((_, _, error)) = error {
                        for stmt in error {
                            stmt.recurse(cx, f);
                        }
                    }

                    for stmt in catch_stmt {
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
            Statement::For { reachable, .. } | Statement::TryCatch { reachable, .. } => *reachable,
        }
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum Level {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum ErrorType {
    None,
    ParserError,
    SyntaxError,
    DeclarationError,
    TypeError,
    Warning,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct Note {
    pub pos: pt::Loc,
    pub message: String,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct Diagnostic {
    pub level: Level,
    pub ty: ErrorType,
    pub pos: Option<pt::Loc>,
    pub message: String,
    pub notes: Vec<Note>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct Tag {
    pub tag: String,
    pub no: usize,
    pub value: String,
}
