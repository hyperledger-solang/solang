
use parser::ast;
use resolver;
use resolver::cfg;
use std::str;
use std::path::Path;

use std::collections::HashMap;
use std::collections::VecDeque;

use tiny_keccak::keccak256;

use inkwell::types::BasicTypeEnum;
use inkwell::OptimizationLevel;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Module, Linkage};
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::targets::{CodeModel, RelocMode, FileType, Target};
use inkwell::AddressSpace;
use inkwell::types::{IntType, StringRadix};
use inkwell::values::{PointerValue, IntValue, PhiValue, FunctionValue, BasicValueEnum};
use inkwell::IntPredicate;

const WASMTRIPLE: &str = "wasm32-unknown-unknown-wasm";

lazy_static::lazy_static! {
    static ref LLVM_INIT: () = {
        Target::initialize_webassembly(&Default::default());
    };
}

#[derive(Clone)]
struct Variable {
    value: BasicValueEnum,
    stack: bool,
}

#[derive(Clone)]
struct Function {
    value_ref: FunctionValue,
    wasm_return: bool,
}

pub struct Contract<'a> {
    pub name: String,
    pub module: Module,
    target: Target,
    context: Context,
    ns: &'a resolver::Contract,
    functions: Vec<Function>,
    externals: HashMap<String, FunctionValue>,
}

impl<'a> Contract<'a> {
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

    pub fn new(contract: &'a resolver::Contract, filename: &str) -> Self {
        lazy_static::initialize(&LLVM_INIT);

        let target = Target::from_triple(WASMTRIPLE).unwrap();
        let context = Context::create();
        let module = context.create_module(&contract.name);
        module.set_target(&target);
        module.set_source_file_name(filename);

        let builder = context.create_builder();

        let mut e = Contract {
            name: contract.name.to_owned(),
            module: module,
            target: target,
            context: context,
            ns: contract,
            functions: Vec::new(),
            externals: HashMap::new(),
        };

        // stdlib
        let intr = load_stdlib(&e.context);

        e.module.link_in_module(intr).unwrap();

        // externals
        e.declare_externals();

        for func in &contract.functions {
            let f = e.emit_func(func, &builder);
            e.functions.push(f);
        }

        e.emit_constructor_dispatch(contract, &builder);
        e.emit_function_dispatch(contract, &builder);

        e
    }

    fn declare_externals(&mut self) {
        let ret = self.context.void_type();
        let args: Vec<BasicTypeEnum> = vec![
            self.context.i32_type().into(),
            self.context.i8_type().ptr_type(AddressSpace::Generic).into(),
            self.context.i32_type().into(),
        ];

        let ftype = ret.fn_type(&args, false);
        let func = self.module.add_function("get_storage32", ftype, Some(Linkage::External));
        self.externals.insert("get_storage32".to_owned(), func);

        let func = self.module.add_function("set_storage32", ftype, Some(Linkage::External));
        self.externals.insert("set_storage32".to_owned(), func);
    }

    fn expression(
        &self,
        builder: &Builder,
        e: &cfg::Expression,
        vartab: &Vec<Variable>,
    ) -> IntValue {
        match e {
            cfg::Expression::NumberLiteral(bits, n) => {
                let ty = self.context.custom_width_int_type(*bits as _);
                let s = n.to_string();

                ty.const_int_from_string(&s, StringRadix::Decimal).unwrap()
            }
            cfg::Expression::Add(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                builder.build_int_add(left, right, "")
            }
            cfg::Expression::Subtract(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                builder.build_int_sub(left, right, "")
            }
            cfg::Expression::Multiply(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                builder.build_int_mul(left, right, "")
            }
            cfg::Expression::UDivide(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                builder.build_int_unsigned_div(left, right, "")
            }
            cfg::Expression::SDivide(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                builder.build_int_signed_div(left, right, "")
            }
            cfg::Expression::Equal(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                builder.build_int_compare(IntPredicate::EQ, left, right, "")
            }
            cfg::Expression::More(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                builder.build_int_compare(IntPredicate::SGT, left, right, "")
            }
            cfg::Expression::Less(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                builder.build_int_compare(IntPredicate::SLT, left, right, "")
            }
            cfg::Expression::Variable(_, s) => {
                if vartab[*s].stack {
                    builder.build_load(vartab[*s].value.into_pointer_value(), "").into_int_value()
                } else {
                    vartab[*s].value.into_int_value()
                }
            }
            cfg::Expression::ZeroExt(t, e) => {
                let e = self.expression(builder, e, vartab);
                let ty = t.LLVMType(self.ns, &self.context);

                builder.build_int_z_extend(e, ty, "")
            }
            cfg::Expression::SignExt(t, e) => {
                let e = self.expression(builder, e, vartab);
                let ty = t.LLVMType(self.ns, &self.context);

                builder.build_int_s_extend(e, ty, "")
            }
            cfg::Expression::Trunc(t, e) => {
                let e = self.expression(builder, e, vartab);
                let ty = t.LLVMType(self.ns, &self.context);

                builder.build_int_truncate(e, ty, "")
            }
            _ => {
                panic!("expression not implemented");
            }
        }
    }

    fn emit_constructor_dispatch(&self, contract: &resolver::Contract, builder: &Builder) {
        // create start function
        let ret = self.context.void_type();
        let ftype = ret.fn_type(&[self.context.i32_type().ptr_type(AddressSpace::Generic).into()], false);
        let function = self.module.add_function("constructor", ftype, None);

        let entry = self.context.append_basic_block(&function, "entry");

        builder.position_at_end(&entry);

        // init our heap
        builder.build_call(
            self.module.get_function("__init_heap").unwrap(),
            &[],
            "");

        if let Some(n) = contract.constructor_function() {
            let mut args = Vec::new();

            let arg = function.get_first_param().unwrap().into_pointer_value();
            let length = builder.build_load(arg, "length");

            // step over length
            let args_ptr = unsafe {
                builder.build_gep(arg,
                    &[self.context.i32_type().const_int(1, false).into()],
                    "args_ptr")
            };

            // insert abi decode
            self.emit_abi_decode(
                builder,
                &function,
                &mut args,
                args_ptr,
                length.into_int_value(),
                &contract.functions[n],
            );

            builder.build_call(self.functions[n].value_ref, &args, "");
        }

        builder.build_return(None);
    }

    fn emit_function_dispatch(&self, contract: &resolver::Contract, builder: &Builder) {
        // create start function
        let ret = self.context.i32_type().ptr_type(AddressSpace::Generic);
        let ftype = ret.fn_type(&[self.context.i32_type().ptr_type(AddressSpace::Generic).into()], false);
        let function = self.module.add_function("function", ftype, None);

        let entry = self.context.append_basic_block(&function, "entry");
        let fallback = self.context.append_basic_block(&function, "fallback");
        let switch = self.context.append_basic_block(&function, "switch");

        builder.position_at_end(&entry);

        let arg = function.get_first_param().unwrap().into_pointer_value();
        let length = builder.build_load(arg, "length").into_int_value();

        let not_fallback = builder.build_int_compare(
            IntPredicate::UGE,
            length,
            self.context.i32_type().const_int(4, false).into(),
            "");

        builder.build_conditional_branch(not_fallback, &switch, &fallback);

        builder.position_at_end(&switch);

        let fid_ptr = unsafe {
            builder.build_gep(
                arg,
                &[self.context.i32_type().const_int(1, false).into()],
                "fid_ptr")
        };

        let fid = builder.build_load(fid_ptr, "fid");

        // pointer/size for abi decoding
        let args_ptr = unsafe {
            builder.build_gep(
                arg,
                &[self.context.i32_type().const_int(2, false).into()],
                "fid_ptr")
        };

        let args_len = builder.build_int_sub(
            length.into(),
            self.context.i32_type().const_int(4, false).into(),
            "args_len"
        );

        let nomatch = self.context.append_basic_block(&function, "nomatch");

        builder.position_at_end(&nomatch);

        builder.build_unreachable();

        let mut cases = Vec::new();

        for (i, f) in contract.functions.iter().enumerate() {
            // ignore constructors and fallback
            if f.name == None {
                continue;
            }

            match &f.visibility {
                ast::Visibility::Internal(_) | ast::Visibility::Private(_) => {
                    continue;
                }
                _ => (),
            }

            let res = keccak256(f.sig.as_bytes());

            let bb = self.context.append_basic_block(&function, "");
            let id = u32::from_le_bytes([res[0], res[1], res[2], res[3]]);

            builder.position_at_end(&bb);

            let mut args = Vec::new();

            // insert abi decode
            self.emit_abi_decode(builder, &function, &mut args, args_ptr, args_len, f);

            let ret = builder.build_call(
                self.functions[i].value_ref,
                &args,
                "").try_as_basic_value().left();

            if f.returns.is_empty() {
                // return ABI of length 0

                // malloc 4 bytes
                let dest = builder.build_call(
                    self.module.get_function("__malloc").unwrap(),
                    &[self.context.i32_type().const_int(4, false).into()],
                    ""
                ).try_as_basic_value().left().unwrap().into_pointer_value();

                builder.build_store(
                    builder.build_pointer_cast(dest,
                        self.context.i32_type().ptr_type(AddressSpace::Generic),
                        ""),
                    self.context.i32_type().const_zero());

                builder.build_return(Some(&dest));
            } else if self.functions[i].wasm_return {
                // malloc 36 bytes
                let dest = builder.build_call(
                    self.module.get_function("__malloc").unwrap(),
                    &[self.context.i32_type().const_int(36, false).into()],
                    ""
                ).try_as_basic_value().left().unwrap().into_pointer_value();

                // write length
                builder.build_store(
                    builder.build_pointer_cast(dest,
                        self.context.i32_type().ptr_type(AddressSpace::Generic),
                        ""),
                    self.context.i32_type().const_int(32, false));

                // malloc returns u8*
                let abi_ptr = unsafe {
                    builder.build_gep(
                        dest,
                        &[ self.context.i32_type().const_int(4, false).into()],
                        "abi_ptr")
                };

                // insert abi decode
                let ty = match &f.returns[0].ty {
                    resolver::TypeName::Elementary(e) => e,
                    resolver::TypeName::Enum(n) => &self.ns.enums[*n].ty,
                    resolver::TypeName::Noreturn => unreachable!(),
                };

                self.emit_abi_encode_single_val(builder, &ty, abi_ptr, ret.unwrap().into_int_value());

                builder.build_return(Some(&dest));
            } else {
                // FIXME: abi encode all the arguments
            }

            cases.push((self.context.i32_type().const_int(id as u64, false), bb));
        }

        builder.position_at_end(&switch);

        let mut c = Vec::new();

        for (id, bb) in cases.iter() {
            c.push((*id, bb));
        }

        //let c = cases.into_iter().map(|(id, bb)| (id, &bb)).collect();

        builder.build_switch(
            fid.into_int_value(), &nomatch,
            &c);

        // FIXME: emit code for public contract variables

        // emit fallback code
        builder.position_at_end(&fallback);

        match contract.fallback_function() {
            Some(n) => {
                builder.build_call(
                    self.functions[n].value_ref,
                    &[],
                    "");

                builder.build_return(None);
            }
            None => {
                builder.build_unreachable();
            },
        }
    }

    fn emit_abi_encode_single_val(
        &self,
        builder: &Builder,
        ty: &ast::ElementaryTypeName,
        dest: PointerValue,
        val: IntValue,
    ) {
        match ty {
            ast::ElementaryTypeName::Bool => {
                // first clear
                let dest8 = builder.build_pointer_cast(dest,
                    self.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid");

                builder.build_call(
                    self.module.get_function("__bzero8").unwrap(),
                    &[ dest8.into(),
                       self.context.i32_type().const_int(4, false).into() ],
                    "");

                let value = builder.build_select(val,
                    self.context.i8_type().const_int(1, false),
                    self.context.i8_type().const_zero(),
                    "bool_val");

                let dest = unsafe {
                    builder.build_gep(
                        dest8,
                        &[ self.context.i32_type().const_int(31, false).into() ],
                        "")
                };

                builder.build_store(dest, value);
            }
            ast::ElementaryTypeName::Int(8) | ast::ElementaryTypeName::Uint(8) => {
                let signval = if let ast::ElementaryTypeName::Int(8) = ty {
                    let negative = builder.build_int_compare(IntPredicate::SLT,
                            val, self.context.i8_type().const_zero(), "neg");

                    builder.build_select(negative,
                        self.context.i64_type().const_zero(),
                        self.context.i64_type().const_int(std::u64::MAX, true),
                        "val").into_int_value()
                } else {
                    self.context.i64_type().const_zero()
                };

                let dest8 = builder.build_pointer_cast(dest,
                    self.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid");

                builder.build_call(
                    self.module.get_function("__memset8").unwrap(),
                    &[ dest8.into(), signval.into(),
                       self.context.i32_type().const_int(4, false).into() ],
                    "");

                let dest = unsafe {
                    builder.build_gep(
                        dest8,
                        &[ self.context.i32_type().const_int(31, false).into() ],
                        "")
                };

                builder.build_store(dest, val);
            }
            ast::ElementaryTypeName::Uint(n) | ast::ElementaryTypeName::Int(n) => {
                // first clear/set the upper bits
                if *n < 256 {
                    let signval = if let ast::ElementaryTypeName::Int(8) = ty {
                        let negative = builder.build_int_compare(IntPredicate::SLT,
                                val, self.context.i8_type().const_zero(), "neg");

                        builder.build_select(negative,
                            self.context.i64_type().const_zero(),
                            self.context.i64_type().const_int(std::u64::MAX, true),
                            "val").into_int_value()
                    } else {
                        self.context.i64_type().const_zero()
                    };

                    let dest8 = builder.build_pointer_cast(dest,
                        self.context.i8_type().ptr_type(AddressSpace::Generic),
                        "destvoid");

                    builder.build_call(
                        self.module.get_function("__memset8").unwrap(),
                        &[ dest8.into(), signval.into(),
                            self.context.i32_type().const_int(4, false).into() ],
                        "");
                }

                // no need to allocate space for each uint64
                // allocate enough for type
                let int_type = self.context.custom_width_int_type(*n as u32);
                let type_size = int_type.size_of();

                let store = builder.build_alloca(int_type, "stack");

                builder.build_store(store, val);

                builder.build_call(
                    self.module.get_function("__leNtobe32").unwrap(),
                    &[ builder.build_pointer_cast(store,
                            self.context.i8_type().ptr_type(AddressSpace::Generic),
                            "destvoid").into(),
                       builder.build_pointer_cast(dest,
                            self.context.i8_type().ptr_type(AddressSpace::Generic),
                            "destvoid").into(),
                        builder.build_int_truncate(type_size,
                            self.context.i32_type(), "").into()
                    ],
                    "");
            }
            _ => unimplemented!(),
        }
    }

    fn emit_abi_decode(
        &self,
        builder: &Builder,
        function: &FunctionValue,
        args: &mut Vec<BasicValueEnum>,
        data: PointerValue,
        length: IntValue,
        spec: &resolver::FunctionDecl,
    ) {
        let mut data = data;
        let decode_block = self.context.append_basic_block(function, "abi_decode");
        let wrong_length_block = self.context.append_basic_block(function, "wrong_abi_length");

        let is_ok = builder.build_int_compare(IntPredicate::EQ, length,
            self.context.i32_type().const_int(32  * spec.params.len() as u64, false),
            "correct_length");

        builder.build_conditional_branch(is_ok, &decode_block, &wrong_length_block);

        builder.position_at_end(&decode_block);

        for arg in &spec.params {
            let ty = match &arg.ty {
                resolver::TypeName::Elementary(e) => e,
                resolver::TypeName::Enum(n) => &self.ns.enums[*n].ty,
                resolver::TypeName::Noreturn => unreachable!(),
            };

            args.push(match ty {
                ast::ElementaryTypeName::Bool => {
                    // solidity checks all the 32 bytes for being non-zero; we will just look at the upper 8 bytes, else we would need four loads
                    // which is unneeded (hopefully)
                    // cast to 64 bit pointer
                    let bool_ptr = builder.build_pointer_cast(data,
                        self.context.i64_type().ptr_type(AddressSpace::Generic), "");

                    let bool_ptr = unsafe {
                        builder.build_gep(bool_ptr,
                            &[ self.context.i32_type().const_int(3, false) ],
                            "bool_ptr")
                    };

                    builder.build_int_compare(IntPredicate::EQ,
                        builder.build_load(bool_ptr, "abi_bool").into_int_value(),
                        self.context.i64_type().const_zero(), "bool").into()
                }
                ast::ElementaryTypeName::Uint(8) | ast::ElementaryTypeName::Int(8) => {
                    let int8_ptr = builder.build_pointer_cast(data,
                        self.context.i8_type().ptr_type(AddressSpace::Generic), "");

                    let int8_ptr = unsafe {
                        builder.build_gep(int8_ptr,
                        &[ self.context.i32_type().const_int(31, false) ],
                        "bool_ptr")
                    };

                    builder.build_load(int8_ptr, "abi_int8")
                }
                ast::ElementaryTypeName::Uint(n) | ast::ElementaryTypeName::Int(n) => {
                    let int_type = self.context.custom_width_int_type(*n as u32);
                    let type_size = int_type.size_of();

                    let store = builder.build_alloca(int_type, "stack");

                    builder.build_call(
                        self.module.get_function("__be32toleN").unwrap(),
                        &[
                            builder.build_pointer_cast(data,
                                self.context.i8_type().ptr_type(AddressSpace::Generic), "").into(),
                            builder.build_pointer_cast(store,
                                self.context.i8_type().ptr_type(AddressSpace::Generic), "").into(),
                            builder.build_int_truncate(type_size,
                                self.context.i32_type(), "size").into()
                        ],
                        ""
                    );

                    if *n <= 64 {
                        builder.build_load(store, &format!("abi_int{}", *n))
                    } else {
                        store.into()
                    }
                }
                _ => panic!(),
            });

            data = unsafe {
                builder.build_gep(data,
                    &[ self.context.i32_type().const_int(8, false)],
                    "data_next")
            };
        }

        // FIXME: generate a call to revert/abort with some human readable error or error code
        builder.position_at_end(&wrong_length_block);
        builder.build_unreachable();

        builder.position_at_end(&decode_block);
    }

    fn emit_func(&self, f: &resolver::FunctionDecl, builder: &Builder) -> Function {
        let mut args: Vec<BasicTypeEnum> = Vec::new();
        let mut wasm_return = false;

        for p in &f.params {
            let ty = p.ty.LLVMType(self.ns, &self.context);
            args.push(if p.ty.stack_based() {
                ty.ptr_type(AddressSpace::Generic).into()
            } else {
                ty.into()
            });
        }

        let fname = if f.constructor {
            "sol::__constructor".to_string()
        } else if let Some(ref name) = f.name {
            format!("sol::{}", name)
        } else {
            "sol::__fallback".to_string()
        };

        let ftype = if f.returns.len() == 1 && !f.returns[0].ty.stack_based() {
            wasm_return = true;
            f.returns[0].ty.LLVMType(self.ns, &self.context).fn_type(&args, false)
        } else {
            // add return
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

        // recurse through basic blocks
        struct BasicBlock {
            bb: inkwell::basic_block::BasicBlock,
            phis: HashMap<usize, PhiValue>,
        }

        struct Work {
            bb_no: usize,
            vars: Vec<Variable>,
        }

        let mut blocks: HashMap<usize, BasicBlock> = HashMap::new();

        let create_bb = |bb_no| -> BasicBlock {
            let cfg_bb: &cfg::BasicBlock = &cfg.bb[bb_no];
            let mut phis = HashMap::new();

            let bb = self.context.append_basic_block(&function, &cfg_bb.name);

            builder.position_at_end(&bb);

            if let Some(ref cfg_phis) = cfg_bb.phis {
                for v in cfg_phis {
                    // FIXME: no phis needed for stack based vars
                    let ty = cfg.vars[*v].ty.LLVMType(self.ns, &self.context);

                    phis.insert(*v, builder.build_phi(ty, &cfg.vars[*v].id.name).into());
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
                cfg::Storage::Local | cfg::Storage::Contract(_) => {
                    vars.push(Variable {
                        value: builder.build_alloca(
                            v.ty.LLVMType(self.ns, &self.context), &v.id.name).into(),
                        stack: true,
                    });
                }
                cfg::Storage::Constant => {
                    // nothing to do
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

                builder.position_at_end(&bb.bb);

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
                        builder.build_return(None);
                    },
                    cfg::Instr::Return { value } if wasm_return => {
                        let retval = self.expression(builder, &value[0], &w.vars);
                        builder.build_return(Some(&retval));
                    }
                    cfg::Instr::Return { value } => {
                        let returns_offset = f.params.len();
                        for (i, val) in value.iter().enumerate() {
                            let arg = function.get_nth_param((returns_offset + i) as u32).unwrap();
                            let retval = self.expression(builder, val, &w.vars);

                            builder.build_store(arg.into_pointer_value(), retval);
                        }
                        builder.build_return(None);
                    }
                    cfg::Instr::Set { res, expr } => {
                        let value_ref = self.expression(builder, expr, &w.vars);
                        if w.vars[*res].stack {
                            builder.build_store(w.vars[*res].value.into_pointer_value(), value_ref);
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

                        builder.position_at_end(&ll_bb);
                        builder.build_unconditional_branch(&bb.bb);
                    }
                    cfg::Instr::BranchCond {
                        cond,
                        true_,
                        false_,
                    } => {
                        let cond = self.expression(builder, cond, &w.vars);

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

                        builder.position_at_end(&ll_bb);
                        builder.build_conditional_branch(cond, &bb_true, &bb_false);
                    }
                    cfg::Instr::GetStorage { local, storage } => {
                        let dest = w.vars[*local].value.into_pointer_value();

                        builder.build_call(
                            self.externals["get_storage32"],
                            &[
                                self.context.i32_type().const_int(*storage as u64, false).into(),
                                builder.build_pointer_cast(dest,
                                    self.context.i8_type().ptr_type(AddressSpace::Generic), "").into(),
                                dest.get_type().size_of().const_cast(
                                    self.context.i32_type(), false).into()
                            ],
                            "");
                    }
                    cfg::Instr::SetStorage { local, storage } => {
                        let dest = w.vars[*local].value.into_pointer_value();

                        builder.build_call(
                            self.externals["set_storage32"],
                            &[
                                self.context.i32_type().const_int(*storage as u64, false).into(),
                                builder.build_pointer_cast(dest,
                                    self.context.i8_type().ptr_type(AddressSpace::Generic), "").into(),
                                dest.get_type().size_of().const_cast(
                                    self.context.i32_type(), false).into()
                            ],
                            "");
                    }
                    cfg::Instr::Call { res, func, args } => {
                        let mut parms: Vec<BasicValueEnum> = Vec::new();

                        for a in args {
                            parms.push(self.expression(builder, &a, &w.vars).into());
                        }

                        let ret = builder.build_call(
                            self.functions[*func].value_ref,
                            &parms, "").try_as_basic_value().left().unwrap();

                        if res.len() > 0 {
                            w.vars[res[0]].value = ret.into();
                        }
                    }
                }
            }
        }

        Function {
            value_ref: function,
            wasm_return,
        }
    }
}

impl ast::ElementaryTypeName {
    #[allow(non_snake_case)]
    fn LLVMType(&self, context: &Context) -> IntType {
        match self {
            ast::ElementaryTypeName::Bool => context.bool_type(),
            ast::ElementaryTypeName::Int(n) |
            ast::ElementaryTypeName::Uint(n) => context.custom_width_int_type(*n as u32),
            ast::ElementaryTypeName::Address => context.custom_width_int_type(20 * 8),
            ast::ElementaryTypeName::Bytes(n) => context.custom_width_int_type((*n * 8) as u32),
            _ => {
                panic!("llvm type for {:?} not implemented", self);
            }
        }
    }

    fn stack_based(&self) -> bool {
        match self {
            ast::ElementaryTypeName::Bool => false,
            ast::ElementaryTypeName::Int(n) => *n > 64,
            ast::ElementaryTypeName::Uint(n) => *n > 64,
            ast::ElementaryTypeName::Address => true,
            ast::ElementaryTypeName::Bytes(n) => *n > 8,
            _ => unimplemented!(),
        }
    }
}

impl resolver::TypeName {
    #[allow(non_snake_case)]
    fn LLVMType(&self, ns: &resolver::Contract, context: &Context) -> IntType {
        match self {
            resolver::TypeName::Elementary(e) => e.LLVMType(context),
            resolver::TypeName::Enum(n) => ns.enums[*n].ty.LLVMType(context),
            resolver::TypeName::Noreturn => unreachable!(),
        }
    }

    fn stack_based(&self) -> bool {
        match self {
            resolver::TypeName::Elementary(e) => e.stack_based(),
            resolver::TypeName::Enum(_) => false,
            resolver::TypeName::Noreturn => unreachable!(),
        }
    }
}

static STDLIB_IR: &'static [u8] = include_bytes!("../../stdlib/stdlib.bc");

fn load_stdlib(context: &Context) -> Module {
    let memory = MemoryBuffer::create_from_memory_range(STDLIB_IR, "stdlib");

    Module::parse_bitcode_from_buffer_in_context(&memory, context).unwrap()
}
