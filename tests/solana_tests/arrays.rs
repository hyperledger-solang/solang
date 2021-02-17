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

    vm.constructor(&[]);

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

    vm.constructor(&[]);

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

    vm.constructor(&[]);

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

            function set() public returns (uint x, uint32[] f, uint g) {
                x = 12123123;
                f = new uint32[](4);
                f[0] = 3; f[1] = 5; f[2] = 7; f[3] = 11;
                g = 102;
            }
        }"#,
    );

    vm.constructor(&[]);

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
            Token::Uint(ethereum_types::U256::from(102)),
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

    vm.constructor(&[]);

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

            function set() public returns (uint x, bytes[] f, uint g) {
                x = 12123123;
                f = new bytes[](4);
                f[0] = hex"030507";
                f[1] = hex"0b0d11";
                f[2] = hex"1317";
                f[3] = hex"1d";
                g = 102;
            }
        }"#,
    );

    vm.constructor(&[]);

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
            Token::Uint(ethereum_types::U256::from(102)),
        ]
    );
}
