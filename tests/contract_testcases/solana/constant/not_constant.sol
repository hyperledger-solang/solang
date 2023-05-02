
        contract C {
            uint256 public constant STATIC = 42;
        }

        contract foo {
            function f() public returns (uint) {
                uint a = C.STATIC();
                return a;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 8:26-27: 'C' is a contract
// error: 8:26-36: function calls via contract name are only valid for base contracts
