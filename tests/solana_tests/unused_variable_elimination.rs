use crate::build_solidity;
use ethabi::{ethereum_types::U256, Token};

/// This tests check that a public storage variable is not eliminated
/// and that an assignment inside an expression works
#[test]
fn test_returns() {
    let file = r#"
    contract c1 {
        int public pb1;

        function assign() public {
            pb1 = 5;
        }

        int t1;
        int t2;
        function test1() public returns (int) {
            t1 = 2;
            t2 = 3;
            int f = 6;
            int c = 32 +4 *(f = t1+t2);
            return c;
        }

        function test2() public returns (int) {
            t1 = 2;
            t2 = 3;
            int f = 6;
            int c = 32 + 4*(f= t1+t2);
            return f;
        }

    }
    "#;

    let mut vm = build_solidity(file);
    vm.constructor("c1", &[], 0);
    let _ = vm.function("assign", &[], &[], 0, None);
    let returns = vm.function("pb1", &[], &[], 0, None);

    assert_eq!(returns, vec![Token::Int(U256::from(5))]);

    let returns = vm.function("test1", &[], &[], 0, None);
    assert_eq!(returns, vec![Token::Int(U256::from(52))]);
    let returns = vm.function("test2", &[], &[], 0, None);
    assert_eq!(returns, vec![Token::Int(U256::from(5))]);
}
