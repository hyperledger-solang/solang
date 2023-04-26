
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
        
// ----
// error (37-83): function 'foo' with this signature already defined
// 	note (187-233): previous definition of function 'foo'
