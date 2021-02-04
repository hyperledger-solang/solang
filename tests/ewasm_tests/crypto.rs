use crate::build_solidity;

#[test]
fn hash_tests() {
    let mut runtime = build_solidity(
        r##"
        contract tester {
            function test() public {
                bytes32 hash = keccak256("Hello, World!");

                assert(hash == hex"acaf3289d7b601cbd114fb36c4d29c85bbfd5e133f14cb355c3fd8d99367964f");
            }
        }"##,
    );

    runtime.constructor(&[]);
    runtime.function("test", &[]);

    let mut runtime = build_solidity(
        r##"
        contract tester {
            function test() public {
                bytes memory s = "Hello, World!";
                bytes32 hash = keccak256(s);

                assert(hash == hex"acaf3289d7b601cbd114fb36c4d29c85bbfd5e133f14cb355c3fd8d99367964f");
            }
        }"##,
    );

    runtime.constructor(&[]);
    runtime.function("test", &[]);

    let mut runtime = build_solidity(
        r##"
        contract tester {
            function test() public {
                bytes32 hash = sha256("Hello, World!");

                assert(hash == hex"dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f");
            }
        }"##,
    );

    runtime.constructor(&[]);
    runtime.function("test", &[]);

    let mut runtime = build_solidity(
        r##"
        contract tester {
            function test() public {
                bytes20 hash = ripemd160("Hello, World!");

                assert(hash == hex"527a6a4b9a6da75607546842e0e00105350b1aaf");
            }
        }"##,
    );

    runtime.constructor(&[]);
    runtime.function("test", &[]);
}
