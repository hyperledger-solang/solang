
        contract c {
            function test() public {
                int32 x = 0xa days;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:27-35: hexadecimal numbers cannot be used with unit denominations
