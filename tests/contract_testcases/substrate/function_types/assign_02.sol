contract test {
            function x(int32 arg1) public payable {}

            function foo() public {
                function(int32) a = x;
            }
        }
// ----
// warning (45-49): function parameter 'arg1' has never been read
// error (142-143): function mutability not compatible in conversion from 'function(int32) internal payable' to 'function(int32) internal'
