
use parser::ast;
use resolver;
use resolver::cfg;
use std::str;
use std::path::Path;

use std::collections::HashMap;
use std::collections::VecDeque;

use inkwell::types::BasicTypeEnum;
use inkwell::OptimizationLevel;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Module, Linkage};
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::targets::{CodeModel, RelocMode, FileType, Target};
use inkwell::AddressSpace;
use inkwell::types::{IntType, StringRadix};
use inkwell::values::{PointerValue, IntValue, PhiValue, FunctionValue, BasicValueEnum, GlobalValue};
use inkwell::IntPredicate;

const WASMTRIPLE: &str = "wasm32-unknown-unknown-wasm";

mod burrow;
mod substrate;

lazy_static::lazy_static! {
    static ref LLVM_INIT: () = {
        Target::initialize_webassembly(&Default::default());
    };
}

#[derive(Clone)]
struct Variable<'a> {
    value: BasicValueEnum<'a>,
    stack: bool,
}

pub trait TargetRuntime {
    // Access storage
    fn set_storage<'a>(&self, contract: &'a Contract, function: FunctionValue, slot: u32, dest: inkwell::values::PointerValue<'a>);
    fn get_storage<'a>(&self, contract: &'a Contract, function: FunctionValue, slot: u32, dest: inkwell::values::PointerValue<'a>);

    fn abi_decode<'b>(
        &self,
        contract: &'b Contract,
        function: FunctionValue,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue,
        spec: &resolver::FunctionDecl,
    );
    fn abi_encode<'b>(
        &self,
        contract: &'b Contract,
        args: &[BasicValueEnum<'b>],
        spec: &resolver::FunctionDecl,
    ) -> (PointerValue<'b>, IntValue<'b>);

    /// Return success without any result
    fn return_empty_abi(&self, contract: &Contract);

    /// Return success with the ABI encoded result
    fn return_abi<'b>(&self, contract: &'b Contract, data: PointerValue<'b>, length: IntValue);

    /// Return failure without any result
    fn assert_failure<'b>(&self, contract: &'b Contract);
}

pub struct Contract<'a> {
    pub name: String,
    pub module: Module<'a>,
    builder: Builder<'a>,
    context: &'a Context,
    target: Target,
    ns: &'a resolver::Contract,
    constructors: Vec<FunctionValue<'a>>,
    functions: Vec<FunctionValue<'a>>,
    globals: Vec<GlobalValue<'a>>,
}

impl<'a> Contract<'a> {
    pub fn build(context: &'a Context, contract: &'a resolver::Contract, filename: &'a str) -> Self {
        match contract.target {
            super::Target::Burrow => burrow::BurrowTarget::build(context, contract, filename),
            super::Target::Substrate => substrate::SubstrateTarget::build(context, contract, filename),
        }
    }

    pub fn wasm(&self, opt: &str) -> Result<Vec<u8>, String> {
        let opt = match opt {
            "none" => OptimizationLevel::None,
            "less" => OptimizationLevel::Less,
            "default" => OptimizationLevel::Default,
            "aggressive" => OptimizationLevel::Aggressive,
            _ => unreachable!()
        };

        let target_machine = self.target.create_target_machine(WASMTRIPLE, "", "", opt, RelocMode::Default, CodeModel::Default).unwrap();

        match target_machine.write_to_memory_buffer(&self.module, FileType::Object) {
            Ok(o) => Ok(o.as_slice().to_vec()),
            Err(s) => Err(s.to_string())
        }
    }

    pub fn bitcode(&self, path: &Path) {
        self.module.write_bitcode_to_path(path);
    }

    pub fn dump_llvm(&self, path: &Path) -> Result<(), String> {
        if let Err(s) = self.module.print_to_file(path) {
            return Err(s.to_string());
        }

        Ok(())
    }

    pub fn new(context: &'a Context, contract: &'a resolver::Contract, filename: &'a str) -> Self {
        lazy_static::initialize(&LLVM_INIT);

        let target = Target::from_triple(WASMTRIPLE).unwrap();
        let module = context.create_module(&contract.name);

        module.set_target(&target);
        module.set_source_file_name(filename);

        // stdlib
        let intr = load_stdlib(&context);
        module.link_in_module(intr).unwrap();

        Contract {
            name: contract.name.to_owned(),
            module: module,
            builder: context.create_builder(),
            target: target,
            context: context,
            ns: contract,
            constructors: Vec::new(),
            functions: Vec::new(),
            globals: Vec::new(),
        }
    }

    /// Creates global string in the llvm module with initializer
    ///
    fn emit_global_string(&mut self, name: &str, data: &[u8], constant: bool) -> usize {
        let ty = self.context.i8_type().array_type(data.len() as u32);

        let gv = self.module.add_global(ty, Some(AddressSpace::Generic), name);

        gv.set_linkage(Linkage::Internal);

        gv.set_initializer(&self.context.const_string(data, false));

        if constant {
            gv.set_constant(true);
        }

        let last = self.globals.len();

        self.globals.push(gv);

        last
    }

    fn emit_functions(&mut self, runtime: &dyn TargetRuntime) {
        self.constructors = self.ns.constructors.iter()
            .map(|func| self.emit_func(&format!("sol::constructor::{}", func.wasm_symbol(&self.ns)), func, runtime))
            .collect();

        for func in &self.ns.functions {
            let name = if func.name != "" {
                format!("sol::function::{}", func.wasm_symbol(&self.ns))
            } else {
                "sol::fallback".to_owned()
            };

            let f = self.emit_func(&name, func, runtime);
            self.functions.push(f);
        }
    }

    fn expression(
        &self,
        e: &cfg::Expression,
        vartab: &Vec<Variable<'a>>,
    ) -> IntValue<'a> {
        match e {
            cfg::Expression::BoolLiteral(val) => {
                self.context.bool_type().const_int(*val as u64, false)
            }
            cfg::Expression::NumberLiteral(bits, n) => {
                let ty = self.context.custom_width_int_type(*bits as _);
                let s = n.to_string();

                ty.const_int_from_string(&s, StringRadix::Decimal).unwrap()
            }
            cfg::Expression::Add(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_add(left, right, "")
            }
            cfg::Expression::Subtract(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_sub(left, right, "")
            }
            cfg::Expression::Multiply(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_mul(left, right, "")
            }
            cfg::Expression::UDivide(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_unsigned_div(left, right, "")
            }
            cfg::Expression::SDivide(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_signed_div(left, right, "")
            }
            cfg::Expression::SModulo(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_signed_rem(left, right, "")
            }
            cfg::Expression::UModulo(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_unsigned_rem(left, right, "")
            }
            cfg::Expression::Equal(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_compare(IntPredicate::EQ, left, right, "")
            }
            cfg::Expression::NotEqual(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_compare(IntPredicate::NE, left, right, "")
            }
            cfg::Expression::SMore(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_compare(IntPredicate::SGT, left, right, "")
            }
            cfg::Expression::SMoreEqual(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_compare(IntPredicate::SGE, left, right, "")
            }
            cfg::Expression::SLess(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_compare(IntPredicate::SLT, left, right, "")
            }
            cfg::Expression::SLessEqual(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_compare(IntPredicate::SLE, left, right, "")
            }
            cfg::Expression::UMore(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_compare(IntPredicate::UGT, left, right, "")
            }
            cfg::Expression::UMoreEqual(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_compare(IntPredicate::UGE, left, right, "")
            }
            cfg::Expression::ULess(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_compare(IntPredicate::ULT, left, right, "")
            }
            cfg::Expression::ULessEqual(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_int_compare(IntPredicate::ULE, left, right, "")
            }
            cfg::Expression::Variable(_, s) => {
                if vartab[*s].stack {
                    self.builder.build_load(vartab[*s].value.into_pointer_value(), "").into_int_value()
                } else {
                    vartab[*s].value.into_int_value()
                }
            }
            cfg::Expression::ZeroExt(t, e) => {
                let e = self.expression(e, vartab);
                let ty = t.LLVMType(self.ns, &self.context);

                self.builder.build_int_z_extend(e, ty, "")
            }
            cfg::Expression::SignExt(t, e) => {
                let e = self.expression(e, vartab);
                let ty = t.LLVMType(self.ns, &self.context);

                self.builder.build_int_s_extend(e, ty, "")
            }
            cfg::Expression::Trunc(t, e) => {
                let e = self.expression(e, vartab);
                let ty = t.LLVMType(self.ns, &self.context);

                self.builder.build_int_truncate(e, ty, "")
            }
            cfg::Expression::Not(e) => {
                let e = self.expression(e, vartab);

                self.builder.build_int_compare(IntPredicate::EQ, e, e.get_type().const_zero(), "")
            }
            cfg::Expression::Or(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_or(left, right, "")
            }
            cfg::Expression::And(l, r) => {
                let left = self.expression(l, vartab);
                let right = self.expression(r, vartab);

                self.builder.build_and(left, right, "")
            }
            _ => {
                panic!("expression not implemented {:?}", e);
            }
        }
    }

    fn emit_initializer(&self, runtime: &dyn TargetRuntime) -> FunctionValue<'a> {
        let function = self.module.add_function("storage_initializers",
            self.context.void_type().fn_type(&[], false), Some(Linkage::Internal));

        self.emit_cfg(&self.ns.initializer, None, function, runtime);

        function
    }

    fn emit_func(&self, fname: &str, f: &resolver::FunctionDecl, runtime: &dyn TargetRuntime) -> FunctionValue<'a> {
        let mut args: Vec<BasicTypeEnum> = Vec::new();

        for p in &f.params {
            let ty = p.ty.LLVMType(self.ns, self.context);
            args.push(if p.ty.stack_based() {
                ty.ptr_type(AddressSpace::Generic).into()
            } else {
                ty.into()
            });
        }

        let ftype = if f.wasm_return {
            f.returns[0].ty.LLVMType(self.ns, &self.context).fn_type(&args, false)
        } else {
            // add return values
            for p in &f.returns {
                args.push(p.ty.LLVMType(self.ns, &self.context).ptr_type(AddressSpace::Generic).into());
            }
            self.context.void_type().fn_type(&args, false)
        };

        let function = self.module.add_function(&fname, ftype, Some(Linkage::Internal));

        let cfg = match f.cfg {
            Some(ref cfg) => cfg,
            None => panic!(),
        };

        self.emit_cfg(cfg, Some(f), function, runtime);

        function
    }

    fn emit_cfg(&self, cfg: &cfg::ControlFlowGraph, resolver_function: Option<&resolver::FunctionDecl>, function: FunctionValue<'a>, runtime: &dyn TargetRuntime) {
        // recurse through basic blocks
        struct BasicBlock<'a> {
            bb: inkwell::basic_block::BasicBlock,
            phis: HashMap<usize, PhiValue<'a>>,
        }

        struct Work<'b> {
            bb_no: usize,
            vars: Vec<Variable<'b>>,
        }

        let mut blocks: HashMap<usize, BasicBlock> = HashMap::new();

        let create_bb = |bb_no| -> BasicBlock {
            let cfg_bb: &cfg::BasicBlock = &cfg.bb[bb_no];
            let mut phis = HashMap::new();

            let bb = self.context.append_basic_block(function, &cfg_bb.name);

            self.builder.position_at_end(&bb);

            if let Some(ref cfg_phis) = cfg_bb.phis {
                for v in cfg_phis {
                    // FIXME: no phis needed for stack based vars
                    let ty = cfg.vars[*v].ty.LLVMType(self.ns, &self.context);

                    phis.insert(*v, self.builder.build_phi(ty, &cfg.vars[*v].id.name).into());
                }
            }

            BasicBlock { bb, phis }
        };

        let mut work = VecDeque::new();

        blocks.insert(0, create_bb(0));

        // Create all the stack variables
        let mut vars = Vec::new();

        for v in &cfg.vars {
            match v.storage {
                cfg::Storage::Local if !v.ty.stack_based() => {
                    vars.push(Variable {
                        value: self.context.i32_type().const_zero().into(),
                        stack: false,
                    });
                }
                cfg::Storage::Local | cfg::Storage::Contract(_) | cfg::Storage::Constant(_) => {
                    vars.push(Variable {
                        value: self.builder.build_alloca(
                            v.ty.LLVMType(self.ns, &self.context), &v.id.name).into(),
                        stack: true,
                    });
                }
            }
        }

        work.push_back(Work {
            bb_no: 0,
            vars: vars,
        });

        loop {
            let mut w = match work.pop_front() {
                Some(w) => w,
                None => break,
            };

            // ensure reference to blocks is short-lived
            let ll_bb = {
                let bb = blocks.get(&w.bb_no).unwrap();

                self.builder.position_at_end(&bb.bb);

                for (v, phi) in bb.phis.iter() {
                    w.vars[*v].value = (*phi).as_basic_value();
                }

                bb.bb
            };

            for ins in &cfg.bb[w.bb_no].instr {
                match ins {
                    cfg::Instr::FuncArg { res, arg } => {
                        w.vars[*res].value = function.get_nth_param(*arg as u32).unwrap().into();
                    }
                    cfg::Instr::Return { value } if value.is_empty() => {
                        self.builder.build_return(None);
                    },
                    cfg::Instr::Return { value } if resolver_function.unwrap().wasm_return => {
                        let retval = self.expression(&value[0], &w.vars);
                        self.builder.build_return(Some(&retval));
                    }
                    cfg::Instr::Return { value } => {
                        let returns_offset = resolver_function.unwrap().params.len();
                        for (i, val) in value.iter().enumerate() {
                            let arg = function.get_nth_param((returns_offset + i) as u32).unwrap();
                            let retval = self.expression(val, &w.vars);

                            self.builder.build_store(arg.into_pointer_value(), retval);
                        }
                        self.builder.build_return(None);
                    }
                    cfg::Instr::Set { res, expr } => {
                        let value_ref = self.expression(expr, &w.vars);
                        if w.vars[*res].stack {
                            self.builder.build_store(w.vars[*res].value.into_pointer_value(), value_ref);
                        } else {
                            w.vars[*res].value = value_ref.into();
                        }
                    }
                    cfg::Instr::Constant { res, constant } => {
                        let const_expr = &self.ns.constants[*constant];
                        let value_ref = self.expression(const_expr, &w.vars);
                        if w.vars[*res].stack {
                            self.builder.build_store(w.vars[*res].value.into_pointer_value(), value_ref);
                        } else {
                            w.vars[*res].value = value_ref.into();
                        }
                    }
                    cfg::Instr::Branch { bb: dest } => {
                        if !blocks.contains_key(&dest) {
                            blocks.insert(*dest, create_bb(*dest));
                            work.push_back(Work {
                                bb_no: *dest,
                                vars: w.vars.clone(),
                            });
                        }

                        let bb = blocks.get(dest).unwrap();

                        for (v, phi) in bb.phis.iter() {
                            phi.add_incoming(&[ (&w.vars[*v].value, &ll_bb) ]);
                        }

                        self.builder.position_at_end(&ll_bb);
                        self.builder.build_unconditional_branch(&bb.bb);
                    }
                    cfg::Instr::BranchCond {
                        cond,
                        true_,
                        false_,
                    } => {
                        let cond = self.expression(cond, &w.vars);

                        let bb_true = {
                            if !blocks.contains_key(&true_) {
                                blocks.insert(*true_, create_bb(*true_));
                                work.push_back(Work {
                                    bb_no: *true_,
                                    vars: w.vars.clone(),
                                });
                            }

                            let bb = blocks.get(true_).unwrap();

                            for (v, phi) in bb.phis.iter() {
                                phi.add_incoming(&[ (&w.vars[*v].value, &ll_bb) ]);
                            }

                            bb.bb
                        };

                        let bb_false = {
                            if !blocks.contains_key(&false_) {
                                blocks.insert(*false_, create_bb(*false_));
                                work.push_back(Work {
                                    bb_no: *false_,
                                    vars: w.vars.clone(),
                                });
                            }

                            let bb = blocks.get(false_).unwrap();

                            for (v, phi) in bb.phis.iter() {
                                phi.add_incoming(&[ (&w.vars[*v].value, &ll_bb) ]);
                            }

                            bb.bb
                        };

                        self.builder.position_at_end(&ll_bb);
                        self.builder.build_conditional_branch(cond, &bb_true, &bb_false);
                    }
                    cfg::Instr::GetStorage { local, storage } => {
                        let dest = w.vars[*local].value.into_pointer_value();

                        runtime.get_storage(&self, function, *storage as u32, dest);
                    }
                    cfg::Instr::SetStorage { local, storage } => {
                        let dest = w.vars[*local].value.into_pointer_value();

                        runtime.set_storage(&self, function, *storage as u32, dest);
                    }
                    cfg::Instr::AssertFailure {} => {
                        runtime.assert_failure(self);
                    }
                    cfg::Instr::Call { res, func, args } => {
                        let mut parms: Vec<BasicValueEnum> = Vec::new();

                        for a in args {
                            parms.push(self.expression(&a, &w.vars).into());
                        }

                        let ret = self.builder.build_call(
                            self.functions[*func],
                            &parms, "").try_as_basic_value().left();

                        if res.len() > 0 {
                            w.vars[res[0]].value = ret.unwrap().into();
                        }
                    }
                }
            }
        }
    }

    pub fn emit_function_dispatch(&self,
                resolver_functions: &Vec<resolver::FunctionDecl>,
                functions: &Vec<FunctionValue>,
                argsdata: inkwell::values::PointerValue,
                argslen: inkwell::values::IntValue,
                function: inkwell::values::FunctionValue,
                fallback_block: &inkwell::basic_block::BasicBlock,
                runtime: &dyn TargetRuntime) {
        // create start function
        let switch_block = self.context.append_basic_block(function, "switch");

        let not_fallback = self.builder.build_int_compare(
            IntPredicate::UGE,
            argslen,
            self.context.i32_type().const_int(4, false).into(),
            "");

        self.builder.build_conditional_branch(not_fallback, &switch_block, fallback_block);

        self.builder.position_at_end(&switch_block);

        let fid = self.builder.build_load(argsdata, "function_selector");

        // step over the function selector
        let argsdata = unsafe {
            self.builder.build_gep(
                argsdata,
                &[self.context.i32_type().const_int(1, false).into()],
                "argsdata")
        };

        let argslen = self.builder.build_int_sub(
            argslen.into(),
            self.context.i32_type().const_int(4, false).into(),
            "argslen"
        );

        let nomatch = self.context.append_basic_block(function, "nomatch");

        self.builder.position_at_end(&nomatch);

        self.builder.build_unreachable();

        let mut cases = Vec::new();

        for (i, f) in resolver_functions.iter().enumerate() {
            match &f.visibility {
                ast::Visibility::Internal(_) | ast::Visibility::Private(_) => {
                    continue;
                }
                _ => (),
            }

            if f.fallback {
                continue;
            }

            let bb = self.context.append_basic_block(function, "");

            let id = f.selector();

            self.builder.position_at_end(&bb);

            let mut args = Vec::new();

            // insert abi decode
            runtime.abi_decode(&self, function, &mut args, argsdata, argslen, f);

            let ret = self.builder.build_call(
                functions[i],
                &args,
                "").try_as_basic_value().left();

            if f.returns.is_empty() {
                // return ABI of length 0
                runtime.return_empty_abi(&self);
            } else if f.wasm_return {
                let (data, length) = runtime.abi_encode(&self, &[ ret.unwrap() ], &f);

                runtime.return_abi(&self, data, length);
            } else {
                // FIXME: abi encode all the arguments
                unimplemented!();
            }

            cases.push((self.context.i32_type().const_int(id as u64, false), bb));
        }

        self.builder.position_at_end(&switch_block);

        let mut c = Vec::new();

        for (id, bb) in cases.iter() {
            c.push((*id, bb));
        }

        //let c = cases.into_iter().map(|(id, bb)| (id, &bb)).collect();

        self.builder.build_switch(
            fid.into_int_value(), &nomatch,
            &c);
    }
}

impl ast::PrimitiveType {
    #[allow(non_snake_case)]
    fn LLVMType<'a>(&self, context: &'a Context) -> IntType<'a> {
        match self {
            ast::PrimitiveType::Bool => context.bool_type(),
            ast::PrimitiveType::Int(n) |
            ast::PrimitiveType::Uint(n) => context.custom_width_int_type(*n as u32),
            ast::PrimitiveType::Address => context.custom_width_int_type(20 * 8),
            ast::PrimitiveType::Bytes(n) => context.custom_width_int_type((*n * 8) as u32),
            _ => {
                panic!("llvm type for {:?} not implemented", self);
            }
        }
    }

    fn stack_based(&self) -> bool {
        match self {
            ast::PrimitiveType::Bool => false,
            ast::PrimitiveType::Int(n) => *n > 64,
            ast::PrimitiveType::Uint(n) => *n > 64,
            ast::PrimitiveType::Address => true,
            ast::PrimitiveType::Bytes(n) => *n > 8,
            _ => unimplemented!(),
        }
    }
}

impl resolver::Type {
    #[allow(non_snake_case)]
    fn LLVMType<'a>(&self, ns: &resolver::Contract, context: &'a Context) -> IntType<'a> {
        match self {
            resolver::Type::Primitive(e) => e.LLVMType(context),
            resolver::Type::Enum(n) => ns.enums[*n].ty.LLVMType(context),
            resolver::Type::Noreturn => unreachable!(),
        }
    }

    pub fn stack_based(&self) -> bool {
        match self {
            resolver::Type::Primitive(e) => e.stack_based(),
            resolver::Type::Enum(_) => false,
            resolver::Type::Noreturn => unreachable!(),
        }
    }
}

static STDLIB_IR: &'static [u8] = include_bytes!("../../stdlib/stdlib.bc");

fn load_stdlib(context: &Context) -> Module {
    let memory = MemoryBuffer::create_from_memory_range(STDLIB_IR, "stdlib");

    Module::parse_bitcode_from_buffer(&memory, context).unwrap()
}
