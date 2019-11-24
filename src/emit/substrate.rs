
use resolver;
use parser::ast;

use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::AddressSpace;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue, FunctionValue};
use inkwell::IntPredicate;

use super::{TargetRuntime, Contract};

pub struct SubstrateTarget {
}

impl SubstrateTarget {
    pub fn build<'a>(context: &'a Context, contract: &'a resolver::Contract, filename: &'a str) -> Contract<'a> {
        let mut c = Contract::new(context, contract, filename);
        let b = SubstrateTarget{};

        // externals
        b.declare_externals(&mut c);

        c.emit_functions(&b);
    
        b.emit_deploy(&c);

        c
    }

    fn declare_externals(&self, contract: &mut Contract) {
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
    }

    fn emit_deploy(&self, contract: &Contract) {
        // create deploy function
        let ret = contract.context.void_type();
        let ftype = ret.fn_type(&[contract.context.i32_type().ptr_type(AddressSpace::Generic).into()], false);
        let function = contract.module.add_function("deploy", ftype, None);

        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(&entry);

        // init our heap
        contract.builder.build_call(
            contract.module.get_function("__init_heap").unwrap(),
            &[],
            "");
        
        // copy arguments from scratch buffer
        let deploy_args_length = contract.builder.build_call(
            contract.module.get_function("ext_scratch_size").unwrap(),
            &[],
            "scratch_size").try_as_basic_value().left().unwrap();
        
        let deploy_args = contract.builder.build_call(
            contract.module.get_function("__malloc").unwrap(),
            &[deploy_args_length],
            ""
        ).try_as_basic_value().left().unwrap().into_pointer_value();

        contract.builder.build_call(
            contract.module.get_function("ext_scratch_read").unwrap(),
            &[
                deploy_args.into(),
                contract.context.i32_type().const_zero().into(),
                deploy_args_length.into(),
            ],
            ""
        );

        let fallback_block = contract.context.append_basic_block(function, "fallback");

        contract.emit_function_dispatch(&contract.ns.constructors, deploy_args, deploy_args_length.into_int_value(), function, &fallback_block, self);
        
        // emit fallback code
        contract.builder.position_at_end(&fallback_block);
        contract.builder.build_unreachable();
    }
}

impl TargetRuntime for SubstrateTarget {
    // TODO
    fn set_storage<'a>(&self, _contract: &'a Contract, _slot: u32, _dest: inkwell::values::PointerValue<'a>) {
    }

    // TODO
    fn get_storage<'a>(&self, _contract: &'a Contract, _slot: u32, _dest: inkwell::values::PointerValue<'a>) {
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

        let decode_block = contract.context.append_basic_block(function, "abi_decode");
        let wrong_length_block = contract.context.append_basic_block(function, "wrong_abi_length");

        let is_ok = contract.builder.build_int_compare(IntPredicate::EQ, datalength,
            contract.context.i32_type().const_int(length, false),
            "correct_length");

        contract.builder.build_conditional_branch(is_ok, &decode_block, &wrong_length_block);
    
        contract.builder.position_at_end(&decode_block);

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
                    // FIXME: check for not 0 or 1
                    args[i] = contract.builder.build_int_compare(IntPredicate::EQ,
                        contract.builder.build_load(argsdata, "abi_bool").into_int_value(),
                        contract.context.i64_type().const_zero(), "bool").into();
                    arglen = 1;
                },
                ast::ElementaryTypeName::Uint(n) |
                ast::ElementaryTypeName::Int(n) => {
                    args[i] = contract.builder.build_load(
                        contract.builder.build_pointer_cast(argsdata,
                            args[i].into_int_value().get_type().ptr_type(AddressSpace::Generic),
                            ""),
                        "");
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
                        contract.builder.build_int_cast(args[i].into_int_value(), contract.context.i8_type(), "bool")
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
            &[ data.into(), length.into() ],
            ""
        );
        contract.builder.build_return(Some(&contract.context.i32_type().const_zero()));
    }
}
