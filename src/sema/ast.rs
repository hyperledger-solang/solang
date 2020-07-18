use codegen::cfg::ControlFlowGraph;
use num_bigint::BigInt;
use num_traits::One;
use output;
use parser::pt;
use sema::symtable::Symtable;
use std::collections::HashMap;
use std::fmt;
use std::ops::Mul;
use tiny_keccak::keccak256;
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
    Void,
    Unreachable,
}

impl Type {
    pub fn to_string(&self, ns: &Namespace) -> String {
        match self {
            Type::Bool => "bool".to_string(),
            Type::Address(false) => "address".to_string(),
            Type::Address(true) => "address payable".to_string(),
            Type::Int(n) => format!("int{}", n),
            Type::Uint(n) => format!("uint{}", n),
            Type::Bytes(n) => format!("bytes{}", n),
            Type::String => "string".to_string(),
            Type::DynamicBytes => "bytes".to_string(),
            Type::Enum(n) => format!("enum {}", ns.enums[*n].print_to_string()),
            Type::Struct(n) => format!("struct {}", ns.structs[*n].print_to_string()),
            Type::Array(ty, len) => format!(
                "{}{}",
                ty.to_string(ns),
                len.iter()
                    .map(|l| match l {
                        None => "[]".to_string(),
                        Some(l) => format!("[{}]", l),
                    })
                    .collect::<String>()
            ),
            Type::Mapping(k, v) => format!("mapping({} => {})", k.to_string(ns), v.to_string(ns)),
            Type::Contract(n) => format!("contract {}", ns.contracts[*n].name),
            Type::Ref(r) => r.to_string(ns),
            Type::StorageRef(ty) => format!("{} storage", ty.to_string(ns)),
            Type::Void => "void".to_owned(),
            Type::Unreachable => "unreachable".to_owned(),
        }
    }

    /// Is this a primitive, i.e. bool, address, int, uint, bytes
    pub fn is_primitive(&self) -> bool {
        match self {
            Type::Bool => true,
            Type::Address(_) => true,
            Type::Int(_) => true,
            Type::Uint(_) => true,
            Type::Bytes(_) => true,
            Type::Ref(r) => r.is_primitive(),
            Type::StorageRef(r) => r.is_primitive(),
            _ => false,
        }
    }

    pub fn to_signature_string(&self, ns: &Namespace) -> String {
        match self {
            Type::Bool => "bool".to_string(),
            Type::Contract(_) | Type::Address(_) => "address".to_string(),
            Type::Int(n) => format!("int{}", n),
            Type::Uint(n) => format!("uint{}", n),
            Type::Bytes(n) => format!("bytes{}", n),
            Type::DynamicBytes => "bytes".to_string(),
            Type::String => "string".to_string(),
            Type::Enum(n) => ns.enums[*n].ty.to_signature_string(ns),
            Type::Array(ty, len) => format!(
                "{}{}",
                ty.to_signature_string(ns),
                len.iter()
                    .map(|l| match l {
                        None => "[]".to_string(),
                        Some(l) => format!("[{}]", l),
                    })
                    .collect::<String>()
            ),
            Type::Ref(r) => r.to_string(ns),
            Type::StorageRef(r) => r.to_string(ns),
            Type::Struct(_) => "tuple".to_owned(),
            _ => unreachable!(),
        }
    }

    /// Give the type of an memory array after dereference.
    pub fn array_deref(&self) -> Self {
        match self {
            Type::String | Type::DynamicBytes => Type::Ref(Box::new(Type::Uint(8))),
            Type::Ref(t) => t.array_deref(),
            Type::Array(ty, dim) if dim.len() > 1 => {
                Type::Array(ty.clone(), dim[..dim.len() - 1].to_vec())
            }
            Type::Array(ty, dim) if dim.len() == 1 => Type::Ref(Box::new(*ty.clone())),
            Type::Bytes(_) => Type::Bytes(1),
            _ => panic!("deref on non-array"),
        }
    }

    /// Given an array, return the type of its elements
    pub fn array_elem(&self) -> Self {
        match self {
            Type::Array(ty, dim) if dim.len() > 1 => {
                Type::Array(ty.clone(), dim[..dim.len() - 1].to_vec())
            }
            Type::Array(ty, dim) if dim.len() == 1 => *ty.clone(),
            _ => panic!("not an array"),
        }
    }

    /// Give the type of an storage array after dereference. This can only be used on
    /// array types and will cause a panic otherwise.
    pub fn storage_array_elem(&self) -> Self {
        match self {
            Type::Array(ty, dim) if dim.len() > 1 => Type::StorageRef(Box::new(Type::Array(
                ty.clone(),
                dim[..dim.len() - 1].to_vec(),
            ))),
            Type::Array(ty, dim) if dim.len() == 1 => Type::StorageRef(Box::new(*ty.clone())),
            Type::StorageRef(ty) => ty.storage_array_elem(),
            _ => panic!("deref on non-array"),
        }
    }

    /// Give the length of the outer array. This can only be called on array types
    /// and will panic otherwise.
    pub fn array_length(&self) -> Option<&BigInt> {
        match self {
            Type::StorageRef(ty) => ty.array_length(),
            Type::Ref(ty) => ty.array_length(),
            Type::Array(_, dim) => dim.last().unwrap().as_ref(),
            _ => panic!("array_length on non-array"),
        }
    }

    /// Calculate how much memory we expect this type to use when allocated on the
    /// stack or on the heap. Depending on the llvm implementation there might be
    /// padding between elements which is not accounted for.
    pub fn size_hint(&self, ns: &Namespace) -> BigInt {
        match self {
            Type::Enum(_) => BigInt::one(),
            Type::Bool => BigInt::one(),
            Type::Contract(_) | Type::Address(_) => BigInt::from(ns.address_length),
            Type::Bytes(n) => BigInt::from(*n),
            Type::Uint(n) | Type::Int(n) => BigInt::from(n / 8),
            Type::Array(ty, dims) => {
                let pointer_size = BigInt::from(4);
                ty.size_hint(ns).mul(
                    dims.iter()
                        .map(|d| match d {
                            None => &pointer_size,
                            Some(d) => d,
                        })
                        .product::<BigInt>(),
                )
            }
            Type::Struct(n) => ns.structs[*n]
                .fields
                .iter()
                .map(|f| f.ty.size_hint(ns))
                .sum(),
            Type::String | Type::DynamicBytes => BigInt::from(4),
            _ => unimplemented!(),
        }
    }

    pub fn bits(&self, ns: &Namespace) -> u16 {
        match self {
            Type::Address(_) => ns.address_length as u16 * 8,
            Type::Bool => 1,
            Type::Int(n) => *n,
            Type::Uint(n) => *n,
            Type::Bytes(n) => *n as u16 * 8,
            Type::Enum(n) => ns.enums[*n].ty.bits(ns),
            _ => panic!("type not allowed"),
        }
    }

    pub fn is_signed_int(&self) -> bool {
        match self {
            Type::Int(_) => true,
            Type::Ref(r) => r.is_signed_int(),
            Type::StorageRef(r) => r.is_signed_int(),
            _ => false,
        }
    }

    pub fn ordered(&self) -> bool {
        match self {
            Type::Int(_) => true,
            Type::Uint(_) => true,
            Type::Struct(_) => unreachable!(),
            Type::Array(_, _) => unreachable!(),
            Type::Ref(r) => r.ordered(),
            Type::StorageRef(r) => r.ordered(),
            _ => false,
        }
    }

    /// Calculate how many storage slots a type occupies. Note that storage arrays can
    /// be very large
    pub fn storage_slots(&self, ns: &Namespace) -> BigInt {
        match self {
            Type::StorageRef(r) | Type::Ref(r) => r.storage_slots(ns),
            Type::Struct(n) => ns.structs[*n]
                .fields
                .iter()
                .map(|f| f.ty.storage_slots(ns))
                .sum(),
            Type::Array(ty, dims) => {
                let one = BigInt::one();

                ty.storage_slots(ns)
                    * dims
                        .iter()
                        .map(|l| match l {
                            None => &one,
                            Some(l) => l,
                        })
                        .product::<BigInt>()
            }
            _ => BigInt::one(),
        }
    }

    /// Is this type an reference type in the solidity language? (struct, array, mapping)
    pub fn is_reference_type(&self) -> bool {
        match self {
            Type::Bool => false,
            Type::Address(_) => false,
            Type::Int(_) => false,
            Type::Uint(_) => false,
            Type::Bytes(_) => false,
            Type::Enum(_) => false,
            Type::Struct(_) => true,
            Type::Array(_, _) => true,
            Type::DynamicBytes => true,
            Type::String => true,
            Type::Mapping(_, _) => true,
            Type::Contract(_) => false,
            Type::Ref(r) => r.is_reference_type(),
            Type::StorageRef(r) => r.is_reference_type(),
            _ => unreachable!(),
        }
    }

    /// Does this type contain any types which are variable-length
    pub fn is_dynamic(&self, ns: &Namespace) -> bool {
        match self {
            Type::String | Type::DynamicBytes => true,
            Type::Ref(r) => r.is_dynamic(ns),
            Type::Array(ty, dim) => {
                if dim.iter().any(|d| d.is_none()) {
                    return true;
                }

                ty.is_dynamic(ns)
            }
            Type::Struct(n) => ns.structs[*n].fields.iter().any(|f| f.ty.is_dynamic(ns)),
            Type::StorageRef(r) => r.is_dynamic(ns),
            _ => false,
        }
    }

    /// Can this type have a calldata, memory, or storage location. This is to be
    /// compatible with ethereum solidity. Opinions on whether other types should be
    /// allowed be storage are welcome.
    pub fn can_have_data_location(&self) -> bool {
        match self {
            Type::Array(_, _)
            | Type::Struct(_)
            | Type::Mapping(_, _)
            | Type::String
            | Type::DynamicBytes => true,
            _ => false,
        }
    }

    /// Is this a reference to contract storage?
    pub fn is_contract_storage(&self) -> bool {
        match self {
            Type::StorageRef(_) => true,
            _ => false,
        }
    }

    /// Is this a storage bytes string
    pub fn is_storage_bytes(&self) -> bool {
        if let Type::StorageRef(ty) = self {
            if let Type::DynamicBytes = ty.as_ref() {
                return true;
            }
        }

        false
    }

    /// Is this a mapping
    pub fn is_mapping(&self) -> bool {
        match self {
            Type::Mapping(_, _) => true,
            Type::StorageRef(ty) => ty.is_mapping(),
            _ => false,
        }
    }

    /// Does the type contain any mapping type
    pub fn contains_mapping(&self, ns: &Namespace) -> bool {
        match self {
            Type::Mapping(_, _) => true,
            Type::Array(ty, _) => ty.contains_mapping(ns),
            Type::Struct(n) => ns.structs[*n]
                .fields
                .iter()
                .any(|f| f.ty.contains_mapping(ns)),
            Type::StorageRef(r) | Type::Ref(r) => r.contains_mapping(ns),
            _ => false,
        }
    }

    /// If the type is Ref or StorageRef, get the underlying type
    pub fn deref_any(&self) -> &Self {
        match self {
            Type::StorageRef(r) => r,
            Type::Ref(r) => r,
            _ => self,
        }
    }

    /// If the type is Ref, get the underlying type
    pub fn deref_memory(&self) -> &Self {
        match self {
            Type::Ref(r) => r,
            _ => self,
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct StructField {
    pub name: String,
    pub loc: pt::Loc,
    pub ty: Type,
}

#[derive(PartialEq, Clone, Debug)]
pub struct StructDecl {
    pub name: String,
    pub loc: pt::Loc,
    pub contract: Option<String>,
    pub fields: Vec<StructField>,
}

impl StructDecl {
    /// Make the struct name into a string for printing. The enum can be declared either
    /// inside or outside a contract.
    pub fn print_to_string(&self) -> String {
        match &self.contract {
            Some(c) => format!("{}.{}", c, self.name),
            None => self.name.to_owned(),
        }
    }
}

pub struct EnumDecl {
    pub name: String,
    pub contract: Option<String>,
    pub loc: pt::Loc,
    pub ty: Type,
    pub values: HashMap<String, (pt::Loc, usize)>,
}

impl EnumDecl {
    /// Make the enum name into a string for printing. The enum can be declared either
    /// inside or outside a contract.
    pub fn print_to_string(&self) -> String {
        match &self.contract {
            Some(c) => format!("{}.{}", c, self.name),
            None => self.name.to_owned(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Parameter {
    pub loc: pt::Loc,
    pub name: String,
    pub ty: Type,
}

pub struct Function {
    pub doc: Vec<String>,
    pub loc: pt::Loc,
    pub name: String,
    pub ty: pt::FunctionTy,
    pub signature: String,
    pub ast_index: Option<usize>,
    pub mutability: Option<pt::StateMutability>,
    pub visibility: pt::Visibility,
    pub params: Vec<Parameter>,
    pub returns: Vec<Parameter>,
    pub noreturn: bool,
    pub is_virtual: bool,
    pub check_nonpayable: bool,
    pub body: Vec<Statement>,
    pub symtable: Symtable,
    pub cfg: Option<ControlFlowGraph>,
}

impl Function {
    pub fn new(
        loc: pt::Loc,
        name: String,
        doc: Vec<String>,
        ty: pt::FunctionTy,
        ast_index: Option<usize>,
        mutability: Option<pt::StateMutability>,
        visibility: pt::Visibility,
        params: Vec<Parameter>,
        returns: Vec<Parameter>,
        ns: &Namespace,
    ) -> Self {
        let signature = format!(
            "{}({})",
            name,
            params
                .iter()
                .map(|p| p.ty.to_signature_string(ns))
                .collect::<Vec<String>>()
                .join(",")
        );

        Function {
            doc,
            loc,
            name,
            ty,
            signature,
            ast_index,
            mutability,
            visibility,
            params,
            returns,
            noreturn: false,
            is_virtual: false,
            check_nonpayable: false,
            body: Vec::new(),
            cfg: None,
            symtable: Symtable::new(),
        }
    }

    /// Generate selector for this function
    pub fn selector(&self) -> u32 {
        let res = keccak256(self.signature.as_bytes());

        u32::from_le_bytes([res[0], res[1], res[2], res[3]])
    }

    /// Is this a constructor
    pub fn is_constructor(&self) -> bool {
        self.ty == pt::FunctionTy::Constructor
    }

    /// Does this function have the payable state
    pub fn is_payable(&self) -> bool {
        if let Some(pt::StateMutability::Payable(_)) = self.mutability {
            true
        } else {
            false
        }
    }

    /// Is this function accessable externally
    pub fn is_public(&self) -> bool {
        match self.visibility {
            pt::Visibility::Public(_) | pt::Visibility::External(_) => true,
            _ => false,
        }
    }

    /// Return a unique string for this function which is a valid wasm symbol
    pub fn wasm_symbol(&self, ns: &Namespace) -> String {
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
                        Type::Enum(i) => ns.enums[*i].print_to_string(),
                        Type::Struct(i) => ns.structs[*i].print_to_string(),
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
    pub fn print_mutability(&self) -> &'static str {
        match &self.mutability {
            None => "nonpayable",
            Some(m) => m.to_string(),
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
    Storage(BigInt),
    Constant,
}

pub struct ContractVariable {
    pub doc: Vec<String>,
    pub name: String,
    pub loc: pt::Loc,
    pub ty: Type,
    pub visibility: pt::Visibility,
    pub var: ContractVariableType,
    pub initializer: Option<Expression>,
}

impl ContractVariable {
    pub fn is_storage(&self) -> bool {
        if let ContractVariableType::Storage(_) = self.var {
            true
        } else {
            false
        }
    }

    pub fn get_storage_slot(&self) -> Expression {
        if let ContractVariableType::Storage(n) = &self.var {
            Expression::NumberLiteral(pt::Loc(0, 0, 0), Type::Uint(256), n.clone())
        } else {
            panic!("get_storage_slot called on non-storage variable");
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum Symbol {
    Enum(pt::Loc, usize),
    Function(Vec<(pt::Loc, usize)>),
    Variable(pt::Loc, usize),
    Struct(pt::Loc, usize),
    Contract(pt::Loc, usize),
    Import(pt::Loc, usize),
}

/// When resolving a Solidity file, this holds all the resolved items
pub struct Namespace {
    pub target: Target,
    pub files: Vec<String>,
    pub enums: Vec<EnumDecl>,
    pub structs: Vec<StructDecl>,
    pub contracts: Vec<Contract>,
    pub address_length: usize,
    pub value_length: usize,
    pub diagnostics: Vec<output::Output>,
    /// Symbol key is file_no, contract, identifier
    pub symbols: HashMap<(usize, Option<usize>, String), Symbol>,
}

pub struct Contract {
    pub doc: Vec<String>,
    pub loc: pt::Loc,
    pub ty: pt::ContractTy,
    pub name: String,
    pub inherit: Vec<usize>,
    // events
    pub functions: Vec<Function>,
    pub variables: Vec<ContractVariable>,
    pub top_of_contract_storage: BigInt,
    pub creates: Vec<usize>,
    pub initializer: ControlFlowGraph,
}

impl Contract {
    // Is this a concrete contract, which can be instantiated
    pub fn is_concrete(&self) -> bool {
        if let pt::ContractTy::Contract(_) = self.ty {
            true
        } else {
            false
        }
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
    ConstantVariable(pt::Loc, Type, usize),
    StorageVariable(pt::Loc, Type, usize),
    Load(pt::Loc, Type, Box<Expression>),
    StorageLoad(pt::Loc, Type, Box<Expression>),
    ZeroExt(pt::Loc, Type, Box<Expression>),
    SignExt(pt::Loc, Type, Box<Expression>),
    Trunc(pt::Loc, Type, Box<Expression>),
    Cast(pt::Loc, Type, Box<Expression>),

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
    StorageBytesSubscript(pt::Loc, Box<Expression>, Box<Expression>),
    StorageBytesPush(pt::Loc, Box<Expression>, Box<Expression>),
    StorageBytesPop(pt::Loc, Box<Expression>),
    StorageBytesLength(pt::Loc, Box<Expression>),
    StringCompare(pt::Loc, StringLocation, StringLocation),
    StringConcat(pt::Loc, Type, StringLocation, StringLocation),

    Or(pt::Loc, Box<Expression>, Box<Expression>),
    And(pt::Loc, Box<Expression>, Box<Expression>),
    InternalFunctionCall(pt::Loc, Vec<Type>, usize, Vec<Expression>),
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
        constructor_no: usize,
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
                | Expression::StorageBytesPush(_, left, right) => {
                    left.recurse(cx, f);
                    right.recurse(cx, f);
                }
                Expression::StorageBytesPop(_, expr) | Expression::StorageBytesLength(_, expr) => {
                    expr.recurse(cx, f)
                }
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
                Expression::InternalFunctionCall(_, _, _, args) => {
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
}

#[derive(Clone, Debug)]
pub enum DestructureField {
    None,
    Expression(Expression),
    VariableDecl(usize, Parameter),
}

impl Statement {
    pub fn reachable(&self) -> bool {
        match self {
            Statement::Destructure(_, _, _) | Statement::VariableDecl(_, _, _, _) => true,
            Statement::If(_, reachable, _, _, _)
            | Statement::While(_, reachable, _, _)
            | Statement::DoWhile(_, reachable, _, _)
            | Statement::Expression(_, reachable, _) => *reachable,
            Statement::Delete(_, _, _) => true,
            Statement::Continue(_) | Statement::Break(_) | Statement::Return(_, _) => false,
            Statement::For { reachable, .. } | Statement::TryCatch { reachable, .. } => *reachable,
        }
    }
}
