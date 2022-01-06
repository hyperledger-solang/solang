use crate::{address_new, build_solidity};

#[test]
fn issue_623() {
    let mut vm = build_solidity(
        r#"
        contract Missing{
            address payable private owner;

            modifier onlyowner {
                require(msg.sender==owner, "Go away");
                _;
            }

            constructor()
                public
            {
                owner = payable(msg.sender);
            }

            receive () external payable {}

            function withdraw()
                public
                onlyowner
            {
                owner.transfer(payable(this).balance);
            }

            function checkmod()
            public
            onlyowner
            {}
    }"#,
    );

    vm.constructor(&[]);

    vm.function("checkmod", &[]);

    vm.caller = address_new();

    let revert = vm.function_revert("checkmod", &[]);

    assert_eq!(revert, Some(String::from("Go away")));
}
