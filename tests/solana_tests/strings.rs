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

#[test]
fn load_string_vector() {
    let mut vm = build_solidity(
        r#"
    contract Testing {
        string[] string_vec;
        function testLength() public returns (uint32, uint32, uint32) {
            string_vec.push("tea");
            string_vec.push("coffe");
            string_vec.push("sixsix");
            string[] memory rr = string_vec;
            return (rr[0].length, rr[1].length, rr[2].length);
        }

        function getString(uint32 index) public view returns (string memory) {
            string[] memory rr = string_vec;
            return rr[index];
        }
    }
      "#,
    );

    vm.constructor("Testing", &[]);
    let returns = vm.function("testLength", &[], &[], None);
    assert_eq!(returns[0], Token::Uint(U256::from(3)));
    assert_eq!(returns[1], Token::Uint(U256::from(5)));
    assert_eq!(returns[2], Token::Uint(U256::from(6)));

    let returns = vm.function("getString", &[Token::Uint(U256::from(0))], &[], None);
    assert_eq!(returns[0], Token::String("tea".to_string()));

    let returns = vm.function("getString", &[Token::Uint(U256::from(1))], &[], None);
    assert_eq!(returns[0], Token::String("coffe".to_string()));

    let returns = vm.function("getString", &[Token::Uint(U256::from(2))], &[], None);
    assert_eq!(returns[0], Token::String("sixsix".to_string()));
}
