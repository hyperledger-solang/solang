
        contract c {
            function foo() public returns (uint) {
                    uint8[4] memory bar = [ 1, 2, 3, 4 ];

                    return bar[];
            }
        }
// ----
// error (159-164): expected expression before ']' token
