
        contract foo {
            uint private val = 0;

            function get() public {
                return val;
            }
        }
// ---- Expect: diagnostics ----
// error: 6:17-27: function has no return values
