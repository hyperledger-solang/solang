use crate::build_solidity;
use soroban_sdk::{Bytes, BytesN, IntoVal, TryFromVal};

#[test]
fn keccak256_basic() {
    let runtime = build_solidity(
        r#"contract Keccak256HashTest {
            function hash(bytes memory input) public pure returns (bytes32) {
                return keccak256(input);
            }
        }"#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let input = Bytes::from_slice(&runtime.env, b"hello_world");
    let result = runtime.invoke_contract(addr, "hash", vec![input.into_val(&runtime.env)]);

    let result_bytes = BytesN::<32>::try_from_val(&runtime.env, &result).unwrap();

    let expected = BytesN::<32>::from_array(
        &runtime.env,
        &hex::decode("5b07e077a81ffc6b47435f65a8727bcc542bc6fc0f25a56210efb1a74b88a5ae")
            .unwrap()
            .try_into()
            .unwrap(),
    );

    assert_eq!(result_bytes, expected);
}
