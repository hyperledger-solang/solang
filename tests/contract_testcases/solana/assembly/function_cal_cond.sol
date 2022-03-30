contract testTypes {
    function testAsm() public pure {
        assembly {
            {
                function tryThis(b, a) -> c {
                    a := add(a, 4)
                    if gt(a, 5) {
                        leave
                    }
                    b := add(a, 6)
                    c := tryThat(b, 2)
                    return(b, 0)
                }

                {
                    function foo(d) -> e {
                        e := shr(d, 3)
                    }

                    revert(tryThis(foo(3), 2), 4)
                }

                function tryThat(b, a) -> c {
                    a := add(a, 4)
                    if gt(a, 5) {
                        leave
                    }
                    c := 5
                    return(b, 0)
                }
                let x := 5
            }
        }
    }
} 