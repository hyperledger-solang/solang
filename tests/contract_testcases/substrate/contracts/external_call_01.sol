
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
// ----
// error (115-134): function expects 2 arguments, 1 provided
// error (115-134): missing argument 't' to function 'get_x'
// warning (223-224): declaration of 'b' shadows contract name
// 	note (169-300): previous declaration of contract name
