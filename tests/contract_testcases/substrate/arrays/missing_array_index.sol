
        contract c {
            function foo() public returns (uint) {
                    uint8[4] memory bar = [ 1, 2, 3, 4 ];

                    return bar[];
            }
        }
// ---- Expect: diagnostics ----
// error: 6:28-33: expected expression before ']' token
