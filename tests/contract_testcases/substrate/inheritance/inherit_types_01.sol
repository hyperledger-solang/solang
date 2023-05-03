
        contract a is b {
            function test() public returns (enum_x) {
                return enum_x.x2;
            }

            function test2() public returns (enum_y) {
                return enum_y.y2;
            }
        }

        abstract contract b is c {
            enum enum_y { y1, y2 }
        }

        abstract contract c {
            enum enum_x { x1, x2 }
        }
        
// ---- Expect: diagnostics ----
// warning: 3:13-52: function can be declared 'pure'
// warning: 7:13-53: function can be declared 'pure'
