use crate::build_solidity;
use ethabi::Token;

#[test]
fn interfaceid() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function get() public returns (bytes4) {
                return type(I).interfaceId;
            }
        }

        interface I {
            function bar(int) external;
            function baz(bytes) external returns (int);
        }"#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function("get", &[], &[], 0, None);

    assert_eq!(
        returns,
        vec![Token::FixedBytes(0xc78d9f3au32.to_be_bytes().to_vec())]
    );
}
