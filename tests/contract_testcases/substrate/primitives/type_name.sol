
        contract c {
            function test() public {
                int32 x = type(bool).max;
            }
        }
// ----
// error (85-99): type 'bool' does not have type function max
