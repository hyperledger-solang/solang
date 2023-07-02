contract test {
            function x(int32 arg1) internal returns (bool) {
                return false;
            }

            function foo() public {
                function(int32) a = x;
            }
        }
// ---- Expect: diagnostics ----
// warning: 2:30-34: function parameter 'arg1' is unused
// error: 7:37-38: function returns do not match in conversion from 'function(int32) internal' to 'function(int32) internal returns (bool)'
