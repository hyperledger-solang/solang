use codegen::cfg;
use hex;
use parser::pt;
use sema::ast;
use sema::ast::{Builtin, Expression, StringLocation};
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
    ArrayValue, BasicValueEnum, FunctionValue, GlobalValue, IntValue, PhiValue, PointerValue,
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
pub struct Variable<'a> {
    value: BasicValueEnum<'a>,
}

pub trait TargetRuntime {
    fn abi_decode<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue<'b>,
        spec: &[ast::Parameter],
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
        spec: &[ast::Parameter],
    ) -> (PointerValue<'b>, IntValue<'b>);

    /// ABI encode into a vector for abi.encode* style builtin functions
    fn abi_encode_to_vector<'b>(
        &self,
        contract: &Contract<'b>,
        selector: Option<IntValue<'b>>,
        function: FunctionValue,
        packed: bool,
        args: &[BasicValueEnum<'b>],
        spec: &[ast::Type],
    ) -> PointerValue<'b>;

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

    /// Return failure code
    fn return_u32<'b>(&self, contract: &'b Contract, ret: IntValue<'b>);

    /// Return success with the ABI encoded result
    fn return_abi<'b>(&self, contract: &'b Contract, data: PointerValue<'b>, length: IntValue);

    /// Return failure without any result
    fn assert_failure<'b>(&self, contract: &'b Contract, data: PointerValue, length: IntValue);

    /// Calls constructor
    fn create_contract<'b>(
        &mut self,
        contract: &Contract<'b>,
        function: FunctionValue,
        success: Option<&mut BasicValueEnum<'b>>,
        contract_no: usize,
        constructor_no: usize,
        address: PointerValue<'b>,
        args: &[BasicValueEnum<'b>],
        gas: IntValue<'b>,
        value: Option<IntValue<'b>>,
        salt: Option<IntValue<'b>>,
    );

    /// call external function
    fn external_call<'b>(
        &self,
        contract: &Contract<'b>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        address: PointerValue<'b>,
        gas: IntValue<'b>,
        value: IntValue<'b>,
        ty: ast::CallTy,
    ) -> IntValue<'b>;

    /// builtin expressions
    fn builtin<'b>(
        &self,
        contract: &Contract<'b>,
        expr: &Expression,
        vartab: &HashMap<usize, Variable<'b>>,
        function: FunctionValue<'b>,
        runtime: &dyn TargetRuntime,
    ) -> BasicValueEnum<'b>;

    /// Return the return data from an external call (either revert error or return values)
    fn return_data<'b>(&self, contract: &Contract<'b>) -> PointerValue<'b>;

    /// Return the value we received
    fn value_transferred<'b>(&self, contract: &Contract<'b>) -> IntValue<'b>;

    /// Return the current address
    fn get_address<'b>(&self, contract: &Contract<'b>) -> IntValue<'b>;

    /// Return the balance for address
    fn balance<'b>(&self, contract: &Contract<'b>, addr: IntValue<'b>) -> IntValue<'b>;

    /// Terminate execution, destroy contract and send remaining funds to addr
    fn selfdestruct<'b>(&self, contract: &Contract<'b>, addr: IntValue<'b>);

    /// Crypto Hash
    fn hash<'b>(
        &self,
        contract: &Contract<'b>,
        hash: cfg::HashTy,
        string: PointerValue<'b>,
        length: IntValue<'b>,
    ) -> IntValue<'b>;
}

pub struct Contract<'a> {
    pub name: String,
    pub module: Module<'a>,
    pub runtime: Option<Box<Contract<'a>>>,
    function_abort_value_transfers: bool,
    constructor_abort_value_transfers: bool,
    builder: Builder<'a>,
    context: &'a Context,
    triple: TargetTriple,
    contract: &'a ast::Contract,
    ns: &'a ast::Namespace,
    functions: HashMap<String, FunctionValue<'a>>,
    wasm: RefCell<Vec<u8>>,
    opt: OptimizationLevel,
    code_size: RefCell<Option<IntValue<'a>>>,
    selector: GlobalValue<'a>,
    calldata_data: GlobalValue<'a>,
    calldata_len: GlobalValue<'a>,
}

impl<'a> Contract<'a> {
    /// Build the LLVM IR for a contract
    pub fn build(
        context: &'a Context,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
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
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
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

        // if there is no payable function, fallback or receive then abort all value transfers at the top
        // note that receive() is always payable so this just checkes for presence.
        let function_abort_value_transfers = !contract
            .functions
            .iter()
            .any(|f| !f.is_constructor() && f.is_payable());

        let constructor_abort_value_transfers = !contract
            .functions
            .iter()
            .any(|f| f.is_constructor() && f.is_payable());

        let selector =
            module.add_global(context.i32_type(), Some(AddressSpace::Generic), "selector");
        selector.set_linkage(Linkage::Internal);
        selector.set_initializer(&context.i32_type().const_zero());

        let calldata_len = module.add_global(
            context.i32_type(),
            Some(AddressSpace::Generic),
            "calldata_len",
        );
        calldata_len.set_linkage(Linkage::Internal);
        calldata_len.set_initializer(&context.i32_type().const_zero());

        let calldata_data = module.add_global(
            context.i8_type().ptr_type(AddressSpace::Generic),
            Some(AddressSpace::Generic),
            "calldata_data",
        );
        calldata_data.set_linkage(Linkage::Internal);
        calldata_data.set_initializer(
            &context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_zero(),
        );

        Contract {
            name: contract.name.to_owned(),
            module,
            runtime,
            function_abort_value_transfers,
            constructor_abort_value_transfers,
            builder: context.create_builder(),
            triple,
            context,
            contract,
            ns,
            functions: HashMap::new(),
            wasm: RefCell::new(Vec::new()),
            opt,
            code_size: RefCell::new(None),
            selector,
            calldata_data,
            calldata_len,
        }
    }

    /// llvm value type, as in chain currency (usually 128 bits int)
    fn value_type(&self) -> IntType<'a> {
        self.context
            .custom_width_int_type(self.ns.value_length as u32 * 8)
    }

    /// llvm address type
    fn address_type(&self) -> IntType<'a> {
        self.context
            .custom_width_int_type(self.ns.address_length as u32 * 8)
    }

    /// If we receive a value transfer, and we are "payable", abort with revert
    fn abort_if_value_transfer(&self, runtime: &dyn TargetRuntime, function: FunctionValue) {
        let value = runtime.value_transferred(&self);

        let got_value = self.builder.build_int_compare(
            IntPredicate::NE,
            value,
            self.value_type().const_zero(),
            "is_value_transfer",
        );

        let not_value_transfer = self
            .context
            .append_basic_block(function, "not_value_transfer");
        let abort_value_transfer = self
            .context
            .append_basic_block(function, "abort_value_transfer");

        self.builder
            .build_conditional_branch(got_value, abort_value_transfer, not_value_transfer);

        self.builder.position_at_end(abort_value_transfer);

        runtime.assert_failure(
            self,
            self.context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            self.context.i32_type().const_zero(),
        );

        self.builder.position_at_end(not_value_transfer);
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

    /// Emit all functions, constructors, fallback and receiver
    fn emit_functions(&mut self, runtime: &mut dyn TargetRuntime) {
        let mut defines = Vec::new();

        for (signature, (base_contract_no, function_no, cfg)) in &self.contract.function_table {
            let contract_name = &self.ns.contracts[*base_contract_no].name;
            let codegen_func = &self.ns.contracts[*base_contract_no].functions[*function_no];

            let name = match codegen_func.ty {
                pt::FunctionTy::Function => format!(
                    "sol::function::{}::{}",
                    contract_name,
                    codegen_func.wasm_symbol(self.ns)
                ),
                pt::FunctionTy::Constructor => format!(
                    "sol::constructor::{}{}",
                    contract_name,
                    codegen_func.wasm_symbol(self.ns)
                ),
                _ => format!("sol::{}::{}", contract_name, codegen_func.ty),
            };

            let func_decl = self.declare_function(&name, codegen_func);
            self.functions.insert(signature.to_owned(), func_decl);

            defines.push((
                func_decl,
                codegen_func,
                cfg.as_ref().expect("cfg should have been generated"),
            ));
        }

        for (func_decl, codegen_func, cfg) in defines {
            self.emit_cfg(cfg, Some(codegen_func), func_decl, runtime);
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
        vartab: &HashMap<usize, Variable<'a>>,
        function: FunctionValue<'a>,
        runtime: &dyn TargetRuntime,
    ) -> BasicValueEnum<'a> {
        match e {
            Expression::FunctionArg(_, _, pos) => function.get_nth_param(*pos as u32).unwrap(),
            Expression::BoolLiteral(_, val) => self
                .context
                .bool_type()
                .const_int(*val as u64, false)
                .into(),
            Expression::NumberLiteral(_, ty, n) => {
                self.number_literal(ty.bits(self.ns) as u32, n).into()
            }
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
            Expression::BytesLiteral(_, _, bs) => {
                let ty = self.context.custom_width_int_type((bs.len() * 8) as u32);

                // hex"11223344" should become i32 0x11223344
                let s = hex::encode(bs);

                ty.const_int_from_string(&s, StringRadix::Hexadecimal)
                    .unwrap()
                    .into()
            }
            Expression::CodeLiteral(_, contract_no, runtime) => {
                let codegen_contract = &self.ns.contracts[*contract_no];

                let target_contract =
                    Contract::build(self.context, &codegen_contract, self.ns, "", self.opt);

                // wasm
                let wasm = if *runtime && target_contract.runtime.is_some() {
                    target_contract
                        .runtime
                        .unwrap()
                        .wasm(true)
                        .expect("compile should succeeed")
                } else {
                    target_contract.wasm(true).expect("compile should succeeed")
                };

                let size = self.context.i32_type().const_int(wasm.len() as u64, false);

                let elem_size = self.context.i32_type().const_int(1, false);

                let init = self.emit_global_string(
                    &format!(
                        "code_{}_{}",
                        if *runtime { "runtime" } else { "deployer" },
                        &codegen_contract.name
                    ),
                    &wasm,
                    false,
                );

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
                            .get_struct_type("struct.vector")
                            .unwrap()
                            .ptr_type(AddressSpace::Generic),
                        "vector",
                    )
                    .into()
            }
            Expression::Add(_, _, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_int_add(left, right, "").into()
            }
            Expression::Subtract(_, _, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_int_sub(left, right, "").into()
            }
            Expression::Multiply(_, _, l, r) => {
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
            Expression::UDivide(_, _, l, r) => {
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
            Expression::SDivide(_, _, l, r) => {
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
            Expression::UModulo(_, _, l, r) => {
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
            Expression::SModulo(_, _, l, r) => {
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
            Expression::Power(_, _, l, r) => {
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
            Expression::Variable(_, _, s) => vartab[s].value,
            Expression::Load(_, _, e) => {
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
            Expression::UnaryMinus(_, _, e) => {
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
            Expression::Cast(_, _, e) => self.expression(e, vartab, function, runtime),
            Expression::Not(_, e) => {
                let e = self
                    .expression(e, vartab, function, runtime)
                    .into_int_value();

                self.builder
                    .build_int_compare(IntPredicate::EQ, e, e.get_type().const_zero(), "")
                    .into()
            }
            Expression::Complement(_, _, e) => {
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
            Expression::BitwiseOr(_, _, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_or(left, right, "").into()
            }
            Expression::BitwiseAnd(_, _, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_and(left, right, "").into()
            }
            Expression::BitwiseXor(_, _, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_xor(left, right, "").into()
            }
            Expression::ShiftLeft(_, _, l, r) => {
                let left = self
                    .expression(l, vartab, function, runtime)
                    .into_int_value();
                let right = self
                    .expression(r, vartab, function, runtime)
                    .into_int_value();

                self.builder.build_left_shift(left, right, "").into()
            }
            Expression::ShiftRight(_, _, l, r, signed) => {
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
            Expression::ArraySubscript(_, _, a, i) => {
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
            Expression::DynamicArraySubscript(_, elem_ty, a, i) => {
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
            Expression::StructMember(_, _, a, i) => {
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
            Expression::Ternary(_, _, c, l, r) => {
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
            Expression::ConstArrayLiteral(_, _, dims, exprs) => {
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
                let ty = self.llvm_type(ty);

                let p = self
                    .builder
                    .build_call(
                        self.module.get_function("__malloc").unwrap(),
                        &[ty.size_of()
                            .unwrap()
                            .const_cast(self.context.i32_type(), false)
                            .into()],
                        "array_literal",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                let array = self.builder.build_pointer_cast(
                    p.into_pointer_value(),
                    ty.ptr_type(AddressSpace::Generic),
                    "array_literal",
                );

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
                let elem = match ty {
                    ast::Type::String | ast::Type::DynamicBytes => ast::Type::Bytes(1),
                    _ => ty.array_elem(),
                };

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
                            .get_struct_type("struct.vector")
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
            Expression::Keccak256(_, _, exprs) => {
                let mut length = self.context.i32_type().const_zero();
                let mut values: Vec<(BasicValueEnum, IntValue, ast::Type)> = Vec::new();

                // first we need to calculate the length of the buffer and get the types/lengths
                for e in exprs {
                    let v = self.expression(&e, vartab, function, runtime);

                    let len = match e.ty() {
                        ast::Type::DynamicBytes | ast::Type::String => {
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

                    values.push((v, len, e.ty()));
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
                        ast::Type::DynamicBytes | ast::Type::String => {
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
            Expression::StringConcat(_, _, l, r) => {
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
                            .get_struct_type("struct.vector")
                            .unwrap()
                            .ptr_type(AddressSpace::Generic),
                        "vector",
                    )
                    .into()
            }
            Expression::ReturnData(_) => runtime.return_data(self).into(),
            Expression::GetAddress(_, _) => runtime.get_address(self).into(),
            Expression::Balance(_, _, addr) => {
                let addr = self
                    .expression(addr, vartab, function, runtime)
                    .into_int_value();

                runtime.balance(self, addr).into()
            }
            Expression::Builtin(_, _, Builtin::Calldata, _) => self
                .builder
                .build_call(
                    self.module.get_function("vector_new").unwrap(),
                    &[
                        self.builder
                            .build_load(self.calldata_len.as_pointer_value(), "calldata_len"),
                        self.context.i32_type().const_int(1, false).into(),
                        self.builder
                            .build_load(self.calldata_data.as_pointer_value(), "calldata_data"),
                    ],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap(),
            Expression::Builtin(_, _, Builtin::Signature, _) => {
                // need to byte-reverse selector
                let selector = self
                    .builder
                    .build_alloca(self.context.i32_type(), "selector");

                // byte order needs to be reversed. e.g. hex"11223344" should be 0x10 0x11 0x22 0x33 0x44
                self.builder.build_call(
                    self.module.get_function("__beNtoleN").unwrap(),
                    &[
                        self.builder
                            .build_pointer_cast(
                                self.selector.as_pointer_value(),
                                self.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        self.builder
                            .build_pointer_cast(
                                selector,
                                self.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        self.context.i32_type().const_int(4, false).into(),
                    ],
                    "",
                );

                self.builder.build_load(selector, "selector")
            }
            Expression::Builtin(_, _, Builtin::AddMod, args) => {
                let arith_ty = self.context.custom_width_int_type(512);
                let res_ty = self.context.custom_width_int_type(256);

                let x = self
                    .expression(&args[0], vartab, function, runtime)
                    .into_int_value();
                let y = self
                    .expression(&args[1], vartab, function, runtime)
                    .into_int_value();
                let k = self
                    .expression(&args[2], vartab, function, runtime)
                    .into_int_value();
                let dividend = self.builder.build_int_add(
                    self.builder.build_int_z_extend(x, arith_ty, "wide_x"),
                    self.builder.build_int_z_extend(y, arith_ty, "wide_y"),
                    "x_plus_y",
                );

                let divisor = self.builder.build_int_z_extend(k, arith_ty, "wide_k");

                let rem = self.builder.build_alloca(arith_ty, "remainder");
                let quotient = self.builder.build_alloca(arith_ty, "quotient");

                let ret = self
                    .builder
                    .build_call(
                        self.udivmod(512, runtime),
                        &[dividend.into(), divisor.into(), rem.into(), quotient.into()],
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
                    .build_load(quotient, "quotient")
                    .into_int_value();

                self.builder
                    .build_int_truncate(quotient, res_ty, "quotient")
                    .into()
            }
            Expression::Builtin(_, _, Builtin::MulMod, args) => {
                let arith_ty = self.context.custom_width_int_type(512);
                let res_ty = self.context.custom_width_int_type(256);

                let x = self
                    .expression(&args[0], vartab, function, runtime)
                    .into_int_value();
                let y = self
                    .expression(&args[1], vartab, function, runtime)
                    .into_int_value();
                let x_m = self.builder.build_alloca(arith_ty, "x_m");
                let y_m = self.builder.build_alloca(arith_ty, "x_y");
                let x_times_y_m = self.builder.build_alloca(arith_ty, "x_times_y_m");

                self.builder
                    .build_store(x_m, self.builder.build_int_z_extend(x, arith_ty, "wide_x"));
                self.builder
                    .build_store(y_m, self.builder.build_int_z_extend(y, arith_ty, "wide_y"));

                self.builder.build_call(
                    self.module.get_function("__mul32").unwrap(),
                    &[
                        self.builder
                            .build_pointer_cast(
                                x_m,
                                self.context.i32_type().ptr_type(AddressSpace::Generic),
                                "left",
                            )
                            .into(),
                        self.builder
                            .build_pointer_cast(
                                y_m,
                                self.context.i32_type().ptr_type(AddressSpace::Generic),
                                "right",
                            )
                            .into(),
                        self.builder
                            .build_pointer_cast(
                                x_times_y_m,
                                self.context.i32_type().ptr_type(AddressSpace::Generic),
                                "output",
                            )
                            .into(),
                        self.context.i32_type().const_int(512 / 32, false).into(),
                    ],
                    "",
                );
                let k = self
                    .expression(&args[2], vartab, function, runtime)
                    .into_int_value();
                let dividend = self.builder.build_load(x_times_y_m, "x_t_y");

                let divisor = self.builder.build_int_z_extend(k, arith_ty, "wide_k");

                let rem = self.builder.build_alloca(arith_ty, "remainder");
                let quotient = self.builder.build_alloca(arith_ty, "quotient");

                let ret = self
                    .builder
                    .build_call(
                        self.udivmod(512, runtime),
                        &[dividend, divisor.into(), rem.into(), quotient.into()],
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
                    .build_load(quotient, "quotient")
                    .into_int_value();

                self.builder
                    .build_int_truncate(quotient, res_ty, "quotient")
                    .into()
            }
            Expression::Builtin(_, _, _, _) => runtime.builtin(self, e, vartab, function, runtime),
            _ => panic!("{:?} not implemented", e),
        }
    }

    /// Load a string from expression or create global
    fn string_location(
        &self,
        location: &StringLocation,
        vartab: &HashMap<usize, Variable<'a>>,
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
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        function: FunctionValue,
        runtime: &dyn TargetRuntime,
    ) -> BasicValueEnum<'a> {
        match ty {
            ast::Type::Ref(ty) => self.storage_load(ty, slot, slot_ptr, function, runtime),
            ast::Type::Array(_, dim) => {
                if let Some(d) = &dim[0] {
                    let llvm_ty = self.llvm_type(ty.deref_any());
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
                    let slot_ty = ast::Type::Uint(256);

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
            ast::Type::Struct(n) => {
                let llvm_ty = self.llvm_type(ty.deref_any());
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
            ast::Type::String | ast::Type::DynamicBytes => {
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
                    self.llvm_type(ty.deref_any()).into_int_type(),
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
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        dest: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
        runtime: &dyn TargetRuntime,
    ) {
        match ty.deref_any() {
            ast::Type::Array(_, dim) => {
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

                    let slot_ty = ast::Type::Uint(256);

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
            ast::Type::Struct(n) => {
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
            ast::Type::String | ast::Type::DynamicBytes => {
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
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        function: FunctionValue<'a>,
        runtime: &dyn TargetRuntime,
    ) {
        match ty.deref_any() {
            ast::Type::Array(_, dim) => {
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
                    self.storage_clear(&ast::Type::Uint(256), slot, slot_ptr, function, runtime);
                }
            }
            ast::Type::Struct(n) => {
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
            ast::Type::Mapping(_, _) => {
                // nothing to do, step over it
            }
            _ => {
                self.builder.build_store(slot_ptr, *slot);

                runtime.clear_storage(&self, function, slot_ptr);
            }
        }
    }

    /// Emit the contract storage initializers
    fn emit_initializer(&self, runtime: &mut dyn TargetRuntime) -> FunctionValue<'a> {
        let function = self.module.add_function(
            "storage_initializers",
            self.context.i32_type().fn_type(&[], false),
            Some(Linkage::Internal),
        );

        self.emit_cfg(&self.contract.initializer, None, function, runtime);

        function
    }

    /// Emit function prototype
    fn declare_function(&self, fname: &str, f: &ast::Function) -> FunctionValue<'a> {
        // function parameters
        let mut args = f
            .params
            .iter()
            .map(|p| self.llvm_var(&p.ty))
            .collect::<Vec<BasicTypeEnum>>();

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

    #[allow(clippy::cognitive_complexity)]
    fn emit_cfg(
        &self,
        cfg: &cfg::ControlFlowGraph,
        codegen_function: Option<&ast::Function>,
        function: FunctionValue<'a>,
        runtime: &mut dyn TargetRuntime,
    ) {
        // recurse through basic blocks
        struct BasicBlock<'a> {
            bb: inkwell::basic_block::BasicBlock<'a>,
            phis: HashMap<usize, PhiValue<'a>>,
        }

        struct Work<'b> {
            bb_no: usize,
            vars: HashMap<usize, Variable<'b>>,
        }

        let mut blocks: HashMap<usize, BasicBlock> = HashMap::new();

        let create_bb = |bb_no| -> BasicBlock {
            let cfg_bb: &cfg::BasicBlock = &cfg.bb[bb_no];
            let mut phis = HashMap::new();

            let bb = self.context.append_basic_block(function, &cfg_bb.name);

            self.builder.position_at_end(bb);

            if let Some(ref cfg_phis) = cfg_bb.phis {
                for v in cfg_phis {
                    let ty = self.llvm_var(&cfg.vars[v].ty);

                    phis.insert(*v, self.builder.build_phi(ty, &cfg.vars[v].id.name));
                }
            }

            BasicBlock { bb, phis }
        };

        let mut work = VecDeque::new();

        blocks.insert(0, create_bb(0));

        // Create all the stack variables
        let mut vars = HashMap::new();

        for (no, v) in &cfg.vars {
            match v.storage {
                cfg::Storage::Local if v.ty.is_reference_type() && !v.ty.is_contract_storage() => {
                    let ty = self.llvm_type(&v.ty);

                    let p = self
                        .builder
                        .build_call(
                            self.module.get_function("__malloc").unwrap(),
                            &[ty.size_of()
                                .unwrap()
                                .const_cast(self.context.i32_type(), false)
                                .into()],
                            &v.id.name,
                        )
                        .try_as_basic_value()
                        .left()
                        .unwrap();

                    vars.insert(
                        *no,
                        Variable {
                            value: self
                                .builder
                                .build_pointer_cast(
                                    p.into_pointer_value(),
                                    ty.ptr_type(AddressSpace::Generic),
                                    &v.id.name,
                                )
                                .into(),
                        },
                    );
                }
                cfg::Storage::Local if v.ty.is_contract_storage() => {
                    vars.insert(
                        *no,
                        Variable {
                            value: self.context.custom_width_int_type(256).const_zero().into(),
                        },
                    );
                }
                cfg::Storage::Constant(_) | cfg::Storage::Contract(_)
                    if v.ty.is_reference_type() =>
                {
                    // This needs a placeholder
                    vars.insert(
                        *no,
                        Variable {
                            value: self.context.bool_type().get_undef().into(),
                        },
                    );
                }
                cfg::Storage::Local | cfg::Storage::Contract(_) | cfg::Storage::Constant(_) => {
                    let ty = self.llvm_type(&v.ty);
                    vars.insert(
                        *no,
                        Variable {
                            value: if ty.is_pointer_type() {
                                ty.into_pointer_type().const_zero().into()
                            } else {
                                ty.into_int_type().const_zero().into()
                            },
                        },
                    );
                }
            }
        }

        work.push_back(Work { bb_no: 0, vars });

        while let Some(mut w) = work.pop_front() {
            let bb = blocks.get(&w.bb_no).unwrap();

            self.builder.position_at_end(bb.bb);

            for (v, phi) in bb.phis.iter() {
                w.vars.get_mut(v).unwrap().value = (*phi).as_basic_value();
            }

            for ins in &cfg.bb[w.bb_no].instr {
                match ins {
                    cfg::Instr::Return { value } if value.is_empty() => {
                        self.builder
                            .build_return(Some(&self.context.i32_type().const_zero()));
                    }
                    cfg::Instr::Return { value } => {
                        let returns_offset = codegen_function.unwrap().params.len();
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

                        w.vars.get_mut(res).unwrap().value = value_ref;
                    }
                    cfg::Instr::Eval { expr } => {
                        self.expression(expr, &w.vars, function, runtime);
                    }
                    cfg::Instr::Constant { res, constant } => {
                        let const_expr = self.contract.variables[*constant]
                            .initializer
                            .as_ref()
                            .unwrap();
                        let value_ref = self.expression(const_expr, &w.vars, function, runtime);

                        w.vars.get_mut(res).unwrap().value = value_ref;
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
                            phi.add_incoming(&[(&w.vars[v].value, pos)]);
                        }

                        self.builder.position_at_end(pos);
                        self.builder.build_unconditional_branch(bb.bb);
                    }
                    cfg::Instr::Store { dest, pos } => {
                        let value_ref = w.vars[pos].value;
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
                                phi.add_incoming(&[(&w.vars[v].value, pos)]);
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
                                phi.add_incoming(&[(&w.vars[v].value, pos)]);
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
                        let value = w.vars[local].value;

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
                        let value = w.vars[local].value;

                        let slot = self
                            .expression(storage, &w.vars, function, runtime)
                            .into_int_value();
                        let offset = self
                            .expression(offset, &w.vars, function, runtime)
                            .into_int_value();
                        let slot_ptr = self.builder.build_alloca(slot.get_type(), "slot");
                        self.builder.build_store(slot_ptr, slot);

                        runtime.set_storage_bytes_subscript(
                            self,
                            function,
                            slot_ptr,
                            offset,
                            value.into_int_value(),
                        );
                    }
                    cfg::Instr::PushMemory {
                        res,
                        ty,
                        array,
                        value,
                    } => {
                        let a = w.vars[array].value.into_pointer_value();
                        let len = unsafe {
                            self.builder.build_gep(
                                a,
                                &[
                                    self.context.i32_type().const_zero(),
                                    self.context.i32_type().const_zero(),
                                ],
                                "array_len",
                            )
                        };
                        let a = self.builder.build_pointer_cast(
                            a,
                            self.context.i8_type().ptr_type(AddressSpace::Generic),
                            "a",
                        );
                        let llvm_ty = self.llvm_type(ty);

                        // Calculate total size for reallocation
                        let elem_ty = match ty {
                            ast::Type::Array(..) => match self.llvm_type(&ty.array_elem()) {
                                elem @ BasicTypeEnum::StructType(_) => {
                                    // We don't store structs directly in the array, instead we store references to structs
                                    elem.ptr_type(AddressSpace::Generic).as_basic_type_enum()
                                }
                                elem => elem,
                            },
                            ast::Type::DynamicBytes => self.context.i8_type().into(),
                            _ => unreachable!(),
                        };
                        let elem_size = elem_ty
                            .size_of()
                            .unwrap()
                            .const_cast(self.context.i32_type(), false);
                        let len = self.builder.build_load(len, "array_len").into_int_value();
                        let new_len = self.builder.build_int_add(
                            len,
                            self.context.i32_type().const_int(1, false),
                            "",
                        );
                        let vec_size = self
                            .module
                            .get_struct_type("struct.vector")
                            .unwrap()
                            .size_of()
                            .unwrap()
                            .const_cast(self.context.i32_type(), false);
                        let size = self.builder.build_int_mul(elem_size, new_len, "");
                        let size = self.builder.build_int_add(size, vec_size, "");

                        // Reallocate and reassign the array pointer
                        let new = self
                            .builder
                            .build_call(
                                self.module.get_function("__realloc").unwrap(),
                                &[a.into(), size.into()],
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
                        w.vars.get_mut(array).unwrap().value = dest.into();

                        // Store the value into the last element
                        let slot_ptr = unsafe {
                            self.builder.build_gep(
                                dest,
                                &[
                                    self.context.i32_type().const_zero(),
                                    self.context.i32_type().const_int(2, false),
                                    self.builder.build_int_mul(len, elem_size, ""),
                                ],
                                "data",
                            )
                        };
                        let value = self.expression(value, &w.vars, function, runtime);
                        let elem_ptr = self.builder.build_pointer_cast(
                            slot_ptr,
                            elem_ty.ptr_type(AddressSpace::Generic),
                            "element pointer",
                        );
                        self.builder.build_store(elem_ptr, value);
                        w.vars.get_mut(res).unwrap().value = value;

                        // Update the len and size field of the vector struct
                        let len_ptr = unsafe {
                            self.builder.build_gep(
                                dest,
                                &[
                                    self.context.i32_type().const_zero(),
                                    self.context.i32_type().const_zero(),
                                ],
                                "len",
                            )
                        };
                        let len_field = self.builder.build_pointer_cast(
                            len_ptr,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "len field",
                        );
                        self.builder.build_store(len_field, new_len);

                        let size_ptr = unsafe {
                            self.builder.build_gep(
                                dest,
                                &[
                                    self.context.i32_type().const_zero(),
                                    self.context.i32_type().const_int(1, false),
                                ],
                                "size",
                            )
                        };
                        let size_field = self.builder.build_pointer_cast(
                            size_ptr,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "size field",
                        );
                        self.builder.build_store(size_field, new_len);
                    }
                    cfg::Instr::PopMemory { res, ty, array } => {
                        let a = w.vars[array].value.into_pointer_value();
                        let len = unsafe {
                            self.builder.build_gep(
                                a,
                                &[
                                    self.context.i32_type().const_zero(),
                                    self.context.i32_type().const_zero(),
                                ],
                                "a_len",
                            )
                        };
                        let len = self.builder.build_load(len, "a_len").into_int_value();

                        // First check if the array is empty
                        let is_array_empty = self.builder.build_int_compare(
                            IntPredicate::EQ,
                            len,
                            self.context.i32_type().const_zero(),
                            "is_array_empty",
                        );
                        let error = self.context.append_basic_block(function, "error");
                        let pop = self.context.append_basic_block(function, "pop");
                        self.builder
                            .build_conditional_branch(is_array_empty, error, pop);

                        self.builder.position_at_end(error);
                        runtime.assert_failure(
                            self,
                            self.context
                                .i8_type()
                                .ptr_type(AddressSpace::Generic)
                                .const_null(),
                            self.context.i32_type().const_zero(),
                        );

                        self.builder.position_at_end(pop);
                        let llvm_ty = self.llvm_type(ty);

                        // Calculate total size for reallocation
                        let elem_ty = match ty {
                            ast::Type::Array(..) => match self.llvm_type(&ty.array_elem()) {
                                elem @ BasicTypeEnum::StructType(_) => {
                                    // We don't store structs directly in the array, instead we store references to structs
                                    elem.ptr_type(AddressSpace::Generic).as_basic_type_enum()
                                }
                                elem => elem,
                            },
                            ast::Type::DynamicBytes => self.context.i8_type().into(),
                            _ => unreachable!(),
                        };
                        let elem_size = elem_ty
                            .size_of()
                            .unwrap()
                            .const_cast(self.context.i32_type(), false);
                        let new_len = self.builder.build_int_sub(
                            len,
                            self.context.i32_type().const_int(1, false),
                            "",
                        );
                        let vec_size = self
                            .module
                            .get_struct_type("struct.vector")
                            .unwrap()
                            .size_of()
                            .unwrap()
                            .const_cast(self.context.i32_type(), false);
                        let size = self.builder.build_int_mul(elem_size, new_len, "");
                        let size = self.builder.build_int_add(size, vec_size, "");

                        // Get the pointer to the last element and return it
                        let slot_ptr = unsafe {
                            self.builder.build_gep(
                                a,
                                &[
                                    self.context.i32_type().const_zero(),
                                    self.context.i32_type().const_int(2, false),
                                    self.builder.build_int_mul(new_len, elem_size, ""),
                                ],
                                "data",
                            )
                        };
                        let slot_ptr = self.builder.build_pointer_cast(
                            slot_ptr,
                            elem_ty.ptr_type(AddressSpace::Generic),
                            "slot_ptr",
                        );
                        let ret_val = self.builder.build_load(slot_ptr, "");
                        w.vars.get_mut(res).unwrap().value = ret_val;

                        // Reallocate and reassign the array pointer
                        let a = self.builder.build_pointer_cast(
                            a,
                            self.context.i8_type().ptr_type(AddressSpace::Generic),
                            "a",
                        );
                        let new = self
                            .builder
                            .build_call(
                                self.module.get_function("__realloc").unwrap(),
                                &[a.into(), size.into()],
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
                        w.vars.get_mut(array).unwrap().value = dest.into();

                        // Update the len and size field of the vector struct
                        let len_ptr = unsafe {
                            self.builder.build_gep(
                                dest,
                                &[
                                    self.context.i32_type().const_zero(),
                                    self.context.i32_type().const_zero(),
                                ],
                                "len",
                            )
                        };
                        let len_field = self.builder.build_pointer_cast(
                            len_ptr,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "len field",
                        );
                        self.builder.build_store(len_field, new_len);

                        let size_ptr = unsafe {
                            self.builder.build_gep(
                                dest,
                                &[
                                    self.context.i32_type().const_zero(),
                                    self.context.i32_type().const_int(1, false),
                                ],
                                "size",
                            )
                        };
                        let size_field = self.builder.build_pointer_cast(
                            size_ptr,
                            self.context.i32_type().ptr_type(AddressSpace::Generic),
                            "size field",
                        );
                        self.builder.build_store(size_field, new_len);
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
                            &[ast::Parameter {
                                loc: pt::Loc(0, 0, 0),
                                name: "error".to_owned(),
                                ty: ast::Type::String,
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
                    cfg::Instr::Call {
                        res,
                        base,
                        func,
                        args,
                    } => {
                        let f = &self.ns.contracts[*base].functions[*func];

                        let mut parms = args
                            .iter()
                            .map(|p| self.expression(p, &w.vars, function, runtime))
                            .collect::<Vec<BasicValueEnum>>();

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
                            .build_call(self.functions[&f.vsignature], &parms, "")
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

                                let dest = w.vars[&res[i]].value;

                                if dest.is_pointer_value() && !v.ty.is_reference_type() {
                                    self.builder.build_store(dest.into_pointer_value(), val);
                                } else {
                                    w.vars.get_mut(&res[i]).unwrap().value = val;
                                }
                            }
                        }
                    }
                    cfg::Instr::Constructor {
                        success,
                        res,
                        contract_no,
                        constructor_no,
                        args,
                        value,
                        gas,
                        salt,
                    } => {
                        let args = &args
                            .iter()
                            .map(|a| self.expression(&a, &w.vars, function, runtime))
                            .collect::<Vec<BasicValueEnum>>();

                        let address = self.builder.build_alloca(self.address_type(), "address");

                        let gas = self
                            .expression(gas, &w.vars, function, runtime)
                            .into_int_value();
                        let value = value.as_ref().map(|v| {
                            self.expression(&v, &w.vars, function, runtime)
                                .into_int_value()
                        });
                        let salt = salt.as_ref().map(|v| {
                            self.expression(&v, &w.vars, function, runtime)
                                .into_int_value()
                        });

                        let success = match success {
                            Some(n) => Some(&mut w.vars.get_mut(n).unwrap().value),
                            None => None,
                        };

                        runtime.create_contract(
                            &self,
                            function,
                            success,
                            *contract_no,
                            *constructor_no,
                            self.builder.build_pointer_cast(
                                address,
                                self.context.i8_type().ptr_type(AddressSpace::Generic),
                                "address",
                            ),
                            args,
                            gas,
                            value,
                            salt,
                        );

                        w.vars.get_mut(res).unwrap().value =
                            self.builder.build_load(address, "address");
                    }
                    cfg::Instr::ExternalCall {
                        success,
                        address,
                        contract_no,
                        function_no,
                        args,
                        value,
                        gas,
                        callty,
                    } => {
                        let (payload, payload_len) = match contract_no {
                            Some(contract_no) => {
                                let dest_func =
                                    &self.ns.contracts[*contract_no].functions[*function_no];

                                let selector = dest_func.selector();

                                runtime.abi_encode(
                                    self,
                                    Some(if self.ns.target == crate::Target::Ewasm {
                                        selector.to_be()
                                    } else {
                                        selector
                                    }),
                                    false,
                                    function,
                                    &args
                                        .iter()
                                        .map(|a| self.expression(&a, &w.vars, function, runtime))
                                        .collect::<Vec<BasicValueEnum>>(),
                                    &dest_func.params,
                                )
                            }
                            None if args.is_empty() => (
                                self.context
                                    .i8_type()
                                    .ptr_type(AddressSpace::Generic)
                                    .const_null(),
                                self.context.i32_type().const_zero(),
                            ),
                            None => {
                                let raw = self
                                    .expression(&args[0], &w.vars, function, runtime)
                                    .into_pointer_value();

                                let data = unsafe {
                                    self.builder.build_gep(
                                        raw,
                                        &[
                                            self.context.i32_type().const_zero(),
                                            self.context.i32_type().const_int(2, false),
                                        ],
                                        "rawdata",
                                    )
                                };

                                let data_len = unsafe {
                                    self.builder.build_gep(
                                        raw,
                                        &[
                                            self.context.i32_type().const_zero(),
                                            self.context.i32_type().const_zero(),
                                        ],
                                        "rawdata_len",
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
                        };
                        let address = self
                            .expression(address, &w.vars, function, runtime)
                            .into_int_value();
                        let gas = self
                            .expression(gas, &w.vars, function, runtime)
                            .into_int_value();
                        let value = self
                            .expression(value, &w.vars, function, runtime)
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

                        let ret = runtime.external_call(
                            self,
                            payload,
                            payload_len,
                            addr,
                            gas,
                            value,
                            callty.clone(),
                        );

                        let is_success = self.builder.build_int_compare(
                            IntPredicate::EQ,
                            ret,
                            self.context.i32_type().const_zero(),
                            "success",
                        );

                        if let Some(success) = success {
                            w.vars.get_mut(success).unwrap().value = is_success.into();
                        } else {
                            let success_block =
                                self.context.append_basic_block(function, "success");
                            let bail_block = self.context.append_basic_block(function, "bail");
                            self.builder.build_conditional_branch(
                                is_success,
                                success_block,
                                bail_block,
                            );
                            self.builder.position_at_end(bail_block);

                            self.builder.build_return(Some(&ret));

                            self.builder.position_at_end(success_block);
                        }
                    }
                    cfg::Instr::AbiEncodeVector {
                        res,
                        tys,
                        selector,
                        packed,
                        args,
                    } => {
                        w.vars.get_mut(res).unwrap().value = runtime
                            .abi_encode_to_vector(
                                self,
                                selector.as_ref().map(|s| {
                                    self.expression(&s, &w.vars, function, runtime)
                                        .into_int_value()
                                }),
                                function,
                                *packed,
                                &args
                                    .iter()
                                    .map(|a| self.expression(&a, &w.vars, function, runtime))
                                    .collect::<Vec<BasicValueEnum>>(),
                                tys,
                            )
                            .into();
                    }
                    cfg::Instr::AbiDecode {
                        res,
                        selector,
                        exception,
                        tys,
                        data,
                    } => {
                        let v = self
                            .expression(data, &w.vars, function, runtime)
                            .into_pointer_value();

                        let mut data = unsafe {
                            self.builder.build_gep(
                                v,
                                &[
                                    self.context.i32_type().const_zero(),
                                    self.context.i32_type().const_int(2, false),
                                ],
                                "data",
                            )
                        };

                        let mut data_len = self
                            .builder
                            .build_load(
                                unsafe {
                                    self.builder.build_gep(
                                        v,
                                        &[
                                            self.context.i32_type().const_zero(),
                                            self.context.i32_type().const_zero(),
                                        ],
                                        "data_len",
                                    )
                                },
                                "data_len",
                            )
                            .into_int_value();

                        if let Some(selector) = selector {
                            let exception = exception.unwrap();

                            let pos = self.builder.get_insert_block().unwrap();

                            blocks.entry(exception).or_insert({
                                work.push_back(Work {
                                    bb_no: exception,
                                    vars: w.vars.clone(),
                                });

                                create_bb(exception)
                            });

                            self.builder.position_at_end(pos);

                            let exception_block = blocks.get(&exception).unwrap();

                            let has_selector = self.builder.build_int_compare(
                                IntPredicate::UGT,
                                data_len,
                                self.context.i32_type().const_int(4, false),
                                "has_selector",
                            );

                            let ok1 = self.context.append_basic_block(function, "ok1");

                            self.builder.build_conditional_branch(
                                has_selector,
                                ok1,
                                exception_block.bb,
                            );
                            self.builder.position_at_end(ok1);

                            let selector_data = self
                                .builder
                                .build_load(
                                    self.builder.build_pointer_cast(
                                        data,
                                        self.context.i32_type().ptr_type(AddressSpace::Generic),
                                        "selector",
                                    ),
                                    "selector",
                                )
                                .into_int_value();

                            // ewasm stores the selector little endian
                            let selector = if self.ns.target == crate::Target::Ewasm {
                                (*selector).to_be()
                            } else {
                                *selector
                            };

                            let correct_selector = self.builder.build_int_compare(
                                IntPredicate::EQ,
                                selector_data,
                                self.context.i32_type().const_int(selector as u64, false),
                                "correct_selector",
                            );

                            let ok2 = self.context.append_basic_block(function, "ok2");

                            self.builder.build_conditional_branch(
                                correct_selector,
                                ok2,
                                exception_block.bb,
                            );

                            self.builder.position_at_end(ok2);

                            data_len = self.builder.build_int_sub(
                                data_len,
                                self.context.i32_type().const_int(4, false),
                                "data_len",
                            );

                            data = unsafe {
                                self.builder.build_gep(
                                    self.builder.build_pointer_cast(
                                        data,
                                        self.context.i8_type().ptr_type(AddressSpace::Generic),
                                        "data",
                                    ),
                                    &[self.context.i32_type().const_int(4, false)],
                                    "data",
                                )
                            };
                        }

                        let mut returns = Vec::new();

                        runtime.abi_decode(self, function, &mut returns, data, data_len, &tys);

                        for (i, ret) in returns.into_iter().enumerate() {
                            w.vars.get_mut(&res[i]).unwrap().value = ret;
                        }
                    }
                    cfg::Instr::Unreachable => {
                        self.builder.build_unreachable();
                    }
                    cfg::Instr::SelfDestruct { recipient } => {
                        let recipient = self
                            .expression(recipient, &w.vars, function, runtime)
                            .into_int_value();

                        runtime.selfdestruct(self, recipient);
                    }
                    cfg::Instr::Hash { res, hash, expr } => {
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

                        w.vars.get_mut(res).unwrap().value = runtime
                            .hash(
                                &self,
                                hash.clone(),
                                self.builder.build_pointer_cast(
                                    data,
                                    self.context.i8_type().ptr_type(AddressSpace::Generic),
                                    "data",
                                ),
                                self.builder
                                    .build_load(data_len, "data_len")
                                    .into_int_value(),
                            )
                            .into();
                    }
                }
            }
        }
    }

    /// Create function dispatch based on abi encoded argsdata. The dispatcher loads the leading function selector,
    /// and dispatches based on that. If no function matches this, or no selector is in the argsdata, then fallback
    /// code is executed. This is either a fallback block provided to this function, or it automatically dispatches
    /// to the fallback function or receive function, if any.
    pub fn emit_function_dispatch<F>(
        &self,
        function_ty: pt::FunctionTy,
        argsdata: inkwell::values::PointerValue<'a>,
        argslen: inkwell::values::IntValue<'a>,
        function: inkwell::values::FunctionValue<'a>,
        fallback: Option<inkwell::basic_block::BasicBlock>,
        runtime: &dyn TargetRuntime,
        nonpayable: F,
    ) where
        F: Fn(&ast::Function) -> bool,
    {
        // create start function
        let no_function_matched = match fallback {
            Some(block) => block,
            None => self
                .context
                .append_basic_block(function, "no_function_matched"),
        };

        let switch_block = self.context.append_basic_block(function, "switch");

        let not_fallback = self.builder.build_int_compare(
            IntPredicate::UGE,
            argslen,
            self.context.i32_type().const_int(4, false),
            "",
        );

        self.builder
            .build_conditional_branch(not_fallback, switch_block, no_function_matched);

        self.builder.position_at_end(switch_block);

        let fid = self
            .builder
            .build_load(argsdata, "function_selector")
            .into_int_value();

        self.builder
            .build_store(self.selector.as_pointer_value(), fid);

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

        for (signature, (base_contract_no, function_no, _)) in &self.contract.function_table {
            let f = &self.ns.contracts[*base_contract_no].functions[*function_no];

            if f.ty != function_ty || !f.is_public() {
                continue;
            }

            if f.is_constructor() && !signature.starts_with(&format!("@{}", self.contract.name)) {
                // base constructor, no dispatch needed
                continue;
            }

            let bb = self.context.append_basic_block(function, "");

            let id = f.selector();

            self.builder.position_at_end(bb);

            if nonpayable(f) {
                self.abort_if_value_transfer(runtime, function);
            }

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
                .build_call(self.functions[&f.vsignature], &args, "")
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

            runtime.return_u32(self, ret.into_int_value());

            cases.push((self.context.i32_type().const_int(id as u64, false), bb));
        }

        self.builder.position_at_end(switch_block);

        self.builder.build_switch(fid, no_function_matched, &cases);

        if fallback.is_some() {
            return; // caller will generate fallback code
        }

        // emit fallback code
        self.builder.position_at_end(no_function_matched);

        if self.functions.get("@fallback").is_none() && self.functions.get("@receive").is_none() {
            // no need to check value transferred; we will abort either way
            runtime.return_u32(self, self.context.i32_type().const_int(2, false));

            return;
        }

        let got_value = if self.function_abort_value_transfers {
            self.context.bool_type().const_zero()
        } else {
            let value = runtime.value_transferred(self);

            self.builder.build_int_compare(
                IntPredicate::NE,
                value,
                self.value_type().const_zero(),
                "is_value_transfer",
            )
        };

        let fallback_block = self.context.append_basic_block(function, "fallback");
        let receive_block = self.context.append_basic_block(function, "receive");

        self.builder
            .build_conditional_branch(got_value, receive_block, fallback_block);

        self.builder.position_at_end(fallback_block);

        match self.functions.get("@fallback") {
            Some(f) => {
                self.builder.build_call(*f, &[], "");

                runtime.return_empty_abi(self);
            }
            None => {
                runtime.return_u32(self, self.context.i32_type().const_int(2, false));
            }
        }

        self.builder.position_at_end(receive_block);

        match self.functions.get("@receive") {
            Some(f) => {
                self.builder.build_call(*f, &[], "");

                runtime.return_empty_abi(self);
            }
            None => {
                runtime.return_u32(self, self.context.i32_type().const_int(2, false));
            }
        }
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
    fn llvm_var(&self, ty: &ast::Type) -> BasicTypeEnum<'a> {
        let llvm_ty = self.llvm_type(ty);
        match ty.deref_memory() {
            ast::Type::Struct(_)
            | ast::Type::Array(_, _)
            | ast::Type::DynamicBytes
            | ast::Type::String => llvm_ty.ptr_type(AddressSpace::Generic).as_basic_type_enum(),
            _ => llvm_ty,
        }
    }

    /// Return the llvm type for the resolved type.
    fn llvm_type(&self, ty: &ast::Type) -> BasicTypeEnum<'a> {
        match ty {
            ast::Type::Bool => BasicTypeEnum::IntType(self.context.bool_type()),
            ast::Type::Int(n) | ast::Type::Uint(n) => {
                BasicTypeEnum::IntType(self.context.custom_width_int_type(*n as u32))
            }
            ast::Type::Contract(_) | ast::Type::Address(_) => {
                BasicTypeEnum::IntType(self.address_type())
            }
            ast::Type::Bytes(n) => {
                BasicTypeEnum::IntType(self.context.custom_width_int_type(*n as u32 * 8))
            }
            ast::Type::Enum(n) => self.llvm_type(&self.ns.enums[*n].ty),
            ast::Type::String | ast::Type::DynamicBytes => {
                self.module.get_struct_type("struct.vector").unwrap().into()
            }
            ast::Type::Array(base_ty, dims) => {
                let ty = self.llvm_var(base_ty);

                let mut dims = dims.iter();

                let mut aty = match dims.next().unwrap() {
                    Some(d) => ty.array_type(d.to_u32().unwrap()),
                    None => return self.module.get_struct_type("struct.vector").unwrap().into(),
                };

                for dim in dims {
                    match dim {
                        Some(d) => aty = aty.array_type(d.to_u32().unwrap()),
                        None => {
                            return self.module.get_struct_type("struct.vector").unwrap().into()
                        }
                    }
                }

                BasicTypeEnum::ArrayType(aty)
            }
            ast::Type::Struct(n) => self
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
            ast::Type::Mapping(_, _) => unreachable!(),
            ast::Type::Ref(r) => self
                .llvm_type(r)
                .ptr_type(AddressSpace::Generic)
                .as_basic_type_enum(),
            ast::Type::StorageRef(_) => {
                BasicTypeEnum::IntType(self.context.custom_width_int_type(256))
            }
            _ => unreachable!(),
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

static STDLIB_IR: &[u8] = include_bytes!("../../stdlib/stdlib.bc");
static SHA3_IR: &[u8] = include_bytes!("../../stdlib/sha3.bc");
static RIPEMD160_IR: &[u8] = include_bytes!("../../stdlib/ripemd160.bc");
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

        // substrate does not provide ripemd160
        let memory = MemoryBuffer::create_from_memory_range(RIPEMD160_IR, "ripemd160");

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
