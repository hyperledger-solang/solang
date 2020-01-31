use resolver;
use std::str;

use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;

use super::ethabiencoder;
use super::{Contract, TargetRuntime};

pub struct BurrowTarget {
    abi: ethabiencoder::EthAbiEncoder,
}

impl BurrowTarget {
    pub fn build<'a>(
        context: &'a Context,
        contract: &'a resolver::Contract,
        filename: &'a str,
    ) -> Contract<'a> {
        let mut c = Contract::new(context, contract, filename, None);
        let b = BurrowTarget {
            abi: ethabiencoder::EthAbiEncoder {},
        };

        // externals
        b.declare_externals(&mut c);

        c.emit_functions(&b);

        b.emit_constructor_dispatch(&c);
        b.emit_function_dispatch(&c);

        c
    }

    fn declare_externals(&self, contract: &mut Contract) {
        let ret = contract.context.void_type();
        let args: Vec<BasicTypeEnum> = vec![
            contract.context.i32_type().into(),
            contract
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .into(),
            contract.context.i32_type().into(),
        ];

        let ftype = ret.fn_type(&args, false);

        contract
            .module
            .add_function("get_storage32", ftype, Some(Linkage::External));
        contract
            .module
            .add_function("set_storage32", ftype, Some(Linkage::External));
    }

    fn emit_constructor_dispatch(&self, contract: &Contract) {
        let initializer = contract.emit_initializer(self);

        // create start function
        let ret = contract.context.void_type();
        let ftype = ret.fn_type(
            &[contract
                .context
                .i32_type()
                .ptr_type(AddressSpace::Generic)
                .into()],
            false,
        );
        let function = contract.module.add_function("constructor", ftype, None);

        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(&entry);

        // init our heap
        contract.builder.build_call(
            contract.module.get_function("__init_heap").unwrap(),
            &[],
            "",
        );

        // init our storage vars
        contract.builder.build_call(initializer, &[], "");

        if let Some(con) = contract.ns.constructors.get(0) {
            let mut args = Vec::new();

            let arg = function.get_first_param().unwrap().into_pointer_value();
            let length = contract.builder.build_load(arg, "length");

            // step over length
            let args_ptr = unsafe {
                contract.builder.build_gep(
                    arg,
                    &[contract.context.i32_type().const_int(1, false)],
                    "args_ptr",
                )
            };

            // insert abi decode
            self.abi_decode(
                contract,
                function,
                &mut args,
                args_ptr,
                length.into_int_value(),
                con,
            );

            contract
                .builder
                .build_call(contract.constructors[0], &args, "");
        }

        contract.builder.build_return(None);
    }

    fn emit_function_dispatch(&self, contract: &Contract) {
        // create start function
        let ret = contract.context.i32_type().ptr_type(AddressSpace::Generic);
        let ftype = ret.fn_type(
            &[contract
                .context
                .i32_type()
                .ptr_type(AddressSpace::Generic)
                .into()],
            false,
        );
        let function = contract.module.add_function("function", ftype, None);

        let entry = contract.context.append_basic_block(function, "entry");
        let fallback_block = contract.context.append_basic_block(function, "fallback");

        contract.builder.position_at_end(&entry);

        let data = function.get_first_param().unwrap().into_pointer_value();
        let argslen = contract.builder.build_load(data, "length").into_int_value();

        let argsdata = unsafe {
            contract.builder.build_gep(
                data,
                &[contract.context.i32_type().const_int(1, false)],
                "argsdata",
            )
        };

        contract.emit_function_dispatch(
            &contract.ns.functions,
            &contract.functions,
            argsdata,
            argslen,
            function,
            &fallback_block,
            self,
        );

        // emit fallback code
        contract.builder.position_at_end(&fallback_block);

        match contract.ns.fallback_function() {
            Some(f) => {
                contract.builder.build_call(contract.functions[f], &[], "");

                contract.builder.build_return(None);
            }
            None => {
                contract.builder.build_unreachable();
            }
        }
    }
}

impl TargetRuntime for BurrowTarget {
    fn return_empty_abi(&self, contract: &Contract) {
        let dest = contract
            .builder
            .build_call(
                contract.module.get_function("__malloc").unwrap(),
                &[contract.context.i32_type().const_int(4, false).into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        contract.builder.build_store(
            contract.builder.build_pointer_cast(
                dest,
                contract.context.i32_type().ptr_type(AddressSpace::Generic),
                "",
            ),
            contract.context.i32_type().const_zero(),
        );

        contract.builder.build_return(Some(&dest));
    }

    fn return_abi<'b>(&self, contract: &'b Contract, data: PointerValue<'b>, _length: IntValue) {
        contract.builder.build_return(Some(&data));
    }

    fn assert_failure<'b>(&self, contract: &'b Contract) {
        contract.builder.build_unreachable();
    }

    fn set_storage<'a>(
        &self,
        contract: &'a Contract,
        _function: FunctionValue,
        slot: u32,
        dest: inkwell::values::PointerValue<'a>,
    ) {
        contract.builder.build_call(
            contract.module.get_function("set_storage32").unwrap(),
            &[
                contract
                    .context
                    .i32_type()
                    .const_int(slot as u64, false)
                    .into(),
                contract
                    .builder
                    .build_pointer_cast(
                        dest,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                dest.get_type()
                    .size_of()
                    .const_cast(contract.context.i32_type(), false)
                    .into(),
            ],
            "",
        );
    }

    fn get_storage<'a>(
        &self,
        contract: &'a Contract,
        _function: FunctionValue,
        slot: u32,
        dest: inkwell::values::PointerValue<'a>,
    ) {
        contract.builder.build_call(
            contract.module.get_function("get_storage32").unwrap(),
            &[
                contract
                    .context
                    .i32_type()
                    .const_int(slot as u64, false)
                    .into(),
                contract
                    .builder
                    .build_pointer_cast(
                        dest,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                dest.get_type()
                    .size_of()
                    .const_cast(contract.context.i32_type(), false)
                    .into(),
            ],
            "",
        );
    }

    fn abi_encode<'b>(
        &self,
        contract: &'b Contract,
        function: FunctionValue,
        args: &[BasicValueEnum<'b>],
        spec: &resolver::FunctionDecl,
    ) -> (PointerValue<'b>, IntValue<'b>) {
        let length = spec
            .returns
            .iter()
            .fold(0, |acc, arg| acc + self.abi.encoded_length(&arg.ty));
        let encoded_data = contract
            .builder
            .build_call(
                contract.module.get_function("__malloc").unwrap(),
                &[contract
                    .context
                    .i32_type()
                    .const_int(length + 4, false)
                    .into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        // write length
        contract.builder.build_store(
            contract.builder.build_pointer_cast(
                encoded_data,
                contract.context.i32_type().ptr_type(AddressSpace::Generic),
                "",
            ),
            contract.context.i32_type().const_int(length, false),
        );

        // malloc returns u8*
        let mut data = unsafe {
            contract.builder.build_gep(
                encoded_data,
                &[contract.context.i32_type().const_int(4, false)],
                "encoded_data",
            )
        };

        for (i, arg) in spec.returns.iter().enumerate() {
            self.abi
                .encode_ty(contract, function, &arg.ty, args[i], &mut data);
        }

        (
            encoded_data,
            contract.context.i32_type().const_int(length + 4, false),
        )
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
