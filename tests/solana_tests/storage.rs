// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use ethabi::{ethereum_types::U256, Token};

#[test]
fn simple() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            int private foo;

            function boom() public view returns (int) {
                int baz = false? foo : 0;
                return baz;
            }
        }"#,
    );

    vm.constructor("foo", &[]);
    let returns = vm.function("boom", &[], &[], None);
    assert_eq!(returns, vec![Token::Int(U256::from(0)),]);

    let mut vm = build_solidity(
        r#"
        contract c {
            struct Struct {
                int field;
            }

            Struct mem;
            constructor() {
                mem = Struct(1);
            }

            function func() public view returns(int) {
                Struct bar = true? mem: mem;
                Struct baz = bar;
                return baz.field;
            }
        }"#,
    );

    vm.constructor("c", &[]);
    let returns = vm.function("func", &[], &[], None);
    assert_eq!(returns, vec![Token::Int(U256::from(1)),]);
}

#[test]
fn string() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            string s;

            function set(string value) public {
                s = value;
            }

            function get() public returns (string) {
                return s;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    assert_eq!(
        vm.data()[0..20].to_vec(),
        vec![65, 177, 160, 100, 0, 0, 0, 0, 0, 0, 0, 0, 24, 0, 0, 0, 0, 0, 0, 0]
    );

    let returns = vm.function("get", &[], &[], None);

    assert_eq!(returns, vec![Token::String(String::from(""))]);

    vm.function(
        "set",
        &[Token::String(String::from("Hello, World!"))],
        &[],
        None,
    );

    assert_eq!(
        vm.data()[0..20].to_vec(),
        vec![65, 177, 160, 100, 0, 0, 0, 0, 0, 0, 0, 0, 24, 0, 0, 0, 40, 0, 0, 0]
    );

    assert_eq!(vm.data()[40..53].to_vec(), b"Hello, World!");

    let returns = vm.function("get", &[], &[], None);

    assert_eq!(returns, vec![Token::String(String::from("Hello, World!"))]);

    // try replacing it with a string of the same length. This is a special
    // fast-path handling
    vm.function(
        "set",
        &[Token::String(String::from("Hallo, Werld!"))],
        &[],
        None,
    );

    let returns = vm.function("get", &[], &[], None);

    assert_eq!(returns, vec![Token::String(String::from("Hallo, Werld!"))]);

    assert_eq!(
        vm.data()[0..20].to_vec(),
        vec![65, 177, 160, 100, 0, 0, 0, 0, 0, 0, 0, 0, 24, 0, 0, 0, 40, 0, 0, 0]
    );

    // Try setting this to an empty string. This is also a special case where
    // the result should be offset 0
    vm.function("set", &[Token::String(String::from(""))], &[], None);

    let returns = vm.function("get", &[], &[], None);

    assert_eq!(returns, vec![Token::String(String::from(""))]);

    assert_eq!(
        vm.data()[0..20].to_vec(),
        vec![65, 177, 160, 100, 0, 0, 0, 0, 0, 0, 0, 0, 24, 0, 0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn bytes() {
    let mut vm = build_solidity(
        r#"
        contract c {
            bytes foo;

            function set_foo(bytes bs) public {
                foo = bs;
            }

            function foo_length() public returns (uint32) {
                return foo.length;
            }

            function set_foo_offset(uint32 index, byte b) public {
                foo[index] = b;
            }

            function get_foo_offset(uint32 index) public returns (byte) {
                return foo[index];
            }
        }"#,
    );

    vm.constructor("c", &[]);

    assert_eq!(
        vm.data()[0..20].to_vec(),
        vec![11, 66, 182, 57, 0, 0, 0, 0, 0, 0, 0, 0, 24, 0, 0, 0, 0, 0, 0, 0]
    );

    let returns = vm.function("foo_length", &[], &[], None);

    assert_eq!(returns, vec![Token::Uint(U256::from(0))]);

    vm.function(
        "set_foo",
        &[Token::Bytes(
            b"The shoemaker always wears the worst shoes".to_vec(),
        )],
        &[],
        None,
    );

    assert_eq!(
        vm.data()[0..20].to_vec(),
        vec![11, 66, 182, 57, 0, 0, 0, 0, 0, 0, 0, 0, 24, 0, 0, 0, 40, 0, 0, 0]
    );

    for (i, b) in b"The shoemaker always wears the worst shoes"
        .iter()
        .enumerate()
    {
        let returns = vm.function("get_foo_offset", &[Token::Uint(U256::from(i))], &[], None);

        assert_eq!(returns, vec![Token::FixedBytes(vec![*b])]);
    }

    vm.function(
        "set_foo_offset",
        &[Token::Uint(U256::from(2)), Token::FixedBytes(b"E".to_vec())],
        &[],
        None,
    );

    vm.function(
        "set_foo_offset",
        &[Token::Uint(U256::from(7)), Token::FixedBytes(b"E".to_vec())],
        &[],
        None,
    );

    for (i, b) in b"ThE shoEmaker always wears the worst shoes"
        .iter()
        .enumerate()
    {
        let returns = vm.function("get_foo_offset", &[Token::Uint(U256::from(i))], &[], None);

        assert_eq!(returns, vec![Token::FixedBytes(vec![*b])]);
    }
}

#[test]
#[should_panic]
fn bytes_set_subscript_range() {
    let mut vm = build_solidity(
        r#"
        contract c {
            bytes foo;

            function set_foo(bytes bs) public {
                foo = bs;
            }

            function foo_length() public returns (uint32) {
                return foo.length;
            }

            function set_foo_offset(uint32 index, byte b) public {
                foo[index] = b;
            }

            function get_foo_offset(uint32 index) public returns (byte) {
                return foo[index];
            }
        }"#,
    );

    vm.constructor("c", &[]);

    vm.function(
        "set_foo_offset",
        &[Token::Uint(U256::from(0)), Token::FixedBytes(b"E".to_vec())],
        &[],
        None,
    );
}

#[test]
#[should_panic]
fn bytes_get_subscript_range() {
    let mut vm = build_solidity(
        r#"
        contract c {
            bytes foo;

            function set_foo(bytes bs) public {
                foo = bs;
            }

            function foo_length() public returns (uint32) {
                return foo.length;
            }

            function set_foo_offset(uint32 index, byte b) public {
                foo[index] = b;
            }

            function get_foo_offset(uint32 index) public returns (byte) {
                return foo[index];
            }
        }"#,
    );

    vm.constructor("c", &[]);

    vm.function(
        "set_foo",
        &[Token::Bytes(
            b"The shoemaker always wears the worst shoes".to_vec(),
        )],
        &[],
        None,
    );

    vm.function(
        "get_foo_offset",
        &[Token::Uint(U256::from(0x80000000u64))],
        &[],
        None,
    );
}

#[test]
fn storage_alignment() {
    let mut vm = build_solidity(
        r#"
        contract c {
            bool f1 = true;
            uint16 f3 = 0x203;
            uint8 f2 = 4;
            uint32 f4 = 0x5060708;
            uint64 f5 = 0x90a0b0c0d0e0f10;
        }"#,
    );

    vm.constructor("c", &[]);

    assert_eq!(
        vm.data()[0..40].to_vec(),
        vec![
            11, 66, 182, 57, 0, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 1, 0, 3, 2, 4, 0, 0, 0, 8, 7, 6,
            5, 0, 0, 0, 0, 16, 15, 14, 13, 12, 11, 10, 9
        ]
    );
}

#[test]
fn bytes_push_pop() {
    let mut vm = build_solidity(
        r#"
        contract c {
            bytes bs = hex"0eda";

            function get_bs() public view returns (bytes) {
                return bs;
            }

            function push(byte v) public {
                bs.push(v);
            }

            function pop() public returns (byte) {
                return bs.pop();
            }
        }"#,
    );

    vm.constructor("c", &[]);

    let returns = vm.function("get_bs", &[], &[], None);

    assert_eq!(returns, vec![Token::Bytes(vec!(0x0e, 0xda))]);

    let returns = vm.function("pop", &[], &[], None);

    assert_eq!(returns, vec![Token::FixedBytes(vec!(0xda))]);

    let returns = vm.function("get_bs", &[], &[], None);

    assert_eq!(returns, vec![Token::Bytes(vec!(0x0e))]);

    vm.function("push", &[Token::FixedBytes(vec![0x41])], &[], None);

    println!("data:{}", hex::encode(vm.data()));

    let returns = vm.function("get_bs", &[], &[], None);

    assert_eq!(returns, vec![Token::Bytes(vec!(0x0e, 0x41))]);

    vm.function("push", &[Token::FixedBytes(vec![0x01])], &[], None);

    let returns = vm.function("get_bs", &[], &[], None);

    assert_eq!(returns, vec![Token::Bytes(vec!(0x0e, 0x41, 0x01))]);
}

#[test]
#[should_panic]
fn bytes_empty_pop() {
    let mut vm = build_solidity(
        r#"
        contract c {
            bytes bs;

            function pop() public returns (byte) {
                return bs.pop();
            }
        }"#,
    );

    vm.constructor("c", &[]);

    vm.function("pop", &[], &[], None);
}

#[test]
fn simple_struct() {
    let mut vm = build_solidity(
        r#"
        contract c {
            struct s {
                uint8 f1;
                uint32 f2;
            }

            uint16 s2 = 0xdead;
            s s1;

            function get_s1() public returns (s) {
                return s1;
            }

            function set_s1(s v) public {
                s1 = v;
            }

            function set_s2() public {
                s1 = s({f1: 254, f2: 0xdead});
            }
        }"#,
    );

    vm.constructor("c", &[]);

    vm.function("set_s2", &[], &[], None);

    assert_eq!(
        vm.data()[0..32].to_vec(),
        vec![
            11, 66, 182, 57, 0, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 173, 222, 0, 0, 254, 0, 0, 0,
            173, 222, 0, 0, 0, 0, 0, 0
        ]
    );

    let returns = vm.function("get_s1", &[], &[], None);

    assert_eq!(
        returns,
        vec![Token::Tuple(vec![
            Token::Uint(U256::from(254)),
            Token::Uint(U256::from(0xdead)),
        ])]
    );

    vm.function(
        "set_s1",
        &[Token::Tuple(vec![
            Token::Uint(U256::from(102)),
            Token::Uint(U256::from(3240121)),
        ])],
        &[],
        None,
    );

    let returns = vm.function("get_s1", &[], &[], None);

    assert_eq!(
        returns,
        vec![Token::Tuple(vec![
            Token::Uint(U256::from(102)),
            Token::Uint(U256::from(3240121)),
        ])]
    );
}

#[test]
fn struct_in_struct() {
    let mut vm = build_solidity(
        r#"
        contract c {
            struct s {
                uint8 f1;
                X f3;
                uint64 f4;
            }

            struct X {
                int32 f1;
                bytes6 f2;
            }

            uint32 s2 = 0xdead;
            s s1;

            function get_s1() public returns (s) {
                return s1;
            }

            function set_s1(s v) public {
                s1 = v;
            }

            function set_s2() public {
                s1 = s({f1: 254, f3: X({f1: 102, f2: "foobar"}), f4: 1234567890});
            }
        }"#,
    );

    vm.constructor("c", &[]);

    vm.function("set_s2", &[], &[], None);

    assert_eq!(
        vm.data()[0..52].to_vec(),
        vec![
            11, 66, 182, 57, 0, 0, 0, 0, 0, 0, 0, 0, 56, 0, 0, 0, 173, 222, 0, 0, 0, 0, 0, 0, 254,
            0, 0, 0, 0, 0, 102, 0, 0, 0, 0, 0, 114, 97, 98, 111, 111, 102, 0, 0, 0, 0, 0, 0, 210,
            2, 150, 73
        ]
    );

    let returns = vm.function("get_s1", &[], &[], None);

    assert_eq!(
        returns,
        vec![Token::Tuple(vec![
            Token::Uint(U256::from(254)),
            Token::Tuple(vec![
                Token::Int(U256::from(102)),
                Token::FixedBytes(vec![102, 111, 111, 98, 97, 114])
            ]),
            Token::Uint(U256::from(1234567890))
        ])]
    );

    vm.function(
        "set_s1",
        &[Token::Tuple(vec![
            Token::Uint(U256::from(127)),
            Token::Tuple(vec![
                Token::Int(U256::from(8192)),
                Token::FixedBytes(vec![1, 2, 3, 4, 5, 6]),
            ]),
            Token::Uint(U256::from(12345678901234567890u64)),
        ])],
        &[],
        None,
    );

    let returns = vm.function("get_s1", &[], &[], None);

    assert_eq!(
        returns,
        vec![Token::Tuple(vec![
            Token::Uint(U256::from(127)),
            Token::Tuple(vec![
                Token::Int(U256::from(8192)),
                Token::FixedBytes(vec![1, 2, 3, 4, 5, 6]),
            ]),
            Token::Uint(U256::from(12345678901234567890u64)),
        ])]
    );
}

#[test]
fn string_in_struct() {
    let mut vm = build_solidity(
        r#"
            contract c {
                struct s {
                    uint8 f1;
                    string f2;
                    uint64 f3;
                }

                uint32 s2 = 0xdead;
                s s1;

                function get_s1() public returns (s) {
                    return s1;
                }

                function set_s1(s v) public {
                    s1 = v;
                }

                function set_s2() public {
                    s1 = s({f1: 254, f2: "foobar", f3: 1234567890});
                }
            }"#,
    );

    vm.constructor("c", &[]);

    vm.function("set_s2", &[], &[], None);

    assert_eq!(
        vm.data()[0..64].to_vec(),
        vec![
            11, 66, 182, 57, 0, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 173, 222, 0, 0, 0, 0, 0, 0, 254,
            0, 0, 0, 56, 0, 0, 0, 210, 2, 150, 73, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0,
            1, 0, 0, 0, 102, 111, 111, 98, 97, 114, 0, 0
        ]
    );

    let returns = vm.function("get_s1", &[], &[], None);

    assert_eq!(
        returns,
        vec![Token::Tuple(vec![
            Token::Uint(U256::from(254)),
            Token::String(String::from("foobar")),
            Token::Uint(U256::from(1234567890))
        ])]
    );

    vm.function(
        "set_s1",
        &[Token::Tuple(vec![
            Token::Uint(U256::from(127)),
            Token::String(String::from("foobar foobar foobar foobar foobar foobar")),
            Token::Uint(U256::from(12345678901234567890u64)),
        ])],
        &[],
        None,
    );

    let returns = vm.function("get_s1", &[], &[], None);

    assert_eq!(
        returns,
        vec![Token::Tuple(vec![
            Token::Uint(U256::from(127)),
            Token::String(String::from("foobar foobar foobar foobar foobar foobar")),
            Token::Uint(U256::from(12345678901234567890u64)),
        ])]
    );
}

#[test]
fn complex_struct() {
    let mut vm = build_solidity(
        r#"
        contract c {
            struct s {
                uint8 f1;
                string f2;
                ss f3;
                uint64 f4;
                sss f5;
                string f6;
            }
            struct ss {
                bool ss1;
                bytes3 ss2;
            }
            struct sss {
                uint256 sss1;
                bytes sss2;
            }

            s s1;
            uint32 s2 = 0xdead;
            string s3;

            function get_s1() public returns (s, string) {
                return (s1, s3);
            }

            function set_s1(s v, string v2) public {
                s1 = v;
                s3 = v2;
            }

            function set_s2() public {
                s1.f1 = 254;
                s1.f2 = "foobar";
                s1.f3.ss1 = true;
                s1.f3.ss2 = hex"edaeda";
                s1.f4 = 1234567890;
                s1.f5.sss1 = 12123131321312;
                s1.f5.sss2 = "jasldajldjaldjlads";
                s1.f6 = "as nervous as a long-tailed cat in a room full of rocking chairs";
            }

            function rm() public {
                delete s1;
            }
        }"#,
    );

    vm.constructor("c", &[]);

    vm.function("set_s2", &[], &[], None);

    let returns = vm.function("get_s1", &[], &[], None);

    assert_eq!(
        returns,
        vec![
            Token::Tuple(vec![
                Token::Uint(U256::from(254)),
                Token::String(String::from("foobar")),
                Token::Tuple(vec!(
                    Token::Bool(true),
                    Token::FixedBytes(vec!(0xed, 0xae, 0xda))
                )),
                Token::Uint(U256::from(1234567890)),
                Token::Tuple(vec!(
                    Token::Uint(U256::from(12123131321312u128)),
                    Token::Bytes(b"jasldajldjaldjlads".to_vec())
                )),
                Token::String(String::from(
                    "as nervous as a long-tailed cat in a room full of rocking chairs"
                ))
            ]),
            Token::String(String::from("")),
        ]
    );

    vm.function(
        "set_s1",
        &[
            Token::Tuple(vec![
                Token::Uint(U256::from(127)),
                Token::String(String::from("foobar foobar foobar foobar foobar foobar")),
                Token::Tuple(vec![
                    Token::Bool(false),
                    Token::FixedBytes(vec![0xc3, 0x9a, 0xfd]),
                ]),
                Token::Uint(U256::from(12345678901234567890u64)),
                Token::Tuple(vec![
                    Token::Uint(U256::from(97560097522392203078545981438598778247u128)),
                    Token::Bytes(b"jasldajldjaldjlads".to_vec()),
                ]),
                Token::String(String::from("be as honest as the day is long")),
            ]),
            Token::String(String::from("yadayada")),
        ],
        &[],
        None,
    );

    let returns = vm.function("get_s1", &[], &[], None);

    assert_eq!(
        returns,
        vec![
            Token::Tuple(vec![
                Token::Uint(U256::from(127)),
                Token::String(String::from("foobar foobar foobar foobar foobar foobar")),
                Token::Tuple(vec![
                    Token::Bool(false),
                    Token::FixedBytes(vec![0xc3, 0x9a, 0xfd]),
                ]),
                Token::Uint(U256::from(12345678901234567890u64)),
                Token::Tuple(vec![
                    Token::Uint(U256::from(97560097522392203078545981438598778247u128,)),
                    Token::Bytes(b"jasldajldjaldjlads".to_vec()),
                ]),
                Token::String(String::from("be as honest as the day is long")),
            ]),
            Token::String(String::from("yadayada")),
        ]
    );

    vm.function("rm", &[], &[], None);

    let returns = vm.function("get_s1", &[], &[], None);

    assert_eq!(
        returns,
        vec![
            Token::Tuple(vec![
                Token::Uint(U256::from(0)),
                Token::String(String::from("")),
                Token::Tuple(vec![Token::Bool(false), Token::FixedBytes(vec![0, 0, 0]),]),
                Token::Uint(U256::from(0)),
                Token::Tuple(vec![Token::Uint(U256::from(0)), Token::Bytes(Vec::new()),]),
                Token::String(String::from("")),
            ]),
            Token::String(String::from("yadayada")),
        ]
    );
}

// dereference struct storage member (read/write)
