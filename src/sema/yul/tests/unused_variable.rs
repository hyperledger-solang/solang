// SPDX-License-Identifier: Apache-2.0

use crate::sema::yul::tests::parse;

#[test]
fn unused_variables() {
    let file = r#"
contract testTypes {
    function testAsm() public {
        assembly {

            let a
            for {let i := 11
              } lt(i, 10) {i := add(i, 1)
            invalid()
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
    assert!(ns
        .diagnostics
        .contains_message("yul variable 'a' has never been read or assigned"));

    let file = r#"
contract testTypes {
    function testAsm() public {
        assembly {

            for {let i := 11
                let c :=5
              } lt(i, 10) {i := add(i, 1)
            invalid()
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
    assert!(ns
        .diagnostics
        .contains_message("yul variable 'c' has never been read"));
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
            invalid()
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
    for item in ns.diagnostics.iter() {
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
    function testAsm(uint256[] calldata vl) view public {
        test storage tt2 = tt1;
        assembly {
            let g := vl.length
            let adr := add(g, tt2.slot)
            tt2.slot := sub(adr, b.slot)
        }
    }
}    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 1);
    assert!(ns
        .diagnostics
        .contains_message("found contract 'testTypes'"));
}
