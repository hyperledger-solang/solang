contract test {
            function x(int64 arg1) internal returns (bool) {
                return false;
            }

            function foo() public {
                function(int32) returns (bool) a = x;
            }
        }
// ----
// warning (45-49): function parameter 'arg1' has never been read
// error (209-210): function arguments do not match in conversion from 'function(int32) internal returns (bool)' to 'function(int64) internal returns (bool)'
