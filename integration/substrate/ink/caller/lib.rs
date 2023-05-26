// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod caller {
    use ink::env::{
        call::{build_call, Call, ExecutionInput, Selector},
        DefaultEnvironment,
    };

    #[ink(storage)]
    #[derive(Default)]
    pub struct Caller {}

    impl Caller {
        #[ink(constructor)]
        pub fn new() -> Self {
            Default::default()
        }

        /// Do a proxy call to `callee` and return its result.
        /// The message under `selector` should have exactly one `u32` arguemnt and return a `u32`.
        #[ink(message)]
        pub fn u32_proxy(
            &self,
            callee: AccountId,
            selector: [u8; 4],
            arg: u32,
            max_gas: Option<u64>,
            transfer_value: Option<u128>,
        ) -> u32 {
            build_call::<DefaultEnvironment>()
                .call_type(Call::new(callee).gas_limit(max_gas.unwrap_or_default()))
                .transferred_value(transfer_value.unwrap_or_default())
                .exec_input(ExecutionInput::new(Selector::new(selector)).push_arg(arg))
                .returns::<u32>() // FIXME: This should be Result<u32, u8> to respect LanguageError
                .invoke()
        }
    }
}
