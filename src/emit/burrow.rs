
use resolver;

use inkwell::types::BasicTypeEnum;
use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::AddressSpace;

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
        c.emit_function_dispatch(contract);

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
        let func = contract.module.add_function("get_storage32", ftype, Some(Linkage::External));
        contract.externals.insert("get_storage32".to_owned(), func);

        let func = contract.module.add_function("set_storage32", ftype, Some(Linkage::External));
        contract.externals.insert("set_storage32".to_owned(), func);
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
            contract.emit_abi_decode(
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
}

impl TargetRuntime for BurrowTarget {
    fn set_storage<'a>(&self, contract: &'a Contract, slot: u32, dest: inkwell::values::PointerValue<'a>) {
        contract.builder.build_call(
            contract.externals["set_storage32"],
            &[
                contract.context.i32_type().const_int(slot as u64, false).into(),
                contract.builder.build_pointer_cast(dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic), "").into(),
                dest.get_type().size_of().const_cast(
                    contract.context.i32_type(), false).into()
            ],
            "");
    }

    fn get_storage<'a>(&self, contract: &'a Contract, slot: u32, dest: inkwell::values::PointerValue<'a>) {
        contract.builder.build_call(
            contract.externals["get_storage32"],
            &[
                contract.context.i32_type().const_int(slot as u64, false).into(),
                contract.builder.build_pointer_cast(dest,
                    contract.context.i8_type().ptr_type(AddressSpace::Generic), "").into(),
                dest.get_type().size_of().const_cast(
                    contract.context.i32_type(), false).into()
            ],
            "");
    }
}
