use crate::build_solidity;
use ethabi::Token;

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
            Token::FixedArray(vec![
                Token::Uint(ethereum_types::U256::from(1)),
                Token::Uint(ethereum_types::U256::from(102)),
                Token::Uint(ethereum_types::U256::from(300331)),
                Token::Uint(ethereum_types::U256::from(12313231))
            ]),
            Token::FixedBytes(vec!(0xfe))
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
        vec![Token::FixedArray(vec![
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(0)),
                Token::Bool(false)
            ]),
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(102)),
                Token::Bool(true)
            ]),
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(0)),
                Token::Bool(false)
            ]),
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(0)),
                Token::Bool(false)
            ])
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
        vec![Token::Tuple(vec![
            Token::Bool(true),
            Token::FixedArray(vec![
                Token::Uint(ethereum_types::U256::from(0)),
                Token::Uint(ethereum_types::U256::from(0)),
                Token::Uint(ethereum_types::U256::from(0)),
                Token::Uint(ethereum_types::U256::from(0)),
            ]),
            Token::Bool(true)
        ])],
    );

    let returns = vm.function(
        "set",
        &[Token::Tuple(vec![
            Token::Bool(true),
            Token::FixedArray(vec![
                Token::Uint(ethereum_types::U256::from(3)),
                Token::Uint(ethereum_types::U256::from(5)),
                Token::Uint(ethereum_types::U256::from(7)),
                Token::Uint(ethereum_types::U256::from(11)),
            ]),
            Token::Bool(true),
        ])],
    );

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(26))]);
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
            Token::Uint(ethereum_types::U256::from(12123123)),
            Token::Array(vec![
                Token::Uint(ethereum_types::U256::from(3)),
                Token::Uint(ethereum_types::U256::from(5)),
                Token::Uint(ethereum_types::U256::from(7)),
                Token::Uint(ethereum_types::U256::from(11)),
            ]),
            Token::Uint(ethereum_types::U256::from(102)),
        ],
    );

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(26))]);

    // test that the abi encoder can handle fixed arrays
    let returns = vm.function("set", &[]);

    assert_eq!(
        returns,
        vec![
            Token::Uint(ethereum_types::U256::from(12123123)),
            Token::Array(vec![
                Token::Uint(ethereum_types::U256::from(3)),
                Token::Uint(ethereum_types::U256::from(5)),
                Token::Uint(ethereum_types::U256::from(7)),
                Token::Uint(ethereum_types::U256::from(11)),
            ]),
            Token::String(String::from("abcd")),
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
            Token::Uint(ethereum_types::U256::from(12123123)),
            Token::FixedArray(vec![
                Token::Bytes(vec![3, 5, 7]),
                Token::Bytes(vec![11, 13, 17]),
                Token::Bytes(vec![19, 23]),
                Token::Bytes(vec![29]),
            ]),
            Token::Uint(ethereum_types::U256::from(102)),
        ],
    );

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(127))]);

    let returns = vm.function("set", &[]);

    assert_eq!(
        returns,
        vec![
            Token::Uint(ethereum_types::U256::from(12123123)),
            Token::FixedArray(vec![
                Token::Bytes(vec![3, 5, 7]),
                Token::Bytes(vec![11, 13, 17]),
                Token::Bytes(vec![19, 23]),
                Token::Bytes(vec![29]),
            ]),
            Token::Uint(ethereum_types::U256::from(102)),
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
            Token::Uint(ethereum_types::U256::from(12123123)),
            Token::Array(vec![
                Token::Bytes(vec![3, 5, 7]),
                Token::Bytes(vec![11, 13, 17]),
                Token::Bytes(vec![19, 23]),
                Token::Bytes(vec![29]),
            ]),
            Token::Uint(ethereum_types::U256::from(102)),
        ],
    );

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(127))]);

    let returns = vm.function("set", &[]);

    assert_eq!(
        returns,
        vec![
            Token::Uint(ethereum_types::U256::from(12123123)),
            Token::Array(vec![
                Token::Bytes(vec![3, 5, 7]),
                Token::Bytes(vec![11, 13, 17]),
                Token::Bytes(vec![19, 23]),
                Token::Bytes(vec![29]),
            ]),
            Token::String(String::from("feh")),
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
            Token::Uint(ethereum_types::U256::from(2)),
            Token::Int(ethereum_types::U256::from(12123123)),
        ],
    );

    vm.function(
        "set_elem",
        &[
            Token::Uint(ethereum_types::U256::from(3)),
            Token::Int(ethereum_types::U256::from(123456789)),
        ],
    );

    let returns = vm.function("get_elem", &[Token::Uint(ethereum_types::U256::from(2))]);

    assert_eq!(
        returns,
        vec![Token::Int(ethereum_types::U256::from(12123123)),],
    );

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![Token::FixedArray(vec![
            Token::Int(ethereum_types::U256::from(0)),
            Token::Int(ethereum_types::U256::from(0)),
            Token::Int(ethereum_types::U256::from(12123123)),
            Token::Int(ethereum_types::U256::from(123456789)),
        ]),],
    );

    vm.function(
        "set",
        &[Token::FixedArray(vec![
            Token::Int(ethereum_types::U256::from(1)),
            Token::Int(ethereum_types::U256::from(2)),
            Token::Int(ethereum_types::U256::from(3)),
            Token::Int(ethereum_types::U256::from(4)),
        ])],
    );

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![Token::FixedArray(vec![
            Token::Int(ethereum_types::U256::from(1)),
            Token::Int(ethereum_types::U256::from(2)),
            Token::Int(ethereum_types::U256::from(3)),
            Token::Int(ethereum_types::U256::from(4)),
        ]),],
    );

    vm.function("del", &[]);

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![Token::FixedArray(vec![
            Token::Int(ethereum_types::U256::from(0)),
            Token::Int(ethereum_types::U256::from(0)),
            Token::Int(ethereum_types::U256::from(0)),
            Token::Int(ethereum_types::U256::from(0)),
        ]),],
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
            Token::Uint(ethereum_types::U256::from(2)),
            Token::String(String::from("abcd")),
        ],
    );

    vm.function(
        "set_elem",
        &[
            Token::Uint(ethereum_types::U256::from(3)),
            Token::String(String::from(
                "you can lead a horse to water but you can’t make him drink",
            )),
        ],
    );

    let returns = vm.function("get_elem", &[Token::Uint(ethereum_types::U256::from(2))]);

    assert_eq!(returns, vec![Token::String(String::from("abcd"))]);

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![Token::FixedArray(vec![
            Token::String(String::from("")),
            Token::String(String::from("")),
            Token::String(String::from("abcd")),
            Token::String(String::from(
                "you can lead a horse to water but you can’t make him drink"
            )),
        ]),],
    );

    vm.function(
        "set",
        &[Token::FixedArray(vec![
            Token::String(String::from("a")),
            Token::String(String::from("b")),
            Token::String(String::from("c")),
            Token::String(String::from("d")),
        ])],
    );

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![Token::FixedArray(vec![
            Token::String(String::from("a")),
            Token::String(String::from("b")),
            Token::String(String::from("c")),
            Token::String(String::from("d")),
        ]),],
    );

    vm.function("del", &[]);

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![Token::FixedArray(vec![
            Token::String(String::from("")),
            Token::String(String::from("")),
            Token::String(String::from("")),
            Token::String(String::from("")),
        ]),],
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

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(0))]);

    vm.function("push", &[Token::Int(ethereum_types::U256::from(102))]);

    vm.function("push_zero", &[]);

    vm.function(
        "push",
        &[Token::Int(ethereum_types::U256::from(12345678901u64))],
    );

    let returns = vm.function("subscript", &[Token::Uint(ethereum_types::U256::from(0))]);

    assert_eq!(returns, vec![Token::Int(ethereum_types::U256::from(102))]);

    let returns = vm.function("subscript", &[Token::Uint(ethereum_types::U256::from(1))]);

    assert_eq!(returns, vec![Token::Int(ethereum_types::U256::from(0))]);

    let returns = vm.function("subscript", &[Token::Uint(ethereum_types::U256::from(2))]);

    assert_eq!(
        returns,
        vec![Token::Int(ethereum_types::U256::from(12345678901u64))]
    );

    let returns = vm.function("copy", &[]);

    assert_eq!(
        returns,
        vec![Token::Array(vec![
            Token::Int(ethereum_types::U256::from(102)),
            Token::Int(ethereum_types::U256::from(0)),
            Token::Int(ethereum_types::U256::from(12345678901u64)),
        ])],
    );

    let returns = vm.function("pop", &[]);

    assert_eq!(
        returns,
        vec![Token::Int(ethereum_types::U256::from(12345678901u64))]
    );

    let returns = vm.function("len", &[]);

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(2))]);

    vm.function(
        "set",
        &[Token::Array(vec![
            Token::Int(ethereum_types::U256::from(1)),
            Token::Int(ethereum_types::U256::from(2)),
            Token::Int(ethereum_types::U256::from(3)),
            Token::Int(ethereum_types::U256::from(4)),
            Token::Int(ethereum_types::U256::from(5)),
            Token::Int(ethereum_types::U256::from(6)),
            Token::Int(ethereum_types::U256::from(7)),
        ])],
    );

    let returns = vm.function("copy", &[]);

    assert_eq!(
        returns,
        vec![Token::Array(vec![
            Token::Int(ethereum_types::U256::from(1)),
            Token::Int(ethereum_types::U256::from(2)),
            Token::Int(ethereum_types::U256::from(3)),
            Token::Int(ethereum_types::U256::from(4)),
            Token::Int(ethereum_types::U256::from(5)),
            Token::Int(ethereum_types::U256::from(6)),
            Token::Int(ethereum_types::U256::from(7)),
        ])],
    );

    vm.function("rm", &[]);

    let returns = vm.function("len", &[]);

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(0))]);
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

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(0))]);

    vm.function(
        "push1",
        &[Token::Tuple(vec![
            Token::Uint(ethereum_types::U256::from(13819038012u64)),
            Token::Bool(true),
        ])],
    );

    vm.function("push_empty", &[]);

    vm.function(
        "push2",
        &[Token::Tuple(vec![
            Token::Uint(ethereum_types::U256::from(12313123141123213u64)),
            Token::Bool(true),
        ])],
    );

    let returns = vm.function("subscript", &[Token::Uint(ethereum_types::U256::from(0))]);

    assert_eq!(
        returns,
        vec![Token::Tuple(vec![
            Token::Uint(ethereum_types::U256::from(13819038012u64)),
            Token::Bool(true),
        ])]
    );

    let returns = vm.function("subscript", &[Token::Uint(ethereum_types::U256::from(1))]);

    assert_eq!(
        returns,
        vec![Token::Tuple(vec![
            Token::Uint(ethereum_types::U256::from(0)),
            Token::Bool(false),
        ])]
    );

    let returns = vm.function("subscript", &[Token::Uint(ethereum_types::U256::from(2))]);

    assert_eq!(
        returns,
        vec![Token::Tuple(vec![
            Token::Uint(ethereum_types::U256::from(12313123141123213u64)),
            Token::Bool(true),
        ])]
    );

    let returns = vm.function("copy", &[]);

    assert_eq!(
        returns,
        vec![Token::Array(vec![
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(13819038012u64)),
                Token::Bool(true)
            ]),
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(0)),
                Token::Bool(false)
            ]),
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(12313123141123213u64)),
                Token::Bool(true)
            ]),
        ])]
    );

    let returns = vm.function("pop", &[]);

    assert_eq!(
        returns,
        vec![Token::Tuple(vec![
            Token::Uint(ethereum_types::U256::from(12313123141123213u64)),
            Token::Bool(true),
        ])]
    );

    let returns = vm.function("len", &[]);

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(2))]);

    vm.function(
        "set",
        &[Token::Array(vec![
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(1)),
                Token::Bool(false),
            ]),
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(2)),
                Token::Bool(true),
            ]),
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(3)),
                Token::Bool(false),
            ]),
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(4)),
                Token::Bool(true),
            ]),
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(5)),
                Token::Bool(false),
            ]),
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(6)),
                Token::Bool(true),
            ]),
        ])],
    );

    let returns = vm.function("copy", &[]);

    assert_eq!(
        returns,
        vec![Token::Array(vec![
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(1)),
                Token::Bool(false)
            ]),
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(2)),
                Token::Bool(true)
            ]),
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(3)),
                Token::Bool(false)
            ]),
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(4)),
                Token::Bool(true)
            ]),
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(5)),
                Token::Bool(false)
            ]),
            Token::Tuple(vec![
                Token::Uint(ethereum_types::U256::from(6)),
                Token::Bool(true)
            ]),
        ])]
    );

    vm.function("rm", &[]);

    let returns = vm.function("len", &[]);

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(0))]);
}
