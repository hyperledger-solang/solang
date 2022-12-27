// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};

#[test]
fn test_slice_in_phi() {
    let file = r#"
    contract c1 {
        function test() public returns (string) {
            string ast = "Hello!";
            string bst = "from Solang";

            while (ast == bst) {
                ast = ast + "a";
            }

            return ast;
        }
    }
    "#;

    let mut vm = build_solidity(file);
    vm.constructor(&[]);
    let returns = vm.function("test", &[]).unwrap();

    assert_eq!(returns, BorshToken::String(String::from("Hello!")));
}
