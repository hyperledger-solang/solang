// SPDX-License-Identifier: Apache-2.0

use crate::borsh_encoding::BorshToken;
use crate::{account_new, build_solidity};
use base58::ToBase58;

#[test]
fn print_addresses() {
    let mut vm = build_solidity(
        r#"
contract Base58 {
    function print_this(address addr) pure public {
        print("{}".format(addr));
    }

    function print_as_hex(address addr) pure public {
        print("{:x}".format(addr));
    }
}
        "#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    for _ in 0..10 {
        let account = account_new();
        let _ = vm
            .function("print_this")
            .arguments(&[BorshToken::Address(account)])
            .call();
        let mut base_58 = account.to_base58();
        while base_58.len() < 44 {
            // Rust's to_base58() ignores leading zeros in the byte array,
            // so it won't transform them into ones. On the other hand, Solana addresses
            // can start with leading ones: 11128aXFh5abEooZ2ouNDjPjk2TqDaHjG6JkX74vK4q is
            // a valid address.
            base_58.insert(0, '1');
        }
        assert_eq!(vm.logs, base_58);
        vm.logs.clear();
        let _ = vm
            .function("print_as_hex")
            .arguments(&[BorshToken::Address(account)])
            .call();
        let decoded = hex::decode(vm.logs.as_str()).unwrap();
        assert_eq!(account, decoded.as_ref());
        vm.logs.clear();
    }
}
