contract test {
            function x(int64 arg1) internal returns (bool) {
                return false;
            }

            function foo() public {
                function(int32) returns (bool) a = x;
            }
        }
// ---- Expect: diagnostics ----
// warning: 2:30-34: function parameter 'arg1' is unused
// error: 7:52-53: function arguments do not match in conversion from 'function(int32) internal returns (bool)' to 'function(int64) internal returns (bool)'
