use crate::codegen::cfg::HashTy;
use crate::parser::pt;
use crate::sema::ast;
use std::collections::HashMap;
use std::str;

use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::types::IntType;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;

use super::ethabiencoder;
use super::{Contract, TargetRuntime, Variable};

pub struct SabreTarget {
    abi: ethabiencoder::EthAbiDecoder,
}

impl SabreTarget {
    pub fn build<'a>(
        context: &'a Context,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        filename: &'a str,
        opt: OptimizationLevel,
        math_overflow_check: bool,
    ) -> Contract<'a> {
        let mut b = SabreTarget {
            abi: ethabiencoder::EthAbiDecoder { bswap: false },
        };
        let mut c = Contract::new(
            context,
            contract,
            ns,
            filename,
            opt,
            math_overflow_check,
            None,
        );

        // externals
        b.declare_externals(&mut c);

        b.emit_functions(&mut c);

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
            "log_buffer",
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
        contract.module.add_function(
            "log_buffer",
            contract.context.void_type().fn_type(
                &[
                    contract.context.i32_type().into(),
                    u8_ptr.into(),
                    contract.context.i32_type().into(),
                ],
                false,
            ),
            Some(Linkage::External),
        );
    }

    fn emit_entrypoint(&mut self, contract: &mut Contract) {
        let initializer = self.emit_initializer(contract);

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

        if let Some((cfg_no, con)) = contract
            .contract
            .functions
            .iter()
            .enumerate()
            .map(|(cfg_no, function_no)| (cfg_no, &contract.ns.functions[*function_no]))
            .find(|(_, f)| f.is_constructor())
        {
            let mut args = Vec::new();

            // insert abi decode
            self.abi.decode(
                contract,
                function,
                &mut args,
                argsdata,
                argslen,
                &con.params,
            );

            contract
                .builder
                .build_call(contract.functions[&cfg_no], &args, "");
        }

        // return 1 for success
        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_int(1, false)));

        contract.builder.position_at_end(function_block);

        self.emit_function_dispatch(
            contract,
            pt::FunctionTy::Function,
            argsdata,
            argslen,
            function,
            None,
            |_| false,
        );
    }
}

impl<'a> TargetRuntime<'a> for SabreTarget {
    fn storage_delete_single_slot(
        &self,
        contract: &Contract,
        _function: FunctionValue,
        slot: PointerValue,
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

    fn set_storage(
        &self,
        contract: &Contract,
        _function: FunctionValue,
        slot: PointerValue,
        dest: PointerValue,
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

    fn set_storage_extfunc(
        &self,
        _contract: &Contract,
        _function: FunctionValue,
        _slot: PointerValue,
        _dest: PointerValue,
    ) {
        unimplemented!();
    }
    fn get_storage_extfunc(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        unimplemented!();
    }

    fn set_storage_string(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue<'a>,
        _slot: PointerValue<'a>,
        _dest: BasicValueEnum<'a>,
    ) {
        unimplemented!();
    }

    fn get_storage_string(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        unimplemented!();
    }
    fn get_storage_bytes_subscript(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: IntValue<'a>,
        _index: IntValue<'a>,
    ) -> IntValue<'a> {
        unimplemented!();
    }
    fn set_storage_bytes_subscript(
        &self,
        _contract: &Contract,
        _function: FunctionValue,
        _slot: IntValue,
        _index: IntValue,
        _val: IntValue,
    ) {
        unimplemented!();
    }
    fn storage_push(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _ty: &ast::Type,
        _slot: IntValue<'a>,
        _val: BasicValueEnum<'a>,
    ) -> BasicValueEnum<'a> {
        unimplemented!();
    }
    fn storage_pop(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _ty: &ast::Type,
        _slot: IntValue<'a>,
    ) -> BasicValueEnum<'a> {
        unimplemented!();
    }

    fn get_storage_int(
        &self,
        contract: &Contract<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ty: IntType<'a>,
    ) -> IntValue<'a> {
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

        let data_size = ty.size_of();

        let exists = contract.builder.build_int_compare(
            IntPredicate::EQ,
            state_size,
            data_size,
            "storage_exists",
        );

        let entry = contract.builder.get_insert_block().unwrap();

        let retrieve_block = contract.context.append_basic_block(function, "in_storage");
        let done_storage = contract
            .context
            .append_basic_block(function, "done_storage");

        contract
            .builder
            .build_conditional_branch(exists, retrieve_block, done_storage);

        contract.builder.position_at_end(retrieve_block);

        let loaded_int = contract.builder.build_load(
            contract
                .builder
                .build_pointer_cast(res, ty.ptr_type(AddressSpace::Generic), ""),
            "loaded_int",
        );

        contract.builder.build_unconditional_branch(done_storage);

        let res = contract.builder.build_phi(ty, "storage_res");

        res.add_incoming(&[(&loaded_int, retrieve_block), (&ty.const_zero(), entry)]);

        res.as_basic_value().into_int_value()
    }

    /// sabre has no keccak256 host function, so call our implementation
    fn keccak256_hash(
        &self,
        contract: &Contract,
        src: PointerValue,
        length: IntValue,
        dest: PointerValue,
    ) {
        contract.builder.build_call(
            contract.module.get_function("keccak256").unwrap(),
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
            ],
            "",
        );
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

    fn assert_failure<'b>(&self, contract: &'b Contract, _data: PointerValue, _length: IntValue) {
        contract.builder.build_unreachable();
    }

    /// ABI encode into a vector for abi.encode* style builtin functions
    fn abi_encode_to_vector<'b>(
        &self,
        _contract: &Contract<'b>,
        _function: FunctionValue<'b>,
        _packed: &[BasicValueEnum<'b>],
        _args: &[BasicValueEnum<'b>],
        _spec: &[ast::Type],
    ) -> PointerValue<'b> {
        unimplemented!();
    }

    fn abi_encode<'b>(
        &self,
        contract: &Contract<'b>,
        selector: Option<IntValue<'b>>,
        load: bool,
        function: FunctionValue<'b>,
        args: &[BasicValueEnum<'b>],
        tys: &[ast::Type],
    ) -> (PointerValue<'b>, IntValue<'b>) {
        let mut tys = tys.to_vec();

        let packed = if let Some(selector) = selector {
            tys.insert(0, ast::Type::Uint(32));
            vec![selector.into()]
        } else {
            vec![]
        };

        let encoder = ethabiencoder::EncoderBuilder::new(
            contract, function, load, args, &packed, &tys, false,
        );

        let length = encoder.encoded_length();

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

        encoder.finish(contract, function, encoded_data);

        (encoded_data, length)
    }

    fn abi_decode<'b>(
        &self,
        contract: &Contract<'b>,
        function: FunctionValue<'b>,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue<'b>,
        spec: &[ast::Parameter],
    ) {
        self.abi
            .decode(contract, function, args, data, length, spec);
    }

    fn print(&self, contract: &Contract, string_ptr: PointerValue, string_len: IntValue) {
        contract.builder.build_call(
            contract.module.get_function("log_buffer").unwrap(),
            &[
                contract.context.i32_type().const_int(2, false).into(),
                string_ptr.into(),
                string_len.into(),
            ],
            "",
        );
    }

    /// Create new contract
    fn create_contract<'b>(
        &mut self,
        _contract: &Contract<'b>,
        _function: FunctionValue,
        _success: Option<&mut BasicValueEnum<'b>>,
        _contract_no: usize,
        _constructor_no: Option<usize>,
        _address: PointerValue<'b>,
        _args: &[BasicValueEnum],
        _gas: IntValue<'b>,
        _value: Option<IntValue<'b>>,
        _salt: Option<IntValue<'b>>,
    ) {
        panic!("Sabre cannot create new contracts");
    }

    /// Call external contract
    fn external_call<'b>(
        &self,
        _contract: &Contract<'b>,
        _function: FunctionValue,
        _success: Option<&mut BasicValueEnum<'b>>,
        _payload: PointerValue<'b>,
        _payload_len: IntValue<'b>,
        _address: PointerValue<'b>,
        _gas: IntValue<'b>,
        _value: IntValue<'b>,
        _ty: ast::CallTy,
    ) {
        panic!("Sabre cannot call other contracts");
    }

    /// Get return buffer for external call
    fn return_data<'b>(&self, _contract: &Contract<'b>) -> PointerValue<'b> {
        panic!("Sabre cannot call other contracts");
    }

    fn return_code<'b>(&self, contract: &'b Contract, ret: IntValue<'b>) {
        contract.builder.build_return(Some(&ret));
    }

    /// Sabre does not know about balances
    fn value_transferred<'b>(&self, contract: &Contract<'b>) -> IntValue<'b> {
        contract.value_type().const_zero()
    }

    /// Terminate execution, destroy contract and send remaining funds to addr
    fn selfdestruct<'b>(&self, _contract: &Contract<'b>, _addr: IntValue<'b>) {
        panic!("Sabre does not have the concept of selfdestruct");
    }

    /// Send event
    fn send_event<'b>(
        &self,
        _contract: &Contract<'b>,
        _event_no: usize,
        _data: PointerValue<'b>,
        _data_len: IntValue<'b>,
        _topics: Vec<(PointerValue<'b>, IntValue<'b>)>,
    ) {
        unimplemented!();
    }

    /// builtin expressions
    fn builtin<'b>(
        &self,
        _contract: &Contract<'b>,
        _expr: &ast::Expression,
        _vartab: &HashMap<usize, Variable<'b>>,
        _function: FunctionValue<'b>,
    ) -> BasicValueEnum<'b> {
        unimplemented!();
    }

    /// Crypto Hash
    fn hash<'b>(
        &self,
        _contract: &Contract<'b>,
        _hash: HashTy,
        _input: PointerValue<'b>,
        _input_len: IntValue<'b>,
    ) -> IntValue<'b> {
        unimplemented!()
    }
}
