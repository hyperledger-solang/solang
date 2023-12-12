
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

// ---- Expect: diagnostics ----
// warning: 4:27-33: 'public': visibility for constructors is ignored
// error: 5:35-36: duplicate argument with name 'a'
// 	note 5:32-33: location of previous argument
// warning: 14:34-40: 'public': visibility for constructors is ignored
