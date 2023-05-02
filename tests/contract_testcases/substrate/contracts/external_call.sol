
        contract c {
            b x;
            function test() public returns (int32) {
                return x.get_x();
            }
        }

        contract b {
            function get_x(int32 t) public returns (int32) {
                return 1;
            }
        }
// ---- Expect: diagnostics ----
// error: 5:24-33: function expects 1 arguments, 0 provided
