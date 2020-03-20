use resolver;
use std::str;

use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::IntPredicate;

use super::ethabiencoder;
use super::{Contract, TargetRuntime};

pub struct SabreTarget {
    abi: ethabiencoder::EthAbiEncoder,
}

impl SabreTarget {
    pub fn build<'a>(
        context: &'a Context,
        contract: &'a resolver::Contract,
        filename: &'a str,
    ) -> Contract<'a> {
        let mut c = Contract::new(context, contract, filename, None);
        let b = SabreTarget {
            abi: ethabiencoder::EthAbiEncoder {},
        };

        // externals
        b.declare_externals(&mut c);

        c.emit_functions(&b);

        b.emit_entrypoint(&mut c);

        c.internalize(&[
            "entrypoint",
            "get_ptr_len",
            "delete_state",
            "get_state",
            "set_state",
            "create_collection",
            "add_to_collection",
            "alloc",
        ]);

        c
    }

    fn declare_externals(&self, contract: &mut Contract) {
        let u8_ptr = contract.context.i8_type().ptr_type(AddressSpace::Generic);
        contract.module.add_function(
            "get_ptr_len",
            contract.context.i32_type().fn_type(&[u8_ptr.into()], false),
            Some(Linkage::External),
        );
        contract.module.add_function(
            "delete_state",
            u8_ptr.fn_type(&[u8_ptr.into()], false),
            Some(Linkage::External),
        );
        contract.module.add_function(
            "set_state",
            u8_ptr.fn_type(&[u8_ptr.into()], false),
            Some(Linkage::External),
        );
        contract.module.add_function(
            "get_state",
            u8_ptr.fn_type(&[u8_ptr.into()], false),
            Some(Linkage::External),
        );
        contract.module.add_function(
            "create_collection",
            u8_ptr.fn_type(&[u8_ptr.into()], false),
            Some(Linkage::External),
        );
        contract.module.add_function(
            "add_to_collection",
            u8_ptr.fn_type(&[u8_ptr.into(), u8_ptr.into()], false),
            Some(Linkage::External),
        );
        contract.module.add_function(
            "alloc",
            u8_ptr.fn_type(&[contract.context.i32_type().into()], false),
            Some(Linkage::External),
        );
    }

    fn emit_entrypoint(&self, contract: &mut Contract) {
        let initializer = contract.emit_initializer(self);

        let bytes_ptr = contract.context.i32_type().ptr_type(AddressSpace::Generic);

        // create start function
        let ret = contract.context.i32_type();
        let ftype = ret.fn_type(
            &[bytes_ptr.into(), bytes_ptr.into(), bytes_ptr.into()],
            false,
        );
        let function = contract.module.add_function("entrypoint", ftype, None);

        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(entry);

        // we should not use our heap; use sabre provided heap instead
        let argsdata = function.get_first_param().unwrap().into_pointer_value();
        let argslen = contract
            .builder
            .build_call(
                contract.module.get_function("get_ptr_len").unwrap(),
                &[contract
                    .builder
                    .build_pointer_cast(
                        argsdata,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "argsdata",
                    )
                    .into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // We now have a reference to the abi encoded data
        // Either this is a constructor call or a function call. A function call always starts with four
        // bytes of function selector followed by a multiple of 32 bytes.
        let is_function_call = contract.builder.build_int_compare(
            IntPredicate::EQ,
            contract.builder.build_and(
                argslen,
                contract.context.i32_type().const_int(31, false),
                "",
            ),
            contract.context.i32_type().const_int(4, false),
            "is_function_call",
        );

        let function_block = contract
            .context
            .append_basic_block(function, "function_call");
        let constructor_block = contract
            .context
            .append_basic_block(function, "constructor_call");

        contract.builder.build_conditional_branch(
            is_function_call,
            function_block,
            constructor_block,
        );

        contract.builder.position_at_end(constructor_block);

        // init our storage vars
        contract.builder.build_call(initializer, &[], "");

        if let Some(con) = contract.ns.constructors.get(0) {
            let mut args = Vec::new();

            // insert abi decode
            self.abi
                .decode(contract, function, &mut args, argsdata, argslen, con);

            contract
                .builder
                .build_call(contract.constructors[0], &args, "");
        }

        // return 1 for success
        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_int(1, false)));

        contract.builder.position_at_end(function_block);

        let fallback_block = contract.context.append_basic_block(function, "fallback");

        contract.emit_function_dispatch(
            &contract.ns.functions,
            &contract.functions,
            argsdata,
            argslen,
            function,
            fallback_block,
            self,
        );

        // emit fallback code
        contract.builder.position_at_end(fallback_block);

        match contract.ns.fallback_function() {
            Some(f) => {
                contract.builder.build_call(contract.functions[f], &[], "");

                // return 1 for success
                contract
                    .builder
                    .build_return(Some(&contract.context.i32_type().const_int(1, false)));
            }
            None => {
                // return -3 for failure
                contract.builder.build_return(Some(
                    &contract.context.i32_type().const_int(-3i64 as u64, true),
                ));
            }
        }
    }
}

impl TargetRuntime for SabreTarget {
    fn clear_storage<'a>(
        &self,
        contract: &'a Contract,
        _function: FunctionValue,
        slot: PointerValue<'a>,
    ) {
        let address = contract
            .builder
            .build_call(
                contract.module.get_function("alloc").unwrap(),
                &[contract.context.i32_type().const_int(64, false).into()],
                "address",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // convert slot to address
        contract.builder.build_call(
            contract.module.get_function("__u256ptohex").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "slot",
                    )
                    .into(),
                address.into(),
            ],
            "address_from_slot",
        );

        // create collection for delete_state
        contract.builder.build_call(
            contract.module.get_function("create_collection").unwrap(),
            &[address.into()],
            "",
        );

        contract.builder.build_call(
            contract.module.get_function("delete_state").unwrap(),
            &[address.into()],
            "",
        );
    }

    fn set_storage<'a>(
        &self,
        contract: &'a Contract,
        _function: FunctionValue,
        slot: PointerValue<'a>,
        dest: PointerValue<'a>,
    ) {
        let address = contract
            .builder
            .build_call(
                contract.module.get_function("alloc").unwrap(),
                &[contract.context.i32_type().const_int(64, false).into()],
                "address",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // convert slot to address
        contract.builder.build_call(
            contract.module.get_function("__u256ptohex").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "slot",
                    )
                    .into(),
                address.into(),
            ],
            "address_from_slot",
        );

        let data_size = dest
            .get_type()
            .get_element_type()
            .into_int_type()
            .size_of()
            .const_cast(contract.context.i32_type(), false);

        let data = contract
            .builder
            .build_call(
                contract.module.get_function("alloc").unwrap(),
                &[data_size.into()],
                "data",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // store data in pointer collection
        let dest = contract.builder.build_pointer_cast(
            dest,
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "dest",
        );

        contract.builder.build_call(
            contract.module.get_function("__memcpy").unwrap(),
            &[data.into(), dest.into(), data_size.into()],
            "destdata",
        );

        // create collection for set_state
        contract.builder.build_call(
            contract.module.get_function("create_collection").unwrap(),
            &[address.into()],
            "",
        );
        contract.builder.build_call(
            contract.module.get_function("add_to_collection").unwrap(),
            &[address.into(), data.into()],
            "",
        );
        contract.builder.build_call(
            contract.module.get_function("set_state").unwrap(),
            &[address.into()],
            "",
        );
    }

    fn get_storage<'a>(
        &self,
        contract: &'a Contract,
        function: FunctionValue,
        slot: PointerValue<'a>,
        dest: PointerValue<'a>,
    ) {
        let address = contract
            .builder
            .build_call(
                contract.module.get_function("alloc").unwrap(),
                &[contract.context.i32_type().const_int(64, false).into()],
                "address",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // convert slot to address
        contract.builder.build_call(
            contract.module.get_function("__u256ptohex").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "slot",
                    )
                    .into(),
                address.into(),
            ],
            "address_from_slot",
        );

        // create collection for set_state
        contract.builder.build_call(
            contract.module.get_function("create_collection").unwrap(),
            &[address.into()],
            "",
        );
        let res = contract
            .builder
            .build_call(
                contract.module.get_function("get_state").unwrap(),
                &[address.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let state_size = contract
            .builder
            .build_call(
                contract.module.get_function("get_ptr_len").unwrap(),
                &[res.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let data_size = dest
            .get_type()
            .get_element_type()
            .into_int_type()
            .size_of()
            .const_cast(contract.context.i32_type(), false);

        let exists = contract.builder.build_int_compare(
            IntPredicate::EQ,
            state_size,
            data_size,
            "storage_exists",
        );

        let clear_block = contract
            .context
            .append_basic_block(function, "not_in_storage");
        let retrieve_block = contract.context.append_basic_block(function, "in_storage");
        let done_storage = contract
            .context
            .append_basic_block(function, "done_storage");

        contract
            .builder
            .build_conditional_branch(exists, retrieve_block, clear_block);

        contract.builder.position_at_end(retrieve_block);

        let dest = contract.builder.build_pointer_cast(
            dest,
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "dest",
        );

        contract.builder.build_call(
            contract.module.get_function("__memcpy").unwrap(),
            &[dest.into(), res.into(), data_size.into()],
            "copy_from_storage",
        );

        contract.builder.build_unconditional_branch(done_storage);

        contract.builder.position_at_end(clear_block);

        contract.builder.build_call(
            contract.module.get_function("__memset").unwrap(),
            &[
                dest.into(),
                contract.context.i8_type().const_zero().into(),
                data_size.into(),
            ],
            "clear_storage",
        );
        contract.builder.build_unconditional_branch(done_storage);

        contract.builder.position_at_end(done_storage);
    }

    fn return_empty_abi(&self, contract: &Contract) {
        // return 1 for success
        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_int(1, false)));
    }

    fn return_abi<'b>(&self, contract: &'b Contract, _data: PointerValue<'b>, _length: IntValue) {
        // FIXME: how to return abi encoded return data?
        // return 1 for success
        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_int(1, false)));
    }

    fn assert_failure<'b>(&self, contract: &'b Contract) {
        contract.builder.build_unreachable();
    }

    fn abi_encode<'b>(
        &self,
        contract: &'b Contract,
        function: FunctionValue,
        args: &[BasicValueEnum<'b>],
        spec: &resolver::FunctionDecl,
    ) -> (PointerValue<'b>, IntValue<'b>) {
        let length = contract.context.i32_type().const_int(
            spec.returns
                .iter()
                .map(|arg| self.abi.encoded_length(&arg.ty, contract.ns))
                .sum(),
            false,
        );
        let encoded_data = contract
            .builder
            .build_call(
                contract.module.get_function("alloc").unwrap(),
                &[length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // malloc returns u8*
        let mut data = encoded_data;

        for (i, arg) in spec.returns.iter().enumerate() {
            self.abi
                .encode_ty(contract, function, &arg.ty, args[i], &mut data);
        }

        (encoded_data, length)
    }

    fn abi_decode<'b>(
        &self,
        contract: &'b Contract,
        function: FunctionValue,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue,
        spec: &resolver::FunctionDecl,
    ) {
        self.abi
            .decode(contract, function, args, data, length, spec);
    }
}
