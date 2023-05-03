
        contract c {
            function test() public {
                int32 x = type(bool).max;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:27-41: type 'bool' does not have type function max
