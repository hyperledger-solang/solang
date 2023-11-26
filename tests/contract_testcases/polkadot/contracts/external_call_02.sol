
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

// ---- Expect: diagnostics ----
// warning: 4:27-33: 'public': visibility for constructors is ignored
// error: 8:24-52: function expects 1 arguments, 2 provided
// error: 8:41-42: duplicate argument with name 't'
// error: 8:44-49: conversion from bool to int32 not possible
// warning: 14:34-40: 'public': visibility for constructors is ignored
