pragma solidity <=0.5.12;

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
// error: 4:13-59: function 'foo' with this signature already defined
// 	note 10:13-59: previous definition of function 'foo'
