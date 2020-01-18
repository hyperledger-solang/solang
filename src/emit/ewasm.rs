use parser::ast;
use resolver;
use std::str;

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::IntPredicate;

use std::collections::HashMap;

use super::{Contract, TargetRuntime};

pub struct EwasmTarget {
    /// This field maps a storage slot to llvm global
    slot_mapping: HashMap<usize, usize>,
}

impl EwasmTarget {
    pub fn build<'a>(
        context: &'a Context,
        contract: &'a resolver::Contract,
        filename: &'a str,
        opt: &str,
    ) -> Contract<'a> {
        // first emit runtime code
        let mut runtime_code = Contract::new(context, contract, filename, None);
        let mut b = EwasmTarget {
            slot_mapping: HashMap::new(),
        };

        // externals
        b.storage_keys(&mut runtime_code);
        b.declare_externals(&mut runtime_code);

        // FIXME: this also emits the constructors. We can either rely on lto linking
        // to optimize them away or do not emit them.
        runtime_code.emit_functions(&b);

        b.emit_function_dispatch(&runtime_code);

        let runtime_bs = runtime_code.wasm(opt).unwrap();

        // Now we have the runtime code, create the deployer
        let mut deploy_code =
            Contract::new(context, contract, filename, Some(Box::new(runtime_code)));
        let mut b = EwasmTarget {
            slot_mapping: HashMap::new(),
        };

        // externals
        b.storage_keys(&mut deploy_code);
        b.declare_externals(&mut deploy_code);

        // FIXME: this emits the constructors, as well as the functions. In Ethereum Solidity,
        // no functions can be called from the constructor. We should either disallow this too
        // and not emit functions, or use lto linking to optimize any unused functions away.
        deploy_code.emit_functions(&b);

        b.emit_constructor_dispatch(&mut deploy_code, &runtime_bs);

        deploy_code
    }

    fn storage_keys<'a>(&mut self, contract: &'a mut Contract) {
        for var in &contract.ns.variables {
            if let resolver::ContractVariableType::Storage(slot) = var.var {
                let mut key = slot.to_be_bytes().to_vec();

                // pad to the left
                let mut padding = Vec::new();

                padding.resize(32 - key.len(), 0u8);

                key = padding.into_iter().chain(key.into_iter()).collect();

                let v = contract.emit_global_string(&format!("sol::key::{}", var.name), &key, true);

                self.slot_mapping.insert(slot, v);
            }
        }
    }

    fn main_prelude<'a>(
        &self,
        contract: &'a Contract,
        function: FunctionValue,
    ) -> (PointerValue<'a>, IntValue<'a>) {
        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(&entry);

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

        if let Some(con) = contract.ns.constructors.get(0) {
            let mut args = Vec::new();

            // insert abi decode
            self.abi_decode(contract, function, &mut args, argsdata, length, con);

            contract
                .builder
                .build_call(contract.constructors[0], &args, "");
        }

        // the deploy code should return the runtime wasm code
        let runtime_code = contract.emit_global_string("runtime_code", runtime, true);

        let runtime_ptr = contract.builder.build_pointer_cast(
            contract.globals[runtime_code].as_pointer_value(),
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "runtime_code",
        );

        contract.builder.build_call(
            contract.module.get_function("finish").unwrap(),
            &[
                runtime_ptr.into(),
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

    fn emit_abi_encode_single_val(
        &self,
        contract: &Contract,
        ty: ast::PrimitiveType,
        dest: PointerValue,
        val: BasicValueEnum,
    ) {
        match ty {
            ast::PrimitiveType::Bool => {
                // first clear
                let dest8 = contract.builder.build_pointer_cast(
                    dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid",
                );

                contract.builder.build_call(
                    contract.module.get_function("__bzero8").unwrap(),
                    &[
                        dest8.into(),
                        contract.context.i32_type().const_int(4, false).into(),
                    ],
                    "",
                );

                let value = contract.builder.build_select(
                    val.into_int_value(),
                    contract.context.i8_type().const_int(1, false),
                    contract.context.i8_type().const_zero(),
                    "bool_val",
                );

                let dest = unsafe {
                    contract.builder.build_gep(
                        dest8,
                        &[contract.context.i32_type().const_int(31, false)],
                        "",
                    )
                };

                contract.builder.build_store(dest, value);
            }
            ast::PrimitiveType::Int(8) | ast::PrimitiveType::Uint(8) => {
                let signval = if let ast::PrimitiveType::Int(8) = ty {
                    let negative = contract.builder.build_int_compare(
                        IntPredicate::SLT,
                        val.into_int_value(),
                        contract.context.i8_type().const_zero(),
                        "neg",
                    );

                    contract
                        .builder
                        .build_select(
                            negative,
                            contract.context.i64_type().const_zero(),
                            contract.context.i64_type().const_int(std::u64::MAX, true),
                            "val",
                        )
                        .into_int_value()
                } else {
                    contract.context.i64_type().const_zero()
                };

                let dest8 = contract.builder.build_pointer_cast(
                    dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid",
                );

                contract.builder.build_call(
                    contract.module.get_function("__memset8").unwrap(),
                    &[
                        dest8.into(),
                        signval.into(),
                        contract.context.i32_type().const_int(4, false).into(),
                    ],
                    "",
                );

                let dest = unsafe {
                    contract.builder.build_gep(
                        dest8,
                        &[contract.context.i32_type().const_int(31, false)],
                        "",
                    )
                };

                contract.builder.build_store(dest, val);
            }
            ast::PrimitiveType::Address
            | ast::PrimitiveType::Uint(_)
            | ast::PrimitiveType::Int(_) => {
                let n = match ty {
                    ast::PrimitiveType::Address => 160,
                    ast::PrimitiveType::Uint(b) => b,
                    ast::PrimitiveType::Int(b) => b,
                    _ => unreachable!(),
                };

                // first clear/set the upper bits
                if n < 256 {
                    let signval = if let ast::PrimitiveType::Int(8) = ty {
                        let negative = contract.builder.build_int_compare(
                            IntPredicate::SLT,
                            val.into_int_value(),
                            contract.context.i8_type().const_zero(),
                            "neg",
                        );

                        contract
                            .builder
                            .build_select(
                                negative,
                                contract.context.i64_type().const_zero(),
                                contract.context.i64_type().const_int(std::u64::MAX, true),
                                "val",
                            )
                            .into_int_value()
                    } else {
                        contract.context.i64_type().const_zero()
                    };

                    let dest8 = contract.builder.build_pointer_cast(
                        dest,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "destvoid",
                    );

                    contract.builder.build_call(
                        contract.module.get_function("__memset8").unwrap(),
                        &[
                            dest8.into(),
                            signval.into(),
                            contract.context.i32_type().const_int(4, false).into(),
                        ],
                        "",
                    );
                }

                // no need to allocate space for each uint64
                // allocate enough for type
                let int_type = contract.context.custom_width_int_type(n as u32);
                let type_size = int_type.size_of();

                let store = if ty.stack_based() {
                    val.into_pointer_value()
                } else {
                    let store = contract.builder.build_alloca(int_type, "stack");

                    contract.builder.build_store(store, val);

                    store
                };

                contract.builder.build_call(
                    contract.module.get_function("__leNtobe32").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                store,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "store",
                            )
                            .into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                dest,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "dest",
                            )
                            .into(),
                        contract
                            .builder
                            .build_int_truncate(type_size, contract.context.i32_type(), "")
                            .into(),
                    ],
                    "",
                );
            }
            ast::PrimitiveType::Bytes(1) => {
                let dest8 = contract.builder.build_pointer_cast(
                    dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid",
                );

                contract.builder.build_call(
                    contract.module.get_function("__bzero8").unwrap(),
                    &[
                        dest8.into(),
                        contract.context.i32_type().const_int(4, false).into(),
                    ],
                    "",
                );

                contract.builder.build_store(dest8, val);
            }
            ast::PrimitiveType::Bytes(b) => {
                // first clear/set the upper bits
                if b < 32 {
                    let dest8 = contract.builder.build_pointer_cast(
                        dest,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "destvoid",
                    );

                    contract.builder.build_call(
                        contract.module.get_function("__bzero8").unwrap(),
                        &[
                            dest8.into(),
                            contract.context.i32_type().const_int(4, false).into(),
                        ],
                        "",
                    );
                }

                // no need to allocate space for each uint64
                // allocate enough for type
                let int_type = contract.context.custom_width_int_type(b as u32 * 8);
                let type_size = int_type.size_of();

                let store = if ty.stack_based() {
                    val.into_pointer_value()
                } else {
                    let store = contract.builder.build_alloca(int_type, "stack");

                    contract.builder.build_store(store, val);

                    store
                };

                contract.builder.build_call(
                    contract.module.get_function("__leNtobeN").unwrap(),
                    &[
                        contract
                            .builder
                            .build_pointer_cast(
                                store,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "store",
                            )
                            .into(),
                        contract
                            .builder
                            .build_pointer_cast(
                                dest,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                "dest",
                            )
                            .into(),
                        contract
                            .builder
                            .build_int_truncate(type_size, contract.context.i32_type(), "")
                            .into(),
                    ],
                    "",
                );
            }
            _ => unimplemented!(),
        }
    }
}

impl TargetRuntime for EwasmTarget {
    fn set_storage<'a>(
        &self,
        contract: &'a Contract,
        _function: FunctionValue,
        slot: u32,
        dest: inkwell::values::PointerValue<'a>,
    ) {
        let key = contract.globals[self.slot_mapping[&(slot as usize)]];
        // FIXME: no need to alloca for 256 bit value
        let value = contract
            .builder
            .build_alloca(contract.context.custom_width_int_type(160), "value");

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
            contract.module.get_function("storageLoad").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        key.as_pointer_value(),
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                value8.into(),
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
        let key = contract.globals[self.slot_mapping[&(slot as usize)]];
        // FIXME: no need to alloca for 256 bit value
        let value = contract
            .builder
            .build_alloca(contract.context.custom_width_int_type(256), "value");

        contract.builder.build_call(
            contract.module.get_function("storageLoad").unwrap(),
            &[
                contract
                    .builder
                    .build_pointer_cast(
                        key.as_pointer_value(),
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                contract
                    .builder
                    .build_pointer_cast(
                        value,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
            ],
            "",
        );

        let val = contract.builder.build_load(
            contract
                .builder
                .build_pointer_cast(value, dest.get_type(), ""),
            "",
        );

        contract.builder.build_store(dest, val);
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

        contract.builder.build_return(None);
    }

    fn return_abi<'b>(&self, contract: &'b Contract, data: PointerValue<'b>, length: IntValue) {
        contract.builder.build_call(
            contract.module.get_function("finish").unwrap(),
            &[data.into(), length.into()],
            "",
        );

        contract.builder.build_return(None);
    }

    fn abi_encode<'b>(
        &self,
        contract: &'b Contract,
        args: &[BasicValueEnum<'b>],
        spec: &resolver::FunctionDecl,
    ) -> (PointerValue<'b>, IntValue<'b>) {
        let length = contract
            .context
            .i32_type()
            .const_int(32 * args.len() as u64, false);
        let mut data = contract
            .builder
            .build_call(
                contract.module.get_function("__malloc").unwrap(),
                &[contract
                    .context
                    .i32_type()
                    .const_int(32 * args.len() as u64, false)
                    .into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // malloc returns u8*
        for (i, arg) in spec.returns.iter().enumerate() {
            // insert abi decode
            let ty = match arg.ty {
                resolver::Type::Primitive(e) => e,
                resolver::Type::Enum(n) => contract.ns.enums[n].ty,
                resolver::Type::FixedArray(_, _) => unimplemented!(),
                resolver::Type::Noreturn => unreachable!(),
            };

            self.emit_abi_encode_single_val(contract, ty, data, args[i]);

            data = unsafe {
                contract.builder.build_gep(
                    data,
                    &[contract.context.i32_type().const_int(32, false)],
                    &format!("abi{}", i),
                )
            };
        }

        (data, length)
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
        let mut data = data;
        let decode_block = contract.context.append_basic_block(function, "abi_decode");
        let wrong_length_block = contract
            .context
            .append_basic_block(function, "wrong_abi_length");

        let is_ok = contract.builder.build_int_compare(
            IntPredicate::EQ,
            length,
            contract
                .context
                .i32_type()
                .const_int(32 * spec.params.len() as u64, false),
            "correct_length",
        );

        contract
            .builder
            .build_conditional_branch(is_ok, &decode_block, &wrong_length_block);

        contract.builder.position_at_end(&decode_block);

        for arg in &spec.params {
            let ty = match &arg.ty {
                resolver::Type::Primitive(e) => e,
                resolver::Type::Enum(n) => &contract.ns.enums[*n].ty,
                resolver::Type::FixedArray(_, _) => unimplemented!(),
                resolver::Type::Noreturn => unreachable!(),
            };

            args.push(match ty {
                ast::PrimitiveType::Bool => {
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

                    contract
                        .builder
                        .build_int_compare(
                            IntPredicate::NE,
                            contract
                                .builder
                                .build_load(bool_ptr, "abi_bool")
                                .into_int_value(),
                            contract.context.i64_type().const_zero(),
                            "bool",
                        )
                        .into()
                }
                ast::PrimitiveType::Uint(8) | ast::PrimitiveType::Int(8) => {
                    let int8_ptr = contract.builder.build_pointer_cast(
                        data,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    );

                    let int8_ptr = unsafe {
                        contract.builder.build_gep(
                            int8_ptr,
                            &[contract.context.i32_type().const_int(31, false)],
                            "bool_ptr",
                        )
                    };

                    contract.builder.build_load(int8_ptr, "abi_int8")
                }
                ast::PrimitiveType::Address => {
                    let int_type = contract.context.custom_width_int_type(160);
                    let type_size = int_type.size_of();

                    let store = contract.builder.build_alloca(int_type, "address");

                    contract.builder.build_call(
                        contract.module.get_function("__be32toleN").unwrap(),
                        &[
                            contract
                                .builder
                                .build_pointer_cast(
                                    data,
                                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                    "",
                                )
                                .into(),
                            contract
                                .builder
                                .build_pointer_cast(
                                    store,
                                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                    "",
                                )
                                .into(),
                            contract
                                .builder
                                .build_int_truncate(type_size, contract.context.i32_type(), "size")
                                .into(),
                        ],
                        "",
                    );

                    store.into()
                }
                ast::PrimitiveType::Uint(n) | ast::PrimitiveType::Int(n) => {
                    let int_type = contract.context.custom_width_int_type(*n as u32);
                    let type_size = int_type.size_of();

                    let store = contract.builder.build_alloca(int_type, "stack");

                    contract.builder.build_call(
                        contract.module.get_function("__be32toleN").unwrap(),
                        &[
                            contract
                                .builder
                                .build_pointer_cast(
                                    data,
                                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                    "",
                                )
                                .into(),
                            contract
                                .builder
                                .build_pointer_cast(
                                    store,
                                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                    "",
                                )
                                .into(),
                            contract
                                .builder
                                .build_int_truncate(type_size, contract.context.i32_type(), "size")
                                .into(),
                        ],
                        "",
                    );

                    if *n <= 64 {
                        contract
                            .builder
                            .build_load(store, &format!("abi_int{}", *n))
                    } else {
                        store.into()
                    }
                }
                ast::PrimitiveType::Bytes(1) => contract.builder.build_load(
                    contract.builder.build_pointer_cast(
                        data,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "bytes1",
                ),
                ast::PrimitiveType::Bytes(b) => {
                    let int_type = contract.context.custom_width_int_type(*b as u32 * 8);
                    let type_size = int_type.size_of();

                    let store = contract.builder.build_alloca(int_type, "stack");

                    contract.builder.build_call(
                        contract.module.get_function("__beNtoleN").unwrap(),
                        &[
                            contract
                                .builder
                                .build_pointer_cast(
                                    data,
                                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                    "",
                                )
                                .into(),
                            contract
                                .builder
                                .build_pointer_cast(
                                    store,
                                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                                    "",
                                )
                                .into(),
                            contract
                                .builder
                                .build_int_truncate(type_size, contract.context.i32_type(), "size")
                                .into(),
                        ],
                        "",
                    );

                    if *b <= 8 {
                        contract.builder.build_load(store, &format!("bytes{}", *b))
                    } else {
                        store.into()
                    }
                }
                _ => panic!(),
            });

            data = unsafe {
                contract.builder.build_gep(
                    data,
                    &[contract.context.i32_type().const_int(8, false)],
                    "data_next",
                )
            };
        }

        // FIXME: generate a call to revert/abort with some human readable error or error code
        contract.builder.position_at_end(&wrong_length_block);
        contract.builder.build_unreachable();

        contract.builder.position_at_end(&decode_block);
    }

    fn assert_failure<'b>(&self, contract: &'b Contract) {
        contract.builder.build_call(
            contract.module.get_function("revert").unwrap(),
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

        // since revert is marked noreturn, this should be optimized away
        // however it is needed to create valid LLVM IR
        contract.builder.build_unreachable();
    }
}
