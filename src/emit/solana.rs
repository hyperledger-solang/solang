use codegen::cfg::HashTy;
use parser::pt;
use sema::ast;
use std::collections::HashMap;
use std::str;

use inkwell::context::Context;
use inkwell::types::IntType;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue, UnnamedAddress};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;

use super::ethabiencoder;
use super::{Contract, TargetRuntime, Variable};

pub struct SolanaTarget {
    abi: ethabiencoder::EthAbiEncoder,
}

impl SolanaTarget {
    pub fn build<'a>(
        context: &'a Context,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        filename: &'a str,
        opt: OptimizationLevel,
    ) -> Contract<'a> {
        let mut b = SolanaTarget {
            abi: ethabiencoder::EthAbiEncoder {},
        };

        let mut c = Contract::new(context, contract, ns, filename, opt, None);

        // externals
        b.declare_externals(&mut c);

        b.emit_functions(&mut c);

        b.emit_constructor(&mut c);
        b.emit_function(&mut c);

        c.internalize(&["entrypoint", "sol_log_", "sol_alloc_free_"]);

        c
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

    fn emit_constructor(&mut self, contract: &mut Contract) {
        let initializer = self.emit_initializer(contract);

        let function = contract.module.get_function("solang_constructor").unwrap();

        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(entry);

        let argsdata = function.get_nth_param(0).unwrap().into_pointer_value();
        let argslen = function.get_nth_param(1).unwrap().into_int_value();

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
            self.decode(
                contract,
                function,
                &mut args,
                argsdata,
                argslen,
                &cfg.params,
            );

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
    fn emit_function<'s>(&'s mut self, contract: &'s mut Contract) {
        let function = contract.module.get_function("solang_function").unwrap();

        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(entry);

        let argsdata = function.get_nth_param(0).unwrap().into_pointer_value();
        let argslen = function.get_nth_param(1).unwrap().into_int_value();

        let argsdata = contract.builder.build_pointer_cast(
            argsdata,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "argsdata32",
        );

        self.emit_function_dispatch(
            contract,
            pt::FunctionTy::Function,
            argsdata,
            argslen,
            function,
            None,
            |_| false,
        );
    }

    /// abi decode the encoded data into the BasicValueEnums
    pub fn decode<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        args: &mut Vec<BasicValueEnum<'a>>,
        data: PointerValue<'a>,
        data_length: IntValue<'a>,
        spec: &[ast::Parameter],
    ) {
        let data = contract.builder.build_pointer_cast(
            data,
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "data",
        );

        let mut offset = contract.context.i64_type().const_zero();

        for arg in spec {
            args.push(self.decode_primitive(
                contract,
                function,
                &arg.ty,
                None,
                &mut offset,
                data,
                data_length,
            ));
        }
    }

    // abi decode a single primitive
    /// decode a single primitive which is always encoded in 32 bytes
    fn decode_primitive<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        ty: &ast::Type,
        to: Option<PointerValue<'a>>,
        offset: &mut IntValue<'a>,
        data: PointerValue<'a>,
        length: IntValue,
    ) -> BasicValueEnum<'a> {
        // TODO: investigate whether we can use build_int_nuw_add() and avoid 64 bit conversions
        let new_offset = contract.builder.build_int_add(
            *offset,
            contract.context.i64_type().const_int(32, false),
            "next_offset",
        );

        self.check_overrun(contract, function, new_offset, length);

        let data = unsafe { contract.builder.build_gep(data, &[*offset], "") };

        *offset = new_offset;

        let ty = if let ast::Type::Enum(n) = ty {
            &contract.ns.enums[*n].ty
        } else {
            ty
        };

        match &ty {
            ast::Type::Bool => {
                // solidity checks all the 32 bytes for being non-zero; we will just look at the upper 8 bytes, else we would need four loads
                // which is unneeded (hopefully)
                // cast to 64 bit pointer
                let bool_ptr = contract.builder.build_pointer_cast(
                    data,
                    contract.context.i64_type().ptr_type(AddressSpace::Generic),
                    "",
                );

                let bool_ptr = unsafe {
                    contract.builder.build_gep(
                        bool_ptr,
                        &[contract.context.i32_type().const_int(3, false)],
                        "bool_ptr",
                    )
                };

                let val = contract.builder.build_int_compare(
                    IntPredicate::NE,
                    contract
                        .builder
                        .build_load(bool_ptr, "abi_bool")
                        .into_int_value(),
                    contract.context.i64_type().const_zero(),
                    "bool",
                );
                if let Some(p) = to {
                    contract.builder.build_store(p, val);
                }
                val.into()
            }
            ast::Type::Uint(8) | ast::Type::Int(8) => {
                let int8_ptr = unsafe {
                    contract.builder.build_gep(
                        data,
                        &[contract.context.i32_type().const_int(31, false)],
                        "uint8_ptr",
                    )
                };

                let val = contract.builder.build_load(int8_ptr, "int8");

                if let Some(p) = to {
                    contract.builder.build_store(p, val);
                }

                val
            }
            ast::Type::Uint(n) | ast::Type::Int(n) if *n == 16 || *n == 32 || *n == 64 => {
                // our value is big endian, 32 bytes. So, find the offset within the 32 bytes
                // where our value starts
                let int8_ptr = unsafe {
                    contract.builder.build_gep(
                        data,
                        &[contract
                            .context
                            .i32_type()
                            .const_int(32 - (*n as u64 / 8), false)],
                        "uint8_ptr",
                    )
                };

                let val = contract.builder.build_load(
                    contract.builder.build_pointer_cast(
                        int8_ptr,
                        contract
                            .context
                            .custom_width_int_type(*n as u32)
                            .ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    &format!("be{}", *n),
                );

                // now convert to le
                let bswap = contract.llvm_bswap(*n as u32);

                let val = contract
                    .builder
                    .build_call(bswap, &[val], "")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                if let Some(p) = to {
                    contract.builder.build_store(p, val);
                }

                val.into()
            }
            ast::Type::Uint(n) | ast::Type::Int(n) if *n < 64 => {
                let uint64_ptr = contract.builder.build_pointer_cast(
                    data,
                    contract.context.i64_type().ptr_type(AddressSpace::Generic),
                    "",
                );

                let uint64_ptr = unsafe {
                    contract.builder.build_gep(
                        uint64_ptr,
                        &[contract.context.i32_type().const_int(3, false)],
                        "uint64_ptr",
                    )
                };

                let bswap = contract.llvm_bswap(64);

                // load and bswap
                let val = contract
                    .builder
                    .build_call(
                        bswap,
                        &[contract.builder.build_load(uint64_ptr, "uint64")],
                        "",
                    )
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                let val = contract.builder.build_right_shift(
                    val,
                    contract.context.i64_type().const_int(64 - *n as u64, false),
                    ty.is_signed_int(),
                    "",
                );

                let int_type = contract.context.custom_width_int_type(*n as u32);

                let val = contract.builder.build_int_truncate(val, int_type, "");

                val.into()
            }
            ast::Type::Bytes(1) => {
                let val = contract.builder.build_load(data, "bytes1");

                if let Some(p) = to {
                    contract.builder.build_store(p, val);
                }
                val
            }
            _ => unreachable!(),
        }
    }

    /// Check that data has not overrun end
    fn check_overrun(
        &self,
        contract: &Contract,
        function: FunctionValue,
        offset: IntValue,
        end: IntValue,
    ) {
        let in_bounds = contract
            .builder
            .build_int_compare(IntPredicate::ULE, offset, end, "");

        let success_block = contract.context.append_basic_block(function, "success");
        let bail_block = contract.context.append_basic_block(function, "bail");
        contract
            .builder
            .build_conditional_branch(in_bounds, success_block, bail_block);

        contract.builder.position_at_end(bail_block);

        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_int(3, false)));

        contract.builder.position_at_end(success_block);
    }
}

impl<'a> TargetRuntime<'a> for SolanaTarget {
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
        // return 0 for success
        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_int(0, false)));
    }

    fn return_abi<'b>(&self, _contract: &'b Contract, _data: PointerValue<'b>, _length: IntValue) {
        unimplemented!();
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

    fn abi_encode<'b>(
        &self,
        contract: &Contract<'b>,
        selector: Option<IntValue<'b>>,
        load: bool,
        function: FunctionValue,
        args: &[BasicValueEnum<'b>],
        spec: &[ast::Parameter],
    ) -> (PointerValue<'b>, IntValue<'b>) {
        let mut offset = contract.context.i32_type().const_int(
            spec.iter()
                .map(|arg| self.abi.encoded_fixed_length(&arg.ty, contract.ns))
                .sum(),
            false,
        );

        let mut length = offset;

        // now add the dynamic lengths
        for (i, s) in spec.iter().enumerate() {
            length = contract.builder.build_int_add(
                length,
                self.abi
                    .encoded_dynamic_length(args[i], load, &s.ty, function, contract),
                "",
            );
        }

        if selector.is_some() {
            length = contract.builder.build_int_add(
                length,
                contract
                    .context
                    .i32_type()
                    .const_int(std::mem::size_of::<u32>() as u64, false),
                "",
            );
        }

        let encoded_data = contract
            .builder
            .build_call(
                contract.module.get_function("solang_malloc").unwrap(),
                &[length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // malloc returns u8*
        let mut data = encoded_data;

        if let Some(selector) = selector {
            contract.builder.build_store(
                contract.builder.build_pointer_cast(
                    data,
                    contract.context.i32_type().ptr_type(AddressSpace::Generic),
                    "",
                ),
                selector,
            );

            data = unsafe {
                contract.builder.build_gep(
                    data,
                    &[contract
                        .context
                        .i32_type()
                        .const_int(std::mem::size_of_val(&selector) as u64, false)],
                    "",
                )
            };
        }

        // We use a little trick here. The length might or might not include the selector.
        // The length will be a multiple of 32 plus the selector (4). So by dividing by 8,
        // we lose the selector.
        // contract.builder.build_call(
        //     contract.module.get_function("__bzero8").unwrap(),
        //     &[
        //         data.into(),
        //         contract
        //             .builder
        //             .build_int_unsigned_div(
        //                 length,
        //                 contract.context.i32_type().const_int(8, false),
        //                 "",
        //             )
        //             .into(),
        //     ],
        //     "",
        // );

        let mut dynamic = unsafe { contract.builder.build_gep(data, &[offset], "") };

        for (i, arg) in spec.iter().enumerate() {
            self.abi.encode_ty(
                contract,
                load,
                function,
                &arg.ty,
                args[i],
                &mut data,
                &mut offset,
                &mut dynamic,
            );
        }

        (encoded_data, length)
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
        self.decode(contract, function, args, data, length, spec);
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

    /// Return the current address
    fn get_address<'b>(&self, contract: &Contract<'b>) -> IntValue<'b> {
        contract.address_type().const_zero()
    }

    /// Return the balance for address
    fn balance<'b>(&self, _contract: &Contract<'b>, _addr: IntValue<'b>) -> IntValue<'b> {
        unimplemented!();
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
