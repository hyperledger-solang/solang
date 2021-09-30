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
            function mr_slot() public returns (uint64) {
                return block.slot;
            }
            function mr_blocknumber() public returns (uint64) {
                return block.number;
            }
        }"#,
    );

    vm.constructor("timestamp", &[], 0);

    let returns = vm.function("mr_now", &[], &[], 0);

    assert_eq!(
        returns,
        vec![Token::Uint(ethereum_types::U256::from(1620656423))]
    );

    let returns = vm.function("mr_slot", &[], &[], 0);

    assert_eq!(
        returns,
        vec![Token::Uint(ethereum_types::U256::from(70818331))]
    );

    let returns = vm.function("mr_blocknumber", &[], &[], 0);

    assert_eq!(
        returns,
        vec![Token::Uint(ethereum_types::U256::from(70818331))]
    );
}
