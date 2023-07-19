
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
// ---- Expect: diagnostics ----
// warning: 3:13-35: function can be declared 'view'
// warning: 4:57-58: local variable 'x' is unused
// warning: 7:13-54: function can be declared 'pure'
// warning: 11:13-54: function can be declared 'pure'
