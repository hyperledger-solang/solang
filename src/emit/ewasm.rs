use link::link;
use resolver;
use std::str;

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::types::{BasicTypeEnum, IntType};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::OptimizationLevel;

use super::ethabiencoder;
use super::{Contract, TargetRuntime};
use crate::Target;

pub struct EwasmTarget {
    abi: ethabiencoder::EthAbiEncoder,
}

impl EwasmTarget {
    pub fn build<'a>(
        context: &'a Context,
        contract: &'a resolver::Contract,
        ns: &'a resolver::Namespace,
        filename: &'a str,
        opt: OptimizationLevel,
    ) -> Contract<'a> {
        // first emit runtime code
        let mut runtime_code = Contract::new(context, contract, ns, filename, None);
        let b = EwasmTarget {
            abi: ethabiencoder::EthAbiEncoder {},
        };

        // externals
        b.declare_externals(&mut runtime_code);

        // FIXME: this also emits the constructors. We can either rely on lto linking
        // to optimize them away or do not emit them.
        runtime_code.emit_functions(&b);

        b.emit_function_dispatch(&runtime_code);

        runtime_code.internalize(&["main"]);

        let runtime_obj = runtime_code.wasm(opt).unwrap();
        let runtime_bs = link(&runtime_obj, Target::Ewasm);

        // Now we have the runtime code, create the deployer
        let mut deploy_code = Contract::new(
            context,
            contract,
            ns,
            filename,
            Some(Box::new(runtime_code)),
        );
        let b = EwasmTarget {
            abi: ethabiencoder::EthAbiEncoder {},
        };

        // externals
        b.declare_externals(&mut deploy_code);

        // FIXME: this emits the constructors, as well as the functions. In Ethereum Solidity,
        // no functions can be called from the constructor. We should either disallow this too
        // and not emit functions, or use lto linking to optimize any unused functions away.
        deploy_code.emit_functions(&b);

        b.emit_constructor_dispatch(&mut deploy_code, &runtime_bs);

        deploy_code.internalize(&[
            "main",
            "getCallDataSize",
            "callDataCopy",
            "storageStore",
            "storageLoad",
            "finish",
            "revert",
            "printMem",
        ]);

        deploy_code
    }

    fn main_prelude<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(entry);

        // init our heap
        contract.builder.build_call(
            contract.module.get_function("__init_heap").unwrap(),
            &[],
            "",
        );

        // copy arguments from scratch buffer
        let args_length = contract
            .builder
            .build_call(
                contract.module.get_function("getCallDataSize").unwrap(),
                &[],
                "calldatasize",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let args = contract
            .builder
            .build_call(
                contract.module.get_function("__malloc").unwrap(),
                &[args_length],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        contract.builder.build_call(
            contract.module.get_function("callDataCopy").unwrap(),
            &[
                args.into(),
                contract.context.i32_type().const_zero().into(),
                args_length,
            ],
            "",
        );

        let args = contract.builder.build_pointer_cast(
            args,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "",
        );

        (args, args_length.into_int_value())
    }

    fn declare_externals(&self, contract: &mut Contract) {
        let ret = contract.context.void_type();
        let args: Vec<BasicTypeEnum> = vec![
            contract
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .into(),
            contract
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .into(),
        ];

        let ftype = ret.fn_type(&args, false);

        contract
            .module
            .add_function("storageStore", ftype, Some(Linkage::External));
        contract
            .module
            .add_function("storageLoad", ftype, Some(Linkage::External));

        contract.module.add_function(
            "getCallDataSize",
            contract.context.i32_type().fn_type(&[], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "callDataCopy",
            contract.context.void_type().fn_type(
                &[
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // resultOffset
                    contract.context.i32_type().into(), // dataOffset
                    contract.context.i32_type().into(), // length
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "printMem",
            contract.context.void_type().fn_type(
                &[
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // string_ptr
                    contract.context.i32_type().into(), // string_length
                ],
                false,
            ),
            Some(Linkage::External),
        );

        let noreturn = contract
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("noreturn"), 0);

        // mark as noreturn
        contract
            .module
            .add_function(
                "finish",
                contract.context.void_type().fn_type(
                    &[
                        contract
                            .context
                            .i8_type()
                            .ptr_type(AddressSpace::Generic)
                            .into(), // data_ptr
                        contract.context.i32_type().into(), // data_len
                    ],
                    false,
                ),
                Some(Linkage::External),
            )
            .add_attribute(AttributeLoc::Function, noreturn);

        // mark as noreturn
        contract
            .module
            .add_function(
                "revert",
                contract.context.void_type().fn_type(
                    &[
                        contract
                            .context
                            .i8_type()
                            .ptr_type(AddressSpace::Generic)
                            .into(), // data_ptr
                        contract.context.i32_type().into(), // data_len
                    ],
                    false,
                ),
                Some(Linkage::External),
            )
            .add_attribute(AttributeLoc::Function, noreturn);
    }

    fn emit_constructor_dispatch(&self, contract: &mut Contract, runtime: &[u8]) {
        let initializer = contract.emit_initializer(self);

        // create start function
        let ret = contract.context.void_type();
        let ftype = ret.fn_type(&[], false);
        let function = contract.module.add_function("main", ftype, None);

        // FIXME: If there is no constructor, do not copy the calldata (but check calldatasize == 0)
        let (argsdata, length) = self.main_prelude(contract, function);

        // init our storage vars
        contract.builder.build_call(initializer, &[], "");

        if let Some(con) = contract.contract.constructors.get(0) {
            let mut args = Vec::new();

            // insert abi decode
            self.abi
                .decode(contract, function, &mut args, argsdata, length, con);

            contract
                .builder
                .build_call(contract.constructors[0], &args, "");
        }

        // the deploy code should return the runtime wasm code
        let runtime_code = contract.emit_global_string("runtime_code", runtime, true);

        contract.builder.build_call(
            contract.module.get_function("finish").unwrap(),
            &[
                runtime_code.into(),
                contract
                    .context
                    .i32_type()
                    .const_int(runtime.len() as u64, false)
                    .into(),
            ],
            "",
        );

        // since finish is marked noreturn, this should be optimized away
        // however it is needed to create valid LLVM IR
        contract.builder.build_unreachable();
    }

    fn emit_function_dispatch(&self, contract: &Contract) {
        // create start function
        let ret = contract.context.void_type();
        let ftype = ret.fn_type(&[], false);
        let function = contract.module.add_function("main", ftype, None);

        let (argsdata, argslen) = self.main_prelude(contract, function);

        let fallback_block = contract.context.append_basic_block(function, "fallback");

        contract.emit_function_dispatch(
            &contract.contract.functions,
            &contract.functions,
            argsdata,
            argslen,
            function,
            fallback_block,
            self,
        );

        // emit fallback code
        contract.builder.position_at_end(fallback_block);

        match contract.contract.fallback_function() {
            Some(f) => {
                contract.builder.build_call(contract.functions[f], &[], "");

                contract
                    .builder
                    .build_return(Some(&contract.context.i32_type().const_zero()));
            }
            None => {
                contract.builder.build_unreachable();
            }
        }
    }
}

impl TargetRuntime for EwasmTarget {
    fn clear_storage<'a>(
        &self,
        contract: &'a Contract,
        _function: FunctionValue,
        slot: PointerValue<'a>,
    ) {
        let value = contract
            .builder
            .build_alloca(contract.context.custom_width_int_type(256), "value");

        let value8 = contract.builder.build_pointer_cast(
            value,
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "value8",
        );

        contract.builder.build_call(
            contract.module.get_function("__bzero8").unwrap(),
            &[
                value8.into(),
                contract.context.i32_type().const_int(4, false).into(),
            ],
            "",
        );

        contract.builder.build_call(
            contract.module.get_function("storageStore").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                value8.into(),
            ],
            "",
        );
    }

    fn set_storage_string<'a>(
        &self,
        _contract: &'a Contract,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
        _dest: PointerValue<'a>,
    ) {
        unimplemented!();
    }

    fn get_storage_string<'a>(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue,
    ) -> PointerValue<'a> {
        unimplemented!();
    }
    fn get_storage_bytes_subscript<'a>(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
        _index: IntValue<'a>,
    ) -> IntValue<'a> {
        unimplemented!();
    }
    fn set_storage_bytes_subscript<'a>(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
        _index: IntValue<'a>,
        _val: IntValue<'a>,
    ) {
        unimplemented!();
    }
    fn storage_bytes_push<'a>(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
        _val: IntValue<'a>,
    ) {
        unimplemented!();
    }
    fn storage_bytes_pop<'a>(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> IntValue<'a> {
        unimplemented!();
    }
    fn storage_string_length<'a>(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> IntValue<'a> {
        unimplemented!();
    }

    fn set_storage<'a>(
        &self,
        contract: &'a Contract,
        _function: FunctionValue,
        slot: PointerValue<'a>,
        dest: PointerValue<'a>,
    ) {
        if dest
            .get_type()
            .get_element_type()
            .into_int_type()
            .get_bit_width()
            == 256
        {
            contract.builder.build_call(
                contract.module.get_function("storageStore").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            slot,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    contract
                        .builder
                        .build_pointer_cast(
                            dest,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                ],
                "",
            );
        } else {
            let value = contract
                .builder
                .build_alloca(contract.context.custom_width_int_type(256), "value");

            let value8 = contract.builder.build_pointer_cast(
                value,
                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                "value8",
            );

            contract.builder.build_call(
                contract.module.get_function("__bzero8").unwrap(),
                &[
                    value8.into(),
                    contract.context.i32_type().const_int(4, false).into(),
                ],
                "",
            );

            let val = contract.builder.build_load(dest, "value");

            contract.builder.build_store(
                contract
                    .builder
                    .build_pointer_cast(value, dest.get_type(), ""),
                val,
            );

            contract.builder.build_call(
                contract.module.get_function("storageStore").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            slot,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    value8.into(),
                ],
                "",
            );
        }
    }

    fn get_storage_int<'a>(
        &self,
        contract: &Contract<'a>,
        _function: FunctionValue,
        slot: PointerValue,
        ty: IntType<'a>,
    ) -> IntValue<'a> {
        let dest = contract.builder.build_array_alloca(
            contract.context.i8_type(),
            contract.context.i32_type().const_int(32, false),
            "buf",
        );

        contract.builder.build_call(
            contract.module.get_function("storageLoad").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        slot,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                contract
                    .builder
                    .build_pointer_cast(
                        dest,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
            ],
            "",
        );

        contract
            .builder
            .build_load(
                contract
                    .builder
                    .build_pointer_cast(dest, ty.ptr_type(AddressSpace::Generic), ""),
                "loaded_int",
            )
            .into_int_value()
    }

    /// ewasm has no keccak256 host function, so call our implementation
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
        contract.builder.build_call(
            contract.module.get_function("finish").unwrap(),
            &[
                contract
                    .context
                    .i8_type()
                    .ptr_type(AddressSpace::Generic)
                    .const_zero()
                    .into(),
                contract.context.i32_type().const_zero().into(),
            ],
            "",
        );

        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_zero()));
    }

    fn return_abi<'b>(&self, contract: &'b Contract, data: PointerValue<'b>, length: IntValue) {
        contract.builder.build_call(
            contract.module.get_function("finish").unwrap(),
            &[data.into(), length.into()],
            "",
        );

        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_zero()));
    }

    fn assert_failure<'b>(&self, contract: &'b Contract, data: PointerValue, len: IntValue) {
        contract.builder.build_call(
            contract.module.get_function("revert").unwrap(),
            &[data.into(), len.into()],
            "",
        );

        // since revert is marked noreturn, this should be optimized away
        // however it is needed to create valid LLVM IR
        contract.builder.build_unreachable();
    }

    fn abi_encode<'b>(
        &self,
        contract: &Contract<'b>,
        selector: Option<u32>,
        _load: bool,
        function: FunctionValue,
        args: &[BasicValueEnum<'b>],
        spec: &[resolver::Parameter],
    ) -> (PointerValue<'b>, IntValue<'b>) {
        let mut length = contract.context.i32_type().const_int(
            spec.iter()
                .map(|arg| self.abi.encoded_length(&arg.ty, contract.ns))
                .sum(),
            false,
        );

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
                contract.module.get_function("__malloc").unwrap(),
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
                contract
                    .context
                    .i32_type()
                    .const_int(selector.to_be() as u64, false),
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

        for (i, arg) in spec.iter().enumerate() {
            self.abi
                .encode_ty(contract, function, &arg.ty, args[i], &mut data);
        }

        (encoded_data, length)
    }

    fn abi_decode<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue,
        spec: &resolver::FunctionDecl,
    ) {
        self.abi
            .decode(contract, function, args, data, length, spec);
    }

    fn print(&self, contract: &Contract, string_ptr: PointerValue, string_len: IntValue) {
        contract.builder.build_call(
            contract.module.get_function("printMem").unwrap(),
            &[string_ptr.into(), string_len.into()],
            "",
        );
    }
}
