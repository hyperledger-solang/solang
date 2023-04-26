
        contract c {
            b x;
            constructor() public {
                x = new b(102);
            }
            function test() public returns (int32) {
                return x.get_x({ t: 10, t: false });
            }
        }

        contract b {
            int32 x;
            constructor(int32 a) public {
                x = a;
            }
            function get_x(int32 t) public returns (int32) {
                return x * t;
            }
        }
// ----
// warning (65-71): 'public': visibility for constructors is ignored
// error (196-224): function expects 1 arguments, 2 provided
// error (213-214): duplicate argument with name 't'
// warning (326-332): 'public': visibility for constructors is ignored
