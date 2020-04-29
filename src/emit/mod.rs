use hex;
use parser::ast;
use resolver;
use resolver::cfg;
use resolver::expression::{Expression, StringLocation};
use std::cell::RefCell;
use std::path::Path;
use std::str;

use num_bigint::BigInt;
use num_traits::One;
use num_traits::ToPrimitive;
use std::collections::HashMap;
use std::collections::VecDeque;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::module::{Linkage, Module};
use inkwell::passes::PassManager;
use inkwell::targets::{CodeModel, FileType, RelocMode, Target, TargetTriple};
use inkwell::types::BasicTypeEnum;
use inkwell::types::{BasicType, IntType, StringRadix};
use inkwell::values::{
    ArrayValue, BasicValueEnum, FunctionValue, IntValue, PhiValue, PointerValue,
};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;

mod ethabiencoder;
mod ewasm;
mod sabre;
mod substrate;

use link::link;

lazy_static::lazy_static! {
    static ref LLVM_INIT: () = {
        Target::initialize_webassembly(&Default::default());
    };
}

#[derive(Clone)]
struct Variable<'a> {
    value: BasicValueEnum<'a>,
    stack: bool,
    phi: bool,
}

pub trait TargetRuntime {
    fn abi_decode<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue<'b>,
        spec: &[resolver::Parameter],
    );

    /// Abi encode with optional four bytes selector. The load parameter should be set if the args are
    /// pointers to data, not the actual data  itself.
    fn abi_encode<'b>(
        &self,
        contract: &Contract<'b>,
        selector: Option<u32>,
        load: bool,
        function: FunctionValue,
        args: &[BasicValueEnum<'b>],
        spec: &[resolver::Parameter],
    ) -> (PointerValue<'b>, IntValue<'b>);

    // Access storage
    fn clear_storage<'a>(
        &self,
        contract: &'a Contract,
        function: FunctionValue,
        slot: PointerValue<'a>,
    );

    fn set_storage<'a>(
        &self,
        contract: &'a Contract,
        function: FunctionValue,
        slot: PointerValue<'a>,
        dest: PointerValue<'a>,
    );
    fn get_storage_int<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: PointerValue,
        ty: IntType<'a>,
    ) -> IntValue<'a>;

    // Bytes and string have special storage layout
    fn set_storage_string<'a>(
        &self,
        contract: &'a Contract,
        function: FunctionValue,
        slot: PointerValue<'a>,
        dest: PointerValue<'a>,
    );
    fn get_storage_string<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> PointerValue<'a>;
    fn get_storage_bytes_subscript<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        index: IntValue<'a>,
    ) -> IntValue<'a>;
    fn set_storage_bytes_subscript<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        index: IntValue<'a>,
        value: IntValue<'a>,
    );
    fn storage_bytes_push<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        val: IntValue<'a>,
    );
    fn storage_bytes_pop<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> IntValue<'a>;
    fn storage_string_length<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> IntValue<'a>;

    /// keccak256 hash
    fn keccak256_hash(
        &self,
        contract: &Contract,
        src: PointerValue,
        length: IntValue,
        dest: PointerValue,
    );

    /// Prints a string
    fn print(&self, contract: &Contract, string: PointerValue, length: IntValue);

    /// Return success without any result
    fn return_empty_abi(&self, contract: &Contract);

    /// Return success with the ABI encoded result
    fn return_abi<'b>(&self, contract: &'b Contract, data: PointerValue<'b>, length: IntValue);

    /// Return failure without any result
    fn assert_failure<'b>(&self, contract: &'b Contract, data: PointerValue, length: IntValue);

    /// Calls constructor
    fn create_contract<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        contract_no: usize,
        constructor_no: usize,
        address: PointerValue<'b>,
        args: &[BasicValueEnum<'b>],
    );

    /// call external function
    fn external_call<'b>(
        &self,
        contract: &Contract<'b>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        address: PointerValue<'b>,
    ) -> IntValue<'b>;

    fn return_data<'b>(&self, contract: &Contract<'b>) -> (PointerValue<'b>, IntValue<'b>);
}

pub struct Contract<'a> {
    pub name: String,
    pub module: Module<'a>,
    pub runtime: Option<Box<Contract<'a>>>,
    builder: Builder<'a>,
    context: &'a Context,
    triple: TargetTriple,
    contract: &'a resolver::Contract,
    ns: &'a resolver::Namespace,
    constructors: Vec<FunctionValue<'a>>,
    functions: Vec<FunctionValue<'a>>,
    wasm: RefCell<Vec<u8>>,
    opt: OptimizationLevel,
    code_size: RefCell<Option<IntValue<'a>>>,
}

impl<'a> Contract<'a> {
    /// Build the LLVM IR for a contract
    pub fn build(
        context: &'a Context,
        contract: &'a resolver::Contract,
        ns: &'a resolver::Namespace,
        filename: &'a str,
        opt: OptimizationLevel,
    ) -> Self {
        match ns.target {
            super::Target::Substrate => {
                substrate::SubstrateTarget::build(context, contract, ns, filename, opt)
            }
            super::Target::Ewasm => ewasm::EwasmTarget::build(context, contract, ns, filename, opt),
            super::Target::Sabre => sabre::SabreTarget::build(context, contract, ns, filename, opt),
        }
    }

    /// Compile the contract to wasm and return the wasm as bytes. The result is
    /// cached, since this function can be called multiple times (e.g. one for
    /// each time a contract of this type is created).
    pub fn wasm(&self, linking: bool) -> Result<Vec<u8>, String> {
        {
            let wasm = self.wasm.borrow();

            if !wasm.is_empty() {
                return Ok(wasm.clone());
            }
        }

        match self.opt {
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

        let target = Target::from_name("wasm32").unwrap();

        let target_machine = target
            .create_target_machine(
                &self.triple,
                "",
                "",
                self.opt,
                RelocMode::Default,
                CodeModel::Default,
            )
            .unwrap();

        loop {
            // we need to loop here to support ewasm deployer. It needs to know the size
            // of itself. Note that in webassembly, the constants are LEB128 encoded so
            // patching the length might actually change the length. So we need to loop
            // until it is right.

            // The correct solution is to make ewasm less insane.
            match target_machine.write_to_memory_buffer(&self.module, FileType::Object) {
                Ok(out) => {
                    let slice = out.as_slice();

                    if linking {
                        let bs = link(slice, self.ns.target);

                        if !self.patch_code_size(bs.len() as u64) {
                            self.wasm.replace(bs.to_vec());

                            return Ok(bs.to_vec());
                        }
                    } else {
                        self.wasm.replace(slice.to_vec());

                        return Ok(slice.to_vec());
                    }
                }
                Err(s) => {
                    return Err(s.to_string());
                }
            }
        }
    }

    /// Mark all functions as internal unless they're in the export_list. This helps the
    /// llvm globaldce pass eliminate unnecessary functions and reduce the wasm output.
    fn internalize(&self, export_list: &[&str]) {
        let mut func = self.module.get_first_function();

        // FIXME: these functions are called from code generated by lowering into wasm,
        // so eliminating them now will cause link errors. Either we should prevent these
        // calls from being done in the first place or do dce at link time
        let mut export_list = export_list.to_vec();
        export_list.push("__ashlti3");
        export_list.push("__lshrti3");

        while let Some(f) = func {
            let name = f.get_name().to_str().unwrap();

            if !name.starts_with("llvm.") && export_list.iter().all(|e| e != &name) {
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
        contract: &'a resolver::Contract,
        ns: &'a resolver::Namespace,
        filename: &'a str,
        opt: OptimizationLevel,
        runtime: Option<Box<Contract<'a>>>,
    ) -> Self {
        lazy_static::initialize(&LLVM_INIT);

        let triple = TargetTriple::create("wasm32-unknown-unknown-wasm");
        let module = context.create_module(&contract.name);

        module.set_triple(&triple);
        module.set_source_file_name(filename);

        // stdlib
        let intr = load_stdlib(&context, &ns.target);
        module.link_in_module(intr).unwrap();

        Contract {
            name: contract.name.to_owned(),
            module,
            runtime,
            builder: context.create_builder(),
            triple,
            context,
            contract,
            ns,
            constructors: Vec::new(),
            functions: Vec::new(),
            wasm: RefCell::new(Vec::new()),
            opt,
            code_size: RefCell::new(None),
        }
    }

    /// Creates global string in the llvm module with initializer
    ///
    fn emit_global_string(&self, name: &str, data: &[u8], constant: bool) -> PointerValue<'a> {
        let ty = self.context.i8_type().array_type(data.len() as u32);

        let gv = self
            .module
            .add_global(ty, Some(AddressSpace::Generic), name);

        gv.set_linkage(Linkage::Internal);

        gv.set_initializer(&self.context.const_string(data, false));

        if constant {
            gv.set_constant(true);
        }

        self.builder.build_pointer_cast(
            gv.as_pointer_value(),
            self.context.i8_type().ptr_type(AddressSpace::Generic),
            name,
        )
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

    /// Emit all functions and constructors. First emit the functions, then the bodies
    fn emit_functions(&mut self, runtime: &dyn TargetRuntime) {
        let mut defines = Vec::new();

        for resolver_func in &self.contract.functions {
            let name = if resolver_func.name != "" {
                format!("sol::function::{}", resolver_func.wasm_symbol(self.ns))
            } else {
                "sol::fallback".to_owned()
            };

            let func_decl = self.declare_function(&name, resolver_func);
            self.functions.push(func_decl);

            defines.push((func_decl, resolver_func));
        }

        for resolver_func in &self.contract.constructors {
            let func_decl = self.declare_function(
                &format!("sol::constructor{}", resolver_func.wasm_symbol(self.ns)),
                resolver_func,
            );

            self.constructors.push(func_decl);
            defines.push((func_decl, resolver_func));
        }

        for (func_decl, resolver_func) in defines {
            self.define_function(resolver_func, func_decl, runtime);
        }
    }

    /// The expression function recursively emits code for expressions. The BasicEnumValue it
    /// returns depends on the context; if it is simple integer, bool or bytes32 expression, the value
    /// is an Intvalue. For references to arrays, it is a PointerValue to the array. For references
    /// to storage, it is the storage slot. The references types are dereferenced by the Expression::Load()
    /// and Expression::StorageLoad() expression types.
    fn expression(
        &self,
        e: &Expression,
        vartab: &[Variable<'a>],
        function: FunctionValue<'a>,
        runtime: &dyn TargetRuntime,
    ) -> BasicValueEnum<'a> {
        match e {
            Expression::BoolLiteral(_, val) => self
                .context
                .bool_type()
                .const_int(*val as u64, false)
                .into(),
            Expression::NumberLiteral(_, bits, n) => self.number_literal(*bits as u32, n).into(),
            Expression::StructLiteral(_, ty, exprs) => {
                let struct_ty = self.llvm_type(ty);

                let s = self
                    .builder
                    .build_call(
                        self.module.get_function("__malloc").unwrap(),
                        &[struct_ty
                            .size_of()
                            .unwrap()
                            .const_cast(self.context.i32_type(), false)
                            .into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let s = self.builder.build_pointer_cast(
                    s,
                    struct_ty.ptr_type(AddressSpace::Generic),
                    "struct_literal",
                );

                for (i, f) in exprs.iter().enumerate() {
                    let elem = unsafe {
                        self.builder.build_gep(
                            s,
                            &[
                                self.context.i32_type().const_zero(),
                                self.context.i32_type().const_int(i as u64, false),
                            ],
                            "struct member",
                        )
                    };

                    self.builder
                        .build_store(elem, self.expression(f, vartab, function, runtime));
                }

                s.into()
            }
            Expression::BytesLiteral(_, bs) => {
                let ty = self.context.custom_width_int_type((bs.len() * 8) as u32);

                // hex"11223344" should become i32 0x11223344
                let s = hex::encode(bs);

                ty.const_int_from_string(&s, StringRadix::Hexadecimal)
                    .unwrap()
                    .into()
            }
            Expression::Add(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_int_add(left, right, "").into()
            }
            Expression::Subtract(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_int_sub(left, right, "").into()
            }
            Expression::Multiply(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                let bits = left.get_type().get_bit_width();

                if bits > 64 {
                    let l = self.builder.build_alloca(left.get_type(), "");
                    let r = self.builder.build_alloca(left.get_type(), "");
                    let o = self.builder.build_alloca(left.get_type(), "");

                    self.builder.build_store(l, left);
                    self.builder.build_store(r, right);

                    self.builder.build_call(
                        self.module.get_function("__mul32").unwrap(),
                        &[
                            self.builder
                                .build_pointer_cast(
                                    l,
                                    self.context.i32_type().ptr_type(AddressSpace::Generic),
                                    "left",
                                )
                                .into(),
                            self.builder
                                .build_pointer_cast(
                                    r,
                                    self.context.i32_type().ptr_type(AddressSpace::Generic),
                                    "right",
                                )
                                .into(),
                            self.builder
                                .build_pointer_cast(
                                    o,
                                    self.context.i32_type().ptr_type(AddressSpace::Generic),
                                    "output",
                                )
                                .into(),
                            self.context
                                .i32_type()
                                .const_int(bits as u64 / 32, false)
                                .into(),
                        ],
                        "",
                    );

                    self.builder.build_load(o, "mul")
                } else {
                    self.builder.build_int_mul(left, right, "").into()
                }
            }
            Expression::UDivide(_, l, r) => {
                let left = self.expression(l, vartab, function, runtime);
                let right = self.expression(r, vartab, function, runtime);

                let bits = left.into_int_value().get_type().get_bit_width();

                if bits > 64 {
                    let f = self.udivmod(bits, runtime);

                    let rem = self
                        .builder
                        .build_alloca(left.into_int_value().get_type(), "");
                    let quotient = self
                        .builder
                        .build_alloca(left.into_int_value().get_type(), "");

                    let ret = self
                        .builder
                        .build_call(f, &[left, right, rem.into(), quotient.into()], "udiv")
                        .try_as_basic_value()
                        .left()
                        .unwrap();

                    let success = self.builder.build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        self.context.i32_type().const_zero(),
                        "success",
                    );

                    let success_block = self.context.append_basic_block(function, "success");
                    let bail_block = self.context.append_basic_block(function, "bail");
                    self.builder
                        .build_conditional_branch(success, success_block, bail_block);

                    self.builder.position_at_end(bail_block);

                    self.builder.build_return(Some(&ret));
                    self.builder.position_at_end(success_block);

                    self.builder.build_load(quotient, "quotient")
                } else {
                    self.builder
                        .build_int_unsigned_div(left.into_int_value(), right.into_int_value(), "")
                        .into()
                }
            }
            Expression::SDivide(_, l, r) => {
                let left = self.expression(l, vartab, function, runtime);
                let right = self.expression(r, vartab, function, runtime);

                let bits = left.into_int_value().get_type().get_bit_width();

                if bits > 64 {
                    let f = self.sdivmod(bits, runtime);

                    let rem = self.builder.build_alloca(left.get_type(), "");
                    let quotient = self.builder.build_alloca(left.get_type(), "");

                    let ret = self
                        .builder
                        .build_call(f, &[left, right, rem.into(), quotient.into()], "udiv")
                        .try_as_basic_value()
                        .left()
                        .unwrap();

                    let success = self.builder.build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        self.context.i32_type().const_zero(),
                        "success",
                    );

                    let success_block = self.context.append_basic_block(function, "success");
                    let bail_block = self.context.append_basic_block(function, "bail");
                    self.builder
                        .build_conditional_branch(success, success_block, bail_block);

                    self.builder.position_at_end(bail_block);

                    self.builder.build_return(Some(&ret));
                    self.builder.position_at_end(success_block);

                    self.builder.build_load(quotient, "quotient")
                } else {
                    self.builder
                        .build_int_signed_div(left.into_int_value(), right.into_int_value(), "")
                        .into()
                }
            }
            Expression::UModulo(_, l, r) => {
                let left = self.expression(l, vartab, function, runtime);
                let right = self.expression(r, vartab, function, runtime);

                let bits = left.into_int_value().get_type().get_bit_width();

                if bits > 64 {
                    let f = self.udivmod(bits, runtime);

                    let rem = self
                        .builder
                        .build_alloca(left.into_int_value().get_type(), "");
                    let quotient = self
                        .builder
                        .build_alloca(left.into_int_value().get_type(), "");

                    let ret = self
                        .builder
                        .build_call(f, &[left, right, rem.into(), quotient.into()], "udiv")
                        .try_as_basic_value()
                        .left()
                        .unwrap();

                    let success = self.builder.build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        self.context.i32_type().const_zero(),
                        "success",
                    );

                    let success_block = self.context.append_basic_block(function, "success");
                    let bail_block = self.context.append_basic_block(function, "bail");
                    self.builder
                        .build_conditional_branch(success, success_block, bail_block);

                    self.builder.position_at_end(bail_block);

                    self.builder.build_return(Some(&ret));
                    self.builder.position_at_end(success_block);

                    self.builder.build_load(rem, "urem")
                } else {
                    self.builder
                        .build_int_unsigned_rem(left.into_int_value(), right.into_int_value(), "")
                        .into()
                }
            }
            Expression::SModulo(_, l, r) => {
                let left = self.expression(l, vartab, function, runtime);
                let right = self.expression(r, vartab, function, runtime);

                let bits = left.into_int_value().get_type().get_bit_width();

                if bits > 64 {
                    let f = self.sdivmod(bits, runtime);
                    let rem = self.builder.build_alloca(left.get_type(), "");
                    let quotient = self.builder.build_alloca(left.get_type(), "");

                    let ret = self
                        .builder
                        .build_call(f, &[left, right, rem.into(), quotient.into()], "sdiv")
                        .try_as_basic_value()
                        .left()
                        .unwrap();

                    let success = self.builder.build_int_compare(
                        IntPredicate::EQ,
                        ret.into_int_value(),
                        self.context.i32_type().const_zero(),
                        "success",
                    );

                    let success_block = self.context.append_basic_block(function, "success");
                    let bail_block = self.context.append_basic_block(function, "bail");
                    self.builder
                        .build_conditional_branch(success, success_block, bail_block);

                    self.builder.position_at_end(bail_block);

                    self.builder.build_return(Some(&ret));
                    self.builder.position_at_end(success_block);

                    self.builder.build_load(rem, "srem")
                } else {
                    self.builder
                        .build_int_signed_rem(left.into_int_value(), right.into_int_value(), "")
                        .into()
                }
            }
            Expression::Power(_, l, r) => {
                let left = self.expression(l, vartab, function, runtime);
                let right = self.expression(r, vartab, function, runtime);

                let bits = left.into_int_value().get_type().get_bit_width();

                let f = self.upower(bits);

                self.builder
                    .build_call(f, &[left, right], "power")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            Expression::Equal(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder
                    .build_int_compare(IntPredicate::EQ, left, right, "")
                    .into()
            }
            Expression::NotEqual(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder
                    .build_int_compare(IntPredicate::NE, left, right, "")
                    .into()
            }
            Expression::SMore(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder
                    .build_int_compare(IntPredicate::SGT, left, right, "")
                    .into()
            }
            Expression::SMoreEqual(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder
                    .build_int_compare(IntPredicate::SGE, left, right, "")
                    .into()
            }
            Expression::SLess(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder
                    .build_int_compare(IntPredicate::SLT, left, right, "")
                    .into()
            }
            Expression::SLessEqual(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder
                    .build_int_compare(IntPredicate::SLE, left, right, "")
                    .into()
            }
            Expression::UMore(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder
                    .build_int_compare(IntPredicate::UGT, left, right, "")
                    .into()
            }
            Expression::UMoreEqual(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder
                    .build_int_compare(IntPredicate::UGE, left, right, "")
                    .into()
            }
            Expression::ULess(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder
                    .build_int_compare(IntPredicate::ULT, left, right, "")
                    .into()
            }
            Expression::ULessEqual(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder
                    .build_int_compare(IntPredicate::ULE, left, right, "")
                    .into()
            }
            Expression::Variable(_, s) => {
                if vartab[*s].stack {
                    self.builder
                        .build_load(vartab[*s].value.into_pointer_value(), "")
                } else {
                    vartab[*s].value
                }
            }
            Expression::Load(_, e) => {
                let expr = self
                    .expression(e, vartab, function, runtime)
                    .into_pointer_value();

                self.builder.build_load(expr, "")
            }
            Expression::StorageLoad(_, ty, e) => {
                // The storage slot is an i256 accessed through a pointer, so we need
                // to store it
                let mut slot = self
                    .expression(e, vartab, function, runtime)
                    .into_int_value();
                let slot_ptr = self.builder.build_alloca(slot.get_type(), "slot");
                self.storage_load(ty, &mut slot, slot_ptr, function, runtime)
            }
            Expression::ZeroExt(_, t, e) => {
                let e = self
                    .expression(e, vartab, function, runtime)
                    .into_int_value();
                let ty = self.llvm_type(t);

                self.builder
                    .build_int_z_extend(e, ty.into_int_type(), "")
                    .into()
            }
            Expression::UnaryMinus(_, e) => {
                let e = self
                    .expression(e, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_int_neg(e, "").into()
            }
            Expression::SignExt(_, t, e) => {
                let e = self
                    .expression(e, vartab, function, runtime)
                    .into_int_value();
                let ty = self.llvm_type(t);

                self.builder
                    .build_int_s_extend(e, ty.into_int_type(), "")
                    .into()
            }
            Expression::Trunc(_, t, e) => {
                let e = self
                    .expression(e, vartab, function, runtime)
                    .into_int_value();
                let ty = self.llvm_type(t);

                self.builder
                    .build_int_truncate(e, ty.into_int_type(), "")
                    .into()
            }
            Expression::Not(_, e) => {
                let e = self
                    .expression(e, vartab, function, runtime)
                    .into_int_value();

                self.builder
                    .build_int_compare(IntPredicate::EQ, e, e.get_type().const_zero(), "")
                    .into()
            }
            Expression::Complement(_, e) => {
                let e = self
                    .expression(e, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_not(e, "").into()
            }
            Expression::Or(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_or(left, right, "").into()
            }
            Expression::And(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_and(left, right, "").into()
            }
            Expression::BitwiseOr(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_or(left, right, "").into()
            }
            Expression::BitwiseAnd(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_and(left, right, "").into()
            }
            Expression::BitwiseXor(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_xor(left, right, "").into()
            }
            Expression::ShiftLeft(_, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_left_shift(left, right, "").into()
            }
            Expression::ShiftRight(_, l, r, signed) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder
                    .build_right_shift(left, right, *signed, "")
                    .into()
            }
            Expression::ArraySubscript(_, a, i) => {
                let array = self
                    .expression(a, vartab, function, runtime)
                    .into_pointer_value();
                let index = self
                    .expression(i, vartab, function, runtime)
                    .into_int_value();

                unsafe {
                    self.builder
                        .build_gep(
                            array,
                            &[self.context.i32_type().const_zero(), index],
                            "index_access",
                        )
                        .into()
                }
            }
            Expression::StorageBytesSubscript(_, a, i) => {
                let index = self
                    .expression(i, vartab, function, runtime)
                    .into_int_value();
                let slot = self
                    .expression(a, vartab, function, runtime)
                    .into_int_value();
                let slot_ptr = self.builder.build_alloca(slot.get_type(), "slot");
                self.builder.build_store(slot_ptr, slot);
                runtime
                    .get_storage_bytes_subscript(&self, function, slot_ptr, index)
                    .into()
            }
            Expression::StorageBytesPush(_, a, v) => {
                let val = self
                    .expression(v, vartab, function, runtime)
                    .into_int_value();
                let slot = self
                    .expression(a, vartab, function, runtime)
                    .into_int_value();
                let slot_ptr = self.builder.build_alloca(slot.get_type(), "slot");
                self.builder.build_store(slot_ptr, slot);
                runtime.storage_bytes_push(&self, function, slot_ptr, val);

                val.into()
            }
            Expression::StorageBytesPop(_, a) => {
                let slot = self
                    .expression(a, vartab, function, runtime)
                    .into_int_value();
                let slot_ptr = self.builder.build_alloca(slot.get_type(), "slot");
                self.builder.build_store(slot_ptr, slot);
                runtime.storage_bytes_pop(&self, function, slot_ptr).into()
            }
            Expression::StorageBytesLength(_, a) => {
                let slot = self
                    .expression(a, vartab, function, runtime)
                    .into_int_value();
                let slot_ptr = self.builder.build_alloca(slot.get_type(), "slot");
                self.builder.build_store(slot_ptr, slot);
                runtime
                    .storage_string_length(&self, function, slot_ptr)
                    .into()
            }
            Expression::DynamicArraySubscript(_, a, elem_ty, i) => {
                let array = self
                    .expression(a, vartab, function, runtime)
                    .into_pointer_value();

                let ty = self.llvm_var(elem_ty);
                let index = self.builder.build_int_mul(
                    self.expression(i, vartab, function, runtime)
                        .into_int_value(),
                    ty.into_pointer_type()
                        .get_element_type()
                        .size_of()
                        .unwrap()
                        .const_cast(self.context.i32_type(), false),
                    "",
                );

                let elem = unsafe {
                    self.builder.build_gep(
                        array,
                        &[
                            self.context.i32_type().const_zero(),
                            self.context.i32_type().const_int(2, false),
                            index,
                        ],
                        "index_access",
                    )
                };

                self.builder
                    .build_pointer_cast(elem, ty.into_pointer_type(), "elem")
                    .into()
            }
            Expression::StructMember(_, a, i) => {
                let array = self
                    .expression(a, vartab, function, runtime)
                    .into_pointer_value();

                unsafe {
                    self.builder
                        .build_gep(
                            array,
                            &[
                                self.context.i32_type().const_zero(),
                                self.context.i32_type().const_int(*i as u64, false),
                            ],
                            "struct member",
                        )
                        .into()
                }
            }
            Expression::Ternary(_, c, l, r) => {
                let cond = self
                    .expression(c, vartab, function, runtime)
                    .into_int_value();
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_select(cond, left, right, "")
            }
            Expression::ConstArrayLiteral(_, dims, exprs) => {
                // For const arrays (declared with "constant" keyword, we should create a global constant
                let mut dims = dims.iter();

                let exprs = exprs
                    .iter()
                    .map(|e| {
                        self.expression(e, vartab, function, runtime)
                            .into_int_value()
                    })
                    .collect::<Vec<IntValue>>();
                let ty = exprs[0].get_type();

                let top_size = *dims.next().unwrap();

                // Create a vector of ArrayValues
                let mut arrays = exprs
                    .chunks(top_size as usize)
                    .map(|a| ty.const_array(a))
                    .collect::<Vec<ArrayValue>>();

                let mut ty = ty.array_type(top_size);

                // for each dimension, split the array into futher arrays
                for d in dims {
                    ty = ty.array_type(*d);

                    arrays = arrays
                        .chunks(*d as usize)
                        .map(|a| ty.const_array(a))
                        .collect::<Vec<ArrayValue>>();
                }

                // We actually end up with an array with a single entry

                // now we've created the type, and the const array. Put it into a global
                let gv =
                    self.module
                        .add_global(ty, Some(AddressSpace::Generic), "const_array_literal");

                gv.set_linkage(Linkage::Internal);

                gv.set_initializer(&arrays[0]);
                gv.set_constant(true);

                gv.as_pointer_value().into()
            }
            Expression::ArrayLiteral(_, ty, dims, exprs) => {
                // non-const array literals should alloca'ed and each element assigned
                let array = self
                    .builder
                    .build_alloca(self.llvm_type(ty), "array_literal");

                for (i, expr) in exprs.iter().enumerate() {
                    let mut ind = vec![self.context.i32_type().const_zero()];

                    let mut e = i as u32;

                    for d in dims {
                        ind.insert(1, self.context.i32_type().const_int((e % *d).into(), false));

                        e /= *d;
                    }

                    let elemptr = unsafe {
                        self.builder
                            .build_gep(array, &ind, &format!("elemptr{}", i))
                    };

                    self.builder
                        .build_store(elemptr, self.expression(expr, vartab, function, runtime));
                }

                array.into()
            }
            Expression::AllocDynamicArray(_, ty, size, init) => {
                let elem = ty.array_deref();

                let size = self
                    .expression(size, vartab, function, runtime)
                    .into_int_value();

                let elem_size = self
                    .llvm_type(&elem)
                    .size_of()
                    .unwrap()
                    .const_cast(self.context.i32_type(), false);

                let init = match init {
                    None => self.builder.build_int_to_ptr(
                        self.context.i32_type().const_all_ones(),
                        self.context.i8_type().ptr_type(AddressSpace::Generic),
                        "invalid",
                    ),
                    Some(s) => self.emit_global_string("const_string", s, false),
                };

                let v = self
                    .builder
                    .build_call(
                        self.module.get_function("vector_new").unwrap(),
                        &[size.into(), elem_size.into(), init.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                self.builder
                    .build_pointer_cast(
                        v.into_pointer_value(),
                        self.module
                            .get_type("struct.vector")
                            .unwrap()
                            .ptr_type(AddressSpace::Generic),
                        "vector",
                    )
                    .into()
            }
            Expression::DynamicArrayLength(_, a) => {
                let array = self
                    .expression(a, vartab, function, runtime)
                    .into_pointer_value();

                // field 0 is the length
                let len = unsafe {
                    self.builder.build_gep(
                        array,
                        &[
                            self.context.i32_type().const_zero(),
                            self.context.i32_type().const_zero(),
                        ],
                        "array_len",
                    )
                };

                self.builder.build_load(len, "array_len")
            }
            Expression::Keccak256(_, exprs) => {
                let mut length = self.context.i32_type().const_zero();
                let mut values: Vec<(BasicValueEnum, IntValue, resolver::Type)> = Vec::new();

                // first we need to calculate the length of the buffer and get the types/lengths
                for e in exprs {
                    let v = self.expression(&e.0, vartab, function, runtime);

                    let len = match e.1 {
                        resolver::Type::DynamicBytes | resolver::Type::String => {
                            // field 0 is the length
                            let array_len = unsafe {
                                self.builder.build_gep(
                                    v.into_pointer_value(),
                                    &[
                                        self.context.i32_type().const_zero(),
                                        self.context.i32_type().const_zero(),
                                    ],
                                    "array_len",
                                )
                            };

                            self.builder
                                .build_load(array_len, "array_len")
                                .into_int_value()
                        }
                        _ => v
                            .get_type()
                            .size_of()
                            .unwrap()
                            .const_cast(self.context.i32_type(), false),
                    };

                    length = self.builder.build_int_add(length, len, "");

                    values.push((v, len, e.1.clone()));
                }

                //  now allocate a buffer
                let src =
                    self.builder
                        .build_array_alloca(self.context.i8_type(), length, "keccak_src");

                // fill in all the fields
                let mut offset = self.context.i32_type().const_zero();

                for (v, len, ty) in values {
                    let elem = unsafe { self.builder.build_gep(src, &[offset], "elem") };

                    offset = self.builder.build_int_add(offset, len, "");

                    match ty {
                        resolver::Type::DynamicBytes | resolver::Type::String => {
                            let data = unsafe {
                                self.builder.build_gep(
                                    v.into_pointer_value(),
                                    &[
                                        self.context.i32_type().const_zero(),
                                        self.context.i32_type().const_int(2, false),
                                    ],
                                    "",
                                )
                            };

                            self.builder.build_call(
                                self.module.get_function("__memcpy").unwrap(),
                                &[
                                    elem.into(),
                                    self.builder
                                        .build_pointer_cast(
                                            data,
                                            self.context.i8_type().ptr_type(AddressSpace::Generic),
                                            "data",
                                        )
                                        .into(),
                                    len.into(),
                                ],
                                "",
                            );
                        }
                        _ => {
                            let elem = self.builder.build_pointer_cast(
                                elem,
                                v.get_type().ptr_type(AddressSpace::Generic),
                                "",
                            );

                            self.builder.build_store(elem, v);
                        }
                    }
                }
                let dst = self
                    .builder
                    .build_alloca(self.context.custom_width_int_type(256), "keccak_dst");

                runtime.keccak256_hash(&self, src, length, dst);

                self.builder.build_load(dst, "keccak256_hash")
            }
            Expression::StringCompare(_, l, r) => {
                let (left, left_len) = self.string_location(l, vartab, function, runtime);
                let (right, right_len) = self.string_location(r, vartab, function, runtime);

                self.builder
                    .build_call(
                        self.module.get_function("memcmp").unwrap(),
                        &[left.into(), left_len.into(), right.into(), right_len.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }
            Expression::StringConcat(_, l, r) => {
                let (left, left_len) = self.string_location(l, vartab, function, runtime);
                let (right, right_len) = self.string_location(r, vartab, function, runtime);

                let v = self
                    .builder
                    .build_call(
                        self.module.get_function("concat").unwrap(),
                        &[left.into(), left_len.into(), right.into(), right_len.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                self.builder
                    .build_pointer_cast(
                        v.into_pointer_value(),
                        self.module
                            .get_type("struct.vector")
                            .unwrap()
                            .ptr_type(AddressSpace::Generic),
                        "vector",
                    )
                    .into()
            }
            Expression::Poison => unreachable!(),
            Expression::Unreachable => unreachable!(),
        }
    }

    /// Load a string from expression or create global
    fn string_location(
        &self,
        location: &StringLocation,
        vartab: &[Variable<'a>],
        function: FunctionValue<'a>,
        runtime: &dyn TargetRuntime,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        match location {
            StringLocation::CompileTime(literal) => (
                self.emit_global_string("const_string", literal, false),
                self.context
                    .i32_type()
                    .const_int(literal.len() as u64, false),
            ),
            StringLocation::RunTime(e) => {
                let v = self
                    .expression(e, vartab, function, runtime)
                    .into_pointer_value();

                let data = unsafe {
                    self.builder.build_gep(
                        v,
                        &[
                            self.context.i32_type().const_zero(),
                            self.context.i32_type().const_int(2, false),
                        ],
                        "data",
                    )
                };

                let data_len = unsafe {
                    self.builder.build_gep(
                        v,
                        &[
                            self.context.i32_type().const_zero(),
                            self.context.i32_type().const_zero(),
                        ],
                        "data_len",
                    )
                };

                (
                    self.builder.build_pointer_cast(
                        data,
                        self.context.i8_type().ptr_type(AddressSpace::Generic),
                        "data",
                    ),
                    self.builder
                        .build_load(data_len, "data_len")
                        .into_int_value(),
                )
            }
        }
    }

    /// Convert a BigInt number to llvm const value
    fn number_literal(&self, bits: u32, n: &BigInt) -> IntValue<'a> {
        let ty = self.context.custom_width_int_type(bits);
        let s = n.to_string();

        ty.const_int_from_string(&s, StringRadix::Decimal).unwrap()
    }

    /// Recursively load a type from contract storage
    fn storage_load(
        &self,
        ty: &resolver::Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        function: FunctionValue,
        runtime: &dyn TargetRuntime,
    ) -> BasicValueEnum<'a> {
        match ty {
            resolver::Type::Ref(ty) => self.storage_load(ty, slot, slot_ptr, function, runtime),
            resolver::Type::Array(_, dim) => {
                if let Some(d) = &dim[0] {
                    let llvm_ty = self.llvm_type(ty.deref());
                    // LLVMSizeOf() produces an i64
                    let size = self.builder.build_int_truncate(
                        llvm_ty.size_of().unwrap(),
                        self.context.i32_type(),
                        "size_of",
                    );

                    let ty = ty.array_deref();

                    let new = self
                        .builder
                        .build_call(
                            self.module.get_function("__malloc").unwrap(),
                            &[size.into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let dest = self.builder.build_pointer_cast(
                        new,
                        llvm_ty.ptr_type(AddressSpace::Generic),
                        "dest",
                    );

                    self.emit_static_loop_with_int(
                        function,
                        self.context.i64_type().const_zero(),
                        self.context
                            .i64_type()
                            .const_int(d.to_u64().unwrap(), false),
                        slot,
                        |index: IntValue<'a>, slot: &mut IntValue<'a>| {
                            let elem = unsafe {
                                self.builder.build_gep(
                                    dest,
                                    &[self.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            let val = self.storage_load(&ty, slot, slot_ptr, function, runtime);

                            self.builder.build_store(elem, val);
                        },
                    );

                    dest.into()
                } else {
                    // iterate over dynamic array
                    let slot_ty = resolver::Type::Uint(256);

                    let size = self.builder.build_int_truncate(
                        self.storage_load(&slot_ty, slot, slot_ptr, function, runtime)
                            .into_int_value(),
                        self.context.i32_type(),
                        "size",
                    );

                    let elem_ty = self.llvm_type(&ty.array_elem());
                    let elem_size = self.builder.build_int_truncate(
                        elem_ty.size_of().unwrap(),
                        self.context.i32_type(),
                        "size_of",
                    );
                    let init = self.builder.build_int_to_ptr(
                        self.context.i32_type().const_all_ones(),
                        self.context.i8_type().ptr_type(AddressSpace::Generic),
                        "invalid",
                    );

                    let dest = self
                        .builder
                        .build_call(
                            self.module.get_function("vector_new").unwrap(),
                            &[size.into(), elem_size.into(), init.into()],
                            "",
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // get the slot for the elements
                    // this hashes in-place
                    runtime.keccak256_hash(
                        &self,
                        slot_ptr,
                        slot.get_type()
                            .size_of()
                            .const_cast(self.context.i32_type(), false),
                        slot_ptr,
                    );

                    let mut elem_slot = self
                        .builder
                        .build_load(slot_ptr, "elem_slot")
                        .into_int_value();

                    self.emit_loop_cond_first_with_int(
                        function,
                        self.context.i32_type().const_zero(),
                        size,
                        &mut elem_slot,
                        |elem_no: IntValue<'a>, slot: &mut IntValue<'a>| {
                            let index = self.builder.build_int_mul(elem_no, elem_size, "");

                            let entry = self.storage_load(
                                &ty.array_elem(),
                                slot,
                                slot_ptr,
                                function,
                                runtime,
                            );

                            let data = unsafe {
                                self.builder.build_gep(
                                    dest,
                                    &[
                                        self.context.i32_type().const_zero(),
                                        self.context.i32_type().const_int(2, false),
                                        index,
                                    ],
                                    "data",
                                )
                            };

                            self.builder.build_store(
                                self.builder.build_pointer_cast(
                                    data,
                                    elem_ty.ptr_type(AddressSpace::Generic),
                                    "entry",
                                ),
                                entry,
                            );
                        },
                    );
                    // load
                    dest.into()
                }
            }
            resolver::Type::Struct(n) => {
                let llvm_ty = self.llvm_type(ty.deref());
                // LLVMSizeOf() produces an i64
                let size = self.builder.build_int_truncate(
                    llvm_ty.size_of().unwrap(),
                    self.context.i32_type(),
                    "size_of",
                );

                let new = self
                    .builder
                    .build_call(
                        self.module.get_function("__malloc").unwrap(),
                        &[size.into()],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let dest = self.builder.build_pointer_cast(
                    new,
                    llvm_ty.ptr_type(AddressSpace::Generic),
                    "dest",
                );

                for (i, field) in self.ns.structs[*n].fields.iter().enumerate() {
                    let val = self.storage_load(&field.ty, slot, slot_ptr, function, runtime);

                    let elem = unsafe {
                        self.builder.build_gep(
                            dest,
                            &[
                                self.context.i32_type().const_zero(),
                                self.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    self.builder.build_store(elem, val);
                }

                dest.into()
            }
            resolver::Type::String | resolver::Type::DynamicBytes => {
                self.builder.build_store(slot_ptr, *slot);

                let ret = runtime.get_storage_string(&self, function, slot_ptr);

                *slot = self.builder.build_int_add(
                    *slot,
                    self.number_literal(256, &BigInt::one()),
                    "string",
                );

                ret.into()
            }
            _ => {
                self.builder.build_store(slot_ptr, *slot);

                let ret = runtime.get_storage_int(
                    &self,
                    function,
                    slot_ptr,
                    self.llvm_type(ty.deref()).into_int_type(),
                );

                *slot = self.builder.build_int_add(
                    *slot,
                    self.number_literal(256, &BigInt::one()),
                    "int",
                );

                ret.into()
            }
        }
    }

    /// Recursively store a type to contract storage
    fn storage_store(
        &self,
        ty: &resolver::Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        dest: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
        runtime: &dyn TargetRuntime,
    ) {
        match ty.deref() {
            resolver::Type::Array(_, dim) => {
                if let Some(d) = &dim[0] {
                    let ty = ty.array_deref();

                    self.emit_static_loop_with_int(
                        function,
                        self.context.i64_type().const_zero(),
                        self.context
                            .i64_type()
                            .const_int(d.to_u64().unwrap(), false),
                        slot,
                        |index: IntValue<'a>, slot: &mut IntValue<'a>| {
                            let mut elem = unsafe {
                                self.builder.build_gep(
                                    dest.into_pointer_value(),
                                    &[self.context.i32_type().const_zero(), index],
                                    "index_access",
                                )
                            };

                            if ty.is_reference_type() {
                                elem = self.builder.build_load(elem, "").into_pointer_value();
                            }

                            self.storage_store(&ty, slot, slot_ptr, elem.into(), function, runtime);

                            if !ty.is_reference_type() {
                                *slot = self.builder.build_int_add(
                                    *slot,
                                    self.number_literal(256, &ty.storage_slots(self.ns)),
                                    "",
                                );
                            }
                        },
                    );
                } else {
                    // get the lenght of the our in-memory array
                    let len = self
                        .builder
                        .build_load(
                            unsafe {
                                self.builder.build_gep(
                                    dest.into_pointer_value(),
                                    &[
                                        self.context.i32_type().const_zero(),
                                        self.context.i32_type().const_zero(),
                                    ],
                                    "array_len",
                                )
                            },
                            "array_len",
                        )
                        .into_int_value();

                    let slot_ty = resolver::Type::Uint(256);

                    // details about our array elements
                    let elem_ty = self.llvm_type(&ty.array_elem());
                    let elem_size = self.builder.build_int_truncate(
                        elem_ty.size_of().unwrap(),
                        self.context.i32_type(),
                        "size_of",
                    );

                    // the previous length of the storage array
                    // we need this to clear any elements
                    let previous_size = self.builder.build_int_truncate(
                        self.storage_load(&slot_ty, slot, slot_ptr, function, runtime)
                            .into_int_value(),
                        self.context.i32_type(),
                        "previous_size",
                    );

                    let new_slot = self
                        .builder
                        .build_alloca(self.llvm_type(&slot_ty).into_int_type(), "new");

                    // set new length
                    self.builder.build_store(
                        new_slot,
                        self.builder.build_int_z_extend(
                            len,
                            self.llvm_type(&slot_ty).into_int_type(),
                            "",
                        ),
                    );

                    runtime.set_storage(self, function, slot_ptr, new_slot);

                    runtime.keccak256_hash(
                        &self,
                        slot_ptr,
                        slot.get_type()
                            .size_of()
                            .const_cast(self.context.i32_type(), false),
                        new_slot,
                    );

                    let mut elem_slot = self
                        .builder
                        .build_load(new_slot, "elem_slot")
                        .into_int_value();

                    let ty = ty.array_deref();

                    self.emit_loop_cond_first_with_int(
                        function,
                        self.context.i32_type().const_zero(),
                        len,
                        &mut elem_slot,
                        |elem_no: IntValue<'a>, slot: &mut IntValue<'a>| {
                            let index = self.builder.build_int_mul(elem_no, elem_size, "");

                            let data = unsafe {
                                self.builder.build_gep(
                                    dest.into_pointer_value(),
                                    &[
                                        self.context.i32_type().const_zero(),
                                        self.context.i32_type().const_int(2, false),
                                        index,
                                    ],
                                    "data",
                                )
                            };

                            let mut elem = self.builder.build_pointer_cast(
                                data,
                                elem_ty.ptr_type(AddressSpace::Generic),
                                "entry",
                            );

                            if ty.is_reference_type() {
                                elem = self.builder.build_load(elem, "").into_pointer_value();
                            }

                            self.storage_store(&ty, slot, slot_ptr, elem.into(), function, runtime);

                            if !ty.is_reference_type() {
                                *slot = self.builder.build_int_add(
                                    *slot,
                                    self.number_literal(256, &ty.storage_slots(self.ns)),
                                    "",
                                );
                            }
                        },
                    );

                    // we've populated the array with the new values; if the new array is shorter
                    // than the previous, clear out the trailing elements
                    self.emit_loop_cond_first_with_int(
                        function,
                        len,
                        previous_size,
                        &mut elem_slot,
                        |_: IntValue<'a>, slot: &mut IntValue<'a>| {
                            self.storage_clear(&ty, slot, slot_ptr, function, runtime);

                            if !ty.is_reference_type() {
                                *slot = self.builder.build_int_add(
                                    *slot,
                                    self.number_literal(256, &ty.storage_slots(self.ns)),
                                    "",
                                );
                            }
                        },
                    );
                }
            }
            resolver::Type::Struct(n) => {
                for (i, field) in self.ns.structs[*n].fields.iter().enumerate() {
                    let mut elem = unsafe {
                        self.builder.build_gep(
                            dest.into_pointer_value(),
                            &[
                                self.context.i32_type().const_zero(),
                                self.context.i32_type().const_int(i as u64, false),
                            ],
                            &field.name,
                        )
                    };

                    if field.ty.is_reference_type() {
                        elem = self
                            .builder
                            .build_load(elem, &field.name)
                            .into_pointer_value();
                    }

                    self.storage_store(&field.ty, slot, slot_ptr, elem.into(), function, runtime);

                    if !field.ty.is_reference_type() {
                        *slot = self.builder.build_int_add(
                            *slot,
                            self.number_literal(256, &field.ty.storage_slots(self.ns)),
                            &field.name,
                        );
                    }
                }
            }
            resolver::Type::String | resolver::Type::DynamicBytes => {
                runtime.set_storage_string(&self, function, slot_ptr, dest.into_pointer_value());
            }
            _ => {
                self.builder.build_store(slot_ptr, *slot);

                let dest = if dest.is_int_value() {
                    let m = self.builder.build_alloca(dest.get_type(), "");
                    self.builder.build_store(m, dest);

                    m
                } else {
                    dest.into_pointer_value()
                };

                // TODO ewasm allocates 32 bytes here, even though we have just
                // allocated test. This can be folded into one allocation, if llvm
                // does not already fold it into one.
                runtime.set_storage(&self, function, slot_ptr, dest);
            }
        }
    }

    /// Recursively clear contract storage
    fn storage_clear(
        &self,
        ty: &resolver::Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        function: FunctionValue<'a>,
        runtime: &dyn TargetRuntime,
    ) {
        match ty.deref() {
            resolver::Type::Array(_, dim) => {
                let ty = ty.array_deref();

                if let Some(d) = &dim[0] {
                    self.emit_static_loop_with_int(
                        function,
                        self.context.i64_type().const_zero(),
                        self.context
                            .i64_type()
                            .const_int(d.to_u64().unwrap(), false),
                        slot,
                        |_index: IntValue<'a>, slot: &mut IntValue<'a>| {
                            self.storage_clear(&ty, slot, slot_ptr, function, runtime);

                            if !ty.is_reference_type() {
                                *slot = self.builder.build_int_add(
                                    *slot,
                                    self.number_literal(256, &ty.storage_slots(self.ns)),
                                    "",
                                );
                            }
                        },
                    );
                } else {
                    // dynamic length array.
                    // load length
                    self.builder.build_store(slot_ptr, *slot);

                    let slot_ty = self.context.custom_width_int_type(256);

                    let buf = self.builder.build_alloca(slot_ty, "buf");

                    let length = runtime.get_storage_int(self, function, slot_ptr, slot_ty);

                    // we need to hash the length slot in order to get the slot of the first
                    // entry of the array
                    runtime.keccak256_hash(
                        &self,
                        slot_ptr,
                        slot.get_type()
                            .size_of()
                            .const_cast(self.context.i32_type(), false),
                        buf,
                    );

                    let mut entry_slot =
                        self.builder.build_load(buf, "entry_slot").into_int_value();

                    // now loop from first slot to first slot + length
                    self.emit_loop_cond_first_with_int(
                        function,
                        length.get_type().const_zero(),
                        length,
                        &mut entry_slot,
                        |_index: IntValue<'a>, slot: &mut IntValue<'a>| {
                            self.storage_clear(&ty, slot, slot_ptr, function, runtime);

                            if !ty.is_reference_type() {
                                *slot = self.builder.build_int_add(
                                    *slot,
                                    self.number_literal(256, &ty.storage_slots(self.ns)),
                                    "",
                                );
                            }
                        },
                    );

                    // clear length itself
                    self.storage_clear(
                        &resolver::Type::Uint(256),
                        slot,
                        slot_ptr,
                        function,
                        runtime,
                    );
                }
            }
            resolver::Type::Struct(n) => {
                for (_, field) in self.ns.structs[*n].fields.iter().enumerate() {
                    self.storage_clear(&field.ty, slot, slot_ptr, function, runtime);

                    if !field.ty.is_reference_type() {
                        *slot = self.builder.build_int_add(
                            *slot,
                            self.number_literal(256, &field.ty.storage_slots(self.ns)),
                            &field.name,
                        );
                    }
                }
            }
            resolver::Type::Mapping(_, _) => {
                // nothing to do, step over it
            }
            _ => {
                self.builder.build_store(slot_ptr, *slot);

                runtime.clear_storage(&self, function, slot_ptr);
            }
        }
    }

    /// Emit the contract storage initializers
    fn emit_initializer(&self, runtime: &dyn TargetRuntime) -> FunctionValue<'a> {
        let function = self.module.add_function(
            "storage_initializers",
            self.context.i32_type().fn_type(&[], false),
            Some(Linkage::Internal),
        );

        self.emit_cfg(&self.contract.initializer, None, function, runtime);

        function
    }

    /// Emit function prototype
    fn declare_function(&self, fname: &str, f: &resolver::FunctionDecl) -> FunctionValue<'a> {
        let mut args: Vec<BasicTypeEnum> = Vec::new();

        for p in &f.params {
            let ty = self.llvm_type(&p.ty);

            args.push(if p.ty.stack_based() && !p.ty.is_contract_storage() {
                ty.ptr_type(AddressSpace::Generic).into()
            } else {
                ty
            });
        }

        // add return values
        for p in &f.returns {
            args.push(if p.ty.is_reference_type() && !p.ty.is_contract_storage() {
                self.llvm_type(&p.ty)
                    .ptr_type(AddressSpace::Generic)
                    .ptr_type(AddressSpace::Generic)
                    .into()
            } else {
                self.llvm_type(&p.ty).ptr_type(AddressSpace::Generic).into()
            });
        }
        let ftype = self.context.i32_type().fn_type(&args, false);

        self.module
            .add_function(&fname, ftype, Some(Linkage::Internal))
    }

    /// Emit function body
    fn define_function(
        &self,
        resolver_func: &resolver::FunctionDecl,
        function: FunctionValue<'a>,
        runtime: &dyn TargetRuntime,
    ) {
        let cfg = match resolver_func.cfg {
            Some(ref cfg) => cfg,
            None => panic!(),
        };

        self.emit_cfg(cfg, Some(resolver_func), function, runtime);
    }

    #[allow(clippy::cognitive_complexity)]
    fn emit_cfg(
        &self,
        cfg: &cfg::ControlFlowGraph,
        resolver_function: Option<&resolver::FunctionDecl>,
        function: FunctionValue<'a>,
        runtime: &dyn TargetRuntime,
    ) {
        // recurse through basic blocks
        struct BasicBlock<'a> {
            bb: inkwell::basic_block::BasicBlock<'a>,
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

            self.builder.position_at_end(bb);

            if let Some(ref cfg_phis) = cfg_bb.phis {
                for v in cfg_phis {
                    if cfg.vars[*v].ty.needs_phi() {
                        let ty = self.llvm_var(&cfg.vars[*v].ty);

                        phis.insert(*v, self.builder.build_phi(ty, &cfg.vars[*v].id.name));
                    }
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
                cfg::Storage::Local if v.ty.is_reference_type() && !v.ty.is_contract_storage() => {
                    vars.push(Variable {
                        value: self
                            .builder
                            .build_alloca(self.llvm_type(&v.ty), &v.id.name)
                            .into(),
                        stack: false,
                        phi: v.ty.needs_phi(),
                    });
                }
                cfg::Storage::Local if !v.ty.stack_based() || v.ty.is_reference_type() => {
                    vars.push(Variable {
                        value: self.context.i32_type().const_zero().into(),
                        stack: false,
                        phi: v.ty.needs_phi(),
                    });
                }
                cfg::Storage::Constant(_) | cfg::Storage::Contract(_)
                    if v.ty.is_reference_type() =>
                {
                    // This needs a placeholder
                    vars.push(Variable {
                        value: self.context.bool_type().get_undef().into(),
                        stack: false,
                        phi: v.ty.needs_phi(),
                    });
                }
                cfg::Storage::Local | cfg::Storage::Contract(_) | cfg::Storage::Constant(_) => {
                    vars.push(Variable {
                        value: self
                            .builder
                            .build_alloca(self.llvm_type(&v.ty), &v.id.name)
                            .into(),
                        stack: true,
                        phi: v.ty.needs_phi(),
                    });
                }
            }
        }

        work.push_back(Work { bb_no: 0, vars });

        while let Some(mut w) = work.pop_front() {
            let bb = blocks.get(&w.bb_no).unwrap();

            self.builder.position_at_end(bb.bb);

            for (v, phi) in bb.phis.iter() {
                w.vars[*v].value = (*phi).as_basic_value();
            }

            for ins in &cfg.bb[w.bb_no].instr {
                match ins {
                    cfg::Instr::FuncArg { res, arg } => {
                        w.vars[*res].value = function.get_nth_param(*arg as u32).unwrap();
                    }
                    cfg::Instr::Return { value } if value.is_empty() => {
                        self.builder
                            .build_return(Some(&self.context.i32_type().const_zero()));
                    }
                    cfg::Instr::Return { value } => {
                        let returns_offset = resolver_function.unwrap().params.len();
                        for (i, val) in value.iter().enumerate() {
                            let arg = function.get_nth_param((returns_offset + i) as u32).unwrap();
                            let retval = self.expression(val, &w.vars, function, runtime);

                            self.builder.build_store(arg.into_pointer_value(), retval);
                        }
                        self.builder
                            .build_return(Some(&self.context.i32_type().const_zero()));
                    }
                    cfg::Instr::Set { res, expr } => {
                        let value_ref = self.expression(expr, &w.vars, function, runtime);
                        if w.vars[*res].stack {
                            self.builder
                                .build_store(w.vars[*res].value.into_pointer_value(), value_ref);
                        } else {
                            w.vars[*res].value = value_ref;
                        }
                    }
                    cfg::Instr::Eval { expr } => {
                        self.expression(expr, &w.vars, function, runtime);
                    }
                    cfg::Instr::Constant { res, constant } => {
                        let const_expr = &self.contract.constants[*constant];
                        let value_ref = self.expression(const_expr, &w.vars, function, runtime);
                        if w.vars[*res].stack {
                            self.builder
                                .build_store(w.vars[*res].value.into_pointer_value(), value_ref);
                        } else {
                            w.vars[*res].value = value_ref;
                        }
                    }
                    cfg::Instr::Branch { bb: dest } => {
                        let pos = self.builder.get_insert_block().unwrap();

                        if !blocks.contains_key(&dest) {
                            blocks.insert(*dest, create_bb(*dest));
                            work.push_back(Work {
                                bb_no: *dest,
                                vars: w.vars.clone(),
                            });
                        }

                        let bb = blocks.get(dest).unwrap();

                        for (v, phi) in bb.phis.iter() {
                            if w.vars[*v].phi {
                                phi.add_incoming(&[(&w.vars[*v].value, pos)]);
                            }
                        }

                        self.builder.position_at_end(pos);
                        self.builder.build_unconditional_branch(bb.bb);
                    }
                    cfg::Instr::Store { dest, pos } => {
                        let value_ref = w.vars[*pos].value;
                        let dest_ref = self
                            .expression(dest, &w.vars, function, runtime)
                            .into_pointer_value();

                        self.builder.build_store(dest_ref, value_ref);
                    }
                    cfg::Instr::BranchCond {
                        cond,
                        true_,
                        false_,
                    } => {
                        let cond = self.expression(cond, &w.vars, function, runtime);

                        let pos = self.builder.get_insert_block().unwrap();

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
                                if w.vars[*v].phi {
                                    phi.add_incoming(&[(&w.vars[*v].value, pos)]);
                                }
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
                                if w.vars[*v].phi {
                                    phi.add_incoming(&[(&w.vars[*v].value, pos)]);
                                }
                            }

                            bb.bb
                        };

                        self.builder.position_at_end(pos);
                        self.builder.build_conditional_branch(
                            cond.into_int_value(),
                            bb_true,
                            bb_false,
                        );
                    }
                    cfg::Instr::ClearStorage { ty, storage } => {
                        let mut slot = self
                            .expression(storage, &w.vars, function, runtime)
                            .into_int_value();
                        let slot_ptr = self.builder.build_alloca(slot.get_type(), "slot");

                        self.storage_clear(ty, &mut slot, slot_ptr, function, runtime);
                    }
                    cfg::Instr::SetStorage { ty, local, storage } => {
                        let value = w.vars[*local].value;

                        let mut slot = self
                            .expression(storage, &w.vars, function, runtime)
                            .into_int_value();
                        let slot_ptr = self.builder.build_alloca(slot.get_type(), "slot");

                        self.storage_store(ty, &mut slot, slot_ptr, value, function, runtime);
                    }
                    cfg::Instr::SetStorageBytes {
                        local,
                        storage,
                        offset,
                    } => {
                        let value = w.vars[*local].value;

                        let slot = self
                            .expression(storage, &w.vars, function, runtime)
                            .into_int_value();
                        let offset = self
                            .expression(offset, &w.vars, function, runtime)
                            .into_int_value();
                        let slot_ptr = self.builder.build_alloca(slot.get_type(), "slot");

                        runtime.set_storage_bytes_subscript(
                            self,
                            function,
                            slot_ptr,
                            offset,
                            value.into_int_value(),
                        );
                    }
                    cfg::Instr::AssertFailure { expr: None } => {
                        runtime.assert_failure(
                            self,
                            self.context
                                .i8_type()
                                .ptr_type(AddressSpace::Generic)
                                .const_null(),
                            self.context.i32_type().const_zero(),
                        );
                    }
                    cfg::Instr::AssertFailure { expr: Some(expr) } => {
                        let v = self.expression(expr, &w.vars, function, runtime);

                        let (data, len) = runtime.abi_encode(
                            self,
                            Some(0x08c3_79a0),
                            false,
                            function,
                            &[v],
                            &[resolver::Parameter {
                                name: "error".to_owned(),
                                ty: resolver::Type::String,
                            }],
                        );

                        runtime.assert_failure(self, data, len);
                    }
                    cfg::Instr::Print { expr } => {
                        let v = self
                            .expression(expr, &w.vars, function, runtime)
                            .into_pointer_value();

                        let data = unsafe {
                            self.builder.build_gep(
                                v,
                                &[
                                    self.context.i32_type().const_zero(),
                                    self.context.i32_type().const_int(2, false),
                                ],
                                "data",
                            )
                        };

                        let data_len = unsafe {
                            self.builder.build_gep(
                                v,
                                &[
                                    self.context.i32_type().const_zero(),
                                    self.context.i32_type().const_zero(),
                                ],
                                "data_len",
                            )
                        };

                        runtime.print(
                            &self,
                            self.builder.build_pointer_cast(
                                data,
                                self.context.i8_type().ptr_type(AddressSpace::Generic),
                                "data",
                            ),
                            self.builder
                                .build_load(data_len, "data_len")
                                .into_int_value(),
                        );
                    }
                    cfg::Instr::Call { res, func, args } => {
                        let mut parms: Vec<BasicValueEnum> = Vec::new();
                        let f = &self.contract.functions[*func];

                        for (i, a) in args.iter().enumerate() {
                            let ty = &f.params[i].ty;
                            let val = self.expression(&a, &w.vars, function, runtime);

                            parms.push(if ty.stack_based() && !ty.is_reference_type() {
                                // copy onto stack
                                let m = self.builder.build_alloca(self.llvm_type(ty), "");

                                self.builder.build_store(m, val);

                                m.into()
                            } else {
                                val
                            });
                        }

                        if !res.is_empty() {
                            for v in f.returns.iter() {
                                parms.push(
                                    self.builder
                                        .build_alloca(self.llvm_var(&v.ty), &v.name)
                                        .into(),
                                );
                            }
                        }

                        let ret = self
                            .builder
                            .build_call(self.functions[*func], &parms, "")
                            .try_as_basic_value()
                            .left()
                            .unwrap();

                        let success = self.builder.build_int_compare(
                            IntPredicate::EQ,
                            ret.into_int_value(),
                            self.context.i32_type().const_zero(),
                            "success",
                        );

                        let success_block = self.context.append_basic_block(function, "success");
                        let bail_block = self.context.append_basic_block(function, "bail");
                        self.builder
                            .build_conditional_branch(success, success_block, bail_block);

                        self.builder.position_at_end(bail_block);

                        self.builder.build_return(Some(&ret));
                        self.builder.position_at_end(success_block);

                        if !res.is_empty() {
                            for (i, v) in f.returns.iter().enumerate() {
                                let val = self.builder.build_load(
                                    parms[f.params.len() + i].into_pointer_value(),
                                    &v.name,
                                );

                                let dest = w.vars[res[i]].value;

                                if dest.is_pointer_value() && !v.ty.is_reference_type() {
                                    self.builder.build_store(dest.into_pointer_value(), val);
                                } else {
                                    w.vars[res[i]].value = val;
                                }
                            }
                        }
                    }
                    cfg::Instr::Constructor {
                        res,
                        contract_no,
                        constructor_no,
                        args,
                    } => {
                        runtime.create_contract(
                            &self,
                            function,
                            *contract_no,
                            *constructor_no,
                            self.builder.build_pointer_cast(
                                w.vars[*res].value.into_pointer_value(),
                                self.context.i8_type().ptr_type(AddressSpace::Generic),
                                "address",
                            ),
                            &args
                                .iter()
                                .map(|a| self.expression(&a, &w.vars, function, runtime))
                                .collect::<Vec<BasicValueEnum>>(),
                        );
                    }
                    cfg::Instr::ExternalCall {
                        address,
                        contract_no,
                        function_no,
                        args,
                        res,
                    } => {
                        let dest_func = &self.ns.contracts[*contract_no].functions[*function_no];
                        let (payload, payload_len) = runtime.abi_encode(
                            self,
                            Some(dest_func.selector()),
                            false,
                            function,
                            &args
                                .iter()
                                .map(|a| self.expression(&a, &w.vars, function, runtime))
                                .collect::<Vec<BasicValueEnum>>(),
                            &dest_func.params,
                        );
                        let address = self
                            .expression(address, &w.vars, function, runtime)
                            .into_int_value();

                        let addr = self.builder.build_array_alloca(
                            self.context.i8_type(),
                            self.context
                                .i32_type()
                                .const_int(self.ns.address_length as u64, false),
                            "address",
                        );

                        self.builder.build_store(
                            self.builder.build_pointer_cast(
                                addr,
                                address.get_type().ptr_type(AddressSpace::Generic),
                                "address",
                            ),
                            address,
                        );

                        let ret = runtime.external_call(self, payload, payload_len, addr);

                        let success = self.builder.build_int_compare(
                            IntPredicate::EQ,
                            ret,
                            self.context.i32_type().const_zero(),
                            "success",
                        );

                        let success_block = self.context.append_basic_block(function, "success");
                        let bail_block = self.context.append_basic_block(function, "bail");
                        self.builder
                            .build_conditional_branch(success, success_block, bail_block);
                        self.builder.position_at_end(bail_block);

                        self.builder.build_return(Some(&ret));

                        self.builder.position_at_end(success_block);

                        if !dest_func.returns.is_empty() {
                            let (return_data, return_size) = runtime.return_data(self);

                            let mut returns = Vec::new();

                            runtime.abi_decode(
                                self,
                                function,
                                &mut returns,
                                return_data,
                                return_size,
                                &dest_func.returns,
                            );

                            for (i, ret) in returns.into_iter().enumerate() {
                                w.vars[res[i]].value = ret;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn emit_function_dispatch(
        &self,
        resolver_functions: &[resolver::FunctionDecl],
        functions: &[FunctionValue<'a>],
        argsdata: inkwell::values::PointerValue<'a>,
        argslen: inkwell::values::IntValue<'a>,
        function: inkwell::values::FunctionValue<'a>,
        fallback_block: inkwell::basic_block::BasicBlock,
        runtime: &dyn TargetRuntime,
    ) {
        // create start function
        let switch_block = self.context.append_basic_block(function, "switch");

        let not_fallback = self.builder.build_int_compare(
            IntPredicate::UGE,
            argslen,
            self.context.i32_type().const_int(4, false),
            "",
        );

        let nomatch = self.context.append_basic_block(function, "nomatch");

        self.builder
            .build_conditional_branch(not_fallback, switch_block, nomatch);

        self.builder.position_at_end(switch_block);

        let fid = self.builder.build_load(argsdata, "function_selector");

        // step over the function selector
        let argsdata = unsafe {
            self.builder.build_gep(
                argsdata,
                &[self.context.i32_type().const_int(1, false)],
                "argsdata",
            )
        };

        let argslen = self.builder.build_int_sub(
            argslen,
            self.context.i32_type().const_int(4, false),
            "argslen",
        );

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

            self.builder.position_at_end(bb);

            let mut args = Vec::new();

            // insert abi decode
            runtime.abi_decode(&self, function, &mut args, argsdata, argslen, &f.params);

            // add return values as pointer arguments at the end
            if !f.returns.is_empty() {
                for v in f.returns.iter() {
                    args.push(if !v.ty.is_reference_type() {
                        self.builder
                            .build_alloca(self.llvm_type(&v.ty), &v.name)
                            .into()
                    } else {
                        self.builder
                            .build_alloca(
                                self.llvm_type(&v.ty).ptr_type(AddressSpace::Generic),
                                &v.name,
                            )
                            .into()
                    });
                }
            }

            let ret = self
                .builder
                .build_call(functions[i], &args, "")
                .try_as_basic_value()
                .left()
                .unwrap();

            let success = self.builder.build_int_compare(
                IntPredicate::EQ,
                ret.into_int_value(),
                self.context.i32_type().const_zero(),
                "success",
            );

            let success_block = self.context.append_basic_block(function, "success");
            let bail_block = self.context.append_basic_block(function, "bail");

            self.builder
                .build_conditional_branch(success, success_block, bail_block);

            self.builder.position_at_end(success_block);

            if f.returns.is_empty() {
                // return ABI of length 0
                runtime.return_empty_abi(&self);
            } else {
                let (data, length) = runtime.abi_encode(
                    &self,
                    None,
                    true,
                    function,
                    &args[f.params.len()..],
                    &f.returns,
                );

                runtime.return_abi(&self, data, length);
            }

            self.builder.position_at_end(bail_block);

            self.builder.build_return(Some(&ret));

            cases.push((self.context.i32_type().const_int(id as u64, false), bb));
        }

        self.builder.position_at_end(switch_block);

        self.builder
            .build_switch(fid.into_int_value(), fallback_block, &cases);

        self.builder.position_at_end(nomatch);

        runtime.assert_failure(
            &self,
            self.context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            self.context.i32_type().const_zero(),
        );
    }

    // Generate an unsigned divmod function for the given bitwidth. This is for int sizes which
    // WebAssembly does not support, i.e. anything over 64.
    // The builder position is maintained.
    //
    // inspired by https://github.com/calccrypto/uint256_t/blob/master/uint256_t.cpp#L397
    pub fn udivmod(&self, bit: u32, runtime: &dyn TargetRuntime) -> FunctionValue<'a> {
        let name = format!("__udivmod{}", bit);
        let ty = self.context.custom_width_int_type(bit);

        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        let pos = self.builder.get_insert_block().unwrap();

        // __udivmod256(dividend, divisor, *rem, *quotient) = error
        let function = self.module.add_function(
            &name,
            self.context.i32_type().fn_type(
                &[
                    ty.into(),
                    ty.into(),
                    ty.ptr_type(AddressSpace::Generic).into(),
                    ty.ptr_type(AddressSpace::Generic).into(),
                ],
                false,
            ),
            None,
        );

        let entry = self.context.append_basic_block(function, "entry");

        self.builder.position_at_end(entry);

        let dividend = function.get_nth_param(0).unwrap().into_int_value();
        let divisor = function.get_nth_param(1).unwrap().into_int_value();
        let rem = function.get_nth_param(2).unwrap().into_pointer_value();
        let quotient_result = function.get_nth_param(3).unwrap().into_pointer_value();

        let error = self.context.append_basic_block(function, "error");
        let next = self.context.append_basic_block(function, "next");
        let is_zero = self.builder.build_int_compare(
            IntPredicate::EQ,
            divisor,
            ty.const_zero(),
            "divisor_is_zero",
        );
        self.builder.build_conditional_branch(is_zero, error, next);

        self.builder.position_at_end(error);
        // throw division by zero error should be an assert
        runtime.assert_failure(
            self,
            self.context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            self.context.i32_type().const_zero(),
        );

        self.builder.position_at_end(next);
        let is_one_block = self.context.append_basic_block(function, "is_one_block");
        let next = self.context.append_basic_block(function, "next");
        let is_one = self.builder.build_int_compare(
            IntPredicate::EQ,
            divisor,
            ty.const_int(1, false),
            "divisor_is_one",
        );
        self.builder
            .build_conditional_branch(is_one, is_one_block, next);

        // return quotient: dividend, rem: 0
        self.builder.position_at_end(is_one_block);
        self.builder.build_store(rem, ty.const_zero());
        self.builder.build_store(quotient_result, dividend);
        self.builder
            .build_return(Some(&self.context.i32_type().const_zero()));

        self.builder.position_at_end(next);
        let is_eq_block = self.context.append_basic_block(function, "is_eq_block");
        let next = self.context.append_basic_block(function, "next");
        let is_eq = self
            .builder
            .build_int_compare(IntPredicate::EQ, dividend, divisor, "is_eq");
        self.builder
            .build_conditional_branch(is_eq, is_eq_block, next);

        // return rem: 0, quotient: 1
        self.builder.position_at_end(is_eq_block);
        self.builder.build_store(rem, ty.const_zero());
        self.builder.build_store(rem, ty.const_zero());
        self.builder
            .build_store(quotient_result, ty.const_int(1, false));
        self.builder
            .build_return(Some(&self.context.i32_type().const_zero()));

        self.builder.position_at_end(next);

        let is_toobig_block = self.context.append_basic_block(function, "is_toobig_block");
        let next = self.context.append_basic_block(function, "next");
        let dividend_is_zero = self.builder.build_int_compare(
            IntPredicate::EQ,
            dividend,
            ty.const_zero(),
            "dividend_is_zero",
        );
        let dividend_lt_divisor = self.builder.build_int_compare(
            IntPredicate::ULT,
            dividend,
            divisor,
            "dividend_lt_divisor",
        );
        self.builder.build_conditional_branch(
            self.builder
                .build_or(dividend_is_zero, dividend_lt_divisor, ""),
            is_toobig_block,
            next,
        );

        // return quotient: 0, rem: divisor
        self.builder.position_at_end(is_toobig_block);
        self.builder.build_store(rem, dividend);
        self.builder.build_store(quotient_result, ty.const_zero());
        self.builder
            .build_return(Some(&self.context.i32_type().const_zero()));

        self.builder.position_at_end(next);

        let ctlz = self.llvm_ctlz(bit);

        let dividend_bits = self.builder.build_int_sub(
            ty.const_int(bit as u64 - 1, false),
            self.builder
                .build_call(
                    ctlz,
                    &[
                        dividend.into(),
                        self.context.bool_type().const_int(1, false).into(),
                    ],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value(),
            "dividend_bits",
        );

        let divisor_bits = self.builder.build_int_sub(
            ty.const_int(bit as u64 - 1, false),
            self.builder
                .build_call(
                    ctlz,
                    &[
                        divisor.into(),
                        self.context.bool_type().const_int(1, false).into(),
                    ],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value(),
            "dividend_bits",
        );

        let copyd1 = self.builder.build_left_shift(
            divisor,
            self.builder.build_int_sub(dividend_bits, divisor_bits, ""),
            "copyd",
        );

        let adder1 = self.builder.build_left_shift(
            ty.const_int(1, false),
            self.builder.build_int_sub(dividend_bits, divisor_bits, ""),
            "adder",
        );

        let true_block = self.context.append_basic_block(function, "true");
        let while_cond_block = self.context.append_basic_block(function, "while_cond");

        let comp = self
            .builder
            .build_int_compare(IntPredicate::UGT, copyd1, dividend, "");

        self.builder
            .build_conditional_branch(comp, true_block, while_cond_block);

        self.builder.position_at_end(true_block);

        let copyd2 = self
            .builder
            .build_right_shift(copyd1, ty.const_int(1, false), false, "");
        let adder2 = self
            .builder
            .build_right_shift(adder1, ty.const_int(1, false), false, "");
        self.builder.build_unconditional_branch(while_cond_block);

        let while_body_block = self.context.append_basic_block(function, "while_body");
        let while_end_block = self.context.append_basic_block(function, "while_post");

        self.builder.position_at_end(while_cond_block);

        let quotient = self.builder.build_phi(ty, "quotient");
        quotient.add_incoming(&[(&ty.const_zero(), next)]);
        quotient.add_incoming(&[(&ty.const_zero(), true_block)]);

        let remainder = self.builder.build_phi(ty, "remainder");
        remainder.add_incoming(&[(&dividend, next)]);
        remainder.add_incoming(&[(&dividend, true_block)]);

        let copyd = self.builder.build_phi(ty, "copyd");
        copyd.add_incoming(&[(&copyd1, next), (&copyd2, true_block)]);
        let adder = self.builder.build_phi(ty, "adder");
        adder.add_incoming(&[(&adder1, next), (&adder2, true_block)]);

        let loop_cond = self.builder.build_int_compare(
            IntPredicate::UGE,
            remainder.as_basic_value().into_int_value(),
            divisor,
            "loop_cond",
        );
        self.builder
            .build_conditional_branch(loop_cond, while_body_block, while_end_block);

        self.builder.position_at_end(while_body_block);

        let if_true_block = self.context.append_basic_block(function, "if_true_block");
        let post_if_block = self.context.append_basic_block(function, "post_if_block");

        self.builder.build_conditional_branch(
            self.builder.build_int_compare(
                IntPredicate::UGE,
                remainder.as_basic_value().into_int_value(),
                copyd.as_basic_value().into_int_value(),
                "",
            ),
            if_true_block,
            post_if_block,
        );

        self.builder.position_at_end(if_true_block);

        let remainder2 = self.builder.build_int_sub(
            remainder.as_basic_value().into_int_value(),
            copyd.as_basic_value().into_int_value(),
            "remainder",
        );
        let quotient2 = self.builder.build_or(
            quotient.as_basic_value().into_int_value(),
            adder.as_basic_value().into_int_value(),
            "quotient",
        );

        self.builder.build_unconditional_branch(post_if_block);

        self.builder.position_at_end(post_if_block);

        let quotient3 = self.builder.build_phi(ty, "quotient3");
        let remainder3 = self.builder.build_phi(ty, "remainder");

        let copyd3 = self.builder.build_right_shift(
            copyd.as_basic_value().into_int_value(),
            ty.const_int(1, false),
            false,
            "copyd",
        );
        let adder3 = self.builder.build_right_shift(
            adder.as_basic_value().into_int_value(),
            ty.const_int(1, false),
            false,
            "adder",
        );
        copyd.add_incoming(&[(&copyd3, post_if_block)]);
        adder.add_incoming(&[(&adder3, post_if_block)]);

        quotient3.add_incoming(&[
            (&quotient2, if_true_block),
            (&quotient.as_basic_value(), while_body_block),
        ]);
        remainder3.add_incoming(&[
            (&remainder2, if_true_block),
            (&remainder.as_basic_value(), while_body_block),
        ]);

        quotient.add_incoming(&[(&quotient3.as_basic_value(), post_if_block)]);
        remainder.add_incoming(&[(&remainder3.as_basic_value(), post_if_block)]);

        self.builder.build_unconditional_branch(while_cond_block);

        self.builder.position_at_end(while_end_block);

        self.builder
            .build_store(rem, remainder.as_basic_value().into_int_value());
        self.builder
            .build_store(quotient_result, quotient.as_basic_value().into_int_value());
        self.builder
            .build_return(Some(&self.context.i32_type().const_zero()));

        self.builder.position_at_end(pos);

        function
    }

    pub fn sdivmod(&self, bit: u32, runtime: &dyn TargetRuntime) -> FunctionValue<'a> {
        let name = format!("__sdivmod{}", bit);
        let ty = self.context.custom_width_int_type(bit);

        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        let pos = self.builder.get_insert_block().unwrap();

        // __sdivmod256(dividend, divisor, *rem, *quotient) -> error
        let function = self.module.add_function(
            &name,
            self.context.i32_type().fn_type(
                &[
                    ty.into(),
                    ty.into(),
                    ty.ptr_type(AddressSpace::Generic).into(),
                    ty.ptr_type(AddressSpace::Generic).into(),
                ],
                false,
            ),
            None,
        );

        let entry = self.context.append_basic_block(function, "entry");

        self.builder.position_at_end(entry);

        let dividend = function.get_nth_param(0).unwrap().into_int_value();
        let divisor = function.get_nth_param(1).unwrap().into_int_value();
        let rem = function.get_nth_param(2).unwrap().into_pointer_value();
        let quotient_result = function.get_nth_param(3).unwrap().into_pointer_value();

        let dividend_negative = self.builder.build_int_compare(
            IntPredicate::SLT,
            dividend,
            ty.const_zero(),
            "dividend_negative",
        );
        let divisor_negative = self.builder.build_int_compare(
            IntPredicate::SLT,
            divisor,
            ty.const_zero(),
            "divisor_negative",
        );

        let dividend_abs = self.builder.build_select(
            dividend_negative,
            self.builder.build_int_neg(dividend, "dividen_neg"),
            dividend,
            "dividend_abs",
        );

        let divisor_abs = self.builder.build_select(
            divisor_negative,
            self.builder.build_int_neg(divisor, "divisor_neg"),
            divisor,
            "divisor_abs",
        );

        let ret = self
            .builder
            .build_call(
                self.udivmod(bit, runtime),
                &[
                    dividend_abs,
                    divisor_abs,
                    rem.into(),
                    quotient_result.into(),
                ],
                "quotient",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let success = self.builder.build_int_compare(
            IntPredicate::EQ,
            ret.into_int_value(),
            self.context.i32_type().const_zero(),
            "success",
        );

        let success_block = self.context.append_basic_block(function, "success");
        let bail_block = self.context.append_basic_block(function, "bail");
        self.builder
            .build_conditional_branch(success, success_block, bail_block);

        self.builder.position_at_end(bail_block);

        self.builder.build_return(Some(&ret));
        self.builder.position_at_end(success_block);

        let quotient = self
            .builder
            .build_load(quotient_result, "quotient")
            .into_int_value();

        let quotient = self.builder.build_select(
            self.builder.build_int_compare(
                IntPredicate::NE,
                dividend_negative,
                divisor_negative,
                "two_negatives",
            ),
            self.builder.build_int_neg(quotient, "quotient_neg"),
            quotient,
            "quotient",
        );

        let negrem = self.context.append_basic_block(function, "negative_rem");
        let posrem = self.context.append_basic_block(function, "positive_rem");

        self.builder
            .build_conditional_branch(dividend_negative, negrem, posrem);

        self.builder.position_at_end(posrem);

        self.builder.build_store(quotient_result, quotient);
        self.builder
            .build_return(Some(&self.context.i32_type().const_zero()));

        self.builder.position_at_end(negrem);

        let remainder = self.builder.build_load(rem, "remainder").into_int_value();

        self.builder.build_store(
            rem,
            self.builder.build_int_neg(remainder, "negative_remainder"),
        );

        self.builder.build_store(quotient_result, quotient);
        self.builder
            .build_return(Some(&self.context.i32_type().const_zero()));

        self.builder.position_at_end(pos);

        function
    }

    pub fn upower(&self, bit: u32) -> FunctionValue<'a> {
        /*
            int ipow(int base, int exp)
            {
                int result = 1;
                for (;;)
                {
                    if (exp & 1)
                        result *= base;
                    exp >>= 1;
                    if (!exp)
                        break;
                    base *= base;
                }

                return result;
            }
        */
        let name = format!("__upower{}", bit);
        let ty = self.context.custom_width_int_type(bit);

        if let Some(f) = self.module.get_function(&name) {
            return f;
        }

        let pos = self.builder.get_insert_block().unwrap();

        // __upower(base, exp)
        let function =
            self.module
                .add_function(&name, ty.fn_type(&[ty.into(), ty.into()], false), None);

        let entry = self.context.append_basic_block(function, "entry");
        let loop_block = self.context.append_basic_block(function, "loop");
        let multiply = self.context.append_basic_block(function, "multiply");
        let nomultiply = self.context.append_basic_block(function, "nomultiply");
        let done = self.context.append_basic_block(function, "done");
        let notdone = self.context.append_basic_block(function, "notdone");

        self.builder.position_at_end(entry);

        let l = self.builder.build_alloca(ty, "");
        let r = self.builder.build_alloca(ty, "");
        let o = self.builder.build_alloca(ty, "");

        self.builder.build_unconditional_branch(loop_block);

        self.builder.position_at_end(loop_block);
        let base = self.builder.build_phi(ty, "base");
        base.add_incoming(&[(&function.get_nth_param(0).unwrap(), entry)]);

        let exp = self.builder.build_phi(ty, "exp");
        exp.add_incoming(&[(&function.get_nth_param(1).unwrap(), entry)]);

        let result = self.builder.build_phi(ty, "result");
        result.add_incoming(&[(&ty.const_int(1, false), entry)]);

        let lowbit = self.builder.build_int_truncate(
            exp.as_basic_value().into_int_value(),
            self.context.bool_type(),
            "bit",
        );

        self.builder
            .build_conditional_branch(lowbit, multiply, nomultiply);

        self.builder.position_at_end(multiply);

        let result2 = if bit > 64 {
            self.builder
                .build_store(l, result.as_basic_value().into_int_value());
            self.builder
                .build_store(r, base.as_basic_value().into_int_value());

            self.builder.build_call(
                self.module.get_function("__mul32").unwrap(),
                &[
                    self.builder
                        .build_pointer_cast(
                            l,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "left",
                        )
                        .into(),
                    self.builder
                        .build_pointer_cast(
                            r,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "right",
                        )
                        .into(),
                    self.builder
                        .build_pointer_cast(
                            o,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "output",
                        )
                        .into(),
                    self.context
                        .i32_type()
                        .const_int(bit as u64 / 32, false)
                        .into(),
                ],
                "",
            );

            self.builder.build_load(o, "result").into_int_value()
        } else {
            self.builder.build_int_mul(
                result.as_basic_value().into_int_value(),
                base.as_basic_value().into_int_value(),
                "result",
            )
        };

        self.builder.build_unconditional_branch(nomultiply);
        self.builder.position_at_end(nomultiply);

        let result3 = self.builder.build_phi(ty, "result");
        result3.add_incoming(&[(&result.as_basic_value(), loop_block), (&result2, multiply)]);

        let exp2 = self.builder.build_right_shift(
            exp.as_basic_value().into_int_value(),
            ty.const_int(1, false),
            false,
            "exp",
        );
        let zero = self
            .builder
            .build_int_compare(IntPredicate::EQ, exp2, ty.const_zero(), "zero");

        self.builder.build_conditional_branch(zero, done, notdone);

        self.builder.position_at_end(done);

        self.builder.build_return(Some(&result3.as_basic_value()));

        self.builder.position_at_end(notdone);

        let base2 = if bit > 64 {
            self.builder
                .build_store(l, base.as_basic_value().into_int_value());
            self.builder
                .build_store(r, base.as_basic_value().into_int_value());

            self.builder.build_call(
                self.module.get_function("__mul32").unwrap(),
                &[
                    self.builder
                        .build_pointer_cast(
                            l,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "left",
                        )
                        .into(),
                    self.builder
                        .build_pointer_cast(
                            r,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "right",
                        )
                        .into(),
                    self.builder
                        .build_pointer_cast(
                            o,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "output",
                        )
                        .into(),
                    self.context
                        .i32_type()
                        .const_int(bit as u64 / 32, false)
                        .into(),
                ],
                "",
            );

            self.builder.build_load(o, "base").into_int_value()
        } else {
            self.builder.build_int_mul(
                base.as_basic_value().into_int_value(),
                base.as_basic_value().into_int_value(),
                "base",
            )
        };

        base.add_incoming(&[(&base2, notdone)]);
        result.add_incoming(&[(&result3.as_basic_value(), notdone)]);
        exp.add_incoming(&[(&exp2, notdone)]);

        self.builder.build_unconditional_branch(loop_block);

        self.builder.position_at_end(pos);

        function
    }

    // Create the llvm intrinsic for counting leading zeros
    pub fn llvm_ctlz(&self, bit: u32) -> FunctionValue<'a> {
        let name = format!("llvm.ctlz.i{}", bit);
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

    /// Return the llvm type for a variable holding the type, not the type itself
    fn llvm_var(&self, ty: &resolver::Type) -> BasicTypeEnum<'a> {
        let llvm_ty = self.llvm_type(ty);
        match ty.deref_nonstorage() {
            resolver::Type::Struct(_)
            | resolver::Type::Array(_, _)
            | resolver::Type::DynamicBytes
            | resolver::Type::String => {
                llvm_ty.ptr_type(AddressSpace::Generic).as_basic_type_enum()
            }
            _ => llvm_ty,
        }
    }

    /// Return the llvm type for the resolved type.
    fn llvm_type(&self, ty: &resolver::Type) -> BasicTypeEnum<'a> {
        match ty {
            resolver::Type::Bool => BasicTypeEnum::IntType(self.context.bool_type()),
            resolver::Type::Int(n) | resolver::Type::Uint(n) => {
                BasicTypeEnum::IntType(self.context.custom_width_int_type(*n as u32))
            }
            resolver::Type::Contract(_) | resolver::Type::Address => BasicTypeEnum::IntType(
                self.context
                    .custom_width_int_type(self.ns.address_length as u32 * 8),
            ),
            resolver::Type::Bytes(n) => {
                BasicTypeEnum::IntType(self.context.custom_width_int_type(*n as u32 * 8))
            }
            resolver::Type::Enum(n) => self.llvm_type(&self.ns.enums[*n].ty),
            resolver::Type::String | resolver::Type::DynamicBytes => {
                self.module.get_type("struct.vector").unwrap()
            }
            resolver::Type::Array(base_ty, dims) => {
                let ty = self.llvm_var(base_ty);

                let mut dims = dims.iter();

                let mut aty = match dims.next().unwrap() {
                    Some(d) => ty.array_type(d.to_u32().unwrap()),
                    None => return self.module.get_type("struct.vector").unwrap(),
                };

                for dim in dims {
                    match dim {
                        Some(d) => aty = aty.array_type(d.to_u32().unwrap()),
                        None => return self.module.get_type("struct.vector").unwrap(),
                    }
                }

                BasicTypeEnum::ArrayType(aty)
            }
            resolver::Type::Struct(n) => self
                .context
                .struct_type(
                    &self.ns.structs[*n]
                        .fields
                        .iter()
                        .map(|f| self.llvm_var(&f.ty))
                        .collect::<Vec<BasicTypeEnum>>(),
                    false,
                )
                .as_basic_type_enum(),
            resolver::Type::Undef => unreachable!(),
            resolver::Type::Mapping(_, _) => unreachable!(),
            resolver::Type::Ref(r) => self
                .llvm_type(r)
                .ptr_type(AddressSpace::Generic)
                .as_basic_type_enum(),
            resolver::Type::StorageRef(_) => {
                BasicTypeEnum::IntType(self.context.custom_width_int_type(256))
            }
        }
    }

    /// ewasm deployer needs to know what its own code size is, so we compile once to
    /// get the size, patch in the value and then recompile.
    fn patch_code_size(&self, code_size: u64) -> bool {
        let current_size = {
            let current_size_opt = self.code_size.borrow();

            if let Some(current_size) = *current_size_opt {
                if code_size == current_size.get_zero_extended_constant().unwrap() {
                    return false;
                }

                current_size
            } else {
                return false;
            }
        };

        let new_size = self.context.i32_type().const_int(code_size, false);

        current_size.replace_all_uses_with(new_size);

        self.code_size.replace(Some(new_size));

        true
    }
}

impl resolver::Type {
    /// Does this type need a phi node
    pub fn needs_phi(&self) -> bool {
        match self {
            resolver::Type::Bool
            | resolver::Type::Int(_)
            | resolver::Type::Uint(_)
            | resolver::Type::Address
            | resolver::Type::Bytes(_) => !self.stack_based(),
            resolver::Type::Enum(_) => true,
            resolver::Type::Struct(_) => true,
            resolver::Type::Array(_, _) => true,
            resolver::Type::DynamicBytes => true,
            resolver::Type::String => true,
            resolver::Type::Mapping(_, _) => true,
            resolver::Type::Contract(_) => true,
            resolver::Type::Ref(r) => r.needs_phi(),
            resolver::Type::StorageRef(_) => true,
            resolver::Type::Undef => unreachable!(),
        }
    }

    /// Should this value be stored in alloca'ed space
    pub fn stack_based(&self) -> bool {
        match self {
            resolver::Type::Bool => false,
            resolver::Type::Int(n) => *n > 64,
            resolver::Type::Uint(n) => *n > 64,
            resolver::Type::Contract(_) | resolver::Type::Address => true,
            resolver::Type::Bytes(n) => *n > 8,
            resolver::Type::Enum(_) => false,
            resolver::Type::Struct(_) => true,
            resolver::Type::Array(_, _) => true,
            resolver::Type::String => true,
            resolver::Type::DynamicBytes => true,
            resolver::Type::Mapping(_, _) => true,
            resolver::Type::Undef => unreachable!(),
            resolver::Type::Ref(_) => false,
            resolver::Type::StorageRef(r) => r.stack_based(),
        }
    }
}

static STDLIB_IR: &[u8] = include_bytes!("../../stdlib/stdlib.bc");
static SHA3_IR: &[u8] = include_bytes!("../../stdlib/sha3.bc");
static SUBSTRATE_IR: &[u8] = include_bytes!("../../stdlib/substrate.bc");

/// Return the stdlib as parsed llvm module. The solidity standard library is hardcoded into
/// the solang library
fn load_stdlib<'a>(context: &'a Context, target: &crate::Target) -> Module<'a> {
    let memory = MemoryBuffer::create_from_memory_range(STDLIB_IR, "stdlib");

    let module = Module::parse_bitcode_from_buffer(&memory, context).unwrap();

    if let super::Target::Substrate = target {
        let memory = MemoryBuffer::create_from_memory_range(SUBSTRATE_IR, "substrate");

        module
            .link_in_module(Module::parse_bitcode_from_buffer(&memory, context).unwrap())
            .unwrap();
    } else {
        // Substrate provides a keccak256 (sha3) host function, others do not
        let memory = MemoryBuffer::create_from_memory_range(SHA3_IR, "sha3");

        module
            .link_in_module(Module::parse_bitcode_from_buffer(&memory, context).unwrap())
            .unwrap();
    }

    module
}
