contract test {
            function x(int32 arg1) internal returns (bool) {
                return false;
            }

            function foo() public {
                function(int32) a = x;
            }
        }
// ----
// warning (45-49): function parameter 'arg1' has never been read
// error (194-195): function returns do not match in conversion from 'function(int32) internal' to 'function(int32) internal returns (bool)'
