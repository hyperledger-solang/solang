use crate::build_solidity;
use ethabi::ethereum_types::U256;
use solang::{file_resolver::FileResolver, Target};
use std::ffi::OsStr;

#[test]
fn simple() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            constructor() {
                print("Hello from constructor");
            }

            function test() public {
                print("Hello from function");
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    assert_eq!(vm.logs, "Hello from constructor");

    vm.logs.truncate(0);

    vm.function("test", &[], &[], 0, None);

    assert_eq!(vm.logs, "Hello from function");
}

#[test]
fn format() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            constructor() {
                int x = 21847450052839212624230656502990235142567050104912751880812823948662932355201;

                print("x = {}".format(x));
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    assert_eq!(
        vm.logs,
        "x = 21847450052839212624230656502990235142567050104912751880812823948662932355201"
    );
}

#[test]
fn parameters() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test(uint32 x, uint64 y) public {
                if (x == 10) {
                    print("x is 10");
                }

                if (y == 102) {
                    print("y is 102");
                }
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    vm.function(
        "test",
        &[
            ethabi::Token::Uint(U256::from(10)),
            ethabi::Token::Uint(U256::from(10)),
        ],
        &[],
        0,
        None,
    );

    assert_eq!(vm.logs, "x is 10");

    vm.logs.truncate(0);

    vm.function(
        "test",
        &[
            ethabi::Token::Uint(U256::from(99)),
            ethabi::Token::Uint(U256::from(102)),
        ],
        &[],
        0,
        None,
    );

    assert_eq!(vm.logs, "y is 102");
}

#[test]
fn returns() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test(uint32 x) public returns (uint32) {
                return x * x;
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function("test", &[ethabi::Token::Uint(U256::from(10))], &[], 0, None);

    assert_eq!(returns, vec![ethabi::Token::Uint(U256::from(100))]);

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test(uint64 x) public returns (bool, uint64) {
                return (true, x * 961748941);
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function(
        "test",
        &[ethabi::Token::Uint(U256::from(982451653))],
        &[],
        0,
        None,
    );

    assert_eq!(
        returns,
        vec![
            ethabi::Token::Bool(true),
            ethabi::Token::Uint(U256::from(961748941u64 * 982451653u64))
        ]
    );
}

#[test]
fn flipper() {
    let mut vm = build_solidity(
        r#"
        contract flipper {
            bool private value;

            /// Constructor that initializes the `bool` value to the given `init_value`.
            constructor(bool initvalue) {
                value = initvalue;
            }

            /// A message that can be called on instantiated contracts.
            /// This one flips the value of the stored `bool` from `true`
            /// to `false` and vice versa.
            function flip() public {
                value = !value;
            }

            /// Simply returns the current value of our `bool`.
            function get() public view returns (bool) {
                return value;
            }
        }"#,
    );

    vm.constructor("flipper", &[ethabi::Token::Bool(true)], 0);

    assert_eq!(
        vm.data()[0..17].to_vec(),
        hex::decode("6fc90ec500000000000000001800000001").unwrap()
    );

    let returns = vm.function("get", &[], &[], 0, None);

    assert_eq!(returns, vec![ethabi::Token::Bool(true)]);

    vm.function("flip", &[], &[], 0, None);

    assert_eq!(
        vm.data()[0..17].to_vec(),
        hex::decode("6fc90ec500000000000000001800000000").unwrap()
    );

    let returns = vm.function("get", &[], &[], 0, None);

    assert_eq!(returns, vec![ethabi::Token::Bool(false)]);
}

#[test]
fn incrementer() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            // make sure incrementer has a base contract with an empty constructor
            // is to check that the correct constructor is selected at emit time
            // https://github.com/hyperledger-labs/solang/issues/487
            constructor() {}
        }

        contract incrementer is foo {
            uint32 private value;

            /// Constructor that initializes the `int32` value to the given `init_value`.
            constructor(uint32 initvalue) {
                value = initvalue;
            }

            /// This increments the value by `by`.
            function inc(uint32 by) public {
                value += by;
            }

            /// Simply returns the current value of our `uint32`.
            function get() public view returns (uint32) {
                return value;
            }
        }"#,
    );

    vm.constructor("incrementer", &[ethabi::Token::Uint(U256::from(5))], 0);

    let returns = vm.function("get", &[], &[], 0, None);

    assert_eq!(returns, vec![ethabi::Token::Uint(U256::from(5))]);

    vm.function("inc", &[ethabi::Token::Uint(U256::from(7))], &[], 0, None);

    let returns = vm.function("get", &[], &[], 0, None);

    assert_eq!(returns, vec![ethabi::Token::Uint(U256::from(12))]);
}

#[test]
fn infinite_loop() {
    let mut cache = FileResolver::new();

    let src = String::from(
        r#"
contract line {
    function foo() public {
        address x = int32(1);
    }
}"#,
    );

    cache.set_file_contents("test.sol", src);

    let ns = solang::parse_and_resolve(OsStr::new("test.sol"), &mut cache, Target::Solana);

    solang::sema::diagnostics::print_diagnostics_plain(&cache, &ns, false);

    assert_eq!(
        ns.diagnostics[1].message,
        "implicit conversion from int to address not allowed"
    );
}

#[test]
fn two_arrays() {
    let mut vm = build_solidity(
        r#"
        contract two_arrays {
            uint[] array1;
            uint[] array2;

            constructor() {
                for(uint i = 0; i < 10; i++) {
                    array1.push((i*uint(sha256("i"))));
                    array2.push(((i+1)*uint(sha256("i"))));
               }
            }
        }"#,
    );

    vm.constructor("two_arrays", &[], 0);
}

#[test]
fn dead_storage_bug() {
    let mut vm = build_solidity(
        r#"
        contract deadstorage {
            uint public maxlen = 10000;
            uint public z;
            uint public v;

            constructor() {
                for(uint i = 0; i < 10; i++) {
                    uint x = i*(10e34+9999);
                    print("x:{}".format(x));
                    v = x%maxlen;
                    print("v:{}".format(v));
                    z = v%maxlen;
                    print("z:{}".format(z));
               }
            }
        }"#,
    );

    vm.constructor("deadstorage", &[], 0);

    let returns = vm.function("v", &[], &[], 0, None);

    assert_eq!(returns, vec![ethabi::Token::Uint(U256::from(9991))]);
}
