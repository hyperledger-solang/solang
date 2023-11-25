
        contract base {
            function foo(uint64 a) public returns (uint64) {
                return a + 102;
            }
        }

        contract apex is base {
            function foo(uint64 a) public returns (uint64) {
                return a + 64;
            }

            function bar() public returns (uint64) {
                return foo({a: 3}) + 3;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 3:13-59: function 'foo' with this signature already defined
// 	note 9:13-59: previous definition of function 'foo'
// error: 14:24-35: function call can be resolved to multiple functions
// 	note 3:13-5:14: candidate function
// 	note 9:13-11:14: candidate function
