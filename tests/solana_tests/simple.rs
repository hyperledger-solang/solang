// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use num_bigint::BigInt;
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

    vm.constructor("foo", &[]);

    assert_eq!(vm.logs, "Hello from constructor");

    vm.logs.truncate(0);

    vm.function("test", &[], None);

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

    vm.constructor("foo", &[]);

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

    vm.constructor("foo", &[]);

    vm.function(
        "test",
        &[
            BorshToken::Uint {
                width: 32,
                value: BigInt::from(10u8),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(10u8),
            },
        ],
        None,
    );

    assert_eq!(vm.logs, "x is 10");

    vm.logs.truncate(0);

    vm.function(
        "test",
        &[
            BorshToken::Uint {
                width: 32,
                value: BigInt::from(99u8),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(102u8),
            },
        ],
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

    vm.constructor("foo", &[]);

    let returns = vm.function(
        "test",
        &[BorshToken::Uint {
            width: 32,
            value: BigInt::from(10u8),
        }],
        None,
    );

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 32,
            value: BigInt::from(100u8)
        },]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test(uint64 x) public returns (bool, uint64) {
                return (true, x * 961748941);
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function(
        "test",
        &[BorshToken::Uint {
            width: 64,
            value: BigInt::from(982451653u64),
        }],
        None,
    );

    assert_eq!(
        returns,
        vec![
            BorshToken::Bool(true),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(961748941u64 * 982451653u64)
            },
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

    vm.constructor("flipper", &[BorshToken::Bool(true)]);

    assert_eq!(
        vm.data()[0..17].to_vec(),
        hex::decode("6fc90ec500000000000000001800000001").unwrap()
    );

    let returns = vm.function("get", &[], None);

    assert_eq!(returns, vec![BorshToken::Bool(true)]);

    vm.function("flip", &[], None);

    assert_eq!(
        vm.data()[0..17].to_vec(),
        hex::decode("6fc90ec500000000000000001800000000").unwrap()
    );

    let returns = vm.function("get", &[], None);

    assert_eq!(returns, vec![BorshToken::Bool(false)]);
}

#[test]
fn incrementer() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            // make sure incrementer has a base contract with an empty constructor
            // is to check that the correct constructor is selected at emit time
            // https://github.com/hyperledger/solang/issues/487
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

    vm.constructor(
        "incrementer",
        &[BorshToken::Uint {
            width: 32,
            value: BigInt::from(5u8),
        }],
    );

    let returns = vm.function("get", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 32,
            value: BigInt::from(5u8),
        }]
    );

    vm.function(
        "inc",
        &[BorshToken::Uint {
            width: 32,
            value: BigInt::from(5u8),
        }],
        None,
    );

    let returns = vm.function("get", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 32,
            value: BigInt::from(10u8),
        }]
    );
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

    ns.print_diagnostics_in_plain(&cache, false);

    assert_eq!(
        ns.diagnostics.iter().nth(1).unwrap().message,
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
                    unchecked {
                    array1.push((i*uint(sha256("i"))));
                    array2.push(((i+1)*uint(sha256("i"))));
                    }
               }
            }
        }"#,
    );

    vm.constructor("two_arrays", &[]);
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

    vm.constructor("deadstorage", &[]);

    let returns = vm.function("v", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(9991u16)
        }]
    );
}

#[test]
fn simple_loops() {
    let mut runtime = build_solidity(
        r##"
contract test3 {
	function foo(uint32 a) public returns (uint32) {
		uint32 b = 50 - a;
		uint32 c;
		c = 100 * b;
		c += 5;
		return a * 1000 + c;
	}

	function bar(uint32 b, bool x) public returns (uint32) {
		uint32 i = 1;
		if (x) {
			do {
				i += 10;
			}
			while (b-- > 0);
		} else {
			uint32 j;
			for (j=2; j<10; j++) {
				i *= 3;
			}
		}
		return i;
	}

	function baz(uint32 x) public returns (uint32) {
		for (uint32 i = 0; i<100; i++) {
			x *= 7;

			if (x > 200) {
				break;
			}

			x++;
		}

		return x;
	}
}"##,
    );

    // call constructor
    runtime.constructor("test3", &[]);

    for i in 0..=50 {
        let res = ((50 - i) * 100 + 5) + i * 1000;

        let returns = runtime.function(
            "foo",
            &[BorshToken::Uint {
                width: 32,
                value: BigInt::from(i),
            }],
            None,
        );

        assert_eq!(
            returns,
            vec![BorshToken::Uint {
                width: 32,
                value: BigInt::from(res)
            }]
        );
    }

    for i in 0..=50 {
        let res = (i + 1) * 10 + 1;

        let returns = runtime.function(
            "bar",
            &[
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(i),
                },
                BorshToken::Bool(true),
            ],
            None,
        );

        assert_eq!(
            returns,
            vec![BorshToken::Uint {
                width: 32,
                value: BigInt::from(res)
            }]
        );
    }

    for i in 0..=50 {
        let mut res = 1;

        for _ in 2..10 {
            res *= 3;
        }

        let returns = runtime.function(
            "bar",
            &[
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(i),
                },
                BorshToken::Bool(false),
            ],
            None,
        );

        assert_eq!(
            returns,
            vec![BorshToken::Uint {
                width: 32,
                value: BigInt::from(res)
            }]
        );
    }

    for i in 1..=50 {
        let mut res = i;

        for _ in 0..100 {
            res *= 7;
            if res > 200 {
                break;
            }
            res += 1;
        }

        let returns = runtime.function(
            "baz",
            &[BorshToken::Uint {
                width: 32,
                value: BigInt::from(i),
            }],
            None,
        );

        assert_eq!(
            returns,
            vec![BorshToken::Uint {
                width: 32,
                value: BigInt::from(res)
            }]
        );
    }
}
