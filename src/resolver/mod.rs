use abi;
use emit;
use num_bigint::BigInt;
use num_traits::Signed;
use num_traits::{One, Zero};
use output::{Note, Output};
use parser::ast;
use std::collections::HashMap;
use std::fmt;
use std::ops::Mul;
use tiny_keccak::keccak256;
use Target;

mod address;
mod builtin;
pub mod cfg;
mod eval;
pub mod expression;
mod functions;
mod storage;
mod structs;
mod variables;

use resolver::cfg::{ControlFlowGraph, Instr, Vartable};
use resolver::eval::eval_number_expression;
use resolver::expression::{expression, Expression};

pub type ArrayDimension = Option<(ast::Loc, BigInt)>;

#[derive(PartialEq, Clone, Debug)]
pub enum Type {
    Address,
    Bool,
    Int(u16),
    Uint(u16),
    Bytes(u8),
    DynamicBytes,
    String,
    Array(Box<Type>, Vec<Option<BigInt>>),
    Enum(usize),
    Struct(usize),
    Ref(Box<Type>),
    StorageRef(Box<Type>),
    Undef,
}

impl Type {
    pub fn to_string(&self, ns: &Contract) -> String {
        match self {
            Type::Bool => "bool".to_string(),
            Type::Address => "address".to_string(),
            Type::Int(n) => format!("int{}", n),
            Type::Uint(n) => format!("uint{}", n),
            Type::Bytes(n) => format!("bytes{}", n),
            Type::String => "string".to_string(),
            Type::DynamicBytes => "bytes".to_string(),
            Type::Enum(n) => format!("enum {}.{}", ns.name, ns.enums[*n].name),
            Type::Struct(n) => format!("struct {}.{}", ns.name, ns.structs[*n].name),
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
            Type::Ref(r) => r.to_string(ns),
            Type::StorageRef(ty) => format!("storage {}", ty.to_string(ns)),
            Type::Undef => "undefined".to_owned(),
        }
    }

    /// Is this a primitive, i.e. bool, address, int, uint, bytes
    pub fn is_primitive(&self) -> bool {
        match self {
            Type::Bool => true,
            Type::Address => true,
            Type::Int(_) => true,
            Type::Uint(_) => true,
            Type::Bytes(_) => true,
            Type::Ref(r) => r.is_primitive(),
            Type::StorageRef(r) => r.is_primitive(),
            _ => false,
        }
    }

    pub fn to_signature_string(&self, ns: &Contract) -> String {
        match self {
            Type::Bool => "bool".to_string(),
            Type::Address => "address".to_string(),
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
            Type::Undef => "undefined".to_owned(),
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
            _ => panic!("deref on non-array"),
        }
    }

    /// Give the type of an storage array after dereference. This can only be used on
    /// array types and will cause a panic otherwise.
    pub fn storage_deref(&self) -> Self {
        match self {
            Type::Array(ty, dim) if dim.len() > 1 => Type::StorageRef(Box::new(Type::Array(
                ty.clone(),
                dim[..dim.len() - 1].to_vec(),
            ))),
            Type::Array(ty, dim) if dim.len() == 1 => Type::StorageRef(Box::new(*ty.clone())),
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
    pub fn size_hint(&self, ns: &Contract) -> BigInt {
        match self {
            Type::Enum(_) => BigInt::one(),
            Type::Bool => BigInt::one(),
            Type::Address => BigInt::from(20),
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

    pub fn bits(&self) -> u16 {
        match self {
            Type::Address => 160,
            Type::Bool => 1,
            Type::Int(n) => *n,
            Type::Uint(n) => *n,
            Type::Bytes(n) => *n as u16 * 8,
            _ => panic!("type not allowed"),
        }
    }

    pub fn signed(&self) -> bool {
        match self {
            Type::Int(_) => true,
            Type::Ref(r) => r.signed(),
            Type::StorageRef(r) => r.signed(),
            _ => false,
        }
    }

    pub fn ordered(&self) -> bool {
        match self {
            Type::Int(_) => true,
            Type::Uint(_) => true,
            Type::Struct(_) => unreachable!(),
            Type::Array(_, _) => unreachable!(),
            Type::Undef => unreachable!(),
            Type::Ref(r) => r.ordered(),
            Type::StorageRef(r) => r.ordered(),
            _ => false,
        }
    }

    /// Calculate how many storage slots a type occupies. Note that storage arrays can
    /// be very large
    pub fn storage_slots(&self, ns: &Contract) -> BigInt {
        match self {
            Type::StorageRef(r) | Type::Ref(r) => r.storage_slots(ns),
            Type::Struct(n) => ns.structs[*n]
                .fields
                .iter()
                .map(|f| f.ty.storage_slots(ns))
                .sum(),
            Type::Undef => unreachable!(),
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

    /// Can this type have a calldata, memory, or storage location. This is to be
    /// compatible with ethereum solidity. Opinions on whether other types should be
    /// allowed be storage are welcome.
    pub fn can_have_data_location(&self) -> bool {
        match self {
            Type::Array(_, _) => true,
            Type::Struct(_) => true,
            Type::String | Type::DynamicBytes => true,
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

    /// If the type is Ref or StorageRef, get the underlying type
    pub fn deref(&self) -> &Self {
        match self {
            Type::StorageRef(r) => r,
            Type::Ref(r) => r,
            _ => self,
        }
    }
}

pub struct StructField {
    pub name: String,
    pub loc: ast::Loc,
    pub ty: Type,
}

pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
}

pub struct EnumDecl {
    pub name: String,
    pub ty: Type,
    pub values: HashMap<String, (ast::Loc, usize)>,
}

pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

pub struct FunctionDecl {
    pub doc: Vec<String>,
    pub loc: ast::Loc,
    pub name: String,
    pub fallback: bool,
    pub signature: String,
    pub ast_index: Option<usize>,
    pub mutability: Option<ast::StateMutability>,
    pub visibility: ast::Visibility,
    pub params: Vec<Parameter>,
    pub returns: Vec<Parameter>,
    pub wasm_return: bool,
    pub cfg: Option<Box<cfg::ControlFlowGraph>>,
}

impl FunctionDecl {
    fn new(
        loc: ast::Loc,
        name: String,
        doc: Vec<String>,
        fallback: bool,
        ast_index: Option<usize>,
        mutability: Option<ast::StateMutability>,
        visibility: ast::Visibility,
        params: Vec<Parameter>,
        returns: Vec<Parameter>,
        ns: &Contract,
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

        let wasm_return = returns.len() == 1 && !returns[0].ty.stack_based();

        FunctionDecl {
            doc,
            loc,
            name,
            fallback,
            signature,
            ast_index,
            mutability,
            visibility,
            params,
            returns,
            wasm_return,
            cfg: None,
        }
    }

    /// Generate selector for this function
    pub fn selector(&self) -> u32 {
        let res = keccak256(self.signature.as_bytes());

        u32::from_le_bytes([res[0], res[1], res[2], res[3]])
    }

    /// Return a unique string for this function which is a valid wasm symbol
    pub fn wasm_symbol(&self, ns: &Contract) -> String {
        let mut sig = self.name.to_owned();

        if !self.params.is_empty() {
            sig.push_str("__");

            for (i, p) in self.params.iter().enumerate() {
                if i > 0 {
                    sig.push('_');
                }

                fn type_to_wasm_name(ty: &Type, ns: &Contract) -> String {
                    match ty {
                        Type::Bool => "bool".to_string(),
                        Type::Address => "address".to_string(),
                        Type::Int(n) => format!("int{}", n),
                        Type::Uint(n) => format!("uint{}", n),
                        Type::Bytes(n) => format!("bytes{}", n),
                        Type::DynamicBytes => "bytes".to_string(),
                        Type::String => "string".to_string(),
                        Type::Enum(i) => ns.enums[*i].name.to_owned(),
                        Type::Struct(i) => ns.structs[*i].name.to_owned(),
                        Type::Array(ty, len) => format!(
                            "{}{}",
                            ty.to_string(ns),
                            len.iter()
                                .map(|r| match r {
                                    None => ":".to_string(),
                                    Some(r) => format!(":{}", r),
                                })
                                .collect::<String>()
                        ),
                        Type::Undef => unreachable!(),
                        Type::Ref(r) => type_to_wasm_name(r, ns),
                        Type::StorageRef(r) => type_to_wasm_name(r, ns),
                    }
                }

                sig.push_str(&type_to_wasm_name(&p.ty, ns));
            }
        }

        sig
    }
}

impl From<ast::Type> for Type {
    fn from(p: ast::Type) -> Type {
        match p {
            ast::Type::Bool => Type::Bool,
            ast::Type::Address => Type::Address,
            ast::Type::Int(n) => Type::Int(n),
            ast::Type::Uint(n) => Type::Uint(n),
            ast::Type::Bytes(n) => Type::Bytes(n),
            ast::Type::String => Type::String,
            ast::Type::DynamicBytes => Type::DynamicBytes,
        }
    }
}

pub enum ContractVariableType {
    Storage(BigInt),
    Constant(usize),
}

pub struct ContractVariable {
    pub doc: Vec<String>,
    pub name: String,
    pub ty: Type,
    pub visibility: ast::Visibility,
    pub var: ContractVariableType,
}

impl ContractVariable {
    pub fn is_storage(&self) -> bool {
        if let ContractVariableType::Storage(_) = self.var {
            true
        } else {
            false
        }
    }
}

pub enum Symbol {
    Enum(ast::Loc, usize),
    Function(Vec<(ast::Loc, usize)>),
    Variable(ast::Loc, usize),
    Struct(ast::Loc, usize),
}

pub struct Contract {
    pub doc: Vec<String>,
    pub name: String,
    pub enums: Vec<EnumDecl>,
    // events
    pub structs: Vec<StructDecl>,
    pub constructors: Vec<FunctionDecl>,
    pub functions: Vec<FunctionDecl>,
    pub variables: Vec<ContractVariable>,
    pub constants: Vec<Expression>,
    pub initializer: cfg::ControlFlowGraph,
    pub target: Target,
    top_of_contract_storage: BigInt,
    symbols: HashMap<String, Symbol>,
}

impl Contract {
    fn add_symbol(
        &mut self,
        id: &ast::Identifier,
        symbol: Symbol,
        errors: &mut Vec<Output>,
    ) -> bool {
        if let Some(prev) = self.symbols.get(&id.name) {
            match prev {
                Symbol::Enum(e, _) => {
                    errors.push(Output::error_with_note(
                        id.loc,
                        format!("{} is already defined as enum", id.name.to_string()),
                        *e,
                        "location of previous definition".to_string(),
                    ));
                }
                Symbol::Function(v) => {
                    let mut notes = Vec::new();

                    for e in v {
                        notes.push(Note {
                            pos: e.0,
                            message: "location of previous definition".into(),
                        });
                    }

                    errors.push(Output::error_with_notes(
                        id.loc,
                        format!("{} is already defined as function", id.name.to_string()),
                        notes,
                    ));
                }
                Symbol::Variable(e, _) => {
                    errors.push(Output::error_with_note(
                        id.loc,
                        format!(
                            "{} is already defined as state variable",
                            id.name.to_string()
                        ),
                        *e,
                        "location of previous definition".to_string(),
                    ));
                }
                Symbol::Struct(e, _) => {
                    errors.push(Output::error_with_note(
                        id.loc,
                        format!(
                            "{} is already defined as struct definition",
                            id.name.to_string()
                        ),
                        *e,
                        "location of previous definition".to_string(),
                    ));
                }
            }
            return false;
        }

        self.symbols.insert(id.name.to_string(), symbol);

        true
    }

    /// Resolve the parsed data type. The type can be a primitive, enum and also an arrays.
    pub fn resolve_type(
        &self,
        id: &ast::ComplexType,
        errors: &mut Vec<Output>,
    ) -> Result<Type, ()> {
        fn resolve_dimensions(
            ast_dimensions: &[Option<(ast::Loc, BigInt)>],
            errors: &mut Vec<Output>,
        ) -> Result<Vec<Option<BigInt>>, ()> {
            let mut dimensions = Vec::new();

            for d in ast_dimensions.iter() {
                if let Some((loc, n)) = d {
                    if n.is_zero() {
                        errors.push(Output::decl_error(
                            *loc,
                            "zero size of array declared".to_string(),
                        ));
                        return Err(());
                    } else if n.is_negative() {
                        errors.push(Output::decl_error(
                            *loc,
                            "negative size of array declared".to_string(),
                        ));
                        return Err(());
                    }
                    dimensions.push(Some(n.clone()));
                } else {
                    dimensions.push(None);
                }
            }

            Ok(dimensions)
        }

        match id {
            ast::ComplexType::Primitive(p, dimensions) if dimensions.is_empty() => {
                Ok(Type::from(*p))
            }
            ast::ComplexType::Primitive(p, exprs) => {
                let mut dimensions = Vec::new();

                for expr in exprs {
                    dimensions.push(match expr {
                        Some(e) => self.resolve_array_dimension(e, errors)?,
                        None => None,
                    });
                }

                Ok(Type::Array(
                    Box::new(Type::from(*p)),
                    resolve_dimensions(&dimensions, errors)?,
                ))
            }
            ast::ComplexType::Unresolved(expr) => {
                let (id, dimensions) = self.expr_to_type(&expr, errors)?;

                match self.symbols.get(&id.name) {
                    None => {
                        errors.push(Output::decl_error(
                            id.loc,
                            format!("type ‘{}’ not found", id.name),
                        ));
                        Err(())
                    }
                    Some(Symbol::Enum(_, n)) if dimensions.is_empty() => Ok(Type::Enum(*n)),
                    Some(Symbol::Enum(_, n)) => Ok(Type::Array(
                        Box::new(Type::Enum(*n)),
                        resolve_dimensions(&dimensions, errors)?,
                    )),
                    Some(Symbol::Struct(_, n)) if dimensions.is_empty() => Ok(Type::Struct(*n)),
                    Some(Symbol::Struct(_, n)) => Ok(Type::Array(
                        Box::new(Type::Struct(*n)),
                        resolve_dimensions(&dimensions, errors)?,
                    )),
                    Some(Symbol::Function(_)) => {
                        errors.push(Output::decl_error(
                            id.loc,
                            format!("‘{}’ is a function", id.name),
                        ));
                        Err(())
                    }
                    Some(Symbol::Variable(_, _)) => {
                        errors.push(Output::decl_error(
                            id.loc,
                            format!("‘{}’ is a contract variable", id.name),
                        ));
                        Err(())
                    }
                }
            }
        }
    }

    // An array type can look like foo[2], if foo is an enum type. The lalrpop parses
    // this as an expression, so we need to convert it to Type and check there are
    // no unexpected expressions types.
    pub fn expr_to_type(
        &self,
        expr: &ast::Expression,
        errors: &mut Vec<Output>,
    ) -> Result<(ast::Identifier, Vec<ArrayDimension>), ()> {
        let mut expr = expr;
        let mut dimensions = Vec::new();

        loop {
            expr = match expr {
                ast::Expression::ArraySubscript(_, r, None) => {
                    dimensions.push(None);

                    &*r
                }
                ast::Expression::ArraySubscript(_, r, Some(index)) => {
                    dimensions.push(self.resolve_array_dimension(index, errors)?);

                    &*r
                }
                ast::Expression::Variable(id) => return Ok((id.clone(), dimensions)),
                _ => {
                    errors.push(Output::decl_error(
                        expr.loc(),
                        "expression found where type expected".to_string(),
                    ));
                    return Err(());
                }
            }
        }
    }

    /// Resolve an expression which defines the array length, e.g. 2**8 in "bool[2**8]"
    pub fn resolve_array_dimension(
        &self,
        expr: &ast::Expression,
        errors: &mut Vec<Output>,
    ) -> Result<ArrayDimension, ()> {
        let mut cfg = ControlFlowGraph::new();
        let (size_expr, size_ty) = expression(&expr, &mut cfg, &self, &mut None, errors)?;
        match size_ty {
            Type::Uint(_) | Type::Int(_) => {}
            _ => {
                errors.push(Output::decl_error(
                    expr.loc(),
                    "expression is not a number".to_string(),
                ));
                return Err(());
            }
        }
        Ok(Some(eval_number_expression(&size_expr, errors)?))
    }

    pub fn resolve_enum(&self, id: &ast::Identifier) -> Option<usize> {
        match self.symbols.get(&id.name) {
            Some(Symbol::Enum(_, n)) => Some(*n),
            _ => None,
        }
    }

    pub fn resolve_func(
        &self,
        id: &ast::Identifier,
        errors: &mut Vec<Output>,
    ) -> Result<&Vec<(ast::Loc, usize)>, ()> {
        match self.symbols.get(&id.name) {
            Some(Symbol::Function(v)) => Ok(v),
            _ => {
                errors.push(Output::error(
                    id.loc,
                    "unknown function or type".to_string(),
                ));

                Err(())
            }
        }
    }

    pub fn resolve_var(&self, id: &ast::Identifier, errors: &mut Vec<Output>) -> Result<usize, ()> {
        match self.symbols.get(&id.name) {
            None => {
                errors.push(Output::decl_error(
                    id.loc,
                    format!("`{}' is not declared", id.name),
                ));
                Err(())
            }
            Some(Symbol::Enum(_, _)) => {
                errors.push(Output::decl_error(
                    id.loc,
                    format!("`{}' is an enum", id.name),
                ));
                Err(())
            }
            Some(Symbol::Struct(_, _)) => {
                errors.push(Output::decl_error(
                    id.loc,
                    format!("`{}' is a struct", id.name),
                ));
                Err(())
            }
            Some(Symbol::Function(_)) => {
                errors.push(Output::decl_error(
                    id.loc,
                    format!("`{}' is a function", id.name),
                ));
                Err(())
            }
            Some(Symbol::Variable(_, n)) => Ok(*n),
        }
    }

    pub fn check_shadowing(&self, id: &ast::Identifier, errors: &mut Vec<Output>) {
        match self.symbols.get(&id.name) {
            Some(Symbol::Enum(loc, _)) => {
                errors.push(Output::warning_with_note(
                    id.loc,
                    format!("declaration of `{}' shadows enum definition", id.name),
                    *loc,
                    "previous definition of enum".to_string(),
                ));
            }
            Some(Symbol::Struct(loc, _)) => {
                errors.push(Output::warning_with_note(
                    id.loc,
                    format!("declaration of `{}' shadows struct definition", id.name),
                    *loc,
                    "previous definition of struct".to_string(),
                ));
            }
            Some(Symbol::Function(v)) => {
                let notes = v
                    .iter()
                    .map(|(pos, _)| Note {
                        pos: *pos,
                        message: "previous declaration of function".to_owned(),
                    })
                    .collect();
                errors.push(Output::warning_with_notes(
                    id.loc,
                    format!("declaration of `{}' shadows function", id.name),
                    notes,
                ));
            }
            Some(Symbol::Variable(loc, _)) => {
                errors.push(Output::warning_with_note(
                    id.loc,
                    format!("declaration of `{}' shadows state variable", id.name),
                    *loc,
                    "previous declaration of state variable".to_string(),
                ));
            }
            None => {}
        }
    }

    pub fn fallback_function(&self) -> Option<usize> {
        for (i, f) in self.functions.iter().enumerate() {
            if f.fallback {
                return Some(i);
            }
        }
        None
    }

    pub fn abi(&self, verbose: bool) -> (String, &'static str) {
        abi::generate_abi(self, verbose)
    }

    pub fn emit<'a>(
        &'a self,
        context: &'a inkwell::context::Context,
        filename: &'a str,
        opt: &str,
    ) -> emit::Contract {
        emit::Contract::build(context, self, filename, opt)
    }
}

impl fmt::Display for Contract {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#\n# Contract: {}\n#\n\n", self.name)?;

        writeln!(f, "# storage initializer")?;
        writeln!(f, "{}", &self.initializer.to_string(self))?;

        for func in &self.constructors {
            writeln!(f, "# constructor {}", func.signature)?;

            if let Some(ref cfg) = func.cfg {
                write!(f, "{}", &cfg.to_string(self))?;
            }
        }

        for (i, func) in self.functions.iter().enumerate() {
            if func.name != "" {
                writeln!(f, "# function({}) {}", i, func.signature)?;
            } else {
                writeln!(f, "# fallback({})", i)?;
            }

            if let Some(ref cfg) = func.cfg {
                writeln!(f, "{}", &cfg.to_string(self))?;
            }
        }

        Ok(())
    }
}

pub fn resolver(s: ast::SourceUnit, target: &Target) -> (Vec<Contract>, Vec<Output>) {
    let mut contracts = Vec::new();
    let mut errors = Vec::new();

    for part in s.0 {
        match part {
            ast::SourceUnitPart::ContractDefinition(def) => {
                if let Some(c) = resolve_contract(def, &target, &mut errors) {
                    contracts.push(c)
                }
            }
            ast::SourceUnitPart::PragmaDirective(name, value) => {
                if name.name == "solidity" {
                    errors.push(Output::info(
                        name.loc,
                        "pragma solidity is ignored".to_string(),
                    ));
                } else if name.name == "experimental" && value.string == "ABIEncoderV2" {
                    errors.push(Output::info(
                        value.loc,
                        "pragma experimental ABIEncoderV2 is ignored".to_string(),
                    ));
                } else {
                    errors.push(Output::warning(
                        name.loc,
                        format!("unknown pragma {} ignored", name.name),
                    ));
                }
            }
            _ => unimplemented!(),
        }
    }

    (contracts, errors)
}

fn resolve_contract(
    def: Box<ast::ContractDefinition>,
    target: &Target,
    errors: &mut Vec<Output>,
) -> Option<Contract> {
    let mut ns = Contract {
        name: def.name.name.to_string(),
        doc: def.doc.clone(),
        enums: Vec::new(),
        structs: Vec::new(),
        constructors: Vec::new(),
        functions: Vec::new(),
        variables: Vec::new(),
        constants: Vec::new(),
        initializer: cfg::ControlFlowGraph::new(),
        target: target.clone(),
        top_of_contract_storage: BigInt::zero(),
        symbols: HashMap::new(),
    };

    errors.push(Output::info(
        def.loc,
        format!("found contract {}", def.name.name),
    ));

    builtin::add_builtin_function(&mut ns);

    let mut broken = false;

    // first resolve enums
    for parts in &def.parts {
        if let ast::ContractPart::EnumDefinition(ref e) = parts {
            if !enum_decl(e, &mut ns, errors) {
                broken = true;
            }
        }
    }

    // FIXME: next resolve event

    // resolve struct definitions
    for parts in &def.parts {
        if let ast::ContractPart::StructDefinition(ref s) = parts {
            if !structs::struct_decl(s, &mut ns, errors) {
                broken = true;
            }
        }
    }

    // resolve function signatures
    for (i, parts) in def.parts.iter().enumerate() {
        if let ast::ContractPart::FunctionDefinition(ref f) = parts {
            if !functions::function_decl(f, i, &mut ns, errors) {
                broken = true;
            }
        }
    }

    // resolve state variables
    if variables::contract_variables(&def, &mut ns, errors) {
        broken = true;
    }

    // resolve constructor bodies
    for f in 0..ns.constructors.len() {
        if let Some(ast_index) = ns.constructors[f].ast_index {
            if let ast::ContractPart::FunctionDefinition(ref ast_f) = def.parts[ast_index] {
                match cfg::generate_cfg(ast_f, &ns.constructors[f], &ns, errors) {
                    Ok(c) => ns.constructors[f].cfg = Some(c),
                    Err(_) => broken = true,
                }
            }
        }
    }

    // Substrate requires one constructor
    if ns.constructors.is_empty() && target == &Target::Substrate {
        let mut fdecl = FunctionDecl::new(
            ast::Loc(0, 0),
            "".to_owned(),
            vec![],
            false,
            None,
            None,
            ast::Visibility::Public(ast::Loc(0, 0)),
            Vec::new(),
            Vec::new(),
            &ns,
        );

        let mut vartab = Vartable::new();
        let mut cfg = ControlFlowGraph::new();

        cfg.add(&mut vartab, Instr::Return { value: Vec::new() });
        cfg.vars = vartab.drain();

        fdecl.cfg = Some(Box::new(cfg));

        ns.constructors.push(fdecl);
    }

    // resolve function bodies
    for f in 0..ns.functions.len() {
        if let Some(ast_index) = ns.functions[f].ast_index {
            if let ast::ContractPart::FunctionDefinition(ref ast_f) = def.parts[ast_index] {
                match cfg::generate_cfg(ast_f, &ns.functions[f], &ns, errors) {
                    Ok(c) => {
                        match &ns.functions[f].mutability {
                            Some(ast::StateMutability::Pure(loc)) => {
                                if c.writes_contract_storage {
                                    errors.push(Output::error(
                                        *loc,
                                        "function declared pure but writes contract storage"
                                            .to_string(),
                                    ));
                                    broken = true;
                                } else if c.reads_contract_storage() {
                                    errors.push(Output::error(
                                        *loc,
                                        "function declared pure but reads contract storage"
                                            .to_string(),
                                    ));
                                    broken = true;
                                }
                            }
                            Some(ast::StateMutability::View(loc)) => {
                                if c.writes_contract_storage {
                                    errors.push(Output::error(
                                        *loc,
                                        "function declared view but writes contract storage"
                                            .to_string(),
                                    ));
                                    broken = true;
                                } else if !c.reads_contract_storage() {
                                    errors.push(Output::warning(
                                        *loc,
                                        "function can be declared pure".to_string(),
                                    ));
                                }
                            }
                            Some(ast::StateMutability::Payable(_)) => {
                                unimplemented!();
                            }
                            None => {
                                let loc = &ns.functions[f].loc;

                                if !c.writes_contract_storage && !c.reads_contract_storage() {
                                    errors.push(Output::warning(
                                        *loc,
                                        "function can be declare pure".to_string(),
                                    ));
                                } else if !c.writes_contract_storage {
                                    errors.push(Output::warning(
                                        *loc,
                                        "function can be declared view".to_string(),
                                    ));
                                }
                            }
                        }
                        ns.functions[f].cfg = Some(c);
                    }
                    Err(_) => broken = true,
                }
            }
        }
    }

    if !broken {
        Some(ns)
    } else {
        None
    }
}

/// Parse enum declaration. If the declaration is invalid, it is still generated
/// so that we can continue parsing, with errors recorded.
fn enum_decl(enum_: &ast::EnumDefinition, ns: &mut Contract, errors: &mut Vec<Output>) -> bool {
    let mut valid = true;

    let mut bits = if enum_.values.is_empty() {
        errors.push(Output::error(
            enum_.name.loc,
            format!("enum ‘{}’ is missing fields", enum_.name.name),
        ));
        valid = false;

        0
    } else {
        // Number of bits required to represent this enum
        std::mem::size_of::<usize>() as u32 * 8 - (enum_.values.len() - 1).leading_zeros()
    };

    // round it up to the next
    if bits <= 8 {
        bits = 8;
    } else {
        bits += 7;
        bits -= bits % 8;
    }

    // check for duplicates
    let mut entries: HashMap<String, (ast::Loc, usize)> = HashMap::new();

    for (i, e) in enum_.values.iter().enumerate() {
        if let Some(prev) = entries.get(&e.name.to_string()) {
            errors.push(Output::error_with_note(
                e.loc,
                format!("duplicate enum value {}", e.name),
                prev.0,
                "location of previous definition".to_string(),
            ));
            valid = false;
            continue;
        }

        entries.insert(e.name.to_string(), (e.loc, i));
    }

    let decl = EnumDecl {
        name: enum_.name.name.to_string(),
        ty: Type::Uint(bits as u16),
        values: entries,
    };

    let pos = ns.enums.len();

    ns.enums.push(decl);

    if !ns.add_symbol(&enum_.name, Symbol::Enum(enum_.name.loc, pos), errors) {
        valid = false;
    }

    valid
}

#[test]
fn enum_256values_is_uint8() {
    let mut e = ast::EnumDefinition {
        doc: vec![],
        name: ast::Identifier {
            loc: ast::Loc(0, 0),
            name: "foo".into(),
        },
        values: Vec::new(),
    };
    let mut ns = Contract {
        name: "foo".to_string(),
        doc: Vec::new(),
        enums: Vec::new(),
        structs: Vec::new(),
        constructors: Vec::new(),
        functions: Vec::new(),
        variables: Vec::new(),
        constants: Vec::new(),
        initializer: cfg::ControlFlowGraph::new(),
        target: Target::Ewasm,
        top_of_contract_storage: BigInt::zero(),
        symbols: HashMap::new(),
    };

    e.values.push(ast::Identifier {
        loc: ast::Loc(0, 0),
        name: "first".into(),
    });

    assert!(enum_decl(&e, &mut ns, &mut Vec::new()));
    assert_eq!(ns.enums.last().unwrap().ty, Type::Uint(8));

    for i in 1..256 {
        e.values.push(ast::Identifier {
            loc: ast::Loc(0, 0),
            name: format!("val{}", i),
        })
    }

    assert_eq!(e.values.len(), 256);

    e.name.name = "foo2".to_owned();
    assert!(enum_decl(&e, &mut ns, &mut Vec::new()));
    assert_eq!(ns.enums.last().unwrap().ty, Type::Uint(8));

    e.values.push(ast::Identifier {
        loc: ast::Loc(0, 0),
        name: "another".into(),
    });

    e.name.name = "foo3".to_owned();
    assert!(enum_decl(&e, &mut ns, &mut Vec::new()));
    assert_eq!(ns.enums.last().unwrap().ty, Type::Uint(16));
}
