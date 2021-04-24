use crate::build_solidity;
use ethabi::Token;

#[test]
fn types() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            int64 public f1 = 102;
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("f1", &[]);

    assert_eq!(returns, vec![Token::Int(ethereum_types::U256::from(102))]);

    let mut vm = build_solidity(
        r#"
        contract foo {
            int64[4] public f1 = [1,3,5,7];
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("f1", &[Token::Uint(ethereum_types::U256::from(2))]);

    assert_eq!(returns, vec![Token::Int(ethereum_types::U256::from(5))]);

    let mut vm = build_solidity(
        r#"
        contract foo {
            int64[4][2] public f1;

            constructor() {
                f1[1][0] = 4;
                f1[1][1] = 3;
                f1[1][2] = 2;
                f1[1][3] = 1;
            }
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function(
        "f1",
        &[
            Token::Uint(ethereum_types::U256::from(1)),
            Token::Uint(ethereum_types::U256::from(2)),
        ],
    );

    assert_eq!(returns, vec![Token::Int(ethereum_types::U256::from(2))]);

    let mut vm = build_solidity(
        r#"
        contract foo {
            mapping(int64 => uint64) public f1;

            constructor() {
                f1[2000] = 1;
                f1[4000] = 2;
            }
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("f1", &[Token::Int(ethereum_types::U256::from(4000))]);

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(2))]);
}

#[test]
fn interfaces() {
    let mut vm = build_solidity(
        r#"
        contract foo is bar {
            bytes2 public f1 = "ab";
        }

        interface bar {
            function f1() external returns (bytes2);
        }
        "#,
    );

    vm.constructor(&[]);

    let returns = vm.function("f1", &[]);

    assert_eq!(returns, vec![Token::FixedBytes(b"ab".to_vec())]);
}
