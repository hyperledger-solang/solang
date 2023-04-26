
        contract c {
            b x;
            constructor() public {
                x = new b({ a: 1, a: 2 });
            }
            function test() public returns (int32) {
                return x.get_x({ t: 10 });
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
// error (94-115): duplicate argument name 'a'
// 	note (105-106): location of previous argument
// warning (327-333): 'public': visibility for constructors is ignored
