use crate::codegen::cfg::HashTy;
use crate::parser::pt;
use crate::sema::ast;
use std::collections::HashMap;
use std::str;

use inkwell::context::Context;
use inkwell::types::{BasicType, IntType};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue, UnnamedAddress};
use inkwell::{AddressSpace, IntPredicate, OptimizationLevel};
use num_traits::ToPrimitive;
use tiny_keccak::{Hasher, Keccak};

use super::ethabiencoder;
use super::{Contract, TargetRuntime, Variable};

pub struct SolanaTarget {
    abi: ethabiencoder::EthAbiEncoder,
    magic: u32,
}

// Implement the Solana target which uses BPF
impl SolanaTarget {
    pub fn build<'a>(
        context: &'a Context,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        filename: &'a str,
        opt: OptimizationLevel,
        math_overflow_check: bool,
    ) -> Contract<'a> {
        // We need a magic number for our contract. This is used to check if the contract storage
        // account is initialized for the correct contract
        let mut hasher = Keccak::v256();
        let mut hash = [0u8; 32];
        hasher.update(contract.name.as_bytes());
        hasher.finalize(&mut hash);
        let mut magic = [0u8; 4];

        magic.copy_from_slice(&hash[0..4]);

        let mut target = SolanaTarget {
            abi: ethabiencoder::EthAbiEncoder { bswap: true },
            magic: u32::from_le_bytes(magic),
        };

        let mut con = Contract::new(
            context,
            contract,
            ns,
            filename,
            opt,
            math_overflow_check,
            None,
        );

        // externals
        target.declare_externals(&mut con);

        target.emit_functions(&mut con);

        target.emit_dispatch(&mut con);

        con.internalize(&["entrypoint", "sol_log_", "sol_alloc_free_"]);

        con
    }

    fn declare_externals(&self, contract: &mut Contract) {
        let void_ty = contract.context.void_type();
        let u8_ptr = contract.context.i8_type().ptr_type(AddressSpace::Generic);
        let u64_ty = contract.context.i64_type();

        let function = contract.module.add_function(
            "sol_alloc_free_",
            u8_ptr.fn_type(&[u8_ptr.into(), u64_ty.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);

        let function = contract.module.add_function(
            "sol_log_",
            void_ty.fn_type(&[u8_ptr.into(), u64_ty.into()], false),
            None,
        );
        function
            .as_global_value()
            .set_unnamed_address(UnnamedAddress::Local);
    }

    fn emit_dispatch(&mut self, contract: &mut Contract) {
        let initializer = self.emit_initializer(contract);

        let function = contract.module.get_function("solang_dispatch").unwrap();

        let entry = contract.context.append_basic_block(function, "entry");

        contract.builder.position_at_end(entry);

        let input = function.get_nth_param(0).unwrap().into_pointer_value();
        let input_len = function.get_nth_param(1).unwrap().into_int_value();
        let accounts = function.get_nth_param(2).unwrap().into_pointer_value();

        // load magic value of contract storage
        let contract_data = contract
            .builder
            .build_load(
                unsafe {
                    contract.builder.build_gep(
                        accounts,
                        &[
                            contract.context.i32_type().const_int(1, false),
                            contract.context.i32_type().const_int(3, false),
                        ],
                        "contract_data",
                    )
                },
                "contract_data",
            )
            .into_pointer_value();

        let magic_value_ptr = contract.builder.build_pointer_cast(
            contract_data,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "magic_value_ptr",
        );

        let magic_value = contract
            .builder
            .build_load(magic_value_ptr, "magic")
            .into_int_value();

        let function_block = contract
            .context
            .append_basic_block(function, "function_call");
        let constructor_block = contract
            .context
            .append_basic_block(function, "constructor_call");
        let badmagic_block = contract.context.append_basic_block(function, "bad_magic");

        // if the magic is zero it's a virgin contract
        // if the magic is our magic value, it's a function call
        // if the magic is another magic value, it is an error
        contract.builder.build_switch(
            magic_value,
            badmagic_block,
            &[
                (contract.context.i32_type().const_zero(), constructor_block),
                (
                    contract
                        .context
                        .i32_type()
                        .const_int(self.magic as u64, false),
                    function_block,
                ),
            ],
        );

        contract.builder.position_at_end(badmagic_block);

        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_int(7, false)));

        contract.accounts = Some(accounts);

        // generate constructor code
        contract.builder.position_at_end(constructor_block);

        // write our magic value to the contract
        contract.builder.build_store(
            magic_value_ptr,
            contract
                .context
                .i32_type()
                .const_int(self.magic as u64, false),
        );

        // write heap_offset.
        let heap_offset_ptr = unsafe {
            contract.builder.build_gep(
                magic_value_ptr,
                &[contract.context.i64_type().const_int(1, false)],
                "heap_offset",
            )
        };

        contract.builder.build_store(
            heap_offset_ptr,
            contract
                .context
                .i32_type()
                .const_int(contract.contract.fixed_layout_size.to_u64().unwrap(), false),
        );

        contract
            .builder
            .build_call(initializer, &[accounts.into()], "");

        // There is only one possible constructor
        let ret = if let Some((cfg_no, cfg)) = contract
            .contract
            .cfg
            .iter()
            .enumerate()
            .find(|(_, cfg)| cfg.ty == pt::FunctionTy::Constructor)
        {
            let mut args = Vec::new();

            // insert abi decode
            self.abi
                .decode(contract, function, &mut args, input, input_len, &cfg.params);

            args.push(accounts.into());

            contract
                .builder
                .build_call(contract.functions[&cfg_no], &args, "")
                .try_as_basic_value()
                .left()
                .unwrap()
        } else {
            // return 0 for success
            contract.context.i32_type().const_int(0, false).into()
        };

        contract.builder.build_return(Some(&ret));

        // Generate function call dispatch
        contract.builder.position_at_end(function_block);

        contract.accounts = Some(accounts);

        let input = contract.builder.build_pointer_cast(
            input,
            contract.context.i32_type().ptr_type(AddressSpace::Generic),
            "input_ptr32",
        );

        self.emit_function_dispatch(
            contract,
            pt::FunctionTy::Function,
            input,
            input_len,
            function,
            None,
            |_| false,
        );
    }

    // Returns the pointer to the length of the return buffer, and the buffer itself
    fn return_buffer<'b>(&self, contract: &Contract<'b>) -> (PointerValue<'b>, PointerValue<'b>) {
        // the first account passed in is the return buffer; 3 field of account is "data"
        let data = contract
            .builder
            .build_load(
                unsafe {
                    contract.builder.build_gep(
                        contract.accounts.unwrap(),
                        &[
                            contract.context.i32_type().const_zero(),
                            contract.context.i32_type().const_int(3, false),
                        ],
                        "data",
                    )
                },
                "data",
            )
            .into_pointer_value();

        // First we have the 64 bit length field
        let data_len_ptr = contract.builder.build_pointer_cast(
            data,
            contract.context.i64_type().ptr_type(AddressSpace::Generic),
            "data_len_ptr",
        );

        // step over that field, and cast to u8* for the buffer itself
        let data_ptr = contract.builder.build_pointer_cast(
            unsafe {
                contract.builder.build_gep(
                    data_len_ptr,
                    &[contract.context.i32_type().const_int(1, false)],
                    "data_ptr",
                )
            },
            contract.context.i8_type().ptr_type(AddressSpace::Generic),
            "data_ptr",
        );

        (data_len_ptr, data_ptr)
    }
}

impl<'a> TargetRuntime<'a> for SolanaTarget {
    fn clear_storage(&self, _contract: &Contract, _function: FunctionValue, _slot: PointerValue) {
        unimplemented!();
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
        _slot: PointerValue<'a>,
        _index: IntValue<'a>,
    ) -> IntValue<'a> {
        unimplemented!();
    }
    fn set_storage_bytes_subscript(
        &self,
        _contract: &Contract,
        _function: FunctionValue,
        _slot: PointerValue,
        _index: IntValue,
        _val: IntValue,
    ) {
        unimplemented!();
    }
    fn storage_bytes_push(
        &self,
        _contract: &Contract,
        _function: FunctionValue,
        _slot: PointerValue,
        _val: IntValue,
    ) {
        unimplemented!();
    }
    fn storage_bytes_pop(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> IntValue<'a> {
        unimplemented!();
    }

    fn storage_string_length(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
    ) -> IntValue<'a> {
        unimplemented!();
    }
    fn get_storage_int(
        &self,
        _contract: &Contract<'a>,
        _function: FunctionValue,
        _slot: PointerValue<'a>,
        _ty: IntType<'a>,
    ) -> IntValue<'a> {
        unimplemented!();
    }

    /// Recursively load a type from contract storage. This overrides the default method
    /// in the trait, which is for chains with 256 bit storage keys.
    fn storage_load(
        &self,
        contract: &Contract<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        _function: FunctionValue,
    ) -> BasicValueEnum<'a> {
        // contract storage is in 2nd account
        let account = unsafe {
            contract.builder.build_gep(
                contract.accounts.unwrap(),
                &[contract.context.i32_type().const_int(1, false)],
                "account",
            )
        };

        // 3rd member of account is data pointer
        let data = unsafe {
            contract.builder.build_gep(
                account,
                &[
                    contract.context.i32_type().const_zero(),
                    contract.context.i32_type().const_int(3, false),
                ],
                "data",
            )
        };

        let data = contract
            .builder
            .build_load(data, "data")
            .into_pointer_value();

        // the slot is simply the offset after the magic
        let member = unsafe { contract.builder.build_gep(data, &[*slot], "data") };

        if *ty == ast::Type::String {
            let offset = contract
                .builder
                .build_load(
                    contract.builder.build_pointer_cast(
                        member,
                        contract.context.i32_type().ptr_type(AddressSpace::Generic),
                        "",
                    ),
                    "offset",
                )
                .into_int_value();

            let string_length = contract
                .builder
                .build_call(
                    contract.module.get_function("account_data_len").unwrap(),
                    &[account.into(), offset.into()],
                    "free",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            let string_data = unsafe { contract.builder.build_gep(data, &[offset], "string_data") };

            contract
                .builder
                .build_call(
                    contract.module.get_function("vector_new").unwrap(),
                    &[
                        string_length.into(),
                        contract.context.i32_type().const_int(1, false).into(),
                        string_data.into(),
                    ],
                    "",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
        } else {
            contract.builder.build_load(
                contract.builder.build_pointer_cast(
                    member,
                    contract.llvm_type(ty).ptr_type(AddressSpace::Generic),
                    "",
                ),
                "",
            )
        }
    }

    fn storage_store(
        &self,
        contract: &Contract<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        val: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
    ) {
        // contract storage is in 2nd account
        let account = unsafe {
            contract.builder.build_gep(
                contract.accounts.unwrap(),
                &[contract.context.i32_type().const_int(1, false)],
                "account",
            )
        };

        // 3rd member of account is data pointer
        let data = unsafe {
            contract.builder.build_gep(
                account,
                &[
                    contract.context.i32_type().const_zero(),
                    contract.context.i32_type().const_int(3, false),
                ],
                "data",
            )
        };

        let data = contract
            .builder
            .build_load(data, "data")
            .into_pointer_value();

        // the slot is simply the offset after the magic
        let member = unsafe { contract.builder.build_gep(data, &[*slot], "data") };

        if *ty == ast::Type::String {
            let offset_ptr = contract.builder.build_pointer_cast(
                member,
                contract.context.i32_type().ptr_type(AddressSpace::Generic),
                "offset_ptr",
            );

            let offset = contract
                .builder
                .build_load(offset_ptr, "offset")
                .into_int_value();

            let existing_string_length = contract
                .builder
                .build_call(
                    contract.module.get_function("account_data_len").unwrap(),
                    &[account.into(), offset.into()],
                    "length",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            let new_string_length = contract.vector_len(val);

            let allocation_necessary = contract.builder.build_int_compare(
                IntPredicate::NE,
                existing_string_length,
                new_string_length,
                "allocation_necessary",
            );

            let entry = contract.builder.get_insert_block().unwrap();

            let realloc = contract.context.append_basic_block(function, "realloc");
            let memcpy = contract.context.append_basic_block(function, "memcpy");

            contract
                .builder
                .build_conditional_branch(allocation_necessary, realloc, memcpy);

            contract.builder.position_at_end(realloc);

            // do not realloc since we're copying everything
            contract.builder.build_call(
                contract.module.get_function("account_data_free").unwrap(),
                &[account.into(), offset.into()],
                "free",
            );

            // account_data_alloc will return 0 if the string is length 0
            let new_offset = contract
                .builder
                .build_call(
                    contract.module.get_function("account_data_alloc").unwrap(),
                    &[account.into(), new_string_length.into()],
                    "alloc",
                )
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();

            contract.builder.build_store(offset_ptr, new_offset);

            contract.builder.build_unconditional_branch(memcpy);

            contract.builder.position_at_end(memcpy);

            let offset_phi = contract
                .builder
                .build_phi(contract.context.i32_type(), "offset");

            offset_phi.add_incoming(&[(&new_offset, realloc), (&offset, entry)]);

            let dest_string_data = unsafe {
                contract.builder.build_gep(
                    data,
                    &[offset_phi.as_basic_value().into_int_value()],
                    "dest_string_data",
                )
            };

            contract.builder.build_call(
                contract.module.get_function("__memcpy").unwrap(),
                &[
                    dest_string_data.into(),
                    contract.vector_bytes(val).into(),
                    new_string_length.into(),
                ],
                "copied",
            );
        } else {
            contract.builder.build_store(
                contract.builder.build_pointer_cast(
                    member,
                    val.get_type().ptr_type(AddressSpace::Generic),
                    "",
                ),
                val,
            );
        }
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
        let (data_len_ptr, _) = self.return_buffer(contract);

        contract
            .builder
            .build_store(data_len_ptr, contract.context.i64_type().const_zero());

        // return 0 for success
        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_int(0, false)));
    }

    fn return_abi<'b>(&self, contract: &'b Contract, _data: PointerValue<'b>, _length: IntValue) {
        // return data already filled in output contract

        // return 0 for success
        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_int(0, false)));
    }

    fn assert_failure<'b>(&self, contract: &'b Contract, _data: PointerValue, _length: IntValue) {
        // the reason code should be null (and already printed)

        // return 1 for failure
        contract
            .builder
            .build_return(Some(&contract.context.i32_type().const_int(1, false)));
    }

    /// ABI encode into a vector for abi.encode* style builtin functions
    fn abi_encode_to_vector<'b>(
        &self,
        _contract: &Contract<'b>,
        _selector: Option<IntValue<'b>>,
        _function: FunctionValue<'b>,
        _packed: bool,
        _args: &[BasicValueEnum<'b>],
        _spec: &[ast::Type],
    ) -> PointerValue<'b> {
        unimplemented!();
    }

    fn abi_encode(
        &self,
        contract: &Contract<'a>,
        selector: Option<IntValue<'a>>,
        load: bool,
        function: FunctionValue<'a>,
        args: &[BasicValueEnum<'a>],
        tys: &[ast::Type],
    ) -> (PointerValue<'a>, IntValue<'a>) {
        let (output_len, mut output) = self.return_buffer(contract);

        let (length, mut offset) = ethabiencoder::EthAbiEncoder::total_encoded_length(
            contract, selector, load, function, args, tys,
        );

        let length64 =
            contract
                .builder
                .build_int_z_extend(length, contract.context.i64_type(), "length64");

        // FIXME ensure we have enough space for our return data
        contract.builder.build_store(output_len, length64);

        if let Some(selector) = selector {
            contract.builder.build_store(
                contract.builder.build_pointer_cast(
                    output,
                    contract.context.i32_type().ptr_type(AddressSpace::Generic),
                    "",
                ),
                selector,
            );

            output = unsafe {
                contract.builder.build_gep(
                    output,
                    &[contract
                        .context
                        .i32_type()
                        .const_int(std::mem::size_of::<u32>() as u64, false)],
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
                output.into(),
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

        let mut dynamic = unsafe { contract.builder.build_gep(output, &[offset], "") };

        for (i, ty) in tys.iter().enumerate() {
            self.abi.encode_ty(
                contract,
                load,
                function,
                ty,
                args[i],
                &mut output,
                &mut offset,
                &mut dynamic,
            );
        }

        (output, length)
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
        let string_len64 =
            contract
                .builder
                .build_int_z_extend(string_len, contract.context.i64_type(), "");

        contract.builder.build_call(
            contract.module.get_function("sol_log_").unwrap(),
            &[string_ptr.into(), string_len64.into()],
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
        unimplemented!();
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
        unimplemented!();
    }

    /// Get return buffer for external call
    fn return_data<'b>(&self, _contract: &Contract<'b>) -> PointerValue<'b> {
        unimplemented!();
    }

    fn return_u32<'b>(&self, contract: &'b Contract, ret: IntValue<'b>) {
        contract.builder.build_return(Some(&ret));
    }

    /// Value received
    fn value_transferred<'b>(&self, contract: &Contract<'b>) -> IntValue<'b> {
        contract.value_type().const_zero()
    }

    /// Terminate execution, destroy contract and send remaining funds to addr
    fn selfdestruct<'b>(&self, _contract: &Contract<'b>, _addr: IntValue<'b>) {
        unimplemented!();
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
