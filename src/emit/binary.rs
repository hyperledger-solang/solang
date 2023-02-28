// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{ArrayLength, Contract, Namespace, StructType, Type};
use std::cell::RefCell;
use std::path::Path;
use std::str;

use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::collections::HashMap;

use crate::codegen::{cfg::ReturnCode, Options};
use crate::emit::substrate;
use crate::emit::{solana, BinaryOp, Generate};
use crate::linker::link;
use crate::Target;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::debug_info::DICompileUnit;
use inkwell::debug_info::DebugInfoBuilder;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::module::{Linkage, Module};
use inkwell::passes::PassManager;
use inkwell::targets::{CodeModel, FileType, RelocMode};
use inkwell::types::{
    ArrayType, BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType, IntType, StringRadix,
};
use inkwell::values::{BasicValueEnum, FunctionValue, GlobalValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;
use once_cell::sync::OnceCell;

static LLVM_INIT: OnceCell<()> = OnceCell::new();

pub struct Binary<'a> {
    pub name: String,
    pub module: Module<'a>,
    pub(crate) options: &'a Options,
    pub runtime: Option<Box<Binary<'a>>>,
    target: Target,
    pub(crate) function_abort_value_transfers: bool,
    pub(crate) constructor_abort_value_transfers: bool,
    pub builder: Builder<'a>,
    pub dibuilder: DebugInfoBuilder<'a>,
    pub compile_unit: DICompileUnit<'a>,
    pub(crate) context: &'a Context,
    pub(crate) functions: HashMap<usize, FunctionValue<'a>>,
    code: RefCell<Vec<u8>>,
    pub(crate) selector: GlobalValue<'a>,
    pub(crate) calldata_len: GlobalValue<'a>,
    pub(crate) scratch_len: Option<GlobalValue<'a>>,
    pub(crate) scratch: Option<GlobalValue<'a>>,
    pub(crate) parameters: Option<PointerValue<'a>>,
    pub(crate) return_values: HashMap<ReturnCode, IntValue<'a>>,
}

impl<'a> Binary<'a> {
    /// Build the LLVM IR for a single contract
    pub fn build(
        context: &'a Context,
        contract: &'a Contract,
        ns: &'a Namespace,
        filename: &'a str,
        opt: &'a Options,
    ) -> Self {
        let std_lib = load_stdlib(context, &ns.target);
        match ns.target {
            Target::Substrate { .. } => {
                substrate::SubstrateTarget::build(context, &std_lib, contract, ns, filename, opt)
            }
            Target::Solana => {
                solana::SolanaTarget::build(context, &std_lib, contract, ns, filename, opt)
            }
            Target::EVM => unimplemented!(),
        }
    }

    /// Compile the bin and return the code as bytes. The result is
    /// cached, since this function can be called multiple times (e.g. one for
    /// each time a bin of this type is created).
    /// Pass our module to llvm for optimization and compilation
    pub fn code(&self, generate: Generate) -> Result<Vec<u8>, String> {
        // return cached result if available
        if !self.code.borrow().is_empty() {
            return Ok(self.code.borrow().clone());
        }

        match self.options.opt_level.into() {
            OptimizationLevel::Default | OptimizationLevel::Aggressive => {
                let pass_manager = PassManager::create(());

                pass_manager.add_promote_memory_to_register_pass();
                pass_manager.add_function_inlining_pass();
                pass_manager.add_global_dce_pass();
                pass_manager.add_constant_merge_pass();

                pass_manager.run_on(&self.module);
            }
            _ => {}
        }

        let target = inkwell::targets::Target::from_name(self.target.llvm_target_name()).unwrap();

        let target_machine = target
            .create_target_machine(
                &self.target.llvm_target_triple(),
                "",
                self.target.llvm_features(),
                self.options.opt_level.into(),
                RelocMode::Default,
                CodeModel::Default,
            )
            .unwrap();

        match target_machine.write_to_memory_buffer(
            &self.module,
            if generate == Generate::Assembly {
                FileType::Assembly
            } else {
                FileType::Object
            },
        ) {
            Ok(out) => {
                let slice = out.as_slice();

                if generate == Generate::Linked {
                    let bs = link(slice, &self.name, self.target);

                    Ok(bs.to_vec())
                } else {
                    Ok(slice.to_vec())
                }
            }
            Err(s) => Err(s.to_string()),
        }
    }

    /// Mark all functions as internal unless they're in the export_list. This helps the
    /// llvm globaldce pass eliminate unnecessary functions and reduce the wasm output.
    pub(crate) fn internalize(&self, export_list: &[&str]) {
        let mut func = self.module.get_first_function();

        // FIXME: these functions are called from code generated by lowering into wasm,
        // so eliminating them now will cause link errors. Either we should prevent these
        // calls from being done in the first place or do dce at link time
        let mut export_list = export_list.to_vec();
        export_list.push("__ashlti3");
        export_list.push("__lshrti3");
        export_list.push("__ashrti3");

        while let Some(f) = func {
            let name = f.get_name().to_str().unwrap();

            if !name.starts_with("llvm.")
                && export_list.iter().all(|e| {
                    // symbols may get renamed foo.1 or foo.2, also export those
                    if let Some(tail) = name.strip_prefix(e) {
                        if let Some(no) = tail.strip_prefix('.') {
                            no.parse::<u32>().is_ok()
                        } else {
                            tail.is_empty()
                        }
                    } else {
                        false
                    }
                })
            {
                f.set_linkage(Linkage::Internal);
            }

            func = f.get_next_function();
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

    pub fn new(
        context: &'a Context,
        target: Target,
        name: &str,
        filename: &str,
        opt: &'a Options,
        std_lib: &Module<'a>,
        runtime: Option<Box<Binary<'a>>>,
    ) -> Self {
        LLVM_INIT.get_or_init(|| {
            inkwell::targets::Target::initialize_webassembly(&Default::default());
            inkwell::targets::Target::initialize_bpf(&Default::default());
        });

        let triple = target.llvm_target_triple();
        let module = context.create_module(name);

        let debug_metadata_version = context.i32_type().const_int(3, false);
        module.add_basic_value_flag(
            "Debug Info Version",
            inkwell::module::FlagBehavior::Warning,
            debug_metadata_version,
        );

        let builder = context.create_builder();
        let (dibuilder, compile_unit) = module.create_debug_info_builder(
            true,
            inkwell::debug_info::DWARFSourceLanguage::C,
            filename,
            ".",
            "Solang",
            false,
            "",
            0,
            "",
            inkwell::debug_info::DWARFEmissionKind::Full,
            0,
            false,
            false,
            "",
            "",
        );

        module.set_triple(&triple);
        module.set_source_file_name(filename);

        module.link_in_module(std_lib.clone()).unwrap();

        let selector = module.add_global(
            context.i32_type(),
            Some(AddressSpace::default()),
            "selector",
        );
        selector.set_linkage(Linkage::Internal);
        selector.set_initializer(&context.i32_type().const_zero());

        let calldata_len = module.add_global(
            context.i32_type(),
            Some(AddressSpace::default()),
            "calldata_len",
        );
        calldata_len.set_linkage(Linkage::Internal);
        calldata_len.set_initializer(&context.i32_type().const_zero());

        let mut return_values = HashMap::new();

        return_values.insert(ReturnCode::Success, context.i32_type().const_zero());
        return_values.insert(
            ReturnCode::FunctionSelectorInvalid,
            context.i32_type().const_int(3, false),
        );
        return_values.insert(
            ReturnCode::AbiEncodingInvalid,
            context.i32_type().const_int(2, false),
        );

        Binary {
            name: name.to_owned(),
            module,
            runtime,
            function_abort_value_transfers: false,
            constructor_abort_value_transfers: false,
            builder,
            dibuilder,
            compile_unit,
            context,
            target,
            functions: HashMap::new(),
            code: RefCell::new(Vec::new()),
            options: opt,
            selector,
            calldata_len,
            scratch: None,
            scratch_len: None,
            parameters: None,
            return_values,
        }
    }

    /// Set flags for early aborts if a value transfer is done and no function/constructor can handle it
    pub fn set_early_value_aborts(&mut self, contract: &Contract, ns: &Namespace) {
        // if there is no payable function, fallback or receive then abort all value transfers at the top
        // note that receive() is always payable so this just checkes for presence.
        self.function_abort_value_transfers = !contract.functions.iter().any(|function_no| {
            let f = &ns.functions[*function_no];
            !f.is_constructor() && f.is_payable()
        });

        self.constructor_abort_value_transfers = !contract.functions.iter().any(|function_no| {
            let f = &ns.functions[*function_no];
            f.is_constructor() && f.is_payable()
        });
    }

    /// llvm value type, as in chain currency (usually 128 bits int)
    pub(crate) fn value_type(&self, ns: &Namespace) -> IntType<'a> {
        self.context
            .custom_width_int_type(ns.value_length as u32 * 8)
    }

    /// llvm address type
    pub(crate) fn address_type(&self, ns: &Namespace) -> ArrayType<'a> {
        self.context.i8_type().array_type(ns.address_length as u32)
    }

    /// Creates global string in the llvm module with initializer
    ///
    pub(crate) fn emit_global_string(
        &self,
        name: &str,
        data: &[u8],
        constant: bool,
    ) -> PointerValue<'a> {
        let ty = self.context.i8_type().array_type(data.len() as u32);

        let gv = self
            .module
            .add_global(ty, Some(AddressSpace::default()), name);

        gv.set_linkage(Linkage::Internal);

        gv.set_initializer(&self.context.const_string(data, false));

        if constant {
            gv.set_constant(true);
            gv.set_unnamed_addr(true);
        }

        gv.as_pointer_value()
    }

    /// Wrapper for alloca. Ensures that the alloca is done on the first basic block.
    /// If alloca is not on the first basic block, llvm will get to llvm_unreachable
    /// for the BPF target.
    pub(crate) fn build_alloca<T: BasicType<'a>>(
        &self,
        function: inkwell::values::FunctionValue<'a>,
        ty: T,
        name: &str,
    ) -> PointerValue<'a> {
        let entry = function
            .get_first_basic_block()
            .expect("function missing entry block");
        let current = self.builder.get_insert_block().unwrap();

        if let Some(instr) = &entry.get_first_instruction() {
            self.builder.position_before(instr);
        } else {
            // if there is no instruction yet, then nothing was built
            self.builder.position_at_end(entry);
        }

        let res = self.builder.build_alloca(ty, name);

        self.builder.position_at_end(current);

        res
    }

    pub(crate) fn build_array_alloca<T: BasicType<'a>>(
        &self,
        function: inkwell::values::FunctionValue<'a>,
        ty: T,
        length: IntValue<'a>,
        name: &str,
    ) -> PointerValue<'a> {
        let entry = function
            .get_first_basic_block()
            .expect("function missing entry block");
        let current = self.builder.get_insert_block().unwrap();

        if let Some(instr) = entry.get_first_instruction() {
            self.builder.position_before(&instr);
        } else {
            self.builder.position_at_end(entry);
        }

        let res = self.builder.build_array_alloca(ty, length, name);

        self.builder.position_at_end(current);

        res
    }

    /// Emit a loop from `from` to `to`. The closure exists to insert the body of the loop; the closure
    /// gets the loop variable passed to it as an IntValue, and a userdata PointerValue
    pub fn emit_static_loop_with_pointer<F>(
        &self,
        function: FunctionValue,
        from: IntValue<'a>,
        to: IntValue<'a>,
        data_ref: &mut PointerValue<'a>,
        mut insert_body: F,
    ) where
        F: FnMut(IntValue<'a>, &mut PointerValue<'a>),
    {
        let body = self.context.append_basic_block(function, "body");
        let done = self.context.append_basic_block(function, "done");
        let entry = self.builder.get_insert_block().unwrap();

        self.builder.build_unconditional_branch(body);
        self.builder.position_at_end(body);

        let loop_ty = from.get_type();
        let loop_phi = self.builder.build_phi(loop_ty, "index");
        let data_phi = self.builder.build_phi(data_ref.get_type(), "data");
        let mut data = data_phi.as_basic_value().into_pointer_value();

        let loop_var = loop_phi.as_basic_value().into_int_value();

        // add loop body
        insert_body(loop_var, &mut data);

        let next = self
            .builder
            .build_int_add(loop_var, loop_ty.const_int(1, false), "next_index");

        let comp = self
            .builder
            .build_int_compare(IntPredicate::ULT, next, to, "loop_cond");
        self.builder.build_conditional_branch(comp, body, done);

        let body = self.builder.get_insert_block().unwrap();
        loop_phi.add_incoming(&[(&from, entry), (&next, body)]);
        data_phi.add_incoming(&[(&*data_ref, entry), (&data, body)]);

        self.builder.position_at_end(done);

        *data_ref = data;
    }

    /// Emit a loop from `from` to `to`. The closure exists to insert the body of the loop; the closure
    /// gets the loop variable passed to it as an IntValue, and a userdata IntValue
    pub fn emit_static_loop_with_int<F>(
        &self,
        function: FunctionValue,
        from: IntValue<'a>,
        to: IntValue<'a>,
        data_ref: &mut IntValue<'a>,
        mut insert_body: F,
    ) where
        F: FnMut(IntValue<'a>, &mut IntValue<'a>),
    {
        let body = self.context.append_basic_block(function, "body");
        let done = self.context.append_basic_block(function, "done");
        let entry = self.builder.get_insert_block().unwrap();

        self.builder.build_unconditional_branch(body);
        self.builder.position_at_end(body);

        let loop_ty = from.get_type();
        let loop_phi = self.builder.build_phi(loop_ty, "index");
        let data_phi = self.builder.build_phi(data_ref.get_type(), "data");
        let mut data = data_phi.as_basic_value().into_int_value();

        let loop_var = loop_phi.as_basic_value().into_int_value();

        // add loop body
        insert_body(loop_var, &mut data);

        let next = self
            .builder
            .build_int_add(loop_var, loop_ty.const_int(1, false), "next_index");

        let comp = self
            .builder
            .build_int_compare(IntPredicate::ULT, next, to, "loop_cond");
        self.builder.build_conditional_branch(comp, body, done);

        let body = self.builder.get_insert_block().unwrap();
        loop_phi.add_incoming(&[(&from, entry), (&next, body)]);
        data_phi.add_incoming(&[(&*data_ref, entry), (&data, body)]);

        self.builder.position_at_end(done);

        *data_ref = data;
    }

    /// Emit a loop from `from` to `to`, checking the condition _before_ the body.
    pub fn emit_loop_cond_first_with_int<F>(
        &self,
        function: FunctionValue,
        from: IntValue<'a>,
        to: IntValue<'a>,
        data_ref: &mut IntValue<'a>,
        mut insert_body: F,
    ) where
        F: FnMut(IntValue<'a>, &mut IntValue<'a>),
    {
        let cond = self.context.append_basic_block(function, "cond");
        let body = self.context.append_basic_block(function, "body");
        let done = self.context.append_basic_block(function, "done");
        let entry = self.builder.get_insert_block().unwrap();

        self.builder.build_unconditional_branch(cond);
        self.builder.position_at_end(cond);

        let loop_ty = from.get_type();
        let loop_phi = self.builder.build_phi(loop_ty, "index");
        let data_phi = self.builder.build_phi(data_ref.get_type(), "data");
        let mut data = data_phi.as_basic_value().into_int_value();

        let loop_var = loop_phi.as_basic_value().into_int_value();

        let next = self
            .builder
            .build_int_add(loop_var, loop_ty.const_int(1, false), "next_index");

        let comp = self
            .builder
            .build_int_compare(IntPredicate::ULT, loop_var, to, "loop_cond");
        self.builder.build_conditional_branch(comp, body, done);

        self.builder.position_at_end(body);
        // add loop body
        insert_body(loop_var, &mut data);

        let body = self.builder.get_insert_block().unwrap();

        loop_phi.add_incoming(&[(&from, entry), (&next, body)]);
        data_phi.add_incoming(&[(&*data_ref, entry), (&data, body)]);

        self.builder.build_unconditional_branch(cond);

        self.builder.position_at_end(done);

        *data_ref = data_phi.as_basic_value().into_int_value();
    }

    /// Emit a loop from `from` to `to`, checking the condition _before_ the body.
    pub fn emit_loop_cond_first_with_pointer<F>(
        &self,
        function: FunctionValue,
        from: IntValue<'a>,
        to: IntValue<'a>,
        data_ref: &mut PointerValue<'a>,
        mut insert_body: F,
    ) where
        F: FnMut(IntValue<'a>, &mut PointerValue<'a>),
    {
        let cond = self.context.append_basic_block(function, "cond");
        let body = self.context.append_basic_block(function, "body");
        let done = self.context.append_basic_block(function, "done");
        let entry = self.builder.get_insert_block().unwrap();

        self.builder.build_unconditional_branch(cond);
        self.builder.position_at_end(cond);

        let loop_ty = from.get_type();
        let loop_phi = self.builder.build_phi(loop_ty, "index");
        let data_phi = self.builder.build_phi(data_ref.get_type(), "data");
        let mut data = data_phi.as_basic_value().into_pointer_value();

        let loop_var = loop_phi.as_basic_value().into_int_value();

        let next = self
            .builder
            .build_int_add(loop_var, loop_ty.const_int(1, false), "next_index");

        let comp = self
            .builder
            .build_int_compare(IntPredicate::ULT, loop_var, to, "loop_cond");
        self.builder.build_conditional_branch(comp, body, done);

        self.builder.position_at_end(body);
        // add loop body
        insert_body(loop_var, &mut data);

        let body = self.builder.get_insert_block().unwrap();

        loop_phi.add_incoming(&[(&from, entry), (&next, body)]);
        data_phi.add_incoming(&[(&*data_ref, entry), (&data, body)]);

        self.builder.build_unconditional_branch(cond);

        self.builder.position_at_end(done);

        *data_ref = data_phi.as_basic_value().into_pointer_value();
    }

    /// Convert a BigInt number to llvm const value
    pub(crate) fn number_literal(&self, bits: u32, n: &BigInt, _ns: &Namespace) -> IntValue<'a> {
        let ty = self.context.custom_width_int_type(bits);
        let s = n.to_string();

        ty.const_int_from_string(&s, StringRadix::Decimal).unwrap()
    }

    /// Emit function prototype
    pub(crate) fn function_type(
        &self,
        params: &[Type],
        returns: &[Type],
        ns: &Namespace,
    ) -> FunctionType<'a> {
        // function parameters
        let mut args = params
            .iter()
            .map(|ty| self.llvm_var_ty(ty, ns).into())
            .collect::<Vec<BasicMetadataTypeEnum>>();

        // add return values
        for ty in returns {
            args.push(if ty.is_reference_type(ns) && !ty.is_contract_storage() {
                self.llvm_type(ty, ns)
                    .ptr_type(AddressSpace::default())
                    .ptr_type(AddressSpace::default())
                    .into()
            } else {
                self.llvm_type(ty, ns)
                    .ptr_type(AddressSpace::default())
                    .into()
            });
        }

        // On Solana, we need to pass around the accounts
        if ns.target == Target::Solana {
            args.push(
                self.module
                    .get_struct_type("struct.SolParameters")
                    .unwrap()
                    .ptr_type(AddressSpace::default())
                    .into(),
            );
        }

        // Solana return type should be 64 bit, 32 bit on wasm
        self.return_values[&ReturnCode::Success]
            .get_type()
            .fn_type(&args, false)
    }

    // Create the llvm intrinsic for counting leading zeros
    pub fn llvm_ctlz(&self, bit: u32) -> FunctionValue<'a> {
        let name = format!("llvm.ctlz.i{bit}");
        let ty = self.context.custom_width_int_type(bit);

        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        self.module.add_function(
            &name,
            ty.fn_type(&[ty.into(), self.context.bool_type().into()], false),
            None,
        )
    }

    // Create the llvm intrinsic for bswap
    pub fn llvm_bswap(&self, bit: u32) -> FunctionValue<'a> {
        let name = format!("llvm.bswap.i{bit}");
        let ty = self.context.custom_width_int_type(bit);

        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        self.module
            .add_function(&name, ty.fn_type(&[ty.into()], false), None)
    }

    // Create the llvm intrinsic for overflows
    pub fn llvm_overflow(
        &self,
        ret_ty: BasicTypeEnum<'a>,
        ty: IntType<'a>,
        signed: bool,
        op: BinaryOp,
    ) -> FunctionValue<'a> {
        let bit = ty.get_bit_width();
        let name = format!(
            "llvm.{}{}.with.overflow.i{}",
            if signed { "s" } else { "u" },
            op,
            bit,
        );

        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        self.module
            .add_function(&name, ret_ty.fn_type(&[ty.into(), ty.into()], false), None)
    }

    /// Return the llvm type for a variable holding the type, not the type itself
    pub(crate) fn llvm_var_ty(&self, ty: &Type, ns: &Namespace) -> BasicTypeEnum<'a> {
        let llvm_ty = self.llvm_type(ty, ns);
        match ty.deref_memory() {
            Type::Struct(_)
            | Type::Array(..)
            | Type::DynamicBytes
            | Type::String
            | Type::ExternalFunction { .. } => llvm_ty
                .ptr_type(AddressSpace::default())
                .as_basic_type_enum(),
            _ => llvm_ty,
        }
    }

    /// Default empty value
    pub(crate) fn default_value(&self, ty: &Type, ns: &Namespace) -> BasicValueEnum<'a> {
        let llvm_ty = self.llvm_var_ty(ty, ns);

        // const_zero() on BasicTypeEnum yet. Should be coming to inkwell soon
        if llvm_ty.is_pointer_type() {
            llvm_ty.into_pointer_type().const_null().into()
        } else if llvm_ty.is_array_type() {
            self.address_type(ns).const_zero().into()
        } else {
            llvm_ty.into_int_type().const_zero().into()
        }
    }

    /// Return the llvm type for field in struct or array
    pub(crate) fn llvm_field_ty(&self, ty: &Type, ns: &Namespace) -> BasicTypeEnum<'a> {
        let llvm_ty = self.llvm_type(ty, ns);
        match ty.deref_memory() {
            Type::Array(_, dim) if dim.last() == Some(&ArrayLength::Dynamic) => llvm_ty
                .ptr_type(AddressSpace::default())
                .as_basic_type_enum(),
            Type::DynamicBytes | Type::String => llvm_ty
                .ptr_type(AddressSpace::default())
                .as_basic_type_enum(),
            _ => llvm_ty,
        }
    }

    /// Return the llvm type for the resolved type.
    pub(crate) fn llvm_type(&self, ty: &Type, ns: &Namespace) -> BasicTypeEnum<'a> {
        if ty.is_builtin_struct() == Some(StructType::AccountInfo) {
            return self
                .module
                .get_struct_type("struct.SolAccountInfo")
                .unwrap()
                .into();
        } else {
            match ty {
                Type::Bool => BasicTypeEnum::IntType(self.context.bool_type()),
                Type::Int(n) | Type::Uint(n) => {
                    BasicTypeEnum::IntType(self.context.custom_width_int_type(*n as u32))
                }
                Type::Value => BasicTypeEnum::IntType(
                    self.context
                        .custom_width_int_type(ns.value_length as u32 * 8),
                ),
                Type::Contract(_) | Type::Address(_) => {
                    BasicTypeEnum::ArrayType(self.address_type(ns))
                }
                Type::Bytes(n) => {
                    BasicTypeEnum::IntType(self.context.custom_width_int_type(*n as u32 * 8))
                }
                Type::Enum(n) => self.llvm_type(&ns.enums[*n].ty, ns),
                Type::String | Type::DynamicBytes => {
                    self.module.get_struct_type("struct.vector").unwrap().into()
                }
                Type::Array(base_ty, dims) => {
                    let ty = self.llvm_field_ty(base_ty, ns);

                    let mut dims = dims.iter();

                    let mut aty = match dims.next().unwrap() {
                        ArrayLength::Fixed(d) => ty.array_type(d.to_u32().unwrap()),
                        ArrayLength::Dynamic => {
                            return self.module.get_struct_type("struct.vector").unwrap().into()
                        }
                        ArrayLength::AnyFixed => {
                            unreachable!()
                        }
                    };

                    for dim in dims {
                        match dim {
                            ArrayLength::Fixed(d) => aty = aty.array_type(d.to_u32().unwrap()),
                            ArrayLength::Dynamic => {
                                return self.module.get_struct_type("struct.vector").unwrap().into()
                            }
                            ArrayLength::AnyFixed => {
                                unreachable!()
                            }
                        }
                    }

                    BasicTypeEnum::ArrayType(aty)
                }
                Type::Struct(StructType::SolParameters) => self
                    .module
                    .get_struct_type("struct.SolParameters")
                    .unwrap()
                    .as_basic_type_enum(),
                Type::Struct(str_ty) => self
                    .context
                    .struct_type(
                        &str_ty
                            .definition(ns)
                            .fields
                            .iter()
                            .map(|f| self.llvm_field_ty(&f.ty, ns))
                            .collect::<Vec<BasicTypeEnum>>(),
                        false,
                    )
                    .as_basic_type_enum(),
                Type::Mapping(..) => self.llvm_type(&ns.storage_type(), ns),
                Type::Ref(r) => self
                    .llvm_type(r, ns)
                    .ptr_type(AddressSpace::default())
                    .as_basic_type_enum(),
                Type::StorageRef(..) => self.llvm_type(&ns.storage_type(), ns),
                Type::InternalFunction {
                    params, returns, ..
                } => {
                    let ftype = self.function_type(params, returns, ns);

                    BasicTypeEnum::PointerType(ftype.ptr_type(AddressSpace::default()))
                }
                Type::ExternalFunction { .. } => {
                    let address = self.llvm_type(&Type::Address(false), ns);
                    let selector = self.llvm_type(&Type::FunctionSelector, ns);
                    self.context
                        .struct_type(&[selector, address], false)
                        .as_basic_type_enum()
                }
                Type::Slice(ty) => BasicTypeEnum::StructType(
                    self.context.struct_type(
                        &[
                            self.llvm_type(ty, ns)
                                .ptr_type(AddressSpace::default())
                                .into(),
                            self.context
                                .custom_width_int_type(ns.target.ptr_size().into())
                                .into(),
                        ],
                        false,
                    ),
                ),
                Type::UserType(no) => self.llvm_type(&ns.user_types[*no].ty, ns),
                Type::BufferPointer => self
                    .context
                    .i8_type()
                    .ptr_type(AddressSpace::default())
                    .as_basic_type_enum(),
                Type::FunctionSelector => {
                    self.llvm_type(&Type::Bytes(ns.target.selector_length()), ns)
                }
                _ => unreachable!(),
            }
        }
    }

    /// Allocate vector
    pub(crate) fn vector_new(
        &self,
        size: IntValue<'a>,
        elem_size: IntValue<'a>,
        init: Option<&Vec<u8>>,
    ) -> PointerValue<'a> {
        if let Some(init) = init {
            if init.is_empty() {
                return self
                    .module
                    .get_struct_type("struct.vector")
                    .unwrap()
                    .ptr_type(AddressSpace::default())
                    .const_null();
            }
        }

        let init = match init {
            None => self.builder.build_int_to_ptr(
                self.context.i32_type().const_all_ones(),
                self.context.i8_type().ptr_type(AddressSpace::default()),
                "invalid",
            ),
            Some(s) => self.emit_global_string("const_string", s, true),
        };

        self.builder
            .build_call(
                self.module.get_function("vector_new").unwrap(),
                &[size.into(), elem_size.into(), init.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value()
    }

    /// Number of element in a vector
    pub(crate) fn vector_len(&self, vector: BasicValueEnum<'a>) -> IntValue<'a> {
        if vector.is_struct_value() {
            // slice
            let slice = vector.into_struct_value();

            self.builder.build_int_truncate(
                self.builder
                    .build_extract_value(slice, 1, "slice_len")
                    .unwrap()
                    .into_int_value(),
                self.context.i32_type(),
                "len",
            )
        } else {
            // field 0 is the length
            let vector = vector.into_pointer_value();
            let vector_type = self.module.get_struct_type("struct.vector").unwrap();

            let len = unsafe {
                self.builder.build_gep(
                    vector_type,
                    vector,
                    &[
                        self.context.i32_type().const_zero(),
                        self.context.i32_type().const_zero(),
                    ],
                    "vector_len",
                )
            };

            self.builder
                .build_select(
                    self.builder.build_is_null(vector, "vector_is_null"),
                    self.context.i32_type().const_zero(),
                    self.builder
                        .build_load(self.context.i32_type(), len, "vector_len")
                        .into_int_value(),
                    "length",
                )
                .into_int_value()
        }
    }

    /// Return the pointer to the actual bytes in the vector
    pub(crate) fn vector_bytes(&self, vector: BasicValueEnum<'a>) -> PointerValue<'a> {
        if vector.is_struct_value() {
            // slice
            let slice = vector.into_struct_value();
            self.builder
                .build_extract_value(slice, 0, "slice_data")
                .unwrap()
                .into_pointer_value()
        } else {
            let vector_type = self.module.get_struct_type("struct.vector").unwrap();
            unsafe {
                self.builder.build_gep(
                    vector_type,
                    vector.into_pointer_value(),
                    &[
                        self.context.i32_type().const_zero(),
                        self.context.i32_type().const_int(2, false),
                    ],
                    "data",
                )
            }
        }
    }

    /// Dereference an array
    pub(crate) fn array_subscript(
        &self,
        array_ty: &Type,
        array: PointerValue<'a>,
        index: IntValue<'a>,
        ns: &Namespace,
    ) -> PointerValue<'a> {
        match array_ty {
            Type::Array(_, dim) => {
                if matches!(dim.last(), Some(ArrayLength::Fixed(_))) {
                    // fixed size array
                    let llvm_ty = self.llvm_type(array_ty, ns);
                    unsafe {
                        self.builder.build_gep(
                            llvm_ty,
                            array,
                            &[self.context.i32_type().const_zero(), index],
                            "index_access",
                        )
                    }
                } else {
                    let elem_ty = array_ty.array_deref();
                    let llvm_elem_ty = self.llvm_type(elem_ty.deref_memory(), ns);

                    // dynamic length array or vector
                    let index = self.builder.build_int_mul(
                        index,
                        llvm_elem_ty
                            .size_of()
                            .unwrap()
                            .const_cast(self.context.i32_type(), false),
                        "",
                    );

                    let vector_type = self.module.get_struct_type("struct.vector").unwrap();

                    unsafe {
                        self.builder.build_gep(
                            vector_type,
                            array,
                            &[
                                self.context.i32_type().const_zero(),
                                self.context.i32_type().const_int(2, false),
                                index,
                            ],
                            "index_access",
                        )
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}

/// Return the stdlib as parsed llvm module. The solidity standard library is hardcoded into
/// the solang library
fn load_stdlib<'a>(context: &'a Context, target: &Target) -> Module<'a> {
    if *target == Target::Solana {
        let memory = MemoryBuffer::create_from_memory_range(BPF_IR[0], "bpf_bc");

        let module = Module::parse_bitcode_from_buffer(&memory, context).unwrap();

        for bc in BPF_IR.iter().skip(1) {
            let memory = MemoryBuffer::create_from_memory_range(bc, "bpf_bc");

            module
                .link_in_module(Module::parse_bitcode_from_buffer(&memory, context).unwrap())
                .unwrap();
        }

        return module;
    }

    let memory = MemoryBuffer::create_from_memory_range(WASM_IR[0], "wasm_bc");

    let module = Module::parse_bitcode_from_buffer(&memory, context).unwrap();

    for bc in WASM_IR.iter().skip(1) {
        let memory = MemoryBuffer::create_from_memory_range(bc, "wasm_bc");

        module
            .link_in_module(Module::parse_bitcode_from_buffer(&memory, context).unwrap())
            .unwrap();
    }

    if let Target::Substrate { .. } = *target {
        let memory = MemoryBuffer::create_from_memory_range(SUBSTRATE_IR, "substrate");

        module
            .link_in_module(Module::parse_bitcode_from_buffer(&memory, context).unwrap())
            .unwrap();

        // substrate does not provide ripemd160
        let memory = MemoryBuffer::create_from_memory_range(RIPEMD160_IR, "ripemd160");

        module
            .link_in_module(Module::parse_bitcode_from_buffer(&memory, context).unwrap())
            .unwrap();
    }

    module
}

static BPF_IR: [&[u8]; 6] = [
    include_bytes!("../../stdlib/bpf/stdlib.bc"),
    include_bytes!("../../stdlib/bpf/bigint.bc"),
    include_bytes!("../../stdlib/bpf/format.bc"),
    include_bytes!("../../stdlib/bpf/solana.bc"),
    include_bytes!("../../stdlib/bpf/ripemd160.bc"),
    include_bytes!("../../stdlib/bpf/heap.bc"),
];

static WASM_IR: [&[u8]; 4] = [
    include_bytes!("../../stdlib/wasm/stdlib.bc"),
    include_bytes!("../../stdlib/wasm/heap.bc"),
    include_bytes!("../../stdlib/wasm/bigint.bc"),
    include_bytes!("../../stdlib/wasm/format.bc"),
];

static RIPEMD160_IR: &[u8] = include_bytes!("../../stdlib/wasm/ripemd160.bc");
static SUBSTRATE_IR: &[u8] = include_bytes!("../../stdlib/wasm/substrate.bc");
