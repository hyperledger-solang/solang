
        contract ft {
            function test() public {
                function(int32) external returns (bool) x = this.foo;
            }

            function foo(int32) internal returns (bool) {
                return false;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:66-69: contract 'ft' has no public function 'foo'
