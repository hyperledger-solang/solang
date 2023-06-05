contract test {
            function x(int32 arg1) public payable {}

            function foo() public {
                function(int32) a = x;
            }
        }
// ---- Expect: diagnostics ----
// warning: 2:30-34: function parameter 'arg1' is unused
// error: 5:37-38: function mutability not compatible in conversion from 'function(int32) internal payable' to 'function(int32) internal'
