// SPDX-License-Identifier: Apache-2.0

mod array_boundary;
pub mod cfg;
mod constant_folding;
mod constructor;
mod dead_storage;
mod dispatch;
mod encoding;
mod events;
mod expression;
mod external_functions;
mod reaching_definitions;
mod solana_deploy;
mod statements;
mod storage;
mod strength_reduce;
pub(crate) mod subexpression_elimination;
mod tests;
mod undefined_variable;
mod unused_variable;
pub(crate) mod vartable;
mod vector_to_slice;
mod yul;

use self::{
    cfg::{optimize_and_check_cfg, ControlFlowGraph, Instr},
    expression::expression,
    vartable::Vartable,
};
use crate::sema::ast::{
    FormatArg, Function, Layout, Namespace, RetrieveType, StringLocation, Type,
};
use crate::{sema::ast, Target};
use std::cmp::Ordering;

use crate::codegen::cfg::ASTFunction;
use crate::codegen::dispatch::function_dispatch;
use crate::codegen::yul::generate_yul_function_cfg;
use crate::sema::Recurse;
use num_bigint::{BigInt, Sign};
use num_rational::BigRational;
use num_traits::{FromPrimitive, Zero};
use solang_parser::{pt, pt::CodeLocation, pt::Loc};

// The sizeof(struct account_data_header)
pub const SOLANA_FIRST_OFFSET: u64 = 16;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum OptimizationLevel {
    None = 0,
    Less = 1,
    Default = 2,
    Aggressive = 3,
}

#[cfg(feature = "llvm")]
impl From<OptimizationLevel> for inkwell::OptimizationLevel {
    fn from(level: OptimizationLevel) -> Self {
        match level {
            OptimizationLevel::None => inkwell::OptimizationLevel::None,
            OptimizationLevel::Less => inkwell::OptimizationLevel::Less,
            OptimizationLevel::Default => inkwell::OptimizationLevel::Default,
            OptimizationLevel::Aggressive => inkwell::OptimizationLevel::Aggressive,
        }
    }
}

#[cfg(feature = "llvm")]
impl From<inkwell::OptimizationLevel> for OptimizationLevel {
    fn from(level: inkwell::OptimizationLevel) -> Self {
        match level {
            inkwell::OptimizationLevel::None => OptimizationLevel::None,
            inkwell::OptimizationLevel::Less => OptimizationLevel::Less,
            inkwell::OptimizationLevel::Default => OptimizationLevel::Default,
            inkwell::OptimizationLevel::Aggressive => OptimizationLevel::Aggressive,
        }
    }
}

#[derive(Debug)]
pub struct Options {
    pub dead_storage: bool,
    pub constant_folding: bool,
    pub strength_reduce: bool,
    pub vector_to_slice: bool,
    pub math_overflow_check: bool,
    pub common_subexpression_elimination: bool,
    pub generate_debug_information: bool,
    pub opt_level: OptimizationLevel,
    pub log_api_return_codes: bool,
    pub log_runtime_errors: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            dead_storage: true,
            constant_folding: true,
            strength_reduce: true,
            vector_to_slice: true,
            math_overflow_check: false,
            common_subexpression_elimination: true,
            generate_debug_information: false,
            opt_level: OptimizationLevel::Default,
            log_api_return_codes: false,
            log_runtime_errors: false,
        }
    }
}

/// The contracts are fully resolved but they do not have any CFGs which is needed for
/// the llvm code emitter. This will also do additional code checks.
pub fn codegen(ns: &mut Namespace, opt: &Options) {
    if ns.diagnostics.any_errors() {
        return;
    }

    let mut contracts_done = Vec::new();

    contracts_done.resize(ns.contracts.len(), false);

    // codegen all the contracts; some additional errors/warnings will be detected here
    while contracts_done.iter().any(|e| !*e) {
        for contract_no in 0..ns.contracts.len() {
            if contracts_done[contract_no] {
                continue;
            }

            if !ns.contracts[contract_no].instantiable {
                contracts_done[contract_no] = true;
                continue;
            }

            // does this contract create any contract which are not done
            if ns.contracts[contract_no]
                .creates
                .iter()
                .any(|c| !contracts_done[*c])
            {
                continue;
            }

            contract(contract_no, ns, opt);

            if ns.diagnostics.any_errors() {
                return;
            }

            contracts_done[contract_no] = true;
        }
    }
}

fn contract(contract_no: usize, ns: &mut Namespace, opt: &Options) {
    if !ns.diagnostics.any_errors() && ns.contracts[contract_no].instantiable {
        layout(contract_no, ns);

        let mut cfg_no = 0;
        let mut all_cfg = Vec::new();

        external_functions::add_external_functions(contract_no, ns);

        // all the functions should have a cfg_no assigned, so we can generate call instructions to the correct function
        for (_, func_cfg) in ns.contracts[contract_no].all_functions.iter_mut() {
            *func_cfg = cfg_no;
            cfg_no += 1;
        }

        // create a cfg number for yul functions
        for yul_fn_no in &ns.contracts[contract_no].yul_functions {
            ns.yul_functions[*yul_fn_no].cfg_no = cfg_no;
            cfg_no += 1;
        }

        all_cfg.resize(cfg_no, ControlFlowGraph::placeholder());

        // clone all_functions so we can pass a mutable reference to generate_cfg
        for (function_no, cfg_no) in ns.contracts[contract_no]
            .all_functions
            .iter()
            .map(|(function_no, cfg_no)| (*function_no, *cfg_no))
            .collect::<Vec<(usize, usize)>>()
            .into_iter()
        {
            cfg::generate_cfg(
                contract_no,
                Some(function_no),
                cfg_no,
                &mut all_cfg,
                ns,
                opt,
            )
        }

        // generate the cfg for yul functions
        for yul_func_no in ns.contracts[contract_no].yul_functions.clone() {
            generate_yul_function_cfg(contract_no, yul_func_no, &mut all_cfg, ns, opt);
        }

        // Generate cfg for storage initializers
        let cfg = storage_initializer(contract_no, ns, opt);
        let pos = all_cfg.len();
        all_cfg.push(cfg);
        ns.contracts[contract_no].initializer = Some(pos);

        if !ns.contracts[contract_no].have_constructor(ns) {
            // generate the default constructor
            let func = ns.default_constructor(contract_no);
            let cfg_no = all_cfg.len();
            all_cfg.push(ControlFlowGraph::placeholder());

            cfg::generate_cfg(contract_no, None, cfg_no, &mut all_cfg, ns, opt);

            ns.contracts[contract_no].default_constructor = Some((func, cfg_no));
        }

        // TODO: This is a temporary solution. Once Substrate's dispatch moves to codegen,
        // we can remove this if-condition.
        if ns.target == Target::Solana {
            let dispatch_cfg = function_dispatch(contract_no, &all_cfg, ns, opt);
            ns.contracts[contract_no].dispatch_no = all_cfg.len();
            all_cfg.push(dispatch_cfg);
        }

        ns.contracts[contract_no].cfg = all_cfg;
    }
}

/// This function will set all contract storage initializers and should be called from the constructor
fn storage_initializer(contract_no: usize, ns: &mut Namespace, opt: &Options) -> ControlFlowGraph {
    // note the single `:` to prevent a name clash with user-declared functions
    let mut cfg = ControlFlowGraph::new(
        format!("{}:storage_initializer", ns.contracts[contract_no].name),
        ASTFunction::None,
    );
    let mut vartab = Vartable::new(ns.next_id);

    for layout in &ns.contracts[contract_no].layout {
        let var = &ns.contracts[layout.contract_no].variables[layout.var_no];

        if let Some(init) = &var.initializer {
            let storage = ns.contracts[contract_no].get_storage_slot(
                pt::Loc::Codegen,
                layout.contract_no,
                layout.var_no,
                ns,
                None,
            );

            let value = expression(init, &mut cfg, contract_no, None, ns, &mut vartab, opt);

            cfg.add(
                &mut vartab,
                Instr::SetStorage {
                    value,
                    ty: var.ty.clone(),
                    storage,
                },
            );
        }
    }

    cfg.add(&mut vartab, Instr::Return { value: Vec::new() });

    vartab.finalize(ns, &mut cfg);

    optimize_and_check_cfg(&mut cfg, ns, ASTFunction::None, opt);

    cfg
}

/// Layout the contract. We determine the layout of variables and deal with overriding variables
fn layout(contract_no: usize, ns: &mut Namespace) {
    let mut slot = if ns.target == Target::Solana {
        BigInt::from(SOLANA_FIRST_OFFSET)
    } else {
        BigInt::zero()
    };

    for base_contract_no in ns.contract_bases(contract_no) {
        for var_no in 0..ns.contracts[base_contract_no].variables.len() {
            if !ns.contracts[base_contract_no].variables[var_no].constant {
                let ty = ns.contracts[base_contract_no].variables[var_no].ty.clone();

                if ns.target == Target::Solana {
                    // elements need to be aligned on solana
                    let alignment = ty.align_of(ns);

                    let offset = slot.clone() % alignment;

                    if offset > BigInt::zero() {
                        slot += alignment - offset;
                    }
                }

                ns.contracts[contract_no].layout.push(Layout {
                    slot: slot.clone(),
                    contract_no: base_contract_no,
                    var_no,
                    ty: ty.clone(),
                });

                slot += ty.storage_slots(ns);
            }
        }
    }

    ns.contracts[contract_no].fixed_layout_size = slot;
}

trait LLVMName {
    fn llvm_symbol(&self, ns: &Namespace) -> String;
}

impl LLVMName for Function {
    /// Return a unique string for this function which is a valid llvm symbol
    fn llvm_symbol(&self, ns: &Namespace) -> String {
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expression {
    AbiEncode {
        loc: pt::Loc,
        tys: Vec<Type>,
        packed: Vec<Expression>,
        args: Vec<Expression>,
    },
    Add(pt::Loc, Type, bool, Box<Expression>, Box<Expression>),
    AllocDynamicBytes(pt::Loc, Type, Box<Expression>, Option<Vec<u8>>),
    ArrayLiteral(pt::Loc, Type, Vec<u32>, Vec<Expression>),
    BitwiseAnd(pt::Loc, Type, Box<Expression>, Box<Expression>),
    BitwiseOr(pt::Loc, Type, Box<Expression>, Box<Expression>),
    BitwiseXor(pt::Loc, Type, Box<Expression>, Box<Expression>),
    BoolLiteral(pt::Loc, bool),
    Builtin(pt::Loc, Vec<Type>, Builtin, Vec<Expression>),
    BytesCast(pt::Loc, Type, Type, Box<Expression>),
    BytesLiteral(pt::Loc, Type, Vec<u8>),
    Cast(pt::Loc, Type, Box<Expression>),
    Complement(pt::Loc, Type, Box<Expression>),
    ConstArrayLiteral(pt::Loc, Type, Vec<u32>, Vec<Expression>),
    UnsignedDivide(pt::Loc, Type, Box<Expression>, Box<Expression>),
    SignedDivide(pt::Loc, Type, Box<Expression>, Box<Expression>),
    Equal(pt::Loc, Box<Expression>, Box<Expression>),
    FormatString(pt::Loc, Vec<(FormatArg, Expression)>),
    FunctionArg(pt::Loc, Type, usize),
    GetRef(pt::Loc, Type, Box<Expression>),
    InternalFunctionCfg(usize),
    Keccak256(pt::Loc, Type, Vec<Expression>),
    List(pt::Loc, Vec<Expression>),
    Less {
        loc: pt::Loc,
        signed: bool,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    LessEqual {
        loc: pt::Loc,
        signed: bool,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Load(pt::Loc, Type, Box<Expression>),
    UnsignedModulo(pt::Loc, Type, Box<Expression>, Box<Expression>),
    SignedModulo(pt::Loc, Type, Box<Expression>, Box<Expression>),
    More {
        loc: pt::Loc,
        signed: bool,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    MoreEqual {
        loc: pt::Loc,
        signed: bool,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Multiply(pt::Loc, Type, bool, Box<Expression>, Box<Expression>),
    Not(pt::Loc, Box<Expression>),
    NotEqual(pt::Loc, Box<Expression>, Box<Expression>),
    NumberLiteral(pt::Loc, Type, BigInt),
    Poison,
    Power(pt::Loc, Type, bool, Box<Expression>, Box<Expression>),
    RationalNumberLiteral(pt::Loc, Type, BigRational),
    ReturnData(pt::Loc),
    SignExt(pt::Loc, Type, Box<Expression>),
    ShiftLeft(pt::Loc, Type, Box<Expression>, Box<Expression>),
    ShiftRight(pt::Loc, Type, Box<Expression>, Box<Expression>, bool),
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
    StructLiteral(pt::Loc, Type, Vec<Expression>),
    StructMember(pt::Loc, Type, Box<Expression>, usize),
    Subscript(pt::Loc, Type, Type, Box<Expression>, Box<Expression>),
    Subtract(pt::Loc, Type, bool, Box<Expression>, Box<Expression>),
    Trunc(pt::Loc, Type, Box<Expression>),
    Negate(pt::Loc, Type, Box<Expression>),
    Undefined(Type),
    Variable(pt::Loc, Type, usize),
    ZeroExt(pt::Loc, Type, Box<Expression>),
    AdvancePointer {
        pointer: Box<Expression>,
        bytes_offset: Box<Expression>,
    },
}

impl CodeLocation for Expression {
    fn loc(&self) -> pt::Loc {
        match self {
            Expression::AbiEncode { loc, .. }
            | Expression::StorageArrayLength { loc, .. }
            | Expression::Builtin(loc, ..)
            | Expression::Cast(loc, ..)
            | Expression::NumberLiteral(loc, ..)
            | Expression::Keccak256(loc, ..)
            | Expression::MoreEqual { loc, .. }
            | Expression::ReturnData(loc, ..)
            | Expression::Subscript(loc, ..)
            | Expression::Trunc(loc, ..)
            | Expression::Variable(loc, ..)
            | Expression::SignExt(loc, ..)
            | Expression::GetRef(loc, ..)
            | Expression::Load(loc, ..)
            | Expression::BytesLiteral(loc, ..)
            | Expression::Add(loc, ..)
            | Expression::Multiply(loc, ..)
            | Expression::Subtract(loc, ..)
            | Expression::FormatString(loc, ..)
            | Expression::LessEqual { loc, .. }
            | Expression::BoolLiteral(loc, ..)
            | Expression::UnsignedDivide(loc, ..)
            | Expression::SignedDivide(loc, ..)
            | Expression::UnsignedModulo(loc, ..)
            | Expression::SignedModulo(loc, ..)
            | Expression::Power(loc, ..)
            | Expression::BitwiseOr(loc, ..)
            | Expression::BitwiseAnd(loc, ..)
            | Expression::BitwiseXor(loc, ..)
            | Expression::Equal(loc, ..)
            | Expression::NotEqual(loc, ..)
            | Expression::Complement(loc, ..)
            | Expression::Negate(loc, ..)
            | Expression::Less { loc, .. }
            | Expression::Not(loc, ..)
            | Expression::StructLiteral(loc, ..)
            | Expression::ArrayLiteral(loc, ..)
            | Expression::ConstArrayLiteral(loc, ..)
            | Expression::StructMember(loc, ..)
            | Expression::StringCompare(loc, ..)
            | Expression::StringConcat(loc, ..)
            | Expression::FunctionArg(loc, ..)
            | Expression::List(loc, ..)
            | Expression::ShiftRight(loc, ..)
            | Expression::ShiftLeft(loc, ..)
            | Expression::RationalNumberLiteral(loc, ..)
            | Expression::AllocDynamicBytes(loc, ..)
            | Expression::BytesCast(loc, ..)
            | Expression::More { loc, .. }
            | Expression::ZeroExt(loc, ..) => *loc,

            Expression::InternalFunctionCfg(_)
            | Expression::Poison
            | Expression::Undefined(_)
            | Expression::AdvancePointer { .. } => pt::Loc::Codegen,
        }
    }
}

impl Recurse for Expression {
    type ArgType = Expression;
    fn recurse<T>(&self, cx: &mut T, f: fn(expr: &Expression, ctx: &mut T) -> bool) {
        if !f(self, cx) {
            return;
        }
        match self {
            Expression::AbiEncode { packed, args, .. } => {
                for item in packed {
                    item.recurse(cx, f);
                }

                for arg in args {
                    arg.recurse(cx, f);
                }
            }

            Expression::BitwiseAnd(_, _, left, right)
            | Expression::BitwiseOr(_, _, left, right)
            | Expression::UnsignedDivide(_, _, left, right)
            | Expression::SignedDivide(_, _, left, right)
            | Expression::Equal(_, left, right)
            | Expression::Less { left, right, .. }
            | Expression::LessEqual { left, right, .. }
            | Expression::BitwiseXor(_, _, left, right)
            | Expression::More { left, right, .. }
            | Expression::MoreEqual { left, right, .. }
            | Expression::Multiply(_, _, _, left, right)
            | Expression::NotEqual(_, left, right)
            | Expression::ShiftLeft(_, _, left, right)
            | Expression::ShiftRight(_, _, left, right, _)
            | Expression::Power(_, _, _, left, right)
            | Expression::Subscript(_, _, _, left, right)
            | Expression::Subtract(_, _, _, left, right)
            | Expression::AdvancePointer {
                pointer: left,
                bytes_offset: right,
                ..
            }
            | Expression::Add(_, _, _, left, right) => {
                left.recurse(cx, f);
                right.recurse(cx, f);
            }

            Expression::BytesCast(_, _, _, exp)
            | Expression::Cast(_, _, exp)
            | Expression::GetRef(_, _, exp)
            | Expression::Not(_, exp)
            | Expression::Trunc(_, _, exp)
            | Expression::Negate(_, _, exp)
            | Expression::ZeroExt(_, _, exp)
            | Expression::SignExt(_, _, exp)
            | Expression::Complement(_, _, exp)
            | Expression::Load(_, _, exp)
            | Expression::StorageArrayLength { array: exp, .. }
            | Expression::StructMember(_, _, exp, _)
            | Expression::AllocDynamicBytes(_, _, exp, _) => {
                exp.recurse(cx, f);
            }

            Expression::Builtin(_, _, _, vec)
            | Expression::ConstArrayLiteral(_, _, _, vec)
            | Expression::Keccak256(_, _, vec)
            | Expression::StructLiteral(_, _, vec)
            | Expression::ArrayLiteral(_, _, _, vec) => {
                for item in vec {
                    item.recurse(cx, f);
                }
            }

            Expression::FormatString(_, vec) => {
                for item in vec {
                    item.1.recurse(cx, f);
                }
            }

            Expression::StringCompare(_, left, right)
            | Expression::StringConcat(_, _, left, right) => {
                if let StringLocation::RunTime(exp) = left {
                    exp.recurse(cx, f);
                }

                if let StringLocation::RunTime(exp) = right {
                    exp.recurse(cx, f);
                }
            }

            _ => (),
        }
    }
}

impl RetrieveType for Expression {
    fn ty(&self) -> Type {
        match self {
            Expression::AbiEncode { .. } | Expression::ReturnData(_) => Type::DynamicBytes,
            Expression::Builtin(_, returns, ..) => {
                assert_eq!(returns.len(), 1);
                returns[0].clone()
            }
            Expression::Keccak256(_, ty, ..)
            | Expression::Undefined(ty)
            | Expression::Variable(_, ty, ..)
            | Expression::Trunc(_, ty, ..)
            | Expression::ZeroExt(_, ty, ..)
            | Expression::Cast(_, ty, ..)
            | Expression::SignExt(_, ty, ..)
            | Expression::GetRef(_, ty, ..)
            | Expression::Load(_, ty, ..)
            | Expression::BytesLiteral(_, ty, ..)
            | Expression::Add(_, ty, ..)
            | Expression::NumberLiteral(_, ty, ..)
            | Expression::Multiply(_, ty, ..)
            | Expression::Subtract(_, ty, ..)
            | Expression::SignedDivide(_, ty, ..)
            | Expression::UnsignedDivide(_, ty, ..)
            | Expression::SignedModulo(_, ty, ..)
            | Expression::UnsignedModulo(_, ty, ..)
            | Expression::Power(_, ty, ..)
            | Expression::BitwiseOr(_, ty, ..)
            | Expression::BitwiseAnd(_, ty, ..)
            | Expression::BitwiseXor(_, ty, ..)
            | Expression::ShiftLeft(_, ty, ..)
            | Expression::ShiftRight(_, ty, ..)
            | Expression::Complement(_, ty, ..)
            | Expression::StorageArrayLength { ty, .. }
            | Expression::Negate(_, ty, ..)
            | Expression::StructLiteral(_, ty, ..)
            | Expression::ArrayLiteral(_, ty, ..)
            | Expression::ConstArrayLiteral(_, ty, ..)
            | Expression::StructMember(_, ty, ..)
            | Expression::StringConcat(_, ty, ..)
            | Expression::FunctionArg(_, ty, ..)
            | Expression::AllocDynamicBytes(_, ty, ..)
            | Expression::BytesCast(_, ty, ..)
            | Expression::RationalNumberLiteral(_, ty, ..)
            | Expression::Subscript(_, ty, ..) => ty.clone(),

            Expression::BoolLiteral(..)
            | Expression::MoreEqual { .. }
            | Expression::More { .. }
            | Expression::Not(..)
            | Expression::NotEqual(..)
            | Expression::Less { .. }
            | Expression::Equal(..)
            | Expression::StringCompare(..)
            | Expression::LessEqual { .. } => Type::Bool,

            Expression::List(_, list) => {
                assert_eq!(list.len(), 1);

                list[0].ty()
            }

            Expression::AdvancePointer { .. } => Type::BufferPointer,
            Expression::FormatString(..) => Type::String,
            Expression::InternalFunctionCfg(_) => Type::Unreachable,
            Expression::Poison => unreachable!("Expression does not have a type"),
        }
    }
}

impl Expression {
    pub(crate) fn cast(&self, to: &Type, ns: &Namespace) -> Expression {
        let from = self.ty();

        if &from == to {
            return self.clone();
        }

        let address_bits = ns.address_length as u16 * 8;

        // When converting from literals, there is not need to trunc or extend.
        match (self, &from, to) {
            (Expression::NumberLiteral(_, _, n), p, &Type::Uint(to_len)) if p.is_primitive() => {
                return if n.sign() == Sign::Minus {
                    let mut bs = n.to_signed_bytes_le();
                    bs.resize(to_len as usize / 8, 0xff);
                    Expression::NumberLiteral(
                        self.loc(),
                        Type::Uint(to_len),
                        BigInt::from_bytes_le(Sign::Plus, &bs),
                    )
                } else {
                    Expression::NumberLiteral(self.loc(), Type::Uint(to_len), n.clone())
                }
            }
            (Expression::NumberLiteral(_, _, n), p, &Type::Int(to_len)) if p.is_primitive() => {
                return Expression::NumberLiteral(self.loc(), Type::Int(to_len), n.clone());
            }
            (Expression::NumberLiteral(_, _, n), p, &Type::Bytes(to_len)) if p.is_primitive() => {
                return Expression::NumberLiteral(self.loc(), Type::Bytes(to_len), n.clone());
            }
            (Expression::NumberLiteral(_, _, n), p, &Type::Address(payable))
                if p.is_primitive() =>
            {
                return Expression::NumberLiteral(self.loc(), Type::Address(payable), n.clone());
            }

            (Expression::BytesLiteral(_, _, bs), p, &Type::Bytes(to_len)) if p.is_primitive() => {
                let mut bs = bs.to_owned();
                bs.resize(to_len as usize, 0);
                return Expression::BytesLiteral(self.loc(), Type::Bytes(to_len), bs);
            }
            (Expression::BytesLiteral(loc, _, init), _, &Type::DynamicBytes)
            | (Expression::BytesLiteral(loc, _, init), _, &Type::String) => {
                return Expression::AllocDynamicBytes(
                    *loc,
                    to.clone(),
                    Box::new(Expression::NumberLiteral(
                        *loc,
                        Type::Uint(32),
                        BigInt::from(init.len()),
                    )),
                    Some(init.clone()),
                );
            }
            (Expression::NumberLiteral(_, _, n), _, &Type::Rational) => {
                return Expression::RationalNumberLiteral(
                    self.loc(),
                    Type::Rational,
                    BigRational::from(n.clone()),
                );
            }

            _ => (),
        }

        let from = match (&from, to) {
            (Type::Enum(enum_no), Type::Uint(_)) | (Type::Enum(enum_no), Type::Int(_)) => {
                let enum_ty = &ns.enums[*enum_no];
                let from_width = enum_ty.ty.bits(ns);
                Type::Uint(from_width)
            }

            (Type::Value, Type::Uint(_)) | (Type::Value, Type::Int(_)) => {
                let from_len = (ns.value_length as u16) * 8;
                Type::Int(from_len)
            }

            _ => from,
        };

        match (&from, to) {
            (Type::Uint(from_width), Type::Enum(enum_no))
            | (Type::Int(from_width), Type::Enum(enum_no)) => {
                let enum_ty = &ns.enums[*enum_no];
                // Not checking eval const number
                let to_width = enum_ty.ty.bits(ns);
                match from_width.cmp(&to_width) {
                    Ordering::Greater => {
                        Expression::Trunc(self.loc(), to.clone(), Box::new(self.clone()))
                    }
                    Ordering::Less => {
                        Expression::ZeroExt(self.loc(), to.clone(), Box::new(self.clone()))
                    }
                    Ordering::Equal => {
                        Expression::Cast(self.loc(), to.clone(), Box::new(self.clone()))
                    }
                }
            }

            (Type::Bytes(1), Type::Uint(8)) | (Type::Uint(8), Type::Bytes(1)) => self.clone(),

            (Type::Uint(from_len), Type::Uint(to_len))
            | (Type::Uint(from_len), Type::Int(to_len)) => match from_len.cmp(to_len) {
                Ordering::Greater => {
                    Expression::Trunc(self.loc(), to.clone(), Box::new(self.clone()))
                }
                Ordering::Less => {
                    Expression::ZeroExt(self.loc(), to.clone(), Box::new(self.clone()))
                }
                Ordering::Equal => Expression::Cast(self.loc(), to.clone(), Box::new(self.clone())),
            },

            (Type::Int(from_len), Type::Uint(to_len))
            | (Type::Int(from_len), Type::Int(to_len)) => match from_len.cmp(to_len) {
                Ordering::Greater => {
                    Expression::Trunc(self.loc(), to.clone(), Box::new(self.clone()))
                }
                Ordering::Less => {
                    Expression::SignExt(self.loc(), to.clone(), Box::new(self.clone()))
                }
                Ordering::Equal => Expression::Cast(self.loc(), to.clone(), Box::new(self.clone())),
            },

            (Type::Uint(from_len), Type::Address(_)) | (Type::Int(from_len), Type::Address(_)) => {
                let address_to_int = if from.is_signed_int() {
                    Type::Int(address_bits)
                } else {
                    Type::Uint(address_bits)
                };

                let expr = match from_len.cmp(&address_bits) {
                    Ordering::Greater => {
                        Expression::Trunc(self.loc(), address_to_int, Box::new(self.clone()))
                    }
                    Ordering::Less if from.is_signed_int() => {
                        Expression::ZeroExt(self.loc(), to.clone(), Box::new(self.clone()))
                    }
                    Ordering::Less => {
                        Expression::SignExt(self.loc(), to.clone(), Box::new(self.clone()))
                    }
                    Ordering::Equal => self.clone(),
                };

                Expression::Cast(self.loc(), to.clone(), Box::new(expr))
            }
            (Type::Address(_), Type::Uint(to_len)) | (Type::Address(_), Type::Int(to_len)) => {
                let address_to_int = if to.is_signed_int() {
                    Type::Int(address_bits)
                } else {
                    Type::Uint(address_bits)
                };

                let expr = Expression::Cast(self.loc(), address_to_int, Box::new(self.clone()));

                // now resize int to request size with sign extension etc
                match to_len.cmp(&address_bits) {
                    Ordering::Less => Expression::Trunc(self.loc(), to.clone(), Box::new(expr)),
                    Ordering::Greater if to.is_signed_int() => {
                        Expression::ZeroExt(self.loc(), to.clone(), Box::new(expr))
                    }
                    Ordering::Greater => {
                        Expression::SignExt(self.loc(), to.clone(), Box::new(expr))
                    }
                    Ordering::Equal => expr,
                }
            }
            (Type::Bytes(from_len), Type::Bytes(to_len)) => {
                if to_len > from_len {
                    let shift = (to_len - from_len) * 8;

                    Expression::ShiftLeft(
                        self.loc(),
                        to.clone(),
                        Box::new(Expression::ZeroExt(
                            self.loc(),
                            to.clone(),
                            Box::new(self.clone()),
                        )),
                        Box::new(Expression::NumberLiteral(
                            self.loc(),
                            Type::Uint(*to_len as u16 * 8),
                            BigInt::from_u8(shift).unwrap(),
                        )),
                    )
                } else {
                    let shift = (from_len - to_len) * 8;

                    Expression::Trunc(
                        self.loc(),
                        to.clone(),
                        Box::new(Expression::ShiftRight(
                            self.loc(),
                            from.clone(),
                            Box::new(self.clone()),
                            Box::new(Expression::NumberLiteral(
                                self.loc(),
                                Type::Uint(*from_len as u16 * 8),
                                BigInt::from_u8(shift).unwrap(),
                            )),
                            false,
                        )),
                    )
                }
            }
            // Conversion from rational will never happen in codegen
            (Type::Uint(_) | Type::Int(_) | Type::Value, Type::Rational) => {
                Expression::Cast(self.loc(), to.clone(), Box::new(self.clone()))
            }

            (Type::Bytes(_), Type::DynamicBytes) | (Type::DynamicBytes, Type::Bytes(_)) => {
                Expression::BytesCast(self.loc(), from.clone(), to.clone(), Box::new(self.clone()))
            }

            (Type::Bool, Type::Int(_) | Type::Uint(_)) => {
                Expression::Cast(self.loc(), to.clone(), Box::new(self.clone()))
            }

            (Type::Int(_) | Type::Uint(_), Type::Bool) => Expression::NotEqual(
                self.loc(),
                Box::new(Expression::NumberLiteral(
                    self.loc(),
                    self.ty(),
                    BigInt::zero(),
                )),
                Box::new(self.clone()),
            ),

            (Type::Bytes(n), Type::Uint(bits) | Type::Int(bits)) => {
                let num_bytes = (bits / 8) as u8;
                match n.cmp(&num_bytes) {
                    Ordering::Greater => {
                        Expression::Trunc(self.loc(), to.clone(), Box::new(self.clone()))
                    }
                    Ordering::Less => {
                        Expression::ZeroExt(self.loc(), to.clone(), Box::new(self.clone()))
                    }
                    Ordering::Equal => {
                        Expression::Cast(self.loc(), to.clone(), Box::new(self.clone()))
                    }
                }
            }

            (Type::FunctionSelector, _) => Expression::Cast(
                self.loc(),
                Type::Bytes(ns.target.selector_length()),
                self.clone().into(),
            )
            .cast(to, ns),

            (_, Type::FunctionSelector) => self.cast(&Type::Bytes(ns.target.selector_length()), ns),

            (Type::Uint(_), Type::Bytes(_))
            | (Type::Int(_), Type::Bytes(_))
            | (Type::Bytes(_), Type::Address(_))
            | (Type::Address(false), Type::Address(true))
            | (Type::Address(_), Type::Contract(_))
            | (Type::Contract(_), Type::Address(_))
            | (Type::Contract(_), Type::Contract(_))
            | (Type::Address(true), Type::Address(false))
            | (Type::Address(_), Type::Bytes(_))
            | (Type::String, Type::DynamicBytes)
            | (Type::DynamicBytes, Type::String)
            | (Type::InternalFunction { .. }, Type::InternalFunction { .. })
            | (Type::ExternalFunction { .. }, Type::ExternalFunction { .. }) => {
                Expression::Cast(self.loc(), to.clone(), Box::new(self.clone()))
            }

            _ if !from.is_contract_storage()
                && !to.is_contract_storage()
                && from.is_reference_type(ns)
                && !to.is_reference_type(ns) =>
            {
                let expr = Expression::Cast(
                    self.loc(),
                    Type::Uint(ns.target.ptr_size()),
                    self.clone().into(),
                );

                expr.cast(to, ns)
            }

            _ if !from.is_contract_storage()
                && !to.is_contract_storage()
                && !from.is_reference_type(ns)
                && to.is_reference_type(ns) =>
            {
                // cast non-pointer to pointer
                let ptr_ty = Type::Uint(ns.target.ptr_size());

                Expression::Cast(self.loc(), to.clone(), self.cast(&ptr_ty, ns).into())
            }

            _ if !from.is_contract_storage()
                && !to.is_contract_storage()
                && !from.is_reference_type(ns)
                && !to.is_reference_type(ns) =>
            {
                // cast pointer to different pointer
                Expression::Cast(self.loc(), to.clone(), self.clone().into())
            }

            _ => self.clone(),
        }
    }

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
                Expression::UnsignedDivide(loc, ty, left, right) => Expression::UnsignedDivide(
                    *loc,
                    ty.clone(),
                    Box::new(filter(left, ctx)),
                    Box::new(filter(right, ctx)),
                ),
                Expression::SignedDivide(loc, ty, left, right) => Expression::SignedDivide(
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
                Expression::More {
                    loc,
                    signed,
                    left,
                    right,
                } => Expression::More {
                    loc: *loc,
                    signed: *signed,
                    left: Box::new(filter(left, ctx)),
                    right: Box::new(filter(right, ctx)),
                },
                Expression::Less {
                    loc,
                    signed,
                    left,
                    right,
                } => Expression::Less {
                    loc: *loc,
                    signed: *signed,
                    left: Box::new(filter(left, ctx)),
                    right: Box::new(filter(right, ctx)),
                },
                Expression::MoreEqual {
                    loc,
                    signed,
                    left,
                    right,
                } => Expression::MoreEqual {
                    loc: *loc,
                    signed: *signed,
                    left: Box::new(filter(left, ctx)),
                    right: Box::new(filter(right, ctx)),
                },
                Expression::LessEqual {
                    loc,
                    signed,
                    left,
                    right,
                } => Expression::LessEqual {
                    loc: *loc,
                    signed: *signed,
                    left: Box::new(filter(left, ctx)),
                    right: Box::new(filter(right, ctx)),
                },
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
                Expression::AdvancePointer {
                    pointer,
                    bytes_offset: offset,
                } => Expression::AdvancePointer {
                    pointer: Box::new(filter(pointer, ctx)),
                    bytes_offset: Box::new(filter(offset, ctx)),
                },
                Expression::Not(loc, expr) => Expression::Not(*loc, Box::new(filter(expr, ctx))),
                Expression::Complement(loc, ty, expr) => {
                    Expression::Complement(*loc, ty.clone(), Box::new(filter(expr, ctx)))
                }
                Expression::Negate(loc, ty, expr) => {
                    Expression::Negate(*loc, ty.clone(), Box::new(filter(expr, ctx)))
                }
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
                Expression::AllocDynamicBytes(loc, ty, expr, initializer) => {
                    Expression::AllocDynamicBytes(
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
                Expression::FormatString(loc, args) => {
                    let args = args.iter().map(|(f, e)| (*f, filter(e, ctx))).collect();

                    Expression::FormatString(*loc, args)
                }
                Expression::Builtin(loc, tys, builtin, args) => {
                    let args = args.iter().map(|e| filter(e, ctx)).collect();

                    Expression::Builtin(*loc, tys.clone(), *builtin, args)
                }
                _ => self.clone(),
            },
            ctx,
        )
    }

    fn external_function_selector(&self) -> Expression {
        debug_assert!(
            matches!(self.ty(), Type::ExternalFunction { .. }),
            "This is not an external function"
        );
        let loc = self.loc();
        let struct_member = Expression::StructMember(
            loc,
            Type::Ref(Box::new(Type::FunctionSelector)),
            Box::new(self.clone()),
            0,
        );
        Expression::Load(loc, Type::FunctionSelector, Box::new(struct_member))
    }

    fn external_function_address(&self) -> Expression {
        debug_assert!(
            matches!(self.ty(), Type::ExternalFunction { .. }),
            "This is not an external function"
        );
        let loc = self.loc();
        let struct_member = Expression::StructMember(
            loc,
            Type::Ref(Box::new(Type::Address(false))),
            Box::new(self.clone()),
            1,
        );
        Expression::Load(loc, Type::Address(false), Box::new(struct_member))
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Builtin {
    Accounts,
    AddMod,
    ArrayLength,
    Balance,
    Blake2_128,
    Blake2_256,
    BlockCoinbase,
    BlockDifficulty,
    BlockHash,
    BlockNumber,
    Calldata,
    Gasleft,
    GasLimit,
    Gasprice,
    GetAddress,
    ExtCodeSize,
    MinimumBalance,
    MulMod,
    Keccak256,
    Origin,
    ReadFromBuffer,
    Ripemd160,
    Sender,
    Slot,
    ProgramId,
    Sha256,
    Signature,
    SignatureVerify,
    Timestamp,
    Value,
    WriteAddress,
    WriteInt8,
    WriteInt16LE,
    WriteInt32LE,
    WriteInt64LE,
    WriteInt128LE,
    WriteInt256LE,
    WriteUint16LE,
    WriteUint32LE,
    WriteUint64LE,
    WriteUint128LE,
    WriteUint256LE,
    WriteBytes,
}

impl From<&ast::Builtin> for Builtin {
    fn from(ast_builtin: &ast::Builtin) -> Self {
        match ast_builtin {
            ast::Builtin::Accounts => Builtin::Accounts,
            ast::Builtin::AddMod => Builtin::AddMod,
            ast::Builtin::ArrayLength => Builtin::ArrayLength,
            ast::Builtin::Balance => Builtin::Balance,
            ast::Builtin::Blake2_128 => Builtin::Blake2_128,
            ast::Builtin::Blake2_256 => Builtin::Blake2_256,
            ast::Builtin::BlockCoinbase => Builtin::BlockCoinbase,
            ast::Builtin::BlockDifficulty => Builtin::BlockDifficulty,
            ast::Builtin::BlockHash => Builtin::BlockHash,
            ast::Builtin::BlockNumber => Builtin::BlockNumber,
            ast::Builtin::Calldata => Builtin::Calldata,
            ast::Builtin::Gasleft => Builtin::Gasleft,
            ast::Builtin::GasLimit => Builtin::GasLimit,
            ast::Builtin::Gasprice => Builtin::Gasprice,
            ast::Builtin::GetAddress => Builtin::GetAddress,
            ast::Builtin::MinimumBalance => Builtin::MinimumBalance,
            ast::Builtin::MulMod => Builtin::MulMod,
            ast::Builtin::Keccak256 => Builtin::Keccak256,
            ast::Builtin::Origin => Builtin::Origin,
            ast::Builtin::ReadAddress
            | ast::Builtin::ReadInt8
            | ast::Builtin::ReadInt16LE
            | ast::Builtin::ReadInt32LE
            | ast::Builtin::ReadInt64LE
            | ast::Builtin::ReadInt128LE
            | ast::Builtin::ReadInt256LE
            | ast::Builtin::ReadUint16LE
            | ast::Builtin::ReadUint32LE
            | ast::Builtin::ReadUint64LE
            | ast::Builtin::ReadUint128LE
            | ast::Builtin::ReadUint256LE => Builtin::ReadFromBuffer,
            ast::Builtin::Ripemd160 => Builtin::Ripemd160,
            ast::Builtin::Sender => Builtin::Sender,
            ast::Builtin::Slot => Builtin::Slot,
            ast::Builtin::ProgramId => Builtin::ProgramId,
            ast::Builtin::Sha256 => Builtin::Sha256,
            ast::Builtin::Signature => Builtin::Signature,
            ast::Builtin::SignatureVerify => Builtin::SignatureVerify,
            ast::Builtin::Timestamp => Builtin::Timestamp,
            ast::Builtin::Value => Builtin::Value,
            ast::Builtin::WriteAddress => Builtin::WriteAddress,
            ast::Builtin::WriteInt8 => Builtin::WriteInt8,
            ast::Builtin::WriteInt16LE => Builtin::WriteInt16LE,
            ast::Builtin::WriteInt32LE => Builtin::WriteInt32LE,
            ast::Builtin::WriteInt64LE => Builtin::WriteInt64LE,
            ast::Builtin::WriteInt128LE => Builtin::WriteInt128LE,
            ast::Builtin::WriteInt256LE => Builtin::WriteInt256LE,
            ast::Builtin::WriteUint16LE => Builtin::WriteUint16LE,
            ast::Builtin::WriteUint32LE => Builtin::WriteUint32LE,
            ast::Builtin::WriteUint64LE => Builtin::WriteUint64LE,
            ast::Builtin::WriteUint128LE => Builtin::WriteUint128LE,
            ast::Builtin::WriteUint256LE => Builtin::WriteUint256LE,
            ast::Builtin::WriteBytes | ast::Builtin::WriteString => Builtin::WriteBytes,
            _ => panic!("Builtin should not be in the cfg"),
        }
    }
}

pub(super) fn error_msg_with_loc(ns: &Namespace, error: &str, loc: Option<Loc>) -> String {
    if let Some(loc) = loc {
        match loc {
            Loc::File(..) => {
                let file_no = loc.file_no();
                let curr_file = &ns.files[file_no];
                let (line_no, offset) = curr_file.offset_to_line_column(loc.start());
                format!(
                    "{} in {}:{}:{}",
                    error,
                    curr_file.path.file_name().unwrap().to_str().unwrap(),
                    line_no + 1,
                    offset
                )
            }
            _ => error.to_string(),
        }
    } else {
        error.to_string()
    }
}
