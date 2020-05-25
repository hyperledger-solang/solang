use parser::ast;
use resolver;
use std::cell::RefCell;
use std::str;

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::types::BasicType;
use inkwell::types::{BasicTypeEnum, IntType};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;

use super::ethabiencoder;
use super::{Contract, TargetRuntime};

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
        let mut runtime_code = Contract::new(context, contract, ns, filename, opt, None);
        let mut b = EwasmTarget {
            abi: ethabiencoder::EthAbiEncoder {},
        };

        // externals
        b.declare_externals(&mut runtime_code);

        // This also emits the constructors. We are relying on DCE to eliminate them from
        // the final code.
        runtime_code.emit_functions(&mut b);

        b.emit_function_dispatch(&runtime_code);

        runtime_code.internalize(&["main"]);

        let runtime_bs = runtime_code.wasm(true).unwrap();

        // Now we have the runtime code, create the deployer
        let mut deploy_code = Contract::new(
            context,
            contract,
            ns,
            filename,
            opt,
            Some(Box::new(runtime_code)),
        );
        let mut b = EwasmTarget {
            abi: ethabiencoder::EthAbiEncoder {},
        };

        // externals
        b.declare_externals(&mut deploy_code);

        // FIXME: this emits the constructors, as well as the functions. In Ethereum Solidity,
        // no functions can be called from the constructor. We should either disallow this too
        // and not emit functions, or use lto linking to optimize any unused functions away.
        deploy_code.emit_functions(&mut b);

        b.deployer_dispatch(&mut deploy_code, &runtime_bs);

        deploy_code.internalize(&[
            "main",
            "getCallDataSize",
            "callDataCopy",
            "storageStore",
            "storageLoad",
            "finish",
            "revert",
            "copyCopy",
            "getCodeSize",
            "printMem",
            "call",
            "create",
            "getReturnDataSize",
            "returnDataCopy",
            "getCallValue",
        ]);

        deploy_code
    }

    fn runtime_prelude<'a>(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(entry);

        // first thing to do is abort value transfers if we're not payable
        if contract.function_abort_value_transfers {
            contract.abort_if_value_transfer(self, function);
        }

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

    fn deployer_prelude<'a>(
        &self,
        contract: &mut Contract<'a>,
        function: FunctionValue,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(entry);

        // first thing to do is abort value transfers if constructors not payable
        if contract.constructor_abort_value_transfers {
            contract.abort_if_value_transfer(self, function);
        }

        // init our heap
        contract.builder.build_call(
            contract.module.get_function("__init_heap").unwrap(),
            &[],
            "",
        );

        // The code_size will need to be patched later
        let code_size = contract.context.i32_type().const_int(0x4000, false);

        // copy arguments from scratch buffer
        let args_length = contract.builder.build_int_sub(
            contract
                .builder
                .build_call(
                    contract.module.get_function("getCodeSize").unwrap(),
                    &[],
                    "codesize",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value(),
            code_size,
            "",
        );

        let args = contract
            .builder
            .build_call(
                contract.module.get_function("__malloc").unwrap(),
                &[args_length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        contract.builder.build_call(
            contract.module.get_function("codeCopy").unwrap(),
            &[args.into(), code_size.into(), args_length.into()],
            "",
        );

        let args = contract.builder.build_pointer_cast(
            args,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "",
        );

        contract.code_size = RefCell::new(Some(code_size));

        (args, args_length)
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
            "getCodeSize",
            contract.context.i32_type().fn_type(&[], false),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "getReturnDataSize",
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
            "codeCopy",
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
            "returnDataCopy",
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

        contract.module.add_function(
            "create",
            contract.context.i32_type().fn_type(
                &[
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // valueOffset
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // input offset
                    contract.context.i32_type().into(), // input length
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // address result
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "call",
            contract.context.i32_type().fn_type(
                &[
                    contract.context.i64_type().into(), // gas
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // address
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // valueOffset
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // input offset
                    contract.context.i32_type().into(), // input length
                ],
                false,
            ),
            Some(Linkage::External),
        );

        contract.module.add_function(
            "getCallValue",
            contract.context.void_type().fn_type(
                &[
                    contract
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::Generic)
                        .into(), // value_ptr
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

    fn deployer_dispatch(&mut self, contract: &mut Contract, runtime: &[u8]) {
        let initializer = contract.emit_initializer(self);

        // create start function
        let ret = contract.context.void_type();
        let ftype = ret.fn_type(&[], false);
        let function = contract.module.add_function("main", ftype, None);

        // FIXME: If there is no constructor, do not copy the calldata (but check calldatasize == 0)
        let (argsdata, length) = self.deployer_prelude(contract, function);

        // init our storage vars
        contract.builder.build_call(initializer, &[], "");

        // ewasm only allows one constructor, hence find()
        if let Some((i, con)) = contract
            .contract
            .functions
            .iter()
            .enumerate()
            .find(|f| f.1.is_constructor())
        {
            let mut args = Vec::new();

            // insert abi decode
            self.abi
                .decode(contract, function, &mut args, argsdata, length, &con.params);

            contract
                .builder
                .build_call(contract.functions[i], &args, "");
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

        let (argsdata, argslen) = self.runtime_prelude(contract, function);

        contract.emit_function_dispatch(
            &contract.contract.functions,
            ast::FunctionTy::Function,
            &contract.functions,
            argsdata,
            argslen,
            function,
            None,
            self,
            |func| !contract.function_abort_value_transfers && !func.is_payable(),
        );
    }

    fn encode<'b>(
        &self,
        contract: &Contract<'b>,
        selector: Option<u32>,
        constant: Option<(PointerValue<'b>, u64)>,
        load: bool,
        function: FunctionValue,
        args: &[BasicValueEnum<'b>],
        spec: &[resolver::Parameter],
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

        if let Some((_, len)) = constant {
            length = contract.builder.build_int_add(
                length,
                contract.context.i32_type().const_int(len, false),
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

        if let Some((code, code_len)) = constant {
            contract.builder.build_call(
                contract.module.get_function("__memcpy").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            data,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    code.into(),
                    contract
                        .context
                        .i32_type()
                        .const_int(code_len, false)
                        .into(),
                ],
                "",
            );

            data = unsafe {
                contract.builder.build_gep(
                    data,
                    &[contract.context.i32_type().const_int(code_len, false)],
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
                data.into(),
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

        // since finish is marked noreturn, this should be optimized away
        // however it is needed to create valid LLVM IR
        contract.builder.build_unreachable();
    }

    fn return_abi<'b>(&self, contract: &'b Contract, data: PointerValue<'b>, length: IntValue) {
        contract.builder.build_call(
            contract.module.get_function("finish").unwrap(),
            &[data.into(), length.into()],
            "",
        );

        // since finish is marked noreturn, this should be optimized away
        // however it is needed to create valid LLVM IR
        contract.builder.build_unreachable();
    }

    // ewasm main cannot return any value
    fn return_u32<'b>(&self, contract: &'b Contract, _ret: IntValue<'b>) {
        self.assert_failure(
            contract,
            contract
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            contract.context.i32_type().const_zero(),
        );
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
        load: bool,
        function: FunctionValue,
        args: &[BasicValueEnum<'b>],
        spec: &[resolver::Parameter],
    ) -> (PointerValue<'b>, IntValue<'b>) {
        self.encode(contract, selector, None, load, function, args, spec)
    }

    fn abi_decode<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue<'b>,
        spec: &[resolver::Parameter],
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

    fn create_contract<'b>(
        &mut self,
        contract: &Contract<'b>,
        function: FunctionValue,
        success: Option<&mut BasicValueEnum<'b>>,
        contract_no: usize,
        constructor_no: usize,
        address: PointerValue<'b>,
        args: &[BasicValueEnum<'b>],
        _gas: IntValue<'b>,
        value: Option<IntValue<'b>>,
        _salt: Option<IntValue<'b>>,
    ) {
        let resolver_contract = &contract.ns.contracts[contract_no];

        let target_contract = Contract::build(
            contract.context,
            &resolver_contract,
            contract.ns,
            "",
            contract.opt,
        );

        // wasm
        let wasm = target_contract.wasm(true).expect("compile should succeeed");

        let code = contract.emit_global_string(
            &format!("contract_{}_code", resolver_contract.name),
            &wasm,
            true,
        );

        assert_eq!(constructor_no, 0);

        let params = if let Some(f) = resolver_contract
            .functions
            .iter()
            .find(|f| f.is_constructor())
        {
            f.params.as_slice()
        } else {
            &[]
        };

        // input
        let (input, input_len) = self.encode(
            contract,
            None,
            Some((code, wasm.len() as u64)),
            false,
            function,
            args,
            params,
        );

        // value is a u128
        let value_ptr = contract
            .builder
            .build_alloca(contract.value_type(), "balance");
        contract.builder.build_store(
            value_ptr,
            match value {
                Some(v) => v,
                None => contract.value_type().const_zero(),
            },
        );

        // call create
        let ret = contract
            .builder
            .build_call(
                contract.module.get_function("create").unwrap(),
                &[
                    contract
                        .builder
                        .build_pointer_cast(
                            value_ptr,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "value_transfer",
                        )
                        .into(),
                    input.into(),
                    input_len.into(),
                    address.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let is_success = contract.builder.build_int_compare(
            IntPredicate::EQ,
            ret,
            contract.context.i32_type().const_zero(),
            "success",
        );

        if let Some(success) = success {
            *success = is_success.into();
        } else {
            let success_block = contract.context.append_basic_block(function, "success");
            let bail_block = contract.context.append_basic_block(function, "bail");
            contract
                .builder
                .build_conditional_branch(is_success, success_block, bail_block);

            contract.builder.position_at_end(bail_block);

            self.assert_failure(
                contract,
                contract
                    .context
                    .i8_type()
                    .ptr_type(AddressSpace::Generic)
                    .const_null(),
                contract.context.i32_type().const_zero(),
            );

            contract.builder.position_at_end(success_block);
        }
    }

    fn external_call<'b>(
        &self,
        contract: &Contract<'b>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        address: PointerValue<'b>,
        gas: IntValue<'b>,
        value: IntValue<'b>,
    ) -> IntValue<'b> {
        // value is a u128
        let value_ptr = contract
            .builder
            .build_alloca(contract.value_type(), "balance");
        contract.builder.build_store(value_ptr, value);

        // call create
        contract
            .builder
            .build_call(
                contract.module.get_function("call").unwrap(),
                &[
                    gas.into(),
                    address.into(),
                    contract
                        .builder
                        .build_pointer_cast(
                            value_ptr,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "value_transfer",
                        )
                        .into(),
                    payload.into(),
                    payload_len.into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value()
    }

    fn return_data<'b>(&self, contract: &Contract<'b>) -> PointerValue<'b> {
        let length = contract
            .builder
            .build_call(
                contract.module.get_function("getReturnDataSize").unwrap(),
                &[],
                "returndatasize",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let malloc_length = contract.builder.build_int_add(
            length,
            contract
                .module
                .get_type("struct.vector")
                .unwrap()
                .size_of()
                .unwrap()
                .const_cast(contract.context.i32_type(), false),
            "size",
        );

        let p = contract
            .builder
            .build_call(
                contract.module.get_function("__malloc").unwrap(),
                &[malloc_length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let v = contract.builder.build_pointer_cast(
            p,
            contract
                .module
                .get_type("struct.vector")
                .unwrap()
                .ptr_type(AddressSpace::Generic),
            "string",
        );

        let data_len = unsafe {
            contract.builder.build_gep(
                v,
                &[
                    contract.context.i32_type().const_zero(),
                    contract.context.i32_type().const_zero(),
                ],
                "data_len",
            )
        };

        contract.builder.build_store(data_len, length);

        let data_size = unsafe {
            contract.builder.build_gep(
                v,
                &[
                    contract.context.i32_type().const_zero(),
                    contract.context.i32_type().const_int(1, false),
                ],
                "data_size",
            )
        };

        contract.builder.build_store(data_size, length);

        let data = unsafe {
            contract.builder.build_gep(
                v,
                &[
                    contract.context.i32_type().const_zero(),
                    contract.context.i32_type().const_int(2, false),
                ],
                "data",
            )
        };

        contract.builder.build_call(
            contract.module.get_function("returnDataCopy").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        data,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                contract.context.i32_type().const_zero().into(),
                length.into(),
            ],
            "",
        );

        v
    }

    /// ewasm value is always 128 bits
    fn value_transferred<'b>(&self, contract: &Contract<'b>) -> IntValue<'b> {
        let value = contract
            .builder
            .build_alloca(contract.value_type(), "value_transferred");

        contract.builder.build_call(
            contract.module.get_function("getCallValue").unwrap(),
            &[contract
                .builder
                .build_pointer_cast(
                    value,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "",
                )
                .into()],
            "value_transferred",
        );

        contract
            .builder
            .build_load(value, "value_transferred")
            .into_int_value()
    }
}
