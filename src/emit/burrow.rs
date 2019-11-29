
use parser::ast;
use resolver;
use std::str;

use inkwell::types::BasicTypeEnum;
use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::AddressSpace;
use inkwell::values::{PointerValue, IntValue, FunctionValue, BasicValueEnum};
use inkwell::IntPredicate;

use super::{TargetRuntime, Contract};

pub struct BurrowTarget {
}

impl BurrowTarget {
    pub fn build<'a>(context: &'a Context, contract: &'a resolver::Contract, filename: &'a str) -> Contract<'a> {
        let mut c = Contract::new(context, contract, filename);
        let b = BurrowTarget{};

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
            contract.context.i8_type().ptr_type(AddressSpace::Generic).into(),
            contract.context.i32_type().into(),
        ];

        let ftype = ret.fn_type(&args, false);
        
        contract.module.add_function("get_storage32", ftype, Some(Linkage::External));
        contract.module.add_function("set_storage32", ftype, Some(Linkage::External));
    }

    fn emit_constructor_dispatch(&self, contract: &Contract) {
        // create start function
        let ret = contract.context.void_type();
        let ftype = ret.fn_type(&[contract.context.i32_type().ptr_type(AddressSpace::Generic).into()], false);
        let function = contract.module.add_function("constructor", ftype, None);

        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(&entry);

        // init our heap
        contract.builder.build_call(
            contract.module.get_function("__init_heap").unwrap(),
            &[],
            "");

        if let Some(con) = contract.ns.constructors.get(0) {
            let mut args = Vec::new();

            let arg = function.get_first_param().unwrap().into_pointer_value();
            let length = contract.builder.build_load(arg, "length");

            // step over length
            let args_ptr = unsafe {
                contract.builder.build_gep(arg,
                    &[contract.context.i32_type().const_int(1, false).into()],
                    "args_ptr")
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

            contract.builder.build_call(contract.constructors[0].value_ref, &args, "");
        }

        contract.builder.build_return(None);
    }

    fn emit_function_dispatch(&self, contract: &Contract) {
        // create start function
        let ret = contract.context.i32_type().ptr_type(AddressSpace::Generic);
        let ftype = ret.fn_type(&[contract.context.i32_type().ptr_type(AddressSpace::Generic).into()], false);
        let function = contract.module.add_function("function", ftype, None);

        let entry = contract.context.append_basic_block(function, "entry");
        let fallback_block = contract.context.append_basic_block(function, "fallback");

        contract.builder.position_at_end(&entry);

        let data = function.get_first_param().unwrap().into_pointer_value();
        let argslen = contract.builder.build_load(data, "length").into_int_value();

        let argsdata = unsafe {
            contract.builder.build_gep(
                data,
                &[contract.context.i32_type().const_int(1, false).into()],
                "argsdata")
        };

        contract.emit_function_dispatch(&contract.ns.functions, argsdata, argslen, function, &fallback_block, self);

        // emit fallback code
        contract.builder.position_at_end(&fallback_block);

        match contract.ns.fallback_function() {
            Some(f) => {
                contract.builder.build_call(
                    contract.functions[f].value_ref,
                    &[],
                    "");

                contract.builder.build_return(None);
            }
            None => {
                contract.builder.build_unreachable();
            },
        }
    }

    fn emit_abi_encode_single_val(
        &self,
        contract: &Contract,
        ty: &ast::ElementaryTypeName,
        dest: PointerValue,
        val: IntValue,
    ) {
        match ty {
            ast::ElementaryTypeName::Bool => {
                // first clear
                let dest8 = contract.builder.build_pointer_cast(dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid");

                contract.builder.build_call(
                    contract.module.get_function("__bzero8").unwrap(),
                    &[ dest8.into(),
                       contract.context.i32_type().const_int(4, false).into() ],
                    "");

                let value = contract.builder.build_select(val,
                    contract.context.i8_type().const_int(1, false),
                    contract.context.i8_type().const_zero(),
                    "bool_val");

                let dest = unsafe {
                    contract.builder.build_gep(
                        dest8,
                        &[ contract.context.i32_type().const_int(31, false).into() ],
                        "")
                };

                contract.builder.build_store(dest, value);
            }
            ast::ElementaryTypeName::Int(8) | ast::ElementaryTypeName::Uint(8) => {
                let signval = if let ast::ElementaryTypeName::Int(8) = ty {
                    let negative = contract.builder.build_int_compare(IntPredicate::SLT,
                            val, contract.context.i8_type().const_zero(), "neg");

                            contract.builder.build_select(negative,
                        contract.context.i64_type().const_zero(),
                        contract.context.i64_type().const_int(std::u64::MAX, true),
                        "val").into_int_value()
                } else {
                    contract.context.i64_type().const_zero()
                };

                let dest8 = contract.builder.build_pointer_cast(dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic),
                    "destvoid");

                    contract.builder.build_call(
                    contract.module.get_function("__memset8").unwrap(),
                    &[ dest8.into(), signval.into(),
                       contract.context.i32_type().const_int(4, false).into() ],
                    "");

                let dest = unsafe {
                    contract.builder.build_gep(
                        dest8,
                        &[ contract.context.i32_type().const_int(31, false).into() ],
                        "")
                };

                contract.builder.build_store(dest, val);
            }
            ast::ElementaryTypeName::Uint(n) | ast::ElementaryTypeName::Int(n) => {
                // first clear/set the upper bits
                if *n < 256 {
                    let signval = if let ast::ElementaryTypeName::Int(8) = ty {
                        let negative = contract.builder.build_int_compare(IntPredicate::SLT,
                                val, contract.context.i8_type().const_zero(), "neg");

                        contract.builder.build_select(negative,
                            contract.context.i64_type().const_zero(),
                            contract.context.i64_type().const_int(std::u64::MAX, true),
                            "val").into_int_value()
                    } else {
                        contract.context.i64_type().const_zero()
                    };

                    let dest8 = contract.builder.build_pointer_cast(dest,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic),
                        "destvoid");

                    contract.builder.build_call(
                        contract.module.get_function("__memset8").unwrap(),
                        &[ dest8.into(), signval.into(),
                            contract.context.i32_type().const_int(4, false).into() ],
                        "");
                }

                // no need to allocate space for each uint64
                // allocate enough for type
                let int_type = contract.context.custom_width_int_type(*n as u32);
                let type_size = int_type.size_of();

                let store = contract.builder.build_alloca(int_type, "stack");

                contract.builder.build_store(store, val);

                contract.builder.build_call(
                    contract.module.get_function("__leNtobe32").unwrap(),
                    &[ contract.builder.build_pointer_cast(store,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "destvoid").into(),
                        contract.builder.build_pointer_cast(dest,
                            contract.context.i8_type().ptr_type(AddressSpace::Generic),
                            "destvoid").into(),
                        contract.builder.build_int_truncate(type_size,
                            contract.context.i32_type(), "").into()
                    ],
                    "");
            }
            _ => unimplemented!(),
        }
    }

}

impl TargetRuntime for BurrowTarget {
    fn set_storage<'a>(&self, contract: &'a Contract, _function: FunctionValue, slot: u32, dest: inkwell::values::PointerValue<'a>) {
        contract.builder.build_call(
            contract.module.get_function("set_storage32").unwrap(),
            &[
                contract.context.i32_type().const_int(slot as u64, false).into(),
                contract.builder.build_pointer_cast(dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic), "").into(),
                dest.get_type().size_of().const_cast(
                    contract.context.i32_type(), false).into()
            ],
            "");
    }

    fn get_storage<'a>(&self, contract: &'a Contract, _function: FunctionValue, slot: u32, dest: inkwell::values::PointerValue<'a>) {
        contract.builder.build_call(
            contract.module.get_function("get_storage32").unwrap(),
            &[
                contract.context.i32_type().const_int(slot as u64, false).into(),
                contract.builder.build_pointer_cast(dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic), "").into(),
                dest.get_type().size_of().const_cast(
                    contract.context.i32_type(), false).into()
            ],
            "");
    }

    fn return_empty_abi(&self, contract: &Contract) {
        let dest = contract.builder.build_call(
            contract.module.get_function("__malloc").unwrap(),
            &[contract.context.i32_type().const_int(4, false).into()],
            ""
        ).try_as_basic_value().left().unwrap().into_pointer_value();

        contract.builder.build_store(
            contract.builder.build_pointer_cast(dest,
                contract.context.i32_type().ptr_type(AddressSpace::Generic),
                ""),
            contract.context.i32_type().const_zero());

        contract.builder.build_return(Some(&dest));
    }

    fn return_abi<'b>(&self, contract: &'b Contract, data: PointerValue<'b>, _length: IntValue) {
        contract.builder.build_return(Some(&data));
    }

    fn abi_encode<'b>(
        &self,
        contract: &'b Contract,
        args: &[ BasicValueEnum<'b> ],
        spec: &resolver::FunctionDecl,
    ) -> (PointerValue<'b>, IntValue<'b>) {
        let length = contract.context.i32_type().const_int(32 * args.len() as u64, false);
        let data = contract.builder.build_call(
            contract.module.get_function("__malloc").unwrap(),
            &[contract.context.i32_type().const_int(4 + 32 * args.len() as u64, false).into()],
            ""
        ).try_as_basic_value().left().unwrap().into_pointer_value();

        // write length
        contract.builder.build_store(
            contract.builder.build_pointer_cast(data,
                contract.context.i32_type().ptr_type(AddressSpace::Generic),
                ""),
            length);

        // malloc returns u8*
        let abi_ptr = unsafe {
            contract.builder.build_gep(
                data,
                &[ contract.context.i32_type().const_int(4, false).into()],
                "abi_ptr")
        };

        for (i, arg) in spec.returns.iter().enumerate() {
            // insert abi decode
            let ty = match arg.ty {
                resolver::TypeName::Elementary(e) => e,
                resolver::TypeName::Enum(n) => contract.ns.enums[n].ty,
                resolver::TypeName::Noreturn => unreachable!(),
            };

            self.emit_abi_encode_single_val(contract, &ty, abi_ptr, args[i].into_int_value());
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
        let wrong_length_block = contract.context.append_basic_block(function, "wrong_abi_length");

        let is_ok = contract.builder.build_int_compare(IntPredicate::EQ, length,
            contract.context.i32_type().const_int(32  * spec.params.len() as u64, false),
            "correct_length");

        contract.builder.build_conditional_branch(is_ok, &decode_block, &wrong_length_block);

        contract.builder.position_at_end(&decode_block);

        for arg in &spec.params {
            let ty = match &arg.ty {
                resolver::TypeName::Elementary(e) => e,
                resolver::TypeName::Enum(n) => &contract.ns.enums[*n].ty,
                resolver::TypeName::Noreturn => unreachable!(),
            };

            args.push(match ty {
                ast::ElementaryTypeName::Bool => {
                    // solidity checks all the 32 bytes for being non-zero; we will just look at the upper 8 bytes, else we would need four loads
                    // which is unneeded (hopefully)
                    // cast to 64 bit pointer
                    let bool_ptr = contract.builder.build_pointer_cast(data,
                        contract.context.i64_type().ptr_type(AddressSpace::Generic), "");

                    let bool_ptr = unsafe {
                        contract.builder.build_gep(bool_ptr,
                            &[ contract.context.i32_type().const_int(3, false) ],
                            "bool_ptr")
                    };

                    contract.builder.build_int_compare(IntPredicate::NE,
                        contract.builder.build_load(bool_ptr, "abi_bool").into_int_value(),
                        contract.context.i64_type().const_zero(), "bool").into()
                }
                ast::ElementaryTypeName::Uint(8) | ast::ElementaryTypeName::Int(8) => {
                    let int8_ptr = contract.builder.build_pointer_cast(data,
                        contract.context.i8_type().ptr_type(AddressSpace::Generic), "");

                    let int8_ptr = unsafe {
                        contract.builder.build_gep(int8_ptr,
                        &[ contract.context.i32_type().const_int(31, false) ],
                        "bool_ptr")
                    };

                    contract.builder.build_load(int8_ptr, "abi_int8")
                }
                ast::ElementaryTypeName::Uint(n) | ast::ElementaryTypeName::Int(n) => {
                    let int_type = contract.context.custom_width_int_type(*n as u32);
                    let type_size = int_type.size_of();

                    let store = contract.builder.build_alloca(int_type, "stack");

                    contract.builder.build_call(
                        contract.module.get_function("__be32toleN").unwrap(),
                        &[
                            contract.builder.build_pointer_cast(data,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic), "").into(),
                            contract.builder.build_pointer_cast(store,
                                contract.context.i8_type().ptr_type(AddressSpace::Generic), "").into(),
                            contract.builder.build_int_truncate(type_size,
                                contract.context.i32_type(), "size").into()
                        ],
                        ""
                    );

                    if *n <= 64 {
                        contract.builder.build_load(store, &format!("abi_int{}", *n))
                    } else {
                        store.into()
                    }
                }
                _ => panic!(),
            });

            data = unsafe {
                contract.builder.build_gep(data,
                    &[ contract.context.i32_type().const_int(8, false)],
                    "data_next")
            };
        }

        // FIXME: generate a call to revert/abort with some human readable error or error code
        contract.builder.position_at_end(&wrong_length_block);
        contract.builder.build_unreachable();

        contract.builder.position_at_end(&decode_block);
    }

}
