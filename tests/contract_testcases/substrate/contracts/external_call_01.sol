
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