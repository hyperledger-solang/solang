contract test {
            function x(int32 arg1) internal {}

            function foo() public {
                function(int32) view a = x;
            }
        }
// ----
// warning (45-49): function parameter 'arg1' has never been read
// error (141-142): function mutability not compatible in conversion from 'function(int32) internal' to 'function(int32) internal view'
