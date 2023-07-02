contract test {
            function x(int32 arg1) internal {}

            function foo() public {
                function(int32) pure a = x;
            }
        }
// ---- Expect: diagnostics ----
// warning: 2:30-34: function parameter 'arg1' is unused
// error: 5:42-43: function mutability not compatible in conversion from 'function(int32) internal' to 'function(int32) internal pure'
