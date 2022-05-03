use crate::build_solidity;
use parity_scale_codec::{Decode, Encode};

#[test]
fn storage_load_on_return() {
    #[derive(Debug, PartialEq, Encode, Decode)]
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
        runtime.vm.output,
        [SStruct { f1: 1 }, SStruct { f1: 2 }].encode(),
    );
}
