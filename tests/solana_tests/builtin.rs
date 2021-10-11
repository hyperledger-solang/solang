use crate::build_solidity;
use ethabi::Token;

#[test]
fn builtins() {
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
            function msg_data(uint32 x) public returns (bytes) {
                return msg.data;
            }
            function sig() public returns (bytes4) {
                return msg.sig;
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

    let returns = vm.function(
        "msg_data",
        &[Token::Uint(ethereum_types::U256::from(0xdeadcafeu32))],
        &[],
        0,
    );

    if let Token::Bytes(v) = &returns[0] {
        println!("{}", hex::encode(v));
    }

    assert_eq!(
        returns,
        vec![Token::Bytes(
            hex::decode("84da38e000000000000000000000000000000000000000000000000000000000deadcafe")
                .unwrap()
        )]
    );

    let returns = vm.function("sig", &[], &[], 0);

    if let Token::FixedBytes(v) = &returns[0] {
        println!("{}", hex::encode(v));
    }

    assert_eq!(
        returns,
        vec![Token::FixedBytes(hex::decode("00a7029b").unwrap())]
    );
}
