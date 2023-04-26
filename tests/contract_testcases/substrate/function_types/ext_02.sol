
        contract ft {
            function test() public {
                function(int32) external returns (bool) x = this.foo;
            }

            function foo(int32) public returns (bool) {
                return false;
            }

            function bar(int64) public returns (bool) {
                return false;
            }
        }
// ----
// warning (35-57): function can be declared 'view'
// warning (116-117): local variable 'x' has been assigned, but never read
// warning (157-198): function can be declared 'pure'
// warning (258-299): function can be declared 'pure'
