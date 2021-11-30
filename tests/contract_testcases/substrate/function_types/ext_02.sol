
        contract ft {
            function test() public {
                function(int32) external returns (bool) x = this.foo;
            }

            function foo(int32) public returns (bool) {
                return false;
            }

            function foo(int64) public returns (bool) {
                return false;
            }
        }