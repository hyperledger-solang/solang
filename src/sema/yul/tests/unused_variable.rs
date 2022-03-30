use crate::sema::yul::tests::{assert_message_in_diagnostics, parse};

#[test]
fn unused_variables() {
    let file = r#"
contract testTypes {
    function testAsm() public {
        assembly {

            let a
            for {let i := 11
              } lt(i, 10) {i := add(i, 1)
            stop()
        } {
                let x := shr(i, 2)
                let b := shr(6, 5)
                    x := mul(x, b)
                //stop()
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "yul variable ‘a‘ has never been read or assigned"
    ));

    let file = r#"
contract testTypes {
    function testAsm() public {
        assembly {

            for {let i := 11
                let c :=5
              } lt(i, 10) {i := add(i, 1)
            stop()
        } {
                let x := shr(i, 2)
                let b := shr(6, 5)
                    x := mul(x, b)
                //stop()
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "yul variable ‘c‘ has never been read"
    ));
}

#[test]
fn correct_contracts() {
    let file = r#"
    contract testTypes {
    function testAsm() public {
        assembly {

            for {let i := 11
                let c :=5
              } c {i := add(i, 1)
            stop()
        } {
                let x := shr(i, 2)
                let b := shr(6, 5)
                    x := mul(x, b)
                //stop()
            }

            let x := 0
            if x {
                let y := add(1, 2)
                return(y, 0)
            }

            let y := 5
            let z
            switch y
            case 0 {z := 4}
            case 1 {z := add(z, 3)}

        }
    }
}    "#;

    let ns = parse(file);
    for item in &ns.diagnostics {
        assert!(!item.message.starts_with("yul variable has never been"));
    }

    let file = r#"
    contract testTypes {
    uint b = 0;
    struct test {
        int a;
        int b;
    }
    test tt1;
    function testAsm(uint256[] calldata vl) public {
        test storage tt2 = tt1;
        assembly {
            let g := vl.length
            let adr := add(g, tt2.slot)
            sstore(adr, b.slot)
        }
    }
}    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 2);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "found contract ‘testTypes’"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "evm assembly not supported on target solana"
    ));
}
