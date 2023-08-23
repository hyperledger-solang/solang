// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use parity_scale_codec::{Decode, Encode};

#[test]
fn storage_load_on_return() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct SStruct {
        f1: i32,
    }

    let mut runtime = build_solidity(
        r##"
contract foo {
    struct S { int32 f1; }
        S[] arr;

    function g() private returns (S storage, S storage) {
        return (arr[0], arr[1]);
    }

    function f() public returns (S, S) {
        S[] storage ptrArr = arr;
        ptrArr.push(S({f1: 1}));
        ptrArr.push(S({f1: 2}));
        return g();
    }
}
        "##,
    );

    runtime.function("f", Vec::new());

    assert_eq!(
        runtime.output(),
        [SStruct { f1: 1 }, SStruct { f1: 2 }].encode(),
    );
}

#[test]
fn storage_initializer_addr_type() {
    // The contracts storage initializer writes to the scratch buffer in the storage.
    let mut runtime = build_solidity(r#"contract C { address public owner = msg.sender; }"#);

    // But this must not overwrite the input length; deploy should not revert.
    runtime.constructor(0, Vec::new());
    assert!(runtime.output().is_empty());

    // Expect the storage initializer to work properly.
    runtime.function("owner", Vec::new());
    assert_eq!(runtime.output(), runtime.caller());
}
