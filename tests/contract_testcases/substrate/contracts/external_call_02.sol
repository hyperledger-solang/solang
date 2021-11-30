
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