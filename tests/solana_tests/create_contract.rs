// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_new, build_solidity, create_program_address, Account, AccountState, BorshToken,
};
use base58::{FromBase58, ToBase58};
use num_bigint::BigInt;

#[test]
fn simple_create_contract_no_seed() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_other() external {
                bar1.new("yo from bar0");
            }

            function call_bar1_at_address(string x) external {
                bar1.say_hello(x);
            }
        }

        @program_id("CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT")
        contract bar1 {
            @payer(payer)
            constructor(string v) {
                print(string.concat("bar1 says: ", v));
            }

            function say_hello(string v) public {
                print("Hello {}".format(v));
            }
        }"#,
    );

    vm.set_program(0);

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let program_id: Account = "CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT"
        .from_base58()
        .unwrap()
        .try_into()
        .unwrap();

    let acc = account_new();
    let payer = account_new();

    println!("new account: {}", acc.to_base58());

    vm.account_data.insert(payer, AccountState::default());
    vm.account_data.insert(
        acc,
        AccountState {
            data: Vec::new(),
            owner: Some(program_id),
            lamports: 0,
        },
    );

    vm.function("test_other")
        .accounts(vec![
            ("bar1_dataAccount", acc),
            ("bar1_programId", program_id),
            ("payer", payer),
            ("systemProgram", [0; 32]),
        ])
        .call();

    assert_eq!(vm.logs, "bar1 says: yo from bar0");

    assert_eq!(vm.account_data[&acc].data.len(), 16);

    vm.logs.truncate(0);

    vm.function("call_bar1_at_address")
        .arguments(&[BorshToken::String(String::from("xywoleh"))])
        .accounts(vec![
            ("bar1_programId", program_id),
            ("systemProgram", [0; 32]),
        ])
        .call();

    assert_eq!(vm.logs, "Hello xywoleh");
}

#[test]
fn simple_create_contract() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_other() external {
                bar1.new("yo from bar0");
            }

            function call_bar1_at_address(string x) external {
                bar1.say_hello(x);
            }
        }

        @program_id("CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT")
        contract bar1 {
            @payer(pay)
            constructor(string v) {
                print(string.concat("bar1 says: ", v));
            }

            function say_hello(string v) public {
                print("Hello {}".format(v));
            }
        }"#,
    );

    vm.set_program(0);

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let program_id: Account = "CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT"
        .from_base58()
        .unwrap()
        .try_into()
        .unwrap();

    let seed = vm.create_pda(&program_id, 7);
    let payer = account_new();

    vm.account_data.insert(payer, AccountState::default());

    vm.function("test_other")
        .accounts(vec![
            ("bar1_dataAccount", seed.0),
            ("bar1_programId", program_id),
            ("pay", payer),
            ("systemProgram", [0; 32]),
        ])
        .call();

    assert_eq!(vm.logs, "bar1 says: yo from bar0");

    vm.logs.truncate(0);

    vm.function("call_bar1_at_address")
        .arguments(&[BorshToken::String(String::from("xywoleh"))])
        .accounts(vec![
            ("bar1_programId", program_id),
            ("systemProgram", [0; 32]),
        ])
        .call();

    assert_eq!(vm.logs, "Hello xywoleh");
}

#[test]
fn create_contract_wrong_program_id() {
    let mut vm = build_solidity(
        r#"
        @program_id("CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT")
        contract bar0 {}
        "#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let program = &vm.programs[0].id;
    let code = vm.account_data[program].data.clone();

    let mut vm = build_solidity(
        r#"
        @program_id("25UGQeMKp1YH8dR1WBtaj26iqfc49xjwfvLnUKavcz8E")
        contract bar0 {}
        "#,
    );

    let program = &vm.programs[0].id;
    vm.account_data.get_mut(program).unwrap().data = code;

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .expected(7 << 32)
        .call();

    assert_eq!(
        vm.logs,
        "program_id should be CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT"
    );
}

#[test]
fn call_constructor_twice() {
    let mut vm = build_solidity(
        r#"
        @program_id("CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT")
        contract bar0 {}
        "#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .expected(2)
        .call();
}

#[test]
fn create_contract_with_payer() {
    let mut vm = build_solidity(
        r#"
        contract x {
            uint64 v;

            @payer(p)
            constructor() {
                v = 102;
            }

            function f() public returns (uint64) {
                return v;
            }
        }"#,
    );

    let payer = account_new();
    vm.account_data.insert(payer, AccountState::default());
    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![
            ("dataAccount", data_account),
            ("p", payer),
            ("systemProgram", [0; 32]),
        ])
        .call();

    let ret = vm
        .function("f")
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();

    assert_eq!(
        ret,
        BorshToken::Uint {
            width: 64,
            value: 102.into()
        }
    );
}

#[test]
#[should_panic(expected = "external call failed")]
// 64424509440 = 15 << 32 (ERROR_NEW_ACCOUNT_NEEDED)
fn missing_contract() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_other() external {
                bar1.new("yo from bar0");
            }

            function call_bar1_at_address(string x) external {
                bar1.say_hello(x);
            }
        }

        @program_id("7vJKRaKLGCNUPuHWdeHCTknkYf3dHXXEZ6ri7dc6ngeV")
        contract bar1 {
            constructor(string v) {
                print(string.concat("bar1 says: ", v));
            }

            function say_hello(string v) public {
                print("Hello {}".format(v));
            }
        }"#,
    );

    vm.set_program(0);
    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let missing = account_new();
    vm.logs.clear();
    vm.account_data.insert(missing, AccountState::default());

    let program_id: Account = "7vJKRaKLGCNUPuHWdeHCTknkYf3dHXXEZ6ri7dc6ngeV"
        .from_base58()
        .unwrap()
        .try_into()
        .unwrap();

    // There is no payer account, so the external call fails.
    let _ = vm
        .function("test_other")
        .accounts(vec![
            ("bar1_programId", program_id),
            ("bar1_dataAccount", missing),
            ("systemProgram", [0; 32]),
        ])
        .must_fail();
}

#[test]
fn two_contracts() {
    let mut vm = build_solidity(
        r#"
        import 'solana';

        contract bar0 {

            @mutableSigner(a)
            @mutableSigner(b)
            @mutableSigner(payer)
            function test_other() external {
                AccountMeta[2] bar1_metas = [
                    AccountMeta({pubkey: tx.accounts.a.key, is_writable: true, is_signer: true}),
                    AccountMeta({pubkey: tx.accounts.payer.key, is_writable: true, is_signer: true})
                ];
                AccountMeta[2] bar2_metas = [
                    AccountMeta({pubkey: tx.accounts.b.key, is_writable: true, is_signer: true}),
                    AccountMeta({pubkey: tx.accounts.payer.key, is_writable: true, is_signer: true})
                ];
                bar1.new{accounts: bar1_metas}("yo from bar0");
                bar1.new{accounts: bar2_metas}("hi from bar0");
            }
        }

        @program_id("CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT")
        contract bar1 {
            @payer(payer_account)
            constructor(string v) {
                print(string.concat("bar1 says: ", v));
            }
        }"#,
    );

    vm.set_program(0);
    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let program_id: Account = "CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT"
        .from_base58()
        .unwrap()
        .try_into()
        .unwrap();

    let seed1 = vm.create_pda(&program_id, 5);
    let seed2 = vm.create_pda(&program_id, 5);
    let payer = account_new();

    vm.account_data.insert(seed1.0, AccountState::default());
    vm.account_data.insert(seed2.0, AccountState::default());
    vm.account_data.insert(payer, AccountState::default());

    vm.function("test_other")
        .accounts(vec![
            ("a", seed1.0),
            ("b", seed2.0),
            ("payer", payer),
            ("systemProgram", [0; 32]),
            ("bar1_programId", program_id),
        ])
        .call();

    assert_eq!(vm.logs, "bar1 says: yo from bar0bar1 says: hi from bar0");

    vm.logs.truncate(0);
}

#[test]
fn account_too_small() {
    let mut vm = build_solidity(
        r#"
        contract bar {
            int[200] foo1;
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.account_data
        .get_mut(&data_account)
        .unwrap()
        .data
        .truncate(100);

    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .expected(5 << 32)
        .call();
}

#[test]
fn account_with_space() {
    let mut vm = build_solidity(
        r#"
        contract bar {

            @payer(payer)
            constructor(@space uint64 x) {}

            function hello() public returns (bool) {
                return true;
            }
        }
        "#,
    );

    let data_account = vm.initialize_data_account();
    vm.account_data
        .get_mut(&data_account)
        .unwrap()
        .data
        .truncate(0);

    let payer = account_new();
    vm.account_data.insert(payer, AccountState::default());

    vm.function("new")
        .accounts(vec![
            ("dataAccount", data_account),
            ("payer", payer),
            ("systemProgram", [0; 32]),
        ])
        .arguments(&[BorshToken::Uint {
            width: 64,
            value: 306.into(),
        }])
        .call();
    assert_eq!(
        vm.account_data.get_mut(&data_account).unwrap().data.len(),
        306
    );

    let ret = vm.function("hello").call().unwrap();

    assert_eq!(ret, BorshToken::Bool(true));
}

#[test]
fn account_with_seed() {
    let mut vm = build_solidity(
        r#"
        contract bar {

            @space(511 + 102)
            @payer(payer)
            constructor(@seed bytes seed) {}

            function hello() public returns (bool) {
                return true;
            }
        }
        "#,
    );

    let program_id = vm.stack[0].id;
    let seed = vm.create_pda(&program_id, 7);
    let payer = account_new();
    vm.account_data.insert(payer, AccountState::default());

    vm.function("new")
        .accounts(vec![
            ("dataAccount", seed.0),
            ("payer", payer),
            ("systemProgram", [0; 32]),
        ])
        .arguments(&[BorshToken::Bytes(seed.1)])
        .call();

    assert_eq!(
        vm.account_data.get_mut(&seed.0).unwrap().data.len(),
        511 + 102
    );

    let ret = vm.function("hello").call().unwrap();

    assert_eq!(ret, BorshToken::Bool(true));
}

#[test]
fn account_with_seed_bump() {
    let mut vm = build_solidity(
        r#"
        contract bar {
            @space(511 + 102)
            @payer(payer)
            constructor(@seed address seed, @seed bytes2 seed2, @bump byte b) {}

            function hello() public returns (bool) {
                return true;
            }
        }
        "#,
    );

    let program_id = vm.stack[0].id;

    let (address, full_seed) = vm.create_pda(&program_id, 35);
    let bump = full_seed[34];
    let seed_addr = &full_seed[0..32];
    let seed2 = &full_seed[32..34];
    let payer = account_new();
    vm.account_data.insert(payer, AccountState::default());

    vm.function("new")
        .arguments(&[
            BorshToken::Address(seed_addr.try_into().unwrap()),
            BorshToken::FixedBytes(seed2.to_vec()),
            BorshToken::Uint {
                width: 8,
                value: bump.into(),
            },
        ])
        .accounts(vec![
            ("dataAccount", address),
            ("payer", payer),
            ("systemProgram", [0; 32]),
        ])
        .call();

    assert_eq!(
        vm.account_data.get_mut(&address).unwrap().data.len(),
        511 + 102
    );

    let ret = vm.function("hello").call().unwrap();

    assert_eq!(ret, BorshToken::Bool(true));
}

#[test]
fn account_with_seed_bump_literals() {
    let mut vm = build_solidity(
        r#"
        @program_id("vS5Tf8mnHGbUCMLQWrnvsFvwHLfA5p3yQM3ozxPckn8")
        contract bar {
            @space(2 << 8 + 4)
            @seed("meh")
            @bump(33) // 33 = ascii !
            @payer(my_account)
            constructor() {}

            function hello() public returns (bool) {
                return true;
            }
        }
        "#,
    );

    let program_id = vm.stack[0].id;

    let account = create_program_address(&program_id, &[b"meh!"]);
    let payer = account_new();
    vm.create_empty_account(&account.0, &program_id);
    vm.account_data.insert(payer, AccountState::default());

    vm.function("new")
        .accounts(vec![
            ("dataAccount", account.0),
            ("my_account", payer),
            ("systemProgram", [0; 32]),
        ])
        .call();

    assert_eq!(
        vm.account_data.get_mut(&account.0).unwrap().data.len(),
        8192
    );

    let ret = vm.function("hello").call().unwrap();

    assert_eq!(ret, BorshToken::Bool(true));
}

#[test]
fn create_child() {
    let mut vm = build_solidity(
        r#"
        contract creator {
            function create_child() external {
                print("Going to create child");
                Child.new();
                Child.say_hello();
            }
        }

        @program_id("Chi1d5XD6nTAp2EyaNGqMxZzUjh6NvhXRxbGHP3D1RaT")
        contract Child {
            @payer(payer)
            @space(511 + 7)
            constructor() {
                print("In child constructor");
            }

            function say_hello() pure public {
                print("Hello there");
            }
        }"#,
    );

    vm.set_program(0);
    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let child_program_id: Account = "Chi1d5XD6nTAp2EyaNGqMxZzUjh6NvhXRxbGHP3D1RaT"
        .from_base58()
        .unwrap()
        .try_into()
        .unwrap();

    let payer = account_new();
    let program_id = vm.stack[0].id;

    let seed = vm.create_pda(&program_id, 7);
    vm.account_data.insert(payer, AccountState::default());
    vm.account_data.insert(seed.0, AccountState::default());

    vm.function("create_child")
        .accounts(vec![
            ("Child_dataAccount", seed.0),
            ("Child_programId", child_program_id),
            ("payer", payer),
            ("systemProgram", [0; 32]),
        ])
        .call();

    assert_eq!(
        vm.logs,
        "Going to create childIn child constructorHello there"
    );
}

#[test]
fn create_child_with_meta() {
    let mut vm = build_solidity(
        r#"
        import 'solana';

contract creator {

    @mutableSigner(child)
    @mutableSigner(payer)
    function create_child_with_meta() external {
        print("Going to create child");
        AccountMeta[2] metas = [
            AccountMeta({pubkey: tx.accounts.child.key, is_signer: true, is_writable: true}),
            AccountMeta({pubkey: tx.accounts.payer.key, is_signer: true, is_writable: true})
            // Passing the system account here crashes the VM, even if I add it to vm.account_data
            // AccountMeta({pubkey: address"11111111111111111111111111111111", is_writable: false, is_signer: false})
        ];
        Child.new{accounts: metas}();
        Child.say_hello{accounts: []}();
    }
}

@program_id("Chi1d5XD6nTAp2EyaNGqMxZzUjh6NvhXRxbGHP3D1RaT")
contract Child {
    @payer(payer)
    @space(511 + 7)
    constructor() {
        print("In child constructor");
    }

    function say_hello() pure public {
        print("Hello there");
    }
}
        "#,
    );

    vm.set_program(0);
    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let payer = account_new();
    let program_id = vm.stack[0].id;
    let seed = vm.create_pda(&program_id, 7);
    vm.account_data.insert(seed.0, AccountState::default());
    vm.account_data.insert(payer, AccountState::default());

    let child_program_id: Account = "Chi1d5XD6nTAp2EyaNGqMxZzUjh6NvhXRxbGHP3D1RaT"
        .from_base58()
        .unwrap()
        .try_into()
        .unwrap();

    vm.function("create_child_with_meta")
        .accounts(vec![
            ("Child_programId", child_program_id),
            ("child", seed.0),
            ("payer", payer),
            ("systemProgram", [0; 32]),
        ])
        .call();

    assert_eq!(
        vm.logs,
        "Going to create childIn child constructorHello there"
    );
}

#[test]
fn not_enough_space() {
    let mut vm = build_solidity(
        r#"
        contract Test {
    address a;
    address b;

    @payer(acc)
    constructor(address c, address d, @space uint64 mySpace) {
        a = c;
        b = d;
    }
}
        "#,
    );

    let data_account = account_new();
    vm.account_data
        .insert(data_account, AccountState::default());
    let payer = account_new();
    vm.account_data.insert(payer, AccountState::default());
    let dummy_account_1 = account_new();
    let dummy_account_2 = account_new();

    // This should work
    vm.function("new")
        .arguments(&[
            BorshToken::Address(dummy_account_1),
            BorshToken::Address(dummy_account_2),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(120),
            },
        ])
        .accounts(vec![
            ("dataAccount", data_account),
            ("acc", payer),
            ("systemProgram", [0; 32]),
        ])
        .call();

    let other_account = account_new();
    vm.account_data
        .insert(other_account, AccountState::default());

    // This must fail
    let res = vm
        .function("new")
        .arguments(&[
            BorshToken::Address(dummy_account_1),
            BorshToken::Address(dummy_account_2),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(8),
            },
        ])
        .accounts(vec![
            ("dataAccount", other_account),
            ("acc", payer),
            ("systemProgram", [0; 32]),
        ])
        .must_fail();

    assert_eq!(res.unwrap(), 5u64 << 32);
    assert_eq!(
        vm.logs,
        "value passed for space is insufficient. Contract requires at least 80 bytes"
    );
}
