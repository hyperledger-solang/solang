// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{Bytes, FromVal, IntoVal};

#[test]
fn set_and_get_bytes() {
    let src = build_solidity(
        r#"contract BytesWriteStub {
            bytes public data;

            function set_data(bytes memory d) public {
                data = d;
            }
        }"#,
        |_| {},
    );

    let addr = src.contracts.last().unwrap();

    // empty bytes round-trip
    let empty = Bytes::from_slice(&src.env, b"");
    src.invoke_contract(addr, "set_data", vec![empty.clone().into_val(&src.env)]);
    let res = src.invoke_contract(addr, "data", vec![]);
    assert_eq!(Bytes::from_val(&src.env, &res), empty);

    // non-empty bytes round-trip
    let payload = Bytes::from_slice(&src.env, b"hello");
    src.invoke_contract(addr, "set_data", vec![payload.clone().into_val(&src.env)]);
    let res = src.invoke_contract(addr, "data", vec![]);
    assert_eq!(Bytes::from_val(&src.env, &res), payload);
}
