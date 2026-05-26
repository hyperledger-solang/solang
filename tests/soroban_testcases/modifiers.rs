// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;

#[test]
fn unused_modifier_compiles_without_panic() {
    let src = build_solidity(
        r#"contract C {
          modifier m0() { _; }
          function run() external pure {}
      }"#,
        |_| {},
    );

    let addr = src.contracts.last().unwrap();
    src.invoke_contract(addr, "run", vec![]);
}
