use codegen::cfg::HashTy;
use parser::pt;
use sema::ast;
use std::collections::HashMap;
use std::str;

use inkwell::context::Context;
use inkwell::types::IntType;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue, UnnamedAddress};
use inkwell::AddressSpace;
use inkwell::OptimizationLevel;

use super::ethabiencoder;
use super::{Contract, TargetRuntime, Variable};

pub struct SolanaTarget<'a> {
    abi: ethabiencoder::EthAbiEncoder,
    output: PointerValue<'a>,
    output_len: PointerValue<'a>,
}

// Implement the Solana target which uses BPF
impl<'s> SolanaTarget<'s> {
    pub fn build<'a>(
        context: &'a Context,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        filename: &'a str,
        opt: OptimizationLevel,
    ) -> Contract<'a> {
        let undef = context
            .i8_type()
            .ptr_type(AddressSpace::Generic)
            .get_undef();

        let mut target = SolanaTarget {
            abi: ethabiencoder::EthAbiEncoder { bswap: true },
            output: undef,
            output_len: undef,
        };

        let mut con = Contract::new(context, contract, ns, filename, opt, None);

        // externals
        target.declare_externals(&mut con);

        target.emit_functions(&mut con);

        target.emit_constructor(&mut con);
        target.emit_function(&mut con);

        con.internalize(&["entrypoint", "sol_log_", "sol_alloc_free_"]);

        con
    }

    fn declare_externals(&self, contract: &mut Contract) {
        let void_ty = contract.context.void_type();
        let u8_ptr = contract.context.i8_type().ptr_type(AddressSpace::Generic);
        let u64_ty = contract.context.i64_type();

        let function = contract.module.add_function(
            "sol_alloc_free_",
            u8_ptr.fn_type(&[u8_ptr.into(), u64_ty.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = contract.module.add_function(
            "sol_log_",
            void_ty.fn_type(&[u8_ptr.into(), u64_ty.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);
    }

    fn emit_constructor(&mut self, contract: &mut Contract<'s>) {
        let initializer = self.emit_initializer(contract);

        let function = contract.module.get_function("solang_constructor").unwrap();

        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(entry);

        let input = function.get_nth_param(0).unwrap().into_pointer_value();
        let input_len = function.get_nth_param(1).unwrap().into_int_value();
        self.output = function.get_nth_param(2).unwrap().into_pointer_value();
        self.output_len = function.get_nth_param(3).unwrap().into_pointer_value();

        // init our storage vars
        contract.builder.build_call(initializer, &[], "");

        // There is only one possible constructor
        let ret = if let Some((cfg_no, cfg)) = contract
            .contract
            .cfg
            .iter()
            .enumerate()
            .find(|(_, cfg)| cfg.ty == pt::FunctionTy::Constructor)
        {
            let mut args = Vec::new();

            // insert abi decode
            self.abi
                .decode(contract, function, &mut args, input, input_len, &cfg.params);

            contract
                .builder
                .build_call(contract.functions[&cfg_no], &args, "")
                .try_as_basic_value()
                .left()
                .unwrap()
        } else {
            // return 0 for success
            contract.context.i32_type().const_int(0, false).into()
        };

        contract.builder.build_return(Some(&ret));
    }

    // emit function dispatch
    fn emit_function(&mut self, contract: &mut Contract<'s>) {
        let function = contract.module.get_function("solang_function").unwrap();

        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(entry);

        let input = function.get_nth_param(0).unwrap().into_pointer_value();
        let input_len = function.get_nth_param(1).unwrap().into_int_value();
        self.output = function.get_nth_param(2).unwrap().into_pointer_value();
        self.output_len = function.get_nth_param(3).unwrap().into_pointer_value();

        let input = contract.builder.build_pointer_cast(
            input,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "input_ptr32",
        );

        self.emit_function_dispatch(
            contract,
            pt::FunctionTy::Function,
            input,
            input_len,
            function,
            None,
            |_| false,
        );
    }
}

impl<'a> TargetRuntime<'a> for SolanaTarget<'a> {
    fn clear_storage(&self, _contract: &Contract, _function: FunctionValue, _slot: PointerValue) {
        unimplemented!();
    }

    fn set_storage(
        &self,
        _contract: &Contract,
        _function: FunctionValue,
        _slot: PointerValue,
        _dest: PointerValue,
    ) {
        unimplemented!();
    }

    fn set_storage_extfunc(
        &self,
        _contract: &Contract,
        _function: FunctionValue,
        _slot: PointerValue,
        _dest: PointerValue,
    ) {
        unimplemented!();
    }
    fn get_storage_extfunc(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        unimplemented!();
    }

    fn set_storage_string(
        &self,
        _contract: &Contract,
        _function: FunctionValue,
        _slot: PointerValue,
        _dest: PointerValue,
    ) {
        unimplemented!();
    }

    fn get_storage_string(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        unimplemented!();
    }
    fn get_storage_bytes_subscript(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
        _index: IntValue<'a>,
    ) -> IntValue<'a> {
        unimplemented!();
    }
    fn set_storage_bytes_subscript(
        &self,
        _contract: &Contract,
        _function: FunctionValue,
        _slot: PointerValue,
        _index: IntValue,
        _val: IntValue,
    ) {
        unimplemented!();
    }
    fn storage_bytes_push(
        &self,
        _contract: &Contract,
        _function: FunctionValue,
        _slot: PointerValue,
        _val: IntValue,
    ) {
        unimplemented!();
    }
    fn storage_bytes_pop(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> IntValue<'a> {
        unimplemented!();
    }
    fn storage_string_length(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> IntValue<'a> {
        unimplemented!();
    }

    fn get_storage_int(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue,
        _ty: IntType<'a>,
    ) -> IntValue<'a> {
        unimplemented!();
    }

    /// sabre has no keccak256 host function, so call our implementation
    fn keccak256_hash(
        &self,
        contract: &Contract,
        src: PointerValue,
        length: IntValue,
        dest: PointerValue,
    ) {
        contract.builder.build_call(
            contract.module.get_function("sha3").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        src,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "src",
                    )
                    .into(),
                length.into(),
                contract
                    .builder
                    .build_pointer_cast(
                        dest,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "dest",
                    )
                    .into(),
                contract.context.i32_type().const_int(32, false).into(),
            ],
            "",
        );
    }

    fn return_empty_abi(&self, contract: &Contract) {
        contract
            .builder
            .build_store(self.output_len, contract.context.i64_type().const_zero());

        // return 0 for success
        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_int(0, false)));
    }

    fn return_abi<'b>(&self, contract: &'b Contract, _data: PointerValue<'b>, _length: IntValue) {
        // return data already filled in output contract

        // return 0 for success
        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_int(0, false)));
    }

    fn assert_failure<'b>(&self, _contract: &'b Contract, _data: PointerValue, _length: IntValue) {
        unimplemented!();
    }

    /// ABI encode into a vector for abi.encode* style builtin functions
    fn abi_encode_to_vector<'b>(
        &self,
        _contract: &Contract<'b>,
        _selector: Option<IntValue<'b>>,
        _function: FunctionValue,
        _packed: bool,
        _args: &[BasicValueEnum<'b>],
        _spec: &[ast::Type],
    ) -> PointerValue<'b> {
        unimplemented!();
    }

    fn abi_encode(
        &self,
        contract: &Contract<'a>,
        selector: Option<IntValue<'a>>,
        load: bool,
        function: FunctionValue,
        args: &[BasicValueEnum<'a>],
        spec: &[ast::Parameter],
    ) -> (PointerValue<'a>, IntValue<'a>) {
        let (length, mut offset) = ethabiencoder::EthAbiEncoder::total_encoded_length(
            contract, selector, load, function, args, spec,
        );

        let length64 =
            contract
                .builder
                .build_int_z_extend(length, contract.context.i64_type(), "length64");

        // FIXME ensure we have enough space for our return data
        contract.builder.build_store(self.output_len, length64);

        let mut output = self.output;

        if let Some(selector) = selector {
            contract.builder.build_store(
                contract.builder.build_pointer_cast(
                    output,
                    contract.context.i32_type().ptr_type(AddressSpace::Generic),
                    "",
                ),
                selector,
            );

            output = unsafe {
                contract.builder.build_gep(
                    output,
                    &[contract
                        .context
                        .i32_type()
                        .const_int(std::mem::size_of::<u32>() as u64, false)],
                    "",
                )
            };
        }

        // We use a little trick here. The length might or might not include the selector.
        // The length will be a multiple of 32 plus the selector (4). So by dividing by 8,
        // we lose the selector.
        contract.builder.build_call(
            contract.module.get_function("__bzero8").unwrap(),
            &[
                output.into(),
                contract
                    .builder
                    .build_int_unsigned_div(
                        length,
                        contract.context.i32_type().const_int(8, false),
                        "",
                    )
                    .into(),
            ],
            "",
        );

        let mut dynamic = unsafe { contract.builder.build_gep(output, &[offset], "") };

        for (i, arg) in spec.iter().enumerate() {
            self.abi.encode_ty(
                contract,
                load,
                function,
                &arg.ty,
                args[i],
                &mut output,
                &mut offset,
                &mut dynamic,
            );
        }

        (output, length)
    }

    fn abi_decode<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue<'b>,
        spec: &[ast::Parameter],
    ) {
        self.abi
            .decode(contract, function, args, data, length, spec);
    }

    fn print(&self, contract: &Contract, string_ptr: PointerValue, string_len: IntValue) {
        let string_len64 =
            contract
                .builder
                .build_int_z_extend(string_len, contract.context.i64_type(), "");

        contract.builder.build_call(
            contract.module.get_function("sol_log_").unwrap(),
            &[string_ptr.into(), string_len64.into()],
            "",
        );
    }

    /// Create new contract
    fn create_contract<'b>(
        &mut self,
        _contract: &Contract<'b>,
        _function: FunctionValue,
        _success: Option<&mut BasicValueEnum<'b>>,
        _contract_no: usize,
        _constructor_no: Option<usize>,
        _address: PointerValue<'b>,
        _args: &[BasicValueEnum],
        _gas: IntValue<'b>,
        _value: Option<IntValue<'b>>,
        _salt: Option<IntValue<'b>>,
    ) {
        unimplemented!();
    }

    /// Call external contract
    fn external_call<'b>(
        &self,
        _contract: &Contract<'b>,
        _function: FunctionValue,
        _success: Option<&mut BasicValueEnum<'b>>,
        _payload: PointerValue<'b>,
        _payload_len: IntValue<'b>,
        _address: PointerValue<'b>,
        _gas: IntValue<'b>,
        _value: IntValue<'b>,
        _ty: ast::CallTy,
    ) {
        unimplemented!();
    }

    /// Get return buffer for external call
    fn return_data<'b>(&self, _contract: &Contract<'b>) -> PointerValue<'b> {
        unimplemented!();
    }

    fn return_u32<'b>(&self, contract: &'b Contract, ret: IntValue<'b>) {
        contract.builder.build_return(Some(&ret));
    }

    /// Value received
    fn value_transferred<'b>(&self, contract: &Contract<'b>) -> IntValue<'b> {
        contract.value_type().const_zero()
    }

    /// Terminate execution, destroy contract and send remaining funds to addr
    fn selfdestruct<'b>(&self, _contract: &Contract<'b>, _addr: IntValue<'b>) {
        unimplemented!();
    }

    /// Send event
    fn send_event<'b>(
        &self,
        _contract: &Contract<'b>,
        _event_no: usize,
        _data: PointerValue<'b>,
        _data_len: IntValue<'b>,
        _topics: Vec<(PointerValue<'b>, IntValue<'b>)>,
    ) {
        unimplemented!();
    }

    /// builtin expressions
    fn builtin<'b>(
        &self,
        _contract: &Contract<'b>,
        _expr: &ast::Expression,
        _vartab: &HashMap<usize, Variable<'b>>,
        _function: FunctionValue<'b>,
    ) -> BasicValueEnum<'b> {
        unimplemented!();
    }

    /// Crypto Hash
    fn hash<'b>(
        &self,
        _contract: &Contract<'b>,
        _hash: HashTy,
        _input: PointerValue<'b>,
        _input_len: IntValue<'b>,
    ) -> IntValue<'b> {
        unimplemented!()
    }
}
