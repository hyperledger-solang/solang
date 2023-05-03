
        contract C {
            uint256 public NOT_CONSTANT = 42;
        }

        contract foo {
            function f() public returns (uint) {
                uint a = C.NOT_CONSTANT;
                return a;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 8:26-40: need instance of contract 'C' to get variable value 'NOT_CONSTANT'
