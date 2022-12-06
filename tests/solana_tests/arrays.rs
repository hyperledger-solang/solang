// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use num_bigint::BigInt;
use num_traits::{One, Zero};

#[test]
fn fixed_array() {
    // test that the abi encoder can handle fixed arrays
    let mut vm = build_solidity(
        r#"
        contract foo {
            function get() public returns (uint32[4] f, bytes1 g) {
                f[0] = 1;
                f[1] = 102;
                f[2] = 300331;
                f[3] = 12313231;
                g = 0xfe;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![
            BorshToken::FixedArray(vec![
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(1u8),
                },
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(102u8),
                },
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(300331u32),
                },
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(12313231u32),
                },
            ]),
            BorshToken::FixedBytes(vec!(0xfe))
        ]
    );

    // let's make it more interesting. Return some structs, some of which will be null pointers
    // when they get to the abi encoder
    let mut vm = build_solidity(
        r#"
        struct X {
            uint32 f1;
            bool f2;
        }

        contract foo {
            function get() public returns (X[4] f) {
                f[1].f1 = 102;
                f[1].f2 = true;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::FixedArray(vec![
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::zero(),
                },
                BorshToken::Bool(false)
            ]),
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(102u8),
                },
                BorshToken::Bool(true)
            ]),
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::zero(),
                },
                BorshToken::Bool(false)
            ]),
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::zero(),
                },
                BorshToken::Bool(false)
            ]),
        ])]
    );

    // Now let's try it the other way round; an struct with an array in it

    let mut vm = build_solidity(
        r#"
        struct X {
            bool f0;
            uint32[4] f1;
            bool f2;
        }

        contract foo {
            function get() public returns (X f) {
                f.f0 = true;
                f.f2 = true;
            }

            function set(X f) public returns (uint32) {
                assert(f.f0 == true);
                assert(f.f2 == true);

                uint32 sum = 0;

                for (uint32 i = 0; i < f.f1.length; i++) {
                    sum += f.f1[i];
                }

                return sum;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::Tuple(vec![
            BorshToken::Bool(true),
            BorshToken::FixedArray(vec![
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::zero(),
                },
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::zero(),
                },
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::zero(),
                },
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::zero(),
                },
            ]),
            BorshToken::Bool(true)
        ])],
    );

    let returns = vm.function(
        "set",
        &[BorshToken::Tuple(vec![
            BorshToken::Bool(true),
            BorshToken::FixedArray(vec![
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(3u8),
                },
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(5u8),
                },
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(7u8),
                },
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(11u8),
                },
            ]),
            BorshToken::Bool(true),
        ])],
    );

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 32,
            value: BigInt::from(26u8),
        }]
    );
}

#[test]
fn dynamic_array_fixed_elements() {
    // test that the abi decoder can handle fixed arrays
    let mut vm = build_solidity(
        r#"
        contract foo {
            function get(uint x, uint32[] f, uint g) public returns (uint32) {
                assert(x == 12123123);
                assert(g == 102);

                uint32 sum = 0;

                for (uint32 i = 0; i < f.length; i++) {
                    sum += f[i];
                }

                return sum;
            }

            function set() public returns (uint x, uint32[] f, string g) {
                x = 12123123;
                f = new uint32[](4);
                f[0] = 3; f[1] = 5; f[2] = 7; f[3] = 11;
                g = "abcd";
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function(
        "get",
        &[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(12123123u32),
            },
            BorshToken::Array(vec![
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(3u8),
                },
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(5u8),
                },
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(7u8),
                },
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(11u8),
                },
            ]),
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(102u8),
            },
        ],
    );

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 32,
            value: BigInt::from(26u8),
        }]
    );

    // test that the abi encoder can handle fixed arrays
    let returns = vm.function("set", &[]);

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(12123123u32),
            },
            BorshToken::Array(vec![
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(3u8),
                },
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(5u8),
                },
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(7u8),
                },
                BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(11u8),
                },
            ]),
            BorshToken::String(String::from("abcd")),
        ]
    );
}

#[test]
fn fixed_array_dynamic_elements() {
    // test that the abi decoder can handle fixed arrays
    let mut vm = build_solidity(
        r#"
        contract foo {
            function get(uint x, bytes[4] f, uint g) public returns (uint32) {
                assert(x == 12123123);
                assert(g == 102);

                uint32 sum = 0;

                for (uint32 i = 0; i < f.length; i++) {
                    for (uint32 j = 0; j < f[i].length; j++)
                        sum += f[i][j];
                }

                return sum;
            }

            function set() public returns (uint x, bytes[4] f, uint g) {
                x = 12123123;
                f[0] = hex"030507";
                f[1] = hex"0b0d11";
                f[2] = hex"1317";
                f[3] = hex"1d";
                g = 102;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function(
        "get",
        &[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(12123123u32),
            },
            BorshToken::FixedArray(vec![
                BorshToken::Bytes(vec![3, 5, 7]),
                BorshToken::Bytes(vec![11, 13, 17]),
                BorshToken::Bytes(vec![19, 23]),
                BorshToken::Bytes(vec![29]),
            ]),
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(102u8),
            },
        ],
    );

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 32,
            value: BigInt::from(127),
        }]
    );

    let returns = vm.function("set", &[]);

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(12123123u32)
            },
            BorshToken::FixedArray(vec![
                BorshToken::Bytes(vec![3, 5, 7]),
                BorshToken::Bytes(vec![11, 13, 17]),
                BorshToken::Bytes(vec![19, 23]),
                BorshToken::Bytes(vec![29]),
            ]),
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(102u8)
            },
        ]
    );
}

#[test]
fn dynamic_array_dynamic_elements() {
    // test that the abi decoder can handle fixed arrays
    let mut vm = build_solidity(
        r#"
        contract foo {
            function get(uint x, bytes[] f, uint g) public returns (uint32) {
                assert(x == 12123123);
                assert(g == 102);

                uint32 sum = 0;

                for (uint32 i = 0; i < f.length; i++) {
                    for (uint32 j = 0; j < f[i].length; j++)
                        sum += f[i][j];
                }

                return sum;
            }

            function set() public returns (uint x, bytes[] f, string g) {
                x = 12123123;
                f = new bytes[](4);
                f[0] = hex"030507";
                f[1] = hex"0b0d11";
                f[2] = hex"1317";
                f[3] = hex"1d";
                g = "feh";
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function(
        "get",
        &[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(12123123u32),
            },
            BorshToken::Array(vec![
                BorshToken::Bytes(vec![3, 5, 7]),
                BorshToken::Bytes(vec![11, 13, 17]),
                BorshToken::Bytes(vec![19, 23]),
                BorshToken::Bytes(vec![29]),
            ]),
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(102u8),
            },
        ],
    );

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 32,
            value: BigInt::from(127),
        }]
    );

    let returns = vm.function("set", &[]);

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(12123123u32)
            },
            BorshToken::Array(vec![
                BorshToken::Bytes(vec![3, 5, 7]),
                BorshToken::Bytes(vec![11, 13, 17]),
                BorshToken::Bytes(vec![19, 23]),
                BorshToken::Bytes(vec![29]),
            ]),
            BorshToken::String(String::from("feh")),
        ]
    );
}

#[test]
fn fixed_array_fixed_elements_storage() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            int64[4] store;

            function set_elem(uint index, int64 val) public {
                store[index] = val;
            }

            function get_elem(uint index) public returns (int64) {
                return store[index];
            }

            function set(int64[4] x) public {
                store = x;
            }

            function get() public returns (int64[4]) {
                return store;
            }

            function del() public {
                delete store;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    vm.function(
        "set_elem",
        &[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(2u8),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(12123123u64),
            },
        ],
    );

    vm.function(
        "set_elem",
        &[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(3u8),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(123456789u64),
            },
        ],
    );

    let returns = vm.function(
        "get_elem",
        &[BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8),
        }],
    );

    assert_eq!(
        returns,
        vec![BorshToken::Int {
            width: 64,
            value: BigInt::from(12123123u64)
        },]
    );

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::FixedArray(vec![
            BorshToken::Int {
                width: 64,
                value: BigInt::zero()
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::zero()
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(12123123u32),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(123456789u32),
            },
        ])],
    );

    vm.function(
        "set",
        &[BorshToken::FixedArray(vec![
            BorshToken::Int {
                width: 64,
                value: BigInt::one(),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(2u8),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(3u8),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(4u8),
            },
        ])],
    );

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::FixedArray(vec![
            BorshToken::Int {
                width: 64,
                value: BigInt::one(),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(2u8),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(3u8),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(4u8),
            },
        ])],
    );

    vm.function("del", &[]);

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::FixedArray(vec![
            BorshToken::Int {
                width: 64,
                value: BigInt::zero(),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::zero(),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::zero(),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::zero(),
            },
        ])],
    );
}

#[test]
fn fixed_array_dynamic_elements_storage() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            string[4] store;

            function set_elem(uint index, string val) public {
                store[index] = val;
            }

            function get_elem(uint index) public returns (string) {
                return store[index];
            }

            function set(string[4] x) public {
                store = x;
            }

            function get() public returns (string[4]) {
                return store;
            }

            function del() public {
                delete store;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    vm.function(
        "set_elem",
        &[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(2u8),
            },
            BorshToken::String(String::from("abcd")),
        ],
    );

    vm.function(
        "set_elem",
        &[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(3u8),
            },
            BorshToken::String(String::from(
                "you can lead a horse to water but you can’t make him drink",
            )),
        ],
    );

    let returns = vm.function(
        "get_elem",
        &[BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8),
        }],
    );

    assert_eq!(returns, vec![BorshToken::String(String::from("abcd"))]);

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::FixedArray(vec![
            BorshToken::String(String::from("")),
            BorshToken::String(String::from("")),
            BorshToken::String(String::from("abcd")),
            BorshToken::String(String::from(
                "you can lead a horse to water but you can’t make him drink"
            )),
        ])],
    );

    vm.function(
        "set",
        &[BorshToken::FixedArray(vec![
            BorshToken::String(String::from("a")),
            BorshToken::String(String::from("b")),
            BorshToken::String(String::from("c")),
            BorshToken::String(String::from("d")),
        ])],
    );

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::FixedArray(vec![
            BorshToken::String(String::from("a")),
            BorshToken::String(String::from("b")),
            BorshToken::String(String::from("c")),
            BorshToken::String(String::from("d")),
        ])],
    );

    vm.function("del", &[]);

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::FixedArray(vec![
            BorshToken::String(String::from("")),
            BorshToken::String(String::from("")),
            BorshToken::String(String::from("")),
            BorshToken::String(String::from("")),
        ])],
    );
}

#[test]
fn storage_simple_dynamic_array() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            int64[] store;

            function push(int64 x) public {
                store.push(x);
            }

            function push_zero() public {
                store.push();
            }

            function pop() public returns (int64) {
                return store.pop();
            }

            function len() public returns (uint) {
                return store.length;
            }

            function subscript(uint32 i) public returns (int64) {
                return store[i];
            }

            function copy() public returns (int64[] memory) {
                return store;
            }

            function set(int64[] n) public {
                store = n;
            }

            function rm() public {
                delete store;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("len", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::zero(),
        }]
    );

    vm.function(
        "push",
        &[BorshToken::Int {
            width: 64,
            value: BigInt::from(102u8),
        }],
    );

    vm.function("push_zero", &[]);

    vm.function(
        "push",
        &[BorshToken::Int {
            width: 64,
            value: BigInt::from(12345678901u64),
        }],
    );

    let returns = vm.function(
        "subscript",
        &[BorshToken::Uint {
            width: 32,
            value: BigInt::zero(),
        }],
    );

    assert_eq!(
        returns,
        vec![BorshToken::Int {
            width: 64,
            value: BigInt::from(102u8),
        }]
    );

    let returns = vm.function(
        "subscript",
        &[BorshToken::Uint {
            width: 32,
            value: BigInt::one(),
        }],
    );

    assert_eq!(
        returns,
        vec![BorshToken::Int {
            width: 64,
            value: BigInt::zero()
        }]
    );

    let returns = vm.function(
        "subscript",
        &[BorshToken::Uint {
            width: 32,
            value: BigInt::from(2u8),
        }],
    );

    assert_eq!(
        returns,
        vec![BorshToken::Int {
            width: 64,
            value: BigInt::from(12345678901u64),
        },]
    );

    let returns = vm.function("copy", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::Array(vec![
            BorshToken::Int {
                width: 64,
                value: BigInt::from(102u8),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::zero(),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(12345678901u64),
            },
        ])],
    );

    let returns = vm.function("pop", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::Int {
            width: 64,
            value: BigInt::from(12345678901u64),
        },]
    );

    let returns = vm.function("len", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8),
        }]
    );

    vm.function(
        "set",
        &[BorshToken::Array(vec![
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(1u8),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(2u8),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(3u8),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(4u8),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(5u8),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(6u8),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(7u8),
            },
        ])],
    );

    let returns = vm.function("copy", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::Array(vec![
            BorshToken::Int {
                width: 64,
                value: BigInt::from(1u8)
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(2u8)
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(3u8)
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(4u8)
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(5u8)
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(6u8)
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(7u8)
            },
        ])],
    );

    vm.function("rm", &[]);

    let returns = vm.function("len", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::zero(),
        }]
    );
}

#[test]
#[should_panic]
fn storage_pop_running_on_empty() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            int64[] store;

            function pop() public returns (int64) {
                return store.pop();
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    vm.function("pop", &[]);
}

#[test]
fn storage_dynamic_array_of_structs() {
    let mut vm = build_solidity(
        r#"
        struct S {
            uint64 f1;
            bool f2;
        }

        contract foo {
            S[] store;

            function push1(S x) public {
                store.push(x);
            }

            function push2(S x) public {
                S storage f = store.push();
                f.f1 = x.f1;
                f.f2 = x.f2;
            }

            function push_empty() public {
                store.push();
            }

            function pop() public returns (S) {
                return store.pop();
            }

            function len() public returns (uint) {
                return store.length;
            }

            function subscript(uint32 i) public returns (S) {
                return store[i];
            }

            function copy() public returns (S[] memory) {
                return store;
            }

            function set(S[] memory n) public {
                store = n;
            }

            function rm() public {
                delete store;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("len", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::zero(),
        }]
    );

    vm.function(
        "push1",
        &[BorshToken::Tuple(vec![
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(13819038012u64),
            },
            BorshToken::Bool(true),
        ])],
    );

    vm.function("push_empty", &[]);

    vm.function(
        "push2",
        &[BorshToken::Tuple(vec![
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(12313123141123213u64),
            },
            BorshToken::Bool(true),
        ])],
    );

    let returns = vm.function(
        "subscript",
        &[BorshToken::Uint {
            width: 32,
            value: BigInt::zero(),
        }],
    );

    assert_eq!(
        returns,
        vec![BorshToken::Tuple(vec![
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(13819038012u64)
            },
            BorshToken::Bool(true),
        ])]
    );

    let returns = vm.function(
        "subscript",
        &[BorshToken::Uint {
            width: 32,
            value: BigInt::one(),
        }],
    );

    assert_eq!(
        returns,
        vec![BorshToken::Tuple(vec![
            BorshToken::Uint {
                width: 64,
                value: BigInt::zero(),
            },
            BorshToken::Bool(false),
        ])]
    );

    let returns = vm.function(
        "subscript",
        &[BorshToken::Uint {
            width: 32,
            value: BigInt::from(2u8),
        }],
    );

    assert_eq!(
        returns,
        vec![BorshToken::Tuple(vec![
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(12313123141123213u64),
            },
            BorshToken::Bool(true),
        ]),]
    );

    let returns = vm.function("copy", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::Array(vec![
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(13819038012u64)
                },
                BorshToken::Bool(true),
            ]),
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::zero(),
                },
                BorshToken::Bool(false),
            ]),
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(12313123141123213u64),
                },
                BorshToken::Bool(true),
            ]),
        ])]
    );

    let returns = vm.function("pop", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::Tuple(vec![
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(12313123141123213u64),
            },
            BorshToken::Bool(true),
        ])]
    );

    let returns = vm.function("len", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8),
        }]
    );

    vm.function(
        "set",
        &[BorshToken::Array(vec![
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::one(),
                },
                BorshToken::Bool(false),
            ]),
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(2u8),
                },
                BorshToken::Bool(true),
            ]),
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(3u8),
                },
                BorshToken::Bool(false),
            ]),
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(4u8),
                },
                BorshToken::Bool(true),
            ]),
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(5u8),
                },
                BorshToken::Bool(false),
            ]),
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(6u8),
                },
                BorshToken::Bool(true),
            ]),
        ])],
    );

    let returns = vm.function("copy", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::Array(vec![
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::one(),
                },
                BorshToken::Bool(false)
            ]),
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(2u8),
                },
                BorshToken::Bool(true)
            ]),
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(3u8),
                },
                BorshToken::Bool(false)
            ]),
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(4u8),
                },
                BorshToken::Bool(true)
            ]),
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(5u8),
                },
                BorshToken::Bool(false)
            ]),
            BorshToken::Tuple(vec![
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(6u8),
                },
                BorshToken::Bool(true)
            ]),
        ]),]
    );

    vm.function("rm", &[]);

    let returns = vm.function("len", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::zero(),
        }]
    );
}

#[test]
fn array_literal() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            int64 constant foo = 1;
            int64 bar = 2;

            function list() public returns (int64[3]) {
                return [foo, bar, 3];
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("list", &[]);

    assert_eq!(
        returns,
        vec![BorshToken::FixedArray(vec![
            BorshToken::Int {
                width: 64,
                value: BigInt::one(),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(2u8),
            },
            BorshToken::Int {
                width: 64,
                value: BigInt::from(3u8),
            },
        ])]
    );
}

#[test]
fn storage_pop_push() {
    let mut vm = build_solidity(
        r#"
    contract Testing {
        struct NonConstantStruct {
            string[] b;
        }

        string[] vec_2;
        NonConstantStruct[] public complex_array;

        function fn1() public {
            vec_2.push("tea");
        }

        function fn2() public {
            vec_2.push("coffee");
        }

        function fn3() public {
            NonConstantStruct memory ss = NonConstantStruct(vec_2);
            complex_array.push(ss);
        }

        function fn4() public {
            vec_2.pop();
        }

        function fn5() public {
            vec_2.pop();
        }

        function fn6() public {
            vec_2.push("cortado");
        }

        function fn7() public {
            vec_2.push("cappuccino");
        }

        function fn8() public {
            NonConstantStruct memory sr = NonConstantStruct(vec_2);
            complex_array.push(sr);
        }

        function clear() public {
            vec_2 = new string[](0);
            complex_array = new NonConstantStruct[](0);
        }
    }"#,
    );

    vm.constructor("Testing", &[]);
    vm.function("fn1", &[]);
    vm.function("fn2", &[]);
    vm.function("fn3", &[]);
    vm.function("fn4", &[]);
    vm.function("fn5", &[]);
    vm.function("fn6", &[]);
    vm.function("fn7", &[]);
    vm.function("fn8", &[]);
    vm.function("clear", &[]);

    // make sure every thing has been freed
    assert_eq!(vm.validate_account_data_heap(), 0);
}

#[test]
fn initialization_with_literal() {
    let mut vm = build_solidity(
        r#"
        contract Testing {
            address[] splitAddresses;

            function split(address addr1, address addr2) public {
                splitAddresses = [addr1, addr2];
            }

            function getIdx(uint32 idx) public view returns (address) {
                return splitAddresses[idx];
            }

            function getVec(uint32 a, uint32 b) public pure returns (uint32[] memory) {
                uint32[] memory vec;
                vec = [a, b];
                return vec;
            }
        }
        "#,
    );

    vm.constructor("Testing", &[]);

    let mut addr1: Vec<u8> = Vec::new();
    addr1.resize(32, 0);
    addr1[0] = 1;
    let mut addr2: Vec<u8> = Vec::new();
    addr2.resize(32, 0);
    addr2[0] = 2;
    let _ = vm.function(
        "split",
        &[
            BorshToken::FixedBytes(addr1[..].to_vec()),
            BorshToken::FixedBytes(addr2[..].to_vec()),
        ],
    );
    let returns = vm.function(
        "getIdx",
        &[BorshToken::Uint {
            width: 32,
            value: BigInt::zero(),
        }],
    );
    let returned_addr1 = returns[0].clone().into_fixed_bytes().unwrap();
    assert_eq!(addr1, returned_addr1);

    let returns = vm.function(
        "getIdx",
        &[BorshToken::Uint {
            width: 32,
            value: BigInt::one(),
        }],
    );
    let returned_addr2 = returns[0].clone().into_fixed_bytes().unwrap();
    assert_eq!(addr2, returned_addr2);

    let returns = vm.function(
        "getVec",
        &[
            BorshToken::Uint {
                width: 32,
                value: BigInt::from(563u16),
            },
            BorshToken::Uint {
                width: 32,
                value: BigInt::from(895u16),
            },
        ],
    );
    let array = returns[0].clone().into_array().unwrap();
    assert_eq!(
        array,
        vec![
            BorshToken::Uint {
                width: 32,
                value: BigInt::from(563u16),
            },
            BorshToken::Uint {
                width: 32,
                value: BigInt::from(895u16),
            },
        ]
    );
}

#[test]
fn dynamic_array_push() {
    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            function test() public {
                int[] bar = (new int[])(1);

                bar[0] = 128;
                bar.push(64);

                assert(bar.length == 2);
                assert(bar[1] == 64);
            }
        }
        "#,
    );

    runtime.constructor("foo", &[]);
    runtime.function("test", &[]);

    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            function test() public {
                bytes bar = (new bytes)(1);

                bar[0] = 128;
                bar.push(64);

                assert(bar.length == 2);
                assert(bar[1] == 64);
            }
        }
        "#,
    );

    runtime.constructor("foo", &[]);
    runtime.function("test", &[]);

    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            struct s {
                int32 f1;
                bool f2;
            }
            function test() public {
                s[] bar = new s[](1);

                bar[0] = s({f1: 0, f2: false});
                bar.push(s({f1: 1, f2: true}));

                assert(bar.length == 2);
                assert(bar[1].f1 == 1);
                assert(bar[1].f2 == true);
            }
        }
        "#,
    );

    runtime.constructor("foo", &[]);
    runtime.function("test", &[]);

    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            enum enum1 { val1, val2, val3 }
            function test() public {
                enum1[] bar = new enum1[](1);

                bar[0] = enum1.val1;
                bar.push(enum1.val2);

                assert(bar.length == 2);
                assert(bar[1] == enum1.val2);
            }
        }
        "#,
    );

    runtime.constructor("foo", &[]);
    runtime.function("test", &[]);

    // push() returns a reference to the thing
    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            struct s {
                int32 f1;
                bool f2;
            }

            function test() public {
                s[] bar = new s[](0);
                s memory n = bar.push();
                n.f1 = 102;
                n.f2 = true;

                assert(bar[0].f1 == 102);
                assert(bar[0].f2 == true);
            }
        }"#,
    );

    runtime.constructor("foo", &[]);
    runtime.function("test", &[]);
}

#[test]
fn dynamic_array_pop() {
    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            function test() public {
                int[] bar = new int[](1);

                bar[0] = 128;

                assert(bar.length == 1);
                assert(128 == bar.pop());
                assert(bar.length == 0);
            }
        }
        "#,
    );

    runtime.constructor("foo", &[]);
    runtime.function("test", &[]);

    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            function test() public {
                bytes bar = new bytes(1);

                bar[0] = 128;

                assert(bar.length == 1);
                assert(128 == bar.pop());
                assert(bar.length == 0);
            }
        }
        "#,
    );

    runtime.constructor("foo", &[]);
    runtime.function("test", &[]);

    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            struct s {
                int32 f1;
                bool f2;
            }
            function test() public {
                s[] bar = new s[](1);

                bar[0] = s(128, true);

                assert(bar.length == 1);

                s baz = bar.pop();
                assert(baz.f1 == 128);
                assert(baz.f2 == true);
                assert(bar.length == 0);
            }
        }
        "#,
    );

    runtime.constructor("foo", &[]);
    runtime.function("test", &[]);

    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            enum enum1 { val1, val2, val3 }
            function test() public {
                enum1[] bar = new enum1[](1);

                bar[0] = enum1.val2;

                assert(bar.length == 1);
                assert(enum1.val2 == bar.pop());
                assert(bar.length == 0);
            }
        }
        "#,
    );

    runtime.constructor("foo", &[]);
    runtime.function("test", &[]);
}

#[test]
#[should_panic]
fn dynamic_array_pop_empty_array() {
    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            function test() public {
                int[] bar = new int[](0);
                bar.pop();
            }
        }"#,
    );

    runtime.constructor("foo", &[]);
    runtime.function("test", &[]);
}

#[test]
#[should_panic]
fn dynamic_array_pop_bounds() {
    let mut runtime = build_solidity(
        r#"
        pragma solidity 0;

        contract foo {
            function test() public {
                int[] bar = new int[](1);
                bar[0] = 12;
                bar.pop();

                assert(bar[0] == 12);
            }
        }"#,
    );

    runtime.constructor("foo", &[]);
    runtime.function("test", &[]);
}

#[test]
fn dynamic_array_push_pop_loop() {
    let mut runtime = build_solidity(
        r#"
        contract foo {
            function test() public {
                uint32[] bar1 = new uint32[](0);
                uint32[] bar2 = new uint32[](0);

                // each time we call a system call, the heap is checked
                // for consistency. So do a print() after each operation
                for (uint64 i = 1; i < 160; i++) {
                    if ((i % 10) == 0) {
                        bar1.pop();
                        print("bar1.pop");
                        bar2.pop();
                        print("bar2.pop");
                    } else {
                        uint32 v = bar1.length;
                        bar1.push(v);
                        print("bar1.push");
                        bar2.push(v);
                        print("bar2.push");
                    }
                }

                assert(bar1.length == bar2.length);

                for (uint32 i = 0; i < bar1.length; i++) {
                    assert(bar1[i] == i);
                    assert(bar2[i] == i);
                }
            }
        }"#,
    );

    runtime.constructor("foo", &[]);
    runtime.function("test", &[]);

    let mut runtime = build_solidity(
        r#"
        contract foo {
            function test() public {
                bytes bar1 = new bytes(0);
                bytes bar2 = new bytes(0);

                // each time we call a system call, the heap is checked
                // for consistency. So do a print() after each operation
                for (uint64 i = 1; i < 160; i++) {
                    if ((i % 10) == 0) {
                        bar1.pop();
                        print("bar1.pop");
                        bar2.pop();
                        print("bar2.pop");
                    } else {
                        uint8 v = uint8(bar1.length);
                        bar1.push(v);
                        print("bar1.push");
                        bar2.push(v);
                        print("bar2.push");
                    }
                }

                assert(bar1.length == bar2.length);

                for (uint32 i = 0; i < bar1.length; i++) {
                    uint8 v = uint8(i);
                    print("{}.{}.{}".format(v, bar1[i], bar2[i]));
                    assert(bar1[i] == v);
                    assert(bar2[i] == v);
                }
            }
        }"#,
    );

    runtime.constructor("foo", &[]);
    runtime.function("test", &[]);
}
