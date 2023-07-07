
        abstract contract base {
            function foo(uint64 a) private returns (uint64) {
                return a + 102;
            }
        }

        contract apex is base {
            function bar() public returns (uint64) {
                return foo({a: 3}) + 3;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 10:24-35: cannot call private function
// 	note 3:13-60: declaration of function 'foo'
