use num_bigint::BigInt;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::str;

use super::expression::expression;
use super::statements::{statement, LoopScopes};
use super::vector_to_slice;
use super::{constant_folding, reaching_definitions, strength_reduce};
use crate::parser::pt;
use crate::sema::ast::{
    CallTy, Contract, Expression, Function, Namespace, Parameter, StringLocation, Type,
};
use crate::sema::contracts::{collect_base_args, visit_bases};
use crate::sema::symtable::Symtable;
use crate::Target;

pub type Vars = HashMap<usize, Variable>;

#[derive(Clone)]
pub enum Instr {
    /// Set variable
    Set {
        loc: pt::Loc,
        res: usize,
        expr: Expression,
    },
    /// Call internal function, either static dispatch or dynamic dispatch
    Call {
        res: Vec<usize>,
        return_tys: Vec<Type>,
        call: InternalCallTy,
        args: Vec<Expression>,
    },
    /// Return
    Return { value: Vec<Expression> },
    /// Jump unconditionally
    Branch { block: usize },
    /// Jump conditionally
    BranchCond {
        cond: Expression,
        true_block: usize,
        false_block: usize,
    },
    /// Set array element in memory
    Store { dest: Expression, pos: usize },
    /// Abort execution
    AssertFailure { expr: Option<Expression> },
    /// Print to log message
    Print { expr: Expression },
    /// Clear storage at slot for ty (might span multiple slots)
    ClearStorage { ty: Type, storage: Expression },
    /// Set storage value at slot
    SetStorage {
        ty: Type,
        value: Expression,
        storage: Expression,
    },
    /// In storage slot, set the value at the offset
    SetStorageBytes {
        value: Expression,
        storage: Expression,
        offset: Expression,
    },
    /// Push an element onto a bytes in storage
    PushStorageBytes {
        value: Expression,
        storage: Expression,
    },
    /// Pop an element from a bytes in storage
    PopStorageBytes { res: usize, storage: Expression },
    /// Push element on memory array
    PushMemory {
        res: usize,
        ty: Type,
        array: usize,
        value: Box<Expression>,
    },
    /// Pop element from memory array. The push builtin returns a reference
    /// to the new element which is stored in res.
    PopMemory { res: usize, ty: Type, array: usize },
    /// Create contract and call constructor. If creating the contract fails,
    /// either store the result in success or abort success.
    Constructor {
        success: Option<usize>,
        res: usize,
        contract_no: usize,
        constructor_no: Option<usize>,
        args: Vec<Expression>,
        value: Option<Expression>,
        gas: Expression,
        salt: Option<Expression>,
    },
    /// Call external functions. If the call fails, set the success failure
    /// or abort if this is None
    ExternalCall {
        success: Option<usize>,
        address: Option<Expression>,
        payload: Expression,
        args: Vec<Expression>,
        value: Expression,
        gas: Expression,
        callty: CallTy,
    },
    /// Value transfer; either <address>.send() or <address>.transfer()
    ValueTransfer {
        success: Option<usize>,
        address: Expression,
        value: Expression,
    },
    /// ABI decoder encoded data. If decoding fails, either jump to exception
    /// or abort if this is None.
    AbiDecode {
        res: Vec<usize>,
        selector: Option<u32>,
        exception_block: Option<usize>,
        tys: Vec<Parameter>,
        data: Expression,
    },
    /// ABI encode data, and store the result
    AbiEncodeVector {
        res: usize,
        tys: Vec<Type>,
        packed: bool,
        selector: Option<Expression>,
        args: Vec<Expression>,
    },
    /// Insert unreachable instruction after e.g. self-destruct
    Unreachable,
    /// Self destruct
    SelfDestruct { recipient: Expression },
    /// Emit event
    EmitEvent {
        event_no: usize,
        data: Vec<Expression>,
        data_tys: Vec<Parameter>,
        topics: Vec<Expression>,
        topic_tys: Vec<Parameter>,
    },
}

#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum InternalCallTy {
    Static(usize),
    Dynamic(Expression),
}

#[derive(Clone, PartialEq)]
pub enum HashTy {
    Keccak256,
    Ripemd160,
    Sha256,
    Blake2_256,
    Blake2_128,
}

impl fmt::Display for HashTy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HashTy::Keccak256 => write!(f, "keccak256"),
            HashTy::Ripemd160 => write!(f, "ripemd160"),
            HashTy::Sha256 => write!(f, "sha256"),
            HashTy::Blake2_128 => write!(f, "blake2_128"),
            HashTy::Blake2_256 => write!(f, "blake2_256"),
        }
    }
}

#[derive(Clone)]
pub struct BasicBlock {
    pub phis: Option<HashSet<usize>>,
    pub name: String,
    pub instr: Vec<Instr>,
    pub defs: reaching_definitions::VarDefs,
    pub transfers: Vec<Vec<reaching_definitions::Transfer>>,
}

impl BasicBlock {
    fn add(&mut self, ins: Instr) {
        self.instr.push(ins);
    }
}

#[derive(Clone)]
pub struct ControlFlowGraph {
    pub name: String,
    pub function_no: Option<usize>,
    pub params: Vec<Parameter>,
    pub returns: Vec<Parameter>,
    pub vars: Vars,
    pub blocks: Vec<BasicBlock>,
    pub nonpayable: bool,
    pub public: bool,
    pub ty: pt::FunctionTy,
    pub selector: u32,
    current: usize,
}

impl ControlFlowGraph {
    pub fn new(name: String, function_no: Option<usize>) -> Self {
        let mut cfg = ControlFlowGraph {
            name,
            function_no,
            params: Vec::new(),
            returns: Vec::new(),
            vars: HashMap::new(),
            blocks: Vec::new(),
            nonpayable: false,
            public: false,
            ty: pt::FunctionTy::Function,
            selector: 0,
            current: 0,
        };

        cfg.new_basic_block("entry".to_string());

        cfg
    }

    /// Create an empty CFG which will be replaced later
    pub fn placeholder() -> Self {
        ControlFlowGraph {
            name: String::new(),
            function_no: None,
            params: Vec::new(),
            returns: Vec::new(),
            vars: HashMap::new(),
            blocks: Vec::new(),
            nonpayable: false,
            public: false,
            ty: pt::FunctionTy::Function,
            selector: 0,
            current: 0,
        }
    }

    /// Is this a placeholder
    pub fn is_placeholder(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn new_basic_block(&mut self, name: String) -> usize {
        let pos = self.blocks.len();

        self.blocks.push(BasicBlock {
            name,
            instr: Vec::new(),
            phis: None,
            transfers: Vec::new(),
            defs: HashMap::new(),
        });

        pos
    }

    pub fn set_phis(&mut self, block: usize, phis: HashSet<usize>) {
        if !phis.is_empty() {
            self.blocks[block].phis = Some(phis);
        }
    }

    pub fn set_basic_block(&mut self, pos: usize) {
        self.current = pos;
    }

    pub fn add(&mut self, vartab: &mut Vartable, ins: Instr) {
        if let Instr::Set { res, .. } = ins {
            vartab.set_dirty(res);
        }
        self.blocks[self.current].add(ins);
    }

    pub fn expr_to_string(&self, contract: &Contract, ns: &Namespace, expr: &Expression) -> String {
        match expr {
            Expression::FunctionArg(_, _, pos) => format!("(arg #{})", pos),
            Expression::BoolLiteral(_, false) => "false".to_string(),
            Expression::BoolLiteral(_, true) => "true".to_string(),
            Expression::BytesLiteral(_, Type::String, s) => {
                format!("{}", String::from_utf8_lossy(s))
            }
            Expression::BytesLiteral(_, _, s) => format!("hex\"{}\"", hex::encode(s)),
            Expression::NumberLiteral(_, ty, n) => {
                format!("{} {}", ty.to_string(ns), n.to_str_radix(10))
            }
            Expression::StructLiteral(_, _, expr) => format!(
                "struct {{ {} }}",
                expr.iter()
                    .map(|e| self.expr_to_string(contract, ns, e))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::ConstArrayLiteral(_, _, dims, exprs) => format!(
                "constant {} [ {} ]",
                dims.iter().map(|d| format!("[{}]", d)).collect::<String>(),
                exprs
                    .iter()
                    .map(|e| self.expr_to_string(contract, ns, e))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::ArrayLiteral(_, _, dims, exprs) => format!(
                "{} [ {} ]",
                dims.iter().map(|d| format!("[{}]", d)).collect::<String>(),
                exprs
                    .iter()
                    .map(|e| self.expr_to_string(contract, ns, e))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::Add(_, _, l, r) => format!(
                "({} + {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::Subtract(_, _, l, r) => format!(
                "({} - {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::BitwiseOr(_, _, l, r) => format!(
                "({} | {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::BitwiseAnd(_, _, l, r) => format!(
                "({} & {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::BitwiseXor(_, _, l, r) => format!(
                "({} ^ {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::ShiftLeft(_, _, l, r) => format!(
                "({} << {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::ShiftRight(_, _, l, r, _) => format!(
                "({} >> {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::Multiply(_, _, l, r) => format!(
                "({} * {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::Divide(_, _, l, r) => format!(
                "({} / {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::Modulo(_, _, l, r) => format!(
                "({} % {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::Power(_, _, l, r) => format!(
                "({} ** {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::Variable(_, _, res) => format!("%{}", self.vars[res].id.name),
            Expression::ConstantVariable(_, _, Some(var_contract_no), var_no)
            | Expression::StorageVariable(_, _, var_contract_no, var_no) => format!(
                "${}.{}",
                ns.contracts[*var_contract_no].name,
                ns.contracts[*var_contract_no].variables[*var_no].name
            ),
            Expression::ConstantVariable(_, _, None, var_no) => {
                format!("${}", ns.constants[*var_no].name)
            }
            Expression::Load(_, _, expr) => {
                format!("(load {})", self.expr_to_string(contract, ns, expr))
            }
            Expression::StorageLoad(_, ty, expr) => format!(
                "(loadstorage ty:{} {})",
                ty.to_string(ns),
                self.expr_to_string(contract, ns, expr)
            ),
            Expression::ZeroExt(_, ty, e) => format!(
                "(zext {} {})",
                ty.to_string(ns),
                self.expr_to_string(contract, ns, e)
            ),
            Expression::SignExt(_, ty, e) => format!(
                "(sext {} {})",
                ty.to_string(ns),
                self.expr_to_string(contract, ns, e)
            ),
            Expression::Trunc(_, ty, e) => format!(
                "(trunc {} {})",
                ty.to_string(ns),
                self.expr_to_string(contract, ns, e)
            ),
            Expression::More(_, l, r) => format!(
                "({} > {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::Less(_, l, r) => format!(
                "({} < {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::MoreEqual(_, l, r) => format!(
                "({} >= {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::LessEqual(_, l, r) => format!(
                "({} <= {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::Equal(_, l, r) => format!(
                "({} == {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::NotEqual(_, l, r) => format!(
                "({} != {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::ArraySubscript(_, _, a, i) => format!(
                "(array index {}[{}])",
                self.expr_to_string(contract, ns, a),
                self.expr_to_string(contract, ns, i)
            ),
            Expression::DynamicArraySubscript(_, _, a, i) => format!(
                "(darray index {}[{}])",
                self.expr_to_string(contract, ns, a),
                self.expr_to_string(contract, ns, i)
            ),
            Expression::StorageBytesSubscript(_, a, i) => format!(
                "(storage bytes index {}[{}])",
                self.expr_to_string(contract, ns, a),
                self.expr_to_string(contract, ns, i)
            ),
            Expression::StorageBytesLength(_, a) => format!(
                "(storage bytes length {})",
                self.expr_to_string(contract, ns, a),
            ),
            Expression::StructMember(_, _, a, f) => format!(
                "(struct {} field {})",
                self.expr_to_string(contract, ns, a),
                f
            ),
            Expression::Or(_, l, r) => format!(
                "({} || {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::And(_, l, r) => format!(
                "({} && {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::Ternary(_, _, c, l, r) => format!(
                "({} ? {} : {})",
                self.expr_to_string(contract, ns, c),
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::Not(_, e) => format!("!{}", self.expr_to_string(contract, ns, e)),
            Expression::Complement(_, _, e) => format!("~{}", self.expr_to_string(contract, ns, e)),
            Expression::UnaryMinus(_, _, e) => format!("-{}", self.expr_to_string(contract, ns, e)),
            Expression::Poison => "☠".to_string(),
            Expression::AllocDynamicArray(_, ty, size, None) => format!(
                "(alloc {} len {})",
                ty.to_string(ns),
                self.expr_to_string(contract, ns, size)
            ),
            Expression::AllocDynamicArray(_, ty, size, Some(init)) => format!(
                "(alloc {} {} {})",
                ty.to_string(ns),
                self.expr_to_string(contract, ns, size),
                match str::from_utf8(init) {
                    Ok(s) => format!("\"{}\"", s.escape_debug()),
                    Err(_) => format!("hex\"{}\"", hex::encode(init)),
                }
            ),
            Expression::DynamicArrayLength(_, a) => {
                format!("(darray {} len)", self.expr_to_string(contract, ns, a))
            }
            Expression::StringCompare(_, l, r) => format!(
                "(strcmp ({}) ({}))",
                self.location_to_string(contract, ns, l),
                self.location_to_string(contract, ns, r)
            ),
            Expression::StringConcat(_, _, l, r) => format!(
                "(concat ({}) ({}))",
                self.location_to_string(contract, ns, l),
                self.location_to_string(contract, ns, r)
            ),
            Expression::Keccak256(_, _, exprs) => format!(
                "(keccak256 {})",
                exprs
                    .iter()
                    .map(|e| self.expr_to_string(contract, ns, &e))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::InternalFunction {
                function_no,
                signature,
                ..
            } => {
                let function_no = if let Some(signature) = signature {
                    contract.virtual_functions[signature]
                } else {
                    *function_no
                };

                ns.functions[function_no].print_name(ns)
            }
            Expression::ExternalFunction {
                address,
                function_no,
                ..
            } => format!(
                "external {} address {}",
                self.expr_to_string(contract, ns, address),
                ns.functions[*function_no].print_name(ns)
            ),
            Expression::InternalFunctionCfg(cfg_no) => {
                format!("function {}", contract.cfg[*cfg_no].name)
            }
            Expression::InternalFunctionCall { function, args, .. } => format!(
                "(call {} ({})",
                self.expr_to_string(contract, ns, function),
                args.iter()
                    .map(|a| self.expr_to_string(contract, ns, &a))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::Constructor {
                contract_no,
                constructor_no: Some(constructor_no),
                args,
                ..
            } => format!(
                "(constructor:{} ({}) ({})",
                ns.contracts[*contract_no].name,
                ns.functions[*constructor_no].signature,
                args.iter()
                    .map(|a| self.expr_to_string(contract, ns, &a))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::Constructor {
                contract_no,
                constructor_no: None,
                args,
                ..
            } => format!(
                "(constructor:{} ({})",
                ns.contracts[*contract_no].name,
                args.iter()
                    .map(|a| self.expr_to_string(contract, ns, &a))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::CodeLiteral(_, contract_no, runtime) => format!(
                "({} code contract {})",
                if *runtime {
                    "runtimeCode"
                } else {
                    "creationCode"
                },
                ns.contracts[*contract_no].name,
            ),
            Expression::ExternalFunctionCall { function, args, .. } => format!(
                "(external call {} ({})",
                self.expr_to_string(contract, ns, function),
                args.iter()
                    .map(|a| self.expr_to_string(contract, ns, &a))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::ReturnData(_) => "(external call return data)".to_string(),
            Expression::Assign(_, _, l, r) => format!(
                "{} = {}",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::PostDecrement(_, _, e) => {
                format!("{}--", self.expr_to_string(contract, ns, e),)
            }
            Expression::PostIncrement(_, _, e) => {
                format!("{}++", self.expr_to_string(contract, ns, e),)
            }
            Expression::PreDecrement(_, _, e) => {
                format!("--{}", self.expr_to_string(contract, ns, e),)
            }
            Expression::PreIncrement(_, _, e) => {
                format!("++{}", self.expr_to_string(contract, ns, e),)
            }
            Expression::Cast(_, ty, e) => format!(
                "{}({})",
                ty.to_string(ns),
                self.expr_to_string(contract, ns, e)
            ),
            Expression::Builtin(_, _, builtin, args) => format!(
                "(builtin {:?} ({}))",
                builtin,
                args.iter()
                    .map(|a| self.expr_to_string(contract, ns, &a))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::FormatString(_, args) => format!(
                "(format string {})",
                args.iter()
                    .map(|(spec, a)| format!(
                        "({} {})",
                        spec,
                        self.expr_to_string(contract, ns, &a)
                    ))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            _ => panic!("{:?}", expr),
        }
    }

    fn location_to_string(
        &self,
        contract: &Contract,
        ns: &Namespace,
        l: &StringLocation,
    ) -> String {
        match l {
            StringLocation::RunTime(e) => self.expr_to_string(contract, ns, e),
            StringLocation::CompileTime(literal) => match str::from_utf8(literal) {
                Ok(s) => format!("\"{}\"", s.to_owned()),
                Err(_) => format!("hex\"{}\"", hex::encode(literal)),
            },
        }
    }

    pub fn instr_to_string(&self, contract: &Contract, ns: &Namespace, instr: &Instr) -> String {
        match instr {
            Instr::Return { value } => format!(
                "return {}",
                value
                    .iter()
                    .map(|expr| self.expr_to_string(contract, ns, expr))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Instr::Set { res, expr, .. } => format!(
                "ty:{} %{} = {}",
                self.vars[res].ty.to_string(ns),
                self.vars[res].id.name,
                self.expr_to_string(contract, ns, expr)
            ),
            Instr::Branch { block } => format!("branch block{}", block),
            Instr::BranchCond {
                cond,
                true_block,
                false_block,
            } => format!(
                "branchcond {}, block{}, block{}",
                self.expr_to_string(contract, ns, cond),
                true_block,
                false_block,
            ),
            Instr::ClearStorage { ty, storage } => format!(
                "clear storage slot({}) ty:{}",
                self.expr_to_string(contract, ns, storage),
                ty.to_string(ns),
            ),
            Instr::SetStorage { ty, value, storage } => format!(
                "set storage slot({}) ty:{} = %{}",
                self.expr_to_string(contract, ns, storage),
                ty.to_string(ns),
                self.expr_to_string(contract, ns, value),
            ),
            Instr::SetStorageBytes {
                value,
                storage,
                offset,
            } => format!(
                "set storage slot({}) offset:{} = {}",
                self.expr_to_string(contract, ns, storage),
                self.expr_to_string(contract, ns, offset),
                self.expr_to_string(contract, ns, value),
            ),
            Instr::PushStorageBytes { storage, value } => {
                format!(
                    "push bytes slot({}) = {}",
                    self.expr_to_string(contract, ns, storage),
                    self.expr_to_string(contract, ns, value),
                )
            }
            Instr::PopStorageBytes { res, storage } => {
                format!(
                    "%{} = pop bytes slot({})",
                    self.vars[res].id.name,
                    self.expr_to_string(contract, ns, storage),
                )
            }
            Instr::PushMemory {
                res,
                ty,
                array,
                value,
            } => format!(
                "%{}, %{} = push array ty:{} value:{}",
                self.vars[res].id.name,
                self.vars[array].id.name,
                ty.to_string(ns),
                self.expr_to_string(contract, ns, value),
            ),
            Instr::PopMemory { res, ty, array } => format!(
                "%{}, %{} = pop array ty:{}",
                self.vars[res].id.name,
                self.vars[array].id.name,
                ty.to_string(ns),
            ),
            Instr::AssertFailure { expr: None } => "assert-failure".to_string(),
            Instr::AssertFailure { expr: Some(expr) } => {
                format!("assert-failure:{}", self.expr_to_string(contract, ns, expr))
            }
            Instr::Call {
                res,
                call: InternalCallTy::Static(cfg_no),
                args,
                ..
            } => format!(
                "{} = call {} {}",
                res.iter()
                    .map(|local| format!("%{}", self.vars[local].id.name))
                    .collect::<Vec<String>>()
                    .join(", "),
                contract.cfg[*cfg_no].name,
                args.iter()
                    .map(|expr| self.expr_to_string(contract, ns, expr))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Instr::Call {
                res,
                call: InternalCallTy::Dynamic(cfg),
                args,
                ..
            } => format!(
                "{} = call {} {}",
                res.iter()
                    .map(|local| format!("%{}", self.vars[local].id.name))
                    .collect::<Vec<String>>()
                    .join(", "),
                self.expr_to_string(contract, ns, cfg),
                args.iter()
                    .map(|expr| self.expr_to_string(contract, ns, expr))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Instr::ExternalCall {
                success,
                address,
                payload,
                args,
                value,
                gas,
                callty,
            } => {
                if let Expression::ExternalFunction {
                    address,
                    function_no,
                    ..
                } = payload
                {
                    format!(
                        "{} = external call::{} address:{} signature:{} value:{} gas:{} {} {}",
                        match success {
                            Some(i) => format!("%{}", self.vars[i].id.name),
                            None => "_".to_string(),
                        },
                        callty,
                        self.expr_to_string(contract, ns, address),
                        ns.functions[*function_no].signature,
                        self.expr_to_string(contract, ns, value),
                        self.expr_to_string(contract, ns, gas),
                        ns.functions[*function_no].print_name(ns),
                        args.iter()
                            .map(|expr| self.expr_to_string(contract, ns, expr))
                            .collect::<Vec<String>>()
                            .join(", ")
                    )
                } else if let Some(address) = address {
                    format!(
                        "{} = external call address:{} value:{}",
                        match success {
                            Some(i) => format!("%{}", self.vars[i].id.name),
                            None => "_".to_string(),
                        },
                        self.expr_to_string(contract, ns, address),
                        self.expr_to_string(contract, ns, value),
                    )
                } else {
                    format!(
                        "{} = external call payload:{} value:{}",
                        match success {
                            Some(i) => format!("%{}", self.vars[i].id.name),
                            None => "_".to_string(),
                        },
                        self.expr_to_string(contract, ns, payload),
                        self.expr_to_string(contract, ns, value),
                    )
                }
            }
            Instr::ValueTransfer {
                success,
                address,
                value,
            } => {
                format!(
                    "{} = value transfer address:{} value:{}",
                    match success {
                        Some(i) => format!("%{}", self.vars[i].id.name),
                        None => "_".to_string(),
                    },
                    self.expr_to_string(contract, ns, address),
                    self.expr_to_string(contract, ns, value),
                )
            }
            Instr::AbiDecode {
                res,
                tys,
                selector,
                exception_block: exception,
                data,
            } => format!(
                "{} = (abidecode:(%{}, {} {} ({}))",
                res.iter()
                    .map(|local| format!("%{}", self.vars[local].id.name))
                    .collect::<Vec<String>>()
                    .join(", "),
                self.expr_to_string(contract, ns, data),
                selector
                    .iter()
                    .map(|s| format!("selector:0x{:08x} ", s))
                    .collect::<String>(),
                exception
                    .iter()
                    .map(|block| format!("exception: block{} ", block))
                    .collect::<String>(),
                tys.iter()
                    .map(|ty| ty.ty.to_string(ns))
                    .collect::<Vec<String>>()
                    .join(", "),
            ),
            Instr::AbiEncodeVector {
                res,
                selector,
                packed,
                args,
                ..
            } => format!(
                "{} = (abiencode{}:(%{} {})",
                format!("%{}", self.vars[res].id.name),
                if *packed { "packed" } else { "" },
                match selector {
                    None => "".to_string(),
                    Some(expr) => self.expr_to_string(contract, ns, expr),
                },
                args.iter()
                    .map(|expr| self.expr_to_string(contract, ns, expr))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Instr::Store { dest, pos } => format!(
                "store {}, {}",
                self.expr_to_string(contract, ns, dest),
                self.vars[pos].id.name
            ),
            Instr::Print { expr } => format!("print {}", self.expr_to_string(contract, ns, expr)),
            Instr::Constructor {
                success,
                res,
                contract_no,
                constructor_no,
                args,
                gas,
                salt,
                value,
            } => format!(
                "%{}, {} = constructor salt:{} value:{} gas:{} {} #{:?} ({})",
                self.vars[res].id.name,
                match success {
                    Some(i) => format!("%{}", self.vars[i].id.name),
                    None => "_".to_string(),
                },
                match salt {
                    Some(salt) => self.expr_to_string(contract, ns, salt),
                    None => "".to_string(),
                },
                match value {
                    Some(value) => self.expr_to_string(contract, ns, value),
                    None => "".to_string(),
                },
                self.expr_to_string(contract, ns, gas),
                ns.contracts[*contract_no].name,
                constructor_no,
                args.iter()
                    .map(|expr| self.expr_to_string(contract, ns, expr))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Instr::Unreachable => "unreachable".to_string(),
            Instr::SelfDestruct { recipient } => format!(
                "selfdestruct {}",
                self.expr_to_string(contract, ns, recipient)
            ),
            Instr::EmitEvent {
                data,
                topics,
                event_no,
                ..
            } => format!(
                "emit event {} topics {} data {}",
                ns.events[*event_no],
                topics
                    .iter()
                    .map(|expr| self.expr_to_string(contract, ns, expr))
                    .collect::<Vec<String>>()
                    .join(", "),
                data.iter()
                    .map(|expr| self.expr_to_string(contract, ns, expr))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }

    pub fn basic_block_to_string(&self, contract: &Contract, ns: &Namespace, pos: usize) -> String {
        let mut s = format!("block{}: # {}\n", pos, self.blocks[pos].name);

        if let Some(ref phis) = self.blocks[pos].phis {
            s.push_str(&format!(
                "\t# phis: {}\n",
                phis.iter()
                    .map(|p| -> &str { &self.vars[p].id.name })
                    .collect::<Vec<&str>>()
                    .join(",")
            ));
        }

        let defs = &self.blocks[pos].defs;

        if !defs.is_empty() {
            s.push_str(&format!(
                "\t# reaching:{}\n",
                defs.iter()
                    .map(|(var_no, defs)| format!(
                        " {}:[{}]",
                        &self.vars[var_no].id.name,
                        defs.keys()
                            .map(|d| format!("{}:{}", d.block_no, d.instr_no))
                            .collect::<Vec<String>>()
                            .join(", ")
                    ))
                    .collect::<Vec<String>>()
                    .join(", ")
            ));
        }

        for ins in &self.blocks[pos].instr {
            s.push_str(&format!("\t{}\n", self.instr_to_string(contract, ns, ins)));
        }

        s
    }

    pub fn to_string(&self, contract: &Contract, ns: &Namespace) -> String {
        let mut s = String::from("");

        for i in 0..self.blocks.len() {
            s.push_str(&self.basic_block_to_string(contract, ns, i));
        }

        s
    }
}

/// Generate the CFG for a function. If function_no is None, generate the implicit default
/// constructor
pub fn generate_cfg(
    contract_no: usize,
    function_no: Option<usize>,
    cfg_no: usize,
    all_cfgs: &mut Vec<ControlFlowGraph>,
    ns: &mut Namespace,
) {
    let default_constructor = &ns.default_constructor(contract_no);

    let func = match function_no {
        Some(function_no) => &ns.functions[function_no],
        None => default_constructor,
    };

    // if the function is a fallback or receive, then don't bother with the overriden functions; they cannot be used
    if func.ty == pt::FunctionTy::Receive {
        // if there is a virtual receive function, and it's not this one, ignore it
        if let Some(receive) = ns.contracts[contract_no].virtual_functions.get("@receive") {
            if Some(*receive) != function_no {
                return;
            }
        }
    }

    if func.ty == pt::FunctionTy::Fallback {
        // if there is a virtual fallback function, and it's not this one, ignore it
        if let Some(fallback) = ns.contracts[contract_no].virtual_functions.get("@fallback") {
            if Some(*fallback) != function_no {
                return;
            }
        }
    }

    if func.ty == pt::FunctionTy::Modifier {
        return;
    }

    let mut cfg = function_cfg(contract_no, function_no, ns);

    // if the function is a modifier, generate the modifier chain
    if !func.modifiers.is_empty() {
        // only function can have modifiers
        assert_eq!(func.ty, pt::FunctionTy::Function);
        let public = cfg.public;
        let nonpayable = cfg.nonpayable;

        cfg.public = false;

        for call in func.modifiers.iter().rev() {
            let modifier_cfg_no = all_cfgs.len();

            all_cfgs.push(cfg);

            let (modifier_no, args) = resolve_modifier_call(call, &ns.contracts[contract_no]);

            let modifier = &ns.functions[modifier_no];

            cfg =
                generate_modifier_dispatch(contract_no, func, modifier, modifier_cfg_no, args, ns);
        }

        cfg.public = public;
        cfg.nonpayable = nonpayable;
        cfg.selector = func.selector();
    }

    reaching_definitions::find(&mut cfg);
    constant_folding::constant_folding(&mut cfg, ns);
    vector_to_slice::vector_to_slice(&mut cfg, ns);
    strength_reduce::strength_reduce(&mut cfg, ns);

    all_cfgs[cfg_no] = cfg;
}

/// resolve modifier call
fn resolve_modifier_call<'a>(
    call: &'a Expression,
    contract: &Contract,
) -> (usize, &'a Vec<Expression>) {
    if let Expression::InternalFunctionCall { function, args, .. } = call {
        if let Expression::InternalFunction {
            function_no,
            signature,
            ..
        } = function.as_ref()
        {
            // is it a virtual function call
            let function_no = if let Some(signature) = signature {
                contract.virtual_functions[signature]
            } else {
                *function_no
            };

            return (function_no, args);
        }
    }

    panic!("modifier should resolve to internal call");
}

/// Generate the CFG for a function. If function_no is None, generate the implicit default
/// constructor
fn function_cfg(
    contract_no: usize,
    function_no: Option<usize>,
    ns: &Namespace,
) -> ControlFlowGraph {
    let mut vartab = match function_no {
        Some(function_no) => {
            Vartable::new_with_syms(&ns.functions[function_no].symtable, ns.next_id)
        }
        None => Vartable::new(ns.next_id),
    };

    let mut loops = LoopScopes::new();
    let default_constructor = &ns.default_constructor(contract_no);

    let func = match function_no {
        Some(function_no) => &ns.functions[function_no],
        None => default_constructor,
    };

    // symbol name
    let contract_name = match func.contract_no {
        Some(contract_no) => format!("::{}", ns.contracts[contract_no].name),
        None => String::new(),
    };

    let name = match func.ty {
        pt::FunctionTy::Function => {
            format!("sol::function{}::{}", contract_name, func.llvm_symbol(ns))
        }
        pt::FunctionTy::Constructor => format!(
            "sol::constructor{}::{}",
            contract_name,
            func.llvm_symbol(ns)
        ),
        _ => format!("sol{}::{}", contract_name, func.ty),
    };

    let mut cfg = ControlFlowGraph::new(name, function_no);

    cfg.params = func.params.clone();
    cfg.returns = func.returns.clone();
    cfg.selector = func.selector();

    // a function is public if is not a library and not a base constructor
    cfg.public = if let Some(base_contract_no) = func.contract_no {
        !ns.contracts[base_contract_no].is_library()
            && !(func.is_constructor() && contract_no != base_contract_no)
            && func.is_public()
    } else {
        false
    };

    // if a function is virtual, and it is overriden, do not make it public
    // Otherwise the runtime function dispatch will have two identical functions to dispatch to
    if func.is_virtual
        && Some(ns.contracts[contract_no].virtual_functions[&func.signature]) != function_no
    {
        cfg.public = false;
    }

    cfg.ty = func.ty;
    cfg.nonpayable = if ns.target == Target::Substrate {
        !func.is_constructor() && !func.is_payable()
    } else {
        !func.is_payable()
    };

    // populate the argument variables
    for (i, arg) in func.symtable.arguments.iter().enumerate() {
        if let Some(pos) = arg {
            let var = &func.symtable.vars[pos];
            cfg.add(
                &mut vartab,
                Instr::Set {
                    loc: func.params[i].name_loc.unwrap(),
                    res: *pos,
                    expr: Expression::FunctionArg(var.id.loc, var.ty.clone(), i),
                },
            );
        }
    }

    // Hold your breath, this is the trickest part of the codegen ahead.
    // For each contract, the top-level constructor calls the base constructors. The base
    // constructors do not call their base constructors; everything is called from the top
    // level constructor. This is done because the arguments to base constructor are only
    // known the top level constructor, since the arguments can be specified elsewhere
    // on a constructor for a superior class
    if func.ty == pt::FunctionTy::Constructor && func.contract_no == Some(contract_no) {
        let mut all_base_args = HashMap::new();
        let mut diagnostics = HashSet::new();

        // Find all the resolved arguments for base contracts. These can be attached
        // to the contract, or the constructor. Contracts can have multiple constructors
        // so this needs to follow the correct constructors all the way
        collect_base_args(
            contract_no,
            function_no,
            &mut all_base_args,
            &mut diagnostics,
            ns,
        );

        // We shouldn't have problems. sema should have checked this
        assert!(diagnostics.is_empty());

        let order = visit_bases(contract_no, ns);
        let mut gen_base_args: HashMap<usize, (usize, Vec<Expression>)> = HashMap::new();

        for base_no in order.iter().rev() {
            if *base_no == contract_no {
                // we can't evaluate arguments to ourselves.
                continue;
            }

            if let Some(base_args) = all_base_args.get(base_no) {
                // There might be some temporary variables needed from the symbol table where
                // the constructor arguments were defined
                if let Some(defined_constructor_no) = base_args.defined_constructor_no {
                    let func = &ns.functions[defined_constructor_no];
                    vartab.add_symbol_table(&func.symtable);
                }

                // So we are evaluating the base arguments, from superior to inferior. The results
                // must be stored somewhere, for two reasons:
                // - The results must be stored by-value, so that variable value don't change
                //   by later base arguments (e.g. x++)
                // - The results are also arguments to the next constructor arguments, so they
                //   might be used again. Therefore we store the result in the vartable entry
                //   for the argument; this means values are passed automatically to the next
                //   constructor. We do need the symbol table for the called constructor, therefore
                //   we have the following two lines which look a bit odd at first
                let func = &ns.functions[base_args.calling_constructor_no];
                vartab.add_symbol_table(&func.symtable);

                let args: Vec<Expression> = base_args
                    .args
                    .iter()
                    .enumerate()
                    .map(|(i, a)| {
                        let expr = expression(a, &mut cfg, contract_no, ns, &mut vartab);

                        if let Some(id) = &func.symtable.arguments[i] {
                            let ty = expr.ty();
                            let loc = expr.loc();

                            cfg.add(
                                &mut vartab,
                                Instr::Set {
                                    loc: func.params[i].name_loc.unwrap_or(pt::Loc(0, 0, 0)),
                                    res: *id,
                                    expr,
                                },
                            );
                            Expression::Variable(loc, ty, *id)
                        } else {
                            Expression::Poison
                        }
                    })
                    .collect();

                gen_base_args.insert(*base_no, (base_args.calling_constructor_no, args));
            }
        }

        for base_no in order.iter() {
            if *base_no == contract_no {
                // we can't evaluate arguments to ourselves.
                continue;
            }

            if let Some((constructor_no, args)) = gen_base_args.remove(base_no) {
                let cfg_no = ns.contracts[contract_no].all_functions[&constructor_no];

                cfg.add(
                    &mut vartab,
                    Instr::Call {
                        res: Vec::new(),
                        return_tys: Vec::new(),
                        call: InternalCallTy::Static(cfg_no),
                        args,
                    },
                );
            } else if let Some(constructor_no) = ns.contracts[*base_no].no_args_constructor(ns) {
                let cfg_no = ns.contracts[contract_no].all_functions[&constructor_no];

                cfg.add(
                    &mut vartab,
                    Instr::Call {
                        res: Vec::new(),
                        return_tys: Vec::new(),
                        call: InternalCallTy::Static(cfg_no),
                        args: Vec::new(),
                    },
                );
            }
        }
    }

    // named returns should be populated
    for (i, pos) in func.symtable.returns.iter().enumerate() {
        if !func.returns[i].name.is_empty() {
            cfg.add(
                &mut vartab,
                Instr::Set {
                    loc: func.returns[i].name_loc.unwrap(),
                    res: *pos,
                    expr: func.returns[i].ty.default(ns),
                },
            );
        }
    }

    for stmt in &func.body {
        statement(
            stmt,
            func,
            &mut cfg,
            contract_no,
            ns,
            &mut vartab,
            &mut loops,
            None,
            None,
        );
    }

    cfg.vars = vartab.drain();

    // walk cfg to check for use for before initialize
    cfg
}

/// Generate the CFG for a modifier on a function
pub fn generate_modifier_dispatch(
    contract_no: usize,
    func: &Function,
    modifier: &Function,
    cfg_no: usize,
    args: &[Expression],
    ns: &Namespace,
) -> ControlFlowGraph {
    let name = format!(
        "sol::modifier::{}::{}::{}",
        &ns.contracts[contract_no].name,
        func.llvm_symbol(ns),
        modifier.llvm_symbol(ns)
    );
    let mut cfg = ControlFlowGraph::new(name, None);

    cfg.params = func.params.clone();
    cfg.returns = func.returns.clone();

    let mut vartab = Vartable::new_with_syms(&func.symtable, ns.next_id);

    vartab.add_symbol_table(&modifier.symtable);
    let mut loops = LoopScopes::new();

    // a modifier takes the same arguments as the function it is applied to. This way we can pass
    // the arguments to the function
    for (i, arg) in func.symtable.arguments.iter().enumerate() {
        if let Some(pos) = arg {
            let var = &func.symtable.vars[pos];
            cfg.add(
                &mut vartab,
                Instr::Set {
                    loc: pt::Loc(0, 0, 0),
                    res: *pos,
                    expr: Expression::FunctionArg(var.id.loc, var.ty.clone(), i),
                },
            );
        }
    }

    // now set the modifier args
    for (i, arg) in modifier.symtable.arguments.iter().enumerate() {
        if let Some(pos) = arg {
            let expr = expression(&args[i], &mut cfg, contract_no, ns, &mut vartab);
            cfg.add(
                &mut vartab,
                Instr::Set {
                    loc: pt::Loc(0, 0, 0),
                    res: *pos,
                    expr,
                },
            );
        }
    }

    // modifiers do not have return values in their syntax, but the return values from the function
    // need to be passed on. So, we need to create some var
    let mut value = Vec::new();
    let mut return_tys = Vec::new();

    for (i, arg) in func.returns.iter().enumerate() {
        value.push(Expression::Variable(
            arg.loc,
            arg.ty.clone(),
            func.symtable.returns[i],
        ));
        return_tys.push(arg.ty.clone());
    }

    let return_instr = Instr::Return { value };

    // create the instruction for the place holder
    let placeholder = Instr::Call {
        res: func.symtable.returns.clone(),
        call: InternalCallTy::Static(cfg_no),
        return_tys,
        args: func
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| Expression::FunctionArg(p.loc, p.ty.clone(), i))
            .collect(),
    };

    for stmt in &modifier.body {
        statement(
            stmt,
            func,
            &mut cfg,
            contract_no,
            ns,
            &mut vartab,
            &mut loops,
            Some(&placeholder),
            Some(&return_instr),
        );
    }

    cfg.vars = vartab.drain();

    cfg
}

#[derive(Clone)]
pub enum Storage {
    Constant(usize),
    Contract(BigInt),
    Local,
}

#[derive(Clone)]
pub struct Variable {
    pub id: pt::Identifier,
    pub ty: Type,
    pub pos: usize,
    pub storage: Storage,
}

#[derive(Default)]
pub struct Vartable {
    vars: Vars,
    next_id: usize,
    dirty: Vec<DirtyTracker>,
}

pub struct DirtyTracker {
    lim: usize,
    set: HashSet<usize>,
}

impl Vartable {
    pub fn new_with_syms(sym: &Symtable, next_id: usize) -> Self {
        let vars = sym
            .vars
            .iter()
            .map(|(no, v)| {
                (
                    *no,
                    Variable {
                        id: v.id.clone(),
                        ty: v.ty.clone(),
                        pos: v.pos,
                        storage: Storage::Local,
                    },
                )
            })
            .collect();

        Vartable {
            vars,
            dirty: Vec::new(),
            next_id,
        }
    }

    pub fn add_symbol_table(&mut self, sym: &Symtable) {
        for (no, v) in &sym.vars {
            self.vars.insert(
                *no,
                Variable {
                    id: v.id.clone(),
                    ty: v.ty.clone(),
                    pos: v.pos,
                    storage: Storage::Local,
                },
            );
        }
    }

    pub fn new(next_id: usize) -> Self {
        Vartable {
            vars: HashMap::new(),
            dirty: Vec::new(),
            next_id,
        }
    }

    pub fn add(&mut self, id: &pt::Identifier, ty: Type) -> Option<usize> {
        let pos = self.next_id;
        self.next_id += 1;

        self.vars.insert(
            pos,
            Variable {
                id: id.clone(),
                ty,
                pos,
                storage: Storage::Local,
            },
        );

        Some(pos)
    }

    pub fn temp_anonymous(&mut self, ty: &Type) -> usize {
        let pos = self.next_id;
        self.next_id += 1;

        self.vars.insert(
            pos,
            Variable {
                id: pt::Identifier {
                    name: format!("temp.{}", pos),
                    loc: pt::Loc(0, 0, 0),
                },
                ty: ty.clone(),
                pos,
                storage: Storage::Local,
            },
        );

        pos
    }

    pub fn temp(&mut self, id: &pt::Identifier, ty: &Type) -> usize {
        let pos = self.next_id;
        self.next_id += 1;

        self.vars.insert(
            pos,
            Variable {
                id: pt::Identifier {
                    name: format!("{}.temp.{}", id.name, pos),
                    loc: id.loc,
                },
                ty: ty.clone(),
                pos,
                storage: Storage::Local,
            },
        );

        pos
    }

    pub fn temp_name(&mut self, name: &str, ty: &Type) -> usize {
        let pos = self.next_id;
        self.next_id += 1;

        self.vars.insert(
            pos,
            Variable {
                id: pt::Identifier {
                    name: format!("{}.temp.{}", name, pos),
                    loc: pt::Loc(0, 0, 0),
                },
                ty: ty.clone(),
                pos,
                storage: Storage::Local,
            },
        );

        pos
    }

    pub fn drain(self) -> Vars {
        self.vars
    }

    // In order to create phi nodes, we need to track what vars are set in a certain scope
    pub fn set_dirty(&mut self, pos: usize) {
        for e in &mut self.dirty {
            if pos < e.lim {
                e.set.insert(pos);
            }
        }
    }

    pub fn new_dirty_tracker(&mut self, lim: usize) {
        self.dirty.push(DirtyTracker {
            lim,
            set: HashSet::new(),
        });
    }

    pub fn pop_dirty_tracker(&mut self) -> HashSet<usize> {
        self.dirty.pop().unwrap().set
    }
}
