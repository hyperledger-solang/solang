// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};

#[test]
fn constants_hash_tests() {
    let mut runtime = build_solidity(
        r#"
        contract tester {
            function test() public {
                bytes32 hash = keccak256("Hello, World!");

                assert(hash == hex"acaf3289d7b601cbd114fb36c4d29c85bbfd5e133f14cb355c3fd8d99367964f");
            }
        }"#,
    );

    runtime.constructor(&[]);
    runtime.function("test", &[]);

    let mut runtime = build_solidity(
        r#"
        contract tester {
            function test() public {
                bytes32 hash = sha256("Hello, World!");

                assert(hash == hex"dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f");
            }
        }"#,
    );

    runtime.constructor(&[]);
    runtime.function("test", &[]);

    let mut runtime = build_solidity(
        r#"
        contract tester {
            function test() public {
                bytes20 hash = ripemd160("Hello, World!");

                assert(hash == hex"527a6a4b9a6da75607546842e0e00105350b1aaf");
            }
        }"#,
    );

    runtime.constructor(&[]);
    runtime.function("test", &[]);
}

#[test]
fn hash_tests() {
    let mut runtime = build_solidity(
        r##"
        contract tester {
            function test(bytes bs) public returns (bytes20) {
                bytes20 hash = ripemd160(bs);

                return hash;
            }
        }"##,
    );

    runtime.constructor(&[]);
    let hash = runtime
        .function("test", &[BorshToken::Bytes(b"Hello, World!".to_vec())])
        .unwrap();

    assert_eq!(
        hash,
        BorshToken::uint8_fixed_array(
            hex::decode("527a6a4b9a6da75607546842e0e00105350b1aaf").unwrap()
        )
    );

    let mut runtime = build_solidity(
        r##"
        contract tester {
            function test(bytes bs) public returns (bytes32) {
                bytes32 hash = sha256(bs);

                return hash;
            }
        }"##,
    );

    runtime.constructor(&[]);
    let hash = runtime
        .function("test", &[BorshToken::Bytes(b"Hello, World!".to_vec())])
        .unwrap();

    assert_eq!(
        hash,
        BorshToken::uint8_fixed_array(
            hex::decode("dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f")
                .unwrap()
        )
    );

    let mut runtime = build_solidity(
        r##"
        contract tester {
            function test(bytes bs) public returns (bytes32) {
                bytes32 hash = keccak256(bs);

                return hash;
            }
        }"##,
    );

    runtime.constructor(&[]);
    let hash = runtime
        .function("test", &[BorshToken::Bytes(b"Hello, World!".to_vec())])
        .unwrap();

    assert_eq!(
        hash,
        BorshToken::uint8_fixed_array(
            hex::decode("acaf3289d7b601cbd114fb36c4d29c85bbfd5e133f14cb355c3fd8d99367964f")
                .unwrap()
        )
    );
}
