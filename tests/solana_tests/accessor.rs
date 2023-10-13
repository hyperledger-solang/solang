// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use num_bigint::BigInt;
use num_traits::One;

#[test]
fn types() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            int64 public f1 = 102;
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm
        .function("f1")
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Int {
            width: 64,
            value: BigInt::from(102u8),
        }
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            int64[4] public f1 = [1,3,5,7];
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm
        .function("f1")
        .arguments(&[BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8),
        }])
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Int {
            width: 64,
            value: BigInt::from(5u8)
        }
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            int64[4][2] public f1;

            constructor() {
                f1[1][0] = 4;
                f1[1][1] = 3;
                f1[1][2] = 2;
                f1[1][3] = 1;
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm
        .function("f1")
        .arguments(&[
            BorshToken::Uint {
                width: 256,
                value: BigInt::one(),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(2u8),
            },
        ])
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Int {
            width: 64,
            value: BigInt::from(2u8),
        }
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            mapping(int64 => uint64) public f1;

            constructor() {
                f1[2000] = 1;
                f1[4000] = 2;
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm
        .function("f1")
        .arguments(&[BorshToken::Int {
            width: 64,
            value: BigInt::from(4000u16),
        }])
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 64,
            value: BigInt::from(2u8)
        }
    );
}

#[test]
fn interfaces() {
    let mut vm = build_solidity(
        r#"
        contract foo is bar {
            bytes2 public f1 = "ab";
        }

        interface bar {
            function f1() external returns (bytes2);
        }
        "#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm
        .function("f1")
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();

    assert_eq!(returns, BorshToken::uint8_fixed_array(b"ab".to_vec()));
}

#[test]
fn constant() {
    let mut vm = build_solidity(
        r#"
        contract x {
            bytes32 public constant z = keccak256("hey man");
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm.function("z").call().unwrap();

    assert_eq!(
        returns,
        BorshToken::uint8_fixed_array(vec![
            0, 91, 121, 69, 17, 39, 209, 87, 169, 94, 81, 10, 68, 17, 183, 52, 82, 28, 128, 159,
            31, 73, 168, 235, 90, 61, 46, 198, 102, 241, 168, 79
        ])
    );

    let mut vm = build_solidity(
        r#"
        contract x {
            bytes32 public constant z = sha256("hey man");
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm.function("z").call().unwrap();

    assert_eq!(
        returns,
        BorshToken::uint8_fixed_array(vec![
            190, 212, 99, 127, 110, 196, 102, 135, 47, 156, 116, 193, 201, 43, 100, 230, 152, 184,
            58, 103, 63, 106, 217, 142, 143, 211, 220, 125, 255, 210, 48, 89
        ])
    );

    let mut vm = build_solidity(
        r#"
        contract x {
            bytes20 public constant z = ripemd160("hey man");
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm.function("z").call().unwrap();

    assert_eq!(
        returns,
        BorshToken::uint8_fixed_array(vec![
            255, 206, 178, 91, 165, 156, 178, 193, 7, 94, 233, 48, 117, 76, 48, 215, 255, 45, 61,
            225
        ])
    );
}

#[test]
fn struct_accessor() {
    let mut vm = build_solidity(
        r#"
        import 'solana';
        contract C {
            struct E {
                bytes4 b4;
            }
            struct S {
                int64 f1;
                bool f2;
                E f3;
            }
            S public a = S({f1: -63, f2: false, f3: E("nuff")});
            S[100] public s;
            mapping(int => S) public m;
            E public constant e = E("cons");

            constructor() {
                s[99] = S({f1: 65535, f2: true, f3: E("naff")});
                m[1023413412] = S({f1: 414243, f2: true, f3: E("niff")});
            }

            @account(pid)
            function f() external view {
                AccountMeta[1] meta = [
                    AccountMeta({pubkey: tx.accounts.dataAccount.key, is_writable: false, is_signer: false})
                ];

                address pid = tx.accounts.pid.key;
                (int64 a1, bool b, E memory c) = this.a{accounts: meta, program_id: pid}();
                require(a1 == -63 && !b && c.b4 == "nuff", "a");
                (a1, b, c) = this.s{accounts: meta, program_id: pid}(99);
                require(a1 == 65535 && b && c.b4 == "naff", "b");
                (a1, b, c) = this.m{accounts: meta, program_id: pid}(1023413412);
                require(a1 == 414243 && b && c.b4 == "niff", "c");
                c.b4 = this.e{accounts: meta, program_id: pid}();
                require(a1 == 414243 && b && c.b4 == "cons", "E");
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let program_id = vm.stack[0].id;
    vm.function("f")
        .accounts(vec![
            ("dataAccount", data_account),
            ("systemProgram", [0; 32]),
            ("pid", program_id),
        ])
        .call();
}
