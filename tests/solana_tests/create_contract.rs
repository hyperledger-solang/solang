// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use ethabi::Token;

#[test]
fn simple_create_contract() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_other() public returns (bar1) {
                bar1 x = new bar1("yo from bar0");

                return x;
            }

            function call_bar1_at_address(bar1 a, string x) public {
                a.say_hello(x);
            }
        }

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

    vm.constructor("bar0", &[]);

    let seed = vm.create_empty_account();

    let bar1 = vm.function("test_other", &[], &[&seed], None);

    assert_eq!(vm.logs, "bar1 says: yo from bar0");

    vm.logs.truncate(0);

    println!("next test, {:?}", bar1);

    vm.function(
        "call_bar1_at_address",
        &[bar1[0].clone(), Token::String(String::from("xywoleh"))],
        &[],
        None,
    );

    assert_eq!(vm.logs, "Hello xywoleh");
}

#[test]
// 64424509440 = 15 << 32 (ERROR_NEW_ACCOUNT_NEEDED)
fn missing_contract() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_other() public returns (bar1) {
                bar1 x = new bar1("yo from bar0");

                return x;
            }

            function call_bar1_at_address(bar1 a, string x) public {
                a.say_hello(x);
            }
        }

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

    vm.constructor("bar0", &[]);

    let res = vm.function_must_fail("test_other", &[], &[], None);
    assert_eq!(res, Ok(64424509440));
}

#[test]
fn two_contracts() {
    let mut vm = build_solidity(
        r#"
        contract bar0 {
            function test_other() public returns (bar1) {
                bar1 x = new bar1("yo from bar0");
                bar1 y = new bar1("hi from bar0");

                return x;
            }
        }

        contract bar1 {
            constructor(string v) {
                print("bar1 says: " + v);
            }
        }"#,
    );

    vm.set_program(0);

    vm.constructor("bar0", &[]);

    let seed1 = vm.create_empty_account();
    let seed2 = vm.create_empty_account();

    let _bar1 = vm.function("test_other", &[], &[&seed1, &seed2], None);

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

    vm.constructor_expected(5 << 32, "bar", &[]);
}
