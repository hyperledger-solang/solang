
        abstract contract base {
            function foo() private returns (uint64) {
                return 102;
            }
        }

        contract apex is base {
            function bar() public returns (uint64) {
                return foo() + 3;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 10:24-29: cannot call private function
// 	note 3:13-52: declaration of function 'foo'
