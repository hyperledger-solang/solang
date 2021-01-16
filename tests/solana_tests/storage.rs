use crate::build_solidity;

#[test]
fn string() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            string s;

            function set(string value) public {
                s = value;
            }

            function get() public returns (string) {
                return s;
            }
        }"#,
    );

    vm.constructor(&[]);

    assert_eq!(
        vm.data[0..12].to_vec(),
        vec![65, 177, 160, 100, 12, 0, 0, 0, 0, 0, 0, 0]
    );

    let returns = vm.function("get", &[]);

    assert_eq!(returns, vec![ethabi::Token::String(String::from(""))]);

    vm.function(
        "set",
        &[ethabi::Token::String(String::from("Hello, World!"))],
    );

    assert_eq!(
        vm.data[0..12].to_vec(),
        vec![65, 177, 160, 100, 12, 0, 0, 0, 28, 0, 0, 0]
    );

    assert_eq!(vm.data[28..41].to_vec(), b"Hello, World!");

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![ethabi::Token::String(String::from("Hello, World!"))]
    );

    // try replacing it with a string of the same length. This is a special
    // fast-path handling
    vm.function(
        "set",
        &[ethabi::Token::String(String::from("Hallo, Werld!"))],
    );

    let returns = vm.function("get", &[]);

    assert_eq!(
        returns,
        vec![ethabi::Token::String(String::from("Hallo, Werld!"))]
    );

    assert_eq!(
        vm.data[0..12].to_vec(),
        vec![65, 177, 160, 100, 12, 0, 0, 0, 28, 0, 0, 0]
    );

    // Try setting this to an empty string. This is also a special case where
    // the result should be offset 0
    vm.function("set", &[ethabi::Token::String(String::from(""))]);

    let returns = vm.function("get", &[]);

    assert_eq!(returns, vec![ethabi::Token::String(String::from(""))]);

    assert_eq!(
        vm.data[0..12].to_vec(),
        vec![65, 177, 160, 100, 12, 0, 0, 0, 0, 0, 0, 0]
    );
}
