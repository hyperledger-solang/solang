
        contract C {
            uint256 public constant STATIC = 42;
        }

        contract foo {
            function f() public returns (uint) {
                uint a = C.STATIC();
                return a;
            }
        }
        
// ----
// error (179-180): 'C' is a contract
// error (179-189): function calls via contract name are only valid for base contracts
