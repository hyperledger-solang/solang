use crate::build_solidity;

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

    vm.constructor(&[]);

    assert_eq!(
        vm.data[0..12].to_vec(),
        vec![65, 177, 160, 100, 16, 0, 0, 0, 0, 0, 0, 0]
    );

    let returns = vm.function("get", &[]);

    assert_eq!(returns, vec![ethabi::Token::String(String::from(""))]);

    vm.function(
        "set",
        &[ethabi::Token::String(String::from("Hello, World!"))],
    );

    assert_eq!(
        vm.data[0..12].to_vec(),
        vec![65, 177, 160, 100, 16, 0, 0, 0, 32, 0, 0, 0]
    );

    assert_eq!(vm.data[32..45].to_vec(), b"Hello, World!");

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![ethabi::Token::String(String::from("Hello, World!"))]
    );

    // try replacing it with a string of the same length. This is a special
    // fast-path handling
    vm.function(
        "set",
        &[ethabi::Token::String(String::from("Hallo, Werld!"))],
    );

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![ethabi::Token::String(String::from("Hallo, Werld!"))]
    );

    assert_eq!(
        vm.data[0..12].to_vec(),
        vec![65, 177, 160, 100, 16, 0, 0, 0, 32, 0, 0, 0]
    );

    // Try setting this to an empty string. This is also a special case where
    // the result should be offset 0
    vm.function("set", &[ethabi::Token::String(String::from(""))]);

    let returns = vm.function("get", &[]);

    assert_eq!(returns, vec![ethabi::Token::String(String::from(""))]);

    assert_eq!(
        vm.data[0..12].to_vec(),
        vec![65, 177, 160, 100, 16, 0, 0, 0, 0, 0, 0, 0]
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

    vm.constructor(&[]);

    assert_eq!(
        vm.data[0..12].to_vec(),
        vec![11, 66, 182, 57, 16, 0, 0, 0, 0, 0, 0, 0]
    );

    let returns = vm.function("foo_length", &[]);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(0))]
    );

    vm.function(
        "set_foo",
        &[ethabi::Token::Bytes(
            b"The shoemaker always wears the worst shoes".to_vec(),
        )],
    );

    assert_eq!(
        vm.data[0..12].to_vec(),
        vec![11, 66, 182, 57, 16, 0, 0, 0, 32, 0, 0, 0]
    );

    for (i, b) in b"The shoemaker always wears the worst shoes"
        .to_vec()
        .into_iter()
        .enumerate()
    {
        let returns = vm.function(
            "get_foo_offset",
            &[ethabi::Token::Uint(ethereum_types::U256::from(i))],
        );

        assert_eq!(returns, vec![ethabi::Token::FixedBytes(vec![b])]);
    }

    vm.function(
        "set_foo_offset",
        &[
            ethabi::Token::Uint(ethereum_types::U256::from(2)),
            ethabi::Token::FixedBytes(b"E".to_vec()),
        ],
    );

    vm.function(
        "set_foo_offset",
        &[
            ethabi::Token::Uint(ethereum_types::U256::from(7)),
            ethabi::Token::FixedBytes(b"E".to_vec()),
        ],
    );

    for (i, b) in b"ThE shoEmaker always wears the worst shoes"
        .to_vec()
        .into_iter()
        .enumerate()
    {
        let returns = vm.function(
            "get_foo_offset",
            &[ethabi::Token::Uint(ethereum_types::U256::from(i))],
        );

        assert_eq!(returns, vec![ethabi::Token::FixedBytes(vec![b])]);
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

    vm.constructor(&[]);

    vm.function(
        "set_foo_offset",
        &[
            ethabi::Token::Uint(ethereum_types::U256::from(0)),
            ethabi::Token::FixedBytes(b"E".to_vec()),
        ],
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

    vm.constructor(&[]);

    vm.function(
        "set_foo",
        &[ethabi::Token::Bytes(
            b"The shoemaker always wears the worst shoes".to_vec(),
        )],
    );

    vm.function(
        "get_foo_offset",
        &[ethabi::Token::Uint(ethereum_types::U256::from(
            0x80000000u64,
        ))],
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

    vm.constructor(&[]);

    assert_eq!(
        vm.data[0..32].to_vec(),
        vec![
            11, 66, 182, 57, 32, 0, 0, 0, 1, 0, 3, 2, 4, 0, 0, 0, 8, 7, 6, 5, 0, 0, 0, 0, 16, 15,
            14, 13, 12, 11, 10, 9
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

    vm.constructor(&[]);

    let returns = vm.function("get_bs", &[]);

    assert_eq!(returns, vec![ethabi::Token::Bytes(vec!(0x0e, 0xda))]);

    let returns = vm.function("pop", &[]);

    assert_eq!(returns, vec![ethabi::Token::FixedBytes(vec!(0xda))]);

    let returns = vm.function("get_bs", &[]);

    assert_eq!(returns, vec![ethabi::Token::Bytes(vec!(0x0e))]);

    vm.function("push", &[ethabi::Token::FixedBytes(vec![0x41])]);

    println!("data:{}", hex::encode(&vm.data));

    let returns = vm.function("get_bs", &[]);

    assert_eq!(returns, vec![ethabi::Token::Bytes(vec!(0x0e, 0x41))]);

    vm.function("push", &[ethabi::Token::FixedBytes(vec![0x01])]);

    let returns = vm.function("get_bs", &[]);

    assert_eq!(returns, vec![ethabi::Token::Bytes(vec!(0x0e, 0x41, 0x01))]);
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

    vm.constructor(&[]);

    vm.function("pop", &[]);
}
