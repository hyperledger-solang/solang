use num_bigint::BigInt;
use std::collections::HashSet;
use std::fmt;
use std::str;

use super::expression::expression;
use super::statements::{statement, LoopScopes};
use hex;
use parser::pt;
use sema::ast::{CallTy, Contract, Expression, Namespace, Parameter, StringLocation, Type};
use sema::symtable::Symtable;

#[allow(clippy::large_enum_variant)]
pub enum Instr {
    ClearStorage {
        ty: Type,
        storage: Expression,
    },
    SetStorage {
        ty: Type,
        local: usize,
        storage: Expression,
    },
    SetStorageBytes {
        local: usize,
        storage: Box<Expression>,
        offset: Box<Expression>,
    },
    PushMemory {
        res: usize,
        ty: Type,
        array: usize,
        value: Box<Expression>,
    },
    PopMemory {
        res: usize,
        ty: Type,
        array: usize,
    },
    Set {
        res: usize,
        expr: Expression,
    },
    Eval {
        expr: Expression,
    },
    Constant {
        res: usize,
        constant: usize,
    },
    Call {
        res: Vec<usize>,
        base: usize,
        func: usize,
        args: Vec<Expression>,
    },
    Return {
        value: Vec<Expression>,
    },
    Branch {
        bb: usize,
    },
    BranchCond {
        cond: Expression,
        true_: usize,
        false_: usize,
    },
    Store {
        dest: Expression,
        pos: usize,
    },
    AssertFailure {
        expr: Option<Expression>,
    },
    Print {
        expr: Expression,
    },
    Constructor {
        success: Option<usize>,
        res: usize,
        contract_no: usize,
        constructor_no: usize,
        args: Vec<Expression>,
        value: Option<Expression>,
        gas: Expression,
        salt: Option<Expression>,
    },
    ExternalCall {
        success: Option<usize>,
        address: Expression,
        contract_no: Option<usize>,
        function_no: usize,
        args: Vec<Expression>,
        value: Expression,
        gas: Expression,
        callty: CallTy,
    },
    AbiDecode {
        res: Vec<usize>,
        selector: Option<u32>,
        exception: Option<usize>,
        tys: Vec<Parameter>,
        data: Expression,
    },
    AbiEncodeVector {
        res: usize,
        tys: Vec<Type>,
        packed: bool,
        selector: Option<Expression>,
        args: Vec<Expression>,
    },
    Unreachable,
    SelfDestruct {
        recipient: Expression,
    },
    Hash {
        res: usize,
        hash: HashTy,
        expr: Expression,
    },
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

pub struct BasicBlock {
    pub phis: Option<HashSet<usize>>,
    pub name: String,
    pub instr: Vec<Instr>,
}

impl BasicBlock {
    fn add(&mut self, ins: Instr) {
        self.instr.push(ins);
    }
}

#[derive(Default)]
pub struct ControlFlowGraph {
    pub vars: Vec<Variable>,
    pub bb: Vec<BasicBlock>,
    current: usize,
    pub writes_contract_storage: bool,
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        let mut cfg = ControlFlowGraph {
            vars: Vec::new(),
            bb: Vec::new(),
            current: 0,
            writes_contract_storage: false,
        };

        cfg.new_basic_block("entry".to_string());

        cfg
    }

    pub fn new_basic_block(&mut self, name: String) -> usize {
        let pos = self.bb.len();

        self.bb.push(BasicBlock {
            name,
            instr: Vec::new(),
            phis: None,
        });

        pos
    }

    pub fn set_phis(&mut self, bb: usize, phis: HashSet<usize>) {
        if !phis.is_empty() {
            self.bb[bb].phis = Some(phis);
        }
    }

    pub fn set_basic_block(&mut self, pos: usize) {
        self.current = pos;
    }

    pub fn add(&mut self, vartab: &mut Vartable, ins: Instr) {
        if let Instr::Set { res, .. } = ins {
            vartab.set_dirty(res);
        }
        self.bb[self.current].add(ins);
    }

    pub fn expr_to_string(&self, contract: &Contract, ns: &Namespace, expr: &Expression) -> String {
        match expr {
            Expression::FunctionArg(_, _, pos) => format!("(arg #{})", pos),
            Expression::BoolLiteral(_, false) => "false".to_string(),
            Expression::BoolLiteral(_, true) => "true".to_string(),
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
            Expression::UDivide(_, _, l, r) | Expression::SDivide(_, _, l, r) => format!(
                "({} / {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::UModulo(_, _, l, r) | Expression::SModulo(_, _, l, r) => format!(
                "({} % {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::Power(_, _, l, r) => format!(
                "({} ** {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::Variable(_, _, res) => format!("%{}", self.vars[*res].id.name),
            Expression::ConstantVariable(_, _, var_contract_no, var_no) | Expression::StorageVariable(_, _, var_contract_no, var_no) => {
                format!("${}.{}", ns.contracts[*var_contract_no].name,
                ns.contracts[*var_contract_no].variables[*var_no].name)
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
            Expression::SMore(_, l, r) => format!(
                "({} >(s) {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::SLess(_, l, r) => format!(
                "({} <(s) {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::SMoreEqual(_, l, r) => format!(
                "({} >=(s) {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::SLessEqual(_, l, r) => format!(
                "({} <=(s) {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::UMore(_, l, r) => format!(
                "({} >(u) {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::ULess(_, l, r) => format!(
                "({} <(u) {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::UMoreEqual(_, l, r) => format!(
                "({} >=(u) {})",
                self.expr_to_string(contract, ns, l),
                self.expr_to_string(contract, ns, r)
            ),
            Expression::ULessEqual(_, l, r) => format!(
                "({} <=(u) {})",
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
            Expression::StorageBytesPush(_, a, i) => format!(
                "(storage bytes push {} {})",
                self.expr_to_string(contract, ns, a),
                self.expr_to_string(contract, ns, i)
            ),
            Expression::StorageBytesPop(_, a) => format!(
                "(storage bytes pop {})",
                self.expr_to_string(contract, ns, a),
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
            Expression::InternalFunctionCall(_, _, signature, args) => format!(
                "(call {} ({})",
                signature,
                args.iter()
                    .map(|a| self.expr_to_string(contract, ns, &a))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::Constructor {
                contract_no,
                constructor_no,
                args,
                ..
            } => format!(
                "(constructor:{} ({}) ({})",
                ns.contracts[*contract_no].name,
                ns.contracts[*contract_no].functions[*constructor_no].signature,
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
            Expression::ExternalFunctionCall {
                function_no,
                contract_no,
                address,
                args,
                ..
            } => format!(
                "(external call address:{} {}.{} ({})",
                self.expr_to_string(contract, ns, address),
                ns.contracts[*contract_no].name,
                contract.functions[*function_no].name,
                args.iter()
                    .map(|a| self.expr_to_string(contract, ns, &a))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expression::ReturnData(_) => "(external call return data)".to_string(),
            Expression::GetAddress(_, _) => "(get adddress)".to_string(),
            Expression::Balance(_, _, addr) => {
                format!("(balance {})", self.expr_to_string(contract, ns, addr))
            }
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
            Expression::Builtin(_, _, builtin, args) =>
                format!("(builtin {:?} ({}))", builtin,                
                     args.iter().map(|a| self.expr_to_string(contract, ns, &a)).collect::<Vec<String>>().join(", ")
            )
            ,
            // FIXME BEFORE MERGE
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
            Instr::Set { res, expr } => format!(
                "ty:{} %{} = {}",
                self.vars[*res].ty.to_string(ns),
                self.vars[*res].id.name,
                self.expr_to_string(contract, ns, expr)
            ),
            Instr::Eval { expr } => format!("_ = {}", self.expr_to_string(contract, ns, expr)),
            Instr::Constant { res, constant } => format!(
                "%{} = const {}",
                self.vars[*res].id.name,
                self.expr_to_string(
                    contract,
                    ns,
                    &contract.variables[*constant].initializer.as_ref().unwrap()
                )
            ),
            Instr::Branch { bb } => format!("branch bb{}", bb),
            Instr::BranchCond {
                cond,
                true_,
                false_,
            } => format!(
                "branchcond {}, bb{}, bb{}",
                self.expr_to_string(contract, ns, cond),
                true_,
                false_
            ),
            Instr::ClearStorage { ty, storage } => format!(
                "clear storage slot({}) ty:{}",
                self.expr_to_string(contract, ns, storage),
                ty.to_string(ns),
            ),
            Instr::SetStorage { ty, local, storage } => format!(
                "set storage slot({}) ty:{} = %{}",
                self.expr_to_string(contract, ns, storage),
                ty.to_string(ns),
                self.vars[*local].id.name
            ),
            Instr::SetStorageBytes {
                local,
                storage,
                offset,
            } => format!(
                "set storage slot({}) offset:{} = %{}",
                self.expr_to_string(contract, ns, storage),
                self.expr_to_string(contract, ns, offset),
                self.vars[*local].id.name
            ),
            Instr::PushMemory {
                res,
                ty,
                array,
                value,
            } => format!(
                "%{}, %{} = push array ty:{} value:{}",
                self.vars[*res].id.name,
                self.vars[*array].id.name,
                ty.to_string(ns),
                self.expr_to_string(contract, ns, value),
            ),
            Instr::PopMemory { res, ty, array } => format!(
                "%{}, %{} = pop array ty:{}",
                self.vars[*res].id.name,
                self.vars[*array].id.name,
                ty.to_string(ns),
            ),
            Instr::AssertFailure { expr: None } => "assert-failure".to_string(),
            Instr::AssertFailure { expr: Some(expr) } => {
                format!("assert-failure:{}", self.expr_to_string(contract, ns, expr))
            }
            Instr::Call {
                res,
                base,
                func,
                args,
            } => format!(
                "{} = call {} {}.{} {}",
                res.iter()
                    .map(|local| format!("%{}", self.vars[*local].id.name))
                    .collect::<Vec<String>>()
                    .join(", "),
                *func,
                ns.contracts[*base].name.to_owned(),
                contract.functions[*func].name.to_owned(),
                args.iter()
                    .map(|expr| self.expr_to_string(contract, ns, expr))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Instr::ExternalCall {
                success,
                address,
                contract_no: Some(contract_no),
                function_no,
                args,
                value,
                gas,
                callty,
            } => format!(
                "{} = external call::{} address:{} signature:{} value:{} gas:{} func:{}.{} {}",
                match success {
                    Some(i) => format!("%{}", self.vars[*i].id.name),
                    None => "_".to_string(),
                },
                callty,
                self.expr_to_string(contract, ns, address),
                ns.contracts[*contract_no].functions[*function_no].signature,
                self.expr_to_string(contract, ns, value),
                self.expr_to_string(contract, ns, gas),
                ns.contracts[*contract_no].name,
                ns.contracts[*contract_no].functions[*function_no].name,
                args.iter()
                    .map(|expr| self.expr_to_string(contract, ns, expr))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Instr::ExternalCall {
                success,
                address,
                contract_no: None,
                value,
                ..
            } => format!(
                "{} = external call address:{} value:{}",
                match success {
                    Some(i) => format!("%{}", self.vars[*i].id.name),
                    None => "_".to_string(),
                },
                self.expr_to_string(contract, ns, address),
                self.expr_to_string(contract, ns, value),
            ),
            Instr::AbiDecode {
                res,
                tys,
                selector,
                exception,
                data,
            } => format!(
                "{} = (abidecode:(%{}, {} {} ({}))",
                res.iter()
                    .map(|local| format!("%{}", self.vars[*local].id.name))
                    .collect::<Vec<String>>()
                    .join(", "),
                self.expr_to_string(contract, ns, data),
                selector
                    .iter()
                    .map(|s| format!("selector:0x{:08x} ", s))
                    .collect::<String>(),
                exception
                    .iter()
                    .map(|bb| format!("exception:bb{} ", bb))
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
                format!("%{}", self.vars[*res].id.name),
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
                self.vars[*pos].id.name
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
                "%{}, {} = constructor salt:{} value:{} gas:{} {} #{} ({})",
                self.vars[*res].id.name,
                match success {
                    Some(i) => format!("%{}", self.vars[*i].id.name),
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
            Instr::Hash { res, hash, expr } => format!(
                "%{} = hash {} {}",
                self.vars[*res].id.name,
                hash,
                self.expr_to_string(contract, ns, expr)
            ),
        }
    }

    pub fn basic_block_to_string(&self, contract: &Contract, ns: &Namespace, pos: usize) -> String {
        let mut s = format!("bb{}: # {}\n", pos, self.bb[pos].name);

        if let Some(ref phis) = self.bb[pos].phis {
            s.push_str("# phis: ");
            let mut first = true;
            for p in phis {
                if !first {
                    s.push_str(", ");
                }
                first = false;
                s.push_str(&self.vars[*p].id.name);
            }
            s.push_str("\n");
        }

        for ins in &self.bb[pos].instr {
            s.push_str(&format!("\t{}\n", self.instr_to_string(contract, ns, ins)));
        }

        s
    }

    pub fn to_string(&self, contract: &Contract, ns: &Namespace) -> String {
        let mut s = String::from("");

        for i in 0..self.bb.len() {
            s.push_str(&self.basic_block_to_string(contract, ns, i));
        }

        s
    }
}

pub fn generate_cfg(
    contract_no: usize,
    base_contract_no: usize,
    function_no: usize,
    ns: &Namespace,
) -> ControlFlowGraph {
    let mut cfg = ControlFlowGraph::new();

    let mut vartab =
        Vartable::new_with_syms(&ns.contracts[base_contract_no].functions[function_no].symtable);
    let mut loops = LoopScopes::new();

    let func = &ns.contracts[base_contract_no].functions[function_no];

    // populate the argument variables
    for (i, arg) in func.symtable.arguments.iter().enumerate() {
        if let Some(pos) = arg {
            let var = &func.symtable.vars[*pos];
            cfg.add(
                &mut vartab,
                Instr::Set {
                    res: *pos,
                    expr: Expression::FunctionArg(var.id.loc, var.ty.clone(), i),
                },
            );
        }
    }

    if func.ty == pt::FunctionTy::Constructor {
        for base in ns.contracts[base_contract_no].bases.iter().rev() {
            if let Some((constructor_no, args)) = &base.constructor {
                base_constructor_call(
                    *constructor_no,
                    args,
                    &mut cfg,
                    contract_no,
                    base.contract_no,
                    &mut vartab,
                    ns,
                );
            } else if let Some((constructor_no, args)) = func.bases.get(&base.contract_no) {
                base_constructor_call(
                    *constructor_no,
                    args,
                    &mut cfg,
                    contract_no,
                    base.contract_no,
                    &mut vartab,
                    ns,
                );
            } else if let Some(constructor_no) =
                ns.contracts[base.contract_no].no_args_constructor()
            {
                base_constructor_call(
                    constructor_no,
                    &[],
                    &mut cfg,
                    contract_no,
                    base.contract_no,
                    &mut vartab,
                    ns,
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
        );
    }

    cfg.vars = vartab.drain();

    // walk cfg to check for use for before initialize
    cfg
}

/// Generate call for base contract constructor
fn base_constructor_call(
    constructor_no: usize,
    args: &[Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    base_contract_no: usize,
    vartab: &mut Vartable,
    ns: &Namespace,
) {
    let args = args
        .iter()
        .map(|a| expression(a, cfg, contract_no, ns, vartab))
        .collect();

    // find the correction function number for this constructor_no
    // TODO: maybe constructor_no should just be function number
    let mut seen_constructors = 0;
    let mut func = 0;

    for (i, f) in ns.contracts[base_contract_no].functions.iter().enumerate() {
        if f.is_constructor() {
            if seen_constructors == constructor_no {
                func = i;
                break;
            } else {
                seen_constructors += 1;
            }
        }
    }

    cfg.add(
        vartab,
        Instr::Call {
            res: Vec::new(),
            base: base_contract_no,
            func,
            args,
        },
    );
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
    vars: Vec<Variable>,
    dirty: Vec<DirtyTracker>,
}

pub struct DirtyTracker {
    lim: usize,
    set: HashSet<usize>,
}

impl Vartable {
    pub fn new_with_syms(sym: &Symtable) -> Self {
        let vars = sym
            .vars
            .iter()
            .map(|v| Variable {
                id: v.id.clone(),
                ty: v.ty.clone(),
                pos: v.pos,
                storage: Storage::Local,
            })
            .collect();

        Vartable {
            vars,
            dirty: Vec::new(),
        }
    }

    pub fn new() -> Self {
        Vartable {
            vars: Vec::new(),
            dirty: Vec::new(),
        }
    }

    pub fn add(&mut self, id: &pt::Identifier, ty: Type) -> Option<usize> {
        let pos = self.vars.len();

        self.vars.push(Variable {
            id: id.clone(),
            ty,
            pos,
            storage: Storage::Local,
        });

        Some(pos)
    }

    pub fn temp_anonymous(&mut self, ty: &Type) -> usize {
        let pos = self.vars.len();

        self.vars.push(Variable {
            id: pt::Identifier {
                name: format!("temp.{}", pos),
                loc: pt::Loc(0, 0, 0),
            },
            ty: ty.clone(),
            pos,
            storage: Storage::Local,
        });

        pos
    }

    pub fn temp(&mut self, id: &pt::Identifier, ty: &Type) -> usize {
        let pos = self.vars.len();

        self.vars.push(Variable {
            id: pt::Identifier {
                name: format!("{}.temp.{}", id.name, pos),
                loc: id.loc,
            },
            ty: ty.clone(),
            pos,
            storage: Storage::Local,
        });

        pos
    }

    pub fn temp_name(&mut self, name: &str, ty: &Type) -> usize {
        let pos = self.vars.len();

        self.vars.push(Variable {
            id: pt::Identifier {
                name: format!("{}.temp.{}", name, pos),
                loc: pt::Loc(0, 0, 0),
            },
            ty: ty.clone(),
            pos,
            storage: Storage::Local,
        });

        pos
    }

    pub fn drain(self) -> Vec<Variable> {
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

    pub fn new_dirty_tracker(&mut self) {
        self.dirty.push(DirtyTracker {
            lim: self.vars.len(),
            set: HashSet::new(),
        });
    }

    pub fn pop_dirty_tracker(&mut self) -> HashSet<usize> {
        self.dirty.pop().unwrap().set
    }
}
