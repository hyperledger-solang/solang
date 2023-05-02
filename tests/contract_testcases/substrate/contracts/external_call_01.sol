
        contract c {
            b x;
            function test() public returns (int32) {
                return x.get_x({b: false});
            }
        }

        contract b {
            function get_x(int32 t, bool b) public returns (int32) {
                return 1;
            }
        }
// ---- Expect: diagnostics ----
// error: 5:24-43: function expects 2 arguments, 1 provided
// error: 5:24-43: missing argument 't' to function 'get_x'
// warning: 10:42-43: declaration of 'b' shadows contract name
// 	note 9:9-13:10: previous declaration of contract name
