
        contract C {
            uint256 public NOT_CONSTANT = 42;
        }

        contract foo {
            function f() public returns (uint) {
                uint a = C.NOT_CONSTANT;
                return a;
            }
        }
        
// ----
// error (176-190): need instance of contract 'C' to get variable value 'NOT_CONSTANT'
