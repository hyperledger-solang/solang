// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use anchor_syn::idl::types::IdlInstruction;

#[test]
fn fallback() {
    let mut vm = build_solidity(
        r#"
        contract c {
            fallback() external {
                print("fallback");
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    if let Some(idl) = &vm.stack[0].idl {
        let mut idl = idl.clone();

        idl.instructions.push(IdlInstruction {
            name: "extinct".to_string(),
            docs: None,
            accounts: vec![],
            args: vec![],
            returns: None,
        });

        vm.stack[0].idl = Some(idl);
    }

    vm.function("extinct").call();

    assert_eq!(vm.logs, "fallback");
}
