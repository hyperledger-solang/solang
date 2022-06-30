use crate::build_solidity;
use ethabi::{ethereum_types::U256, Token};

#[test]
fn returns_and_phis_needed() {
    let mut vm = build_solidity(
        r#"
        contract c {
            int foo;
            bool bar;

            function func(bool cond) external mod(cond) returns (int, bool) {
                return (foo, bar);
            }

            modifier mod(bool cond) {
                bar = cond;
                if (cond) {
                    foo = 12;
                    _;
                } else {
                    foo = 40;
                    _;
                }
            }
        }"#,
    );

    vm.constructor("c", &[]);

    let returns = vm.function("func", &[Token::Bool(false)], &[], None);

    assert_eq!(
        returns,
        vec![Token::Int(U256::from(40)), Token::Bool(false)]
    );

    let returns = vm.function("func", &[Token::Bool(true)], &[], None);

    assert_eq!(returns, vec![Token::Int(U256::from(12)), Token::Bool(true)]);
}
