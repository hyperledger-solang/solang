// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_new, build_solidity, create_program_address, Account, AccountMeta, AccountState,
    BorshToken, Pubkey,
};
use base58::{FromBase58, ToBase58};

#[test]
fn simple_create_contract_no_seed() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_other(address foo, address payer) public returns (bar1) {
                bar1 x = new bar1{address: foo}("yo from bar0", payer);

                return x;
            }

            function call_bar1_at_address(bar1 a, string x) public {
                a.say_hello(x);
            }
        }

        @program_id("CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT")
        contract bar1 {
            @payer(payer)
            constructor(string v, address payer) {
                print("bar1 says: " + v);
            }

            function say_hello(string v) public {
                print("Hello {}".format(v));
            }
        }"#,
    );

    vm.set_program(0);

    vm.constructor(&[]);

    let program_id: Account = "CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT"
        .from_base58()
        .unwrap()
        .try_into()
        .unwrap();

    let acc = account_new();
    let payer = account_new();

    println!("new account: {}", acc.to_base58());

    vm.account_data.insert(
        acc,
        AccountState {
            data: Vec::new(),
            owner: Some(program_id),
            lamports: 0,
        },
    );

    let bar1 = vm
        .function(
            "test_other",
            &[BorshToken::Address(acc), BorshToken::Address(payer)],
        )
        .unwrap();

    assert_eq!(vm.logs, "bar1 says: yo from bar0");

    assert_eq!(vm.account_data[&acc].data.len(), 16);

    vm.logs.truncate(0);

    vm.function(
        "call_bar1_at_address",
        &[bar1, BorshToken::String(String::from("xywoleh"))],
    );

    assert_eq!(vm.logs, "Hello xywoleh");
}

#[test]
fn simple_create_contract() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_other(address foo, address payer) public returns (bar1) {
                bar1 x = new bar1{address: foo}("yo from bar0", payer);

                return x;
            }

            function call_bar1_at_address(bar1 a, string x) public {
                a.say_hello(x);
            }
        }

        @program_id("CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT")
        contract bar1 {
            @payer(pay)
            constructor(string v, address pay) {
                print("bar1 says: " + v);
            }

            function say_hello(string v) public {
                print("Hello {}".format(v));
            }
        }"#,
    );

    vm.set_program(0);

    vm.constructor(&[]);

    let program_id: Account = "CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT"
        .from_base58()
        .unwrap()
        .try_into()
        .unwrap();

    let seed = vm.create_pda(&program_id);
    let payer = account_new();

    let bar1 = vm
        .function(
            "test_other",
            &[BorshToken::Address(seed.0), BorshToken::Address(payer)],
        )
        .unwrap();

    assert_eq!(vm.logs, "bar1 says: yo from bar0");

    vm.logs.truncate(0);

    println!("next test, {bar1:?}");

    vm.function(
        "call_bar1_at_address",
        &[bar1, BorshToken::String(String::from("xywoleh"))],
    );

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

    vm.constructor(&[]);

    let program = &vm.programs[0].program;
    let code = vm.account_data[program].data.clone();

    let mut vm = build_solidity(
        r#"
        @program_id("25UGQeMKp1YH8dR1WBtaj26iqfc49xjwfvLnUKavcz8E")
        contract bar0 {}
        "#,
    );

    let program = &vm.programs[0].program;
    vm.account_data.get_mut(program).unwrap().data = code;

    vm.constructor_expected(7 << 32, &[]);

    assert_eq!(
        vm.logs,
        "program_id should be CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT"
    );
}

#[test]
fn create_contract_with_payer() {
    let mut vm = build_solidity(
        r#"
        contract x {
            uint64 v;

            @payer(p)
            constructor(address p) {
                v = 102;
            }

            function f() public returns (uint64) {
                return v;
            }
        }"#,
    );

    // 'remove' the data account
    let data = vm.programs[0].data;
    vm.account_data.get_mut(&data).unwrap().data.truncate(0);
    let payer = account_new();

    vm.constructor(&[BorshToken::Address(payer)]);

    let ret = vm.function("f", &[]).unwrap();

    assert_eq!(
        ret,
        BorshToken::Uint {
            width: 64,
            value: 102.into()
        }
    );
}

#[test]
// 64424509440 = 15 << 32 (ERROR_NEW_ACCOUNT_NEEDED)
fn missing_contract() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_other(address foo) public returns (bar1) {
                bar1 x = new bar1{address: foo}("yo from bar0");

                return x;
            }

            function call_bar1_at_address(bar1 a, string x) public {
                a.say_hello(x);
            }
        }

        @program_id("7vJKRaKLGCNUPuHWdeHCTknkYf3dHXXEZ6ri7dc6ngeV")
        contract bar1 {
            constructor(string v) {
                print("bar1 says: " + v);
            }

            function say_hello(string v) public {
                print("Hello {}".format(v));
            }
        }"#,
    );

    vm.set_program(0);

    vm.constructor(&[]);

    let missing = account_new();

    vm.logs.clear();
    let _ = vm.function_must_fail("test_other", &[BorshToken::Address(missing)]);
    assert_eq!(vm.logs, "new account needed");
}

#[test]
fn two_contracts() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_other(address a, address b) public returns (bar1) {
                bar1 x = new bar1{address: a}("yo from bar0");
                bar1 y = new bar1{address: b}("hi from bar0");

                return x;
            }
        }

        @program_id("CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT")
        contract bar1 {
            @payer(address"3wvhRNAJSDCk5Mub8NEcShszRrHHVDHsuSUdAcL2aaMV")
            constructor(string v) {
                print("bar1 says: " + v);
            }
        }"#,
    );

    vm.set_program(0);

    vm.constructor(&[]);

    let program_id: Account = "CPDgqnhHDCsjFkJKMturRQ1QeM9EXZg3EYCeDoRP8pdT"
        .from_base58()
        .unwrap()
        .try_into()
        .unwrap();

    let seed1 = vm.create_pda(&program_id);
    let seed2 = vm.create_pda(&program_id);

    let _bar1 = vm.function(
        "test_other",
        &[BorshToken::Address(seed1.0), BorshToken::Address(seed2.0)],
    );

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

    let data = vm.stack[0].data;

    vm.account_data.get_mut(&data).unwrap().data.truncate(100);

    vm.constructor_expected(5 << 32, &[]);
}

#[test]
fn account_with_space() {
    let mut vm = build_solidity(
        r#"
        contract bar {

            @space(102 * x)
            @payer(payer)
            constructor(uint64 x, address payer) {}

            function hello() public returns (bool) {
                return true;
            }
        }
        "#,
    );

    let data = vm.stack[0].data;

    vm.account_data.get_mut(&data).unwrap().data.truncate(0);

    let payer = account_new();

    vm.constructor(&[
        BorshToken::Uint {
            width: 64,
            value: 3.into(),
        },
        BorshToken::Address(payer),
    ]);

    assert_eq!(vm.account_data.get_mut(&data).unwrap().data.len(), 3 * 102);

    let ret = vm.function("hello", &[]).unwrap();

    assert_eq!(ret, BorshToken::Bool(true));
}

#[test]
fn account_with_seed() {
    let mut vm = build_solidity(
        r#"
        contract bar {

            @space(511 + 102)
            @seed(seed)
            @payer(payer)
            constructor(bytes seed, address payer) {}

            function hello() public returns (bool) {
                return true;
            }
        }
        "#,
    );

    let program_id = vm.stack[0].program;

    let seed = vm.create_pda(&program_id);

    vm.stack[0].data = seed.0;

    let payer = account_new();

    vm.constructor(&[BorshToken::Bytes(seed.1), BorshToken::Address(payer)]);

    assert_eq!(
        vm.account_data.get_mut(&seed.0).unwrap().data.len(),
        511 + 102
    );

    let ret = vm.function("hello", &[]).unwrap();

    assert_eq!(ret, BorshToken::Bool(true));
}

#[test]
fn account_with_seed_bump() {
    let mut vm = build_solidity(
        r#"
        contract bar {

            @space(511 + 102)
            @seed(seed)
            @bump(b)
            @payer(payer)
            constructor(bytes seed, address payer, byte b) {}

            function hello() public returns (bool) {
                return true;
            }
        }
        "#,
    );

    let program_id = vm.stack[0].program;

    let mut seed = vm.create_pda(&program_id);

    let bump = seed.1.pop().unwrap();

    vm.stack[0].data = seed.0;

    let payer = account_new();

    vm.constructor(&[
        BorshToken::Bytes(seed.1),
        BorshToken::Address(payer),
        BorshToken::Uint {
            width: 8,
            value: bump.into(),
        },
    ]);

    assert_eq!(
        vm.account_data.get_mut(&seed.0).unwrap().data.len(),
        511 + 102
    );

    let ret = vm.function("hello", &[]).unwrap();

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
            @payer(address"vS5Tf8mnHGbUCMLQWrnvsFvwHLfA5p3yQM3ozxPckn8")
            constructor() {}

            function hello() public returns (bool) {
                return true;
            }
        }
        "#,
    );

    let program_id = vm.stack[0].program;

    let account = create_program_address(&program_id, &[b"meh!"]);

    vm.create_empty_account(&account.0, &program_id);

    vm.stack[0].data = account.0;

    vm.constructor(&[]);

    assert_eq!(
        vm.account_data.get_mut(&account.0).unwrap().data.len(),
        8192
    );

    let ret = vm.function("hello", &[]).unwrap();

    assert_eq!(ret, BorshToken::Bool(true));
}

#[test]
fn create_child() {
    let mut vm = build_solidity(
        r#"
        contract creator {
            Child public c;

            function create_child(address child, address payer) public {
                print("Going to create child");
                c = new Child{address: child}(payer);

                c.say_hello();
            }
        }

        @program_id("Chi1d5XD6nTAp2EyaNGqMxZzUjh6NvhXRxbGHP3D1RaT")
        contract Child {
            @payer(payer)
            @space(511 + 7)
            constructor(address payer) {
                print("In child constructor");
            }

            function say_hello() pure public {
                print("Hello there");
            }
        }"#,
    );

    vm.set_program(0);
    vm.constructor(&[]);

    let payer = account_new();

    let program_id = vm.stack[0].program;

    let seed = vm.create_pda(&program_id);

    vm.function(
        "create_child",
        &[BorshToken::Address(seed.0), BorshToken::Address(payer)],
    );

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
    Child public c;
    function create_child_with_meta(address child, address payer) public {
        print("Going to create child");
        AccountMeta[2] metas = [
            AccountMeta({pubkey: child, is_signer: true, is_writable: true}),
            AccountMeta({pubkey: payer, is_signer: true, is_writable: true})
            // Passing the system account here crashes the VM, even if I add it to vm.account_data
            // AccountMeta({pubkey: address"11111111111111111111111111111111", is_writable: false, is_signer: false})
        ];
        c = new Child{accounts: metas}(payer);
        c.say_hello();
    }
}

@program_id("Chi1d5XD6nTAp2EyaNGqMxZzUjh6NvhXRxbGHP3D1RaT")
contract Child {
    @payer(payer)
    @space(511 + 7)
    constructor(address payer) {
        print("In child constructor");
    }

    function say_hello() pure public {
        print("Hello there");
    }
}
        "#,
    );

    vm.set_program(0);

    vm.constructor(&[]);

    let payer = account_new();

    let program_id = vm.stack[0].program;

    let seed = vm.create_pda(&program_id);

    vm.account_data.insert(
        seed.0,
        AccountState {
            data: vec![],
            owner: None,
            lamports: 0,
        },
    );

    vm.account_data.insert(
        payer,
        AccountState {
            data: vec![],
            owner: None,
            lamports: 0,
        },
    );

    let mut metas = vm.default_metas();
    metas.push(AccountMeta {
        pubkey: Pubkey(seed.0),
        is_signer: false,
        is_writable: false,
    });
    metas.push(AccountMeta {
        pubkey: Pubkey(payer),
        is_signer: true,
        is_writable: false,
    });

    vm.function_metas(
        "create_child_with_meta",
        &metas,
        &[BorshToken::Address(seed.0), BorshToken::Address(payer)],
    );

    assert_eq!(
        vm.logs,
        "Going to create childIn child constructorHello there"
    );
}
