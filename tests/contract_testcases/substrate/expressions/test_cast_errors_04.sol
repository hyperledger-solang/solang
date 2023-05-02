contract foo {
            uint bar;

            function set_bar(uint32 b) public {
                bar = b;
            }

            function get_bar() public returns (uint32) {
                return uint32(bar);
            }
        }

        contract foo2 {
            enum X { Y1, Y2, Y3}
            X y;

            function set_x(uint32 b) public {
                y = X(b);
            }

            function get_x() public returns (uint32) {
                return uint32(y);
            }

            function set_enum_x(X b) public {
                set_x(uint32(b));
            }
        }
// ---- Expect: diagnostics ----
// warning: 8:13-55: function can be declared 'view'
// warning: 21:13-53: function can be declared 'view'
