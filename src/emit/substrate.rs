
use resolver;
use parser::ast;

use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::AddressSpace;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue, FunctionValue};
use inkwell::IntPredicate;

use std::collections::HashMap;

use super::{TargetRuntime, Contract};

pub struct SubstrateTarget {
    /// This field maps a storage slot to llvm global
    slot_mapping: HashMap<usize, usize>
}

impl SubstrateTarget {
    pub fn build<'a>(context: &'a Context, contract: &'a resolver::Contract, filename: &'a str) -> Contract<'a> {
        let mut c = Contract::new(context, contract, filename);
        let mut b = SubstrateTarget{
            slot_mapping: HashMap::new()
        };

        b.storage_keys(&mut c);
        b.declare_externals(&c);

        c.emit_functions(&b);

        b.emit_deploy(&c);
        b.emit_call(&c);

        c
    }

    fn storage_keys<'a>(&mut self, contract: &'a mut Contract) {
        for var in &contract.ns.variables {
            if let Some(slot) = var.storage {
                let mut key = slot.to_le_bytes().to_vec();

                key.resize(32, 0);

                let v = contract.emit_global_string(&format!("sol::key::{}", var.name), &key, true);

                self.slot_mapping.insert(slot, v);
            }
        }
    }

    fn public_function_prelude<'a>(&self, contract: &'a Contract, function: FunctionValue) -> (PointerValue<'a>, IntValue<'a>) {
        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(&entry);

        // init our heap
        contract.builder.build_call(
            contract.module.get_function("__init_heap").unwrap(),
            &[],
            "");

        // copy arguments from scratch buffer
        let args_length = contract.builder.build_call(
            contract.module.get_function("ext_scratch_size").unwrap(),
            &[],
            "scratch_size").try_as_basic_value().left().unwrap();

        let args = contract.builder.build_call(
            contract.module.get_function("__malloc").unwrap(),
            &[args_length],
            ""
        ).try_as_basic_value().left().unwrap().into_pointer_value();

        contract.builder.build_call(
            contract.module.get_function("ext_scratch_read").unwrap(),
            &[
                args.into(),
                contract.context.i32_type().const_zero().into(),
                args_length.into(),
            ],
            ""
        );

        let args = contract.builder.build_pointer_cast(args,
            contract.context.i32_type().ptr_type(AddressSpace::Generic), "").into();

        (args, args_length.into_int_value())
    }

    fn declare_externals(&self, contract: &Contract) {
        // Access to scratch buffer
        contract.module.add_function(
            "ext_scratch_size",
            contract.context.i32_type().fn_type(&[], false),
            Some(Linkage::External)
        );

        contract.module.add_function(
            "ext_scratch_read",
            contract.context.void_type().fn_type(&[
                contract.context.i8_type().ptr_type(AddressSpace::Generic).into(),  // dest_ptr
                contract.context.i32_type().into(), // offset
                contract.context.i32_type().into(), // len
            ], false),
            Some(Linkage::External)
        );

        contract.module.add_function(
            "ext_scratch_write",
            contract.context.void_type().fn_type(&[
                contract.context.i8_type().ptr_type(AddressSpace::Generic).into(),  // dest_ptr
                contract.context.i32_type().into(), // len
            ], false),
            Some(Linkage::External)
        );

        contract.module.add_function(
            "ext_set_storage",
            contract.context.void_type().fn_type(&[
                contract.context.i8_type().ptr_type(AddressSpace::Generic).into(), // key_ptr
                contract.context.i32_type().into(), // value_non_null
                contract.context.i8_type().ptr_type(AddressSpace::Generic).into(), // value_ptr
                contract.context.i32_type().into(), // value_len
            ], false),
            Some(Linkage::External)
        );

        contract.module.add_function(
            "ext_get_storage",
            contract.context.i32_type().fn_type(&[
                contract.context.i8_type().ptr_type(AddressSpace::Generic).into(), // key_ptr
            ], false),
            Some(Linkage::External)
        );

        contract.module.add_function(
            "ext_return",
            contract.context.void_type().fn_type(&[
                contract.context.i8_type().ptr_type(AddressSpace::Generic).into(), // data_ptr
                contract.context.i32_type().into(), // data_len
            ], false),
            Some(Linkage::External)
        );
    }

    fn emit_deploy(&self, contract: &Contract) {
        // create deploy function
        let function = contract.module.add_function(
            "deploy",
            contract.context.i32_type().fn_type(&[], false),
            None);

        let (deploy_args, deploy_args_length) = self.public_function_prelude(contract, function);

        let fallback_block = contract.context.append_basic_block(function, "fallback");

        contract.emit_function_dispatch(&contract.ns.constructors, &contract.constructors, deploy_args, deploy_args_length, function, &fallback_block, self);

        // emit fallback code
        contract.builder.position_at_end(&fallback_block);
        contract.builder.build_unreachable();
    }

    fn emit_call(&self, contract: &Contract) {
        // create call function
        let function = contract.module.add_function(
            "call",
            contract.context.i32_type().fn_type(&[], false),
            None);

        let (call_args, call_args_length) = self.public_function_prelude(contract, function);

        let fallback_block = contract.context.append_basic_block(function, "fallback");

        contract.emit_function_dispatch(&contract.ns.functions, &contract.functions, call_args, call_args_length, function, &fallback_block, self);

        // emit fallback code
        contract.builder.position_at_end(&fallback_block);

        if let Some(fallback) = contract.ns.fallback_function() {
            contract.builder.build_call(
                contract.functions[fallback].value_ref,
                &[],
                "");

            contract.builder.build_return(Some(&contract.context.i32_type().const_zero()));
        } else {
            contract.builder.build_unreachable();
        }
    }
}

impl TargetRuntime for SubstrateTarget {
    fn set_storage<'a>(&self, contract: &'a Contract, _function: FunctionValue, slot: u32, dest: inkwell::values::PointerValue<'a>) {
        // FIXME: check for non-zero
        let key = contract.globals[self.slot_mapping[&(slot as usize)]];

        contract.builder.build_call(
            contract.module.get_function("ext_set_storage").unwrap(),
            &[
                contract.builder.build_pointer_cast(key.as_pointer_value(),
                    contract.context.i8_type().ptr_type(AddressSpace::Generic), "").into(),
                contract.context.i32_type().const_int(1, false).into(),
                contract.builder.build_pointer_cast(dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic), "").into(),
                dest.get_type().get_element_type().into_int_type().size_of().const_cast(
                    contract.context.i32_type(), false).into()
            ],
            "");
    }

    /// Read from substrate storage
    fn get_storage<'a>(&self, contract: &'a Contract, function: FunctionValue, slot: u32, dest: inkwell::values::PointerValue<'a>) {
        let key = contract.globals[self.slot_mapping[&(slot as usize)]];

        let exists = contract.builder.build_call(
            contract.module.get_function("ext_get_storage").unwrap(),
            &[
                contract.builder.build_pointer_cast(key.as_pointer_value(),
                    contract.context.i8_type().ptr_type(AddressSpace::Generic), "").into(),
            ],
            "").try_as_basic_value().left().unwrap();

        let exists = contract.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
            contract.context.i32_type().const_zero(),
            "storage_exists");
        
        let clear_block = contract.context.append_basic_block(function, "not_in_storage");
        let retrieve_block = contract.context.append_basic_block(function, "in_storage");
        let done_storage = contract.context.append_basic_block(function, "done_storage");
        
        contract.builder.build_conditional_branch(exists, &retrieve_block, &clear_block);

        contract.builder.position_at_end(&retrieve_block);
        
        contract.builder.build_call(
            contract.module.get_function("ext_scratch_read").unwrap(),
            &[
                contract.builder.build_pointer_cast(dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic), "").into(),
                contract.context.i32_type().const_zero().into(),
                dest.get_type().get_element_type().into_int_type().size_of().const_cast(
                    contract.context.i32_type(), false).into()
            ],
            ""
        );

        contract.builder.build_unconditional_branch(&done_storage);

        contract.builder.position_at_end(&clear_block);

        contract.builder.build_store(dest, dest.get_type().get_element_type().into_int_type().const_zero());

        contract.builder.build_unconditional_branch(&done_storage);

        contract.builder.position_at_end(&done_storage);
    }

    fn abi_decode<'b>(
        &self,
        contract: &'b Contract,
        function: FunctionValue,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        datalength: IntValue,
        spec: &resolver::FunctionDecl,
    ) {
        let mut length = 0;

        for arg in spec.params.iter() {
            let ty = match arg.ty {
                resolver::TypeName::Elementary(e) => e,
                resolver::TypeName::Enum(n) => contract.ns.enums[n].ty,
                resolver::TypeName::Noreturn => unreachable!(),
            };

            match ty {
                ast::ElementaryTypeName::Bool => length += 1,
                ast::ElementaryTypeName::Uint(n) |
                ast::ElementaryTypeName::Int(n) => length += n as u64 / 8,
                _ => unimplemented!()
            }
        }

        let decode_block = contract.context.append_basic_block(function, "abi_decode");
        let wrong_length_block = contract.context.append_basic_block(function, "wrong_abi_length");

        let is_ok = contract.builder.build_int_compare(IntPredicate::EQ, datalength,
            contract.context.i32_type().const_int(length, false),
            "correct_length");

        contract.builder.build_conditional_branch(is_ok, &decode_block, &wrong_length_block);

        contract.builder.position_at_end(&wrong_length_block);
        contract.builder.build_unreachable();

        contract.builder.position_at_end(&decode_block);

        let mut argsdata = contract.builder.build_pointer_cast(data,
            contract.context.i8_type().ptr_type(AddressSpace::Generic), "");

        for (i, arg) in spec.params.iter().enumerate() {
            let ty = match arg.ty {
                resolver::TypeName::Elementary(e) => e,
                resolver::TypeName::Enum(n) => contract.ns.enums[n].ty,
                resolver::TypeName::Noreturn => unreachable!(),
            };

            let arglen;

            match ty {
                ast::ElementaryTypeName::Bool => {
                    args.push(contract.builder.build_int_compare(IntPredicate::EQ,
                        contract.builder.build_load(argsdata, "abi_bool").into_int_value(),
                        contract.context.i8_type().const_int(1, false), "bool").into());
                    arglen = 1;
                },
                ast::ElementaryTypeName::Uint(n) |
                ast::ElementaryTypeName::Int(n) => {
                    args.push(contract.builder.build_load(
                        contract.builder.build_pointer_cast(argsdata,
                            args[i].into_int_value().get_type().ptr_type(AddressSpace::Generic),
                            ""),
                        ""));
                    arglen = n as u64 / 8;
                },
                _ => unimplemented!()
            }

            argsdata = unsafe {
                contract.builder.build_gep(
                    argsdata,
                    &[ contract.context.i32_type().const_int(arglen, false).into()],
                    "abi_ptr")
            };
        }
    }

    fn abi_encode<'b>(
        &self,
        contract: &'b Contract,
        args: &[BasicValueEnum<'b>],
        spec: &resolver::FunctionDecl,
    ) -> (PointerValue<'b>, IntValue<'b>) {
        let mut length = 0;

        for arg in spec.returns.iter() {
            let ty = match arg.ty {
                resolver::TypeName::Elementary(e) => e,
                resolver::TypeName::Enum(n) => contract.ns.enums[n].ty,
                resolver::TypeName::Noreturn => unreachable!(),
            };

            match ty {
                ast::ElementaryTypeName::Bool => length += 1,
                ast::ElementaryTypeName::Uint(n) |
                ast::ElementaryTypeName::Int(n) => length += n as u64 / 8,
                _ => unimplemented!()
            }
        }

        let length = contract.context.i32_type().const_int(length, false);

        let data = contract.builder.build_call(
            contract.module.get_function("__malloc").unwrap(),
            &[ length.into() ],
            ""
        ).try_as_basic_value().left().unwrap().into_pointer_value();

        let mut argsdata = data;

        for (i, arg) in spec.returns.iter().enumerate() {
            let ty = match arg.ty {
                resolver::TypeName::Elementary(e) => e,
                resolver::TypeName::Enum(n) => contract.ns.enums[n].ty,
                resolver::TypeName::Noreturn => unreachable!(),
            };

            let arglen;

            match ty {
                ast::ElementaryTypeName::Bool => {
                    contract.builder.build_store(argsdata,
                        contract.builder.build_int_z_extend(args[i].into_int_value(), contract.context.i8_type(), "bool")
                    );
                    arglen = 1;
                },
                ast::ElementaryTypeName::Uint(n) |
                ast::ElementaryTypeName::Int(n) => {
                    contract.builder.build_store(
                        contract.builder.build_pointer_cast(argsdata,
                            args[i].into_int_value().get_type().ptr_type(AddressSpace::Generic),
                            ""),
                        args[i].into_int_value()
                    );
                    arglen = n as u64 / 8;
                }
                _ => unimplemented!()
            }

            argsdata = unsafe {
                contract.builder.build_gep(
                    argsdata,
                    &[ contract.context.i32_type().const_int(arglen, false).into()],
                    "abi_ptr")
            };
        }

        (data, length)
    }

    fn return_empty_abi(&self, contract: &Contract) {
        // This will clear the scratch buffer
        contract.builder.build_call(
            contract.module.get_function("ext_scratch_write").unwrap(),
            &[
                contract.context.i8_type().ptr_type(AddressSpace::Generic).const_zero().into(),
                contract.context.i32_type().const_zero().into(),
            ],
            ""
        );
            
        contract.builder.build_return(Some(&contract.context.i32_type().const_zero()));
    }

    fn return_abi<'b>(&self, contract: &'b Contract, data: PointerValue<'b>, length: IntValue) {
        contract.builder.build_call(
            contract.module.get_function("ext_scratch_write").unwrap(),
            &[
                data.into(),
                length.into(),
            ],
            ""
        );

        contract.builder.build_return(Some(&contract.context.i32_type().const_zero()));
    }
}
