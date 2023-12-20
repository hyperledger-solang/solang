// SPDX-License-Identifier: Apache-2.0

use crate::{
    build_solidity, create_program_address, AccountState, BorshToken, Instruction, Pubkey,
    VirtualMachine,
};
use base58::FromBase58;
use num_bigint::BigInt;
use num_traits::One;

#[test]
fn simple_external_call() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_bar(string v) public {
                print(string.concat("bar0", " ", "says: ", v, ""));
            }

            @account(pid)
            function test_other() external {
                bar1.test_bar{program_id: tx.accounts.pid.key}("cross contract call");
            }
        }

        contract bar1 {
            function test_bar(string v) public {
                print(string.concat("bar1 says: ", v));
            }
        }"#,
    );

    let bar1_account = vm.initialize_data_account();
    let bar1_program_id = vm.stack[0].id;
    vm.function("new")
        .accounts(vec![("dataAccount", bar1_account)])
        .call();

    vm.function("test_bar")
        .arguments(&[BorshToken::String(String::from("yo"))])
        .call();

    assert_eq!(vm.logs, "bar1 says: yo");

    vm.logs.truncate(0);

    vm.set_program(0);

    let bar0_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", bar0_account)])
        .call();

    vm.function("test_bar")
        .arguments(&[BorshToken::String(String::from("uncle beau"))])
        .call();

    assert_eq!(vm.logs, "bar0 says: uncle beau");

    vm.logs.truncate(0);

    vm.function("test_other")
        .accounts(vec![("pid", bar1_program_id), ("systemProgram", [0; 32])])
        .call();

    assert_eq!(vm.logs, "bar1 says: cross contract call");
}

#[test]
fn external_call_with_returns() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            @account(pid)
            function test_other() external returns (int64) {
                return bar1.test_bar{program_id: tx.accounts.pid.key}(7) + 5;
            }
        }

        contract bar1 {
            function test_bar(int64 y) public returns (int64) {
                return 3 + y;
            }
        }"#,
    );

    let bar1_account = vm.initialize_data_account();
    let bar1_program_id = vm.stack[0].id;
    vm.function("new")
        .accounts(vec![("dataAccount", bar1_account)])
        .call();

    let res = vm
        .function("test_bar")
        .arguments(&[BorshToken::Int {
            width: 64,
            value: BigInt::from(21),
        }])
        .call()
        .unwrap();

    assert_eq!(
        res,
        BorshToken::Int {
            width: 64,
            value: BigInt::from(24u8)
        }
    );

    vm.set_program(0);

    let bar0_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", bar0_account)])
        .call();

    let res = vm
        .function("test_other")
        .accounts(vec![("pid", bar1_program_id), ("systemProgram", [0; 32])])
        .call()
        .unwrap();

    assert_eq!(
        res,
        BorshToken::Int {
            width: 64,
            value: BigInt::from(15u8)
        }
    );
}

#[test]
fn external_raw_call_with_returns() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            bytes8 private constant SELECTOR = bytes8(sha256(bytes('global:test_bar')));

            @account(bar1_pid)
            function test_other() external returns (int64) {
                bytes select = abi.encodeWithSelector(SELECTOR, int64(7));
                bytes signature = abi.encodeWithSignature("global:test_bar", int64(7));
                require(select == signature, "must be the same");
                (, bytes raw) = tx.accounts.bar1_pid.key.call{accounts: []}(signature);
                (int64 v) = abi.decode(raw, (int64));
                return v + 5;
            }
        }

        contract bar1 {
            function test_bar(int64 y) public returns (int64) {
                return 3 + y;
            }
        }"#,
    );

    let bar1_account = vm.initialize_data_account();
    let bar1_program_id = vm.stack[0].id;
    vm.function("new")
        .accounts(vec![("dataAccount", bar1_account)])
        .call();

    let res = vm
        .function("test_bar")
        .arguments(&[BorshToken::Int {
            width: 64,
            value: BigInt::from(21u8),
        }])
        .call()
        .unwrap();

    assert_eq!(
        res,
        BorshToken::Int {
            width: 64,
            value: BigInt::from(24u8),
        }
    );

    vm.set_program(0);

    let bar0_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", bar0_account)])
        .call();

    let res = vm
        .function("test_other")
        .accounts(vec![
            ("bar1_pid", bar1_program_id),
            ("systemProgram", [0; 32]),
        ])
        .call()
        .unwrap();

    assert_eq!(
        res,
        BorshToken::Int {
            width: 64,
            value: BigInt::from(15u8),
        }
    );
}

#[test]
fn call_external_func_type() {
    let mut vm = build_solidity(
        r#"
    contract testing {

    function testPtr(int a) public pure returns (int, int) {
        return (a/2, 3);
    }

    function doTest() public view returns (int, int) {
    function(int) external pure returns (int, int) sfPtr = this.testPtr;

       (int a, int b) = sfPtr{accounts: []}(2);
       return (a, b);
    }
}
    "#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let res = vm
        .function("doTest")
        .accounts(vec![("systemProgram", [0; 32])])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        res,
        vec![
            BorshToken::Int {
                width: 256,
                value: BigInt::one(),
            },
            BorshToken::Int {
                width: 256,
                value: BigInt::from(3u8)
            }
        ]
    );
}

#[test]
fn external_call_with_string_returns() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            @account(pid)
            function test_other() external returns (string) {
                string y = bar1.test_bar{program_id: tx.accounts.pid.key}(7);
                print(y);
                return y;
            }

            @account(pid)
            function test_this() external {
                address a = bar1.who_am_i{program_id: tx.accounts.pid.key}();
                assert(a == tx.accounts.pid.key);
            }
        }

        contract bar1 {
            function test_bar(int64 y) public returns (string) {
                return "foo:{}".format(y);
            }

            function who_am_i() public returns (address) {
                return address(this);
            }
        }"#,
    );

    let bar1_account = vm.initialize_data_account();
    let bar1_program_id = vm.stack[0].id;
    vm.function("new")
        .accounts(vec![("dataAccount", bar1_account)])
        .call();

    let res = vm
        .function("test_bar")
        .arguments(&[BorshToken::Int {
            width: 64,
            value: BigInt::from(22u8),
        }])
        .call()
        .unwrap();

    assert_eq!(res, BorshToken::String(String::from("foo:22")));

    vm.set_program(0);

    let bar0_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", bar0_account)])
        .call();

    let res = vm
        .function("test_other")
        .accounts(vec![("pid", bar1_program_id), ("systemProgram", [0; 32])])
        .call()
        .unwrap();

    assert_eq!(res, BorshToken::String(String::from("foo:7")));

    vm.function("test_this")
        .accounts(vec![("pid", bar1_program_id), ("systemProgram", [0; 32])])
        .call();
}

#[test]
fn encode_call() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            bytes8 private constant SELECTOR = bytes8(sha256(bytes('global:test_bar')));
            bytes8 private constant SELECTOR2 = bytes8(sha256(bytes('global:test_baz')));

            @account(bar1_pid)
            function test_other() external returns (int64) {
                bytes select = abi.encodeWithSelector(SELECTOR, int64(7));
                bytes signature = abi.encodeCall(bar1.test_bar, 7);
                require(select == signature, "must be the same");
                (, bytes raw) = tx.accounts.bar1_pid.key.call{accounts: []}(signature);
                (int64 v) = abi.decode(raw, (int64));
                return v + 5;
            }

            @account(bar1_pid)
            function test_other2() external returns (int64) {
                bytes select = abi.encodeWithSelector(SELECTOR2, int64(7), int64(5));
                bytes signature = abi.encodeCall(bar1.test_baz, (7, 5));
                require(select == signature, "must be the same");
                (, bytes raw) = tx.accounts.bar1_pid.key.call{accounts: []}(signature);
                (int64 v) = abi.decode(raw, (int64));
                return v + 5;
            }
        }

        contract bar1 {
            function test_bar(int64 y) public pure returns (int64) {
                return 3 + y;
            }
            function test_baz(int64 y, int64 x) public pure returns (int64) {
                return 3 + y + x;
            }
        }"#,
    );

    let bar1_account = vm.initialize_data_account();
    let bar1_program_id = vm.stack[0].id;
    vm.function("new")
        .accounts(vec![("dataAccount", bar1_account)])
        .call();

    let res = vm
        .function("test_bar")
        .arguments(&[BorshToken::Int {
            width: 64,
            value: BigInt::from(21u8),
        }])
        .call()
        .unwrap();

    assert_eq!(
        res,
        BorshToken::Int {
            width: 64,
            value: BigInt::from(24u8)
        }
    );

    vm.set_program(0);

    let bar0_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", bar0_account)])
        .call();

    let res = vm
        .function("test_other")
        .accounts(vec![
            ("bar1_pid", bar1_program_id),
            ("systemProgram", [0; 32]),
        ])
        .call()
        .unwrap();

    assert_eq!(
        res,
        BorshToken::Int {
            width: 64,
            value: BigInt::from(15u8)
        }
    );

    let res = vm
        .function("test_other2")
        .accounts(vec![
            ("bar1_pid", bar1_program_id),
            ("systemProgram", [0; 32]),
        ])
        .call()
        .unwrap();

    assert_eq!(
        res,
        BorshToken::Int {
            width: 64,
            value: BigInt::from(20u8)
        }
    );
}

#[test]
fn internal_function_storage() {
    let mut vm = build_solidity(
        r#"
        contract ft {
            function(int32,int32) internal returns (int32) func;

            function mul(int32 a, int32 b) internal returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal returns (int32) {
                return a + b;
            }

            function set_op(bool action) public {
                if (action) {
                    func = mul;
                } else {
                    func = add;
                }
            }

            function test(int32 a, int32 b) public returns (int32) {
                return func(a, b);
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let res = vm
        .function("set_op")
        .arguments(&[BorshToken::Bool(true)])
        .accounts(vec![("dataAccount", data_account)])
        .call();

    assert!(res.is_none());

    let res = vm
        .function("test")
        .arguments(&[
            BorshToken::Int {
                width: 32,
                value: BigInt::from(3u8),
            },
            BorshToken::Int {
                width: 32,
                value: BigInt::from(5u8),
            },
        ])
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();

    assert_eq!(
        res,
        BorshToken::Int {
            width: 32,
            value: BigInt::from(15u8)
        }
    );

    let res = vm
        .function("set_op")
        .arguments(&[BorshToken::Bool(false)])
        .accounts(vec![("dataAccount", data_account)])
        .call();

    assert!(res.is_none());

    let res = vm
        .function("test")
        .arguments(&[
            BorshToken::Int {
                width: 32,
                value: BigInt::from(3u8),
            },
            BorshToken::Int {
                width: 32,
                value: BigInt::from(5u8),
            },
        ])
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();

    assert_eq!(
        res,
        BorshToken::Int {
            width: 32,
            value: BigInt::from(8u8)
        }
    );
}

#[test]
fn raw_call_accounts() {
    let mut vm = build_solidity(
        r#"
        import {AccountMeta} from 'solana';

        contract SplToken {
            address constant tokenProgramId = address"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
            address constant SYSVAR_RENT_PUBKEY = address"SysvarRent111111111111111111111111111111111";

            struct InitializeMintInstruction {
                uint8 instruction;
                uint8 decimals;
                address mintAuthority;
                uint8 freezeAuthorityOption;
                address freezeAuthority;
            }

            function create_mint_with_freezeauthority(uint8 decimals, address mintAuthority, address freezeAuthority) public {
                InitializeMintInstruction instr = InitializeMintInstruction({
                    instruction: 0,
                    decimals: decimals,
                    mintAuthority: mintAuthority,
                    freezeAuthorityOption: 1,
                    freezeAuthority: freezeAuthority
                });

                AccountMeta[2] metas = [
                    AccountMeta({pubkey: instr.mintAuthority, is_writable: true, is_signer: false}),
                    AccountMeta({pubkey: SYSVAR_RENT_PUBKEY, is_writable: false, is_signer: false})
                ];

                tokenProgramId.call{accounts: metas}(instr);
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let token = Pubkey(
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            .from_base58()
            .unwrap()
            .try_into()
            .unwrap(),
    );
    vm.account_data.insert(token.0, AccountState::default());

    let test_args = |_vm: &VirtualMachine, instr: &Instruction, _signers: &[Pubkey]| {
        let sysvar_rent = Pubkey(
            "SysvarRent111111111111111111111111111111111"
                .from_base58()
                .unwrap()
                .try_into()
                .unwrap(),
        );

        assert_eq!(
            &instr.data,
            &[
                0, 11, 113, 117, 105, 110, 113, 117, 97, 103, 105, 110, 116, 97, 113, 117, 97, 100,
                114, 105, 110, 103, 101, 110, 116, 105, 108, 108, 105, 97, 114, 100, 116, 104, 1,
                113, 117, 105, 110, 113, 117, 97, 103, 105, 110, 116, 97, 113, 117, 97, 100, 114,
                105, 110, 103, 101, 110, 116, 105, 108, 108, 105, 111, 110, 116, 104, 115,
            ]
        );

        assert!(instr.accounts[0].is_writable);
        assert!(!instr.accounts[0].is_signer);
        assert_eq!(
            instr.accounts[0].pubkey,
            Pubkey([
                113, 117, 105, 110, 113, 117, 97, 103, 105, 110, 116, 97, 113, 117, 97, 100, 114,
                105, 110, 103, 101, 110, 116, 105, 108, 108, 105, 97, 114, 100, 116, 104
            ])
        );

        assert!(!instr.accounts[1].is_writable);
        assert!(!instr.accounts[1].is_signer);
        assert_eq!(instr.accounts[1].pubkey, sysvar_rent);
    };

    vm.call_params_check.insert(token.clone(), test_args);

    vm.function("create_mint_with_freezeauthority")
        .arguments(&[
            BorshToken::Uint {
                width: 8,
                value: BigInt::from(11u8),
            },
            BorshToken::Address(b"quinquagintaquadringentilliardth".to_owned()),
            BorshToken::Address(b"quinquagintaquadringentillionths".to_owned()),
        ])
        .accounts(vec![("tokenProgram", token.0), ("systemProgram", [0; 32])])
        .call();
}

#[test]
fn pda() {
    let mut vm = build_solidity(
        r#"
        import {AccountMeta} from 'solana';

        contract pda {
            address constant tokenProgramId = address"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
            address constant SYSVAR_RENT_PUBKEY = address"SysvarRent111111111111111111111111111111111";

            function test() public {
                bytes instr = new bytes(1);

                AccountMeta[1] metas = [
                    AccountMeta({pubkey: SYSVAR_RENT_PUBKEY, is_writable: false, is_signer: false})
                ];

                bytes3 foo = "foo";
                address addr = address"8dtukUTHTZoVQTA5i4UdC2z6A2b5yvnJhkzhYnwAk3Fm";

                tokenProgramId.call{seeds: [ [ foo ] , ["b", "a", "r"], [addr], [foo, addr, "meh"] ], accounts: metas}(instr);
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let test_args = |vm: &VirtualMachine, _instr: &Instruction, signers: &[Pubkey]| {
        assert_eq!(
            signers[0],
            create_program_address(&vm.stack[0].id, &[b"foo"])
        );
        assert_eq!(
            signers[1],
            create_program_address(&vm.stack[0].id, &[b"bar"])
        );
        assert_eq!(
            signers[2],
            create_program_address(&vm.stack[0].id, &[b"quinquagintaquadringentilliardth"])
        );
        assert_eq!(
            signers[3],
            create_program_address(
                &vm.stack[0].id,
                &[b"fooquinquagintaquadringentilliardthmeh"]
            )
        );
    };

    let token = Pubkey(
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            .from_base58()
            .unwrap()
            .try_into()
            .unwrap(),
    );

    vm.account_data.insert(token.0, AccountState::default());
    vm.call_params_check.insert(token.clone(), test_args);

    vm.function("test")
        .accounts(vec![("tokenProgram", token.0), ("systemProgram", [0; 32])])
        .call();
}

#[test]
fn pda_array() {
    // now more dynamic
    let mut vm = build_solidity(
        r#"
            import {AccountMeta} from 'solana';

            contract pda {
                address constant tokenProgramId = address"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
                address constant SYSVAR_RENT_PUBKEY = address"SysvarRent111111111111111111111111111111111";

                function test(bytes[] dyn, address[] addr, bytes5[] b5) public {
                    bytes instr = new bytes(1);

                    instr[0] = 0x95;

                    AccountMeta[1] metas = [
                        AccountMeta({pubkey: SYSVAR_RENT_PUBKEY, is_writable: false, is_signer: false})
                    ];

                    bytes3 foo = "foo";

                    tokenProgramId.call{seeds: [ dyn, addr, b5 ], accounts: metas}(instr);
                }
            }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let test_args = |vm: &VirtualMachine, _instr: &Instruction, signers: &[Pubkey]| {
        assert_eq!(
            signers[0],
            create_program_address(&vm.stack[0].id, &[b"foobar"])
        );
        assert_eq!(
            signers[1],
            create_program_address(
                &vm.stack[0].id,
                &[
                    b"quinquagintaquadringentilliardth",
                    b"quinquagintaquadringentillionths"
                ]
            )
        );
        assert_eq!(
            signers[2],
            create_program_address(&vm.stack[0].id, &[b"tares", b"enoki"])
        );
    };

    let token = Pubkey(
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            .from_base58()
            .unwrap()
            .try_into()
            .unwrap(),
    );

    vm.account_data.insert(token.0, AccountState::default());
    vm.call_params_check.insert(token.clone(), test_args);

    vm.function("test")
        .accounts(vec![("tokenProgram", token.0), ("systemProgram", [0; 32])])
        .arguments(&[
            BorshToken::Array(vec![
                BorshToken::Bytes(b"foo".to_vec()),
                BorshToken::Bytes(b"bar".to_vec()),
            ]),
            BorshToken::Array(vec![
                BorshToken::Address(*b"quinquagintaquadringentilliardth"),
                BorshToken::Address(*b"quinquagintaquadringentillionths"),
            ]),
            BorshToken::Array(vec![
                BorshToken::FixedBytes(b"tares".to_vec()),
                BorshToken::FixedBytes(b"enoki".to_vec()),
            ]),
        ])
        .call();
}

#[test]
fn pda_array_of_array() {
    // now more dynamic
    let mut vm = build_solidity(
        r#"
            import {AccountMeta} from 'solana';

            contract pda {
                address constant tokenProgramId = address"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
                address constant SYSVAR_RENT_PUBKEY = address"SysvarRent111111111111111111111111111111111";

                function test_bytes(bytes[][] seeds) public {
                    bytes instr = new bytes(1);

                    instr[0] = 0x95;

                    AccountMeta[1] metas = [
                        AccountMeta({pubkey: SYSVAR_RENT_PUBKEY, is_writable: false, is_signer: false})
                    ];

                    tokenProgramId.call{seeds: seeds, accounts: metas}(instr);
                }

                function test_string(string[][] seeds) public {
                    bytes instr = new bytes(1);

                    instr[0] = 0x95;

                    AccountMeta[1] metas = [
                        AccountMeta({pubkey: SYSVAR_RENT_PUBKEY, is_writable: false, is_signer: false})
                    ];

                    tokenProgramId.call{seeds: seeds, accounts: metas}(instr);
                }

                function test_bytes4(bytes4[][] seeds) public {
                    bytes instr = new bytes(1);

                    instr[0] = 0x95;

                    AccountMeta[1] metas = [
                        AccountMeta({pubkey: SYSVAR_RENT_PUBKEY, is_writable: false, is_signer: false})
                    ];

                    tokenProgramId.call{seeds: seeds, accounts: metas}(instr);
                }

                function test_addr(address[][] seeds) public {
                    bytes instr = new bytes(1);

                    instr[0] = 0x95;

                    AccountMeta[1] metas = [
                        AccountMeta({pubkey: SYSVAR_RENT_PUBKEY, is_writable: false, is_signer: false})
                    ];

                    tokenProgramId.call{seeds: seeds, accounts: metas}(instr);
                }
            }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let token = Pubkey(
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            .from_base58()
            .unwrap()
            .try_into()
            .unwrap(),
    );

    vm.account_data.insert(token.0, AccountState::default());

    let test_args = |vm: &VirtualMachine, _instr: &Instruction, signers: &[Pubkey]| {
        assert_eq!(signers.len(), 2);

        assert_eq!(
            signers[0],
            create_program_address(&vm.stack[0].id, &[b"foobar"])
        );
        assert_eq!(
            signers[1],
            create_program_address(
                &vm.stack[0].id,
                &[b"zemmiphobia", b"extemporaneousness", b"automysophobia"]
            )
        );
    };

    vm.call_params_check.insert(token.clone(), test_args);

    vm.function("test_bytes")
        .accounts(vec![("tokenProgram", token.0), ("systemProgram", [0; 32])])
        .arguments(&[BorshToken::Array(vec![
            BorshToken::Array(vec![
                BorshToken::Bytes(b"foo".to_vec()),
                BorshToken::Bytes(b"bar".to_vec()),
            ]),
            BorshToken::Array(vec![
                BorshToken::Bytes(b"zemmiphobia".to_vec()),
                BorshToken::Bytes(b"extemporaneousness".to_vec()),
                BorshToken::Bytes(b"automysophobia".to_vec()),
            ]),
        ])])
        .call();

    // test string
    let test_args = |vm: &VirtualMachine, _instr: &Instruction, signers: &[Pubkey]| {
        assert_eq!(
            signers[0],
            create_program_address(&vm.stack[0].id, &[b"Finifugal", b"Falsiloquence"])
        );
        assert_eq!(
            signers[1],
            create_program_address(&vm.stack[0].id, &[b"Obrotund"])
        );
    };

    vm.call_params_check.insert(token.clone(), test_args);

    vm.function("test_string")
        .accounts(vec![("tokenProgram", token.0), ("systemProgram", [0; 32])])
        .arguments(&[BorshToken::Array(vec![
            BorshToken::Array(vec![
                BorshToken::Bytes(b"Finifugal".to_vec()),
                BorshToken::Bytes(b"Falsiloquence".to_vec()),
            ]),
            BorshToken::Array(vec![BorshToken::Bytes(b"Obrotund".to_vec())]),
        ])])
        .call();

    // test address
    let test_args = |vm: &VirtualMachine, _instr: &Instruction, signers: &[Pubkey]| {
        assert_eq!(
            signers[0],
            create_program_address(
                &vm.stack[0].id,
                &[
                    b"quinquagintaquadringentilliardth",
                    b"quinquagintaquadringentillionths"
                ]
            )
        );
        assert_eq!(
            signers[1],
            create_program_address(
                &vm.stack[0].id,
                &[
                    b"quinquagintaquadringentilliardt1",
                    b"quinquagintaquadringentilliardt2",
                    b"quinquagintaquadringentilliardt3"
                ]
            )
        );
    };
    vm.call_params_check.insert(token.clone(), test_args);

    vm.function("test_addr")
        .accounts(vec![("tokenProgram", token.0), ("systemProgram", [0; 32])])
        .arguments(&[BorshToken::Array(vec![
            BorshToken::Array(vec![
                BorshToken::Address(*b"quinquagintaquadringentilliardth"),
                BorshToken::Address(*b"quinquagintaquadringentillionths"),
            ]),
            BorshToken::Array(vec![
                BorshToken::Address(*b"quinquagintaquadringentilliardt1"),
                BorshToken::Address(*b"quinquagintaquadringentilliardt2"),
                BorshToken::Address(*b"quinquagintaquadringentilliardt3"),
            ]),
        ])])
        .call();

    // test bytes4
    let test_args = |vm: &VirtualMachine, _instr: &Instruction, signers: &[Pubkey]| {
        assert_eq!(
            signers[0],
            create_program_address(&vm.stack[0].id, &[b"foofbarf"])
        );
        assert_eq!(
            signers[1],
            create_program_address(&vm.stack[0].id, &[b"drat", b"plop", b"dang"])
        );
    };

    vm.call_params_check.insert(token.clone(), test_args);

    vm.function("test_bytes4")
        .accounts(vec![("tokenProgram", token.0), ("systemProgram", [0; 32])])
        .arguments(&[BorshToken::Array(vec![
            BorshToken::Array(vec![
                BorshToken::FixedBytes(b"foof".to_vec()),
                BorshToken::FixedBytes(b"barf".to_vec()),
            ]),
            BorshToken::Array(vec![
                BorshToken::FixedBytes(b"drat".to_vec()),
                BorshToken::FixedBytes(b"plop".to_vec()),
                BorshToken::FixedBytes(b"dang".to_vec()),
            ]),
        ])])
        .call();
}

#[test]
fn pda_array_of_array_fixed() {
    // now more dynamic
    let mut vm = build_solidity(
        r#"
            import {AccountMeta} from 'solana';

            contract pda {
                address constant tokenProgramId = address"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
                address constant SYSVAR_RENT_PUBKEY = address"SysvarRent111111111111111111111111111111111";

                function test_bytes(bytes[2][] seeds) public {
                    bytes instr = new bytes(1);

                    instr[0] = 0x95;

                    AccountMeta[1] metas = [
                        AccountMeta({pubkey: SYSVAR_RENT_PUBKEY, is_writable: false, is_signer: false})
                    ];

                    tokenProgramId.call{seeds: seeds, accounts: metas}(instr);
                }

                function test_string(string[1][2] seeds) public {
                    bytes instr = new bytes(1);

                    instr[0] = 0x95;

                    AccountMeta[1] metas = [
                        AccountMeta({pubkey: SYSVAR_RENT_PUBKEY, is_writable: false, is_signer: false})
                    ];

                    tokenProgramId.call{seeds: seeds, accounts: metas}(instr);
                }

                function test_addr(address[][2] seeds) public {
                    bytes instr = new bytes(1);

                    instr[0] = 0x95;

                    AccountMeta[1] metas = [
                        AccountMeta({pubkey: SYSVAR_RENT_PUBKEY, is_writable: false, is_signer: false})
                    ];

                    tokenProgramId.call{seeds: seeds, accounts: metas}(instr);
                }
            }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let token = Pubkey(
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            .from_base58()
            .unwrap()
            .try_into()
            .unwrap(),
    );

    vm.account_data.insert(token.0, AccountState::default());

    let test_args = |vm: &VirtualMachine, _instr: &Instruction, signers: &[Pubkey]| {
        assert_eq!(
            signers[0],
            create_program_address(&vm.stack[0].id, &[b"foobar"])
        );
        assert_eq!(
            signers[1],
            create_program_address(&vm.stack[0].id, &[b"zemmiphobia", b"extemporaneousness"])
        );
    };

    vm.call_params_check.insert(token.clone(), test_args);

    vm.function("test_bytes")
        .accounts(vec![("tokenProgram", token.0), ("systemProgram", [0; 32])])
        .arguments(&[BorshToken::Array(vec![
            BorshToken::FixedArray(vec![
                BorshToken::Bytes(b"foo".to_vec()),
                BorshToken::Bytes(b"bar".to_vec()),
            ]),
            BorshToken::FixedArray(vec![
                BorshToken::Bytes(b"zemmiphobia".to_vec()),
                BorshToken::Bytes(b"extemporaneousness".to_vec()),
            ]),
        ])])
        .call();

    // test string
    let test_args = |vm: &VirtualMachine, _instr: &Instruction, signers: &[Pubkey]| {
        assert_eq!(
            signers[0],
            create_program_address(&vm.stack[0].id, &[b"Finifugal"])
        );
        assert_eq!(
            signers[1],
            create_program_address(&vm.stack[0].id, &[b"Obrotund"])
        );
    };

    vm.call_params_check.insert(token.clone(), test_args);

    vm.function("test_string")
        .accounts(vec![("tokenProgram", token.0), ("systemProgram", [0; 32])])
        .arguments(&[BorshToken::FixedArray(vec![
            BorshToken::FixedArray(vec![BorshToken::Bytes(b"Finifugal".to_vec())]),
            BorshToken::FixedArray(vec![BorshToken::Bytes(b"Obrotund".to_vec())]),
        ])])
        .call();

    // // test address
    let test_args = |vm: &VirtualMachine, _instr: &Instruction, signers: &[Pubkey]| {
        assert_eq!(
            signers[0],
            create_program_address(
                &vm.stack[0].id,
                &[
                    b"quinquagintaquadringentilliardth",
                    b"quinquagintaquadringentillionths"
                ]
            )
        );
        assert_eq!(
            signers[1],
            create_program_address(
                &vm.stack[0].id,
                &[
                    b"quinquagintaquadringentilliardt1",
                    b"quinquagintaquadringentilliardt2",
                    b"quinquagintaquadringentilliardt3"
                ]
            )
        );
    };
    vm.call_params_check.insert(token.clone(), test_args);

    vm.function("test_addr")
        .accounts(vec![("tokenProgram", token.0), ("systemProgram", [0; 32])])
        .arguments(&[BorshToken::FixedArray(vec![
            BorshToken::Array(vec![
                BorshToken::Address(*b"quinquagintaquadringentilliardth"),
                BorshToken::Address(*b"quinquagintaquadringentillionths"),
            ]),
            BorshToken::Array(vec![
                BorshToken::Address(*b"quinquagintaquadringentilliardt1"),
                BorshToken::Address(*b"quinquagintaquadringentilliardt2"),
                BorshToken::Address(*b"quinquagintaquadringentilliardt3"),
            ]),
        ])])
        .call();
}

#[test]
fn pda_array_of_array_mixed() {
    // now more dynamic
    let mut vm = build_solidity(
        r#"
            import {AccountMeta} from 'solana';

            contract pda {
                address constant tokenProgramId = address"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
                address constant SYSVAR_RENT_PUBKEY = address"SysvarRent111111111111111111111111111111111";

                function test(bytes[] dyn, address[] addr, bytes5[] b5, string f) public {
                    bytes instr = new bytes(1);
                    instr[0] = 0x95;
                    AccountMeta[1] metas = [
                        AccountMeta({pubkey: SYSVAR_RENT_PUBKEY, is_writable: false, is_signer: false})
                    ];
                    tokenProgramId.call{seeds: [ dyn, addr, b5, [f] ], accounts: metas}(instr);
                }

                function test_bytes2(bytes[][] seeds) public {
                    bytes instr = new bytes(1);

                    instr[0] = 0x95;

                    AccountMeta[1] metas = [
                        AccountMeta({pubkey: SYSVAR_RENT_PUBKEY, is_writable: false, is_signer: false})
                    ];

                    tokenProgramId.call{seeds: [seeds[0], seeds[1]], accounts: metas}(instr);
                }
            }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let token = Pubkey(
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            .from_base58()
            .unwrap()
            .try_into()
            .unwrap(),
    );

    vm.account_data.insert(token.0, AccountState::default());

    let test_args = |vm: &VirtualMachine, _instr: &Instruction, signers: &[Pubkey]| {
        assert_eq!(signers.len(), 4);

        assert_eq!(
            signers[0],
            create_program_address(&vm.stack[0].id, &[b"foobar"])
        );
        assert_eq!(
            signers[1],
            create_program_address(
                &vm.stack[0].id,
                &[
                    b"quinquagintaquadringentilliardt1",
                    b"quinquagintaquadringentilliardt2",
                    b"quinquagintaquadringentilliardt3"
                ]
            )
        );
        assert_eq!(
            signers[2],
            create_program_address(&vm.stack[0].id, &[b"azure", b"squab", b"tares"])
        );
        assert_eq!(
            signers[3],
            create_program_address(&vm.stack[0].id, &[b"penultimatum"])
        );
    };

    vm.call_params_check.insert(token.clone(), test_args);

    vm.function("test")
        .accounts(vec![("tokenProgram", token.0), ("systemProgram", [0; 32])])
        .arguments(&[
            BorshToken::Array(vec![
                BorshToken::Bytes(b"foo".to_vec()),
                BorshToken::Bytes(b"bar".to_vec()),
            ]),
            BorshToken::Array(vec![
                BorshToken::Address(*b"quinquagintaquadringentilliardt1"),
                BorshToken::Address(*b"quinquagintaquadringentilliardt2"),
                BorshToken::Address(*b"quinquagintaquadringentilliardt3"),
            ]),
            BorshToken::Array(vec![
                BorshToken::FixedBytes(b"azure".to_vec()),
                BorshToken::FixedBytes(b"squab".to_vec()),
                BorshToken::FixedBytes(b"tares".to_vec()),
            ]),
            BorshToken::String("penultimatum".into()),
        ])
        .call();

    let test_args = |vm: &VirtualMachine, _instr: &Instruction, signers: &[Pubkey]| {
        assert_eq!(signers.len(), 2);

        assert_eq!(
            signers[0],
            create_program_address(&vm.stack[0].id, &[b"foobar"])
        );
        assert_eq!(
            signers[1],
            create_program_address(
                &vm.stack[0].id,
                &[b"zemmiphobia", b"extemporaneousness", b"automysophobia"]
            )
        );
    };
    vm.call_params_check.insert(token.clone(), test_args);

    vm.function("test_bytes2")
        .accounts(vec![("tokenProgram", token.0), ("systemProgram", [0; 32])])
        .arguments(&[BorshToken::Array(vec![
            BorshToken::Array(vec![
                BorshToken::Bytes(b"foo".to_vec()),
                BorshToken::Bytes(b"bar".to_vec()),
            ]),
            BorshToken::Array(vec![
                BorshToken::Bytes(b"zemmiphobia".to_vec()),
                BorshToken::Bytes(b"extemporaneousness".to_vec()),
                BorshToken::Bytes(b"automysophobia".to_vec()),
            ]),
            BorshToken::Array(vec![
                BorshToken::Bytes(b"ignore".to_vec()),
                BorshToken::Bytes(b"this".to_vec()),
            ]),
        ])])
        .call();
}
