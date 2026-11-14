// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{vec, Error, IntoVal, Symbol, Val};

#[test]
fn custom_error_returns_contract_error_code() {
    let runtime = build_solidity(
        r#"contract c {
            error First();
            error Second(uint32 x);

            function f(uint32 x) public pure returns (uint32) {
                if (x == 1) revert First();
                if (x == 2) revert Second(x);
                require(x != 3, "plain require");
                return x;
            }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;
    let call = |x: u32| {
        env.try_invoke_contract::<u32, Error>(
            addr,
            &Symbol::new(env, "f"),
            vec![env, x.into_val(env)],
        )
    };

    assert_eq!(call(0), Ok(Ok(0)));
    assert_eq!(call(1), Err(Ok(Error::from_contract_error(1))));
    assert_eq!(call(2), Err(Ok(Error::from_contract_error(2))));
    // Reverts without a custom error still trap.
    let trapped: Val = call(3).unwrap_err().unwrap().into_val(env);
    assert!(!Error::try_from(trapped)
        .unwrap()
        .is_type(soroban_sdk::xdr::ScErrorType::Contract));
}
