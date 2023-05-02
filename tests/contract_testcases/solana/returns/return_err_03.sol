
        contract foo {
            uint private val = 0;
            function f() public returns (uint, uint, uint) {
                return (val, val, val);
            }

            function get() public {
                return f();
            }
        }
// ---- Expect: diagnostics ----
// error: 9:17-27: function has no return values
