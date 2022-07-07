use crate::build_solidity;
use base58::ToBase58;
use ethabi::{ethereum_types::U256, Token};

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

    vm.constructor("timestamp", &[]);

    let returns = vm.function("mr_now", &[], &[], None);

    assert_eq!(returns, vec![Token::Uint(U256::from(1620656423))]);

    let returns = vm.function("mr_slot", &[], &[], None);

    assert_eq!(returns, vec![Token::Uint(U256::from(70818331))]);

    let returns = vm.function("mr_blocknumber", &[], &[], None);

    assert_eq!(returns, vec![Token::Uint(U256::from(70818331))]);

    let returns = vm.function(
        "msg_data",
        &[Token::Uint(U256::from(0xdeadcafeu32))],
        &[],
        None,
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

    let returns = vm.function("sig", &[], &[], None);

    if let Token::FixedBytes(v) = &returns[0] {
        println!("{}", hex::encode(v));
    }

    assert_eq!(
        returns,
        vec![Token::FixedBytes(hex::decode("00a7029b").unwrap())]
    );
}

#[test]
fn pda() {
    let mut vm = build_solidity(
        r#"
        import 'solana';

        contract pda {
            function create_pda() public returns (address) {
                address program_id = address"BPFLoaderUpgradeab1e11111111111111111111111";

                return create_program_address(["Talking", "Squirrels"], program_id);
            }
        }"#,
    );

    vm.constructor("pda", &[]);

    let returns = vm.function("create_pda", &[], &[], None);

    if let Token::FixedBytes(bs) = &returns[0] {
        assert_eq!(
            bs.to_base58(),
            "2fnQrngrQT4SeLcdToJAD96phoEjNL2man2kfRLCASVk"
        );
    } else {
        panic!("{:?} not expected", returns);
    }
}
