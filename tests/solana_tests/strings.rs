use crate::build_solidity;
use ethabi::ethereum_types::U256;
use ethabi::Token;

#[test]
fn storage_string_length() {
    let mut vm = build_solidity(
        r#"
    contract Testing {
        string st;
        function setString(string input) public {
            st = input;
        }

        function getLength() public view returns (uint32) {
            return st.length;
        }
    }
    "#,
    );
    vm.constructor("Testing", &[]);

    let _ = vm.function(
        "setString",
        &[Token::String("coffee_tastes_good".to_string())],
        &[],
        None,
    );
    let returns = vm.function("getLength", &[], &[], None);

    assert_eq!(returns[0], Token::Uint(U256::from(18)));
}
