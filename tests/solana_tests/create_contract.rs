use crate::{build_solidity, first_error, no_errors, parse_and_resolve};
use ethabi::Token;
use solang::Target;

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

    vm.constructor("bar0", &[], 0);

    let seed = vm.create_empty_account();

    let bar1 = vm.function("test_other", &[], &[&seed], 0, None);

    assert_eq!(vm.logs, "bar1 says: yo from bar0");

    vm.logs.truncate(0);

    println!("next test, {:?}", bar1);

    vm.function(
        "call_bar1_at_address",
        &[bar1[0].clone(), Token::String(String::from("xywoleh"))],
        &[],
        0,
        None,
    );

    assert_eq!(vm.logs, "Hello xywoleh");
}

#[test]
fn base_contract() {
    let ns = parse_and_resolve(
        r#"
        contract Math {
            enum MathError {
                NO_ERROR
            }
        }

        contract IsMath is Math {
            struct WithMath {
                MathError math;
            }
        }
    "#,
        Target::Solana,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        contract Logic {
            enum LogicError {
                LE_ERROR
            }
        }
        contract Math is Logic {
            enum MathError {
                NO_ERROR
            }
        }

        contract IsMath is Math {
            struct WithMath {
                MathError math;
                LogicError logic;
            }
        }
    "#,
        Target::Solana,
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        contract Logic {
            struct LogicFields {
                uint logia;
            }
        }
        contract Math is Logic {
        }

        contract IsMath is Math {
            struct WithMath {
                LogicFields logia;
            }
        }
    "#,
        Target::Solana,
    );

    no_errors(ns.diagnostics);
}

#[test]
// 64424509440 = 15 << 32 (ERROR_NEW_ACCOUNT_NEEDED)
#[should_panic(expected = "64424509440")]
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

    vm.constructor("bar0", &[], 0);

    let _ = vm.function("test_other", &[], &[], 0, None);
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

    vm.constructor("bar0", &[], 0);

    let seed1 = vm.create_empty_account();
    let seed2 = vm.create_empty_account();

    let _bar1 = vm.function("test_other", &[], &[&seed1, &seed2], 0, None);

    assert_eq!(vm.logs, "bar1 says: yo from bar0bar1 says: hi from bar0");

    vm.logs.truncate(0);
}

#[test]
fn syntax() {
    let ns = parse_and_resolve(
        r#"
        contract y {
            function f() public {
                x a = new x{gas: 102}();
            }
        }
        contract x {}
    "#,
        Target::Solana,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "‘gas’ not permitted for external calls or constructors on solana"
    );

    let ns = parse_and_resolve(
        r#"
        contract y {
            function f() public {
                x a = new x{salt: 102}();
            }
        }
        contract x {}
    "#,
        Target::Solana,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "‘salt’ not permitted for external calls or constructors on solana"
    );
}
