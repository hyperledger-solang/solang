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
use super::{Binary, TargetRuntime, Variable};

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
    ) -> Binary<'a> {
        let mut b = SabreTarget {
            abi: ethabiencoder::EthAbiDecoder { bswap: false },
        };
        let mut c = Binary::new(
            context,
            contract,
            ns,
            &contract.name,
            filename,
            opt,
            math_overflow_check,
            None,
        );

        // externals
        b.declare_externals(&mut c);

        b.emit_functions(&mut c, contract);

        b.emit_entrypoint(&mut c, contract);

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

    fn declare_externals(&self, binary: &mut Binary) {
        let u8_ptr = binary.context.i8_type().ptr_type(AddressSpace::Generic);
        binary.module.add_function(
            "get_ptr_len",
            binary.context.i32_type().fn_type(&[u8_ptr.into()], false),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "delete_state",
            u8_ptr.fn_type(&[u8_ptr.into()], false),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "set_state",
            u8_ptr.fn_type(&[u8_ptr.into()], false),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "get_state",
            u8_ptr.fn_type(&[u8_ptr.into()], false),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "create_collection",
            u8_ptr.fn_type(&[u8_ptr.into()], false),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "add_to_collection",
            u8_ptr.fn_type(&[u8_ptr.into(), u8_ptr.into()], false),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "alloc",
            u8_ptr.fn_type(&[binary.context.i32_type().into()], false),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "log_buffer",
            binary.context.void_type().fn_type(
                &[
                    binary.context.i32_type().into(),
                    u8_ptr.into(),
                    binary.context.i32_type().into(),
                ],
                false,
            ),
            Some(Linkage::External),
        );
    }

    fn emit_entrypoint(&mut self, binary: &mut Binary, contract: &ast::Contract) {
        let initializer = self.emit_initializer(binary, contract);

        let bytes_ptr = binary.context.i32_type().ptr_type(AddressSpace::Generic);

        // create start function
        let ret = binary.context.i32_type();
        let ftype = ret.fn_type(
            &[bytes_ptr.into(), bytes_ptr.into(), bytes_ptr.into()],
            false,
        );
        let function = binary.module.add_function("entrypoint", ftype, None);

        let entry = binary.context.append_basic_block(function, "entry");

        binary.builder.position_at_end(entry);

        // we should not use our heap; use sabre provided heap instead
        let argsdata = function.get_first_param().unwrap().into_pointer_value();
        let argslen = binary
            .builder
            .build_call(
                binary.module.get_function("get_ptr_len").unwrap(),
                &[binary
                    .builder
                    .build_pointer_cast(
                        argsdata,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
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
        let is_function_call = binary.builder.build_int_compare(
            IntPredicate::EQ,
            binary
                .builder
                .build_and(argslen, binary.context.i32_type().const_int(31, false), ""),
            binary.context.i32_type().const_int(4, false),
            "is_function_call",
        );

        let function_block = binary.context.append_basic_block(function, "function_call");
        let constructor_block = binary
            .context
            .append_basic_block(function, "constructor_call");

        binary.builder.build_conditional_branch(
            is_function_call,
            function_block,
            constructor_block,
        );

        binary.builder.position_at_end(constructor_block);

        // init our storage vars
        binary.builder.build_call(initializer, &[], "");

        if let Some((cfg_no, con)) = contract
            .functions
            .iter()
            .enumerate()
            .map(|(cfg_no, function_no)| (cfg_no, &binary.ns.functions[*function_no]))
            .find(|(_, f)| f.is_constructor())
        {
            let mut args = Vec::new();

            // insert abi decode
            self.abi
                .decode(binary, function, &mut args, argsdata, argslen, &con.params);

            binary
                .builder
                .build_call(binary.functions[&cfg_no], &args, "");
        }

        // return 1 for success
        binary
            .builder
            .build_return(Some(&binary.context.i32_type().const_int(1, false)));

        binary.builder.position_at_end(function_block);

        self.emit_function_dispatch(
            binary,
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
        binary: &Binary,
        _function: FunctionValue,
        slot: PointerValue,
    ) {
        let address = binary
            .builder
            .build_call(
                binary.module.get_function("alloc").unwrap(),
                &[binary.context.i32_type().const_int(64, false).into()],
                "address",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // convert slot to address
        binary.builder.build_call(
            binary.module.get_function("__u256ptohex").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        slot,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "slot",
                    )
                    .into(),
                address.into(),
            ],
            "address_from_slot",
        );

        // create collection for delete_state
        binary.builder.build_call(
            binary.module.get_function("create_collection").unwrap(),
            &[address.into()],
            "",
        );

        binary.builder.build_call(
            binary.module.get_function("delete_state").unwrap(),
            &[address.into()],
            "",
        );
    }

    fn set_storage(
        &self,
        binary: &Binary,
        _function: FunctionValue,
        slot: PointerValue,
        dest: PointerValue,
    ) {
        let address = binary
            .builder
            .build_call(
                binary.module.get_function("alloc").unwrap(),
                &[binary.context.i32_type().const_int(64, false).into()],
                "address",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // convert slot to address
        binary.builder.build_call(
            binary.module.get_function("__u256ptohex").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        slot,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
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
            .const_cast(binary.context.i32_type(), false);

        let data = binary
            .builder
            .build_call(
                binary.module.get_function("alloc").unwrap(),
                &[data_size.into()],
                "data",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // store data in pointer collection
        let dest = binary.builder.build_pointer_cast(
            dest,
            binary.context.i8_type().ptr_type(AddressSpace::Generic),
            "dest",
        );

        binary.builder.build_call(
            binary.module.get_function("__memcpy").unwrap(),
            &[data.into(), dest.into(), data_size.into()],
            "destdata",
        );

        // create collection for set_state
        binary.builder.build_call(
            binary.module.get_function("create_collection").unwrap(),
            &[address.into()],
            "",
        );
        binary.builder.build_call(
            binary.module.get_function("add_to_collection").unwrap(),
            &[address.into(), data.into()],
            "",
        );
        binary.builder.build_call(
            binary.module.get_function("set_state").unwrap(),
            &[address.into()],
            "",
        );
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
    ) -> PointerValue<'a> {
        unimplemented!();
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
        _slot: PointerValue<'a>,
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
        _val: BasicValueEnum<'a>,
    ) -> BasicValueEnum<'a> {
        unimplemented!();
    }
    fn storage_pop(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue,
        _ty: &ast::Type,
        _slot: IntValue<'a>,
    ) -> BasicValueEnum<'a> {
        unimplemented!();
    }

    fn get_storage_int(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ty: IntType<'a>,
    ) -> IntValue<'a> {
        let address = binary
            .builder
            .build_call(
                binary.module.get_function("alloc").unwrap(),
                &[binary.context.i32_type().const_int(64, false).into()],
                "address",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // convert slot to address
        binary.builder.build_call(
            binary.module.get_function("__u256ptohex").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        slot,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "slot",
                    )
                    .into(),
                address.into(),
            ],
            "address_from_slot",
        );

        // create collection for set_state
        binary.builder.build_call(
            binary.module.get_function("create_collection").unwrap(),
            &[address.into()],
            "",
        );
        let res = binary
            .builder
            .build_call(
                binary.module.get_function("get_state").unwrap(),
                &[address.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let state_size = binary
            .builder
            .build_call(
                binary.module.get_function("get_ptr_len").unwrap(),
                &[res.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let data_size = ty.size_of();

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            state_size,
            data_size,
            "storage_exists",
        );

        let entry = binary.builder.get_insert_block().unwrap();

        let retrieve_block = binary.context.append_basic_block(function, "in_storage");
        let done_storage = binary.context.append_basic_block(function, "done_storage");

        binary
            .builder
            .build_conditional_branch(exists, retrieve_block, done_storage);

        binary.builder.position_at_end(retrieve_block);

        let loaded_int = binary.builder.build_load(
            binary
                .builder
                .build_pointer_cast(res, ty.ptr_type(AddressSpace::Generic), ""),
            "loaded_int",
        );

        binary.builder.build_unconditional_branch(done_storage);

        let res = binary.builder.build_phi(ty, "storage_res");

        res.add_incoming(&[(&loaded_int, retrieve_block), (&ty.const_zero(), entry)]);

        res.as_basic_value().into_int_value()
    }

    /// sabre has no keccak256 host function, so call our implementation
    fn keccak256_hash(
        &self,
        binary: &Binary,
        src: PointerValue,
        length: IntValue,
        dest: PointerValue,
    ) {
        binary.builder.build_call(
            binary.module.get_function("keccak256").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        src,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "src",
                    )
                    .into(),
                length.into(),
                binary
                    .builder
                    .build_pointer_cast(
                        dest,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "dest",
                    )
                    .into(),
            ],
            "",
        );
    }

    fn return_empty_abi(&self, binary: &Binary) {
        // return 1 for success
        binary
            .builder
            .build_return(Some(&binary.context.i32_type().const_int(1, false)));
    }

    fn return_abi<'b>(&self, binary: &'b Binary, _data: PointerValue<'b>, _length: IntValue) {
        // FIXME: how to return abi encoded return data?
        // return 1 for success
        binary
            .builder
            .build_return(Some(&binary.context.i32_type().const_int(1, false)));
    }

    fn assert_failure<'b>(&self, binary: &'b Binary, _data: PointerValue, _length: IntValue) {
        binary.builder.build_unreachable();
    }

    /// ABI encode into a vector for abi.encode* style builtin functions
    fn abi_encode_to_vector<'b>(
        &self,
        _binary: &Binary<'b>,
        _function: FunctionValue<'b>,
        _packed: &[BasicValueEnum<'b>],
        _args: &[BasicValueEnum<'b>],
        _spec: &[ast::Type],
    ) -> PointerValue<'b> {
        unimplemented!();
    }

    fn abi_encode<'b>(
        &self,
        binary: &Binary<'b>,
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

        let encoder =
            ethabiencoder::EncoderBuilder::new(binary, function, load, args, &packed, &tys, false);

        let length = encoder.encoded_length();

        let encoded_data = binary
            .builder
            .build_call(
                binary.module.get_function("alloc").unwrap(),
                &[length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        encoder.finish(binary, function, encoded_data);

        (encoded_data, length)
    }

    fn abi_decode<'b>(
        &self,
        binary: &Binary<'b>,
        function: FunctionValue<'b>,
        args: &mut Vec<BasicValueEnum<'b>>,
        data: PointerValue<'b>,
        length: IntValue<'b>,
        spec: &[ast::Parameter],
    ) {
        self.abi.decode(binary, function, args, data, length, spec);
    }

    fn print(&self, binary: &Binary, string_ptr: PointerValue, string_len: IntValue) {
        binary.builder.build_call(
            binary.module.get_function("log_buffer").unwrap(),
            &[
                binary.context.i32_type().const_int(2, false).into(),
                string_ptr.into(),
                string_len.into(),
            ],
            "",
        );
    }

    /// Create new binary
    fn create_contract<'b>(
        &mut self,
        _binary: &Binary<'b>,
        _function: FunctionValue,
        _success: Option<&mut BasicValueEnum<'b>>,
        _binary_no: usize,
        _constructor_no: Option<usize>,
        _address: PointerValue<'b>,
        _args: &[BasicValueEnum],
        _gas: IntValue<'b>,
        _value: Option<IntValue<'b>>,
        _salt: Option<IntValue<'b>>,
    ) {
        panic!("Sabre cannot create new binarys");
    }

    /// Call external binary
    fn external_call<'b>(
        &self,
        _binary: &Binary<'b>,
        _function: FunctionValue,
        _success: Option<&mut BasicValueEnum<'b>>,
        _payload: PointerValue<'b>,
        _payload_len: IntValue<'b>,
        _address: Option<PointerValue<'b>>,
        _gas: IntValue<'b>,
        _value: IntValue<'b>,
        _ty: ast::CallTy,
    ) {
        panic!("Sabre cannot call other binarys");
    }

    /// Get return buffer for external call
    fn return_data<'b>(&self, _binary: &Binary<'b>) -> PointerValue<'b> {
        panic!("Sabre cannot call other binarys");
    }

    fn return_code<'b>(&self, binary: &'b Binary, ret: IntValue<'b>) {
        binary.builder.build_return(Some(&ret));
    }

    /// Sabre does not know about balances
    fn value_transferred<'b>(&self, binary: &Binary<'b>) -> IntValue<'b> {
        binary.value_type().const_zero()
    }

    /// Terminate execution, destroy binary and send remaining funds to addr
    fn selfdestruct<'b>(&self, _binary: &Binary<'b>, _addr: IntValue<'b>) {
        panic!("Sabre does not have the concept of selfdestruct");
    }

    /// Send event
    fn send_event<'b>(
        &self,
        _binary: &Binary<'b>,
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
        _binary: &Binary<'b>,
        _expr: &ast::Expression,
        _vartab: &HashMap<usize, Variable<'b>>,
        _function: FunctionValue<'b>,
    ) -> BasicValueEnum<'b> {
        unimplemented!();
    }

    /// Crypto Hash
    fn hash<'b>(
        &self,
        _binary: &Binary<'b>,
        _hash: HashTy,
        _input: PointerValue<'b>,
        _input_len: IntValue<'b>,
    ) -> IntValue<'b> {
        unimplemented!()
    }
}
