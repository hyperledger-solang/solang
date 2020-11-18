use crate::build_solidity;

/*

We need to test
 - uintN
 - intN
 - bytesN
 - enum
*/

#[test]
#[should_panic]
fn assert_false() {
    // without a working assert, this is not going to work
    let mut vm = build_solidity(
        r#"
        contract foo {
            function assert_fails() public {
                require(false, "humpty-dumpty");
            }
        }"#,
    );

    vm.constructor(&[]);

    vm.function("assert_fails", &[]);
}

#[test]
fn assert_true() {
    // without a working assert, this is not going to work
    let mut vm = build_solidity(
        r#"
        contract foo {
            function assert_fails() public {
                require(true, "humpty-dumpty");
            }
        }"#,
    );

    vm.constructor(&[]);

    vm.function("assert_fails", &[]);
}

#[test]
fn boolean() {
    // we need to test: literals
    // passing address around
    // abi encoding/decoding address
    // comparing address to another
    let mut vm = build_solidity(
        r#"
        contract foo {
            function return_true() public returns (bool) {
                return true;
            }

            function return_false() public returns (bool) {
                return false;
            }

            function true_arg(bool b) public {
                assert(b);
            }

            function false_arg(bool b) public {
                assert(!b);
            }
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("return_true", &[]);

    assert_eq!(returns, vec![ethabi::Token::Bool(true),]);

    let returns = vm.function("return_false", &[]);

    assert_eq!(returns, vec![ethabi::Token::Bool(false),]);

    vm.function("true_arg", &[ethabi::Token::Bool(true)]);
    vm.function("false_arg", &[ethabi::Token::Bool(false)]);
}

#[test]
fn address() {
    // we need to test: literals
    // passing address around
    // abi encoding/decoding address
    // comparing address to another

    let mut vm = build_solidity(
        r#"
        contract foo {
            function return_address() public returns (address) {
                return 0x7d5839e24ACaDa338c257643a7d2e025453F77D058b8335C1c3791Bc6742b320;
            }

            function address_arg(address a) public {
                assert(a == 0x8D166E028f3148854F2427d29B8755F617EED0651Bc6C8809b189200A4E3aaa9);
            }
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("return_address", &[]);

    assert_eq!(
        returns,
        vec![ethabi::Token::FixedBytes(vec![
            0x7d, 0x58, 0x39, 0xe2, 0x4a, 0xca, 0xda, 0x33, 0x8c, 0x25, 0x76, 0x43, 0xa7, 0xd2,
            0xe0, 0x25, 0x45, 0x3f, 0x77, 0xd0, 0x58, 0xb8, 0x33, 0x5c, 0x1c, 0x37, 0x91, 0xbc,
            0x67, 0x42, 0xb3, 0x20,
        ]),]
    );

    vm.function(
        "address_arg",
        &[ethabi::Token::FixedBytes(vec![
            0x8d, 0x16, 0x6e, 0x2, 0x8f, 0x31, 0x48, 0x85, 0x4f, 0x24, 0x27, 0xd2, 0x9b, 0x87,
            0x55, 0xf6, 0x17, 0xee, 0xd0, 0x65, 0x1b, 0xc6, 0xc8, 0x80, 0x9b, 0x18, 0x92, 0x0,
            0xa4, 0xe3, 0xaa, 0xa9,
        ])],
    );
}

#[test]
fn test_enum() {
    // we need to test enum literals
    // abi encoding/decode literals
    // comparing enums

    let mut vm = build_solidity(
        r#"
        contract foo {
            enum bar { bar0, bar1, bar2, bar3, bar4, bar5, bar6, bar7, bar8, bar9, bar10 }

            function return_enum() public returns (bar) {
                return bar.bar9;
            }

            function enum_arg(bar a) public {
                assert(a == bar.bar6);
            }
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("return_enum", &[]);

    assert_eq!(
        returns,
        vec![ethabi::Token::Uint(ethereum_types::U256::from(9))]
    );

    vm.function(
        "enum_arg",
        &[ethabi::Token::Uint(ethereum_types::U256::from(6))],
    );
}
