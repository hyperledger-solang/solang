use crate::build_solidity;
use ethabi::Token;

#[test]
fn timestamp() {
    let mut vm = build_solidity(
        r#"
        contract timestamp {
            function mr_now() public returns (uint64) {
                return block.timestamp;
            }
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("mr_now", &[]);

    assert_eq!(
        returns,
        vec![Token::Uint(ethereum_types::U256::from(1620656423))]
    );
}
