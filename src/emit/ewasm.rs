use crate::codegen::cfg::HashTy;
use crate::parser::pt;
use crate::sema::ast;
use std::cell::RefCell;
use std::collections::HashMap;
use std::str;

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::types::IntType;
use inkwell::values::{
    BasicMetadataValueEnum, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;

use super::ethabiencoder;
use super::{Binary, TargetRuntime, Variable};
use crate::emit::Generate;

pub struct EwasmTarget {
    abi: ethabiencoder::EthAbiDecoder,
}

impl EwasmTarget {
    pub fn build<'a>(
        context: &'a Context,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        filename: &'a str,
        opt: OptimizationLevel,
        math_overflow_check: bool,
    ) -> Binary<'a> {
        // first emit runtime code
        let mut b = EwasmTarget {
            abi: ethabiencoder::EthAbiDecoder { bswap: false },
        };
        let mut runtime_code = Binary::new(
            context,
            ns.target,
            &contract.name,
            filename,
            opt,
            math_overflow_check,
            None,
        );

        runtime_code.set_early_value_aborts(contract, ns);

        // externals
        b.declare_externals(&mut runtime_code);

        // This also emits the constructors. We are relying on DCE to eliminate them from
        // the final code.
        b.emit_functions(&mut runtime_code, contract, ns);

        b.function_dispatch(&runtime_code, contract, ns);

        runtime_code.internalize(&["main"]);

        let runtime_bs = runtime_code.code(Generate::Linked).unwrap();

        // Now we have the runtime code, create the deployer
        let mut b = EwasmTarget {
            abi: ethabiencoder::EthAbiDecoder { bswap: false },
        };
        let mut deploy_code = Binary::new(
            context,
            ns.target,
            &contract.name,
            filename,
            opt,
            math_overflow_check,
            Some(Box::new(runtime_code)),
        );

        deploy_code.set_early_value_aborts(contract, ns);

        // externals
        b.declare_externals(&mut deploy_code);

        // FIXME: this emits the constructors, as well as the functions. In Ethereum Solidity,
        // no functions can be called from the constructor. We should either disallow this too
        // and not emit functions, or use lto linking to optimize any unused functions away.
        b.emit_functions(&mut deploy_code, contract, ns);

        b.deployer_dispatch(&mut deploy_code, contract, &runtime_bs, ns);

        deploy_code.internalize(&[
            "main",
            "getCallDataSize",
            "callDataCopy",
            "storageStore",
            "storageLoad",
            "finish",
            "revert",
            "codeCopy",
            "getCodeSize",
            "printMem",
            "call",
            "callStatic",
            "callDelegate",
            "create",
            "getReturnDataSize",
            "returnDataCopy",
            "getCallValue",
            "getAddress",
            "getExternalBalance",
            "getBlockHash",
            "getBlockDifficulty",
            "getGasLeft",
            "getBlockGasLimit",
            "getBlockTimestamp",
            "getBlockNumber",
            "getTxGasPrice",
            "getTxOrigin",
            "getBlockCoinbase",
            "getCaller",
            "log",
        ]);

        deploy_code
    }

    fn runtime_prelude<'a>(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue,
        ns: &ast::Namespace,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        let entry = binary.context.append_basic_block(function, "entry");

        binary.builder.position_at_end(entry);

        // first thing to do is abort value transfers if we're not payable
        if binary.function_abort_value_transfers {
            self.abort_if_value_transfer(binary, function, ns);
        }

        // init our heap
        binary
            .builder
            .build_call(binary.module.get_function("__init_heap").unwrap(), &[], "");

        // copy arguments from scratch buffer
        let args_length = binary
            .builder
            .build_call(
                binary.module.get_function("getCallDataSize").unwrap(),
                &[],
                "calldatasize",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        binary.builder.build_store(
            binary.calldata_len.as_pointer_value(),
            args_length.into_int_value(),
        );

        let args = binary
            .builder
            .build_call(
                binary.module.get_function("__malloc").unwrap(),
                &[args_length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        binary
            .builder
            .build_store(binary.calldata_data.as_pointer_value(), args);

        binary.builder.build_call(
            binary.module.get_function("callDataCopy").unwrap(),
            &[
                args.into(),
                binary.context.i32_type().const_zero().into(),
                args_length.into(),
            ],
            "",
        );

        let args = binary.builder.build_pointer_cast(
            args,
            binary.context.i32_type().ptr_type(AddressSpace::Generic),
            "",
        );

        (args, args_length.into_int_value())
    }

    fn deployer_prelude<'a>(
        &self,
        binary: &mut Binary<'a>,
        function: FunctionValue,
        ns: &ast::Namespace,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        let entry = binary.context.append_basic_block(function, "entry");

        binary.builder.position_at_end(entry);

        // first thing to do is abort value transfers if constructors not payable
        if binary.constructor_abort_value_transfers {
            self.abort_if_value_transfer(binary, function, ns);
        }

        // init our heap
        binary
            .builder
            .build_call(binary.module.get_function("__init_heap").unwrap(), &[], "");

        // The code_size will need to be patched later
        let code_size = binary.context.i32_type().const_int(0x4000, false);

        // copy arguments from scratch buffer
        let args_length = binary.builder.build_int_sub(
            binary
                .builder
                .build_call(
                    binary.module.get_function("getCodeSize").unwrap(),
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

        binary
            .builder
            .build_store(binary.calldata_len.as_pointer_value(), args_length);

        let args = binary
            .builder
            .build_call(
                binary.module.get_function("__malloc").unwrap(),
                &[args_length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        binary
            .builder
            .build_store(binary.calldata_data.as_pointer_value(), args);

        binary.builder.build_call(
            binary.module.get_function("codeCopy").unwrap(),
            &[args.into(), code_size.into(), args_length.into()],
            "",
        );

        let args = binary.builder.build_pointer_cast(
            args,
            binary.context.i32_type().ptr_type(AddressSpace::Generic),
            "",
        );

        binary.code_size = RefCell::new(Some(code_size));

        (args, args_length)
    }

    fn declare_externals(&self, binary: &mut Binary) {
        let u8_ptr_ty = binary.context.i8_type().ptr_type(AddressSpace::Generic);
        let u32_ty = binary.context.i32_type();
        let u64_ty = binary.context.i64_type();
        let void_ty = binary.context.void_type();

        let ftype = void_ty.fn_type(&[u8_ptr_ty.into(), u8_ptr_ty.into()], false);

        binary
            .module
            .add_function("storageStore", ftype, Some(Linkage::External));
        binary
            .module
            .add_function("storageLoad", ftype, Some(Linkage::External));

        binary.module.add_function(
            "getCallDataSize",
            u32_ty.fn_type(&[], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "getCodeSize",
            u32_ty.fn_type(&[], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "getReturnDataSize",
            u32_ty.fn_type(&[], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "callDataCopy",
            void_ty.fn_type(
                &[
                    u8_ptr_ty.into(), // resultOffset
                    u32_ty.into(),    // dataOffset
                    u32_ty.into(),    // length
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "codeCopy",
            void_ty.fn_type(
                &[
                    u8_ptr_ty.into(), // resultOffset
                    u32_ty.into(),    // dataOffset
                    u32_ty.into(),    // length
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "returnDataCopy",
            void_ty.fn_type(
                &[
                    u8_ptr_ty.into(), // resultOffset
                    u32_ty.into(),    // dataOffset
                    u32_ty.into(),    // length
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "printMem",
            void_ty.fn_type(
                &[
                    u8_ptr_ty.into(), // string_ptr
                    u32_ty.into(),    // string_length
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "create",
            u32_ty.fn_type(
                &[
                    u8_ptr_ty.into(), // valueOffset
                    u8_ptr_ty.into(), // input offset
                    u32_ty.into(),    // input length
                    u8_ptr_ty.into(), // address result
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "call",
            u32_ty.fn_type(
                &[
                    u64_ty.into(),    // gas
                    u8_ptr_ty.into(), // address
                    u8_ptr_ty.into(), // valueOffset
                    u8_ptr_ty.into(), // input offset
                    u32_ty.into(),    // input length
                ],
                false,
            ),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "callStatic",
            u32_ty.fn_type(
                &[
                    u64_ty.into(),    // gas
                    u8_ptr_ty.into(), // address
                    u8_ptr_ty.into(), // input offset
                    u32_ty.into(),    // input length
                ],
                false,
            ),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "callDelegate",
            u32_ty.fn_type(
                &[
                    u64_ty.into(),    // gas
                    u8_ptr_ty.into(), // address
                    u8_ptr_ty.into(), // input offset
                    u32_ty.into(),    // input length
                ],
                false,
            ),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "getCallValue",
            void_ty.fn_type(
                &[
                    u8_ptr_ty.into(), // value_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "getAddress",
            void_ty.fn_type(
                &[
                    u8_ptr_ty.into(), // value_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "getCaller",
            void_ty.fn_type(
                &[
                    u8_ptr_ty.into(), // value_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "getExternalBalance",
            void_ty.fn_type(
                &[
                    u8_ptr_ty.into(), // address_ptr
                    u8_ptr_ty.into(), // balance_ptr
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "getBlockHash",
            u32_ty.fn_type(
                &[
                    u64_ty.into(),    // block number
                    u8_ptr_ty.into(), // hash_ptr result
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "getBlockCoinbase",
            void_ty.fn_type(
                &[
                    u8_ptr_ty.into(), // address_ptr result
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "getBlockDifficulty",
            void_ty.fn_type(
                &[
                    u8_ptr_ty.into(), // u256_ptr result
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "getGasLeft",
            u64_ty.fn_type(&[], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "getBlockGasLimit",
            u64_ty.fn_type(&[], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "getBlockTimestamp",
            u64_ty.fn_type(&[], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "getBlockNumber",
            u64_ty.fn_type(&[], false),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "getTxGasPrice",
            void_ty.fn_type(
                &[
                    u8_ptr_ty.into(), // value_ptr result
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "getTxOrigin",
            void_ty.fn_type(
                &[
                    u8_ptr_ty.into(), // address_ptr result
                ],
                false,
            ),
            Some(Linkage::External),
        );

        binary.module.add_function(
            "log",
            void_ty.fn_type(
                &[
                    u8_ptr_ty.into(), // data_ptr result
                    u32_ty.into(),    // data length
                    u32_ty.into(),    // number of topics
                    u8_ptr_ty.into(), // topic1
                    u8_ptr_ty.into(), // topic2
                    u8_ptr_ty.into(), // topic3
                    u8_ptr_ty.into(), // topic4
                ],
                false,
            ),
            Some(Linkage::External),
        );

        let noreturn = binary
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("noreturn"), 0);

        // mark as noreturn
        binary
            .module
            .add_function(
                "finish",
                void_ty.fn_type(
                    &[
                        u8_ptr_ty.into(), // data_ptr
                        u32_ty.into(),    // data_len
                    ],
                    false,
                ),
                Some(Linkage::External),
            )
            .add_attribute(AttributeLoc::Function, noreturn);

        // mark as noreturn
        binary
            .module
            .add_function(
                "revert",
                void_ty.fn_type(
                    &[
                        u8_ptr_ty.into(), // data_ptr
                        u32_ty.into(),    // data_len
                    ],
                    false,
                ),
                Some(Linkage::External),
            )
            .add_attribute(AttributeLoc::Function, noreturn);

        // mark as noreturn
        binary
            .module
            .add_function(
                "selfDestruct",
                void_ty.fn_type(
                    &[
                        u8_ptr_ty.into(), // address_ptr
                    ],
                    false,
                ),
                Some(Linkage::External),
            )
            .add_attribute(AttributeLoc::Function, noreturn);
    }

    fn deployer_dispatch(
        &mut self,
        binary: &mut Binary,
        contract: &ast::Contract,
        runtime: &[u8],
        ns: &ast::Namespace,
    ) {
        let initializer = self.emit_initializer(binary, contract, ns);

        // create start function
        let ret = binary.context.void_type();
        let ftype = ret.fn_type(&[], false);
        let function = binary.module.add_function("main", ftype, None);

        // FIXME: If there is no constructor, do not copy the calldata (but check calldatasize == 0)
        let (argsdata, length) = self.deployer_prelude(binary, function, ns);

        // init our storage vars
        binary.builder.build_call(initializer, &[], "");

        // ewasm only allows one constructor, hence find()
        if let Some((cfg_no, cfg)) = contract
            .cfg
            .iter()
            .enumerate()
            .find(|(_, cfg)| cfg.ty == pt::FunctionTy::Constructor)
        {
            let mut args = Vec::new();

            // insert abi decode
            self.abi.decode(
                binary,
                function,
                &mut args,
                argsdata,
                length,
                &cfg.params,
                ns,
            );

            let args: Vec<BasicMetadataValueEnum> = args.iter().map(|arg| (*arg).into()).collect();

            binary
                .builder
                .build_call(binary.functions[&cfg_no], &args, "");
        }

        // the deploy code should return the runtime wasm code
        let runtime_code = binary.emit_global_string("runtime_code", runtime, true);

        binary.builder.build_call(
            binary.module.get_function("finish").unwrap(),
            &[
                runtime_code.into(),
                binary
                    .context
                    .i32_type()
                    .const_int(runtime.len() as u64, false)
                    .into(),
            ],
            "",
        );

        // since finish is marked noreturn, this should be optimized away
        // however it is needed to create valid LLVM IR
        binary.builder.build_unreachable();
    }

    fn function_dispatch(
        &mut self,
        binary: &Binary,
        contract: &ast::Contract,
        ns: &ast::Namespace,
    ) {
        // create start function
        let ret = binary.context.void_type();
        let ftype = ret.fn_type(&[], false);
        let function = binary.module.add_function("main", ftype, None);

        let (argsdata, argslen) = self.runtime_prelude(binary, function, ns);

        self.emit_function_dispatch(
            binary,
            contract,
            ns,
            pt::FunctionTy::Function,
            argsdata,
            argslen,
            function,
            &binary.functions,
            None,
            |func| !binary.function_abort_value_transfers && func.nonpayable,
        );
    }

    fn encode<'b>(
        &self,
        binary: &Binary<'b>,
        constant: Option<(PointerValue<'b>, u64)>,
        load: bool,
        function: FunctionValue<'b>,
        packed: &[BasicValueEnum<'b>],
        args: &[BasicValueEnum<'b>],
        tys: &[ast::Type],
        ns: &ast::Namespace,
    ) -> (PointerValue<'b>, IntValue<'b>) {
        let encoder = ethabiencoder::EncoderBuilder::new(
            binary, function, load, packed, args, tys, false, ns,
        );

        let mut length = encoder.encoded_length();

        if let Some((_, len)) = constant {
            length = binary.builder.build_int_add(
                length,
                binary.context.i32_type().const_int(len, false),
                "",
            );
        }

        let encoded_data = binary
            .builder
            .build_call(
                binary.module.get_function("__malloc").unwrap(),
                &[length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let mut data = encoded_data;

        if let Some((code, code_len)) = constant {
            binary.builder.build_call(
                binary.module.get_function("__memcpy").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            data,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    code.into(),
                    binary.context.i32_type().const_int(code_len, false).into(),
                ],
                "",
            );

            data = unsafe {
                binary.builder.build_gep(
                    data,
                    &[binary.context.i32_type().const_int(code_len, false)],
                    "",
                )
            };
        }

        encoder.finish(binary, function, data, ns);

        (encoded_data, length)
    }
}

impl<'a> TargetRuntime<'a> for EwasmTarget {
    fn storage_delete_single_slot(
        &self,
        binary: &Binary,
        _function: FunctionValue,
        slot: PointerValue,
    ) {
        let value = binary
            .builder
            .build_alloca(binary.context.custom_width_int_type(256), "value");

        let value8 = binary.builder.build_pointer_cast(
            value,
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "value8",
        );

        binary.builder.build_call(
            binary.module.get_function("__bzero8").unwrap(),
            &[
                value8.into(),
                binary.context.i32_type().const_int(4, false).into(),
            ],
            "",
        );

        binary.builder.build_call(
            binary.module.get_function("storageStore").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        slot,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                value8.into(),
            ],
            "",
        );
    }

    fn set_storage_string(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue<'a>,
        _slot: PointerValue<'a>,
        _dest: BasicValueEnum<'a>,
    ) {
        unimplemented!();
    }

    fn get_storage_string(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue,
        _slot: PointerValue,
    ) -> PointerValue<'a> {
        unimplemented!();
    }
    fn set_storage_extfunc(
        &self,
        _binary: &Binary,
        _function: FunctionValue,
        _slot: PointerValue,
        _dest: PointerValue,
    ) {
        unimplemented!();
    }
    fn get_storage_extfunc(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
        _ns: &ast::Namespace,
    ) -> PointerValue<'a> {
        unimplemented!();
    }
    fn get_storage_bytes_subscript(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue,
        _slot: IntValue<'a>,
        _index: IntValue<'a>,
    ) -> IntValue<'a> {
        unimplemented!();
    }
    fn set_storage_bytes_subscript(
        &self,
        _binary: &Binary,
        _function: FunctionValue,
        _slot: IntValue,
        _index: IntValue,
        _val: IntValue,
    ) {
        unimplemented!();
    }
    fn storage_push(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue,
        _ty: &ast::Type,
        _slot: IntValue<'a>,
        _val: Option<BasicValueEnum<'a>>,
        _ns: &ast::Namespace,
    ) -> BasicValueEnum<'a> {
        unimplemented!();
    }
    fn storage_pop(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue<'a>,
        _ty: &ast::Type,
        _slot: IntValue<'a>,
        _load: bool,
        _ns: &ast::Namespace,
    ) -> Option<BasicValueEnum<'a>> {
        unimplemented!();
    }

    fn set_storage(
        &self,
        binary: &Binary,
        _function: FunctionValue,
        slot: PointerValue,
        dest: PointerValue,
    ) {
        if dest
            .get_type()
            .get_element_type()
            .into_int_type()
            .get_bit_width()
            == 256
        {
            binary.builder.build_call(
                binary.module.get_function("storageStore").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            slot,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    binary
                        .builder
                        .build_pointer_cast(
                            dest,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                ],
                "",
            );
        } else {
            let value = binary
                .builder
                .build_alloca(binary.context.custom_width_int_type(256), "value");

            let value8 = binary.builder.build_pointer_cast(
                value,
                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                "value8",
            );

            binary.builder.build_call(
                binary.module.get_function("__bzero8").unwrap(),
                &[
                    value8.into(),
                    binary.context.i32_type().const_int(4, false).into(),
                ],
                "",
            );

            let val = binary.builder.build_load(dest, "value");

            binary.builder.build_store(
                binary
                    .builder
                    .build_pointer_cast(value, dest.get_type(), ""),
                val,
            );

            binary.builder.build_call(
                binary.module.get_function("storageStore").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            slot,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into(),
                    value8.into(),
                ],
                "",
            );
        }
    }

    fn get_storage_int(
        &self,
        binary: &Binary<'a>,
        _function: FunctionValue,
        slot: PointerValue<'a>,
        ty: IntType<'a>,
    ) -> IntValue<'a> {
        let dest = binary.builder.build_array_alloca(
            binary.context.i8_type(),
            binary.context.i32_type().const_int(32, false),
            "buf",
        );

        binary.builder.build_call(
            binary.module.get_function("storageLoad").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        slot,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary
                    .builder
                    .build_pointer_cast(
                        dest,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
            ],
            "",
        );

        binary
            .builder
            .build_load(
                binary
                    .builder
                    .build_pointer_cast(dest, ty.ptr_type(AddressSpace::Generic), ""),
                "loaded_int",
            )
            .into_int_value()
    }

    /// ewasm has no keccak256 host function, so call our implementation
    fn keccak256_hash(
        &self,
        binary: &Binary,
        src: PointerValue,
        length: IntValue,
        dest: PointerValue,
        ns: &ast::Namespace,
    ) {
        let balance = binary
            .builder
            .build_alloca(binary.value_type(ns), "balance");

        binary
            .builder
            .build_store(balance, binary.value_type(ns).const_zero());

        let keccak256_pre_compile_address: [u8; 20] =
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 20];

        let address =
            binary.emit_global_string("keccak256_precompile", &keccak256_pre_compile_address, true);

        binary.builder.build_call(
            binary.module.get_function("call").unwrap(),
            &[
                binary
                    .context
                    .i64_type()
                    .const_int(i64::MAX as u64, false)
                    .into(),
                binary
                    .builder
                    .build_pointer_cast(
                        address,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "address",
                    )
                    .into(),
                binary
                    .builder
                    .build_pointer_cast(
                        balance,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "balance",
                    )
                    .into(),
                binary
                    .builder
                    .build_pointer_cast(
                        src,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "source",
                    )
                    .into(),
                length.into(),
            ],
            "",
        );

        // We're not checking return value or returnDataSize;
        // assuming precompiles always succeed
        binary.builder.build_call(
            binary.module.get_function("returnDataCopy").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        dest,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "dest",
                    )
                    .into(),
                binary.context.i32_type().const_zero().into(),
                binary.context.i32_type().const_int(32, false).into(),
            ],
            "",
        );
    }

    fn return_empty_abi(&self, binary: &Binary) {
        binary.builder.build_call(
            binary.module.get_function("finish").unwrap(),
            &[
                binary
                    .context
                    .i8_type()
                    .ptr_type(AddressSpace::Generic)
                    .const_zero()
                    .into(),
                binary.context.i32_type().const_zero().into(),
            ],
            "",
        );

        // since finish is marked noreturn, this should be optimized away
        // however it is needed to create valid LLVM IR
        binary.builder.build_unreachable();
    }

    fn return_abi<'b>(&self, binary: &'b Binary, data: PointerValue<'b>, length: IntValue) {
        binary.builder.build_call(
            binary.module.get_function("finish").unwrap(),
            &[data.into(), length.into()],
            "",
        );

        // since finish is marked noreturn, this should be optimized away
        // however it is needed to create valid LLVM IR
        binary.builder.build_unreachable();
    }

    // ewasm main cannot return any value
    fn return_code<'b>(&self, binary: &'b Binary, _ret: IntValue<'b>) {
        self.assert_failure(
            binary,
            binary
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .const_null(),
            binary.context.i32_type().const_zero(),
        );
    }

    fn assert_failure<'b>(&self, binary: &'b Binary, data: PointerValue, len: IntValue) {
        binary.builder.build_call(
            binary.module.get_function("revert").unwrap(),
            &[data.into(), len.into()],
            "",
        );

        // since revert is marked noreturn, this should be optimized away
        // however it is needed to create valid LLVM IR
        binary.builder.build_unreachable();
    }

    /// ABI encode into a vector for abi.encode* style builtin functions
    fn abi_encode_to_vector<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        packed: &[BasicValueEnum<'b>],
        args: &[BasicValueEnum<'b>],
        tys: &[ast::Type],
        ns: &ast::Namespace,
    ) -> PointerValue<'b> {
        ethabiencoder::encode_to_vector(binary, function, packed, args, tys, false, ns)
    }

    fn abi_encode<'b>(
        &self,
        binary: &Binary<'b>,
        selector: Option<IntValue<'b>>,
        load: bool,
        function: FunctionValue<'b>,
        args: &[BasicValueEnum<'b>],
        tys: &[ast::Type],
        ns: &ast::Namespace,
    ) -> (PointerValue<'b>, IntValue<'b>) {
        let mut tys = tys.to_vec();

        let packed = if let Some(selector) = selector {
            tys.insert(0, ast::Type::Uint(32));
            vec![selector.into()]
        } else {
            vec![]
        };

        self.encode(binary, None, load, function, &packed, args, &tys, ns)
    }

    fn abi_decode<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue<'b>,
        spec: &[ast::Parameter],
        ns: &ast::Namespace,
    ) {
        self.abi
            .decode(binary, function, args, data, length, spec, ns);
    }

    fn print(&self, binary: &Binary, string_ptr: PointerValue, string_len: IntValue) {
        binary.builder.build_call(
            binary.module.get_function("printMem").unwrap(),
            &[string_ptr.into(), string_len.into()],
            "",
        );
    }

    fn create_contract<'b>(
        &mut self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        success: Option<&mut BasicValueEnum<'b>>,
        contract_no: usize,
        constructor_no: Option<usize>,
        address: PointerValue<'b>,
        args: &[BasicValueEnum<'b>],
        _gas: IntValue<'b>,
        value: Option<IntValue<'b>>,
        _salt: Option<IntValue<'b>>,
        _space: Option<IntValue<'b>>,
        ns: &ast::Namespace,
    ) {
        let resolver_binary = &ns.contracts[contract_no];

        let target_binary = Binary::build(
            binary.context,
            resolver_binary,
            ns,
            "",
            binary.opt,
            binary.math_overflow_check,
        );

        // wasm
        let wasm = target_binary
            .code(Generate::Linked)
            .expect("compile should succeeed");

        let code = binary.emit_global_string(
            &format!("contract_{}_code", resolver_binary.name),
            &wasm,
            true,
        );

        let tys: Vec<ast::Type> = match constructor_no {
            Some(function_no) => ns.functions[function_no]
                .params
                .iter()
                .map(|p| p.ty.clone())
                .collect(),
            None => Vec::new(),
        };

        // input
        let (input, input_len) = self.encode(
            binary,
            Some((code, wasm.len() as u64)),
            false,
            function,
            &[],
            args,
            &tys,
            ns,
        );

        // value is a u128
        let value_ptr = binary
            .builder
            .build_alloca(binary.value_type(ns), "balance");

        binary.builder.build_store(
            value_ptr,
            match value {
                Some(v) => v,
                None => binary.value_type(ns).const_zero(),
            },
        );

        // address needs its bytes reordered
        let be_address = binary
            .builder
            .build_alloca(binary.address_type(ns), "be_address");

        // call create
        let ret = binary
            .builder
            .build_call(
                binary.module.get_function("create").unwrap(),
                &[
                    binary
                        .builder
                        .build_pointer_cast(
                            value_ptr,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "value_transfer",
                        )
                        .into(),
                    input.into(),
                    input_len.into(),
                    binary
                        .builder
                        .build_pointer_cast(
                            be_address,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "be_address",
                        )
                        .into(),
                ],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        binary.builder.build_call(
            binary.module.get_function("__beNtoleN").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        be_address,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary
                    .builder
                    .build_pointer_cast(
                        address,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary.context.i32_type().const_int(20, false).into(),
            ],
            "",
        );

        let is_success = binary.builder.build_int_compare(
            IntPredicate::EQ,
            ret,
            binary.context.i32_type().const_zero(),
            "success",
        );

        if let Some(success) = success {
            *success = is_success.into();
        } else {
            let success_block = binary.context.append_basic_block(function, "success");
            let bail_block = binary.context.append_basic_block(function, "bail");
            binary
                .builder
                .build_conditional_branch(is_success, success_block, bail_block);

            binary.builder.position_at_end(bail_block);

            self.assert_failure(
                binary,
                binary
                    .context
                    .i8_type()
                    .ptr_type(AddressSpace::Generic)
                    .const_null(),
                binary.context.i32_type().const_zero(),
            );

            binary.builder.position_at_end(success_block);
        }
    }

    fn external_call<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue,
        success: Option<&mut BasicValueEnum<'b>>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        address: Option<PointerValue<'b>>,
        gas: IntValue<'b>,
        value: IntValue<'b>,
        callty: ast::CallTy,
        ns: &ast::Namespace,
    ) {
        // address needs its bytes reordered
        let be_address = binary
            .builder
            .build_alloca(binary.address_type(ns), "be_address");

        binary.builder.build_call(
            binary.module.get_function("__leNtobeN").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        address.unwrap(),
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary
                    .builder
                    .build_pointer_cast(
                        be_address,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary.context.i32_type().const_int(20, false).into(),
            ],
            "",
        );

        let ret;

        if callty == ast::CallTy::Regular {
            // needs value

            // value is a u128
            let value_ptr = binary
                .builder
                .build_alloca(binary.value_type(ns), "balance");
            binary.builder.build_store(value_ptr, value);

            // call create
            ret = binary
                .builder
                .build_call(
                    binary.module.get_function("call").unwrap(),
                    &[
                        gas.into(),
                        binary
                            .builder
                            .build_pointer_cast(
                                be_address,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "address",
                            )
                            .into(),
                        binary
                            .builder
                            .build_pointer_cast(
                                value_ptr,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
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
                .into_int_value();
        } else {
            ret = binary
                .builder
                .build_call(
                    binary
                        .module
                        .get_function(match callty {
                            ast::CallTy::Regular => "call",
                            ast::CallTy::Static => "callStatic",
                            ast::CallTy::Delegate => "callDelegate",
                        })
                        .unwrap(),
                    &[
                        gas.into(),
                        binary
                            .builder
                            .build_pointer_cast(
                                be_address,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "address",
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
                .into_int_value();
        }

        let is_success = binary.builder.build_int_compare(
            IntPredicate::EQ,
            ret,
            binary.context.i32_type().const_zero(),
            "success",
        );

        if let Some(success) = success {
            *success = is_success.into();
        } else {
            let success_block = binary.context.append_basic_block(function, "success");
            let bail_block = binary.context.append_basic_block(function, "bail");
            binary
                .builder
                .build_conditional_branch(is_success, success_block, bail_block);

            binary.builder.position_at_end(bail_block);

            self.assert_failure(
                binary,
                binary
                    .context
                    .i8_type()
                    .ptr_type(AddressSpace::Generic)
                    .const_null(),
                binary.context.i32_type().const_zero(),
            );

            binary.builder.position_at_end(success_block);
        }
    }

    fn return_data<'b>(&self, binary: &Binary<'b>, _function: FunctionValue) -> PointerValue<'b> {
        let length = binary
            .builder
            .build_call(
                binary.module.get_function("getReturnDataSize").unwrap(),
                &[],
                "returndatasize",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let malloc_length = binary.builder.build_int_add(
            length,
            binary
                .module
                .get_struct_type("struct.vector")
                .unwrap()
                .size_of()
                .unwrap()
                .const_cast(binary.context.i32_type(), false),
            "size",
        );

        let p = binary
            .builder
            .build_call(
                binary.module.get_function("__malloc").unwrap(),
                &[malloc_length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let v = binary.builder.build_pointer_cast(
            p,
            binary
                .module
                .get_struct_type("struct.vector")
                .unwrap()
                .ptr_type(AddressSpace::Generic),
            "string",
        );

        let data_len = unsafe {
            binary.builder.build_gep(
                v,
                &[
                    binary.context.i32_type().const_zero(),
                    binary.context.i32_type().const_zero(),
                ],
                "data_len",
            )
        };

        binary.builder.build_store(data_len, length);

        let data_size = unsafe {
            binary.builder.build_gep(
                v,
                &[
                    binary.context.i32_type().const_zero(),
                    binary.context.i32_type().const_int(1, false),
                ],
                "data_size",
            )
        };

        binary.builder.build_store(data_size, length);

        let data = unsafe {
            binary.builder.build_gep(
                v,
                &[
                    binary.context.i32_type().const_zero(),
                    binary.context.i32_type().const_int(2, false),
                ],
                "data",
            )
        };

        binary.builder.build_call(
            binary.module.get_function("returnDataCopy").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        data,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary.context.i32_type().const_zero().into(),
                length.into(),
            ],
            "",
        );

        v
    }

    /// ewasm value is always 128 bits
    fn value_transferred<'b>(&self, binary: &Binary<'b>, ns: &ast::Namespace) -> IntValue<'b> {
        let value = binary
            .builder
            .build_alloca(binary.value_type(ns), "value_transferred");

        binary.builder.build_call(
            binary.module.get_function("getCallValue").unwrap(),
            &[binary
                .builder
                .build_pointer_cast(
                    value,
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "",
                )
                .into()],
            "value_transferred",
        );

        binary
            .builder
            .build_load(value, "value_transferred")
            .into_int_value()
    }

    /// Terminate execution, destroy binary and send remaining funds to addr
    fn selfdestruct<'b>(&self, binary: &Binary<'b>, addr: IntValue<'b>, ns: &ast::Namespace) {
        let address = binary
            .builder
            .build_alloca(binary.address_type(ns), "address");

        binary.builder.build_store(address, addr);

        binary.builder.build_call(
            binary.module.get_function("selfDestruct").unwrap(),
            &[binary
                .builder
                .build_pointer_cast(
                    address,
                    binary.context.i8_type().ptr_type(AddressSpace::Generic),
                    "",
                )
                .into()],
            "terminated",
        );
    }

    /// Crypto Hash
    fn hash<'b>(
        &self,
        binary: &Binary<'b>,
        _function: FunctionValue<'b>,

        hash: HashTy,
        input: PointerValue<'b>,
        input_len: IntValue<'b>,
        ns: &ast::Namespace,
    ) -> IntValue<'b> {
        let (precompile, hashlen) = match hash {
            HashTy::Keccak256 => (
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 20],
                32,
            ),
            HashTy::Ripemd160 => (
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3],
                20,
            ),
            HashTy::Sha256 => (
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2],
                32,
            ),
            _ => unreachable!(),
        };

        let res = binary.builder.build_array_alloca(
            binary.context.i8_type(),
            binary.context.i32_type().const_int(hashlen, false),
            "res",
        );

        let balance = binary
            .builder
            .build_alloca(binary.value_type(ns), "balance");

        binary
            .builder
            .build_store(balance, binary.value_type(ns).const_zero());

        let address = binary.emit_global_string(&format!("precompile_{}", hash), &precompile, true);

        binary.builder.build_call(
            binary.module.get_function("call").unwrap(),
            &[
                binary
                    .context
                    .i64_type()
                    .const_int(i64::MAX as u64, false)
                    .into(),
                binary
                    .builder
                    .build_pointer_cast(
                        address,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "address",
                    )
                    .into(),
                binary
                    .builder
                    .build_pointer_cast(
                        balance,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "balance",
                    )
                    .into(),
                input.into(),
                input_len.into(),
            ],
            "",
        );

        // We're not checking return value or returnDataSize;
        // assuming precompiles always succeed

        binary.builder.build_call(
            binary.module.get_function("returnDataCopy").unwrap(),
            &[
                res.into(),
                binary.context.i32_type().const_zero().into(),
                binary.context.i32_type().const_int(hashlen, false).into(),
            ],
            "",
        );

        // bytes32 needs to reverse bytes
        let temp = binary.builder.build_alloca(
            binary.llvm_type(&ast::Type::Bytes(hashlen as u8), ns),
            "hash",
        );

        binary.builder.build_call(
            binary.module.get_function("__beNtoleN").unwrap(),
            &[
                res.into(),
                binary
                    .builder
                    .build_pointer_cast(
                        temp,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary.context.i32_type().const_int(hashlen, false).into(),
            ],
            "",
        );

        binary.builder.build_load(temp, "hash").into_int_value()
    }

    /// Emit event
    fn emit_event<'b>(
        &self,
        binary: &Binary<'b>,
        _contract: &ast::Contract,
        function: FunctionValue<'b>,
        _event_no: usize,
        data: &[BasicValueEnum<'b>],
        data_tys: &[ast::Type],
        topics: &[BasicValueEnum<'b>],
        topic_tys: &[ast::Type],
        ns: &ast::Namespace,
    ) {
        let (data_ptr, data_len) =
            self.abi_encode(binary, None, false, function, data, data_tys, ns);

        let (mut topic_ptr, _) =
            self.abi_encode(binary, None, false, function, topics, topic_tys, ns);

        let empty_topic = binary
            .context
            .i8_type()
            .ptr_type(AddressSpace::Generic)
            .const_null();

        let mut topic_ptrs = [empty_topic; 4];

        #[allow(clippy::needless_range_loop)]
        for topic_no in 0..topics.len() {
            topic_ptrs[topic_no] = topic_ptr;

            topic_ptr = unsafe {
                binary.builder.build_gep(
                    topic_ptr,
                    &[binary.context.i32_type().const_int(32, false)],
                    "next_topic",
                )
            };
        }

        binary.builder.build_call(
            binary.module.get_function("log").unwrap(),
            &[
                data_ptr.into(),
                data_len.into(),
                binary
                    .context
                    .i32_type()
                    .const_int(topics.len() as u64, false)
                    .into(),
                topic_ptrs[0].into(),
                topic_ptrs[1].into(),
                topic_ptrs[2].into(),
                topic_ptrs[3].into(),
            ],
            "",
        );
    }

    /// builtin expressions
    fn builtin<'b>(
        &self,
        binary: &Binary<'b>,
        expr: &ast::Expression,
        vartab: &HashMap<usize, Variable<'b>>,
        function: FunctionValue<'b>,
        ns: &ast::Namespace,
    ) -> BasicValueEnum<'b> {
        macro_rules! straight_call {
            ($name:literal, $func:literal) => {{
                binary
                    .builder
                    .build_call(binary.module.get_function($func).unwrap(), &[], $name)
                    .try_as_basic_value()
                    .left()
                    .unwrap()
            }};
        }

        macro_rules! single_value_stack {
            ($name:literal, $func:literal, $width:expr) => {{
                let value = binary
                    .builder
                    .build_alloca(binary.context.custom_width_int_type($width), $name);

                binary.builder.build_call(
                    binary.module.get_function($func).unwrap(),
                    &[binary
                        .builder
                        .build_pointer_cast(
                            value,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into()],
                    $name,
                );

                binary.builder.build_load(value, $name)
            }};
        }

        match expr {
            ast::Expression::Builtin(_, _, ast::Builtin::BlockNumber, _) => {
                straight_call!("block_number", "getBlockNumber")
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Gasleft, _) => {
                straight_call!("gas_left", "getGasLeft")
            }
            ast::Expression::Builtin(_, _, ast::Builtin::GasLimit, _) => {
                straight_call!("gas_limit", "getBlockGasLimit")
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Timestamp, _) => {
                straight_call!("time_stamp", "getBlockTimestamp")
            }
            ast::Expression::Builtin(_, _, ast::Builtin::BlockDifficulty, _) => {
                single_value_stack!("block_difficulty", "getBlockDifficulty", 256)
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Origin, _) => {
                single_value_stack!("origin", "getTxOrigin", ns.address_length as u32 * 8)
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Sender, _) => {
                single_value_stack!("caller", "getCaller", ns.address_length as u32 * 8)
            }
            ast::Expression::Builtin(_, _, ast::Builtin::BlockCoinbase, _) => {
                single_value_stack!("coinbase", "getBlockCoinbase", ns.address_length as u32 * 8)
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Gasprice, _) => {
                single_value_stack!("gas_price", "getTxGasPrice", ns.value_length as u32 * 8)
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Value, _) => {
                single_value_stack!("value", "getCallValue", ns.value_length as u32 * 8)
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Calldata, _) => binary
                .builder
                .build_call(
                    binary.module.get_function("vector_new").unwrap(),
                    &[
                        binary
                            .builder
                            .build_load(binary.calldata_len.as_pointer_value(), "calldata_len")
                            .into(),
                        binary.context.i32_type().const_int(1, false).into(),
                        binary
                            .builder
                            .build_load(binary.calldata_data.as_pointer_value(), "calldata_data")
                            .into(),
                    ],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap(),
            ast::Expression::Builtin(_, _, ast::Builtin::BlockHash, args) => {
                let block_number = self.expression(binary, &args[0], vartab, function, ns);

                let value = binary
                    .builder
                    .build_alloca(binary.context.custom_width_int_type(256), "block_hash");

                binary.builder.build_call(
                    binary.module.get_function("getBlockHash").unwrap(),
                    &[
                        block_number.into(),
                        binary
                            .builder
                            .build_pointer_cast(
                                value,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                    ],
                    "block_hash",
                );

                binary.builder.build_load(value, "block_hash")
            }
            ast::Expression::Builtin(_, _, ast::Builtin::GetAddress, _) => {
                let value = binary
                    .builder
                    .build_alloca(binary.address_type(ns), "self_address");

                binary.builder.build_call(
                    binary.module.get_function("getAddress").unwrap(),
                    &[binary
                        .builder
                        .build_pointer_cast(
                            value,
                            binary.context.i8_type().ptr_type(AddressSpace::Generic),
                            "",
                        )
                        .into()],
                    "self_address",
                );

                binary.builder.build_load(value, "self_address")
            }
            ast::Expression::Builtin(_, _, ast::Builtin::Balance, addr) => {
                let addr = self
                    .expression(binary, &addr[0], vartab, function, ns)
                    .into_int_value();

                let address = binary
                    .builder
                    .build_alloca(binary.address_type(ns), "address");

                binary.builder.build_store(address, addr);

                let balance = binary
                    .builder
                    .build_alloca(binary.value_type(ns), "balance");

                binary.builder.build_call(
                    binary.module.get_function("getExternalBalance").unwrap(),
                    &[
                        binary
                            .builder
                            .build_pointer_cast(
                                address,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                        binary
                            .builder
                            .build_pointer_cast(
                                balance,
                                binary.context.i8_type().ptr_type(AddressSpace::Generic),
                                "",
                            )
                            .into(),
                    ],
                    "balance",
                );

                binary.builder.build_load(balance, "balance")
            }
            _ => unimplemented!(),
        }
    }
}
