use crate::{account_new, build_solidity};
use ethabi::Token;

#[test]
fn simple_mapping() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            mapping (uint64 => uint64) map;

            function set(uint64 index, uint64 val) public {
                map[index] = val;
            }

            function get(uint64 index) public returns (uint64) {
                return map[index];
            }

            function rm(uint64 index) public {
                delete map[index];
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    for i in 0..10 {
        vm.function(
            "set",
            &[
                Token::Uint(ethereum_types::U256::from(102 + i)),
                Token::Uint(ethereum_types::U256::from(300331 + i)),
            ],
            &[],
            0,
            None,
        );
    }

    for i in 0..10 {
        let returns = vm.function(
            "get",
            &[Token::Uint(ethereum_types::U256::from(102 + i))],
            &[],
            0,
            None,
        );

        assert_eq!(
            returns,
            vec![Token::Uint(ethereum_types::U256::from(300331 + i))]
        );
    }

    let returns = vm.function(
        "get",
        &[Token::Uint(ethereum_types::U256::from(101))],
        &[],
        0,
        None,
    );

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(0))]);

    vm.function(
        "rm",
        &[Token::Uint(ethereum_types::U256::from(104))],
        &[],
        0,
        None,
    );

    for i in 0..10 {
        let returns = vm.function(
            "get",
            &[Token::Uint(ethereum_types::U256::from(102 + i))],
            &[],
            0,
            None,
        );

        if 102 + i != 104 {
            assert_eq!(
                returns,
                vec![Token::Uint(ethereum_types::U256::from(300331 + i))]
            );
        } else {
            assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(0))]);
        }
    }
}

#[test]
fn less_simple_mapping() {
    let mut vm = build_solidity(
        r#"
        struct S {
            string f1;
            int64[] f2;
        }

        contract foo {
            mapping (uint => S) map;

            function set_string(uint index, string s) public {
                map[index].f1 = s;
            }

            function add_int(uint index, int64 n) public {
                map[index].f2.push(n);
            }

            function get(uint index) public returns (S) {
                return map[index];
            }

            function rm(uint index) public {
                delete map[index];
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    vm.function(
        "set_string",
        &[
            Token::Uint(ethereum_types::U256::from(12313132131321312311213131u128)),
            Token::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
        ], &[], 0, None
    );

    vm.function(
        "add_int",
        &[
            Token::Uint(ethereum_types::U256::from(12313132131321312311213131u128)),
            Token::Int(ethereum_types::U256::from(102)),
        ],
        &[],
        0,
        None,
    );

    let returns = vm.function(
        "get",
        &[Token::Uint(ethereum_types::U256::from(
            12313132131321312311213131u128,
        ))],
        &[],
        0,
        None,
    );

    assert_eq!(
        returns,
        vec![Token::Tuple(vec![
            Token::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
            Token::Array(vec![Token::Int(ethereum_types::U256::from(102))]),
        ])]
    );
}

#[test]
fn string_mapping() {
    let mut vm = build_solidity(
        r#"
        struct S {
            string f1;
            int64[] f2;
        }

        contract foo {
            mapping (string => S) map;

            function set_string(string index, string s) public {
                map[index].f1 = s;
            }

            function add_int(string index, int64 n) public {
                map[index].f2.push(n);
            }

            function get(string index) public returns (S) {
                return map[index];
            }

            function rm(string index) public {
                delete map[index];
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    vm.function(
        "set_string",
        &[
            Token::String(String::from("a")),
            Token::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
        ], &[],0, None
    );

    vm.function(
        "add_int",
        &[
            Token::String(String::from("a")),
            Token::Int(ethereum_types::U256::from(102)),
        ],
        &[],
        0,
        None,
    );

    let returns = vm.function("get", &[Token::String(String::from("a"))], &[], 0, None);

    assert_eq!(
        returns,
        vec![Token::Tuple(vec![
            Token::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
            Token::Array(vec![Token::Int(ethereum_types::U256::from(102))]),
        ])]
    );
}

#[test]
fn contract_mapping() {
    let mut vm = build_solidity(
        r#"
        interface I {}

        contract foo {
            mapping (I => string) public map;

            function set(I index, string s) public {
                map[index] = s;
            }

            function get(I index) public returns (string) {
                return map[index];
            }

            function rm(I index) public {
                delete map[index];
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    let index = Token::FixedBytes(account_new().to_vec());

    vm.function(
        "set",
        &[
            index.clone(),
            Token::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
        ], &[],0, None
    );

    let returns = vm.function("get", &[index.clone()], &[], 0, None);

    assert_eq!(
        returns,
        vec![Token::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder"))]
    );

    vm.function("rm", &[index.clone()], &[], 0, None);

    let returns = vm.function("get", &[index], &[], 0, None);

    assert_eq!(returns, vec![Token::String(String::from(""))]);
}

#[test]
fn mapping_in_mapping() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            mapping (string => mapping(int64 => byte)) public map;

            function set(string s, int64 n, bytes1 v) public {
                map[s][n] = v;
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    vm.function(
        "set",
        &[
            Token::String(String::from("a")),
            Token::Int(ethereum_types::U256::from(102)),
            Token::FixedBytes(vec![0x98]),
        ],
        &[],
        0,
        None,
    );

    let returns = vm.function(
        "map",
        &[
            Token::String(String::from("a")),
            Token::Int(ethereum_types::U256::from(102)),
        ],
        &[],
        0,
        None,
    );

    assert_eq!(returns, vec![Token::FixedBytes(vec![0x98])]);

    let returns = vm.function(
        "map",
        &[
            Token::String(String::from("a")),
            Token::Int(ethereum_types::U256::from(103)),
        ],
        &[],
        0,
        None,
    );

    assert_eq!(returns, vec![Token::FixedBytes(vec![0])]);

    let returns = vm.function(
        "map",
        &[
            Token::String(String::from("b")),
            Token::Int(ethereum_types::U256::from(102)),
        ],
        &[],
        0,
        None,
    );

    assert_eq!(returns, vec![Token::FixedBytes(vec![0])]);
}

#[test]
fn sparse_array() {
    let mut vm = build_solidity(
        r#"
        struct S {
            string f1;
            int64[] f2;
        }

        contract foo {
            S[1e9] map;

            function set_string(uint index, string s) public {
                map[index].f1 = s;
            }

            function add_int(uint index, int64 n) public {
                map[index].f2.push(n);
            }

            function get(uint index) public returns (S) {
                return map[index];
            }

            function rm(uint index) public {
                delete map[index];
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    vm.function(
        "set_string",
        &[
            Token::Uint(ethereum_types::U256::from(909090909)),
            Token::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
        ], &[],0, None
    );

    vm.function(
        "add_int",
        &[
            Token::Uint(ethereum_types::U256::from(909090909)),
            Token::Int(ethereum_types::U256::from(102)),
        ],
        &[],
        0,
        None,
    );

    let returns = vm.function(
        "get",
        &[Token::Uint(ethereum_types::U256::from(909090909))],
        &[],
        0,
        None,
    );

    assert_eq!(
        returns,
        vec![Token::Tuple(vec![
            Token::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
            Token::Array(vec![Token::Int(ethereum_types::U256::from(102))]),
        ])]
    );
}

#[test]
fn massive_sparse_array() {
    let mut vm = build_solidity(
        r#"
        struct S {
            string f1;
            int64[] f2;
        }

        contract foo {
            S[1e24] map;

            function set_string(uint index, string s) public {
                map[index].f1 = s;
            }

            function add_int(uint index, int64 n) public {
                map[index].f2.push(n);
            }

            function get(uint index) public returns (S) {
                return map[index];
            }

            function rm(uint index) public {
                delete map[index];
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    vm.function(
        "set_string",
        &[
            Token::Uint(ethereum_types::U256::from(786868768768678687686877u128)),
            Token::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
        ], &[],0, None
    );

    vm.function(
        "add_int",
        &[
            Token::Uint(ethereum_types::U256::from(786868768768678687686877u128)),
            Token::Int(ethereum_types::U256::from(102)),
        ],
        &[],
        0,
        None,
    );

    let returns = vm.function(
        "get",
        &[Token::Uint(ethereum_types::U256::from(
            786868768768678687686877u128,
        ))],
        &[],
        0,
        None,
    );

    assert_eq!(
        returns,
        vec![Token::Tuple(vec![
            Token::String(String::from("This is a string which should be a little longer than 32 bytes so we the the abi encoder")),
            Token::Array(vec![Token::Int(ethereum_types::U256::from(102))]),
        ])]
    );
}

#[test]
fn mapping_in_dynamic_array() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            mapping (uint64 => uint64)[] public map;
            int64 public number;

            function set(uint64 array_no, uint64 index, uint64 val) public {
                map[array_no][index] = val;
            }

            function rm(uint64 array_no, uint64 index) public {
                delete map[array_no][index];
            }

            function push() public {
                map.push();
            }

            function pop() public {
                map.pop();
            }

            function setNumber(int64 x) public {
                number = x;
            }
        }"#,
    );

    vm.constructor("foo", &[], 0);

    vm.function("push", &[], &[], 0, None);
    vm.function("push", &[], &[], 0, None);

    for array_no in 0..2 {
        for i in 0..10 {
            vm.function(
                "set",
                &[
                    Token::Uint(ethereum_types::U256::from(array_no)),
                    Token::Uint(ethereum_types::U256::from(102 + i + array_no * 500)),
                    Token::Uint(ethereum_types::U256::from(300331 + i)),
                ],
                &[],
                0,
                None,
            );
        }
    }

    for array_no in 0..2 {
        for i in 0..10 {
            let returns = vm.function(
                "map",
                &[
                    Token::Uint(ethereum_types::U256::from(array_no)),
                    Token::Uint(ethereum_types::U256::from(102 + i + array_no * 500)),
                ],
                &[],
                0,
                None,
            );

            assert_eq!(
                returns,
                vec![Token::Uint(ethereum_types::U256::from(300331 + i))]
            );
        }
    }

    let returns = vm.function(
        "map",
        &[
            Token::Uint(ethereum_types::U256::from(0)),
            Token::Uint(ethereum_types::U256::from(101)),
        ],
        &[],
        0,
        None,
    );

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(0))]);

    vm.function(
        "rm",
        &[
            Token::Uint(ethereum_types::U256::from(0)),
            Token::Uint(ethereum_types::U256::from(104)),
        ],
        &[],
        0,
        None,
    );

    for i in 0..10 {
        let returns = vm.function(
            "map",
            &[
                Token::Uint(ethereum_types::U256::from(0)),
                Token::Uint(ethereum_types::U256::from(102 + i)),
            ],
            &[],
            0,
            None,
        );

        if 102 + i != 104 {
            assert_eq!(
                returns,
                vec![Token::Uint(ethereum_types::U256::from(300331 + i))]
            );
        } else {
            assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(0))]);
        }
    }
}
