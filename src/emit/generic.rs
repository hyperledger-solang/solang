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

pub struct GenericTarget {
    abi: ethabiencoder::EthAbiDecoder,
}

impl GenericTarget {
    pub fn build<'a>(
        context: &'a Context,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        filename: &'a str,
        opt: OptimizationLevel,
        math_overflow_check: bool,
    ) -> Binary<'a> {
        let mut b = GenericTarget {
            abi: ethabiencoder::EthAbiDecoder { bswap: false },
        };

        let mut binary = Binary::new(
            context,
            ns.target,
            &contract.name,
            filename,
            opt,
            math_overflow_check,
            None,
        );

        // externals
        b.declare_externals(&mut binary);

        b.emit_functions(&mut binary, contract, ns);

        b.emit_constructor(&mut binary, contract, ns);
        b.emit_function(&mut binary, contract, ns);

        binary
    }

    fn declare_externals(&self, binary: &mut Binary) {
        let void_ty = binary.context.void_type();
        let u8_ptr = binary.context.i8_type().ptr_type(AddressSpace::Generic);
        let u32_ty = binary.context.i32_type();

        binary.module.add_function(
            "solang_storage_delete",
            void_ty.fn_type(&[u8_ptr.into()], false),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "solang_storage_set",
            void_ty.fn_type(
                &[
                    u8_ptr.into(),
                    u8_ptr.into(),
                    binary.context.i32_type().into(),
                ],
                false,
            ),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "solang_storage_size",
            u32_ty.fn_type(&[u8_ptr.into()], false),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "solang_storage_get",
            void_ty.fn_type(&[u8_ptr.into(), u8_ptr.into()], false),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "solang_malloc",
            u8_ptr.fn_type(&[binary.context.i32_type().into()], false),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "solang_print",
            void_ty.fn_type(&[u8_ptr.into(), u32_ty.into()], false),
            Some(Linkage::External),
        );
        binary.module.add_function(
            "solang_set_return",
            void_ty.fn_type(&[u8_ptr.into(), u32_ty.into()], false),
            Some(Linkage::External),
        );
    }

    fn emit_constructor(
        &mut self,
        binary: &mut Binary,
        contract: &ast::Contract,
        ns: &ast::Namespace,
    ) {
        let initializer = self.emit_initializer(binary, contract, ns);

        let u8_ptr_ty = binary.context.i8_type().ptr_type(AddressSpace::Generic);
        let u32_ty = binary.context.i32_type();

        let ret = binary.context.i32_type();
        let ftype = ret.fn_type(&[u8_ptr_ty.into(), u32_ty.into()], false);
        let function = binary
            .module
            .add_function("solang_constructor", ftype, None);

        let entry = binary.context.append_basic_block(function, "entry");

        binary.builder.position_at_end(entry);

        // we should not use our heap; use sabre provided heap instead
        let argsdata = function.get_nth_param(0).unwrap().into_pointer_value();
        let argslen = function.get_nth_param(1).unwrap().into_int_value();

        // init our storage vars
        binary.builder.build_call(initializer, &[], "");

        if let Some((cfg_no, con)) = contract
            .functions
            .iter()
            .enumerate()
            .map(|(cfg_no, function_no)| (cfg_no, &ns.functions[*function_no]))
            .find(|(_, f)| f.is_constructor())
        {
            let mut args = Vec::new();

            // insert abi decode
            self.abi.decode(
                binary,
                function,
                &mut args,
                argsdata,
                argslen,
                &con.params,
                ns,
            );

            binary
                .builder
                .build_call(binary.functions[&cfg_no], &args, "");
        }

        // return 0 for success
        binary
            .builder
            .build_return(Some(&binary.context.i32_type().const_int(0, false)));
    }

    // emit function dispatch
    fn emit_function<'s>(
        &'s mut self,
        binary: &'s mut Binary,
        contract: &ast::Contract,
        ns: &ast::Namespace,
    ) {
        let u8_ptr_ty = binary.context.i8_type().ptr_type(AddressSpace::Generic);
        let u32_ty = binary.context.i32_type();

        let ret = binary.context.i32_type();
        let ftype = ret.fn_type(&[u8_ptr_ty.into(), u32_ty.into()], false);
        let function = binary.module.add_function("solang_function", ftype, None);

        let entry = binary.context.append_basic_block(function, "entry");

        binary.builder.position_at_end(entry);

        // we should not use our heap; use sabre provided heap instead
        let argsdata = function.get_nth_param(0).unwrap().into_pointer_value();
        let argslen = function.get_nth_param(1).unwrap().into_int_value();

        let argsdata = binary.builder.build_pointer_cast(
            argsdata,
            binary.context.i32_type().ptr_type(AddressSpace::Generic),
            "argsdata32",
        );

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
            |_| false,
        );
    }
}

impl<'a> TargetRuntime<'a> for GenericTarget {
    fn storage_delete_single_slot(
        &self,
        binary: &Binary,
        _function: FunctionValue,
        slot: PointerValue,
    ) {
        binary.builder.build_call(
            binary.module.get_function("solang_storage_delete").unwrap(),
            &[slot.into()],
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
        // TODO: check for non-zero
        binary.builder.build_call(
            binary.module.get_function("solang_storage_set").unwrap(),
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
                dest.get_type()
                    .get_element_type()
                    .into_int_type()
                    .size_of()
                    .const_cast(binary.context.i32_type(), false)
                    .into(),
            ],
            "",
        );
    }

    fn set_storage_string(
        &self,
        binary: &Binary<'a>,
        _function: FunctionValue<'a>,
        slot: PointerValue<'a>,
        dest: BasicValueEnum<'a>,
    ) {
        // TODO: check for non-zero
        binary.builder.build_call(
            binary.module.get_function("solang_storage_set").unwrap(),
            &[
                binary
                    .builder
                    .build_pointer_cast(
                        slot,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into(),
                binary.vector_bytes(dest).into(),
                binary.vector_len(dest).into(),
            ],
            "",
        );
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
        _ns: &ast::Namespace,
    ) -> BasicValueEnum<'a> {
        unimplemented!();
    }
    fn storage_pop(
        &self,
        _binary: &Binary<'a>,
        _function: FunctionValue,
        _ty: &ast::Type,
        _slot: IntValue<'a>,
        _ns: &ast::Namespace,
    ) -> BasicValueEnum<'a> {
        unimplemented!();
    }
    fn storage_array_length(
        &self,
        binary: &Binary<'a>,
        _function: FunctionValue,
        slot: IntValue<'a>,
        _ty: &ast::Type,
        _ns: &ast::Namespace,
    ) -> IntValue<'a> {
        let slot_ptr = binary.builder.build_alloca(slot.get_type(), "slot");
        binary.builder.build_store(slot_ptr, slot);

        binary
            .builder
            .build_call(
                binary.module.get_function("solang_storage_size").unwrap(),
                &[binary
                    .builder
                    .build_pointer_cast(
                        slot_ptr,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value()
    }

    fn get_storage_int(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ty: IntType<'a>,
    ) -> IntValue<'a> {
        let exists = binary
            .builder
            .build_call(
                binary.module.get_function("solang_storage_size").unwrap(),
                &[binary
                    .builder
                    .build_pointer_cast(
                        slot,
                        binary.context.i8_type().ptr_type(AddressSpace::Generic),
                        "",
                    )
                    .into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        let data_size = ty.size_of().const_cast(binary.context.i32_type(), false);

        let exists = binary.builder.build_int_compare(
            IntPredicate::EQ,
            exists.into_int_value(),
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

        let dest = binary.builder.build_alloca(ty, "int");

        binary.builder.build_call(
            binary.module.get_function("solang_storage_get").unwrap(),
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

        let loaded_int = binary.builder.build_load(dest, "int");

        binary.builder.build_unconditional_branch(done_storage);

        binary.builder.position_at_end(done_storage);

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
        _ns: &ast::Namespace,
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
        // return 0 for success
        binary
            .builder
            .build_return(Some(&binary.context.i32_type().const_int(0, false)));
    }

    fn return_abi<'b>(&self, binary: &'b Binary, data: PointerValue<'b>, length: IntValue) {
        binary.builder.build_call(
            binary.module.get_function("solang_set_return").unwrap(),
            &[data.into(), length.into()],
            "",
        );
        binary
            .builder
            .build_return(Some(&binary.context.i32_type().const_int(0, false)));
    }

    fn assert_failure<'b>(&self, binary: &'b Binary, data: PointerValue, length: IntValue) {
        binary.builder.build_call(
            binary.module.get_function("solang_set_return").unwrap(),
            &[data.into(), length.into()],
            "",
        );
        binary
            .builder
            .build_return(Some(&binary.context.i32_type().const_int(2, false)));
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

        let encoder = ethabiencoder::EncoderBuilder::new(
            binary, function, load, args, &packed, &tys, false, ns,
        );

        let length = encoder.encoded_length();

        let encoded_data = binary
            .builder
            .build_call(
                binary.module.get_function("solang_malloc").unwrap(),
                &[length.into()],
                "",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        encoder.finish(binary, function, encoded_data, ns);

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
        ns: &ast::Namespace,
    ) {
        self.abi
            .decode(binary, function, args, data, length, spec, ns);
    }

    fn print(&self, binary: &Binary, string_ptr: PointerValue, string_len: IntValue) {
        binary.builder.build_call(
            binary.module.get_function("solang_print").unwrap(),
            &[string_ptr.into(), string_len.into()],
            "",
        );
    }

    /// Create new contract
    fn create_contract<'b>(
        &mut self,
        _binary: &Binary<'b>,
        _function: FunctionValue,
        _success: Option<&mut BasicValueEnum<'b>>,
        _contract_no: usize,
        _constructor_no: Option<usize>,
        _address: PointerValue<'b>,
        _args: &[BasicValueEnum],
        _gas: IntValue<'b>,
        _value: Option<IntValue<'b>>,
        _salt: Option<IntValue<'b>>,
        _space: Option<IntValue<'b>>,
        _ns: &ast::Namespace,
    ) {
        panic!("generic cannot create new contracts");
    }

    /// Call external contract
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
        _ns: &ast::Namespace,
    ) {
        panic!("generic cannot call other contracts");
    }

    /// Get return buffer for external call
    fn return_data<'b>(&self, _binary: &Binary<'b>, _function: FunctionValue) -> PointerValue<'b> {
        panic!("generic cannot call other contracts");
    }

    fn return_code<'b>(&self, binary: &'b Binary, ret: IntValue<'b>) {
        binary.builder.build_return(Some(&ret));
    }

    /// Sabre does not know about balances
    fn value_transferred<'b>(&self, binary: &Binary<'b>, ns: &ast::Namespace) -> IntValue<'b> {
        binary.value_type(ns).const_zero()
    }

    /// Terminate execution, destroy contract and send remaining funds to addr
    fn selfdestruct<'b>(&self, _binary: &Binary<'b>, _addr: IntValue<'b>, _ns: &ast::Namespace) {
        panic!("generic does not have the concept of selfdestruct");
    }

    /// Emit event
    fn emit_event<'b>(
        &self,
        _binary: &Binary<'b>,
        _contract: &ast::Contract,
        _function: FunctionValue<'b>,
        _event_no: usize,
        _data: &[BasicValueEnum<'b>],
        _data_tys: &[ast::Type],
        _topics: &[BasicValueEnum<'b>],
        _topic_tys: &[ast::Type],
        _ns: &ast::Namespace,
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
        _ns: &ast::Namespace,
    ) -> BasicValueEnum<'b> {
        unimplemented!();
    }

    /// Crypto Hash
    fn hash<'b>(
        &self,
        _binary: &Binary<'b>,
        _function: FunctionValue<'b>,

        _hash: HashTy,
        _input: PointerValue<'b>,
        _input_len: IntValue<'b>,
        _ns: &ast::Namespace,
    ) -> IntValue<'b> {
        unimplemented!()
    }
}
