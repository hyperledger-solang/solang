use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::str;

use super::statements::{statement, LoopScopes};
use super::{
    constant_folding, dead_storage,
    expression::expression,
    reaching_definitions, strength_reduce,
    vartable::{Vars, Vartable},
    vector_to_slice, Options,
};
use crate::codegen::subexpression_elimination::common_sub_expression_elimination;
use crate::codegen::undefined_variable;
use crate::parser::pt;
use crate::parser::pt::CodeLocation;
use crate::sema::ast::{
    CallTy, Contract, Expression, Function, Namespace, Parameter, StringLocation, Type,
};
use crate::sema::contracts::{collect_base_args, visit_bases};

#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
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
    /// Load storage (this is an instruction rather than an expression
    /// so that it can be moved around by the dead storage pass
    LoadStorage {
        res: usize,
        ty: Type,
        storage: Expression,
    },
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
    /// Push an element onto an array in storage
    PushStorage {
        res: usize,
        ty: Type,
        value: Option<Expression>,
        storage: Expression,
    },
    /// Pop an element from an array in storage
    PopStorage {
        res: Option<usize>,
        ty: Type,
        storage: Expression,
    },
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
        space: Option<Expression>,
    },
    /// Call external functions. If the call fails, set the success failure
    /// or abort if this is None
    ExternalCall {
        success: Option<usize>,
        address: Option<Expression>,
        payload: Expression,
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
    /// Insert unreachable instruction after e.g. self-destruct
    Unreachable,
    /// Self destruct
    SelfDestruct { recipient: Expression },
    /// Emit event
    EmitEvent {
        event_no: usize,
        data: Vec<Expression>,
        data_tys: Vec<Type>,
        topics: Vec<Expression>,
        topic_tys: Vec<Type>,
    },
    /// Write Buffer
    WriteBuffer {
        buf: Expression,
        offset: Expression,
        value: Expression,
    },
    /// Do nothing
    Nop,
}

impl Instr {
    pub fn recurse_expressions<T>(
        &self,
        cx: &mut T,
        f: fn(expr: &Expression, ctx: &mut T) -> bool,
    ) {
        match self {
            Instr::BranchCond { cond: expr, .. }
            | Instr::Store { dest: expr, .. }
            | Instr::LoadStorage { storage: expr, .. }
            | Instr::ClearStorage { storage: expr, .. }
            | Instr::Print { expr }
            | Instr::AssertFailure { expr: Some(expr) }
            | Instr::PopStorage { storage: expr, .. }
            | Instr::AbiDecode { data: expr, .. }
            | Instr::SelfDestruct { recipient: expr }
            | Instr::Set { expr, .. } => {
                expr.recurse(cx, f);
            }

            Instr::PushMemory { value: expr, .. } => {
                expr.recurse(cx, f);
            }

            Instr::SetStorage { value, storage, .. } => {
                value.recurse(cx, f);
                storage.recurse(cx, f);
            }
            Instr::PushStorage { value, storage, .. } => {
                if let Some(value) = value {
                    value.recurse(cx, f);
                }
                storage.recurse(cx, f);
            }

            Instr::SetStorageBytes {
                value,
                storage,
                offset,
            } => {
                value.recurse(cx, f);
                storage.recurse(cx, f);
                offset.recurse(cx, f);
            }

            Instr::Return { value: exprs } | Instr::Call { args: exprs, .. } => {
                for expr in exprs {
                    expr.recurse(cx, f);
                }
            }

            Instr::Constructor {
                args,
                value,
                gas,
                salt,
                space,
                ..
            } => {
                for arg in args {
                    arg.recurse(cx, f);
                }
                if let Some(expr) = value {
                    expr.recurse(cx, f);
                }
                gas.recurse(cx, f);

                if let Some(expr) = salt {
                    expr.recurse(cx, f);
                }

                if let Some(expr) = space {
                    expr.recurse(cx, f);
                }
            }

            Instr::ExternalCall {
                address,
                payload,
                value,
                gas,
                ..
            } => {
                if let Some(expr) = address {
                    expr.recurse(cx, f);
                }
                payload.recurse(cx, f);
                value.recurse(cx, f);
                gas.recurse(cx, f);
            }

            Instr::ValueTransfer { address, value, .. } => {
                address.recurse(cx, f);
                value.recurse(cx, f);
            }

            Instr::EmitEvent { data, topics, .. } => {
                for expr in data {
                    expr.recurse(cx, f);
                }

                for expr in topics {
                    expr.recurse(cx, f);
                }
            }

            Instr::WriteBuffer { offset, value, .. } => {
                value.recurse(cx, f);
                offset.recurse(cx, f);
            }

            Instr::AssertFailure { expr: None }
            | Instr::Unreachable
            | Instr::Nop
            | Instr::Branch { .. }
            | Instr::PopMemory { .. } => {}
        }
    }
}

#[derive(Clone, Debug)]
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
    pub phis: Option<BTreeSet<usize>>,
    pub name: String,
    pub instr: Vec<Instr>,
    pub defs: reaching_definitions::VarDefs,
    pub loop_reaching_variables: HashSet<usize>,
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
            vars: IndexMap::new(),
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
            vars: IndexMap::new(),
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
            defs: IndexMap::new(),
            loop_reaching_variables: HashSet::new(),
        });

        pos
    }

    pub fn set_phis(&mut self, block: usize, phis: BTreeSet<usize>) {
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
            Expression::NumberLiteral(_, ty @ Type::Address(_), n) => {
                format!("{} {:#x}", ty.to_string(ns), n)
            }
            Expression::NumberLiteral(_, ty, n) => {
                format!("{} {}", ty.to_string(ns), n)
            }
            Expression::RationalNumberLiteral(_, ty, n) => {
                format!("{} {}", ty.to_string(ns), n)
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
            Expression::Add(_, _, _, l, r) => format!(
                "({} + {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::Subtract(_, _, _, l, r) => format!(
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
            Expression::Multiply(_, _, _, l, r) => format!(
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
            Expression::Power(_, _, _, l, r) => format!(
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
            Expression::Subscript(_, _, ty, a, i) => format!(
                "(subscript {} {}[{}])",
                ty.to_string(ns),
                self.expr_to_string(contract, ns, a),
                self.expr_to_string(contract, ns, i)
            ),
            Expression::StorageArrayLength { array, elem_ty, .. } => format!(
                "(storage array length {}[{}])",
                self.expr_to_string(contract, ns, array),
                elem_ty.to_string(ns),
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
                    .map(|e| self.expr_to_string(contract, ns, e))
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
                    .map(|a| self.expr_to_string(contract, ns, a))
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
                    .map(|a| self.expr_to_string(contract, ns, a))
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
                    .map(|a| self.expr_to_string(contract, ns, a))
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
                    .map(|a| self.expr_to_string(contract, ns, a))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::ReturnData(_) => "(external call return data)".to_string(),
            Expression::Assign(_, _, l, r) => format!(
                "{} = {}",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::PostDecrement(_, _, _, e) => {
                format!("{}--", self.expr_to_string(contract, ns, e),)
            }
            Expression::PostIncrement(_, _, _, e) => {
                format!("{}++", self.expr_to_string(contract, ns, e),)
            }
            Expression::PreDecrement(_, _, _, e) => {
                format!("--{}", self.expr_to_string(contract, ns, e),)
            }
            Expression::PreIncrement(_, _, _, e) => {
                format!("++{}", self.expr_to_string(contract, ns, e),)
            }
            Expression::Cast(_, ty, e) => format!(
                "{}({})",
                ty.to_string(ns),
                self.expr_to_string(contract, ns, e)
            ),
            Expression::BytesCast(_, ty, from, e) => format!(
                "{} from:{} ({})",
                ty.to_string(ns),
                from.to_string(ns),
                self.expr_to_string(contract, ns, e)
            ),
            Expression::Builtin(_, _, builtin, args) => format!(
                "(builtin {:?} ({}))",
                builtin,
                args.iter()
                    .map(|a| self.expr_to_string(contract, ns, a))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::FormatString(_, args) => format!(
                "(format string {})",
                args.iter()
                    .map(|(spec, a)| format!("({} {})", spec, self.expr_to_string(contract, ns, a)))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::AbiEncode { packed, args, .. } => format!(
                "(abiencode packed:{} non-packed:{})",
                packed
                    .iter()
                    .map(|expr| self.expr_to_string(contract, ns, expr))
                    .collect::<Vec<String>>()
                    .join(", "),
                args.iter()
                    .map(|expr| self.expr_to_string(contract, ns, expr))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::Undefined(_) => "undef".to_string(),
            Expression::GetRef(_, _, expr) => {
                format!("(deref {}", self.expr_to_string(contract, ns, expr))
            }
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
            Instr::LoadStorage { ty, res, storage } => format!(
                "%{} = load storage slot({}) ty:{}",
                self.vars[res].id.name,
                self.expr_to_string(contract, ns, storage),
                ty.to_string(ns),
            ),
            Instr::ClearStorage { ty, storage } => format!(
                "clear storage slot({}) ty:{}",
                self.expr_to_string(contract, ns, storage),
                ty.to_string(ns),
            ),
            Instr::SetStorage { ty, value, storage } => format!(
                "store storage slot({}) ty:{} = {}",
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
            Instr::PushStorage {
                res,
                ty,
                storage,
                value,
            } => {
                format!(
                    "%{} = push storage ty:{} slot:{} = {}",
                    self.vars[res].id.name,
                    ty.to_string(ns),
                    self.expr_to_string(contract, ns, storage),
                    if let Some(value) = value {
                        self.expr_to_string(contract, ns, value)
                    } else {
                        String::from("empty")
                    }
                )
            }
            Instr::PopStorage {
                res: Some(res),
                ty,
                storage,
            } => {
                format!(
                    "%{} = pop storage ty:{} slot({})",
                    self.vars[res].id.name,
                    ty.to_string(ns),
                    self.expr_to_string(contract, ns, storage),
                )
            }
            Instr::PopStorage {
                res: None,
                ty,
                storage,
            } => {
                format!(
                    "pop storage ty:{} slot({})",
                    ty.to_string(ns),
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
                value,
                gas,
                callty,
            } => {
                format!(
                    "{} = external call::{} address:{} payload:{} value:{} gas:{}",
                    match success {
                        Some(i) => format!("%{}", self.vars[i].id.name),
                        None => "_".to_string(),
                    },
                    callty,
                    if let Some(address) = address {
                        self.expr_to_string(contract, ns, address)
                    } else {
                        String::new()
                    },
                    self.expr_to_string(contract, ns, payload),
                    self.expr_to_string(contract, ns, value),
                    self.expr_to_string(contract, ns, gas),
                )
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
                space,
            } => format!(
                "%{}, {} = constructor salt:{} value:{} gas:{} space:{} {} #{:?} ({})",
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
                match space {
                    Some(space) => self.expr_to_string(contract, ns, space),
                    None => "".to_string(),
                },
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
            Instr::WriteBuffer { buf, offset, value } => format!(
                "writebuffer buffer:{} offset:{} value:{}",
                self.expr_to_string(contract, ns, buf),
                self.expr_to_string(contract, ns, offset),
                self.expr_to_string(contract, ns, value)
            ),
            Instr::EmitEvent {
                data,
                topics,
                event_no,
                ..
            } => format!(
                "emit event {} topics {} data {}",
                ns.events[*event_no].symbol_name(ns),
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
            Instr::Nop => String::from("nop"),
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

/// Checks whether there is a virtual fallback or receive function
fn is_there_virtual_function(
    ns: &Namespace,
    contract_no: usize,
    function_no: Option<usize>,
) -> bool {
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
                return true;
            }
        }
    }

    if func.ty == pt::FunctionTy::Fallback {
        // if there is a virtual fallback function, and it's not this one, ignore it
        if let Some(fallback) = ns.contracts[contract_no].virtual_functions.get("@fallback") {
            if Some(*fallback) != function_no {
                return true;
            }
        }
    }

    if func.ty == pt::FunctionTy::Modifier || !func.has_body {
        return true;
    }

    false
}

/// Generate the CFG for a function. If function_no is None, generate the implicit default
/// constructor
pub fn generate_cfg(
    contract_no: usize,
    function_no: Option<usize>,
    cfg_no: usize,
    all_cfgs: &mut Vec<ControlFlowGraph>,
    ns: &mut Namespace,
    opt: &Options,
) {
    if is_there_virtual_function(ns, contract_no, function_no) {
        return;
    }

    let mut cfg = function_cfg(contract_no, function_no, ns, opt);

    let default_constructor = &ns.default_constructor(contract_no);
    let func = match function_no {
        Some(function_no) => &ns.functions[function_no],
        None => default_constructor,
    };

    // if the function is a modifier, generate the modifier chain
    if !func.modifiers.is_empty() {
        // only function can have modifiers
        assert_eq!(func.ty, pt::FunctionTy::Function);
        let public = cfg.public;
        let nonpayable = cfg.nonpayable;

        cfg.public = false;

        for (chain_no, call) in func.modifiers.iter().enumerate().rev() {
            let modifier_cfg_no = all_cfgs.len();

            all_cfgs.push(cfg);

            let (modifier_no, args) = resolve_modifier_call(call, &ns.contracts[contract_no]);

            let modifier = &ns.functions[modifier_no];

            let (new_cfg, next_id) = generate_modifier_dispatch(
                contract_no,
                func,
                modifier,
                modifier_cfg_no,
                chain_no,
                args,
                ns,
                opt,
            );

            cfg = new_cfg;
            ns.next_id = next_id;
        }

        cfg.public = public;
        cfg.nonpayable = nonpayable;
        cfg.selector = func.selector();
    }

    optimize_and_check_cfg(&mut cfg, ns, function_no, opt);

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

/// Detect undefined variables and run codegen optimizer passess
pub fn optimize_and_check_cfg(
    cfg: &mut ControlFlowGraph,
    ns: &mut Namespace,
    func_no: Option<usize>,
    opt: &Options,
) {
    reaching_definitions::find(cfg);
    if let Some(function) = func_no {
        // If there are undefined variables, we raise an error and don't run optimizations
        if undefined_variable::find_undefined_variables(cfg, ns, function) {
            return;
        }
    }
    if opt.constant_folding {
        constant_folding::constant_folding(cfg, ns);
    }
    if opt.vector_to_slice {
        vector_to_slice::vector_to_slice(cfg, ns);
    }
    if opt.strength_reduce {
        strength_reduce::strength_reduce(cfg, ns);
    }
    if opt.dead_storage {
        dead_storage::dead_storage(cfg, ns);
    }

    // If the function is a default constructor, there is nothing to optimize.
    if opt.common_subexpression_elimination && func_no.is_some() {
        common_sub_expression_elimination(cfg, ns);
    }
}

/// Generate the CFG for a function. If function_no is None, generate the implicit default
/// constructor
fn function_cfg(
    contract_no: usize,
    function_no: Option<usize>,
    ns: &mut Namespace,
    opt: &Options,
) -> ControlFlowGraph {
    let mut vartab = match function_no {
        Some(function_no) => {
            Vartable::from_symbol_table(&ns.functions[function_no].symtable, ns.next_id)
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
        Some(base_contract_no) => format!(
            "{}::{}",
            ns.contracts[contract_no].name, ns.contracts[base_contract_no].name
        ),
        None => ns.contracts[contract_no].name.to_string(),
    };

    let name = match func.ty {
        pt::FunctionTy::Function => {
            format!("{}::function::{}", contract_name, func.llvm_symbol(ns))
        }
        // There can be multiple constructors on Substrate, give them an unique name
        pt::FunctionTy::Constructor => {
            format!("{}::constructor::{:08x}", contract_name, func.selector())
        }
        _ => format!("{}::{}", contract_name, func.ty),
    };

    let mut cfg = ControlFlowGraph::new(name, function_no);

    cfg.params = func.params.clone();
    cfg.returns = func.returns.clone();
    cfg.selector = func.selector();

    // a function is public if is not a library and not a base constructor
    cfg.public = if let Some(base_contract_no) = func.contract_no {
        !(ns.contracts[base_contract_no].is_library()
            || func.is_constructor() && contract_no != base_contract_no)
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
    cfg.nonpayable = if ns.target.is_substrate() {
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
                    loc: func.params[i].loc,
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
        let mut all_base_args = BTreeMap::new();
        let mut diagnostics = BTreeSet::new();

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
                        let expr =
                            expression(a, &mut cfg, contract_no, Some(func), ns, &mut vartab, opt);

                        if let Some(id) = &func.symtable.arguments[i] {
                            let ty = expr.ty();
                            let loc = expr.loc();

                            cfg.add(
                                &mut vartab,
                                Instr::Set {
                                    loc: func.params[i].loc,
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
        if let Some(name) = &func.returns[i].name {
            if let Some(expr) = func.returns[i].ty.default(ns) {
                cfg.add(
                    &mut vartab,
                    Instr::Set {
                        loc: name.loc,
                        res: *pos,
                        expr,
                    },
                );
            }
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
            opt,
        );
    }

    if func
        .body
        .last()
        .map(|stmt| stmt.reachable())
        .unwrap_or(true)
    {
        // add implicit return
        cfg.add(
            &mut vartab,
            Instr::Return {
                value: func
                    .symtable
                    .returns
                    .iter()
                    .map(|pos| {
                        Expression::Variable(
                            pt::Loc::Codegen,
                            func.symtable.vars[pos].ty.clone(),
                            *pos,
                        )
                    })
                    .collect::<Vec<_>>(),
            },
        );
    }

    let (vars, next_id) = vartab.drain();
    cfg.vars = vars;
    ns.next_id = next_id;

    // walk cfg to check for use for before initialize
    cfg
}

/// Generate the CFG for a modifier on a function
pub fn generate_modifier_dispatch(
    contract_no: usize,
    func: &Function,
    modifier: &Function,
    cfg_no: usize,
    chain_no: usize,
    args: &[Expression],
    ns: &Namespace,
    opt: &Options,
) -> (ControlFlowGraph, usize) {
    let name = format!(
        "{}::{}::{}::modifier{}::{}",
        &ns.contracts[contract_no].name,
        &ns.contracts[func.contract_no.unwrap()].name,
        func.llvm_symbol(ns),
        chain_no,
        modifier.llvm_symbol(ns)
    );
    let mut cfg = ControlFlowGraph::new(name, None);

    cfg.params = func.params.clone();
    cfg.returns = func.returns.clone();

    let mut vartab = Vartable::from_symbol_table(&func.symtable, ns.next_id);

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
                    loc: pt::Loc::Codegen,
                    res: *pos,
                    expr: Expression::FunctionArg(var.id.loc, var.ty.clone(), i),
                },
            );
        }
    }

    // now set the modifier args
    for (i, arg) in modifier.symtable.arguments.iter().enumerate() {
        if let Some(pos) = arg {
            let expr = expression(
                &args[i],
                &mut cfg,
                contract_no,
                Some(func),
                ns,
                &mut vartab,
                opt,
            );
            cfg.add(
                &mut vartab,
                Instr::Set {
                    loc: pt::Loc::Codegen,
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
            modifier,
            &mut cfg,
            contract_no,
            ns,
            &mut vartab,
            &mut loops,
            Some(&placeholder),
            Some(&return_instr),
            opt,
        );
    }

    if modifier
        .body
        .last()
        .map(|stmt| stmt.reachable())
        .unwrap_or(true)
    {
        // add implicit return
        cfg.add(
            &mut vartab,
            Instr::Return {
                value: func
                    .symtable
                    .returns
                    .iter()
                    .map(|pos| {
                        Expression::Variable(
                            pt::Loc::Codegen,
                            func.symtable.vars[pos].ty.clone(),
                            *pos,
                        )
                    })
                    .collect::<Vec<_>>(),
            },
        );
    }
    let (vars, next_id) = vartab.drain();
    cfg.vars = vars;

    (cfg, next_id)
}

impl Contract {
    /// Print the entire contract; storage initializers, constructors and functions and their CFGs
    pub fn print_cfg(&self, ns: &Namespace) -> String {
        let mut out = format!("#\n# Contract: {}\n#\n\n", self.name);

        for cfg in &self.cfg {
            if !cfg.is_placeholder() {
                out += &format!(
                    "\n# {} {} public:{} selector:{} nonpayable:{}\n",
                    cfg.ty,
                    cfg.name,
                    cfg.public,
                    hex::encode(cfg.selector.to_be_bytes()),
                    cfg.nonpayable,
                );

                out += &format!(
                    "# params: {}\n",
                    cfg.params
                        .iter()
                        .map(|p| p.ty.to_string(ns))
                        .collect::<Vec<String>>()
                        .join(",")
                );
                out += &format!(
                    "# returns: {}\n",
                    cfg.returns
                        .iter()
                        .map(|p| p.ty.to_string(ns))
                        .collect::<Vec<String>>()
                        .join(",")
                );

                out += &cfg.to_string(self, ns);
            }
        }

        out
    }

    /// Get the storage slot for a variable, possibly from base contract
    pub fn get_storage_slot(
        &self,
        var_contract_no: usize,
        var_no: usize,
        ns: &Namespace,
    ) -> Expression {
        if let Some(layout) = self
            .layout
            .iter()
            .find(|l| l.contract_no == var_contract_no && l.var_no == var_no)
        {
            Expression::NumberLiteral(pt::Loc::Codegen, ns.storage_type(), layout.slot.clone())
        } else {
            panic!("get_storage_slot called on non-storage variable");
        }
    }
}
